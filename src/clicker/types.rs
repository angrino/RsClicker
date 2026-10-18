use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(target_os = "linux")]
use evdev::uinput::VirtualDevice;
#[cfg(target_os = "linux")]
use evdev::{AttributeSet, KeyCode, KeyEvent, RelativeAxisCode, RelativeAxisEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MouseButtonKind {
    #[default]
    Left,
    Right,
    Middle,
}

impl MouseButtonKind {
    pub fn all() -> &'static [MouseButtonKind] {
        &[MouseButtonKind::Left, MouseButtonKind::Right, MouseButtonKind::Middle]
    }
}

impl From<MouseButtonKind> for Button {
    fn from(value: MouseButtonKind) -> Self {
        match value {
            MouseButtonKind::Left => Button::Left,
            MouseButtonKind::Right => Button::Right,
            MouseButtonKind::Middle => Button::Middle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClickKind {
    #[default]
    Single,
    Double,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickTarget {
    CurrentPosition,
    Fixed { x: i32, y: i32 },
    Sequence(Vec<(i32, i32)>),
}

impl Default for ClickTarget {
    fn default() -> Self {
        ClickTarget::CurrentPosition
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickConfig {
    pub interval_ms: u64,
    pub button: MouseButtonKind,
    pub kind: ClickKind,
    pub target: ClickTarget,
}

impl Default for ClickConfig {
    fn default() -> Self {
        Self {
            interval_ms: 100,
            button: MouseButtonKind::Left,
            kind: ClickKind::Single,
            target: ClickTarget::CurrentPosition,
        }
    }
}

/// Upper bound for a configured click interval (1 hour). The UI rejects
/// larger values and the engine clamps anyway — a stored profile can never
/// carry an absurd interval.
pub const MAX_INTERVAL_MS: u64 = 3_600_000;

/// Inter-click gap inside one double click. A strict zero gap can merge
/// both clicks into one press event on X11/Wayland/Win32; a human-like gap
/// is recognized as a double click everywhere.
pub const DOUBLE_CLICK_GAP_MS: u64 = 30;

impl ClickConfig {
    pub fn sanitized_interval_ms(&self) -> u64 {
        self.interval_ms.clamp(1, MAX_INTERVAL_MS)
    }
}

/// Which injection backend a click engine uses. `Uinput` exists on Linux
/// only — a kernel-level virtual mouse that needs no portals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionBackend {
    Enigo,
    #[cfg(target_os = "linux")]
    Uinput,
}

/// Backend instance owned by one worker thread. Dropping the value tears
/// the backend down (uinput destroys its virtual device).
pub trait ClickBackend {
    /// One click (move + press [+ press]). `Err` = injection failed.
    fn click(&mut self, config: &ClickConfig, seq_index: usize) -> Result<(), String>;
}

/// Resolves where a click lands: `None` keeps the current cursor position.
fn target_point(config: &ClickConfig, seq_index: usize) -> Result<Option<(i32, i32)>, String> {
    match &config.target {
        ClickTarget::Fixed { x, y } => Ok(Some((*x, *y))),
        ClickTarget::Sequence(points) => {
            let (x, y) = points
                .get(seq_index % points.len().max(1))
                .ok_or_else(|| "empty sequence".to_string())?;
            Ok(Some((*x, *y)))
        }
        ClickTarget::CurrentPosition => Ok(None),
    }
}

/// Deltas steering a REL pointer from any on-screen position to an exact
/// absolute target: the oversized negative move clamps at the top-left
/// screen corner, then the exact offset applies. Bound: no virtual screen
/// is wider than 2 × 32767 px, so one clamp step always suffices.
#[cfg(target_os = "linux")]
pub fn abs_move_deltas(x: i32, y: i32) -> [(i32, i32); 2] {
    [(-32_767, -32_767), (x, y)]
}

#[cfg(test)]
mod backend_tests {
    #[allow(unused_imports)]
    use super::{abs_move_deltas, ClickTarget, ClickConfig, MouseButtonKind};

    /// Simulates a pointer with edge clamping on a screen of the given
    /// size and follows the deltas.
    #[cfg(target_os = "linux")]
    fn apply(from: (i32, i32), deltas: [(i32, i32); 2], w: i32, h: i32) -> (i32, i32) {
        let (mut x, mut y) = from;
        for (dx, dy) in deltas {
            x = (x + dx).clamp(0, w - 1);
            y = (y + dy).clamp(0, h - 1);
        }
        (x, y)
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn corner_clamp_reaches_exact_target() {
        for (x, y) in [(0, 0), (100, 40), (7999, 5999), (30720, 21600)] {
            let from = (7999, 5999);
            let deltas = abs_move_deltas(x, y);
            let big = (32_000, 32_000);
            assert_eq!(apply(from, deltas, big.0, big.1), (x, y));
            let full_hd = (1920, 1080);
            let (tx, ty) = (x.min(full_hd.0 - 1), y.min(full_hd.1 - 1));
            assert_eq!(apply(from, deltas, full_hd.0, full_hd.1), (tx, ty));
        }
    }

    #[test]
    fn sequence_target_resolution_uses_seq_index() {
        use super::target_point;
        let pts = vec![(5, 6), (7, 8), (9, 10)];
        let config = ClickConfig {
            target: ClickTarget::Sequence(pts),
            button: MouseButtonKind::Left,
            ..ClickConfig::default()
        };
        assert_eq!(target_point(&config, 0).unwrap(), Some((5, 6)));
        assert_eq!(target_point(&config, 2).unwrap(), Some((9, 10)));
        assert_eq!(target_point(&config, 3).unwrap(), Some((5, 6)));
    }
}

pub struct EnigoBackend {
    enigo: Enigo,
}

impl EnigoBackend {
    /// Panics internally when no backend exists (the libei portal path) —
    /// the engine wraps this in `catch_unwind`.
    pub fn new() -> Option<Self> {
        Enigo::new(&Settings::default()).ok().map(|enigo| Self { enigo })
    }
}

impl ClickBackend for EnigoBackend {
    fn click(&mut self, config: &ClickConfig, seq_index: usize) -> Result<(), String> {
        if let Some((x, y)) = target_point(config, seq_index)? {
            self.enigo
                .move_mouse(x, y, Coordinate::Abs)
                .map_err(|e| e.to_string())?;
        }
        click_enigo(&mut self.enigo, config)
    }
}

fn click_enigo(enigo: &mut Enigo, config: &ClickConfig) -> Result<(), String> {
    let button: Button = config.button.into();
    enigo
        .button(button, Direction::Click)
        .map_err(|e| e.to_string())?;
    if config.kind == ClickKind::Double {
        std::thread::sleep(Duration::from_millis(DOUBLE_CLICK_GAP_MS));
        enigo
            .button(button, Direction::Click)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Kernel-level virtual mouse (evdev uinput). Clicks go straight to the
/// compositor — no X11 server and no desktop portals involved. Needs write
/// access to `/dev/uinput` (the `uinput` group).
#[cfg(target_os = "linux")]
pub struct UinputBackend {
    device: VirtualDevice,
}

#[cfg(target_os = "linux")]
impl UinputBackend {
    pub fn new() -> Result<Self, String> {
        let io_err = |e: std::io::Error| e.to_string();
        let mut keys: AttributeSet<KeyCode> = AttributeSet::new();
        keys.insert(KeyCode::BTN_LEFT);
        keys.insert(KeyCode::BTN_RIGHT);
        keys.insert(KeyCode::BTN_MIDDLE);
        let mut rel: AttributeSet<RelativeAxisCode> = AttributeSet::new();
        rel.insert(RelativeAxisCode::REL_X);
        rel.insert(RelativeAxisCode::REL_Y);
        let device = VirtualDevice::builder()
            .map_err(io_err)?
            .name("rsclicker")
            .with_keys(&keys)
            .map_err(io_err)?
            .with_relative_axes(&rel)
            .map_err(io_err)?
            .build()
            .map_err(io_err)?;
        Ok(Self { device })
    }

    fn button_code(button: MouseButtonKind) -> KeyCode {
        match button {
            MouseButtonKind::Left => KeyCode::BTN_LEFT,
            MouseButtonKind::Right => KeyCode::BTN_RIGHT,
            MouseButtonKind::Middle => KeyCode::BTN_MIDDLE,
        }
    }

    fn touch(&mut self, button: KeyCode, down: bool) -> Result<(), String> {
        let press = KeyEvent::new(button, if down { 1 } else { 0 });
        self.device
            .emit(&[press.into()])
            .map_err(|e| e.to_string())
    }

    fn move_to(&mut self, x: i32, y: i32) -> Result<(), String> {
        for (dx, dy) in abs_move_deltas(x, y) {
            let batch = [
                RelativeAxisEvent::new(RelativeAxisCode::REL_X, dx).into(),
                RelativeAxisEvent::new(RelativeAxisCode::REL_Y, dy).into(),
            ];
            self.device.emit(&batch).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
impl ClickBackend for UinputBackend {
    fn click(&mut self, config: &ClickConfig, seq_index: usize) -> Result<(), String> {
        if let Some((x, y)) = target_point(config, seq_index)? {
            self.move_to(x, y)?;
        }
        let button = Self::button_code(config.button);
        self.touch(button, true)?;
        self.touch(button, false)?;
        if config.kind == ClickKind::Double {
            std::thread::sleep(Duration::from_millis(DOUBLE_CLICK_GAP_MS));
            self.touch(button, true)?;
            self.touch(button, false)?;
        }
        Ok(())
    }
}
