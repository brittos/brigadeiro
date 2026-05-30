// SPDX-License-Identifier: GPL-3.0
//
// Queue Sidebar - Shows the current playback queue
// Features: Edit mode, Clear, Toggle visibility

use crate::app::Message;
use crate::model::track::Track;
use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::widget::dnd_destination::dnd_destination_for_data;
use std::borrow::Cow;
use std::collections::HashMap;
//use std::convert::Infallible;
use std::sync::Arc;

const QUEUE_ITEM_MIME: &str = "application/x-brigadeiro-queue-item";

#[derive(Debug, Clone, Copy)]
pub struct QueueItemMime(pub usize);

impl cosmic::iced::clipboard::mime::AllowedMimeTypes for QueueItemMime {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![QUEUE_ITEM_MIME.to_string()])
    }
}

impl cosmic::iced::clipboard::mime::AsMimeTypes for QueueItemMime {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![QUEUE_ITEM_MIME.to_string()])
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        (mime_type == QUEUE_ITEM_MIME).then(|| self.0.to_le_bytes().to_vec().into())
    }
}

impl From<(Vec<u8>, String)> for QueueItemMime {
    fn from(value: (Vec<u8>, String)) -> Self {
        let bytes = value.0;
        let index = if bytes.len() >= 8 {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[..8]);
            usize::from_le_bytes(arr)
        } else {
            // Se for menor que 8 bytes, tenta ler o que tiver ou assume 0
            let mut arr = [0u8; 8];
            let len = bytes.len().min(8);
            arr[..len].copy_from_slice(&bytes[..len]);
            usize::from_le_bytes(arr)
        };
        QueueItemMime(index)
    }
}

/// Constants for layout
const SIDEBAR_WIDTH: f32 = 350.0;
const COVER_SIZE: f32 = 50.0;
const ROW_HEIGHT: f32 = 66.0;

/// Format duration as "Xh `XXmin`" or "XX min"
fn format_remaining_time(seconds: f64) -> String {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let total_minutes = (seconds.max(0.0) / 60.0).ceil() as u64; // Safe: duration is always non-negative
    if total_minutes >= 60 {
        let hours = total_minutes / 60;
        let mins = total_minutes % 60;
        format!("{hours}h {mins}min restantes")
    } else {
        format!("{total_minutes} min restantes")
    }
}

fn view_sidebar_header(
    track_count: usize,
    remaining_time: f64,
    edit_mode: bool,
) -> Element<'static, Message> {
    let title = widget::text::title4("Fila de Reprodução"); // TODO: Use fl! macro
    let count_badge = widget::text::caption(format!("{track_count} faixas")).size(11); // TODO: Use fl! macro
    let time_label = widget::text::caption(format_remaining_time(remaining_time)).size(11);

    let clear_button = widget::button::icon(widget::icon::from_name("edit-clear-all-symbolic"))
        .on_press(Message::ClearQueue)
        .class(cosmic::theme::Button::Text);

    let edit_icon = if edit_mode {
        "emblem-ok-symbolic"
    } else {
        "document-edit-symbolic"
    };
    let edit_button = widget::button::icon(widget::icon::from_name(edit_icon))
        .on_press(Message::ToggleQueueEditMode)
        .class(cosmic::theme::Button::Text);

    let close_button = widget::button::icon(widget::icon::from_name("window-close-symbolic"))
        .on_press(Message::ToggleQueue)
        .class(cosmic::theme::Button::Text);

    let header_info = widget::column::with_capacity(2)
        .push(
            widget::row::with_capacity(2)
                .push(title)
                .push(count_badge)
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .push(time_label)
        .spacing(2)
        .width(Length::Fill);

    let header_buttons = widget::row::with_capacity(3)
        .push(edit_button)
        .push(clear_button)
        .push(close_button)
        .spacing(4);

    widget::row::with_capacity(2)
        .push(header_info)
        .push(header_buttons)
        .padding([12, 16])
        .align_y(Alignment::Center)
        .into()
}

