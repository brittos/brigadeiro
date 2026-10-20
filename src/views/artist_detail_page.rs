// SPDX-License-Identifier: GPL-3.0
//
// Artist Detail Page - Shows artist info and their albums

use crate::app::Message;
use crate::views::albums_page::AlbumData;
use crate::views::artists_page::ArtistData;
use cosmic::Element;
use cosmic::iced::{Alignment, Length, Vector};
use cosmic::widget;
use std::collections::HashMap;

/// Renders the artist detail page with info and album grid
///
/// Displays:
/// - Back button
/// - Artist avatar (circle with initial)
/// - Artist name, album count, track count
/// - Play All button
/// - Grid of albums by this artist
pub fn view(
    artist_idx: usize,
    artist: &ArtistData,
    albums: &[AlbumData],
    covers: &HashMap<i64, widget::image::Handle>,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // Back button
    let back_button = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
        .on_press(Message::BackToArtists)
        .class(cosmic::theme::Button::Text);

    // Header with back button and title
    let header = widget::row::with_capacity(2)
        .push(back_button)
        .push(widget::text::title3(artist.name.clone()))
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([16, 16]);

    // Artist info (without avatar)
    let info = widget::column::with_capacity(2)
        .push(widget::text::title2(artist.name.clone()))
        .push(
            widget::text::body(format!(
                "{} álbuns • {} faixas", // TODO: Use fl! macro
                artist.album_count, artist.track_count
            ))
            .size(14),
        )
        .spacing(8)
        .align_x(Alignment::Center);

    // Play All button
    let play_button = widget::button::suggested("Reproduzir Tudo") // TODO: Use fl! macro
        .on_press(Message::PlayArtistTracks(artist_idx))
        .class(crate::views::suggested_button_style(use_dark_text));

    let artist_header = widget::column::with_capacity(2)
        .push(info)
        .push(play_button)
        .spacing(16)
        .align_x(Alignment::Center)
        .padding([0, 0, 24, 0]);

    // Albums section title
    let albums_title = widget::text::title4("Álbuns").width(Length::Fill); // TODO: Use fl! macro

    // Filter albums by this artist
    let artist_albums: Vec<(usize, &AlbumData)> = albums
        .iter()
        .enumerate()
        .filter(|(_, a)| a.artist == artist.name)
        .collect();

    // Album cards
    let album_cards: Vec<Element<'static, Message>> = artist_albums
        .iter()
        .map(|(album_idx, album)| album_card(*album_idx, album, covers, use_dark_text, artist_idx))
        .collect();

    let albums_grid: Element<'static, Message> = if album_cards.is_empty() {
        widget::container(
            widget::text::body("Nenhum álbum encontrado") // TODO: Use fl! macro
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .padding(20)
        .into()
    } else {
        widget::flex_row(album_cards)
            .spacing(16)
            .padding([0, 0])
            .width(Length::Fill)
            .align_items(Alignment::Start)
            .into()
    };

    // Main content
    let content = widget::column::with_capacity(4)
        .push(header)
        .push(artist_header)
        .push(albums_title)
        .push(albums_grid)
        .spacing(16)
        .padding([0, 20])
        .width(Length::Fill);

    // Wrap in container with adaptive text color based on background
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

/// Renders a single album card for the artist detail page
fn album_card(
    album_idx: usize,
    album: &AlbumData,
    covers: &HashMap<i64, widget::image::Handle>,
    use_dark_text: bool,
    source_artist_idx: usize,
) -> Element<'static, Message> {
    use cosmic::iced::widget::text::Wrapping;

    let cover_size = 150.0;

    // Album cover
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

    // Album title
    let title_widget = widget::container(
        widget::text::body(album.title.clone())
            .size(13)
            .font(cosmic::iced::Font {
                weight: cosmic::iced::font::Weight::Bold,
                ..Default::default()
            })
            .wrapping(Wrapping::None),
    )
    .width(Length::Fixed(cover_size))
    .height(Length::Fixed(18.0))
    .clip(true);

    // Track count
    let track_count = widget::text::caption(format!("{} faixas", album.tracks.len())).size(11); // TODO: Use fl! macro

    let content = widget::column::with_capacity(3)
        .push(cover_widget)
        .push(title_widget)
        .push(track_count)
        .spacing(5)
        .align_x(Alignment::Start);

    widget::button::custom(widget::container(content).padding(10).style(
        move |theme: &cosmic::Theme| {
            // Get base background color from theme
            let bg = theme.cosmic().background(false).base;

            // The card overlay color (10% lighter, 30% opacity)
            let overlay_color = cosmic::iced::Color::from_rgba(
                (bg.red + 0.1).min(1.0),
                (bg.green + 0.1).min(1.0),
                (bg.blue + 0.1).min(1.0),
                0.3,
            );

            // Calculate adaptive text color based on effective background luminance
            // For the card labels, we prioritize the local contrast of the card itself
            // BUT we also respect the global use_dark_text if it's forced by a very bright album art
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };

            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(overlay_color)),
                text_color: Some(text_color),
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        },
    ))
    .on_press(Message::OpenAlbumDetailFromArtist(
        album_idx,
        source_artist_idx,
    ))
    .class(album_card_button_style())
    .width(Length::Fixed(170.0))
    .height(Length::Fixed(220.0))
    .into()
}

/// Custom button style for album cards
///
/// Provides clean visual feedback without the oval gray background effect.
fn album_card_button_style() -> cosmic::theme::Button {
    use cosmic::iced::{Background, Color};
    use cosmic::widget::button;

    cosmic::theme::Button::Custom {
        // Normal state - transparent background
        active: Box::new(|_focused, theme| {
            let cosmic = theme.cosmic();
            button::Style {
                background: None,
                border_radius: cosmic.corner_radii.radius_s.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                outline_width: 0.0,
                outline_color: Color::TRANSPARENT,
                icon_color: None,
                text_color: None,
                shadow_offset: Vector::default(),
                overlay: None,
            }
        }),
        // Disabled state
        disabled: Box::new(|theme| {
            let cosmic = theme.cosmic();
            button::Style {
                background: None,
                border_radius: cosmic.corner_radii.radius_s.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                outline_width: 0.0,
                outline_color: Color::TRANSPARENT,
                icon_color: None,
                text_color: None,
                shadow_offset: Vector::default(),
                overlay: None,
            }
        }),
        // Hover state - subtle highlight
        hovered: Box::new(|_focused, theme| {
            let cosmic = theme.cosmic();
            let hover_color = cosmic.button.hover;
            button::Style {
                background: Some(Background::Color(hover_color.into())),
                border_radius: cosmic.corner_radii.radius_s.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                outline_width: 0.0,
                outline_color: Color::TRANSPARENT,
                icon_color: None,
                text_color: None,
                shadow_offset: Vector::default(),
                overlay: None,
            }
        }),
        // Pressed state - slightly darker
        pressed: Box::new(|_focused, theme| {
            let cosmic = theme.cosmic();
            let pressed_color = cosmic.button.pressed;
            button::Style {
                background: Some(Background::Color(pressed_color.into())),
                border_radius: cosmic.corner_radii.radius_s.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
                outline_width: 0.0,
                outline_color: Color::TRANSPARENT,
                icon_color: None,
                text_color: None,
                shadow_offset: Vector::default(),
                overlay: None,
            }
        }),
    }
}
