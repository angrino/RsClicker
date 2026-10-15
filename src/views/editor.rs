//! Profile editor rendered as a modal card (same style as settings).

use iced::widget::{button, column, row, scrollable, text, text_input, pick_list, space};
use iced::{Alignment, Element, Length};

use crate::app::Message;
use crate::clicker::{ClickKind, ClickTarget, MouseButtonKind};
use crate::profile::{PressMode, TimeUnit};
use crate::trigger::trigger_label;

use super::{option_button, section};
use crate::app::RsClicker;
use crate::theme::bold;

fn mode_card(
    title: &'static str,
    desc: &'static str,
    selected: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let inner = column![
        text(title).size(15).font(bold()).center(),
        text(desc).size(12).center(),
    ]
    .spacing(4)
    .align_x(Alignment::Center);
    button(inner)
        .width(Length::Fill)
        .on_press(on_press)
        .style(if selected {
            button::primary
        } else {
            button::secondary
        })
        .into()
}

/// The editor card content (wrapped in a backdrop by `app::view`).
pub fn editor_card(state: &RsClicker) -> Element<'_, Message> {
    let running = state.editor_running();

    let header = row![
        button(text("← Back").size(15))
            .on_press(Message::CloseModal)
            .style(button::secondary),
        text(state.profile.name.as_str()).size(20).font(bold()),
        space::horizontal(),
        text(if running { "● Running" } else { "○ Idle" }).size(14),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let profile_card = section(
        "Profile",
        text_input("profile name", &state.profile.name)
            .on_input(Message::ProfileNameChanged)
            .into(),
    );

    let trigger_card = section(
        "Trigger",
        column![
            row![
                text_input("hotkey e.g. F6", &state.profile.trigger_hotkey)
                    .on_input(Message::HotkeyChanged)
                    .width(Length::Fill),
                button(text("Capture").center())
                    .width(Length::Fixed(110.0))
                    .on_press(Message::OpenCapture)
                    .style(button::primary),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            row![
                text(format!("Fires on: {}", trigger_label(&state.profile))).size(13),
                space::horizontal(),
            ],
            text("Capture listens to keyboard and mouse together — press either.").size(12),
            row![
                mode_card(
                    PressMode::Toggle.label(),
                    PressMode::Toggle.description(),
                    state.profile.press_mode == PressMode::Toggle,
                    Message::PressModeSelected(PressMode::Toggle),
                ),
                mode_card(
                    PressMode::Hold.label(),
                    PressMode::Hold.description(),
                    state.profile.press_mode == PressMode::Hold,
                    Message::PressModeSelected(PressMode::Hold),
                ),
            ]
            .spacing(10),
        ]
        .spacing(10)
        .into(),
    );

    let timing_card = section(
        "Timing",
        row![
            text("Every"),
            text_input("2", &state.interval_value)
                .on_input(Message::IntervalValueChanged)
                .width(Length::Fill),
            pick_list(
                TimeUnit::all(),
                Some(state.interval_unit),
                Message::IntervalUnitSelected,
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
    );

    let click_card = section(
        "Click",
        column![
            row![
                text("Button:"),
                option_button(
                    "Left",
                    state.profile.button == MouseButtonKind::Left,
                    Message::ButtonSelected(MouseButtonKind::Left),
                ),
                option_button(
                    "Right",
                    state.profile.button == MouseButtonKind::Right,
                    Message::ButtonSelected(MouseButtonKind::Right),
                ),
                option_button(
                    "Middle",
                    state.profile.button == MouseButtonKind::Middle,
                    Message::ButtonSelected(MouseButtonKind::Middle),
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Type:"),
                option_button(
                    "Single",
                    state.profile.kind == ClickKind::Single,
                    Message::KindSelected(ClickKind::Single),
                ),
                option_button(
                    "Double",
                    state.profile.kind == ClickKind::Double,
                    Message::KindSelected(ClickKind::Double),
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10)
        .into(),
    );

    let mut target_inner = column![
        row![
            text("Mode:"),
            option_button(
                "Current",
                state.profile.target == ClickTarget::CurrentPosition,
                Message::TargetCurrent,
            ),
            option_button(
                "Fixed",
                matches!(state.profile.target, ClickTarget::Fixed { .. }),
                Message::TargetFixed,
            ),
            option_button(
                "Sequence",
                matches!(state.profile.target, ClickTarget::Sequence(_)),
                Message::TargetSequence,
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(10);

    if matches!(state.profile.target, ClickTarget::Fixed { .. }) {
        target_inner = target_inner.push(
            row![
                text_input("x", &state.fixed_x).on_input(Message::FixedXChanged),
                text_input("y", &state.fixed_y).on_input(Message::FixedYChanged),
            ]
            .spacing(8),
        );
    }
    if matches!(state.profile.target, ClickTarget::Sequence(_)) {
        target_inner = target_inner.push(
            row![
                text_input("px", &state.point_x).on_input(Message::PointXChanged),
                text_input("py", &state.point_y).on_input(Message::PointYChanged),
                button("Add").on_press(Message::AddPoint),
                button("Clear").on_press(Message::ClearPoints),
                text(format!("{} pts", state.profile.points.len())),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    let target_card = section("Target", target_inner.into());

    let footer = row![
        button("Delete")
            .on_press(Message::DeleteProfile)
            .style(button::danger),
        space::horizontal(),
        text(&state.status).size(13),
        space::horizontal(),
        button("Save")
            .on_press(Message::SaveProfile)
            .style(button::secondary),
        if running {
            button("Stop")
                .on_press(Message::Stop)
                .style(button::danger)
        } else {
            button("Start")
                .on_press(Message::Start)
                .style(button::primary)
        },
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body = column![
        profile_card,
        trigger_card,
        timing_card,
        click_card,
        target_card,
    ]
    .spacing(12);

    column![header, scrollable(body).height(Length::Fixed(430.0)), footer]
        .spacing(12)
        .into()
}
