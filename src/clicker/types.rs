use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};
use serde::{Deserialize, Serialize};

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

impl ClickConfig {
    pub fn sanitized_interval_ms(&self) -> u64 {
        self.interval_ms.clamp(1, 3_600_000)
    }
}

pub fn perform_single_click(enigo: &mut Enigo, config: &ClickConfig, seq_index: usize) {
    match &config.target {
        ClickTarget::Fixed { x, y } => {
            let _ = enigo.move_mouse(*x, *y, Coordinate::Abs);
        }
        ClickTarget::Sequence(points) => {
            if let Some((x, y)) = points.get(seq_index % points.len().max(1)) {
                let _ = enigo.move_mouse(*x, *y, Coordinate::Abs);
            }
        }
        ClickTarget::CurrentPosition => {}
    }

    let button: Button = config.button.into();
    let _ = enigo.button(button, Direction::Click);
    if config.kind == ClickKind::Double {
        let _ = enigo.button(button, Direction::Click);
    }
}

pub fn create_enigo() -> Option<Enigo> {
    Enigo::new(&Settings::default()).ok()
}
