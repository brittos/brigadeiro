pub mod album_detail_page;
pub mod albums_page;
pub mod artist_detail_page;
pub mod artists_page;
pub mod control_bar;
pub mod onboarding_page;
pub mod playlist_detail_page;
pub mod playlists_page;
pub mod queue_page;
pub mod queue_sidebar;
pub mod settings_dialog;
pub mod track_entry;
pub mod tracks_page;

/// Shared button style for "Suggested" buttons that adapt to accent color brightness.
pub fn suggested_button_style(use_dark_text: bool) -> cosmic::theme::Button {
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
