// SPDX-License-Identifier: GPL-3.0

//! Onboarding page for Brigadeiro
//!
//! This view is shown when the library is empty, guiding the user
//! to add their music folders.

use crate::app::Message;
use crate::fl;
use cosmic::Element;
use cosmic::iced::Alignment;
use cosmic::iced::Length;
use cosmic::widget;

/// Build the onboarding page view
pub fn view(use_dark_text: bool) -> Element<'static, Message> {
    let icon = widget::icon::from_name("folder-music-symbolic").size(128);

    let title = widget::text::title1(fl!("onboarding-welcome"))
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    let subtitle = widget::text::title3(fl!("onboarding-subtitle"))
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    let description = widget::text::body(fl!("onboarding-description"))
        .width(Length::Fixed(400.0))
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    let add_button = widget::button::suggested(fl!("onboarding-btn-add"))
        .on_press(Message::OpenFolderPicker)
        .padding([12, 24]);

    widget::container(
        widget::column::with_capacity(5)
            .spacing(24)
            .align_x(Alignment::Center)
            .push(icon)
            .push(
                widget::column::with_capacity(2)
                    .spacing(8)
                    .push(title)
                    .push(subtitle),
            )
            .push(description)
            .push(widget::space().height(Length::Fixed(16.0)))
            .push(add_button),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center)
    .style(move |_theme| {
        let text_color = if use_dark_text {
            cosmic::iced::Color::BLACK
        } else {
            cosmic::iced::Color::WHITE
        };
        cosmic::widget::container::Style {
            text_color: Some(text_color),
            ..Default::default()
        }
    })
    .into()
}
