// SPDX-License-Identifier: GPL-3.0
//
// Playlist Detail Page - Shows playlist cover collage, info, track list and edit controls

use crate::app::Message;
use crate::fl;
use crate::model::playlist::Playlist;
use crate::model::track::Track;
use crate::views::track_actions::TrackActionsContext;
use crate::views::track_entry;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use std::collections::HashMap;

fn view_playlist_header<'a>(playlist: &'a Playlist, edit_mode: bool) -> Element<'a, Message> {
    // Back button
    let back_button = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
        .on_press(Message::NavTo(crate::app::NavPage::Playlists))
        .class(cosmic::theme::Button::Text);

    // Header with title or input
    let title_widget: Element<Message> = if edit_mode {
        widget::text_input("Título da Playlist", &playlist.title) // TODO: Use fl! macro
            .on_input(|title| Message::RenamePlaylist(playlist.id, title))
            .width(Length::Fill)
            .into()
    } else {
        widget::text::title3(playlist.title.clone()).into()
    };

    let mut header = widget::row::with_capacity(4)
        .push(back_button)
        .push(title_widget)
        .push(widget::horizontal_space().width(Length::Fill));

    // The Favorites collection is not editable
    if !playlist.is_favorites() {
        header = header.push(
            widget::button::icon(widget::icon::from_name(if edit_mode {
                "emblem-ok-symbolic"
            } else {
                "document-edit-symbolic"
            }))
            .on_press(Message::TogglePlaylistEditMode(!edit_mode))
            .class(cosmic::theme::Button::Text),
        );
    }

    header
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([16, 16])
        .into()
}

fn view_playlist_cover<'a>(
    tracks: &'a [Track],
    covers: &'a HashMap<i64, widget::image::Handle>,
) -> Element<'static, Message> {
    let cover_size = 200.0;

    // Find first available cover
    let first_cover_id = tracks.iter().find_map(|t| t.cover_art_id);
    let cover_handle = first_cover_id.and_then(|id| covers.get(&id)).cloned();

    if let Some(handle) = cover_handle {
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
            widget::text::title1("☰")
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(cover_size))
        .height(Length::Fixed(cover_size))
        .style(|_| cosmic::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(0.25, 0.2, 0.3, 0.8),
            )),
            border: cosmic::iced::Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }
}

fn view_playlist_info<'a>(
    playlist: &'a Playlist,
    track_count: usize,
    duration_secs: u64,
    use_dark_text: bool,
    edit_mode: bool,
) -> Element<'a, Message> {
    let duration_str = format!("{}:{:02}", duration_secs / 60, duration_secs % 60);

    let desc_widget: Element<Message> = if edit_mode {
        widget::text_input("Descrição", playlist.description.as_deref().unwrap_or(""))
            .on_input(|desc| Message::UpdatePlaylistDescription(playlist.id, desc))
            .width(Length::Fill)
            .into()
    } else {
        widget::text::body(playlist.description.clone().unwrap_or_default()).into()
    };

    let meta_text = format!("{track_count} músicas • {duration_str}");
    let playlist_meta = widget::text::caption(meta_text);

    // Play buttons
    let play_all_btn = widget::button::suggested("▶ Reproduzir")
        .on_press(Message::PlayPlaylist(playlist.id))
        .class(crate::views::suggested_button_style(use_dark_text));

    let delete_btn = if edit_mode {
        Some(
            widget::button::standard("🗑 Deletar")
                .on_press(Message::DeletePlaylist(playlist.id))
                .class(cosmic::theme::Button::Destructive),
        )
    } else {
        None
    };

    let mut buttons_row = widget::row().push(play_all_btn).spacing(8);
    if let Some(btn) = delete_btn {
        buttons_row = buttons_row.push(btn);
    }

    widget::column::with_capacity(5)
        .push(desc_widget)
        .push(playlist_meta)
        .push(widget::vertical_space().height(Length::Fixed(16.0)))
        .push(buttons_row)
        .spacing(8)
        .width(Length::Fill)
        .into()
}

fn view_track_list<'a>(
    playlist: &'a Playlist,
    tracks: &'a [Track],
    current_track_path: Option<&'a str>,
    ctx: TrackActionsContext<'_>,
) -> Element<'a, Message> {
    if tracks.is_empty() && playlist.is_favorites() {
        return widget::container(
            widget::text::body(fl!("favorites-empty"))
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding([16, 16])
        .into();
    }

    let track_elements: Vec<Element<'static, Message>> = tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let is_current = current_track_path.is_some_and(|p| p == track.uri.to_string_lossy());

            #[allow(clippy::cast_possible_truncation)]
            let track_number = (idx + 1) as u32; // Safe: idx is a small index

            track_entry::view(
                track,
                track_number,
                is_current,
                Message::PlayAlbumFromTrack(0, idx),
                None,
                ctx,
            )
        })
        .collect();

    widget::column::with_children(track_elements)
        .spacing(4)
        .padding([16, 16])
        .into()
}

/// Renders the playlist detail page
pub fn view<'a>(
    playlist: &'a Playlist,
    tracks: &'a [Track],
    covers: &'a HashMap<i64, widget::image::Handle>,
    current_track_path: Option<&'a str>,
    edit_mode: bool,
    ctx: TrackActionsContext<'_>,
) -> Element<'a, Message> {
    let use_dark_text = ctx.use_dark_text;
    let header = view_playlist_header(playlist, edit_mode);
    let cover_widget = view_playlist_cover(tracks, covers);

    let track_count = tracks.len();
    let total_duration: f64 = tracks.iter().map(|t| t.duration).sum();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let duration_secs = total_duration.max(0.0) as u64; // Safe: duration is always non-negative

    let info_column = view_playlist_info(
        playlist,
        track_count,
        duration_secs,
        use_dark_text,
        edit_mode,
    );
    let tracks_list = view_track_list(playlist, tracks, current_track_path, ctx);

    // Playlist header: cover + info side by side
    let playlist_header = widget::row::with_capacity(2)
        .push(cover_widget)
        .push(info_column)
        .spacing(24)
        .padding([0, 16]);

    // Main content
    let content = widget::column::with_capacity(5)
        .push(header)
        .push(widget::divider::horizontal::default())
        .push(playlist_header)
        .push(widget::vertical_space().height(Length::Fixed(16.0)))
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
