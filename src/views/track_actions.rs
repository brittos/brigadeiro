// SPDX-License-Identifier: GPL-3.0
//
// Track Actions - Shared per-track buttons (favorite heart, add to playlist)
// used by every track row and by the control bar.

use crate::app::Message;
use crate::fl;
use crate::model::playlist::Playlist;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

const MENU_WIDTH: f32 = 240.0;
const MENU_MAX_HEIGHT: f32 = 320.0;

/// Everything a track row needs to render its action buttons.
#[derive(Clone, Copy)]
pub struct TrackActionsContext<'a> {
    /// User playlists shown in the "Add to playlist" menu
    pub playlists: &'a [Playlist],
    /// Track id whose "Add to playlist" menu is currently open, if any
    pub open_menu: Option<i64>,
    /// Whether to use dark text on the current background
    pub use_dark_text: bool,
}

/// Action buttons shown at the end of a track row.
pub fn row_actions(
    track_id: i64,
    favorite: bool,
    ctx: TrackActionsContext<'_>,
) -> Element<'static, Message> {
    widget::row::with_capacity(2)
        .push(favorite_button(track_id, favorite, ctx.use_dark_text))
        .push(add_to_playlist_button(track_id, ctx))
        .spacing(2)
        .align_y(Alignment::Center)
        .into()
}

/// Button that opens a popover listing the user's playlists (plus "New playlist").
///
/// The popover is attached only while `ctx.open_menu` points at this track, so
/// at most one menu is open at a time and its state lives in the app model.
pub fn add_to_playlist_button(
    track_id: i64,
    ctx: TrackActionsContext<'_>,
) -> Element<'static, Message> {
    let button = widget::button::icon(widget::icon::from_name("playlist-symbolic"))
        .on_press(Message::OpenAddToPlaylistMenu(track_id))
        .class(cosmic::theme::Button::Text);

    let button = widget::tooltip(
        button,
        widget::text::body(fl!("add-to-playlist")),
        widget::tooltip::Position::Top,
    );

    if ctx.open_menu != Some(track_id) {
        return button.into();
    }

    widget::popover(button)
        .popup(add_to_playlist_menu(track_id, ctx.playlists))
        .position(widget::popover::Position::Bottom)
        .on_close(Message::CloseAddToPlaylistMenu(track_id))
        .into()
}

fn add_to_playlist_menu(track_id: i64, playlists: &[Playlist]) -> Element<'static, Message> {
    let mut items = widget::column::with_capacity(playlists.len().max(1)).width(Length::Fill);

    if playlists.is_empty() {
        items = items.push(
            widget::container(widget::text::caption(fl!("no-playlists"))).padding([4, 8]),
        );
    } else {
        for playlist in playlists {
            items = items.push(
                widget::button::text(playlist.title.clone())
                    .on_press(Message::AddTrackToPlaylist(playlist.id, track_id))
                    .class(cosmic::theme::Button::MenuItem)
                    .width(Length::Fill),
            );
        }
    }

    let list = widget::container(widget::scrollable(items).width(Length::Fill))
        .max_height(MENU_MAX_HEIGHT)
        .width(Length::Fill);

    let new_playlist = widget::button::text(fl!("new-playlist"))
        .leading_icon(widget::icon::from_name("list-add-symbolic"))
        .on_press(Message::CreatePlaylistWithTrack(track_id))
        .class(cosmic::theme::Button::MenuItem)
        .width(Length::Fill);

    let content = widget::column::with_capacity(4)
        .push(widget::container(widget::text::caption_heading(fl!("add-to-playlist"))).padding([4, 8]))
        .push(list)
        .push(widget::divider::horizontal::default())
        .push(new_playlist)
        .spacing(4)
        .width(Length::Fixed(MENU_WIDTH));

    widget::container(content)
        .padding(8)
        .class(cosmic::theme::Container::Dropdown)
        .into()
}

/// Heart button that toggles the favorite flag of a track.
///
/// Highlighted with the accent color when the track is a favorite,
/// rendered as a plain text button otherwise.
pub fn favorite_button(
    track_id: i64,
    favorite: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let class = if favorite {
        crate::views::suggested_button_style(use_dark_text)
    } else {
        cosmic::theme::Button::Text
    };
    let label = if favorite {
        fl!("btn-unfavorite")
    } else {
        fl!("btn-favorite")
    };

    let button = widget::button::icon(widget::icon::from_name("emblem-favorite-symbolic"))
        .on_press(Message::ToggleFavorite(track_id))
        .class(class);

    widget::tooltip(
        button,
        widget::text::body(label),
        widget::tooltip::Position::Top,
    )
    .into()
}
