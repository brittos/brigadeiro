// SPDX-License-Identifier: GPL-3.0

//! Settings dialog for Brigadeiro
//!
//! This module provides a modal dialog window for configuring the application.
//! It uses libcosmic's settings widgets (Section, Item, Toggler, etc.).

use cosmic::Element;
use cosmic::iced::Length;
use cosmic::iced::alignment::Horizontal;
use cosmic::widget::{self, settings};

use crate::config::{AlbumSort, ArtistSort, Config, TrackSort};
use crate::fl;

/// Messages emitted by the settings dialog
#[derive(Debug, Clone)]
pub enum SettingsMessage {
    // Library settings
    AddMusicFolder,
    RemoveMusicFolder(usize),
    FolderSelected(Option<String>),
    RefreshLibrary,
    RescanLibrary,

    // Album settings
    SetAlbumSort(AlbumSort),
    SetAlbumShowLabels(bool),

    // Artist settings
    SetArtistSort(ArtistSort),

    // Track settings
    SetTrackSort(TrackSort),
    SetTrackShowSearch(bool),

    // Queue settings
    SetQueueOpenDefault(bool),

    // Playback settings
    SetDefaultVolume(f64),
    SetShuffleLoops(bool),
    SetAlbumFromTrack(bool),

    // Navigation
    ChangePage(SettingsPage),

    // Dialog actions
    Close,
    Save,
    ResetAll,
}

/// Current page/tab in settings dialog
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    Library,
    Interface,
    Playback,
}

impl SettingsPage {
    pub fn label(&self) -> String {
        match self {
            SettingsPage::Library => fl!("settings-library"),
            SettingsPage::Interface => fl!("settings-interface"),
            SettingsPage::Playback => fl!("settings-playback"),
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SettingsPage::Library => "folder-music-symbolic",
            SettingsPage::Interface => "preferences-desktop-appearance-symbolic",
            SettingsPage::Playback => "media-playback-start-symbolic",
        }
    }
}

