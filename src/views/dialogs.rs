//! Small modal dialogs: new profile, app settings, about, license, capture.

use iced::widget::{button, column, row, scrollable, text, text_input, space};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};
use iced::border::Radius;

use crate::app::{Message, RsClicker, REPO, VERSION};
use crate::theme::{Accent, BaseTheme, ViewMode};

use super::{dialog_card, option_button};
use crate::trigger::trigger_label;

const LICENSE_TEXT: &str = include_str!("../../LICENSE");

pub fn new_profile_dialog(state: &RsClicker) -> Element<'_, Message> {
    let mut content = column![
        text("New Profile").size(20).font(crate::theme::bold()).center(),
        text_input("Profile name", &state.new_profile_name)
            .on_input(Message::NewProfileNameChanged)
            .on_submit(Message::ConfirmNewProfile),
        row![
            button("Cancel")
                .width(Length::Fill)
                .on_press(Message::CloseModal)
                .style(button::secondary),
            button("Confirm")
                .width(Length::Fill)
                .on_press(Message::ConfirmNewProfile)
                .style(button::primary),
        ]
        .spacing(10),
    ]
    .spacing(14);
    if !state.dialog_error.is_empty() {
        content = content.push(text(state.dialog_error.as_str()).size(13).center());
    }
    dialog_card(content.into(), 320.0)
}

fn accent_swatch(accent: Accent, base: BaseTheme, selected: bool) -> Element<'static, Message> {
    let color = accent.primary(base);
    let border = if selected {
        Border {
            color: Color::WHITE,
            width: 2.0,
            radius: Radius::from(18.0),
        }
    } else {
        Border {
            radius: Radius::from(18.0),
            ..Default::default()
        }
    };
    button(text("").center())
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .on_press(Message::SetAccent(accent))
        .style(move |theme: &Theme, status: button::Status| {
            let base_style = button::primary(theme, status);
            button::Style {
                background: Some(Background::Color(color)),
                text_color: Color::WHITE,
                border,
                ..base_style
            }
        })
        .into()
}

pub fn app_settings_dialog(state: &RsClicker) -> Element<'_, Message> {
    let pending = state.pending_settings;
    let theme_row = row![
        option_button(
            BaseTheme::Light.label(),
            pending.base == BaseTheme::Light,
            Message::SetTheme(BaseTheme::Light)
        ),
        option_button(
            BaseTheme::Dark.label(),
            pending.base == BaseTheme::Dark,
            Message::SetTheme(BaseTheme::Dark)
        ),
    ]
    .spacing(10);

    let mut accent_grid = column![].spacing(8);
    for chunk in Accent::all().chunks(4) {
        let mut r = row![].spacing(8);
        for accent in chunk {
            r = r.push(accent_swatch(*accent, pending.base, pending.accent == *accent));
        }
        accent_grid = accent_grid.push(r);
    }

    let view_row = row![
        option_button(
            ViewMode::List.label(),
            pending.view == ViewMode::List,
            Message::SetViewMode(ViewMode::List)
        ),
        option_button(
            ViewMode::Icons.label(),
            pending.view == ViewMode::Icons,
            Message::SetViewMode(ViewMode::Icons)
        ),
    ]
    .spacing(10);

    #[cfg(target_os = "linux")]
    let input_row = row![
        option_button(
            crate::settings_store::InputBackend::Enigo.label(),
            pending.input_backend == crate::settings_store::InputBackend::Enigo,
            Message::SetInputBackend(crate::settings_store::InputBackend::Enigo)
        ),
        option_button(
            crate::settings_store::InputBackend::Uinput.label(),
            pending.input_backend == crate::settings_store::InputBackend::Uinput,
            Message::SetInputBackend(crate::settings_store::InputBackend::Uinput)
        ),
    ]
    .spacing(10);

    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut content = column![
        text("Settings").size(20).font(crate::theme::bold()).center(),
        text("Theme").size(14).font(crate::theme::bold()),
        theme_row,
        text(format!("Accent — {}", pending.accent.label()))
            .size(14)
            .font(crate::theme::bold()),
        accent_grid,
        text("Profiles view").size(14).font(crate::theme::bold()),
        view_row,
    ];

    #[cfg(target_os = "linux")]
    {
        content = content
            .push(text("Input emulation").size(14).font(crate::theme::bold()))
            .push(input_row)
            .push(
                text("uinput clicks straight through the kernel (needs the 'uinput' \
                      group); enigo uses X11 or the desktop portals.")
                    .size(12),
            );
    }

    let content = content
        .push(
            button(text("About RsClicker").center())
                .width(Length::Fill)
                .on_press(Message::OpenAbout)
                .style(button::secondary),
        )
        .push(
            row![
                button("Cancel")
                    .width(Length::Fill)
                    .on_press(Message::CloseModal)
                    .style(button::secondary),
                button("Apply")
                    .width(Length::Fill)
                    .on_press(Message::ApplySettings)
                    .style(button::primary),
            ]
            .spacing(10),
        )
        .spacing(12);

    dialog_card(content.into(), 340.0)
}

pub fn about_dialog() -> Element<'static, Message> {
    let content = column![
        text("About RsClicker").size(20).font(crate::theme::bold()).center(),
        text(format!(
            "RsClicker v{VERSION}\n\nA cross-platform auto clicker written in Rust. \
             Create profiles, arm them with a hotkey or mouse button, and fire \
             precise click sequences anywhere.\n\nRepository:\n{REPO}"
        ))
        .size(13),
        row![
            button(text("License").center())
                .width(Length::Fill)
                .on_press(Message::OpenLicense)
                .style(button::secondary),
            button(text("Close").center())
                .width(Length::Fill)
                .on_press(Message::CloseModal)
                .style(button::primary),
        ]
        .spacing(10),
    ]
    .spacing(14);
    dialog_card(content.into(), 340.0)
}

pub fn license_dialog() -> Element<'static, Message> {
    let content = column![
        text("MIT License").size(20).font(crate::theme::bold()).center(),
        scrollable(text(LICENSE_TEXT).size(12)).height(Length::Fixed(360.0)),
        button(text("Close").center())
            .width(Length::Fill)
            .on_press(Message::CloseModal)
            .style(button::primary),
    ]
    .spacing(14);
    dialog_card(content.into(), 360.0)
}

pub fn capture_dialog(state: &RsClicker) -> Element<'_, Message> {
    let content = column![
        text("Capture trigger").size(20).font(crate::theme::bold()).center(),
        text("Press the key you want to assign")
            .size(14)
            .center(),
        text("Keyboard and mouse are listened together — use either.").size(12).center(),
        text(format!("Current: {}", trigger_label(&state.profile)))
            .size(13)
            .center(),
        row![space::horizontal(), button("Cancel")
            .on_press(Message::CloseModal)
            .style(button::secondary), space::horizontal(),]
        .spacing(10),
    ]
    .spacing(14)
    .align_x(Alignment::Center);
    dialog_card(content.into(), 320.0)
}
