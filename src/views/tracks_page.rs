// SPDX-License-Identifier: GPL-3.0
//
// Tracks Page - Shows all tracks in the library

use crate::app::Message;
use crate::model::track::Track;
use crate::views::track_actions::{self, TrackActionsContext};
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

use crate::fl;
use std::collections::HashMap;

/// Constants
const ROW_HEIGHT: f32 = 70.0;
const COVER_SIZE: f32 = 50.0;

/// Renders the tracks page
///
/// Layout: [Cover 50x50] | [Title / Album / Artist stacked] | [Duration]
/// Renders the tracks page with virtual scrolling optimization
///
/// Layout: [Cover 50x50] | [Title / Album / Artist stacked] | [Duration]
pub fn view<'a>(
    tracks: &'a [Track],
    covers: &'a HashMap<i64, widget::image::Handle>,
    current_track_path: Option<String>,
    scroll_offset: cosmic::iced::widget::scrollable::AbsoluteOffset,
    ctx: TrackActionsContext<'a>,
) -> Element<'a, Message> {
    let use_dark_text = ctx.use_dark_text;
    if tracks.is_empty() {
        return widget::container(
            widget::text::title3("Nenhuma música encontrada")
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into();
    }

    // Track count header
    let info_text = fl!("total-tracks", count = tracks.len());

    // Use responsive widget to get viewport size for virtualization
    widget::responsive(move |size| {
        let viewport_height = size.height;
        let spacing = 4.0;

        // Calculate visible range
        // Adjust for header height since scrollable is below it?
        // Actually, the structure in original was: Column [ Header, Scrollable(List) ]
        // So the scroll offset applies only to the list.
        // But here we need to know the scrollable's viewport.

        let visible_y_start = scroll_offset.y;
        let visible_y_end = visible_y_start + viewport_height;

        // Build visible rows and placeholders
        let mut track_rows: Vec<Element<'static, Message>> = Vec::with_capacity(tracks.len());

        let mut current_y = 0.0;

        for (idx, track) in tracks.iter().enumerate() {
            let row_height = ROW_HEIGHT;
            let item_end = current_y + row_height;

            // Check intersection with visible area
            // We add a small buffer (one row height) to prevent pop-in
            let buffer = ROW_HEIGHT;
            let is_visible =
                item_end >= (visible_y_start - buffer) && current_y <= (visible_y_end + buffer);

            if is_visible {
                let is_current = current_track_path
                    .as_ref()
                    .is_some_and(|p| p == &track.uri.to_string_lossy().to_string());
                let cover = track.cover_art_id.and_then(|id| covers.get(&id)).cloned();
                track_rows.push(track_row(idx, track, cover, is_current, ctx));
            } else {
                // Placeholder for off-screen items to maintain scroll height
                track_rows.push(
                    widget::column()
                        .width(Length::Fill)
                        .height(Length::Fixed(row_height))
                        .into(),
                );
            }

            current_y += row_height + spacing;
        }

        // Build content
        let tracks_content = widget::column::with_children(track_rows)
            .spacing(spacing)
            .width(Length::Fill);

        // Scrollable container for tracks
        let scrollable_tracks = widget::scrollable(tracks_content)
            .on_scroll(Message::TracksScroll)
            .id(cosmic::widget::Id::new(crate::app::TRACKS_SCROLL_ID))
            .width(Length::Fill)
            .height(Length::Fill);

        // Main layout
        let content = widget::column::with_capacity(2)
            .push(widget::container(widget::text::title4(info_text.clone())).padding([16, 20]))
            .push(scrollable_tracks)
            .width(Length::Fill);

        // Wrap in container with adaptive text color and clip to prevent overflow
        let styled_content = widget::container(content)
            .width(Length::Fill)
            .clip(true)
            .style(move |_theme: &cosmic::Theme| {
                let text_color = if use_dark_text {
                    cosmic::iced::Color::BLACK
                } else {
                    cosmic::iced::Color::WHITE
                };
                cosmic::widget::container::Style {
                    text_color: Some(text_color),
                    ..Default::default()
                }
            });

        styled_content.into()
    })
    .into()
}

/// Renders a single track row
/// [Cover 50x50] | [Title / Album / Artist stacked] | [Duration]
fn track_row(
    idx: usize,
    track: &Track,
    cover: Option<widget::image::Handle>,
    is_current: bool,
    ctx: TrackActionsContext<'_>,
) -> Element<'static, Message> {
    let use_dark_text = ctx.use_dark_text;
    // Format duration as mm:ss
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let secs = track.duration.max(0.0) as u64; // Safe: duration is always non-negative
    let duration_str = format!("{}:{:02}", secs / 60, secs % 60);

    // Cover art or placeholder
    let cover_widget: Element<Message> = if let Some(handle) = cover {
        widget::container(
            widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Contain),
        )
        .width(Length::Fixed(COVER_SIZE))
        .height(Length::Fixed(COVER_SIZE))
        .clip(true)
        .style(|_| cosmic::widget::container::Style {
            border: cosmic::iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    } else {
        widget::container(
            widget::text::body("🎵")
                .size(20)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(COVER_SIZE))
        .height(Length::Fixed(COVER_SIZE))
        .style(|theme: &cosmic::Theme| {
            let bg = theme.cosmic().background.base;
            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red + 0.1,
                        bg.green + 0.1,
                        bg.blue + 0.1,
                        0.6,
                    ),
                )),
                border: cosmic::iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    };

    // Stacked info: Title (bold) -> Album -> Artist
    let info_column = widget::column::with_capacity(3)
        .push(
            widget::text::body(track.title.clone())
                .size(14)
                .font(cosmic::iced::Font {
                    weight: cosmic::iced::font::Weight::Medium,
                    ..Default::default()
                }),
        )
        .push(widget::text::caption(track.album.clone()).size(12))
        .push(widget::text::caption(track.artist.clone()).size(11))
        .spacing(2)
        .width(Length::Fill);

    // Duration on the right
    let duration_widget = widget::text::caption(duration_str)
        .size(12)
        .width(Length::Fixed(50.0))
        .align_x(cosmic::iced::alignment::Horizontal::Right);

    // Full row layout with info container
    let info_container = widget::row::with_capacity(3)
        .push(cover_widget)
        .push(info_column)
        .push(duration_widget)
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    // Add to queue button
    let add_button = widget::button::icon(widget::icon::from_name("list-add-symbolic"))
        .on_press(Message::AddToQueue(idx))
        .class(cosmic::theme::Button::Text);

    // Favorite / add to playlist
    let actions = track_actions::row_actions(track.id, track.favorite, ctx);

    // Full row layout with action buttons
    let row_content = widget::row::with_capacity(3)
        .push(info_container)
        .push(actions)
        .push(add_button)
        .spacing(8)
        .padding([8, 16])
        .align_y(Alignment::Center);

    // Make clickable with custom button style
    let button = widget::button::custom(row_content)
        .on_press(Message::PlayTrack(idx))
        .width(Length::Fill)
        .height(Length::Fixed(ROW_HEIGHT))
        .class(track_button_style(is_current, use_dark_text));

    button.into()
}

/// Custom button style for track rows
fn track_button_style(is_current: bool, use_dark_text: bool) -> cosmic::theme::Button {
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
                let bg = theme.cosmic().background.base;
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
                let bg = theme.cosmic().background.base;
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
