use serde::{Deserialize, Serialize};

use crate::clicker::{ClickKind, ClickTarget, MouseButtonKind};
use crate::mouse::MouseTrigger;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickPoint {
    pub x: i32,
    pub y: i32,
}

/// How the trigger fires the profile event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PressMode {
    /// Press once to run, press again to stop.
    #[default]
    Toggle,
    /// Runs while the trigger is held, stops on release.
    Hold,
}

impl PressMode {
    pub fn label(self) -> &'static str {
        match self {
            PressMode::Toggle => "Toggle",
            PressMode::Hold => "Hold",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PressMode::Toggle => "Press once to run, press again to stop.",
            PressMode::Hold => "Runs while held, stops on release.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub interval_ms: u64,
    pub button: MouseButtonKind,
    pub kind: ClickKind,
    pub target: ClickTarget,
    /// Keyboard trigger (e.g. "F6", "Control+R"). Renamed from
    /// `toggle_hotkey`; the serde alias keeps old profile files loadable.
    #[serde(alias = "toggle_hotkey")]
    pub trigger_hotkey: String,
    /// Mouse-button trigger. When `Some`, it takes precedence over the
    /// keyboard hotkey.
    #[serde(default)]
    pub mouse_trigger: Option<MouseTrigger>,
    #[serde(default)]
    pub press_mode: PressMode,
    pub points: Vec<ClickPoint>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            interval_ms: 100,
            button: MouseButtonKind::Left,
            kind: ClickKind::Single,
            target: ClickTarget::CurrentPosition,
            trigger_hotkey: "F6".to_string(),
            mouse_trigger: None,
            press_mode: PressMode::Toggle,
            points: Vec::new(),
        }
    }
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Self::default()
        }
    }
}

/// Interval unit for the Timing editor (e.g. "2 ms", "0.7 s").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Ms,
    S,
    Min,
}

impl std::fmt::Display for TimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TimeUnit::Ms => "ms",
            TimeUnit::S => "s",
            TimeUnit::Min => "min",
        })
    }
}

impl TimeUnit {
    pub fn mult(self) -> u64 {
        match self {
            TimeUnit::Ms => 1,
            TimeUnit::S => 1_000,
            TimeUnit::Min => 60_000,
        }
    }

    pub fn all() -> &'static [TimeUnit] {
        &[TimeUnit::Ms, TimeUnit::S, TimeUnit::Min]
    }
}

/// Splits stored milliseconds into a friendly (value, unit) pair.
pub fn split_interval(ms: u64) -> (String, TimeUnit) {
    if ms % 60_000 == 0 && ms > 0 {
        ((ms / 60_000).to_string(), TimeUnit::Min)
    } else if ms % 1_000 == 0 && ms > 0 {
        ((ms / 1_000).to_string(), TimeUnit::S)
    } else {
        (ms.to_string(), TimeUnit::Ms)
    }
}
