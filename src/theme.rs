//! Theme system: Light/Dark base x 8 accent colors, plus shared styles.
//!
//! The accent color drives `palette.primary`, so default iced buttons,
//! progress bars and selections follow it automatically.

use serde::{Deserialize, Serialize};

use iced::border::Radius;
use iced::theme::Palette;
use iced::widget::{button, container};
use iced::{color, Background, Border, Color, Font, Theme};
use iced::font::Weight;

/// Window base: white background or black background.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BaseTheme {
    #[default]
    Dark,
    Light,
}

impl BaseTheme {
    pub fn label(self) -> &'static str {
        match self {
            BaseTheme::Dark => "Dark",
            BaseTheme::Light => "Light",
        }
    }
}

/// Accent color applied on top of the base theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Accent {
    #[default]
    Orange,
    Aqua,
    Green,
    Purple,
    Red,
    Yellow,
    Pink,
    Blue,
}

impl Accent {
    pub fn all() -> &'static [Accent] {
        &[
            Accent::Orange,
            Accent::Aqua,
            Accent::Green,
            Accent::Purple,
            Accent::Red,
            Accent::Yellow,
            Accent::Pink,
            Accent::Blue,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Accent::Orange => "Orange",
            Accent::Aqua => "Aqua",
            Accent::Green => "Green",
            Accent::Purple => "Purple",
            Accent::Red => "Red",
            Accent::Yellow => "Yellow",
            Accent::Pink => "Pink",
            Accent::Blue => "Blue",
        }
    }

    /// (dark-theme primary, light-theme primary). Light variants are darkened
    /// for contrast on white backgrounds.
    fn primaries(self) -> (Color, Color) {
        match self {
            Accent::Orange => (color!(0xF56A00), color!(0xE85D04)),
            Accent::Aqua => (color!(0x22D3EE), color!(0x0097A7)),
            Accent::Green => (color!(0x2FA36B), color!(0x1E7A4F)),
            Accent::Purple => (color!(0x9B5DE5), color!(0x7C3AED)),
            Accent::Red => (color!(0xE5484D), color!(0xC93A2E)),
            Accent::Yellow => (color!(0xE8A13C), color!(0xB26A00)),
            Accent::Pink => (color!(0xF15BB5), color!(0xD63384)),
            Accent::Blue => (color!(0x3E8EDE), color!(0x2563EB)),
        }
    }

    pub fn primary(self, base: BaseTheme) -> Color {
        let (dark, light) = self.primaries();
        match base {
            BaseTheme::Dark => dark,
            BaseTheme::Light => light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppTheme {
    pub base: BaseTheme,
    pub accent: Accent,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            base: BaseTheme::Dark,
            accent: Accent::Orange,
        }
    }
}

impl AppTheme {
    fn palette(self) -> Palette {
        let (background, text) = match self.base {
            BaseTheme::Dark => (color!(0x141210), color!(0xF5F1E8)),
            BaseTheme::Light => (Color::WHITE, color!(0x1E1B16)),
        };
        Palette {
            background,
            text,
            primary: self.accent.primary(self.base),
            success: color!(0x2FA36B),
            warning: color!(0xE8A13C),
            danger: color!(0xD6493B),
        }
    }

    pub fn theme(self) -> Theme {
        let name = match self.base {
            BaseTheme::Dark => "RsClicker Dark",
            BaseTheme::Light => "RsClicker Light",
        };
        Theme::custom(name, self.palette())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ViewMode {    #[default]
    List,
    Icons,
}

impl ViewMode {
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::List => "List",
            ViewMode::Icons => "Icons",
        }
    }
}

// ---------------------------------------------------------------------------
// Shared widget styles
// ---------------------------------------------------------------------------

/// The fixed RsClicker brand orange (Rust-logo homage), independent of accent.
pub fn brand_orange() -> Color {
    color!(0xF56A00)
}

pub fn bold() -> Font {
    Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    }
}

/// Primary (accent) circular button, e.g. the "+" add button.
pub fn circle_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::primary(theme, status);
    let primary = theme.palette().primary;
    let bg = match status {
        button::Status::Hovered => Color {
            a: 0.88,
            ..primary
        },
        button::Status::Pressed => Color {
            r: primary.r * 0.8,
            g: primary.g * 0.8,
            b: primary.b * 0.8,
            a: primary.a,
        },
        _ => primary,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: Radius::from(28.0),
            ..base.border
        },
        ..base
    }
}

pub fn small_circle_button(theme: &Theme, status: button::Status) -> button::Style {
    let mut style = circle_button(theme, status);
    style.border.radius = Radius::from(20.0);
    style
}

/// Circular button with an explicit background color (run-state colors).
pub fn state_circle_button(
    bg: Color,
    radius: f32,
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let base = button::primary(theme, status);
    let bg = match status {
        button::Status::Hovered => Color { a: 0.85, ..bg },
        _ => bg,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            radius: Radius::from(radius),
            ..base.border
        },
        ..base
    }
}

pub fn card_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            theme.extended_palette().background.weak.color,
        )),
        border: Border {
            radius: Radius::from(14.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn dialog_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette().background)),
        border: Border {
            radius: Radius::from(16.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn avatar_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(theme.palette().primary)),
        border: Border {
            radius: Radius::from(26.0),
            ..Default::default()
        },
        ..Default::default()
    }
}
