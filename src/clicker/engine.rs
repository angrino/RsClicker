use std::sync::mpsc::{channel, Receiver};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::types::{create_enigo, perform_single_click, ClickConfig};

/// Grace period `stop()` waits for an in-flight click to finish before it
/// hands the join to a reaper thread.
const STOP_WAIT_MS: u64 = 50;

/// Why a click engine failed to run.
#[derive(Debug, Clone)]
pub enum StartError {
    /// No input-emulation backend is available (e.g. on Wayland without an
    /// `org.freedesktop.portal.RemoteDesktop` provider, enigo cannot inject
    /// input and panics internally — caught here and reported instead).
    NoBackend(&'static str),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartError::NoBackend(backend) => write!(
                f,
                "No input backend ({backend}). On Wayland this needs an \
                 xdg-desktop-portal RemoteDesktop provider (e.g. GNOME, KDE \
                 or Hyprland with portals enabled)."
            ),
        }
    }
}

pub struct ClickEngine {
    running: Arc<AtomicBool>,
    /// Identity token of the spawned worker, bumped by `start()` before it
    /// raises the flag. Worker tails touch shared state only while they are
    /// still the current generation — without the old blocking join, the
    /// tail of an in-flight old worker could clobber the next worker.
    generation: Arc<AtomicU64>,
    handle: Option<JoinHandle<()>>,
    /// Per-worker exit signal; `stop()` waits up to `STOP_WAIT_MS` on it.
    done_rx: Option<Receiver<()>>,
    /// Set by the worker thread when it fails to start (e.g. enigo backend
    /// missing). Drained by the UI to surface an ERROR state.
    error: Arc<Mutex<Option<String>>>,
}

impl Default for ClickEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClickEngine {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            generation: Arc::new(AtomicU64::new(0)),
            handle: None,
            done_rx: None,
            error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns and clears a pending start error, if the worker reported one.
    pub fn take_error(&self) -> Option<String> {
        self.error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    pub fn start(&mut self, config: ClickConfig) {
        self.stop();
        *self.error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.running.store(true, Ordering::SeqCst);
        let flag = Arc::clone(&self.running);
        let generation = Arc::clone(&self.generation);
        let error = Arc::clone(&self.error);
        let (done_tx, done_rx) = channel();
        self.done_rx = Some(done_rx);

        // The whole worker body is wrapped in `catch_unwind`: a panic inside
        // the click loop (e.g. enigo internals) records "Engine crashed: …"
        // and clears the flag, so the UI shows the red ERROR state instead
        // of a zombie running state.
        let worker = thread::Builder::new()
            .name("rsclicker-click".to_string())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || worker_body(&flag, &error, &generation, gen, &config),
                ));
                if generation.load(Ordering::SeqCst) == gen {
                    if let Err(payload) = outcome {
                        let msg = panic_message(payload.as_ref());
                        *error.lock().unwrap_or_else(|e| e.into_inner()) =
                            Some(format!("Engine crashed: {msg}"));
                    }
                    clear_flag_if_current(&flag, &generation, gen);
                }
                let _ = done_tx.send(());
            });
        match worker {
            Ok(handle) => self.handle = Some(handle),
            Err(e) => {
                self.done_rx = None;
                *self.error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("failed to spawn click thread: {e}"));
                self.running.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        // Never join on the caller thread: an in-flight click (or a hung
        // portal call) would freeze the UI. Wait briefly for the worker's
        // exit signal so a rapid restart doesn't overlap its last click,
        // then hand the join to a detached reaper.
        if let Some(done_rx) = self.done_rx.take() {
            let _ = done_rx.recv_timeout(Duration::from_millis(STOP_WAIT_MS));
        }
        if let Some(handle) = self.handle.take() {
            let _ = thread::Builder::new()
                .name("rsclicker-join".to_string())
                .spawn(move || {
                    let _ = handle.join();
                });
        }
    }
}

impl Drop for ClickEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Creates the enigo backend and runs the click loop. Reports startup
/// failure (no backend) through `error`, but only while `gen` is the
/// current generation.
fn worker_body(
    flag: &AtomicBool,
    error: &Mutex<Option<String>>,
    generation: &AtomicU64,
    gen: u64,
    config: &ClickConfig,
) {
    // enigo may panic internally when no backend exists (notably the libei
    // RemoteDesktop portal path on Wayland). Catch it so the whole app
    // doesn't go down; report through the error flag.
    let mut enigo = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        create_enigo,
    )) {
        Ok(Some(e)) => e,
        Ok(None) => {
            if generation.load(Ordering::SeqCst) == gen {
                *error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(StartError::NoBackend("enigo").to_string());
            }
            return;
        }
        Err(_) => {
            if generation.load(Ordering::SeqCst) == gen {
                *error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(StartError::NoBackend("enigo/libei").to_string());
            }
            return;
        }
    };
    let interval = config.sanitized_interval_ms();
    run_click_loop(flag, generation, gen, error, interval, |index| {
        perform_single_click(&mut enigo, config, index)
    });
}

