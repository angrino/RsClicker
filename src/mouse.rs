//! Global mouse-button monitoring for mouse triggers.
//!
//! - Linux: reads `/dev/input/event*` via evdev (no grab — input is only
//!   observed, never stolen). Needs read permission, i.e. membership in the
//!   `input` group.
//! - Windows 10/11: low-level mouse hook via rdev (no extra setup).
//! - Other platforms: stub that never yields events.

use std::sync::mpsc::{self, Receiver};

use serde::{Deserialize, Serialize};

/// A mouse button usable as a profile trigger, including side buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy)]
pub enum MouseEvent {
    Press(MouseTrigger),
    Release(MouseTrigger),
}

#[cfg(target_os = "linux")]
fn code_to_trigger(code: u16) -> Option<MouseTrigger> {
    // Linux input-event-codes.h button codes.
    match code {
        0x110 => Some(MouseTrigger::Left),   // BTN_LEFT
        0x111 => Some(MouseTrigger::Right),  // BTN_RIGHT
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

/// Spawns background reader threads (one per mouse device) and returns the
/// receiving end. Never grabs devices.
#[cfg(target_os = "linux")]
pub fn spawn_monitor() -> Receiver<MouseEvent> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("rsclicker-evdev-enum".to_string())
        .spawn(move || {
            for (_path, device) in evdev::enumerate() {
                if !device_is_mouse(&device) {
                    continue;
                }
                let tx = tx.clone();
                let _ = std::thread::Builder::new()
                    .name("rsclicker-evdev".to_string())
                    .spawn(move || {
                        let mut device = device;
                        loop {
                            let events = match device.fetch_events() {
                                Ok(events) => events,
                                Err(_) => return, // device unplugged
                            };
                            for ev in events {
                                if ev.event_type() != evdev::EventType::KEY {
                                    continue;
                                }
                                let trigger = match code_to_trigger(ev.code()) {
                                    Some(t) => t,
                                    None => continue,
                                };
                                let msg = match ev.value() {
                                    1 => MouseEvent::Press(trigger),
                                    0 => MouseEvent::Release(trigger),
                                    _ => continue,
                                };
                                if tx.send(msg).is_err() {
                                    return;
                                }
                            }
                        }
                    });
            }
        });
    rx
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

/// Windows 10/11 global mouse listener via a low-level hook (rdev).
#[cfg(target_os = "windows")]
pub fn spawn_monitor() -> Receiver<MouseEvent> {
    let (tx, rx) = mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("rsclicker-rdev".to_string())
        .spawn(move || {
        let tx = tx;
        let _ = rdev::listen(move |event| {
            let (trigger, pressed) = match event.event_type {
                rdev::EventType::ButtonPress(b) => (map_button(b), true),
                rdev::EventType::ButtonRelease(b) => (map_button(b), false),
                _ => return,
            };
            if let Some(t) = trigger {
                let _ = tx.send(if pressed {
                    MouseEvent::Press(t)
                } else {
                    MouseEvent::Release(t)
                });
            }
        });
    });
    rx
}

/// Other platforms: mouse triggers unsupported — channel stays empty.
#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn spawn_monitor() -> Receiver<MouseEvent> {
    let (_tx, rx) = mpsc::channel();
    rx
}