/// Build the library page content
fn library_page(config: &Config) -> Element<'static, SettingsMessage> {
    // 1. Folders Section
    let mut folders_section = settings::section().title(fl!("settings-library-folders-title"));

    folders_section = folders_section.add(
        settings::item::builder(fl!("folder-add-title"))
            .description(fl!("folder-add-desc"))
            .control(
                widget::button::suggested(fl!("btn-add")).on_press(SettingsMessage::AddMusicFolder),
            ),
    );

    for (index, folder) in config.music_folders.iter().enumerate() {
        let folder_name = folder.clone();
        let idx = index;
        folders_section = folders_section.add(
            settings::item::builder(folder_name).control(
                widget::button::destructive("Remover")
                    .on_press(SettingsMessage::RemoveMusicFolder(idx)),
            ),
        );
    }

    if config.music_folders.is_empty() {
        folders_section =
            folders_section.add(widget::text::body(fl!("empty-folder-list")).width(Length::Fill));
    }

    // 2. Maintenance Section
    let maintenance_section = settings::section()
        .title(fl!("settings-library-maintenance-title"))
        .add(
            settings::item::builder(fl!("btn-refresh-library"))
                .description(fl!("btn-refresh-library-desc"))
                .control(
                    widget::button::standard(fl!("btn-refresh"))
                        .on_press(SettingsMessage::RefreshLibrary),
                ),
        );

    // 3. Danger Zone Section
    let danger_section = settings::section()
        .title(fl!("settings-library-danger-title"))
        .add(
            settings::item::builder(fl!("btn-rescan-library"))
                .description(fl!("btn-rescan-library-desc"))
                .control(
                    widget::button::destructive(fl!("btn-reset"))
                        .on_press(SettingsMessage::RescanLibrary),
                ),
        );

    widget::scrollable(
        widget::column::with_capacity(3)
            .spacing(24)
            .push(folders_section)
            .push(maintenance_section)
            .push(danger_section)
            .padding(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Build the interface page content
fn interface_page(config: &Config) -> Element<'static, SettingsMessage> {
    let album_section = settings::section()
        .title(fl!("settings-albums-title"))
        .add(
            settings::item::builder(fl!("sort-default"))
                .description(fl!("sort-default"))
                .control(album_sort_dropdown(config.album_sort)),
        )
        .add(
            settings::item::builder(fl!("show-labels"))
                .description(fl!("show-labels-desc"))
                .toggler(
                    config.album_show_labels,
                    SettingsMessage::SetAlbumShowLabels,
                ),
        );

    let artist_section = settings::section()
        .title(fl!("settings-artists-title"))
        .add(
            settings::item::builder(fl!("sort-default"))
                .description(fl!("sort-default"))
                .control(artist_sort_dropdown(config.artist_sort)),
        );

    let track_section = settings::section()
        .title(fl!("settings-tracks-title"))
        .add(
            settings::item::builder(fl!("sort-default"))
                .description(fl!("sort-default"))
                .control(track_sort_dropdown(config.track_sort)),
        )
        .add(
            settings::item::builder(fl!("open-search"))
                .description(fl!("open-search-desc"))
                .toggler(
                    config.track_show_search,
                    SettingsMessage::SetTrackShowSearch,
                ),
        );

    let queue_section = settings::section().title(fl!("settings-queue-title")).add(
        settings::item::builder(fl!("queue-default"))
            .description(fl!("queue-default-desc"))
            .toggler(
                config.queue_open_default,
                SettingsMessage::SetQueueOpenDefault,
            ),
    );

    widget::scrollable(
        widget::column::with_capacity(4)
            .spacing(24)
            .push(album_section)
            .push(artist_section)
            .push(track_section)
            .push(queue_section)
            .padding(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Build the playback page content
fn playback_page(config: &Config) -> Element<'static, SettingsMessage> {
    let playback_section = settings::section()
        .title(fl!("settings-playback-title"))
        .add(
            settings::item::builder(fl!("default-volume"))
                .description(fl!("default-volume-desc"))
                .control(volume_slider(config.default_volume)),
        )
        .add(
            settings::item::builder(fl!("shuffle-loops"))
                .description(fl!("shuffle-loops-desc"))
                .toggler(config.shuffle_loops, SettingsMessage::SetShuffleLoops),
        )
        .add(
            settings::item::builder(fl!("album-from-track"))
                .description(fl!("album-from-track-desc"))
                .toggler(config.album_from_track, SettingsMessage::SetAlbumFromTrack),
        );

    widget::scrollable(
        widget::column::with_capacity(1)
            .spacing(16)
            .push(playback_section)
            .padding(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Create album sort dropdown
fn album_sort_dropdown(current: AlbumSort) -> Element<'static, SettingsMessage> {
    let options = vec![
        fl!("sort-title"),
        fl!("sort-artist"),
        fl!("sort-release"),
        fl!("sort-duration"),
        fl!("sort-tracks"),
    ];
    let selected = match current {
        AlbumSort::Title => Some(0),
        AlbumSort::Artist => Some(1),
        AlbumSort::ReleaseDate => Some(2),
        AlbumSort::Duration => Some(3),
        AlbumSort::TrackCount => Some(4),
    };

    widget::dropdown(options, selected, |idx| {
        let sort = match idx {
            0 => AlbumSort::Title,
            1 => AlbumSort::Artist,
            2 => AlbumSort::ReleaseDate,
            3 => AlbumSort::Duration,
            _ => AlbumSort::TrackCount,
        };
        SettingsMessage::SetAlbumSort(sort)
    })
    .width(Length::Fixed(180.0))
    .into()
}

/// Create artist sort dropdown
fn artist_sort_dropdown(current: ArtistSort) -> Element<'static, SettingsMessage> {
    let options = vec![
        fl!("sort-artist"), // Assuming sort-artist maps to "By Name" in this context or create a specific key
        fl!("sort-albums"),
        fl!("sort-songs"),
    ];
    let selected = match current {
        ArtistSort::Name => Some(0),
        ArtistSort::AlbumCount => Some(1),
        ArtistSort::TrackCount => Some(2),
    };

    widget::dropdown(options, selected, |idx| {
        let sort = match idx {
            0 => ArtistSort::Name,
            1 => ArtistSort::AlbumCount,
            _ => ArtistSort::TrackCount,
        };
        SettingsMessage::SetArtistSort(sort)
    })
    .width(Length::Fixed(180.0))
    .into()
}

/// Create track sort dropdown
fn track_sort_dropdown(current: TrackSort) -> Element<'static, SettingsMessage> {
    let options = vec![
        fl!("sort-title"),
        fl!("sort-album"),
        fl!("sort-artist"),
        fl!("sort-release"),
        fl!("sort-duration"),
    ];
    let selected = match current {
        TrackSort::Title => Some(0),
        TrackSort::Album => Some(1),
        TrackSort::Artist => Some(2),
        TrackSort::ReleaseDate => Some(3),
        TrackSort::Duration => Some(4),
    };

    widget::dropdown(options, selected, |idx| {
        let sort = match idx {
            0 => TrackSort::Title,
            1 => TrackSort::Album,
            2 => TrackSort::Artist,
            3 => TrackSort::ReleaseDate,
            _ => TrackSort::Duration,
        };
        SettingsMessage::SetTrackSort(sort)
    })
    .width(Length::Fixed(180.0))
    .into()
}

/// Create volume slider
fn volume_slider(current: f64) -> Element<'static, SettingsMessage> {
    #[allow(clippy::cast_possible_truncation)]
    let percentage = (current * 100.0) as f32; // Safe: volume is 0.0..=1.0, result is 0.0..=100.0

    #[allow(clippy::cast_possible_truncation)]
    let percentage_int = percentage.round() as i32; // Safe: percentage is 0..=100

    widget::row::with_capacity(2)
        .spacing(12)
        .align_y(cosmic::iced::Alignment::Center)
        .push(
            cosmic::iced::widget::slider(0.0..=100.0, percentage, |val| {
                SettingsMessage::SetDefaultVolume(f64::from(val / 100.0))
            })
            .width(Length::Fixed(180.0)),
        )
        .push(widget::text::body(fl!(
            "percentage",
            value = percentage_int
        )))
        .into()
}

/// Build the tab bar for navigation
fn page_tabs(current: SettingsPage) -> Element<'static, SettingsMessage> {
    let pages = [
        SettingsPage::Library,
        SettingsPage::Interface,
        SettingsPage::Playback,
    ];

    let tabs: Vec<Element<'static, SettingsMessage>> = pages
        .iter()
        .map(|page| {
            let is_active = *page == current;
            let label = page.label();
            let page_clone = *page;

            if is_active {
                widget::button::suggested(label).into()
            } else {
                widget::button::standard(label)
                    .on_press(SettingsMessage::ChangePage(page_clone))
                    .into()
            }
        })
        .collect();

    // Create a centered row with horizontal space on both sides
    let tabs_row = widget::row::with_children(tabs)
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center);

    widget::row::with_capacity(3)
        .push(widget::horizontal_space().width(Length::Fill))
        .push(tabs_row)
        .push(widget::horizontal_space().width(Length::Fill))
        .padding([8, 16])
        .width(Length::Fill)
        .into()
}

