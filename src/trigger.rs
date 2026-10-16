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

/// True when the armed snapshot still matches the profile as it is defined
/// right now: same trigger (for hotkeys, the same *parsed* hotkey — a
/// profile whose hotkey became unparseable never matches), same press mode
/// and the same click config. Used to decide whether saving an armed
/// profile needs a live resync.
pub fn armed_matches_profile(armed: &ArmedProfile, p: &Profile) -> bool {
    let trigger_matches = match &armed.trigger {
        EffectiveTrigger::Mouse { button } => p.mouse_trigger == Some(*button),
        EffectiveTrigger::Hotkey { id, .. } => {
            p.mouse_trigger.is_none()
                && parse_hotkey(&p.trigger_hotkey)
                    .map(|hotkey| hotkey.id() == *id)
                    .unwrap_or(false)
        }
    };
    trigger_matches
        && armed.press_mode == p.press_mode
        && match profile_config(p) {
            Ok(config) => config == armed.config,
            Err(_) => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clicker::MouseButtonKind;

    fn armed_for(p: &Profile) -> ArmedProfile {
        ArmedProfile {
            config: profile_config(p).unwrap(),
            trigger: match p.mouse_trigger {
                Some(button) => EffectiveTrigger::Mouse { button },
                None => {
                    let hotkey = parse_hotkey(&p.trigger_hotkey).unwrap();
                    EffectiveTrigger::Hotkey {
                        id: hotkey.id(),
                        key: hotkey,
                    }
                }
            },
            press_mode: p.press_mode,
        }
    }

    #[test]
    fn matching_snapshot_matches() {
        let p = Profile::default();
        let armed = armed_for(&p);
        assert!(armed_matches_profile(&armed, &p));
    }

    #[test]
    fn edits_break_the_match() {
        let base = Profile::default();
        let armed = armed_for(&base);

        let interval_edit = Profile {
            interval_ms: base.interval_ms + 1,
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &interval_edit));

        let hotkey_edit = Profile {
            trigger_hotkey: "F7".to_string(),
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &hotkey_edit));

        let press_edit = Profile {
            press_mode: PressMode::Hold,
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &press_edit));

        let button_edit = Profile {
            button: MouseButtonKind::Right,
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &button_edit));

        let trigger_button_edit = Profile {
            mouse_trigger: Some(MouseTrigger::Side),
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &trigger_button_edit));
    }

    #[test]
    fn unusable_states_never_match() {
        let base = Profile::default();
        let armed = armed_for(&base);

        let unparseable = Profile {
            trigger_hotkey: "not a hotkey!".to_string(),
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &unparseable));

        let empty_sequence = Profile {
            target: ClickTarget::Sequence(Vec::new()),
            ..base.clone()
        };
        assert!(!armed_matches_profile(&armed, &empty_sequence));
    }
}