fn view_sidebar_track_list<'a>(
    queue_tracks: &'a [Arc<Track>],
    queue_position: usize,
    covers: &'a HashMap<i64, widget::image::Handle>,
    edit_mode: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let track_rows: Vec<Element<'static, Message>> = queue_tracks
        .iter()
        .enumerate()
        .map(|(idx, track)| {
            let is_current = idx == queue_position;
            let cover = track.cover_art_id.and_then(|id| covers.get(&id)).cloned();
            queue_track_row(
                idx,
                track.as_ref(),
                cover,
                is_current,
                edit_mode,
                use_dark_text,
            )
        })
        .collect();

    if track_rows.is_empty() {
        widget::container(
            widget::column::with_capacity(2)
                .push(widget::text::body("Fila vazia").size(16)) // TODO: Use fl! macro
                .push(widget::text::caption("Adicione músicas para começar").size(12)) // TODO: Use fl! macro
                .spacing(8)
                .align_x(cosmic::iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .align_y(cosmic::iced::alignment::Vertical::Center)
        .into()
    } else {
        widget::scrollable(
            widget::column::with_children(track_rows)
                .spacing(2)
                .padding([0, 8])
                .width(Length::Fill),
        )
        .id(widget::Id::new("queue_scrollable"))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// Renders the queue sidebar
///
/// # Parameters
/// - `queue_tracks`: List of tracks in the queue
/// - `queue_position`: Current playing position
/// - `covers`: `HashMap` of cover art handles
/// - `edit_mode`: If true, show edit controls (move up/down, remove)
pub fn view(
    queue_tracks: &[Arc<Track>],
    queue_position: usize,
    covers: &HashMap<i64, widget::image::Handle>,
    edit_mode: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    // Header row 1: Title + count
    let track_count = queue_tracks.len();

    // Calculate remaining time
    let remaining_time: f64 = queue_tracks
        .iter()
        .skip(queue_position)
        .map(|t| t.duration)
        .sum();

    let header = view_sidebar_header(track_count, remaining_time, edit_mode);
    let tracks_list = view_sidebar_track_list(
        queue_tracks,
        queue_position,
        covers,
        edit_mode,
        use_dark_text,
    );

    // Sidebar container
    let content = widget::column::with_capacity(2)
        .push(header)
        .push(tracks_list)
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill);

    // Apply styling
    let sidebar_container = widget::container(content)
        .width(Length::Fixed(SIDEBAR_WIDTH))
        .height(Length::Fill)
        .style(move |theme: &cosmic::Theme| {
            let cosmic = theme.cosmic();
            let bg = cosmic.background.base;
            let _bg_color = cosmic::iced::Color::from_rgba(bg.red, bg.green, bg.blue, 1.0);
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red - 0.02,
                        bg.green - 0.02,
                        bg.blue - 0.02,
                        1.0,
                    ),
                )),
                text_color: Some(text_color),
                border: cosmic::iced::Border {
                    width: 1.0,
                    radius: 0.0.into(),
                    color: cosmic::iced::Color::from_rgba(0.3, 0.3, 0.3, 0.3),
                },
                ..Default::default()
            }
        });

    // IMPORTANTE: Envolvemos todo o sidebar em um dnd_destination "fantasma" //FIXME Traduzir
    // Isso força o registro do tipo MIME com o sistema operacional (Wayland), //FIXME Traduzir
    // garantindo que os eventos de drop sejam aceitos. //FIXME Traduzir
    widget::dnd_destination(sidebar_container, vec![Cow::Borrowed(QUEUE_ITEM_MIME)])
        .id(widget::Id::new("sidebar_dnd_dest"))
        .into()
}

