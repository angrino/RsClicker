use iced::widget::{button, column, container, row, scrollable, text, space};
use iced::{Alignment, Color, Element, Length, Theme};

use crate::app::{Message, RsClicker};
use crate::theme::{avatar_style, bold, card_style, small_circle_button, state_circle_button};
use crate::theme::ViewMode;
use crate::trigger::RunState;

use super::{avatar_letter, brand};

fn run_button(state: &RsClicker, name: &str) -> Element<'static, Message> {
    let run_state = state.run_state(name);
    let label = match run_state {
        RunState::Idle => "▶",
        RunState::Listening => "●",
        RunState::Running => "■",
        RunState::Error => "⚠",
    };
    button(text(label).size(15).center())
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .on_press(Message::ToggleArm(name.to_string()))
        .style(move |t: &Theme, s: button::Status| {
            let bg = match run_state {
                RunState::Idle => color_idle(),
                RunState::Listening => t.palette().primary,
                RunState::Running => color_running(),
                RunState::Error => color_error(),
            };
            state_circle_button(bg, 20.0, t, s)
        })
        .into()
}

fn color_idle() -> Color {
    iced::color!(0x6B7280)
}

fn color_running() -> Color {
    iced::color!(0x2FA36B)
}

fn color_error() -> Color {
    iced::color!(0xD6493B)
}

fn gear_button(name: &str) -> Element<'static, Message> {
    button(text("⚙").size(16).center())
        .width(Length::Fixed(40.0))
        .height(Length::Fixed(40.0))
        .on_press(Message::OpenEditor(name.to_string()))
        .style(small_circle_button)
        .into()
}

pub fn main_view(state: &RsClicker) -> Element<'_, Message> {
    let brand_block = column![
        brand(22.0, None),
        text(crate::app::VERSION).size(11),
    ]
    .spacing(0);
    let top_bar = row![
        brand_block,
        space::horizontal(),
        button(text("⚙").size(18).center())
            .width(Length::Fixed(42.0))
            .height(Length::Fixed(42.0))
            .on_press(Message::OpenAppSettings)
            .style(small_circle_button),
    ]
    .align_y(Alignment::Center);

    let header = row![
        text("Profiles").size(20).font(bold()),
        space::horizontal(),
        text(format!("{} saved", state.profiles.len())).size(13),
    ]
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = if state.profiles.is_empty() {
        container(
            column![
                text("No profiles yet").size(18).font(bold()).center(),
                text("Press + below to create your first profile.")
                    .size(14)
                    .center(),
            ]
            .spacing(6)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
    } else {
        match state.view_mode() {
            ViewMode::List => scrollable(profile_list(state))
                .height(Length::Fill)
                .into(),
            ViewMode::Icons => scrollable(profile_grid(state))
                .height(Length::Fill)
                .into(),
        }
    };

    let bottom_bar = row![
        button(text("+").size(28).center())
            .width(Length::Fixed(56.0))
            .height(Length::Fixed(56.0))
            .on_press(Message::OpenNewProfile)
            .style(crate::theme::circle_button),
        space::horizontal(),
        text(&state.status).size(13),
    ]
    .align_y(Alignment::Center);

    column![top_bar, header, body, bottom_bar]
        .spacing(14)
        .padding(18)
        .into()
}

fn profile_list(state: &RsClicker) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    for name in &state.profiles {
        let row_item = container(
            row![
                button(text(name.as_str()).size(16))
                    .width(Length::Fill)
                    .on_press(Message::OpenEditor(name.clone()))
                    .style(button::text),
                run_button(state, name),
                gear_button(name),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .style(card_style);
        list = list.push(row_item);
    }
    list.into()
}

fn profile_grid(state: &RsClicker) -> Element<'_, Message> {
    let mut grid = column![].spacing(10);
    for chunk in state.profiles.chunks(3) {
        let mut r = row![].spacing(10);
        for name in chunk {
            let card = container(
                column![
                    container(
                        text(avatar_letter(name))
                            .size(22)
                            .font(bold())
                            .color(Color::WHITE)
                            .center()
                    )
                    .width(Length::Fixed(52.0))
                    .height(Length::Fixed(52.0))
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center)
                    .style(avatar_style),
                    button(text(name.as_str()).size(13).center())
                        .width(Length::Fill)
                        .on_press(Message::OpenEditor(name.clone()))
                        .style(button::text),
                    row![run_button(state, name), gear_button(name)]
                        .spacing(8)
                        .align_y(Alignment::Center),
                ]
                .spacing(8)
                .align_x(Alignment::Center),
            )
            .padding(12)
            .width(Length::Fill)
            .style(card_style);
            r = r.push(card);
        }
        for _ in chunk.len()..3 {
            r = r.push(space::horizontal());
        }
        grid = grid.push(r);
    }
    grid.into()
}
