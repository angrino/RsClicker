//! UI views: splash intro, main library, editor modal, dialogs.
//!
//! Pure rendering + shared widgets. State lives in `crate::app`.

pub mod dialogs;
pub mod editor;
pub mod main;
pub mod splash;

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Background, Color, Element, Length};

use crate::app::Message;
use crate::theme::{card_style, dialog_style};

pub fn brand(size: f32, client_color: Option<Color>) -> Element<'static, Message> {
    use crate::theme::bold;
    use crate::theme::brand_orange;
    let rs = text("Rs").size(size).font(bold()).color(brand_orange());
    let client = match client_color {
        Some(c) => text("Clicker").size(size).font(bold()).color(c),
        None => text("Clicker").size(size).font(bold()),
    };
    row![rs, client]
        .spacing(0)
        .align_y(Alignment::Center)
        .into()
}

pub fn section<'a>(title: &'a str, content: Element<'a, Message>) -> Element<'a, Message> {
    container(column![text(title).size(15).font(crate::theme::bold()), content].spacing(10))
        .padding(14)
        .width(Length::Fill)
        .style(card_style)
        .into()
}

pub fn option_button(
    label: &'static str,
    selected: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let style_fn = if selected {
        button::primary
    } else {
        button::secondary
    };
    button(text(label).center())
        .width(Length::Fill)
        .on_press(on_press)
        .style(style_fn)
        .into()
}

/// Full-window dimmed backdrop centering a small dialog card.
pub fn backdrop(alpha: f32, dialog: Element<'_, Message>) -> Element<'_, Message> {
    container(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: alpha, ..Color::BLACK })),
            ..Default::default()
        })
        .into()
}

pub fn dialog_card(content: Element<'_, Message>, width: f32) -> Element<'_, Message> {
    container(content)
        .width(Length::Fixed(width))
        .padding(20)
        .style(dialog_style)
        .into()
}

pub fn avatar_letter(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}
