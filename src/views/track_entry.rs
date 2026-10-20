// SPDX-License-Identifier: GPL-3.0
//
// Track Entry Component - Single track row for album detail

use crate::app::Message;
use crate::model::track::Track;
use crate::views::track_actions::{self, TrackActionsContext};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

/// Renders a single track entry row
///
/// Returns a clickable row with track number, title, duration and the
/// per-track action buttons (favorite, add to playlist). `on_press` is emitted
/// when the row itself is clicked; `trailing` is an optional extra control
/// rendered after the actions (e.g. a remove button in edit mode).
pub fn view(
    track: &Track,
    track_number: u32,
    is_current: bool,
    on_press: Message,
    trailing: Option<Element<'static, Message>>,
    ctx: TrackActionsContext<'_>,
) -> Element<'static, Message> {
    // Format duration as mm:ss
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let secs = track.duration.max(0.0) as u64; // Safe: duration is always non-negative
    let duration_str = format!("{}:{:02}", secs / 60, secs % 60);

    // Track number display
    let number_widget = widget::text::body(format!("{track_number:02}"))
        .size(14)
        .width(Length::Fixed(30.0));

    // Title
    let title_widget = widget::text::body(track.title.clone())
        .size(14)
        .width(Length::Fill);

    // Duration
    let duration_widget = widget::text::body(duration_str)
        .size(14)
        .width(Length::Fixed(50.0));

    let actions = track_actions::row_actions(track.id, track.favorite, ctx);

    // Create row content
    let mut content = widget::row::with_capacity(5)
        .push(number_widget)
        .push(title_widget)
        .push(duration_widget)
        .push(actions)
        .spacing(12)
        .padding([4, 16])
        .align_y(Alignment::Center);
    if let Some(trailing) = trailing {
        content = content.push(trailing);
    }

    // Wrap in button for click handling
    let button = widget::button::custom(content)
        .on_press(on_press)
        .width(Length::Fill);

    // Style based on current track
    let button = button.class(track_entry_style(is_current, ctx.use_dark_text));

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
                let bg = cosmic.background(false).base;
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
                let bg = cosmic.background(false).base;
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
