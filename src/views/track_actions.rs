// SPDX-License-Identifier: GPL-3.0
//
// Track Actions - Shared per-track buttons (favorite heart, add to playlist)
// used by every track row and by the control bar.

use crate::app::Message;
use crate::fl;
use crate::model::playlist::Playlist;
use cosmic::Element;
use cosmic::widget;

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
        .spacing(2)
        .align_y(cosmic::iced::Alignment::Center)
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
