// SPDX-License-Identifier: GPL-3.0
//
// Track Entry Component - Single track row for album detail

use crate::app::Message;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

/// Renders a single track entry row
///
/// Returns a clickable row with track number, title, and duration.
/// Clicking plays the album starting from this track.
pub fn view(
    album_idx: usize,
    track_idx: usize,
    track_number: u32,
    title: String,
    duration: f64,
    is_current: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // Format duration as mm:ss
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let secs = duration.max(0.0) as u64; // Safe: duration is always non-negative
    let duration_str = format!("{}:{:02}", secs / 60, secs % 60);

    // Track number display
    let number_widget = widget::text::body(format!("{track_number:02}"))
        .size(14)
        .width(Length::Fixed(30.0));

    // Title
    let title_widget = widget::text::body(title).size(14).width(Length::Fill);

    // Duration
    let duration_widget = widget::text::body(duration_str)
        .size(14)
        .width(Length::Fixed(50.0));

    // Create row content
    let content = widget::row::with_capacity(3)
        .push(number_widget)
        .push(title_widget)
        .push(duration_widget)
        .spacing(12)
        .padding([8, 16])
        .align_y(Alignment::Center);

    // Wrap in button for click handling
    let button = widget::button::custom(content)
        .on_press(Message::PlayAlbumFromTrack(album_idx, track_idx))
        .width(Length::Fill);

    // Style based on current track
    let button = button.class(track_entry_style(is_current, use_dark_text));

    button.into()
}

fn track_entry_style(is_current: bool, use_dark_text: bool) -> cosmic::theme::Button {
    if is_current {
        let text_color = if use_dark_text {
            cosmic::iced::Color::BLACK
        } else {
            cosmic::iced::Color::WHITE
        };

        let make_style = move || {
            Box::new(move |_focused: bool, theme: &cosmic::Theme| {
                let cosmic = theme.cosmic();
                cosmic::widget::button::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic.accent.base.into())),
                    text_color: Some(text_color),
                    border_radius: 8.0.into(),
                    ..Default::default()
                }
            })
        };

        cosmic::theme::Button::Custom {
            active: make_style(),
            disabled: Box::new(move |theme| {
                let cosmic = theme.cosmic();
                cosmic::widget::button::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic.accent.base.into())),
                    text_color: Some(text_color),
                    border_radius: 8.0.into(),
                    ..Default::default()
                }
            }),
            hovered: make_style(),
            pressed: make_style(),
        }
    } else {
        cosmic::theme::Button::Custom {
            active: Box::new(|_, _| cosmic::widget::button::Style {
                background: None,
                border_radius: 8.0.into(),
                ..Default::default()
            }),
            disabled: Box::new(|_| cosmic::widget::button::Style {
                background: None,
                border_radius: 8.0.into(),
                ..Default::default()
            }),
            hovered: Box::new(|_, theme| {
                let cosmic = theme.cosmic();
                let bg = cosmic.background.base;
                cosmic::widget::button::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(
                            bg.red + 0.08,
                            bg.green + 0.08,
                            bg.blue + 0.08,
                            0.7,
                        ),
                    )),
                    border_radius: 8.0.into(),
                    ..Default::default()
                }
            }),
            pressed: Box::new(|_, theme| {
                let cosmic = theme.cosmic();
                let bg = cosmic.background.base;
                cosmic::widget::button::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(
                            bg.red + 0.12,
                            bg.green + 0.12,
                            bg.blue + 0.12,
                            0.8,
                        ),
                    )),
                    border_radius: 8.0.into(),
                    ..Default::default()
                }
            }),
        }
    }
}
