// SPDX-License-Identifier: GPL-3.0
//
// Albums Page - Grid view of albums

use crate::app::Message;
use crate::model::track::Track;
use cosmic::Element;
use cosmic::iced::{Alignment, Length, Vector};
use cosmic::widget;
use std::collections::HashMap;

/// Represents album data for display in the grid
#[derive(Debug, Clone)]
pub struct AlbumData {
    pub _id: Option<i64>,
    pub title: String,
    pub artist: String,
    pub cover_art_id: Option<i64>,
    pub accent_color: Option<cosmic::iced::Color>,
    pub secondary_color: Option<cosmic::iced::Color>,
    pub tertiary_color: Option<cosmic::iced::Color>,
    pub tracks: Vec<Track>,
}

impl PartialEq for AlbumData {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.artist == other.artist
    }
}

/// Constants for virtualization calculations
const CARD_HEIGHT: f32 = 280.0;
const CARD_WIDTH: f32 = 220.0;
const SPACING: f32 = 20.0;
const PADDING: f32 = 20.0;
const BUFFER_ROWS: usize = 2; // Extra rows above/below viewport for smooth scrolling

/// Renders the album grid view with virtualization for performance
///
/// Only renders visible albums + buffer, using spacers to maintain scroll height.
/// Returns 'static Element because it clones all necessary data.
pub fn view(
    albums: &[AlbumData],
    covers: &HashMap<i64, widget::image::Handle>,
    scroll_y: f32,
    viewport_h: f32,
    viewport_w: f32,
    use_dark_text: bool,
) -> Element<'static, Message> {
    if albums.is_empty() {
        return widget::container(
            widget::text::title3("Nenhum álbum encontrado") // TODO: Use fl! macro
                .width(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into();
    }

    // Use real viewport width for column calculation
    let available_width = viewport_w - PADDING * 2.0;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cols = ((available_width + SPACING) / (CARD_WIDTH + SPACING)).floor() as usize;
    let cols = cols.max(1);

    // Calculate row dimensions
    let row_height = CARD_HEIGHT + SPACING;
    let total_rows = albums.len().div_ceil(cols);

    // Calculate visible row range
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_row = (scroll_y / row_height).floor() as usize;
    let first_row = first_row.saturating_sub(BUFFER_ROWS);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let visible_rows = (viewport_h / row_height).ceil() as usize + 1 + BUFFER_ROWS * 2;
    let last_row = (first_row + visible_rows).min(total_rows);

    // Calculate album index range
    let first_idx = first_row * cols;
    let last_idx = (last_row * cols).min(albums.len());

    // Calculate spacer heights
    #[allow(clippy::cast_precision_loss)]
    let top_height = (first_row as f32 * row_height).max(0.0);
    let bottom_rows = total_rows.saturating_sub(last_row);
    #[allow(clippy::cast_precision_loss)]
    let bottom_height = (bottom_rows as f32 * row_height).max(0.0);

    // Build only visible album cards
    let visible_albums = &albums[first_idx..last_idx];
    let cards: Vec<Element<'static, Message>> = visible_albums
        .iter()
        .enumerate()
        .map(|(visible_idx, album)| {
            let album_idx = first_idx + visible_idx; // Actual index in albums
            let cover = album.cover_art_id.and_then(|id| covers.get(&id)).cloned();
            album_card(
                album_idx,
                album.title.clone(),
                album.artist.clone(),
                cover,
                use_dark_text,
            )
        })
        .collect();

    // Build layout with spacers
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let spacing_u16 = SPACING as u16; // Safe: SPACING is a small constant
    let content = widget::Column::new()
        .push(widget::space().height(Length::Fixed(top_height)))
        .push(
            widget::flex_row(cards)
                .spacing(spacing_u16)
                .padding(PADDING)
                .width(Length::Fill)
                .align_items(Alignment::Start)
                .justify_content(cosmic::widget::JustifyContent::Center),
        )
        .push(widget::space().height(Length::Fixed(bottom_height)));

    widget::scrollable(content)
        .on_scroll(Message::AlbumsScrolled)
        .into()
}

/// Renders a single album card
///
/// Accepts owned data to produce a 'static Element
fn album_card(
    album_idx: usize,
    title: String,
    artist: String,
    cover: Option<widget::image::Handle>,
    use_dark_text: bool,
) -> Element<'static, Message> {
    use cosmic::iced::widget::text::Wrapping;

    // Adaptive text color based on background luminance
    let text_color = if use_dark_text {
        cosmic::iced::Color::BLACK
    } else {
        cosmic::iced::Color::WHITE
    };

    let cover_widget: Element<Message> = if let Some(handle) = cover {
        // Wrap image in container with clip to prevent overflow
        widget::container(
            widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Contain),
        )
        //.width(Length::Fixed(200.0)) //150.0
        //.height(Length::Fixed(190.0)) //150.0
        .clip(true)
        .into()
    } else {
        widget::Column::new()
            .push(widget::space().height(Length::Fixed(19.0))) // Espaço em cima
            .push(
                widget::container(
                    widget::text::title1("🎵")
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center)
                        .align_y(cosmic::iced::alignment::Vertical::Center),
                )
                .style(|_| cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(0.2, 0.2, 0.25, 0.8),
                    )),
                    ..Default::default()
                }),
            )
            .into()
    };
    // Title with ellipsis-like behavior (no wrap + fixed height = clipped)
    let title_widget = widget::container(
        widget::text::body(title)
            .size(12)
            .font(cosmic::iced::Font {
                weight: cosmic::iced::font::Weight::Bold,
                ..Default::default()
            })
            .wrapping(Wrapping::None)
            .class(cosmic::theme::Text::Color(text_color)),
    )
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center)
    .width(Length::Fixed(200.0)) //150.0
    .height(Length::Fixed(20.0)) // Fixed height for single line
    .clip(true); // Clip overflowing text

    // Artist with ellipsis-like behavior
    let artist_widget = widget::container(
        widget::text::caption(artist)
            .size(12)
            .wrapping(Wrapping::None)
            .class(cosmic::theme::Text::Color(text_color)),
    )
    .align_x(cosmic::iced::alignment::Horizontal::Center)
    .align_y(cosmic::iced::alignment::Vertical::Center)
    .width(Length::Fixed(200.0)) //150.0
    .height(Length::Fixed(16.0)) // Fixed height for single line
    .clip(true); // Clip overflowing text

    let content = widget::Column::new()
        .push(cover_widget)
        .push(title_widget)
        .push(artist_widget)
        .spacing(5)
        .align_x(Alignment::Center);

    widget::button::custom(
        widget::container(content)
            .padding(10)
            .style(|theme: &cosmic::Theme| {
                // Get base background color from theme
                let bg = theme.cosmic().background(false).base;

                // Make 10% lighter and 10% transparent
                let lighter_color = cosmic::iced::Color::from_rgba(
                    (bg.red + 0.1).min(1.0),
                    (bg.green + 0.1).min(1.0),
                    (bg.blue + 0.1).min(1.0),
                    0.3, // 0.1 = 10% opacity
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
    .on_press(Message::OpenAlbumDetail(album_idx))
    .class(album_card_button_style())
    .width(Length::Fixed(220.0)) // Fixed card width (200px content + 20px padding)
    .height(Length::Fixed(280.0)) // Fixed card height (200 cover + 20+16 text + spacing + padding)
    .into()
}

/// Custom button style for album cards
///
/// Provides clean visual feedback without the oval gray background effect.
/// Can be extended to use album accent colors in the future.
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
