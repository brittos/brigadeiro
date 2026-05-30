// SPDX-License-Identifier: GPL-3.0
//
// Playlists Page - Grid view of playlists

use crate::app::Message;
use crate::fl;
use crate::model::playlist::Playlist;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use std::collections::HashMap;

/// Constants for virtualization calculations (same as albums for consistency)
const CARD_HEIGHT: f32 = 280.0;
const CARD_WIDTH: f32 = 220.0;
const SPACING: f32 = 20.0;
const PADDING: f32 = 20.0;
const BUFFER_ROWS: usize = 2;

/// Renders the playlists grid view
pub fn view(
    playlists: &[Playlist],
    _covers: &HashMap<i64, widget::image::Handle>, // Can be used for collage covers later
    scroll_y: f32,
    viewport_h: f32,
    viewport_w: f32,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // Header with "New Playlist" button
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let header_padding = PADDING as u16; // Safe: PADDING is a small constant
    let header = widget::row()
        .push(widget::horizontal_space().width(Length::Fill))
        .push(
            widget::button::text(fl!("new-playlist"))
                .on_press(Message::CreatePlaylist("Nova Playlist".to_string()))
                .class(cosmic::theme::Button::Suggested),
        )
        .padding([0, header_padding])
        .align_y(Alignment::Center);

    if playlists.is_empty() {
        return widget::column()
            .push(header)
            .push(
                widget::container(
                    widget::text::title3("Nenhuma playlist criada") // TODO: Use fl! macro
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
            )
            .into();
    }

    // Use real viewport width for column calculation
    let available_width = viewport_w - PADDING * 2.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cols = ((available_width + SPACING) / (CARD_WIDTH + SPACING)).floor() as usize;
    let cols = cols.max(1);

    // Calculate row dimensions
    let row_height = CARD_HEIGHT + SPACING;
    let total_rows = playlists.len().div_ceil(cols);

    // Calculate visible row range
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_row = (scroll_y / row_height).floor() as usize;
    let first_row = first_row.saturating_sub(BUFFER_ROWS);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let visible_rows = (viewport_h / row_height).ceil() as usize + 1 + BUFFER_ROWS * 2;
    let last_row = (first_row + visible_rows).min(total_rows);

    // Calculate album index range
    let first_idx = first_row * cols;
    let last_idx = (last_row * cols).min(playlists.len());

    // Calculate spacer heights
    #[allow(clippy::cast_precision_loss)]
    let top_height = (first_row as f32 * row_height).max(0.0); // Safe: small index
    let bottom_rows = total_rows.saturating_sub(last_row);
    #[allow(clippy::cast_precision_loss)]
    let bottom_height = (bottom_rows as f32 * row_height).max(0.0); // Safe: small index

    // Build only visible cards
    let visible_playlists = &playlists[first_idx..last_idx];
    let cards: Vec<Element<'static, Message>> = visible_playlists
        .iter()
        .map(|playlist| {
            playlist_card(
                playlist.clone(),
                None, // We generally don't have playlist covers yet
                use_dark_text,
            )
        })
        .collect();

    // Build layout with spacers
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let spacing_u16 = SPACING as u16; // Safe: SPACING is a small constant
    let grid_content = widget::column()
        .push(widget::vertical_space().height(Length::Fixed(top_height)))
        .push(
            widget::flex_row(cards)
                .spacing(spacing_u16)
                .padding(PADDING)
                .width(Length::Fill)
                .align_items(Alignment::Start)
                .justify_content(cosmic::widget::JustifyContent::Center),
        )
        .push(widget::vertical_space().height(Length::Fixed(bottom_height)));

    let content = widget::column().push(header).push(grid_content);

    widget::scrollable(content)
        .on_scroll(Message::AlbumsScrolled) // Reusing the scroll message or create a new one?
        // Let's reuse AlbumsScrolled or create PlaylistsScrolled.
        // Reusing AlbumsScrolled might be confusing if we have separate state.
        // I'll assume we add PlaylistsScrolled to Message enum.
        .into()
}

fn playlist_card(
    playlist: Playlist,
    cover: Option<widget::image::Handle>,
    use_dark_text: bool,
) -> Element<'static, Message> {
    use cosmic::iced::widget::text::Wrapping;

    let text_color = if use_dark_text {
        cosmic::iced::Color::BLACK
    } else {
        cosmic::iced::Color::WHITE
    };

    let cover_widget: Element<Message> = if let Some(handle) = cover {
        widget::container(
            widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Contain),
        )
        .clip(true)
        .into()
    } else {
        // Placeholder with an icon
        widget::container(
            widget::text::title1("☰") // Playlist icon
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .style(|_| cosmic::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgba(0.25, 0.2, 0.25, 0.8), // Slightly violet for playlists
            )),
            ..Default::default()
        })
        .into()
    };

    let title_widget = widget::container(
        widget::text::body(playlist.title)
            .size(14)
            .font(cosmic::iced::Font {
                weight: cosmic::iced::font::Weight::Bold,
                ..Default::default()
            })
            .wrapping(Wrapping::None)
            .class(cosmic::theme::Text::Color(text_color)),
    )
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center)
    .width(Length::Fixed(200.0))
    .height(Length::Fixed(20.0))
    .clip(true);

    let desc = playlist
        .description
        .unwrap_or_else(|| format!("{} músicas", 0)); // We need real count

    let desc_widget = widget::container(
        widget::text::caption(desc)
            .size(12)
            .wrapping(Wrapping::None)
            .class(cosmic::theme::Text::Color(text_color)),
    )
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center)
    .width(Length::Fixed(200.0))
    .height(Length::Fixed(16.0))
    .clip(true);

    let content = widget::column()
        .push(cover_widget)
        .push(title_widget)
        .push(desc_widget)
        .spacing(5)
        .align_x(Alignment::Center);

    widget::button::custom(
        widget::container(content)
            .padding(10)
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().background.base;
                let lighter_color = cosmic::iced::Color::from_rgba(
                    (bg.red + 0.1).min(1.0),
                    (bg.green + 0.1).min(1.0),
                    (bg.blue + 0.1).min(1.0),
                    0.3,
                );
                cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(lighter_color)),
                    border: cosmic::iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
    )
    .on_press(Message::OpenPlaylistDetail(playlist.id))
    .width(Length::Fixed(220.0))
    .height(Length::Fixed(280.0))
    .into()
}