fn view_row_cover(cover: Option<widget::image::Handle>) -> Element<'static, Message> {
    if let Some(handle) = cover {
        widget::container(
            widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(cosmic::iced::ContentFit::Contain),
        )
        .width(Length::Fixed(COVER_SIZE))
        .height(Length::Fixed(COVER_SIZE))
        .clip(true)
        .style(|_| cosmic::widget::container::Style {
            border: cosmic::iced::Border {
                radius: 6.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    } else {
        widget::container(
            widget::text::body("🎵")
                .size(18)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .width(Length::Fixed(COVER_SIZE))
        .height(Length::Fixed(COVER_SIZE))
        .style(|theme: &cosmic::Theme| {
            let bg = theme.cosmic().background.base;
            cosmic::widget::container::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red + 0.1,
                        bg.green + 0.1,
                        bg.blue + 0.1,
                        0.6,
                    ),
                )),
                border: cosmic::iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }
}

fn view_edit_mode_row(
    idx: usize,
    cover_widget: Element<'static, Message>,
    info_column: Element<'static, Message>,
    playing_icon: Element<'static, Message>,
    is_current: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let drag_handle_icon = widget::text::body("≡").size(25).width(Length::Fixed(30.0));
    let drag_handle = widget::dnd_source(drag_handle_icon).drag_content(move || QueueItemMime(idx));

    let move_up_btn = widget::button::icon(widget::icon::from_name("go-up-symbolic"))
        .on_press(Message::MoveQueueTrack {
            from: idx,
            to: idx.saturating_sub(1),
        })
        .class(cosmic::theme::Button::Text);

    let move_down_btn = widget::button::icon(widget::icon::from_name("go-down-symbolic"))
        .on_press(Message::MoveQueueTrack {
            from: idx,
            to: idx + 1,
        })
        .class(cosmic::theme::Button::Text);

    let remove_btn = widget::button::icon(widget::icon::from_name("user-trash-symbolic"))
        .on_press(Message::RemoveFromQueue(idx))
        .class(cosmic::theme::Button::Destructive);

    let controls = widget::row::with_capacity(3)
        .push(move_up_btn)
        .push(move_down_btn)
        .push(remove_btn)
        .spacing(2);

    let row_content = widget::row::with_capacity(5)
        .push(drag_handle)
        .push(cover_widget)
        .push(info_column)
        .push(playing_icon)
        .push(controls)
        .spacing(8)
        .padding([6, 8])
        .align_y(Alignment::Center);

    let row_button = widget::button::custom(row_content)
        .on_press(Message::GoToQueuePosition(idx))
        .width(Length::Fill)
        .class(queue_track_button_style(is_current, use_dark_text));

    let destination =
        dnd_destination_for_data::<QueueItemMime, Message>(row_button, move |data, _action| {
            if let Some(data) = data {
                Message::MoveQueueTrack {
                    from: data.0,
                    to: idx,
                }
            } else {
                Message::NoOp
            }
        });

    widget::container(destination)
        .width(Length::Fill)
        .height(Length::Fixed(ROW_HEIGHT))
        .into()
}

fn view_normal_mode_row(
    idx: usize,
    cover_widget: Element<'static, Message>,
    info_column: Element<'static, Message>,
    playing_icon: Element<'static, Message>,
    is_current: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let drag_cover = widget::dnd_source(cover_widget).drag_content(move || QueueItemMime(idx));

    let row_content = widget::row::with_capacity(3)
        .push(drag_cover)
        .push(info_column)
        .push(playing_icon)
        .spacing(10)
        .padding([6, 12])
        .align_y(Alignment::Center);

    let row_button = widget::button::custom(row_content)
        .on_press(Message::GoToQueuePosition(idx))
        .width(Length::Fill)
        .class(queue_track_button_style(is_current, use_dark_text));

    let destination =
        dnd_destination_for_data::<QueueItemMime, Message>(row_button, move |data, _action| {
            if let Some(data) = data {
                Message::MoveQueueTrack {
                    from: data.0,
                    to: idx,
                }
            } else {
                Message::NoOp
            }
        });

    widget::container(destination)
        .width(Length::Fill)
        .height(Length::Fixed(ROW_HEIGHT))
        .into()
}

/// Renders a single queue track row
fn queue_track_row(
    idx: usize,
    track: &Track,
    cover: Option<widget::image::Handle>,
    is_current: bool,
    edit_mode: bool,
    use_dark_text: bool,
) -> Element<'static, Message> {
    let cover_widget = view_row_cover(cover);

    let playing_icon: Element<Message> = if is_current {
        widget::text::body("▶")
            .size(14)
            .width(Length::Fixed(20.0))
            .into()
    } else {
        widget::horizontal_space().width(Length::Fixed(20.0)).into()
    };

    let info_column = widget::column::with_capacity(2)
        .push(
            widget::text::body(track.title.clone())
                .size(13)
                .font(cosmic::iced::Font {
                    weight: cosmic::iced::font::Weight::Medium,
                    ..Default::default()
                }),
        )
        .push(widget::text::caption(track.album.clone()).size(11))
        .spacing(2)
        .width(Length::Fill)
        .into();

    if edit_mode {
        view_edit_mode_row(
            idx,
            cover_widget,
            info_column,
            playing_icon,
            is_current,
            use_dark_text,
        )
    } else {
        view_normal_mode_row(
            idx,
            cover_widget,
            info_column,
            playing_icon,
            is_current,
            use_dark_text,
        )
    }
}

/// Custom button style for queue track rows
fn queue_track_button_style(is_current: bool, use_dark_text: bool) -> cosmic::theme::Button {
    if is_current {
        queue_track_button_style_current(use_dark_text)
    } else {
        queue_track_button_style_normal()
    }
}

fn queue_track_button_style_current(use_dark_text: bool) -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(move |_focused, theme| {
            let cosmic = theme.cosmic();
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(cosmic.accent.base.into())),
                text_color: Some(text_color),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
        disabled: Box::new(move |theme| {
            let cosmic = theme.cosmic();
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(cosmic.accent.base.into())),
                text_color: Some(text_color),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
        hovered: Box::new(move |_focused, theme| {
            let cosmic = theme.cosmic();
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(cosmic.accent.hover.into())),
                text_color: Some(text_color),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
        pressed: Box::new(move |_focused, theme| {
            let cosmic = theme.cosmic();
            let text_color = if use_dark_text {
                cosmic::iced::Color::BLACK
            } else {
                cosmic::iced::Color::WHITE
            };
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic.accent.pressed.into(),
                )),
                text_color: Some(text_color),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
    }
}

fn queue_track_button_style_normal() -> cosmic::theme::Button {
    cosmic::theme::Button::Custom {
        active: Box::new(|_focused, _theme| cosmic::widget::button::Style {
            background: None,
            border_radius: 8.0.into(),
            ..Default::default()
        }),
        disabled: Box::new(|_theme| cosmic::widget::button::Style {
            background: None,
            border_radius: 8.0.into(),
            ..Default::default()
        }),
        hovered: Box::new(|_focused, theme| {
            let cosmic = theme.cosmic();
            let bg = cosmic.background.base;
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red + 0.08,
                        bg.green + 0.08,
                        bg.blue + 0.08,
                        0.7,
                    ),
                )),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
        pressed: Box::new(|_focused, theme| {
            let cosmic = theme.cosmic();
            let bg = cosmic.background.base;
            cosmic::widget::button::Style {
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(
                        bg.red + 0.12,
                        bg.green + 0.12,
                        bg.blue + 0.12,
                        0.8,
                    ),
                )),
                border_radius: 8.0.into(),
                ..Default::default()
            }
        }),
    }
}
