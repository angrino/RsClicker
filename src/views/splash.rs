use iced::widget::{column, container, mouse_area, progress_bar, row, text};
use iced::{color, Alignment, Background, Color, Element, Length};

use crate::app::{Message, RsClicker, SPLASH_TICKS, VERSION};
use crate::theme::{bold, brand_orange};

/// Animated brand intro (tap anywhere to skip): logo fades in, version
/// follows, then a progress bar fills until the main screen takes over.
pub fn splash_view(state: &RsClicker) -> Element<'_, Message> {
    let tick = state.splash_tick as f32;
    let total = SPLASH_TICKS as f32;
    let progress = (tick / total).min(1.0);
    let logo_fade = (tick / 10.0).min(1.0);
    let version_fade = ((tick - 8.0) / 10.0).clamp(0.0, 1.0);

    let logo = row![
        text("Rs")
            .size(64)
            .font(bold())
            .color(Color {
                a: logo_fade,
                ..brand_orange()
            }),
        text("Clicker")
            .size(64)
            .font(bold())
            .color(Color {
                a: logo_fade,
                ..Color::WHITE
            }),
    ]
    .spacing(0)
    .align_y(Alignment::Center);

    let version = text(VERSION)
        .size(14)
        .color(Color {
            a: 0.55 * version_fade,
            ..Color::WHITE
        })
        .center();

    let bar_wrap = if tick > 4.0 {
        container(progress_bar(0.0..=1.0, progress).length(Length::Fixed(220.0)))
            .width(Length::Fill)
            .align_x(Alignment::Center)
    } else {
        container(row![])
            .width(Length::Fill)
            .height(Length::Fixed(28.0))
    };

    let hint = text("click anywhere to skip")
        .size(12)
        .color(Color {
            a: 0.35 * version_fade,
            ..Color::WHITE
        })
        .center();

    let content = column![logo, version, bar_wrap, hint]
        .spacing(12)
        .align_x(Alignment::Center);

    mouse_area(
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(color!(0x0E0C0A))),
                ..Default::default()
            }),
    )
    .on_press(Message::SkipSplash)
    .into()
}