/// Build the main settings view for a given page
pub fn view(config: &Config, current_page: SettingsPage) -> Element<'static, SettingsMessage> {
    let content = match current_page {
        SettingsPage::Library => library_page(config),
        SettingsPage::Interface => interface_page(config),
        SettingsPage::Playback => playback_page(config),
    };

    // Header with title
    let header = widget::row::with_capacity(3)
        .push(widget::text::title3(fl!("settings-title")).width(Length::Fill))
        .push(widget::horizontal_space().width(Length::Fill))
        .padding(16)
        .align_y(cosmic::iced::Alignment::Center);

    // Footer with actions
    let footer = widget::row::with_capacity(3)
        .push(widget::button::destructive(fl!("btn-reset")).on_press(SettingsMessage::ResetAll))
        .push(widget::horizontal_space().width(Length::Fill))
        .push(widget::button::standard(fl!("btn-close")).on_press(SettingsMessage::Close))
        .padding(16)
        .spacing(8);

    let content_column = widget::column::with_capacity(6)
        .push(header)
        .push(widget::divider::horizontal::default())
        .push(page_tabs(current_page))
        .push(widget::divider::horizontal::default())
        .push(content)
        .push(widget::divider::horizontal::default())
        .push(footer)
        .push(
            widget::container(
                widget::text(fl!(
                    "git-description",
                    hash = env!("GIT_COMMIT_HASH"),
                    date = env!("GIT_COMMIT_DATE")
                )), // .style(cosmic::theme::Text::Muted)
            )
            .padding([0, 0, 16, 0])
            .width(Length::Fill)
            .align_x(Horizontal::Center),
        )
        .width(Length::Fixed(500.0))
        .height(Length::Fixed(600.0));

    widget::container(content_column)
        .style(|_theme| cosmic::iced::widget::container::Style {
            background: Some(cosmic::iced::Background::Color(
                cosmic::iced::Color::from_rgb8(50, 50, 50),
            )),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Apply a settings message to the config
/// Returns true if the config was modified
pub fn apply_message(config: &mut Config, message: &SettingsMessage) -> bool {
    match message {
        SettingsMessage::SetAlbumSort(sort) => {
            config.album_sort = *sort;
            true
        }
        SettingsMessage::SetAlbumShowLabels(value) => {
            config.album_show_labels = *value;
            true
        }
        SettingsMessage::SetArtistSort(sort) => {
            config.artist_sort = *sort;
            true
        }
        SettingsMessage::SetTrackSort(sort) => {
            config.track_sort = *sort;
            true
        }
        SettingsMessage::SetTrackShowSearch(value) => {
            config.track_show_search = *value;
            true
        }
        SettingsMessage::SetQueueOpenDefault(value) => {
            config.queue_open_default = *value;
            true
        }
        SettingsMessage::SetDefaultVolume(value) => {
            config.default_volume = *value;
            true
        }
        SettingsMessage::SetShuffleLoops(value) => {
            config.shuffle_loops = *value;
            true
        }
        SettingsMessage::SetAlbumFromTrack(value) => {
            config.album_from_track = *value;
            true
        }
        SettingsMessage::RemoveMusicFolder(index) => {
            if *index < config.music_folders.len() {
                config.music_folders.remove(*index);
                true
            } else {
                false
            }
        }
        SettingsMessage::FolderSelected(Some(folder)) => {
            config.add_music_folder(folder.clone());
            true
        }
        SettingsMessage::ResetAll => {
            *config = Config::default();
            true
        }
        _ => false,
    }
}
