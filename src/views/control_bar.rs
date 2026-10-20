// SPDX-License-Identifier: GPL-3.0
//
// Control Bar - Persistent bottom bar with playback controls
// [Art+Info] | [Prev/Play/Next | Slider | Time] | [Volume | Loop/Repeat/Shuffle | GoToQueue]

use crate::app::Message;
use crate::model::track::Track;
use crate::player::queue::RepeatMode;
use crate::views::track_actions;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Constants for control bar dimensions
const BAR_HEIGHT: f32 = 100.0;
const ART_SIZE: f32 = 70.0;

/// Format duration as mm:ss
/// Format duration as mm:ss or hh:mm:ss
fn format_duration(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total_seconds = seconds.max(0.0) as u64; // Safe: duration is always non-negative
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

/// Helper function for style logic (avoids closure capture lifetime issues)
fn slider_style_logic(
    theme: &cosmic::Theme,
    tertiary: Option<cosmic::iced::Color>,
    secondary: Option<cosmic::iced::Color>,
    handle_radius: f32,
    handle_alpha: f32,
    border_width: f32,
    border_alpha: f32,
) -> cosmic::widget::slider::Style {
    let cosmic = theme.cosmic();

    // Define colors
    let rail_fill = tertiary.unwrap_or(cosmic.accent.base.into());

    // Handle color logic
    let base_handle = tertiary.unwrap_or(cosmic.accent.base.into());
    let handle_color = cosmic::iced::Color {
        a: handle_alpha,
        ..base_handle
    };
    let border_color = cosmic::iced::Color {
        a: border_alpha,
        ..base_handle
    };

    let breakpoint_color = secondary.unwrap_or(cosmic.success.base.color.into());

    // Default empty rail color (grayish)
    let rail_empty = cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.2);

    cosmic::widget::slider::Style {
        rail: cosmic::widget::slider::Rail {
            backgrounds: (
                cosmic::iced::Background::Color(rail_fill),
                cosmic::iced::Background::Color(rail_empty),
            ),
            width: 4.0,
            border: cosmic::iced::Border {
                radius: 2.0.into(),
                width: 0.0,
                color: cosmic::iced::Color::TRANSPARENT,
            },
        },
        handle: cosmic::widget::slider::Handle {
            shape: cosmic::widget::slider::HandleShape::Circle {
                radius: handle_radius,
            },
            background: cosmic::iced::Background::Color(handle_color),
            border_width,
            border_color,
        },
        breakpoint: cosmic::widget::slider::Breakpoint {
            color: breakpoint_color,
        },
    }
}

fn view_track_info<'a>(
    track: &'a Arc<Track>,
    covers: &'a HashMap<i64, widget::image::Handle>,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // Cover art
    let cover_widget: Element<Message> =
        if let Some(handle) = track.cover_art_id.and_then(|id| covers.get(&id)).cloned() {
            widget::container(
                widget::image(handle)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(cosmic::iced::ContentFit::Cover),
            )
            .width(Length::Fixed(ART_SIZE))
            .height(Length::Fixed(ART_SIZE))
            .clip(true)
            .style(|_| cosmic::widget::container::Style {
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        } else {
            // Placeholder
            widget::container(
                widget::text::body("🎵")
                    .size(28)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center),
            )
            .width(Length::Fixed(ART_SIZE))
            .height(Length::Fixed(ART_SIZE))
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().background.base;
                cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(
                            bg.red + 0.1,
                            bg.green + 0.1,
                            bg.blue + 0.1,
                            0.8,
                        ),
                    )),
                    border: cosmic::iced::Border {
                        radius: 8.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
        };

    // Track info (title, album, artist)
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
        .width(Length::FillPortion(1));

    // Favorite heart next to the track info
    let favorite_btn = track_actions::favorite_button(track.id, track.favorite, use_dark_text);

    widget::row::with_capacity(3)
        .push(cover_widget)
        .push(info_column)
        .push(favorite_btn)
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(1))
        .into()
}

fn view_playback_controls(
    is_playing: bool,
    current_position: f64,
    duration: f64,
    use_dark_text: bool,
    tertiary: Option<cosmic::iced::Color>,
    secondary: Option<cosmic::iced::Color>,
) -> Element<'static, Message> {
    // Previous button
    let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
        .on_press(Message::Previous)
        .class(cosmic::theme::Button::Standard);

    // Play/Pause button
    let play_icon = if is_playing {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    };
    let play_btn = widget::button::icon(widget::icon::from_name(play_icon))
        .on_press(Message::TogglePlayback)
        .class(crate::views::suggested_button_style(use_dark_text));

    // Next button
    let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
        .on_press(Message::Next)
        .class(cosmic::theme::Button::Standard);

    // Controls row
    let controls = widget::row::with_capacity(3)
        .push(prev_btn)
        .push(play_btn)
        .push(next_btn)
        .spacing(8)
        .align_y(Alignment::Center);

    // Progress slider
    // Clamp duration to avoid bad slider behavior
    let safe_duration = if duration > 0.0 { duration } else { 1.0 };
    // Clamp current position to duration
    let safe_position = current_position.clamp(0.0, safe_duration);

    // State 1: Resting (Active)
    // Raio 12.0, 60% opacidade
    let active_style = Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 9.0, 1.0, 2.0, 0.0)
    });

    // State 2: Interactive (Hovered/Dragging)
    // Raio 13.0, 100% opacidade no core (para destaque), Halo 4px com 30% opacidade
    let interactive_style = Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 12.0, 0.9, 3.1, 0.9)
    });

    let progress_slider = widget::slider(0.0..=safe_duration, safe_position, |v| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Message::Seek(v.max(0.0) as u64) // Safe: slider value is clamped
    })
    .width(Length::Fixed(400.0))
    .class(cosmic::theme::iced::Slider::Custom {
        active: active_style,
        hovered: interactive_style.clone(),
        dragging: interactive_style,
    });

    // Time labels
    let current_time = widget::text::caption(format_duration(current_position)).size(11);
    let total_time = widget::text::caption(format_duration(duration)).size(11);

    // Time + progress row
    let time_row = widget::row::with_capacity(3)
        .push(current_time)
        .push(progress_slider)
        .push(total_time)
        .spacing(12)
        .align_y(Alignment::Center);

    widget::column::with_capacity(2)
        .push(controls)
        .push(time_row)
        .spacing(4)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .width(Length::FillPortion(2))
        .into()
}

