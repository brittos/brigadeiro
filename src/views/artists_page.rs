// SPDX-License-Identifier: GPL-3.0
//
// Artists Page - Grid view of artists

use crate::app::Message;
use crate::model::track::Track;
use crate::utils::color;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

use crate::fl;

/// Represents artist data for display in the grid
#[derive(Debug, Clone)]
pub struct ArtistData {
    pub name: String,
    pub album_count: usize,
    pub track_count: usize,
    pub tracks: Vec<Track>,
}

impl PartialEq for ArtistData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

/// Constants for artist card layout
const SPACING: f32 = 20.0;
const PADDING: f32 = 20.0;

/// Renders the artist grid view
pub fn view(artists: &[ArtistData], use_dark_text: bool) -> Element<'static, Message> {
    if artists.is_empty() {
        return widget::container(
            widget::text::title3("Nenhum artista encontrado") // TODO: Use fl! macro
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into();
    }

    // Build artist cards
    let cards: Vec<Element<'static, Message>> = artists
        .iter()
        .enumerate()
        .map(|(idx, artist)| {
            artist_card(
                idx,
                artist.name.clone(),
                artist.album_count,
                artist.track_count,
                use_dark_text,
            )
        })
        .collect();

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let spacing_u16 = SPACING as u16; // Safe: SPACING is a small constant

    let content = widget::column().push(
        widget::flex_row(cards)
            .spacing(spacing_u16)
            .padding(PADDING)
            .width(Length::Fill)
            .align_items(Alignment::Start)
            .justify_content(cosmic::widget::JustifyContent::Center),
    );

    widget::scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Renders a single artist card - circle with name and info inside
fn artist_card(
    artist_idx: usize,
    name: String,
    album_count: usize,
    track_count: usize,
    _use_dark_text: bool,
) -> Element<'static, Message> {
    use cosmic::iced::widget::text::Wrapping;

    let circle_size = 210.0;

    // Artist name (bold, centered) - 15px
    // Text color is set by the container's text_color style
    let name_widget = widget::text::body(name)
        .size(15)
        .font(cosmic::iced::Font {
            weight: cosmic::iced::font::Weight::Bold,
            ..Default::default()
        })
        .wrapping(Wrapping::WordOrGlyph)
        .width(Length::Fixed(circle_size - 40.0))
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    // Album/track count (smaller, centered) - 13px
    let meta_widget = widget::text::caption(fl!(
        "artist-stats",
        albums = album_count,
        songs = track_count
    ))
    .size(13)
    .width(Length::Fixed(circle_size - 40.0))
    .align_x(cosmic::iced::alignment::Horizontal::Center);

    // Content inside the circle
    let circle_content = widget::column::with_capacity(2)
        .push(name_widget)
        .push(meta_widget)
        .spacing(8)
        .align_x(cosmic::iced::Alignment::Center);

    // The circle itself with content inside
    let avatar = widget::container(circle_content)
        .width(Length::Fixed(circle_size))
        .height(Length::Fixed(circle_size))
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .padding(10)
        .style(move |theme: &cosmic::Theme| {
            let cosmic = theme.cosmic();
            let accent_color: cosmic::iced::Color = cosmic.accent.base.into();
            let use_dark = color::should_use_dark_text(accent_color);
            let text_color = if use_dark {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(accent_color)),
                text_color: Some(text_color),
                border: cosmic::iced::Border {
                    radius: (circle_size / 2.0).into(), // Circle
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    // Wrap in button for click handling - same size as circle
    widget::button::custom(avatar)
        .on_press(Message::OpenArtistDetail(artist_idx))
        .class(cosmic::theme::Button::Icon)
        .width(Length::Fixed(circle_size))
        .height(Length::Fixed(circle_size))
        .into()
}
