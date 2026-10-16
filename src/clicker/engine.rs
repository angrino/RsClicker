use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::types::{create_enigo, perform_single_click, ClickConfig};

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
    handle: Option<JoinHandle<()>>,
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
            handle: None,
            error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Returns and clears a pending start error, if the worker reported one.
    pub fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }

    pub fn start(&mut self, config: ClickConfig) {
        self.stop();
        *self.error.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.running.store(true, Ordering::SeqCst);
        let flag = Arc::clone(&self.running);
        let error = Arc::clone(&self.error);
        let flag_for_spawn = Arc::clone(&flag);
        let error_for_spawn = Arc::clone(&error);

        let worker = thread::Builder::new()
            .name("rsclicker-click".to_string())
            .spawn(move || {
                let flag = flag_for_spawn;
                let error = error_for_spawn;
                // The whole worker body is wrapped in `catch_unwind`: a panic
                // inside the click loop (e.g. enigo internals) must not leave
                // the engine stuck flagged as running with no clicks and no
                // error. A crash is recorded in the error slot instead, which
                // the UI drains each tick and surfaces as the red ERROR state.
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || worker_body(&flag, &error, &config),
                ));
                if let Err(payload) = outcome {
                    let msg = panic_message(payload.as_ref());
                    *error.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(format!("Engine crashed: {msg}"));
                }
                // Safety net: a thread that has ended must never leave the
                // running flag raised. No race with a new `start()`, which
                // only raises the flag after `stop()` joins this thread.
                flag.store(false, Ordering::SeqCst);
            });
        match worker {
            Ok(handle) => self.handle = Some(handle),
            Err(e) => {
                *error.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(format!("failed to spawn click thread: {e}"));
                flag.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ClickEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Runs the click loop for one worker thread. Reports startup failure
/// (no backend) through `error` and clears `flag` on every exit path.
fn worker_body(flag: &AtomicBool, error: &Mutex<Option<String>>, config: &ClickConfig) {
    // enigo may panic internally when no backend exists (notably the
    // libei RemoteDesktop portal path on Wayland). Catch it so the
    // whole app doesn't go down; report through the error flag.
    let mut enigo = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        create_enigo,
    )) {
        Ok(Some(e)) => e,
        Ok(None) => {
            *error.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(StartError::NoBackend("enigo").to_string());
            flag.store(false, Ordering::SeqCst);
            return;
        }
        Err(_) => {
            *error.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(StartError::NoBackend("enigo/libei").to_string());
            flag.store(false, Ordering::SeqCst);
            return;
        }
    };
    let interval = config.sanitized_interval_ms();
    let mut index = 0usize;
    while flag.load(Ordering::SeqCst) {
        perform_single_click(&mut enigo, config, index);
        index = index.wrapping_add(1);
        // Sleep in small slices so `stop()` reacts immediately even
        // when the configured interval is very large (minutes/hours).
        // Without this, joining the worker thread would freeze the UI.
        let mut remaining = interval;
        while remaining > 0 && flag.load(Ordering::SeqCst) {
            let step = remaining.min(50);
            thread::sleep(Duration::from_millis(step));
            remaining = remaining.saturating_sub(step);
        }
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
    use super::panic_message;
    use std::panic::{catch_unwind, panic_any};

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
}