fn view_extra_controls(
    volume: f32,
    shuffle: bool,
    repeat_mode: RepeatMode,
    use_dark_text: bool,
    tertiary: Option<cosmic::iced::Color>,
    secondary: Option<cosmic::iced::Color>,
) -> Element<'static, Message> {
    // Volume Slider
    let volume_icon = if volume <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if volume < 0.33 {
        "audio-volume-low-symbolic"
    } else if volume < 0.66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    };

    // Icon (not clickable for now, just visual indicator)
    let vol_ico = widget::icon::from_name(volume_icon).size(16);

    // State 1: Resting (Active) - Slightly smaller for volume
    let active_style = Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 7.0, 1.0, 2.0, 0.0)
    });

    // State 2: Interactive (Hovered/Dragging)
    let interactive_style = Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 10.0, 0.9, 3.1, 0.9)
    });

    let volume_slider = widget::slider(0.0..=1.0, f64::from(volume), |v| {
        #[allow(clippy::cast_possible_truncation)]
        Message::VolumeChanged(v as f32) // Safe: volume is 0.0..=1.0
    })
    .step(0.01)
    .width(Length::Fixed(100.0))
    .class(cosmic::theme::iced::Slider::Custom {
        active: active_style,
        hovered: interactive_style.clone(),
        dragging: interactive_style,
    });

    // Loop/Repeat button
    let repeat_icon = match repeat_mode {
        RepeatMode::LoopSong => "media-playlist-repeat-song-symbolic",
        _ => "media-playlist-repeat-symbolic",
    };

    let repeat_class = if repeat_mode == RepeatMode::Normal {
        cosmic::theme::Button::Text
    } else {
        crate::views::suggested_button_style(use_dark_text)
    };

    let loop_btn = widget::button::icon(widget::icon::from_name(repeat_icon))
        .on_press(Message::ToggleRepeat)
        .class(repeat_class);

    // Shuffle button
    let shuffle_class = if shuffle {
        crate::views::suggested_button_style(use_dark_text)
    } else {
        cosmic::theme::Button::Text
    };

    let shuffle_btn =
        widget::button::icon(widget::icon::from_name("media-playlist-shuffle-symbolic"))
            .on_press(Message::ToggleShuffle)
            .class(shuffle_class);

    // Go to Queue button
    let queue_btn = widget::button::icon(widget::icon::from_name("view-list-symbolic"))
        .on_press(Message::ToggleQueue)
        .class(cosmic::theme::Button::Text);

    widget::row::with_capacity(6)
        .push(vol_ico)
        .push(volume_slider)
        .push(widget::horizontal_space().width(Length::Fixed(12.0)))
        .push(loop_btn)
        .push(shuffle_btn)
        .push(widget::horizontal_space().width(Length::Fixed(8.0)))
        .push(queue_btn)
        .spacing(4)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(1))
        .into()
}

/// Renders the control bar (persistent bottom bar)
///
/// Layout: [Art + Info] | [Controls + Slider] | [Volume + Buttons + `GoToQueue`]
pub fn view(
    current_track: Option<&Arc<Track>>,
    covers: &HashMap<i64, widget::image::Handle>,
    is_playing: bool,
    current_position: f64,
    duration: f64,
    volume: f32,
    shuffle: bool,
    repeat_mode: RepeatMode,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // If no track, return empty bar
    let Some(track) = current_track else {
        return widget::container(widget::Space::new(Length::Fill, Length::Fixed(0.0)))
            .width(Length::Fill)
            .into();
    };

    // Shared styling logic and colors
    let tertiary = track.tertiary_color;
    let secondary = track.secondary_color;

    // Build the sections
    let left_section = view_track_info(track, covers, use_dark_text);
    let center_section = view_playback_controls(
        is_playing,
        current_position,
        duration,
        use_dark_text,
        tertiary,
        secondary,
    );
    let right_section = view_extra_controls(
        volume,
        shuffle,
        repeat_mode,
        use_dark_text,
        tertiary,
        secondary,
    );

    // Full control bar
    let bar_content = widget::row::with_capacity(3)
        .push(left_section)
        .push(center_section)
        .push(right_section)
        .spacing(20)
        .padding([10, 20])
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT));

    // Container with styling
    widget::container(bar_content)
        .width(Length::Fill)
        .height(Length::Fixed(BAR_HEIGHT))
        .style(move |theme: &cosmic::Theme| {
            let cosmic = theme.cosmic();
            let bg = cosmic.background.base;

            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };

            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red, bg.green, bg.blue,
                        0.4, // Reduced for better blur effect visibility (normal:0.7)
                    ),
                )),
                text_color: Some(text_color),
                border: cosmic::iced::Border {
                    width: 1.0,
                    radius: 0.0.into(),
                    color: cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.1),
                },
                ..Default::default()
            }
        })
        .into()
}