/// Click engine core: runs `click_once` while the running flag holds and
/// fail-fasts on the first injection failure (a denied backend keeps
/// failing forever — continuing would stay green while clicking nothing).
fn run_click_loop(
    flag: &AtomicBool,
    generation: &AtomicU64,
    gen: u64,
    error: &Mutex<Option<String>>,
    interval: u64,
    mut click_once: impl FnMut(usize) -> Result<(), String>,
) {
    let mut index = 0usize;
    let interval_dur = Duration::from_millis(interval.max(1));
    let mut next_deadline = Instant::now();
    while flag.load(Ordering::SeqCst) {
        if let Err(e) = click_once(index) {
            if generation.load(Ordering::SeqCst) == gen {
                *error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("Click failed: {e}"));
            }
            break;
        }
        index = index.wrapping_add(1);
        next_deadline += interval_dur;
        let now = Instant::now();
        if next_deadline < now {
            // Prevent catch-up bursts if a lag spike or sleep overshoot occurs.
            next_deadline = now + interval_dur;
        } else {
            // Sleep in small slices until `next_deadline` so `stop()` stays responsive.
            while flag.load(Ordering::SeqCst) {
                let remaining = next_deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                let step = remaining.min(Duration::from_millis(50));
                thread::sleep(step);
            }
        }
    }
}

/// Worker tail: clears the running flag only while this worker is still the
/// current generation.
fn clear_flag_if_current(flag: &AtomicBool, generation: &AtomicU64, gen: u64) {
    if generation.load(Ordering::SeqCst) == gen {
        flag.store(false, Ordering::SeqCst);
    }
}

/// Best-effort printable message from a panic payload.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_flag_if_current, panic_message, run_click_loop};
    use std::panic::{catch_unwind, panic_any};
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn str_payload() {
        let payload = catch_unwind(|| panic!("backed exploded")).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "backed exploded");
    }

    #[test]
    fn string_payload() {
        let payload = catch_unwind(|| panic_any("portal missing".to_string())).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "portal missing");
    }

    #[test]
    fn non_string_payload() {
        let payload = catch_unwind(|| panic_any(7u32)).unwrap_err();
        assert_eq!(panic_message(payload.as_ref()), "unknown panic payload");
    }

    #[test]
    fn stale_generation_cannot_clobber_flag() {
        let flag = AtomicBool::new(true);
        let generation = AtomicU64::new(2);
        clear_flag_if_current(&flag, &generation, 1);
        assert!(flag.load(Ordering::SeqCst));
        clear_flag_if_current(&flag, &generation, 2);
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn click_failure_stops_engine_and_reports() {
        let flag = AtomicBool::new(true);
        let generation = AtomicU64::new(0);
        let error: Mutex<Option<String>> = Mutex::new(None);
        run_click_loop(&flag, &generation, 0, &error, 1, |i| {
            if i < 2 {
                Ok(())
            } else {
                Err("portal denied".to_string())
            }
        });
        assert_eq!(
            error.lock().unwrap().as_deref(),
            Some("Click failed: portal denied")
        );
        // The worker tail (generation-guarded) owns the flag clearing.
        clear_flag_if_current(&flag, &generation, 0);
        assert!(!flag.load(Ordering::SeqCst));
    }

    #[test]
    fn click_error_from_stale_generation_is_dropped() {
        let flag = AtomicBool::new(true);
        let generation = AtomicU64::new(2);
        let error: Mutex<Option<String>> = Mutex::new(None);
        run_click_loop(&flag, &generation, 1, &error, 1, |_| {
            Err("late".to_string())
        });
        assert!(error.lock().unwrap().is_none());
        assert!(flag.load(Ordering::SeqCst));
    }

    #[test]
    fn cadence_follows_deadlines_not_click_latency() {
        // Injection cost must not shift the schedule: click k lands on the
        // deadline train anchored at loop start, not after
        // (interval + latency) * k like the old slice-sleep code.
        let flag = AtomicBool::new(true);
        let generation = AtomicU64::new(0);
        let error: Mutex<Option<String>> = Mutex::new(None);
        let mut stamps: Vec<Instant> = Vec::new();
        run_click_loop(&flag, &generation, 0, &error, 50, |i| {
            stamps.push(Instant::now());
            thread::sleep(Duration::from_millis(30));
            if i < 5 {
                Ok(())
            } else {
                Err("end".to_string())
            }
        });
        assert_eq!(stamps.len(), 6);
        let span_ms = (stamps[5] - stamps[0]).as_millis();
        // Ideal span is 250 ms; the old drift code produced ≈400 ms here.
        assert!(span_ms >= 240, "span {span_ms} ms — too fast for 5×50 ms");
        assert!(span_ms <= 330, "span {span_ms} ms — drifted like the old loop");
    }
}
