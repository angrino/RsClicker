//! Global mouse-button monitoring for mouse triggers.
//!
//! - Linux: reads `/dev/input/event*` via evdev (no grab — input is only
//!   observed, never stolen). Needs read permission, i.e. membership in the
//!   `input` group. A scanner respawns dead readers and picks up hot-plugged
//!   devices every `RESCAN_SECS`.
//! - Windows 10/11: low-level mouse hook via rdev (no extra setup).
//! - Other platforms: stub that never yields events.

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

/// A mouse button usable as a profile trigger, including side buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MouseTrigger {
    Left,
    Right,
    Middle,
    Side,
    Extra,
}

impl std::fmt::Display for MouseTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MouseTrigger::Left => "Mouse Left",
            MouseTrigger::Right => "Mouse Right",
            MouseTrigger::Middle => "Mouse Middle",
            MouseTrigger::Side => "Mouse Side",
            MouseTrigger::Extra => "Mouse Extra",
        })
    }
}

/// A mouse-button edge as seen by the global monitor: `pressed` plus the
/// device timestamp in millis since the UNIX epoch (drivers/OS event time,
/// NOT processing time — needed to recognize injected press→release pairs
/// by their near-zero gap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseEvent {
    Press(MouseTrigger, u64),
    Release(MouseTrigger, u64),
}

/// Health of the trigger monitor, checked when arming a mouse-trigger
/// profile (never pushed proactively).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonitorStatus {
    /// The first device scan has not completed yet.
    Unknown,
    /// Monitoring `n` mouse device(s).
    Running(usize),
    /// Nothing can be monitored; the reason is shown in the status line.
    Unavailable(String),
}

pub struct MouseMonitor {
    pub rx: Receiver<MouseEvent>,
    status: Arc<Mutex<MonitorStatus>>,
}

impl MouseMonitor {
    pub fn status(&self) -> MonitorStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(MonitorStatus::Unknown)
    }
}

fn epoch_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

/// Same-button press→release pairs closer than this are treated as
/// injected clicks and dropped wholesale. SendInput pairs reach a
/// low-level hook ≤ ~2 ms apart (there is no LLMHF_INJECTED flag in rdev)
/// and evdev pairs share a timeval, while the fastest physical release
/// stays far above this bound.
pub const PAIR_EPS_MS: u64 = 20;

/// Removes injected click pairs from a monitor batch in arrival order.
/// A release exactly cancels its latest unmatched press of the same button
/// when the device-timestamp gap is within `PAIR_EPS_MS`; unpaired presses
/// (held buttons) and slower pairs route untouched. Both edges are dropped
/// together so toggle/stop logic never sees self-injected input.
pub fn filter_injected_pairs(events: Vec<MouseEvent>) -> Vec<MouseEvent> {
    let mut keep = vec![true; events.len()];
    let mut last_press: HashMap<MouseTrigger, usize> = HashMap::new();
    for (i, ev) in events.iter().enumerate() {
        match *ev {
            MouseEvent::Press(button, _) => {
                last_press.insert(button, i);
            }
            MouseEvent::Release(button, ts) => {
                let matched = last_press
                    .get(&button)
                    .copied()
                    .and_then(|pi| match events[pi] {
                        MouseEvent::Press(_, press_ts) => {
                            if ts >= press_ts && ts - press_ts <= PAIR_EPS_MS {
                                Some(pi)
                            } else {
                                None
                            }
                        }
                        MouseEvent::Release(..) => None,
                    });
                if let Some(pi) = matched {
                    keep[i] = false;
                    keep[pi] = false;
                    last_press.remove(&button);
                }
            }
        }
    }
    events
        .into_iter()
        .zip(keep)
        .filter(|(_, k)| *k)
        .map(|(ev, _)| ev)
        .collect()
}

#[cfg(target_os = "linux")]
/// Rescan interval for hot-plug discovery and dead-reader respawn.
const RESCAN_SECS: u64 = 5;

#[cfg(target_os = "linux")]
fn code_to_trigger(code: u16) -> Option<MouseTrigger> {
    // Linux input-event-codes.h button codes.
    match code {
        0x110 => Some(MouseTrigger::Left),   // BTN_LEFT
        0x111 => Some(MouseTrigger::Right),  // BTN_RIGHT
        // Bro , go find a job :D
        0x112 => Some(MouseTrigger::Middle), // BTN_MIDDLE
        0x113 | 0x116 => Some(MouseTrigger::Side), // BTN_SIDE / BTN_BACK
        0x114 | 0x115 => Some(MouseTrigger::Extra), // BTN_EXTRA / BTN_FORWARD
        _ => None,
    }
}

