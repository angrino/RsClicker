//! Trigger model: keyboard hotkeys and mouse buttons, capture logic,
//! armed-profile routing helpers.

use global_hotkey::hotkey::HotKey;
use iced::keyboard;
use iced::keyboard::key::{Key, Named};

use crate::clicker::{ClickConfig, ClickTarget};
use crate::hotkey::parse_hotkey;
use crate::mouse::MouseTrigger;
use crate::profile::{PressMode, Profile};

/// Per-profile run-button state, following the user's color scheme:
/// IDLE gray / LISTENING orange / RUNNING green / ERROR red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunState {
    #[default]
    Idle,
    Listening,
    Running,
    Error,
}

/// The effective trigger of a profile: a mouse button when set, otherwise
/// the keyboard hotkey (kept with its id for routing and unregistering).
#[derive(Debug, Clone)]
pub enum EffectiveTrigger {
    Hotkey { id: u32, key: HotKey },
    Mouse { button: MouseTrigger },
}

#[derive(Debug, Clone)]
pub struct ArmedProfile {
    pub config: ClickConfig,
    pub trigger: EffectiveTrigger,
    pub press_mode: PressMode,
}

pub fn profile_config(p: &Profile) -> Result<ClickConfig, &'static str> {
    let mut target = p.target.clone();
    if let ClickTarget::Sequence(_) = target {
        let pts: Vec<(i32, i32)> = p.points.iter().map(|pt| (pt.x, pt.y)).collect();
        if pts.is_empty() {
            return Err("Sequence is empty, add points first");
        }
        target = ClickTarget::Sequence(pts);
    }
    Ok(ClickConfig {
        interval_ms: p.interval_ms.max(1),
        button: p.button,
        kind: p.kind,
        target,
    })
}

/// Human-readable trigger, e.g. "F6" or "Mouse Side".
pub fn trigger_label(p: &Profile) -> String {
    match p.mouse_trigger {
        Some(m) => m.to_string(),
        None => p.trigger_hotkey.clone(),
    }
}

pub enum Capture {
    Hotkey(String),
    Mouse(MouseTrigger),
    Cancel,
    Unsupported,
}

/// Turns a physical key press (+ held modifiers) into a global-hotkey string.
/// `Escape` alone cancels the capture instead of being recorded.
pub fn capture_hotkey(key: &Key, modifiers: &keyboard::Modifiers) -> Capture {
    let mut parts: Vec<&str> = Vec::new();
    if modifiers.control() {
        parts.push("Control");
    }
    if modifiers.shift() {
        parts.push("Shift");
    }
    if modifiers.alt() {
        parts.push("Alt");
    }
    if modifiers.logo() {
        parts.push("Super");
    }
    let main = match key {
        Key::Named(named) => {
            if matches!(named, Named::Escape) && parts.is_empty() {
                return Capture::Cancel;
            }
            format!("{named:?}")
        }
        Key::Character(c) => c.to_uppercase(),
        Key::Unidentified => return Capture::Unsupported,
    };
    let text = if parts.is_empty() {
        main
    } else {
        format!("{}+{main}", parts.join("+"))
    };
    match parse_hotkey(&text) {
        Ok(_) => Capture::Hotkey(text),
        Err(_) => Capture::Unsupported,
    }
}
