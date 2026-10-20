// SPDX-License-Identifier: GPL-3.0
//
// Album Detail Page - Shows album cover, info, and track list

use crate::app::Message;
use crate::views::albums_page::AlbumData;
use crate::views::track_actions::TrackActionsContext;
use crate::views::track_entry;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

use crate::fl;
use std::collections::HashMap;

/// Renders the album detail page with cover, info, and track list
///
/// Displays:
/// - Back button
/// - Large album cover
/// - Album title, artist, track count, duration
/// - Play All and Shuffle buttons
/// - Scrollable list of tracks
///
/// `source_artist` - If Some, navigating back returns to artist detail instead of albums grid
#[allow(clippy::too_many_lines)]
pub fn view(
    album_idx: usize,
    album: &AlbumData,
    covers: &HashMap<i64, widget::image::Handle>,
    current_track_path: Option<&str>,
    source_artist: Option<usize>,
    ctx: TrackActionsContext<'_>,
) -> Element<'static, Message> {
    let use_dark_text = ctx.use_dark_text;
    // Back button
    let back_button = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
        .on_press(Message::BackToAlbums(source_artist))
        .class(cosmic::theme::Button::Text);

    // Header with back button and title
    let header = widget::row::with_capacity(2)
        .push(back_button)
        .push(widget::text::title3(album.title.clone()))
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([16, 16]);

    // Album cover (large)
    let cover_size = 250.0;
    let cover_widget: Element<Message> =
        if let Some(handle) = album.cover_art_id.and_then(|id| covers.get(&id)).cloned() {
            widget::container(
                widget::image(handle)
                    .width(Length::Fixed(cover_size))
                    .height(Length::Fixed(cover_size))
                    .content_fit(cosmic::iced::ContentFit::Contain),
            )
            .clip(true)
            .into()
        } else {
            widget::container(
                widget::text::title1("🎵")
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center),
            )
            .width(Length::Fixed(cover_size))
            .height(Length::Fixed(cover_size))
            .style(|_| cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(0.2, 0.2, 0.25, 0.8),
                )),
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        };

    // Album info
    let track_count = album.tracks.len();
    let total_duration: f64 = album.tracks.iter().map(|t| t.duration).sum();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration_secs = total_duration.max(0.0) as u64; // Safe: duration is always non-negative
    let duration_str = format!("{}:{:02}", duration_secs / 60, duration_secs % 60);

    let album_title = widget::text::title2(album.title.clone());
    let album_artist = widget::text::body(album.artist.clone()).size(16);
    let album_meta = widget::text::caption(fl!(
        "album-stats",
        count = track_count,
        duration = duration_str
    ));

    // Play buttons
    let play_all_btn = widget::button::suggested(fl!("btn-play"))
        .leading_icon(widget::icon::from_name("media-playback-start-symbolic"))
        .on_press(Message::PlayAlbumFromTrack(album_idx, 0))
        .class(crate::views::suggested_button_style(use_dark_text));

    let buttons_row = widget::row::with_capacity(1).push(play_all_btn).spacing(8);

    let info_column = widget::column::with_capacity(5)
        .push(album_title)
        .push(album_artist)
        .push(album_meta)
        .push(widget::space().height(Length::Fixed(16.0)))
        .push(buttons_row)
        .spacing(8);

    // Album header: cover + info side by side
    let album_header = widget::row::with_capacity(2)
        .push(cover_widget)
        .push(info_column)
        .spacing(24)
        .padding([0, 16]);

    // Track list
    let tracks: Vec<Element<'static, Message>> = album
        .tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let is_current = current_track_path.is_some_and(|p| p == track.uri.to_string_lossy());

            #[allow(clippy::cast_possible_truncation)]
            let track_num = track.track_number.unwrap_or((idx + 1) as u32); // Safe: small index

            track_entry::view(
                track,
                track_num,
                is_current,
                Message::PlayAlbumFromTrack(album_idx, idx),
                None,
                ctx,
            )
        })
        .collect();

    let tracks_list = widget::column::with_children(tracks)
        .spacing(4)
        .padding([16, 16]);

    // Main content
    let content = widget::column::with_capacity(4)
        .push(header)
        .push(widget::divider::horizontal::default())
        .push(album_header)
        .push(widget::space().height(Length::Fixed(16.0)))
        .push(tracks_list);

    let styled_content = widget::container(content)
        .width(Length::Fill)
        .height(Length::Fill)
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
        });

    widget::scrollable(styled_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