#[cfg(target_os = "linux")]
fn device_is_mouse(device: &evdev::Device) -> bool {
    device
        .supported_keys()
        .map(|keys| keys.contains(evdev::KeyCode::BTN_LEFT))
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
/// Returns a fresh alive flag when a reader for this device path is wanted
/// (not known yet, or the previous one died); `None` while a reader is live.
fn reader_wanted(
    known: &mut HashMap<PathBuf, Arc<AtomicBool>>,
    path: &Path,
) -> Option<Arc<AtomicBool>> {
    if let Some(alive) = known.get(path) {
        if alive.load(Ordering::SeqCst) {
            return None;
        }
    }
    let alive = Arc::new(AtomicBool::new(true));
    known.insert(path.to_path_buf(), alive.clone());
    Some(alive)
}

/// Spawns the background scanner (one reader thread per live mouse device)
/// and returns the receiving end. Never grabs devices. `evdev::enumerate()`
/// silently swallows its own errors, so availability is self-detected.
#[cfg(target_os = "linux")]
pub fn spawn_monitor() -> MouseMonitor {
    let (tx, rx) = mpsc::channel();
    let status = Arc::new(Mutex::new(MonitorStatus::Unknown));
    let status_for_scan = Arc::clone(&status);
    let _ = std::thread::Builder::new()
        .name("rsclicker-mouse-scan".to_string())
        .spawn(move || {
            let mut known: HashMap<PathBuf, Arc<AtomicBool>> = HashMap::new();
            loop {
                let dir_readable = std::fs::read_dir("/dev/input").is_ok();
                let mut mice = 0usize;
                if dir_readable {
                    for (path, device) in evdev::enumerate() {
                        if !device_is_mouse(&device) {
                            continue;
                        }
                        mice += 1;
                        if let Some(alive) = reader_wanted(&mut known, &path) {
                            let tx = tx.clone();
                            let spawn_failed = Arc::clone(&alive);
                            let spawn = std::thread::Builder::new()
                                .name("rsclicker-evdev".to_string())
                                .spawn(move || {
                                    let mut device = device;
                                    loop {
                                        let events = match device.fetch_events() {
                                            Ok(events) => events,
                                            Err(_) => {
                                                alive.store(false, Ordering::SeqCst);
                                                return; // scanner respawns the reader
                                            }
                                        };
                                        for ev in events {
                                            if ev.event_type() != evdev::EventType::KEY {
                                                continue;
                                            }
                                            let ts = epoch_ms(ev.timestamp());
                                            let trigger = match code_to_trigger(ev.code()) {
                                                Some(t) => t,
                                                None => continue,
                                            };
                                            let msg = match ev.value() {
                                                1 => MouseEvent::Press(trigger, ts),
                                                0 => MouseEvent::Release(trigger, ts),
                                                _ => continue,
                                            };
                                            if tx.send(msg).is_err() {
                                                alive.store(false, Ordering::SeqCst);
                                                return;
                                            }
                                        }
                                    }
                                });
                            if spawn.is_err() {
                                spawn_failed.store(false, Ordering::SeqCst);
                            }
                        }
                    }
                }
                *status_for_scan.lock().unwrap_or_else(|e| e.into_inner()) = if !dir_readable {
                    MonitorStatus::Unavailable(
                        "cannot read /dev/input (add your user to the input group)".to_string(),
                    )
                } else if mice == 0 {
                    MonitorStatus::Unavailable(
                        "no readable mouse devices (add your user to the input group)"
                            .to_string(),
                    )
                } else {
                    MonitorStatus::Running(mice)
                };
                std::thread::sleep(std::time::Duration::from_secs(RESCAN_SECS));
            }
        });
    MouseMonitor { rx, status }
}

#[cfg(target_os = "windows")]
fn map_button(button: rdev::Button) -> Option<MouseTrigger> {
    match button {
        rdev::Button::Left => Some(MouseTrigger::Left),
        rdev::Button::Right => Some(MouseTrigger::Right),
        rdev::Button::Middle => Some(MouseTrigger::Middle),
        // XBUTTON1 / XBUTTON2 (side buttons).
        rdev::Button::Unknown(1) => Some(MouseTrigger::Side),
        rdev::Button::Unknown(2) => Some(MouseTrigger::Extra),
        rdev::Button::Unknown(_) => None,
    }
}

/// Windows 10/11 global mouse listener via a low-level hook (rdev). A hook
/// that cannot be installed is surfaced instead of failing silently.
#[cfg(target_os = "windows")]
pub fn spawn_monitor() -> MouseMonitor {
    let (tx, rx) = mpsc::channel();
    let status = Arc::new(Mutex::new(MonitorStatus::Unknown));
    let status_for_hook = Arc::clone(&status);
    let _ = std::thread::Builder::new()
        .name("rsclicker-rdev".to_string())
        .spawn(move || {
            let status = status_for_hook;
            *status.lock().unwrap_or_else(|p| p.into_inner()) = MonitorStatus::Running(1);
            match rdev::listen(move |event| {
                let ts = epoch_ms(event.time);
                let (trigger, pressed) = match event.event_type {
                    rdev::EventType::ButtonPress(b) => (map_button(b), true),
                    rdev::EventType::ButtonRelease(b) => (map_button(b), false),
                    _ => return,
                };
                if let Some(t) = trigger {
                    let _ = tx.send(if pressed {
                        MouseEvent::Press(t, ts)
                    } else {
                        MouseEvent::Release(t, ts)
                    });
                }
            }) {
                Ok(()) => {}
                Err(e) => {
                    let msg = format!("mouse hook failed: {e:?}");
                    *status.lock().unwrap_or_else(|p| p.into_inner()) =
                        MonitorStatus::Unavailable(msg);
                }
            }
        });
    MouseMonitor { rx, status }
}

/// Other platforms: mouse triggers unsupported — channel stays empty.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn spawn_monitor() -> MouseMonitor {
    let (_tx, rx) = mpsc::channel();
    let status = Arc::new(Mutex::new(MonitorStatus::Unavailable(
        "mouse triggers unsupported on this platform".to_string(),
    )));
    MouseMonitor { rx, status }
}

