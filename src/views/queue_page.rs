// SPDX-License-Identifier: GPL-3.0
//
// Queue Page - Expanded player view for the "Fila" tab
use crate::app::Message;
use crate::model::track::Track;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use std::collections::HashMap;
use std::sync::Arc;
/// Constants for queue page layout
const ART_SIZE: f32 = 300.0;
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
) -> Element<'static, Message> {
    // Large cover art (300x300)
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
                    radius: 12.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into()
        } else {
            // Placeholder
            widget::container(
                widget::text::body("🎵")
                    .size(64)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center),
            )
            .width(Length::Fixed(ART_SIZE))
            .height(Length::Fixed(ART_SIZE))
            .style(|theme: &cosmic::Theme| {
                let bg = theme.cosmic().background(false).base;
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
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
        };
    // Track info (centered)
    let title = widget::text::title2(track.title.clone())
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);
    let artist = widget::text::body(track.artist.clone())
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);
    let album = widget::text::caption(track.album.clone())
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center);
    
    let info_column = widget::column::with_capacity(3)
        .push(title)
        .push(artist)
        .push(album)
        .spacing(4)
        .align_x(cosmic::iced::alignment::Horizontal::Center);

    widget::column::with_capacity(3)
        .push(cover_widget)
        .push(widget::space().height(Length::Fixed(20.0)))
        .push(info_column)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .into()
}

fn view_playback_timeline(
    current_position: f64,
    duration: f64,
    tertiary: Option<cosmic::iced::Color>,
    secondary: Option<cosmic::iced::Color>,
) -> Element<'static, Message> {
    // State 1: Resting (Active)
    let active_style = std::rc::Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 9.0, 1.0, 2.0, 0.0)
    });

    // State 2: Interactive (Hovered/Dragging)
    let interactive_style = std::rc::Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 12.0, 0.9, 3.1, 0.9)
    });

    // Generate progress slider
    let safe_duration = if duration > 0.0 { duration } else { 1.0 };
    let safe_position = current_position.clamp(0.0, safe_duration);

    let progress_slider = widget::slider(0.0..=safe_duration, safe_position, |v| {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Message::Seek(v.max(0.0) as u64) // Safe: slider value is clamped to 0.0..=duration
    })
    .width(Length::Fixed(400.0))
    .class(cosmic::theme::iced::Slider::Custom {
        active: active_style.clone(),
        hovered: interactive_style.clone(),
        dragging: interactive_style.clone(),
    });

    let time_display = widget::row::with_capacity(2)
        .push(widget::text::caption(format_duration(current_position)))
        .push(widget::space().width(Length::Fill))
        .push(widget::text::caption(format_duration(duration)))
        .width(Length::Fixed(400.0));

    widget::column::with_capacity(2)
        .push(progress_slider)
        .push(time_display)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .into()
}

fn view_playback_controls_and_options(
    is_playing: bool,
    volume: f32,
    use_dark_text: bool,
    tertiary: Option<cosmic::iced::Color>,
    secondary: Option<cosmic::iced::Color>,
) -> Element<'static, Message> {
    // Playback controls (large)
    let prev_btn = widget::button::icon(widget::icon::from_name("media-skip-backward-symbolic"))
        .on_press(Message::Previous)
        .class(cosmic::theme::Button::Text);
    let play_pause_icon = if is_playing {
        "media-playback-pause-symbolic"
    } else {
        "media-playback-start-symbolic"
    };
    let play_btn = widget::button::icon(widget::icon::from_name(play_pause_icon))
        .on_press(Message::TogglePlayback)
        .class(crate::views::suggested_button_style(use_dark_text));
    let next_btn = widget::button::icon(widget::icon::from_name("media-skip-forward-symbolic"))
        .on_press(Message::Next)
        .class(cosmic::theme::Button::Text);
    let controls = widget::row::with_capacity(3)
        .push(prev_btn)
        .push(play_btn)
        .push(next_btn)
        .spacing(20)
        .align_y(Alignment::Center);
    // Toggle sidebar button
    let sidebar_btn = widget::button::icon(widget::icon::from_name("dock-left-symbolic"))
        .on_press(Message::ToggleQueue)
        .class(cosmic::theme::Button::Text);

    // Volume controls
    let volume_icon = if volume <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if volume < 0.33 {
        "audio-volume-low-symbolic"
    } else if volume < 0.66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    };

    let vol_ico = widget::icon::from_name(volume_icon).size(16);

    // Volume Styles (using slightly smaller handles)
    let vol_active_style = std::rc::Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 7.0, 1.0, 2.0, 0.0)
    });
    let vol_interactive_style = std::rc::Rc::new(move |theme: &cosmic::Theme| {
        slider_style_logic(theme, tertiary, secondary, 10.0, 0.9, 3.1, 0.9)
    });

    let volume_slider = widget::slider(0.0..=1.0, f64::from(volume), |v| {
        #[allow(clippy::cast_possible_truncation)]
        Message::VolumeChanged(v as f32) // Safe: volume is 0.0..=1.0, f32 is sufficient
    })
    .step(0.01)
    .width(Length::Fixed(120.0))
    .class(cosmic::theme::iced::Slider::Custom {
        active: vol_active_style,
        hovered: vol_interactive_style.clone(),
        dragging: vol_interactive_style,
    });

    let options_row = widget::row::with_capacity(4)
        .push(sidebar_btn)
        .push(widget::space().width(Length::Fill))
        .push(vol_ico)
        .push(volume_slider)
        .align_y(Alignment::Center)
        .width(Length::Fixed(400.0)); // Match progress slider width
    
    widget::column::with_capacity(3)
        .push(controls)
        .push(widget::space().height(Length::Fixed(10.0)))
        .push(options_row)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .into()
}

/// Renders the Queue Page (Fila tab content)
///
/// Layout: Centered column with [Large Cover] | [Track Info] | [Progress] | [Controls] | [Options]
#[allow(clippy::too_many_arguments)]
pub fn view(
    current_track: Option<&Arc<Track>>,
    covers: &HashMap<i64, widget::image::Handle>,
    is_playing: bool,
    current_position: f64,
    duration: f64,
    volume: f32,
    _queue_visible: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let Some(track) = current_track else {
        // Empty state when no track is playing
        return widget::container(
            widget::column::with_capacity(2)
                .push(
                    widget::text::title1("Fila de Reprodução") // TODO: Use fl! macro
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                .push(
                    widget::text::body("Adicione músicas à fila para começar") // TODO: Use fl! macro
                        .width(Length::Fill)
                        .align_x(cosmic::iced::alignment::Horizontal::Center),
                )
                .spacing(20)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
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
        })
        .into();
    };

    let tertiary = track.tertiary_color;
    let secondary = track.secondary_color;

    let track_info = view_track_info(track, covers);
    let timeline = view_playback_timeline(current_position, duration, tertiary, secondary);
    let controls = view_playback_controls_and_options(is_playing, volume, use_dark_text, tertiary, secondary);

    // Main layout
    let content = widget::column::with_capacity(5)
        .push(track_info)
        .push(widget::space().height(Length::Fixed(20.0)))
        .push(timeline)
        .push(widget::space().height(Length::Fixed(20.0)))
        .push(controls)
        .align_x(cosmic::iced::alignment::Horizontal::Center);
    
    widget::container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
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
        })
        .into()
}