#[cfg(test)]
mod filter_tests {
    use super::{filter_injected_pairs, MouseEvent, MouseTrigger, PAIR_EPS_MS};

    /// (button, pressed) projection of a monitor batch, order preserved.
    fn project(events: &[MouseEvent]) -> Vec<(MouseTrigger, bool)> {
        events
            .iter()
            .map(|ev| match *ev {
                MouseEvent::Press(b, _) => (b, true),
                MouseEvent::Release(b, _) => (b, false),
            })
            .collect()
    }

    #[test]
    fn eps_bound_is_documented_size() {
        assert_eq!(PAIR_EPS_MS, 20);
    }

    #[test]
    fn instant_pair_dropped() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Side, 1_000),
            MouseEvent::Release(MouseTrigger::Side, 1_002),
        ]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn double_click_pairs_dropped() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Left, 1_000),
            MouseEvent::Release(MouseTrigger::Left, 1_001),
            MouseEvent::Press(MouseTrigger::Left, 1_010),
            MouseEvent::Release(MouseTrigger::Left, 1_012),
        ]);
        assert!(filtered.is_empty());
    }

    #[test]
    fn slow_pair_survives() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Left, 1_000),
            MouseEvent::Release(MouseTrigger::Left, 1_000 + super::PAIR_EPS_MS + 1),
        ]);
        assert_eq!(
            project(&filtered),
            vec![(MouseTrigger::Left, true), (MouseTrigger::Left, false)]
        );
    }

    #[test]
    fn release_before_press_never_pairs() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Release(MouseTrigger::Left, 1_000),
            MouseEvent::Press(MouseTrigger::Left, 1_003),
        ]);
        assert_eq!(
            project(&filtered),
            vec![(MouseTrigger::Left, false), (MouseTrigger::Left, true)]
        );
    }

    #[test]
    fn stale_long_press_not_broken_by_late_release() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Side, 1_000),
            MouseEvent::Release(MouseTrigger::Side, 3_000),
        ]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn unpaired_held_press_survives() {
        let filtered = filter_injected_pairs(vec![MouseEvent::Press(MouseTrigger::Extra, 1_000)]);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn buttons_independent_and_order_kept() {
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Extra, 1_000),
            MouseEvent::Press(MouseTrigger::Side, 1_001),
            MouseEvent::Release(MouseTrigger::Side, 1_002),
            MouseEvent::Release(MouseTrigger::Extra, 5_000),
        ]);
        assert_eq!(
            project(&filtered),
            vec![
                (MouseTrigger::Extra, true),
                (MouseTrigger::Extra, false),
            ]
        );
    }

    #[test]
    fn injected_pair_between_held_presses_does_not_pair_wrong_edges() {
        // Hardware setup: Extra held (user press, no release yet) while the
        // engine injects a Side pair; each button only pairs with itself.
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Extra, 1_000),
            MouseEvent::Press(MouseTrigger::Side, 1_500),
            MouseEvent::Release(MouseTrigger::Side, 1_501),
        ]);
        assert_eq!(
            project(&filtered),
            vec![(MouseTrigger::Extra, true)]
        );
    }

    #[test]
    fn second_pair_after_held_release() {
        // A long-standing press must not absorb a later fast pair: the
        // release first completes the slow pair (no match), the next pair
        // after a new press is still recognized.
        let filtered = filter_injected_pairs(vec![
            MouseEvent::Press(MouseTrigger::Side, 1_000),
            MouseEvent::Release(MouseTrigger::Side, 2_000),
            MouseEvent::Press(MouseTrigger::Side, 2_100),
            MouseEvent::Release(MouseTrigger::Side, 2_101),
        ]);
        assert_eq!(
            project(&filtered),
            vec![
                (MouseTrigger::Side, true),
                (MouseTrigger::Side, false),
            ]
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::reader_wanted;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn reader_respawn_lifecycle() {
        let mut known: HashMap<PathBuf, Arc<AtomicBool>> = HashMap::new();
        let path = Path::new("/dev/input/event7");

        let alive = reader_wanted(&mut known, path).expect("first visit spawns a reader");
        assert!(alive.load(Ordering::SeqCst));
        assert!(reader_wanted(&mut known, path).is_none());

        alive.store(false, Ordering::SeqCst);
        let respawned = reader_wanted(&mut known, path).expect("dead readers respawn");
        assert!(!Arc::ptr_eq(&alive, &respawned));
        assert!(respawned.load(Ordering::SeqCst));
        assert!(reader_wanted(&mut known, path).is_none());
    }
}
