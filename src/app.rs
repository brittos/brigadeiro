// SPDX-License-Identifier: GPL-3.0

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::Config;
use crate::fl;
use crate::model::track::Track;
use crate::player::audio_backend::{AudioBackend, PlaybackEvent, PlaybackState};
use crate::player::mpris::{self, MprisAction, MprisStateUpdate};
use crate::player::queue::RepeatMode;
use crate::player::state::PlayerState;
use crate::views::album_detail_page;
use crate::views::albums_page::{self, AlbumData};
use crate::views::artist_detail_page;
use crate::views::artists_page::{self, ArtistData};
use crate::views::control_bar;
use crate::views::queue_sidebar;
use crate::views::settings_dialog::{self, SettingsMessage, SettingsPage};
use crate::views::track_actions::TrackActionsContext;
use crate::views::tracks_page;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::{Length, Subscription, time, window};
use cosmic::prelude::*;
use cosmic::widget::{self};
use tokio::sync::mpsc;

const APP_ID: &str = "io.github.brittos.brigadeiro";
const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const TRACKS_SCROLL_ID: &str = "tracks_scroll";

/// Navigation pages for the library view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavPage {
    Queue,
    Tracks,
    #[default]
    Albums,
    Artists,
    Playlists,
}

/// Application view state for stack navigation
#[derive(Debug, Clone, PartialEq)]
pub enum AppView {
    /// Main grid view (albums, artists, etc.)
    Grid,
    /// Album detail page with track list
    /// (`album_idx`, `source_artist_idx`) - `source_artist_idx` is Some when navigating from artist detail
    AlbumDetail(usize, Option<usize>),
    /// Artist detail page with albums
    ArtistDetail(usize), // index into library_artists
    /// Playlist detail page with tracks
    PlaylistDetail(i64), // playlist ID
    /// Built-in "Favorites" collection, rendered with the playlist detail page
    Favorites,
}

/// Calculate perceived luminance of a color (0.0 = black, 1.0 = white)
/// Uses the formula for relative luminance from WCAG 2.0
fn luminance(color: cosmic::iced::Color) -> f32 {
    // sRGB to linear conversion
    let to_linear = |c: f32| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = to_linear(color.r);
    let g = to_linear(color.g);
    let b = to_linear(color.b);
    // Weighted sum for perceived brightness
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// Returns true if we should use dark (black) text on this background color
fn should_use_dark_text(color: cosmic::iced::Color) -> bool {
    luminance(color) > 0.4 // Threshold: if brighter than 40%, use dark text
}

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Configuration data that persists between application runs.
    config: Config,
    /// The dominant color extracted from the album art
    theme_color: Option<cosmic::iced::Color>,
    /// The secondary color for accents/borders
    theme_color_secondary: Option<cosmic::iced::Color>,
    /// The tertiary color for fine details
    theme_color_tertiary: Option<cosmic::iced::Color>,
    /// Background image handle (album art)
    background_image: Option<cosmic::iced::widget::image::Handle>,
    /// Dimensions of the background image (width, height)
    background_image_dims: Option<(u32, u32)>,
    /// Player state for UI binding
    player_state: PlayerState,
    /// Current track path
    current_track_path: Option<PathBuf>,
    /// Audio backend for playback
    audio_backend: Option<Arc<Mutex<AudioBackend>>>,
    /// Event receiver from audio backend
    event_rx: Option<Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<PlaybackEvent>>>>,
    /// MPRIS action receiver (commands from D-Bus controllers)
    mpris_rx: Option<Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<MprisAction>>>>,
    /// MPRIS state sender (to push state updates to the MPRIS thread)
    mpris_tx: Option<mpsc::UnboundedSender<MprisStateUpdate>>,
    /// Database persistence
    database: Option<crate::database::Database>,
    /// Library tracks
    library_tracks: Vec<Track>,
    /// ID of the separate settings window, if open
    settings_window_id: Option<cosmic::iced::window::Id>,

    /// Library albums, grouped from tracks
    library_albums: Vec<AlbumData>,
    /// Library artists, grouped from tracks
    library_artists: Vec<ArtistData>,
    /// Current navigation page
    current_page: NavPage,
    /// Whether to use dark text (true) or light text (false) based on background luminance
    use_dark_text: bool,
    /// Cache of loaded cover images (`cover_art_id` -> Handle)
    covers_cache: std::collections::HashMap<i64, widget::image::Handle>,
    /// Albums scroll Y position for virtualization
    albums_scroll_y: f32,
    /// Albums viewport height for virtualization
    albums_viewport_h: f32,
    /// Albums viewport width for virtualization
    albums_viewport_w: f32,
    /// Tracks scroll request offset
    tracks_scroll_offset: cosmic::iced::widget::scrollable::AbsoluteOffset,
    /// Whether the settings dialog is visible
    settings_open: bool,
    /// Current page in settings dialog
    settings_page: SettingsPage,
    /// Current view state (grid or detail)
    current_view: AppView,
    /// Tracks in the playback queue
    queue_tracks: Vec<std::sync::Arc<crate::model::track::Track>>,
    /// Current position in the queue
    queue_position: usize,
    /// Whether the queue sidebar is visible
    queue_visible: bool,
    /// Whether edit mode is active in queue sidebar
    queue_edit_mode: bool,
    /// Library playlists
    library_playlists: Vec<crate::model::playlist::Playlist>,
    /// Whether playlist edit mode is active
    playlist_edit_mode: bool,
    /// Tracks for the currently viewed playlist (cached for detail page)
    playlist_tracks: Vec<crate::model::track::Track>,
    /// Playlists scroll Y
    playlists_scroll_y: f32,
    /// Track id whose "Add to playlist" menu is open, if any
    add_to_playlist_menu: Option<i64>,
    /// Synthetic playlist used to render the Favorites collection
    favorites_playlist: crate::model::playlist::Playlist,

    // Default cover art assets for fallback
    /// Default background image handle
    default_background_image: Option<cosmic::iced::widget::image::Handle>,
    /// Default background image dimensions
    default_background_image_dims: Option<(u32, u32)>,
    /// Default theme colors (accent, secondary, tertiary)
    default_theme_colors: (
        cosmic::iced::Color,
        cosmic::iced::Color,
        cosmic::iced::Color,
    ),
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    UpdateConfig(Config),
    LoadMusic,
    MusicLoaded(Option<crate::utils::metadata::AudioMetadata>),
    // File/Folder picker
    OpenFilePicker,
    FileSelected(Option<std::path::PathBuf>),
    OpenFolderPicker,
    FolderSelected(Option<std::path::PathBuf>),
    // Playback controls
    Play,
    Pause,
    Stop,
    TogglePlayback,
    Previous,
    Next,
    // Playback events from backend
    PlaybackTick(u64),
    PlaybackDuration(u64),
    Seek(u64),
    VolumeChanged(f32),
    PlaybackStateChanged(PlaybackState),
    PlaybackEnded,
    TimerTick,
    ToggleShuffle,
    ToggleRepeat,
    // Window controls
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    WindowDrag,
    /// Window was created (multi-window)
    WindowCreated(cosmic::iced::window::Id),
    /// Window was closed
    WindowClosed(cosmic::iced::window::Id),
    /// Settings window was closed
    SettingsWindowClosed,
    // Library
    LibraryLoaded(Vec<crate::model::track::Track>),
    /// Background processing results for visual metadata (batch)
    VisualsProcessed(Vec<crate::model::track::Track>),
    /// Navigation page change
    NavTo(NavPage),
    /// Load missing covers for albums
    LoadCovers,
    /// Covers loaded from database
    CoversLoaded(Vec<(i64, widget::image::Handle)>),
    /// Refresh Library (Scan all existing folders)
    RefreshLibrary,
    /// Rescan Library (Clear DB and re-scan existing folders)
    RescanLibrary,
    /// Albums page scrolled - for virtualization
    AlbumsScrolled(cosmic::iced::widget::scrollable::Viewport),
    /// Tracks page scrolled - for virtualization
    TracksScroll(cosmic::iced::widget::scrollable::Viewport),
    /// Open settings dialog
    OpenSettings,
    /// Settings dialog message
    Settings(SettingsMessage),
    /// Change settings page
    SettingsChangePage(SettingsPage),
    /// Open album detail page (from albums grid)
    OpenAlbumDetail(usize),
    /// Open album detail page from artist detail
    OpenAlbumDetailFromArtist(usize, usize), // (album_idx, artist_idx)
    /// Go back to album grid or artist detail (depending on context)
    BackToAlbums(Option<usize>), // source_artist_idx
    /// Play album starting from track index
    PlayAlbumFromTrack(usize, usize), // album_idx, track_idx
    /// Play individual track from library
    PlayTrack(usize), // index into library_tracks
    /// Play all tracks from an artist
    PlayArtistTracks(usize), // index into library_artists
    /// Open artist detail page
    OpenArtistDetail(usize), // index into library_artists
    /// Go back to artist grid
    BackToArtists,
    /// Toggle queue sidebar visibility
    ToggleQueue,
    /// Clear the playback queue
    ClearQueue,
    /// Toggle queue edit mode (show/hide controls)
    ToggleQueueEditMode,
    /// Go to specific position in queue
    GoToQueuePosition(usize),
    /// Add track to queue
    AddToQueue(usize), // index into library_tracks
    /// Move track in queue from one position to another
    MoveQueueTrack {
        from: usize,
        to: usize,
    },
    /// Remove track from queue at position
    RemoveFromQueue(usize),
    // Playlists
    LoadPlaylists,
    PlaylistsLoaded(Vec<crate::model::playlist::Playlist>),
    CreatePlaylist(String),
    DeletePlaylist(i64),
    RenamePlaylist(i64, String),
    UpdatePlaylistDescription(i64, String),
    TogglePlaylistEditMode(bool),
    OpenPlaylistDetail(i64),
    PlayPlaylist(i64),
    AddTrackToPlaylist(i64, i64),
    RemoveTrackFromPlaylist(i64, i64),
    PlaylistsScrolled(cosmic::iced::widget::scrollable::Viewport),
    PlaylistTracksLoaded(Vec<crate::model::track::Track>),
    /// Reload the tracks of the playlist being viewed (keeps edit mode)
    ReloadPlaylistTracks(i64),
    /// The contents of a playlist changed: refresh counts and the open page
    PlaylistTracksChanged(i64),
    /// Toggle the "Add to playlist" popover for a track
    OpenAddToPlaylistMenu(i64),
    /// Close the "Add to playlist" popover of a track (ignored if another is open)
    CloseAddToPlaylistMenu(i64),
    /// Create a new playlist and add the track to it
    CreatePlaylistWithTrack(i64),
    // Favorites
    /// Toggle the favorite flag of a track (by track id)
    ToggleFavorite(i64),
    /// Open the built-in Favorites collection
    OpenFavorites,
    /// MPRIS D-Bus action received from external controller
    MprisCommand(MprisAction),
    NoOp,
}

impl AppModel {
    /// Shared context for the per-track action buttons (favorite, add to playlist).
    fn track_actions_ctx(&self) -> TrackActionsContext<'_> {
        TrackActionsContext {
            playlists: &self.library_playlists,
            open_menu: self.add_to_playlist_menu,
            use_dark_text: self.use_dark_text,
        }
    }

    /// Helper to group tracks into albums for display.
    fn refresh_albums(&self) -> Vec<AlbumData> {
        use std::collections::HashMap;
        let mut albums_map: HashMap<(String, String), AlbumData> = HashMap::new();

        for track in &self.library_tracks {
            let album_key = (track.album.clone(), track.artist.clone());

            let album_data = albums_map.entry(album_key).or_insert_with(|| AlbumData {
                _id: track.album_id,
                title: track.album.clone(),
                artist: if track.album_artist.is_empty() {
                    track.artist.clone()
                } else {
                    track.album_artist.clone()
                },
                cover_art_id: track.cover_art_id,
                accent_color: None,
                secondary_color: None,
                tertiary_color: None,
                tracks: Vec::new(),
            });

            // Set colors from the first track that has them
            if album_data.accent_color.is_none() {
                album_data.accent_color = track.accent_color;
            }
            if album_data.secondary_color.is_none() {
                album_data.secondary_color = track.secondary_color;
            }
            if album_data.tertiary_color.is_none() {
                album_data.tertiary_color = track.tertiary_color;
            }

            album_data.tracks.push(track.clone());
        }

        let mut albums: Vec<AlbumData> = albums_map.into_values().collect();
        albums.sort_by(|a, b| a.title.cmp(&b.title));
        albums
    }

    /// Groups library tracks into artists for the artist grid view
    fn refresh_artists(&self) -> Vec<ArtistData> {
        use std::collections::{HashMap, HashSet};
        let mut artists_map: HashMap<String, ArtistData> = HashMap::new();

        for track in &self.library_tracks {
            let artist_name = if track.album_artist.is_empty() {
                track.artist.clone()
            } else {
                track.album_artist.clone()
            };

            let artist_data =
                artists_map
                    .entry(artist_name.clone())
                    .or_insert_with(|| ArtistData {
                        name: artist_name,
                        album_count: 0,
                        track_count: 0,
                        tracks: Vec::new(),
                    });

            artist_data.tracks.push(track.clone());
        }

        // Calculate album counts from unique albums
        for artist in artists_map.values_mut() {
            let unique_albums: HashSet<String> =
                artist.tracks.iter().map(|t| t.album.clone()).collect();
            artist.album_count = unique_albums.len();
            artist.track_count = artist.tracks.len();
        }

        let mut artists: Vec<ArtistData> = artists_map.into_values().collect();
        artists.sort_by(|a, b| a.name.cmp(&b.name));
        artists
    }

    /// Flip the favorite flag of a track everywhere it is cached in memory,
    /// then persist the new value in the background.
    fn toggle_favorite(&mut self, track_id: i64) -> Task<cosmic::Action<Message>> {
        // The library is the source of truth; fall back to other caches for
        // tracks that are playing but not (yet) in the library.
        let current = self
            .library_tracks
            .iter()
            .find(|t| t.id == track_id)
            .map(|t| t.favorite)
            .or_else(|| {
                self.player_state
                    .current_track
                    .as_ref()
                    .filter(|t| t.id == track_id)
                    .map(|t| t.favorite)
            })
            .or_else(|| {
                self.queue_tracks
                    .iter()
                    .find(|t| t.id == track_id)
                    .map(|t| t.favorite)
            });
        let Some(current) = current else {
            return Task::none();
        };
        let favorite = !current;

        let set = |t: &mut Track| {
            if t.id == track_id {
                t.favorite = favorite;
            }
        };
        self.library_tracks.iter_mut().for_each(set);
        self.library_albums
            .iter_mut()
            .flat_map(|a| a.tracks.iter_mut())
            .for_each(set);
        self.library_artists
            .iter_mut()
            .flat_map(|a| a.tracks.iter_mut())
            .for_each(set);
        self.playlist_tracks.iter_mut().for_each(set);
        // The Favorites page lists `playlist_tracks`: keep it in sync so the
        // heart removes the row (or adds it when favorited from the control bar).
        if self.current_view == AppView::Favorites {
            if favorite {
                if !self.playlist_tracks.iter().any(|t| t.id == track_id)
                    && let Some(track) = self.library_tracks.iter().find(|t| t.id == track_id)
                {
                    self.playlist_tracks.push(track.clone());
                    self.playlist_tracks
                        .sort_by_key(|t| t.title.to_lowercase());
                }
            } else {
                self.playlist_tracks.retain(|t| t.id != track_id);
            }
        }
        for track in &mut self.queue_tracks {
            if track.id == track_id {
                Arc::make_mut(track).favorite = favorite;
            }
        }
        if let Some(track) = self.player_state.current_track.as_mut()
            && track.id == track_id
        {
            Arc::make_mut(track).favorite = favorite;
        }

        let db = self.database.clone();
        Task::perform(
            async move {
                if let Some(db) = db {
                    let _ = tokio::task::spawn_blocking(move || {
                        db.set_track_favorite(track_id, favorite)
                    })
                    .await;
                }
            },
            |()| Message::NoOp,
        )
        .map(cosmic::Action::App)
    }

    /// Update player state metadata based on current queue
    fn update_queue_metadata(&mut self) {
        self.player_state.is_empty = self.queue_tracks.is_empty();

        // Calculate total remaining time from current position to end
        let remaining_secs: f64 = if self.queue_position < self.queue_tracks.len() {
            self.queue_tracks[self.queue_position..]
                .iter()
                .map(|t| t.duration)
                .sum()
        } else {
            0.0
        };

        self.player_state.queue_time_remaining = remaining_secs;

        // Sync MPRIS capabilities (CanGoNext/CanGoPrevious)
        self.mpris_send_capabilities();
    }
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        mut core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Disable the default COSMIC header bar
        core.window.show_headerbar = false;
        // Disable content container padding to prevent scrollbars with full-width content
        core.window.content_container = false;

        // Load and process default cover art from assets
        let default_cover_bytes = include_bytes!("../assets/default_cover1.png");
        let default_cover_img = image::load_from_memory(default_cover_bytes).ok();

        let mut theme_color = Some(cosmic::iced::Color::from_rgb8(0x3D, 0x67, 0x97));
        let mut theme_color_secondary = Some(cosmic::iced::Color::from_rgb8(0x1B, 0x2E, 0x44));
        let mut theme_color_tertiary = Some(cosmic::iced::Color::from_rgb8(0x56, 0x8A, 0xC2));
        let mut background_image = None;
        let mut background_image_dims = None;

        if let Some(img) = default_cover_img {
            let (processed_img, colors) = crate::utils::metadata::process_image_for_theme(img);
            if let Some(c) = colors.0 {
                theme_color = Some(c);
            }
            if let Some(c) = colors.1 {
                theme_color_secondary = Some(c);
            }
            if let Some(c) = colors.2 {
                theme_color_tertiary = Some(c);
            }

            let rgba = processed_img.to_rgba8();
            background_image_dims = Some((rgba.width(), rgba.height()));
            background_image = Some(widget::image::Handle::from_rgba(
                rgba.width(),
                rgba.height(),
                rgba.into_raw(),
            ));
        }

        let theme_color_for_init =
            theme_color.unwrap_or(cosmic::iced::Color::from_rgb8(0x3D, 0x67, 0x97));

        let config = cosmic_config::Config::new(Self::APP_ID, Config::VERSION)
            .map(|context| match Config::get_entry(&context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        let initial_volume = config.default_volume;

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            config,
            theme_color,
            theme_color_secondary,
            theme_color_tertiary,
            background_image: background_image.clone(),
            background_image_dims,
            player_state: {
                let mut state = PlayerState::new();
                state.volume = initial_volume;
                state
            },
            current_track_path: None,
            audio_backend: None,
            event_rx: None,
            mpris_rx: None,
            mpris_tx: None,
            database: {
                let mut db_path = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".local/share"));
                db_path.push("brigadeiro");
                let _ = std::fs::create_dir_all(&db_path);
                db_path.push("library.db");
                crate::database::Database::open(db_path).ok()
            },
            library_tracks: Vec::new(),
            settings_window_id: None,
            library_albums: Vec::new(),
            library_artists: Vec::new(),
            current_page: NavPage::default(),
            use_dark_text: theme_color.is_some_and(should_use_dark_text),
            covers_cache: std::collections::HashMap::new(),
            albums_scroll_y: 0.0,
            albums_viewport_h: 600.0,  // Default viewport height
            albums_viewport_w: 1200.0, // Default viewport width
            tracks_scroll_offset: cosmic::iced::widget::scrollable::AbsoluteOffset::default(),
            settings_open: false,
            settings_page: SettingsPage::default(),
            current_view: AppView::Grid,
            queue_tracks: Vec::new(),
            queue_position: 0,
            queue_visible: false,
            queue_edit_mode: false,
            library_playlists: Vec::new(),
            playlist_edit_mode: false,
            playlist_tracks: Vec::new(),
            playlists_scroll_y: 0.0,
            add_to_playlist_menu: None,
            favorites_playlist: crate::model::playlist::Playlist::favorites(fl!("favorites")),
            default_background_image: background_image.clone(),
            default_background_image_dims: background_image_dims,
            default_theme_colors: (
                theme_color.unwrap_or(cosmic::iced::Color::from_rgb8(0x3D, 0x67, 0x97)),
                theme_color_secondary.unwrap_or(cosmic::iced::Color::from_rgb8(0x1B, 0x2E, 0x44)),
                theme_color_tertiary.unwrap_or(cosmic::iced::Color::from_rgb8(0x56, 0x8A, 0xC2)),
            ),
        };

        // Initialize MPRIS D-Bus player on dedicated thread
        let (mpris_action_rx, mpris_state_tx) = mpris::spawn_mpris_thread();
        app.mpris_rx = Some(Arc::new(tokio::sync::Mutex::new(mpris_action_rx)));
        app.mpris_tx = Some(mpris_state_tx);

        // Initialize with default theme
        let mut theme = cosmic::cosmic_theme::Theme::dark_default();
        let rgb = cosmic::cosmic_theme::palette::rgb::Rgb::from(theme_color_for_init);
        let alpha_color = cosmic::cosmic_theme::palette::Alpha {
            color: rgb,
            alpha: 0.2,
        };
        theme.is_frosted = true;
        theme.primary.base = alpha_color;
        theme.background.base = alpha_color;
        theme.accent.base = alpha_color;
        let app_theme = cosmic::Theme::custom(std::sync::Arc::new(theme));
        let theme_task = Task::done(cosmic::Action::Cosmic(cosmic::app::Action::AppThemeChange(
            app_theme,
        )));

        // Load library from DB
        let db_clone = app.database.clone();
        let load_task = Task::perform(
            async move {
                if let Some(db) = db_clone {
                    db.get_all_tracks().unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
            Message::LibraryLoaded,
        )
        .map(cosmic::Action::App);

        // Create a startup command that sets the window title for the OS.
        let window_title = "Brigadeiro".to_string();
        let command = if let Some(id) = app.core.main_window_id() {
            Task::batch(vec![
                app.set_window_title(window_title, id),
                theme_task, // Apply theme com os circulos em artistas e no botoes do header
                load_task,
                Task::done(Message::LoadMusic).map(cosmic::Action::App),
                Task::done(Message::LoadPlaylists).map(cosmic::Action::App),
            ])
        } else {
            Task::batch(vec![
                theme_task,
                load_task,
                Task::done(Message::LoadMusic).map(cosmic::Action::App),
                Task::done(Message::LoadPlaylists).map(cosmic::Action::App),
            ])
        };

        (app, command)
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        // Adaptive text color based on background luminance
        let text_color = if self.use_dark_text {
            cosmic::iced::Color::BLACK
        } else {
            cosmic::iced::Color::WHITE
        };

        // Custom Window Controls
        // Custom Window Controls with adaptive color
        // Window controls (settings, rescan, minimize, maximize, close)
        let controls = widget::row::with_capacity(5)
            .push(
                widget::button::custom(
                    cosmic::widget::icon::from_name("emblem-system-symbolic")
                        .icon()
                        .class(custom_icon_theme(text_color)),
                )
                .on_press(Message::OpenSettings),
            )
            .push(
                widget::button::custom(
                    cosmic::widget::icon::from_name("window-minimize-symbolic")
                        .icon()
                        .class(custom_icon_theme(text_color)),
                )
                .on_press(Message::WindowMinimize),
            )
            .push(
                widget::button::custom(
                    cosmic::widget::icon::from_name("window-maximize-symbolic")
                        .icon()
                        .class(custom_icon_theme(text_color)),
                )
                .on_press(Message::WindowMaximize),
            )
            .push(
                widget::button::custom(
                    cosmic::widget::icon::from_name("window-close-symbolic")
                        .icon()
                        .class(custom_icon_theme(text_color)),
                )
                .on_press(Message::WindowClose),
            )
            .spacing(8);

        // Create the drag area (Bottom Layer)
        // We use MouseArea because standard Buttons trigger on release, but dragging requires press.
        let drag_button = cosmic::widget::MouseArea::new(
            widget::horizontal_space().width(Length::Fill).height(40),
        )
        .on_press(Message::WindowDrag)
        .on_double_click(Message::WindowMaximize);

        // Create the controls row (Top Layer)
        let controls_container = widget::container(controls)
            .width(Length::Fill)
            .align_x(Horizontal::Right);

        // Navigation tabs with adaptive text color
        let nav_button =
            |page: NavPage, label: &str, _icon: &str, text_col: cosmic::iced::Color| {
                let label_text = label.to_string();
                // Create custom styled button content
                let content = widget::text::body(label_text)
                    .class(cosmic::theme::Text::Color(text_col))
                    .size(16);
                widget::button::custom(content)
                    .on_press(Message::NavTo(page))
                    .class(cosmic::theme::Button::HeaderBar)
            };

        let nav_tabs = widget::row::with_capacity(4)
            .push(nav_button(
                NavPage::Queue,
                &fl!("nav-queue"),
                "",
                text_color,
            ))
            .push(nav_button(
                NavPage::Tracks,
                &fl!("nav-tracks"),
                "",
                text_color,
            ))
            .push(nav_button(
                NavPage::Albums,
                &fl!("nav-albums"),
                "",
                text_color,
            ))
            .push(nav_button(
                NavPage::Artists,
                &fl!("nav-artists"),
                "",
                text_color,
            ))
            .push(nav_button(
                NavPage::Playlists,
                &fl!("nav-playlists"),
                "",
                text_color,
            ))
            .spacing(4);

        let nav_container = widget::container(nav_tabs)
            .align_x(Horizontal::Center)
            .width(Length::Fill);

        // Stack them: Drag button fills everything, Nav tabs in center, Controls on right
        let header =
            crate::stack![drag_button, nav_container, controls_container].width(Length::Fill);

        // Content area based on current page
        let page_content: Element<Message> = if self.library_tracks.is_empty()
            && self.config.music_folders.is_empty()
            && self.current_page != NavPage::Queue
        {
            use crate::views::onboarding_page;
            onboarding_page::view(self.use_dark_text)
        } else {
            match self.current_page {
                NavPage::Tracks => {
                    let current_path = self
                        .current_track_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string());

                    tracks_page::view(
                        &self.library_tracks,
                        &self.covers_cache,
                        current_path,
                        self.tracks_scroll_offset,
                        self.track_actions_ctx(),
                    )
                }
                NavPage::Albums => {
                    // Check if we're viewing album detail or grid
                    match &self.current_view {
                        AppView::AlbumDetail(album_idx, source_artist) => {
                            if let Some(album) = self.library_albums.get(*album_idx) {
                                let current_path = self
                                    .current_track_path
                                    .as_ref()
                                    .map(|p| p.to_string_lossy().to_string());
                                album_detail_page::view(
                                    *album_idx,
                                    album,
                                    &self.covers_cache,
                                    current_path.as_deref(),
                                    *source_artist,
                                    self.track_actions_ctx(),
                                )
                            } else {
                                // Invalid album index, go back to grid
                                widget::text::body(fl!("album-not-found")).into()
                            }
                        }
                        AppView::Grid => albums_page::view(
                            &self.library_albums,
                            &self.covers_cache,
                            self.albums_scroll_y,
                            self.albums_viewport_h,
                            self.albums_viewport_w,
                            self.use_dark_text,
                        ),
                        _ => albums_page::view(
                            &self.library_albums,
                            &self.covers_cache,
                            self.albums_scroll_y,
                            self.albums_viewport_h,
                            self.albums_viewport_w,
                            self.use_dark_text,
                        ),
                    }
                }
                NavPage::Artists => {
                    // Check if we're viewing artist detail, album detail, or grid
                    match &self.current_view {
                        AppView::ArtistDetail(artist_idx) => {
                            if let Some(artist) = self.library_artists.get(*artist_idx) {
                                artist_detail_page::view(
                                    *artist_idx,
                                    artist,
                                    &self.library_albums,
                                    &self.covers_cache,
                                    self.use_dark_text,
                                )
                            } else {
                                widget::text::body(fl!("artist-not-found")).into()
                            }
                        }
                        // Allow navigating to album detail from artist page
                        AppView::AlbumDetail(album_idx, source_artist) => {
                            if let Some(album) = self.library_albums.get(*album_idx) {
                                let current_path = self
                                    .current_track_path
                                    .as_ref()
                                    .map(|p| p.to_string_lossy().to_string());
                                album_detail_page::view(
                                    *album_idx,
                                    album,
                                    &self.covers_cache,
                                    current_path.as_deref(),
                                    *source_artist,
                                    self.track_actions_ctx(),
                                )
                            } else {
                                widget::text::body(fl!("album-not-found")).into()
                            }
                        }
                        _ => artists_page::view(&self.library_artists, self.use_dark_text),
                    }
                }
                NavPage::Playlists => {
                    use crate::views::playlist_detail_page;
                    use crate::views::playlists_page;

                    match &self.current_view {
                        AppView::PlaylistDetail(id) => {
                            if let Some(playlist) =
                                self.library_playlists.iter().find(|p| p.id == *id)
                            {
                                let current_path =
                                    self.current_track_path.as_ref().and_then(|p| p.to_str());
                                playlist_detail_page::view(
                                    playlist,
                                    &self.playlist_tracks,
                                    &self.covers_cache,
                                    current_path,
                                    self.playlist_edit_mode,
                                    self.track_actions_ctx(),
                                )
                            } else {
                                widget::text::body(fl!("playlist-not-found")).into()
                            }
                        }
                        AppView::Favorites => {
                            let current_path =
                                self.current_track_path.as_ref().and_then(|p| p.to_str());
                            playlist_detail_page::view(
                                &self.favorites_playlist,
                                &self.playlist_tracks,
                                &self.covers_cache,
                                current_path,
                                false,
                                self.track_actions_ctx(),
                            )
                        }
                        _ => playlists_page::view(
                            &self.library_playlists,
                            self.library_tracks.iter().filter(|t| t.favorite).count(),
                            &self.covers_cache,
                            self.playlists_scroll_y,
                            self.albums_viewport_h,
                            self.albums_viewport_w,
                            self.use_dark_text,
                        ),
                    }
                }
                NavPage::Queue => {
                    use crate::views::queue_page;
                    #[allow(clippy::cast_precision_loss)]
                    let position = self.player_state.position as f64; // Safe: position fits in f64
                    #[allow(clippy::cast_possible_truncation)]
                    let volume = self.player_state.volume as f32; // Safe: volume is 0..=100
                    queue_page::view(
                        self.player_state.current_track.as_ref(),
                        &self.covers_cache,
                        self.player_state.is_playing(),
                        position,
                        self.player_state
                            .current_track
                            .as_ref()
                            .map_or(0.0, |t| t.duration),
                        volume,
                        self.queue_visible,
                        self.use_dark_text,
                    )
                }
            }
        };

        let content_container = widget::container(page_content)
            .padding([0, 20])
            .width(Length::Fill)
            .height(Length::Fill)
            .clip(true);

        // Create main column with header and content
        let main_column = widget::column().push(header).push(content_container);

        // Show queue sidebar when on Queue page OR when explicitly toggled visible
        let show_sidebar = self.current_page == NavPage::Queue || self.queue_visible;
        let main_content: Element<Message> = if show_sidebar {
            let queue_sidebar = queue_sidebar::view(
                &self.queue_tracks,
                self.queue_position,
                &self.covers_cache,
                self.queue_edit_mode,
                self.track_actions_ctx(),
            );

            // Queue sidebar on LEFT
            widget::row()
                .push(queue_sidebar)
                .push(main_column.width(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            main_column.into()
        };

        let container = widget::container(main_content)
            .height(Length::Fill)
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);

        let main_ui: Element<Message> = container.into();

        // Add control bar at bottom (hide on Queue page since it has its own controls)
        let final_ui: Element<Message> = if self.current_page == NavPage::Queue {
            main_ui
        } else {
            #[allow(clippy::cast_precision_loss)]
            let position = self.player_state.position as f64; // Safe: position fits in f64
            #[allow(clippy::cast_possible_truncation)]
            let volume = self.player_state.volume as f32; // Safe: volume is 0..=100
            let control_bar = control_bar::view(
                self.player_state.current_track.as_ref(),
                &self.covers_cache,
                self.player_state.is_playing(),
                position,
                self.player_state
                    .current_track
                    .as_ref()
                    .map_or(0.0, |t| t.duration),
                volume,
                self.player_state.shuffle,
                self.player_state.repeat_mode,
                self.use_dark_text,
            );
            widget::column::with_capacity(2)
                .push(main_ui)
                .push(control_bar)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        // Add settings dialog overlay if open
        let final_ui: Element<Message> = if self.settings_open {
            let settings_content =
                settings_dialog::view(&self.config, self.settings_page).map(Message::Settings);

            // Semi-transparent dark backdrop overlay to dim the background
            let backdrop = widget::container(widget::Space::new(Length::Fill, Length::Fill)).class(
                cosmic::style::Container::custom(|_theme| cosmic::widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(
                        cosmic::iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                    )),
                    ..Default::default()
                }),
            );

            // Dialog with solid opaque COSMIC native background
            let dialog_box = widget::container(settings_content)
                .padding(0)
                .class(cosmic::style::Container::Dialog);

            // Center the settings dialog
            let dialog_container = widget::container(dialog_box)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center);

            crate::stack![final_ui, backdrop, dialog_container]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            final_ui
        };

        // FINAL LAYER: Background Stack (Covers the whole window including header and control bar)
        if let (Some(handle), Some((w, h))) =
            (self.background_image.clone(), self.background_image_dims)
        {
            let bg = cosmic::iced::widget::canvas(CoverImage {
                handle,
                img_width: w,
                img_height: h,
            })
            .width(Length::Fill)
            .height(Length::Fill);

            crate::stack![bg, final_ui]
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            final_ui
        }
    }

    fn view_window(&self, id: cosmic::iced::window::Id) -> Element<'_, Self::Message> {
        if self.settings_window_id == Some(id) {
            return settings_dialog::view(&self.config, self.settings_page).map(Message::Settings);
        }

        widget::text("Unknown Window").into()
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch(vec![
            self.core()
                .watch_config::<Config>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
            self.audio_subscription(),
            self.mpris_subscription(),
            window::events().map(|(id, event)| match event {
                window::Event::Closed => Message::WindowClosed(id),
                _ => Message::TimerTick,
            }),
            time::every(std::time::Duration::from_millis(1000)).map(|_| Message::TimerTick),
        ])
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            // --- Playback controls & events ---
            Message::Play
            | Message::Pause
            | Message::Stop
            | Message::TogglePlayback
            | Message::Previous
            | Message::Next
            | Message::ToggleShuffle
            | Message::ToggleRepeat
            | Message::PlaybackTick(_)
            | Message::PlaybackDuration(_)
            | Message::Seek(_)
            | Message::VolumeChanged(_)
            | Message::PlaybackStateChanged(_)
            | Message::PlaybackEnded
            | Message::TimerTick => self.update_playback(message),

            // --- Queue management ---
            Message::ToggleQueue
            | Message::ClearQueue
            | Message::ToggleQueueEditMode
            | Message::GoToQueuePosition(_)
            | Message::AddToQueue(_)
            | Message::MoveQueueTrack { .. }
            | Message::RemoveFromQueue(_) => self.update_queue(message),

            // --- Library loading & processing ---
            Message::LoadMusic
            | Message::MusicLoaded(_)
            | Message::LibraryLoaded(_)
            | Message::VisualsProcessed(_)
            | Message::LoadCovers
            | Message::CoversLoaded(_)
            | Message::RefreshLibrary
            | Message::RescanLibrary => self.update_library(message),

            // --- Navigation ---
            Message::NavTo(_)
            | Message::AlbumsScrolled(_)
            | Message::TracksScroll(_)
            | Message::PlaylistsScrolled(_)
            | Message::OpenAlbumDetail(_)
            | Message::OpenAlbumDetailFromArtist(_, _)
            | Message::BackToAlbums(_)
            | Message::OpenArtistDetail(_)
            | Message::BackToArtists => self.update_navigation(message),

            // --- Play actions (track, album, artist, playlist) ---
            Message::PlayAlbumFromTrack(_, _)
            | Message::PlayTrack(_)
            | Message::PlayArtistTracks(_)
            | Message::PlayPlaylist(_) => self.update_play_actions(message),

            // --- Settings & window management ---
            Message::WindowMinimize
            | Message::WindowMaximize
            | Message::WindowClose
            | Message::WindowDrag
            | Message::WindowCreated(_)
            | Message::WindowClosed(_)
            | Message::SettingsWindowClosed
            | Message::OpenSettings
            | Message::SettingsChangePage(_)
            | Message::Settings(_) => self.update_settings_window(message),

            // --- Playlists CRUD ---
            Message::LoadPlaylists
            | Message::PlaylistsLoaded(_)
            | Message::CreatePlaylist(_)
            | Message::DeletePlaylist(_)
            | Message::RenamePlaylist(_, _)
            | Message::UpdatePlaylistDescription(_, _)
            | Message::TogglePlaylistEditMode(_)
            | Message::OpenPlaylistDetail(_)
            | Message::PlaylistTracksLoaded(_)
            | Message::ReloadPlaylistTracks(_)
            | Message::PlaylistTracksChanged(_)
            | Message::OpenAddToPlaylistMenu(_)
            | Message::CloseAddToPlaylistMenu(_)
            | Message::CreatePlaylistWithTrack(_)
            | Message::AddTrackToPlaylist(_, _)
            | Message::RemoveTrackFromPlaylist(_, _) => self.update_playlists(message),

            // --- File/folder operations ---
            Message::OpenFilePicker
            | Message::OpenFolderPicker
            | Message::FileSelected(_)
            | Message::FolderSelected(_) => self.update_file_ops(message),

            // --- Favorites ---
            Message::ToggleFavorite(track_id) => self.toggle_favorite(track_id),
            Message::OpenFavorites => {
                self.current_view = AppView::Favorites;
                self.playlist_edit_mode = false;
                self.add_to_playlist_menu = None;
                let db = self.database.clone();
                Task::perform(
                    async move {
                        if let Some(db) = db {
                            tokio::task::spawn_blocking(move || {
                                db.get_favorite_tracks().unwrap_or_default()
                            })
                            .await
                            .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    },
                    Message::PlaylistTracksLoaded,
                )
                .map(cosmic::Action::App)
            }

            // --- MPRIS D-Bus commands ---
            Message::MprisCommand(ref action) => self.update_mpris(action),

            // --- Inline handlers ---
            Message::UpdateConfig(config) => {
                self.config = config;
                Task::none()
            }
            Message::NoOp => Task::none(),
        }
    }
}

impl AppModel {
    fn audio_subscription(&self) -> Subscription<Message> {
        if let Some(ref event_rx) = self.event_rx {
            let event_rx = event_rx.clone();
            let stream = cosmic::iced::stream::channel(10, move |mut output| {
                let event_rx = event_rx.clone();
                async move {
                    let mut rx = event_rx.lock().await;
                    while let Some(event) = rx.recv().await {
                        let msg = match event {
                            PlaybackEvent::Tick(secs) => Message::PlaybackTick(secs),
                            PlaybackEvent::Duration(secs) => {
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                let duration = secs.max(0.0) as u64; // Safe: duration is non-negative
                                Message::PlaybackDuration(duration)
                            }
                            PlaybackEvent::StateChanged(state) => {
                                Message::PlaybackStateChanged(state)
                            }
                            PlaybackEvent::EndOfStream => Message::PlaybackEnded,
                            PlaybackEvent::Error(e) => {
                                println!("Playback error: {e}");
                                Message::NoOp
                            }
                        };
                        let _ = output.send(msg).await;
                    }
                }
            });
            Subscription::run_with_id("audio-events", stream)
        } else {
            Subscription::none()
        }
    }

    /// Subscription for MPRIS D-Bus actions (play, pause, next, etc. from external controllers)
    fn mpris_subscription(&self) -> Subscription<Message> {
        if let Some(ref mpris_rx) = self.mpris_rx {
            let mpris_rx = mpris_rx.clone();
            let stream = cosmic::iced::stream::channel(10, move |mut output| async move {
                let mut rx = mpris_rx.lock().await;
                while let Some(action) = rx.recv().await {
                    let _ = output.send(Message::MprisCommand(action)).await;
                }
            });
            Subscription::run_with_id("mpris-events", stream)
        } else {
            Subscription::none()
        }
    }

    /// Handle MPRIS D-Bus commands by mapping them to existing application messages
    fn update_mpris(&mut self, action: &MprisAction) -> Task<cosmic::Action<Message>> {
        match *action {
            MprisAction::Play => self.update_playback(Message::Play),
            MprisAction::Pause => self.update_playback(Message::Pause),
            MprisAction::PlayPause => self.update_playback(Message::TogglePlayback),
            MprisAction::Stop => self.update_playback(Message::Stop),
            MprisAction::Next => self.update_playback(Message::Next),
            MprisAction::Previous => self.update_playback(Message::Previous),
            MprisAction::Seek(offset_micros) => {
                // Convert offset in microseconds to absolute position in seconds
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let current = self.player_state.position;
                let offset_secs = offset_micros / 1_000_000;
                #[allow(clippy::cast_sign_loss)]
                let new_pos = if offset_secs < 0 {
                    current.saturating_sub(offset_secs.unsigned_abs())
                } else {
                    current.saturating_add(offset_secs as u64)
                };
                self.update_playback(Message::Seek(new_pos))
            }
            MprisAction::SetPosition(micros) => {
                #[allow(clippy::cast_sign_loss)]
                let secs = (micros / 1_000_000).max(0) as u64;
                self.update_playback(Message::Seek(secs))
            }
            MprisAction::SetVolume(vol) => {
                #[allow(clippy::cast_possible_truncation)]
                let volume = vol.clamp(0.0, 1.0) as f32;
                self.update_playback(Message::VolumeChanged(volume))
            }
            MprisAction::SetLoopStatus(loop_status) => {
                // Set repeat mode directly from D-Bus value
                let mode = mpris::from_mpris_loop_status(loop_status);
                self.player_state.repeat_mode = mode;
                self.mpris_send(MprisStateUpdate::LoopStatus(loop_status));
                self.mpris_send_capabilities();
                Task::none()
            }
            MprisAction::SetShuffle(shuffle) => {
                self.player_state.shuffle = shuffle;
                self.mpris_send(MprisStateUpdate::Shuffle(shuffle));
                Task::none()
            }
            MprisAction::Raise => {
                // Focus the main window
                if let Some(id) = self.core.main_window_id() {
                    Task::done(cosmic::Action::Cosmic(cosmic::app::Action::Focus(id)))
                } else {
                    Task::none()
                }
            }
            MprisAction::Quit => self.update_settings_window(Message::WindowClose),
        }
    }
}

fn custom_icon_theme(color: cosmic::iced::Color) -> cosmic::theme::Svg {
    use cosmic::iced::widget::svg;
    use std::rc::Rc;

    let style_fn =
        move |_theme: &cosmic::Theme| -> svg::Style { svg::Style { color: Some(color) } };

    cosmic::theme::Svg::Custom(Rc::new(style_fn))
}

struct CoverImage {
    handle: cosmic::iced::widget::image::Handle,
    img_width: u32,
    img_height: u32,
}

impl<Message> cosmic::iced::widget::canvas::Program<Message, cosmic::Theme> for CoverImage {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::iced::Renderer,
        _theme: &cosmic::Theme,
        bounds: cosmic::iced::Rectangle,
        _cursor: cosmic::iced::mouse::Cursor,
    ) -> Vec<cosmic::iced::widget::canvas::Geometry> {
        let mut frame = cosmic::iced::widget::canvas::Frame::new(renderer, bounds.size());

        // Calculate aspect ratios
        #[allow(clippy::cast_precision_loss)]
        let img_aspect = self.img_width as f32 / self.img_height as f32; // Safe: small image dimensions
        let view_aspect = bounds.width / bounds.height;

        #[allow(clippy::cast_precision_loss)]
        let scale = if view_aspect > img_aspect {
            // View is wider than image: fit width
            bounds.width / self.img_width as f32
        } else {
            // View is taller than image: fit height
            bounds.height / self.img_height as f32
        };

        // Apply Overscan (102% zoom) to ensure coverage
        let final_scale = scale * 1.02;

        #[allow(clippy::cast_precision_loss)]
        let draw_width = self.img_width as f32 * final_scale;
        #[allow(clippy::cast_precision_loss)]
        let draw_height = self.img_height as f32 * final_scale;

        // Center the image
        let x = (bounds.width - draw_width) / 2.0;
        let y = (bounds.height - draw_height) / 2.0;

        let image_bounds = cosmic::iced::Rectangle::new(
            cosmic::iced::Point::new(x, y),
            cosmic::iced::Size::new(draw_width, draw_height),
        );

        let image = cosmic::iced::widget::canvas::Image::new(self.handle.clone());
        frame.draw_image(image_bounds, image);

        vec![frame.into_geometry()]
    }
}

// =============================================================================
// Update helpers — following COSMIC pattern (cosmic-files, cosmic-store, cosmic-player)
// Each helper handles a logical group of messages.
// =============================================================================

impl AppModel {
    /// Send a state update to the MPRIS D-Bus thread (fire-and-forget)
    fn mpris_send(&self, update: MprisStateUpdate) {
        if let Some(ref tx) = self.mpris_tx {
            let _ = tx.send(update);
        }
    }

    /// Send current track metadata to MPRIS
    fn mpris_send_track_metadata(&self) {
        if let Some(ref track) = self.player_state.current_track {
            self.mpris_send(mpris::track_to_state_update(track));
        }
    }

    /// Send dynamic capabilities to MPRIS based on current queue state
    fn mpris_send_capabilities(&self) {
        let queue_len = self.queue_tracks.len();
        let queue_pos = self.queue_position;
        let repeat = self.player_state.repeat_mode;

        let can_next = queue_pos + 1 < queue_len
            || repeat == RepeatMode::Loop
            || repeat == RepeatMode::LoopSong;
        let can_prev = queue_pos > 0 || repeat == RepeatMode::Loop;

        self.mpris_send(MprisStateUpdate::CanGoNext(can_next));
        self.mpris_send(MprisStateUpdate::CanGoPrevious(can_prev));
    }

    /// Handle playback control messages
    #[allow(clippy::too_many_lines)]
    fn update_playback(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::Play => {
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.play();
                    self.mpris_send(MprisStateUpdate::PlaybackStatus(
                        mpris_server::PlaybackStatus::Playing,
                    ));
                }
            }
            Message::Pause => {
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.pause();
                    self.mpris_send(MprisStateUpdate::PlaybackStatus(
                        mpris_server::PlaybackStatus::Paused,
                    ));
                }
            }
            Message::Stop => {
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.stop();
                    self.mpris_send(MprisStateUpdate::PlaybackStatus(
                        mpris_server::PlaybackStatus::Stopped,
                    ));
                }
            }
            Message::TogglePlayback => {
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.toggle();
                }
            }
            Message::ToggleShuffle => {
                self.player_state.shuffle = !self.player_state.shuffle;
                println!("Telemetry: Shuffle is now {}", self.player_state.shuffle);
                self.mpris_send(MprisStateUpdate::Shuffle(self.player_state.shuffle));
            }
            Message::ToggleRepeat => {
                self.player_state.repeat_mode = match self.player_state.repeat_mode {
                    RepeatMode::Normal => RepeatMode::Loop,
                    RepeatMode::Loop => RepeatMode::LoopSong,
                    RepeatMode::LoopSong => RepeatMode::Normal,
                };
                println!(
                    "Telemetry: Repeat mode is now {:?}",
                    self.player_state.repeat_mode
                );
                self.mpris_send(MprisStateUpdate::LoopStatus(mpris::to_mpris_loop_status(
                    self.player_state.repeat_mode,
                )));
            }
            Message::Previous => {
                if self.queue_position > 0 && !self.queue_tracks.is_empty() {
                    self.queue_position -= 1;
                    if let Some(track) = self.queue_tracks.get(self.queue_position) {
                        self.player_state.current_track = Some(track.clone());
                        let path = track.uri.clone();
                        self.update_queue_metadata();
                        self.mpris_send_track_metadata();
                        return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                    }
                }
                self.update_queue_metadata();
                return Task::none();
            }
            Message::Next => {
                if self.player_state.repeat_mode == RepeatMode::LoopSong {
                    // Loop current song - do nothing to position, just reload
                } else if self.player_state.shuffle && !self.queue_tracks.is_empty() {
                    // Random next track
                    use rand::Rng;
                    let mut rng = rand::rng();
                    if self.queue_tracks.len() > 1 {
                        let current_pos = self.queue_position;
                        let mut next_pos = rng.random_range(0..self.queue_tracks.len());
                        while next_pos == current_pos {
                            next_pos = rng.random_range(0..self.queue_tracks.len());
                        }
                        self.queue_position = next_pos;
                    } else {
                        self.queue_position = 0;
                    }
                } else if self.queue_position + 1 < self.queue_tracks.len() {
                    self.queue_position += 1;
                } else if self.player_state.repeat_mode == RepeatMode::Loop {
                    self.queue_position = 0;
                } else {
                    // End of queue
                    self.update_queue_metadata();
                    return Task::none();
                }

                if let Some(track) = self.queue_tracks.get(self.queue_position).cloned() {
                    self.player_state.current_track = Some(track.clone());
                    let path = track.uri.clone();
                    self.update_queue_metadata();
                    self.mpris_send_track_metadata();
                    return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                }
                self.update_queue_metadata();
                return Task::none();
            }
            Message::PlaybackTick(position) => {
                self.player_state.position = position;
                self.mpris_send(MprisStateUpdate::Position(mpris::secs_to_micros(position)));
            }
            Message::PlaybackDuration(secs) => {
                println!("Telemetry: Message::PlaybackDuration received: {secs}s");
                #[allow(clippy::cast_precision_loss)]
                let duration = secs as f64; // Safe: duration value fits in f64
                if let Some(ref mut track) = self.player_state.current_track {
                    let mut new_track = (**track).clone();
                    new_track.duration = duration;
                    self.player_state.current_track = Some(Arc::new(new_track));
                } else {
                    let mut track = Track::default();
                    track.duration = duration;
                    self.player_state.current_track = Some(Arc::new(track));
                }
            }
            Message::Seek(secs) => {
                println!("Telemetry: Message::Seek requested for: {secs}s");
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.seek(secs);
                    self.player_state.position = secs;
                    self.mpris_send(MprisStateUpdate::Position(mpris::secs_to_micros(secs)));
                }
            }
            Message::VolumeChanged(volume) => {
                println!("Telemetry: Volume changed to {:.2}%", volume * 100.0);
                let vol_f64 = f64::from(volume);
                self.player_state.volume = vol_f64;
                self.config.default_volume = vol_f64;

                // Persist volume change
                if let Ok(config_context) = cosmic_config::Config::new(APP_ID, Config::VERSION)
                    && let Err(e) = self.config.write_entry(&config_context)
                {
                    eprintln!("Error persisting volume: {e:?}");
                }

                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.set_volume(vol_f64);
                }
                self.mpris_send(MprisStateUpdate::Volume(vol_f64));
            }
            Message::PlaybackStateChanged(state) => {
                self.player_state.playback_state = state;
                self.mpris_send(MprisStateUpdate::PlaybackStatus(mpris::to_mpris_status(
                    state,
                )));
            }
            Message::PlaybackEnded => {
                self.player_state.playback_state = PlaybackState::Stopped;
                self.mpris_send(MprisStateUpdate::PlaybackStatus(
                    mpris_server::PlaybackStatus::Stopped,
                ));
                return Task::done(cosmic::Action::App(Message::Next));
            }
            Message::TimerTick => {
                if let Some(ref backend) = self.audio_backend
                    && let Ok(mut backend) = backend.lock()
                {
                    backend.tick();
                }
                return Task::none();
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle queue management messages
    fn update_queue(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::ToggleQueue => {
                self.queue_visible = !self.queue_visible;
            }
            Message::ClearQueue => {
                self.queue_tracks.clear();
                self.queue_position = 0;
                self.update_queue_metadata();
            }
            Message::ToggleQueueEditMode => {
                self.queue_edit_mode = !self.queue_edit_mode;
            }
            Message::GoToQueuePosition(position) => {
                if position < self.queue_tracks.len() {
                    self.queue_position = position;
                    if let Some(track) = self.queue_tracks.get(position) {
                        self.player_state.current_track = Some(track.clone());
                        let path = track.uri.clone();
                        self.update_queue_metadata();
                        self.mpris_send_track_metadata();
                        return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                    }
                }
                self.update_queue_metadata();
            }
            Message::AddToQueue(track_idx) => {
                if let Some(track) = self.library_tracks.get(track_idx) {
                    self.queue_tracks.push(std::sync::Arc::new(track.clone()));
                }
                self.update_queue_metadata();
            }
            Message::MoveQueueTrack { from, to } => {
                if from < self.queue_tracks.len() && to < self.queue_tracks.len() && from != to {
                    let track = self.queue_tracks.remove(from);
                    self.queue_tracks.insert(to, track);
                    if from == self.queue_position {
                        self.queue_position = to;
                    } else if from < self.queue_position && to >= self.queue_position {
                        self.queue_position = self.queue_position.saturating_sub(1);
                    } else if from > self.queue_position && to <= self.queue_position {
                        self.queue_position = self.queue_position.saturating_add(1);
                    }
                }
                self.update_queue_metadata();
            }
            Message::RemoveFromQueue(position) => {
                if position < self.queue_tracks.len() {
                    self.queue_tracks.remove(position);
                    if self.queue_tracks.is_empty() {
                        self.queue_position = 0;
                    } else if position < self.queue_position {
                        self.queue_position = self.queue_position.saturating_sub(1);
                    } else if self.queue_position >= self.queue_tracks.len() {
                        self.queue_position = self.queue_tracks.len().saturating_sub(1);
                    }
                }
                self.update_queue_metadata();
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle library loading and processing messages
    #[allow(clippy::too_many_lines)]
    fn update_library(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::LoadMusic => {
                let path = std::path::PathBuf::from("test.mp3");
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::utils::metadata::extract_metadata(path)
                        })
                        .await
                        .unwrap_or(None)
                    },
                    Message::MusicLoaded,
                )
                .map(cosmic::Action::App);
            }
            Message::MusicLoaded(metadata) => {
                println!("Telemetry: Message::MusicLoaded received");
                if let Some(data) = metadata {
                    self.theme_color = data.accent_color;
                    self.theme_color_secondary = data.secondary_color;
                    self.theme_color_tertiary = data.tertiary_color;
                    if let Some(color) = data.accent_color {
                        self.use_dark_text = should_use_dark_text(color);
                    }

                    if let Some(ref mut track) = self.player_state.current_track {
                        let mut new_track = (**track).clone();
                        if let Some(ref title) = data.title {
                            new_track.title.clone_from(title);
                        }
                        if let Some(ref artist) = data.artist {
                            new_track.artist.clone_from(artist);
                        }
                        if let Some(ref album) = data.album {
                            new_track.album.clone_from(album);
                        }
                        self.player_state.current_track = Some(Arc::new(new_track));
                        println!("Telemetry: Updated track metadata from MusicLoaded");
                        // Sync updated metadata to MPRIS
                        self.mpris_send_track_metadata();
                    }

                    if let (Some(db), Some(path)) =
                        (self.database.as_ref(), self.current_track_path.as_ref())
                    {
                        match db.save_track_metadata(path, &data) {
                            Ok(id) => {
                                println!("Telemetry: Track saved to database with ID: {id}");
                            }
                            Err(e) => {
                                eprintln!("Telemetry Error: Failed to save track to database: {e}")
                            }
                        }
                    }

                    if let Some(img) = data.cover.as_ref() {
                        let rgba = img.to_rgba8();
                        self.background_image_dims = Some((rgba.width(), rgba.height()));
                        let handle = widget::image::Handle::from_rgba(
                            rgba.width(),
                            rgba.height(),
                            rgba.into_raw(),
                        );
                        self.background_image = Some(handle);
                    } else {
                        self.background_image = self.default_background_image.clone();
                        self.background_image_dims = self.default_background_image_dims;
                        self.theme_color = Some(self.default_theme_colors.0);
                        self.theme_color_secondary = Some(self.default_theme_colors.1);
                        self.theme_color_tertiary = Some(self.default_theme_colors.2);
                        self.use_dark_text = should_use_dark_text(self.default_theme_colors.0);
                    }

                    if let Some(color) = self.theme_color {
                        let mut theme = cosmic::cosmic_theme::Theme::dark_default();
                        let rgb = cosmic::cosmic_theme::palette::rgb::Rgb::from(color);
                        let alpha_color = cosmic::cosmic_theme::palette::Alpha {
                            color: rgb,
                            alpha: 0.2,
                        };

                        theme.is_frosted = true;
                        theme.primary.base = alpha_color;
                        theme.background.base = alpha_color;
                        theme.accent.base = alpha_color;

                        let app_theme = cosmic::Theme::custom(std::sync::Arc::new(theme));

                        return Task::done(cosmic::Action::Cosmic(
                            cosmic::app::Action::AppThemeChange(app_theme),
                        ));
                    }
                }
            }
            Message::LibraryLoaded(tracks) => {
                println!("Telemetry: Library loaded with {} tracks", tracks.len());
                self.library_tracks.clone_from(&tracks);
                self.library_albums = self.refresh_albums();
                self.library_artists = self.refresh_artists();

                let tracks_to_process: Vec<_> = tracks
                    .into_iter()
                    .filter(|t| t.accent_color.is_none())
                    .collect();

                if !tracks_to_process.is_empty() {
                    let db = self.database.clone();
                    return Task::perform(
                        async move {
                            if let Some(db) = db {
                                tokio::task::spawn_blocking(move || {
                                    for track in tracks_to_process {
                                        if let Some(meta) =
                                            crate::utils::metadata::extract_metadata(&track.uri)
                                        {
                                            let _ = db.update_track_visuals(
                                                track.id,
                                                meta.accent_color,
                                                meta.secondary_color,
                                                meta.tertiary_color,
                                                track.cover_art_id,
                                            );
                                        }
                                    }
                                    db.get_all_tracks().unwrap_or_default()
                                })
                                .await
                                .unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        },
                        Message::VisualsProcessed,
                    )
                    .map(cosmic::Action::App);
                }

                return Task::done(Message::LoadCovers).map(cosmic::Action::App);
            }
            Message::VisualsProcessed(tracks) => {
                println!(
                    "Telemetry: Visuals processed, updating library ({} tracks)",
                    tracks.len()
                );
                self.library_tracks.clone_from(&tracks);
                self.library_albums = self.refresh_albums();
                self.library_artists = self.refresh_artists();

                if let Some(current) = &self.player_state.current_track
                    && let Some(updated_track) = tracks.iter().find(|t| t.id == current.id)
                    && updated_track.accent_color.is_some()
                {
                    self.theme_color = updated_track.accent_color;
                    self.theme_color_secondary = updated_track.secondary_color;
                    self.theme_color_tertiary = updated_track.tertiary_color;
                }
                println!(
                    "DEBUG: VisualsProcessed. Triggering LoadCovers. Albums count: {}",
                    self.library_albums.len()
                );
                return Task::done(Message::LoadCovers).map(cosmic::Action::App);
            }
            Message::LoadCovers => {
                let mut missing_covers = std::collections::HashSet::new();

                for album in &self.library_albums {
                    if let Some(id) = album.cover_art_id
                        && !self.covers_cache.contains_key(&id)
                    {
                        missing_covers.insert(id);
                    }
                }

                for track in &self.library_tracks {
                    if let Some(id) = track.cover_art_id
                        && !self.covers_cache.contains_key(&id)
                    {
                        missing_covers.insert(id);
                    }
                }

                let albums_missing_covers: Vec<i64> = missing_covers.into_iter().collect();

                println!(
                    "DEBUG: LoadCovers. Total unique missing covers to load: {}",
                    albums_missing_covers.len()
                );

                if !albums_missing_covers.is_empty() {
                    let mut tasks = Vec::new();
                    for chunk in albums_missing_covers.chunks(20) {
                        let chunk_ids = chunk.to_vec();
                        let db = self.database.clone();

                        let task = Task::perform(
                            async move {
                                if let Some(db) = db {
                                    tokio::task::spawn_blocking(move || {
                                        let mut loaded = Vec::new();
                                        for id in chunk_ids {
                                            if let Ok(data) = db.get_cover_art(id)
                                                && let Ok(img) = image::load_from_memory(&data)
                                            {
                                                let rgba = img.to_rgba8();
                                                let handle = widget::image::Handle::from_rgba(
                                                    rgba.width(),
                                                    rgba.height(),
                                                    rgba.into_raw(),
                                                );
                                                loaded.push((id, handle));
                                            }
                                        }
                                        loaded
                                    })
                                    .await
                                    .unwrap_or_default()
                                } else {
                                    Vec::new()
                                }
                            },
                            Message::CoversLoaded,
                        )
                        .map(cosmic::Action::App);

                        tasks.push(task);
                    }

                    println!(
                        "DEBUG: LoadCovers. Returning batch of {} tasks.",
                        tasks.len()
                    );
                    return Task::batch(tasks);
                }
                println!("DEBUG: LoadCovers. No missing covers to load.");
                return Task::none();
            }
            Message::CoversLoaded(covers) => {
                println!("DEBUG: CoversLoaded. Received {} covers.", covers.len());
                for (id, handle) in covers {
                    self.covers_cache.insert(id, handle);
                }
                return Task::none();
            }
            Message::RefreshLibrary => {
                let folders = self.config.music_folders.clone();
                let db_clone = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db_clone {
                            for folder in folders {
                                let path = std::path::PathBuf::from(folder);
                                let files = crate::utils::metadata::scan_directory(&path);
                                let mut batch = Vec::new();
                                for file_path in files {
                                    if let Some(metadata) =
                                        crate::utils::metadata::extract_metadata_fast(&file_path)
                                    {
                                        batch.push((file_path, metadata));
                                    }
                                    if batch.len() >= 50 {
                                        let _ = db.save_tracks_batch(&batch);
                                        batch.clear();
                                    }
                                }
                                if !batch.is_empty() {
                                    let _ = db.save_tracks_batch(&batch);
                                }
                            }
                            db.get_all_tracks().unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    },
                    Message::LibraryLoaded,
                )
                .map(cosmic::Action::App);
            }
            Message::RescanLibrary => {
                if let Some(db) = &self.database {
                    let _ = db.clear_library();
                }
                self.library_tracks.clear();
                self.library_albums.clear();
                self.covers_cache.clear();
                return Task::done(cosmic::Action::App(Message::RefreshLibrary));
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle navigation messages
    fn update_navigation(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::NavTo(page) => {
                self.current_page = page;
                self.add_to_playlist_menu = None;
                if page == NavPage::Albums {
                    self.albums_scroll_y = 0.0;
                }
                // The back button of a playlist page navigates here: leave the
                // detail view so the grid is shown again.
                if page == NavPage::Playlists
                    && matches!(
                        self.current_view,
                        AppView::PlaylistDetail(_) | AppView::Favorites
                    )
                {
                    self.current_view = AppView::Grid;
                }
            }
            Message::AlbumsScrolled(viewport) => {
                self.albums_scroll_y = viewport.absolute_offset().y;
                self.albums_viewport_h = viewport.bounds().height;
                self.albums_viewport_w = viewport.bounds().width;
            }
            Message::TracksScroll(viewport) => {
                self.tracks_scroll_offset = viewport.absolute_offset();
            }
            Message::PlaylistsScrolled(viewport) => {
                self.playlists_scroll_y = viewport.absolute_offset().y;
                // The grid viewport is shared with the albums page
                self.albums_viewport_h = viewport.bounds().height;
                self.albums_viewport_w = viewport.bounds().width;
            }
            Message::OpenAlbumDetail(album_idx) => {
                println!("Telemetry: Opening album detail for index {album_idx}");
                self.current_view = AppView::AlbumDetail(album_idx, None);
            }
            Message::OpenAlbumDetailFromArtist(album_idx, artist_idx) => {
                println!("Telemetry: Opening album detail {album_idx} from artist {artist_idx}");
                self.current_view = AppView::AlbumDetail(album_idx, Some(artist_idx));
            }
            Message::BackToAlbums(source_artist) => {
                if let Some(artist_idx) = source_artist {
                    println!("Telemetry: Back to artist detail {artist_idx}");
                    self.current_view = AppView::ArtistDetail(artist_idx);
                } else {
                    println!("Telemetry: Back to albums grid");
                    self.current_view = AppView::Grid;
                }
            }
            Message::OpenArtistDetail(artist_idx) => {
                println!("Telemetry: Opening artist detail for index {artist_idx}");
                self.current_view = AppView::ArtistDetail(artist_idx);
            }
            Message::BackToArtists => {
                println!("Telemetry: Back to artists grid");
                self.current_view = AppView::Grid;
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle play actions (tracks, albums, artists, playlists)
    fn update_play_actions(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::PlayAlbumFromTrack(album_idx, track_idx) => {
                println!("Telemetry: Play album {album_idx} from track {track_idx}");
                if let Some(album) = self.library_albums.get(album_idx) {
                    self.queue_tracks.clear();
                    for t in &album.tracks {
                        self.queue_tracks.push(std::sync::Arc::new(t.clone()));
                    }
                    self.queue_position = track_idx;

                    if let Some(track) = album.tracks.get(track_idx) {
                        self.player_state.current_track = Some(std::sync::Arc::new(track.clone()));
                        let path = track.uri.clone();
                        self.update_queue_metadata();
                        self.mpris_send_track_metadata();
                        return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                    }
                }
                self.update_queue_metadata();
            }
            Message::PlayTrack(track_idx) => {
                println!("Telemetry: Play track from library index {track_idx}");
                if let Some(track) = self.library_tracks.get(track_idx) {
                    let path = track.uri.clone();
                    self.queue_tracks.clear();
                    self.queue_tracks.push(std::sync::Arc::new(track.clone()));
                    self.queue_position = 0;
                    self.player_state.current_track = Some(std::sync::Arc::new(track.clone()));
                    self.update_queue_metadata();
                    self.mpris_send_track_metadata();
                    return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                }
                self.update_queue_metadata();
            }
            Message::PlayArtistTracks(artist_idx) => {
                println!("Telemetry: Play all tracks for artist index {artist_idx}");
                if let Some(artist) = self.library_artists.get(artist_idx) {
                    self.queue_tracks.clear();
                    for t in &artist.tracks {
                        self.queue_tracks.push(std::sync::Arc::new(t.clone()));
                    }

                    if !self.queue_tracks.is_empty() {
                        self.queue_position = 0;
                        if let Some(track) = self.queue_tracks.first() {
                            self.player_state.current_track = Some(track.clone());
                            let path = track.uri.clone();
                            self.update_queue_metadata();
                            self.mpris_send_track_metadata();
                            return Task::done(cosmic::Action::App(Message::FileSelected(Some(
                                path,
                            ))));
                        }
                    }
                }
            }
            Message::PlayPlaylist(id) => {
                if let Some(_p) = self.library_playlists.iter().find(|p| p.id == id) {
                    self.queue_tracks.clear();
                    for t in &self.playlist_tracks {
                        self.queue_tracks.push(std::sync::Arc::new(t.clone()));
                    }
                    self.queue_position = 0;

                    if let Some(track) = self.playlist_tracks.first() {
                        self.player_state.current_track = Some(std::sync::Arc::new(track.clone()));
                        let path = track.uri.clone();
                        self.update_queue_metadata();
                        self.mpris_send_track_metadata();
                        return Task::done(cosmic::Action::App(Message::FileSelected(Some(path))));
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle settings dialog and window management messages
    #[allow(clippy::too_many_lines)]
    fn update_settings_window(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::WindowMinimize => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::iced::window::minimize(id, true)
                        .map(|(): ()| Message::NoOp)
                        .map(cosmic::Action::App);
                }
            }
            Message::WindowMaximize => {
                if let Some(id) = self.core.main_window_id() {
                    let is_maximized = self.core.window.is_maximized;
                    return cosmic::iced::window::maximize(id, !is_maximized)
                        .map(|(): ()| Message::NoOp)
                        .map(cosmic::Action::App);
                }
            }
            Message::WindowClose => {
                if let Some(_id) = self.core.main_window_id() {
                    std::process::exit(0);
                }
            }
            Message::WindowDrag => {
                if let Some(id) = self.core.main_window_id() {
                    return cosmic::iced::window::drag(id)
                        .map(|(): ()| Message::NoOp)
                        .map(cosmic::Action::App);
                }
            }
            Message::WindowCreated(id) => {
                if self.core.main_window_id() != Some(id) {
                    self.settings_window_id = Some(id);
                }
            }
            Message::WindowClosed(id) => {
                if self.settings_window_id == Some(id) {
                    self.settings_window_id = None;
                }
            }
            Message::SettingsWindowClosed => {
                self.settings_window_id = None;
            }
            Message::OpenSettings => {
                if self.settings_window_id.is_some() {
                    return Task::none();
                }
                let (id, task) = window::open(window::Settings {
                    size: cosmic::iced::Size::new(500.0, 700.0),
                    min_size: Some(cosmic::iced::Size::new(400.0, 500.0)),
                    ..Default::default()
                });
                self.settings_window_id = Some(id);
                return task.map(|id| cosmic::Action::App(Message::WindowCreated(id)));
            }
            Message::SettingsChangePage(page) => {
                self.settings_page = page;
            }
            Message::Settings(msg) => {
                if matches!(msg, SettingsMessage::Close) {
                    if let Some(id) = self.settings_window_id {
                        return cosmic::iced::window::close(id);
                    }
                    return Task::none();
                }

                if matches!(msg, SettingsMessage::RefreshLibrary) {
                    return Task::done(cosmic::Action::App(Message::RefreshLibrary));
                }

                if matches!(msg, SettingsMessage::RescanLibrary) {
                    return Task::done(cosmic::Action::App(Message::RescanLibrary));
                }

                if matches!(msg, SettingsMessage::AddMusicFolder) {
                    return Task::perform(
                        async {
                            use cosmic::dialog::file_chooser;

                            let result: Result<
                                file_chooser::open::FileResponse,
                                file_chooser::Error,
                            > = file_chooser::open::Dialog::new()
                                .title("Adicionar Pasta de Músicas")
                                .open_folder()
                                .await;

                            match result {
                                Ok(response) => response
                                    .url()
                                    .to_file_path()
                                    .ok()
                                    .map(|p| p.to_string_lossy().to_string()),
                                Err(_) => None,
                            }
                        },
                        |path_opt| Message::Settings(SettingsMessage::FolderSelected(path_opt)),
                    )
                    .map(cosmic::Action::App);
                }

                if let SettingsMessage::FolderSelected(Some(ref folder)) = msg {
                    self.config.add_music_folder(folder.clone());
                    println!("Telemetry: Added music folder: {folder}");

                    if let Ok(config_context) = cosmic_config::Config::new(APP_ID, Config::VERSION)
                    {
                        if let Err(e) = self.config.write_entry(&config_context) {
                            eprintln!("Error persisting config: {e:?}");
                        } else {
                            println!("Telemetry: Config persisted successfully");
                        }
                    }

                    let folder_path = std::path::PathBuf::from(folder.clone());
                    return Task::done(cosmic::Action::App(Message::FolderSelected(Some(
                        folder_path,
                    ))));
                }

                if let SettingsMessage::RemoveMusicFolder(index) = msg {
                    if index < self.config.music_folders.len() {
                        let removed = self.config.music_folders.remove(index);
                        println!("Telemetry: Removed music folder: {removed}");

                        if let Ok(config_context) =
                            cosmic_config::Config::new(APP_ID, Config::VERSION)
                        {
                            if let Err(e) = self.config.write_entry(&config_context) {
                                eprintln!("Error persisting config: {e:?}");
                            } else {
                                println!("Telemetry: Config persisted after folder removal");
                            }
                        }
                    }
                    return Task::none();
                }

                if let SettingsMessage::ChangePage(page) = msg {
                    self.settings_page = page;
                    return Task::none();
                }

                if settings_dialog::apply_message(&mut self.config, &msg) {
                    println!("Telemetry: Config updated via settings dialog");
                    if let Ok(config_context) = cosmic_config::Config::new(APP_ID, Config::VERSION)
                        && let Err(e) = self.config.write_entry(&config_context)
                    {
                        eprintln!("Error persisting config: {e:?}");
                    }
                }
                return Task::none();
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle playlist CRUD messages
    #[allow(clippy::too_many_lines)]
    fn update_playlists(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::LoadPlaylists => {
                let db = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            tokio::task::spawn_blocking(move || {
                                db.get_all_playlists().unwrap_or_default()
                            })
                            .await
                            .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    },
                    Message::PlaylistsLoaded,
                )
                .map(cosmic::Action::App);
            }
            Message::PlaylistsLoaded(playlists) => {
                self.library_playlists = playlists;
            }
            Message::CreatePlaylist(title) => {
                let db = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ = tokio::task::spawn_blocking(move || {
                                db.create_playlist(title, None)
                            })
                            .await;
                        }
                    },
                    |()| Message::LoadPlaylists,
                )
                .map(cosmic::Action::App);
            }
            Message::DeletePlaylist(id) => {
                let db = self.database.clone();
                match self.current_view {
                    AppView::PlaylistDetail(playlist_id) if playlist_id == id => {
                        self.current_view = AppView::Grid;
                    }
                    _ => {}
                }
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ =
                                tokio::task::spawn_blocking(move || db.delete_playlist(id)).await;
                        }
                    },
                    |()| Message::LoadPlaylists,
                )
                .map(cosmic::Action::App);
            }
            Message::RenamePlaylist(id, title) => {
                if let Some(p) = self.library_playlists.iter_mut().find(|p| p.id == id) {
                    p.title.clone_from(&title);
                }
                let db = self.database.clone();
                let p_desc = self
                    .library_playlists
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.description.clone());
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ = tokio::task::spawn_blocking(move || {
                                db.update_playlist_metadata(id, title, p_desc)
                            })
                            .await;
                        }
                    },
                    |()| Message::NoOp,
                )
                .map(cosmic::Action::App);
            }
            Message::UpdatePlaylistDescription(id, desc) => {
                if let Some(p) = self.library_playlists.iter_mut().find(|p| p.id == id) {
                    p.description = Some(desc.clone());
                }
                let db = self.database.clone();
                let p_title = self
                    .library_playlists
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.title.clone())
                    .unwrap_or_default();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ = tokio::task::spawn_blocking(move || {
                                db.update_playlist_metadata(id, p_title, Some(desc))
                            })
                            .await;
                        }
                    },
                    |()| Message::NoOp,
                )
                .map(cosmic::Action::App);
            }
            Message::TogglePlaylistEditMode(enabled) => {
                self.playlist_edit_mode = enabled;
            }
            Message::OpenPlaylistDetail(id) => {
                self.current_view = AppView::PlaylistDetail(id);
                self.playlist_edit_mode = false;
                self.add_to_playlist_menu = None;
                return self.update_playlists(Message::ReloadPlaylistTracks(id));
            }
            Message::ReloadPlaylistTracks(id) => {
                let db = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            tokio::task::spawn_blocking(move || {
                                db.get_playlist_tracks(id).unwrap_or_default()
                            })
                            .await
                            .unwrap_or_default()
                        } else {
                            Vec::new()
                        }
                    },
                    Message::PlaylistTracksLoaded,
                )
                .map(cosmic::Action::App);
            }
            Message::PlaylistTracksLoaded(tracks) => {
                self.playlist_tracks = tracks;
            }
            Message::PlaylistTracksChanged(playlist_id) => {
                let mut tasks = vec![self.update_playlists(Message::LoadPlaylists)];
                if self.current_view == AppView::PlaylistDetail(playlist_id) {
                    tasks.push(self.update_playlists(Message::ReloadPlaylistTracks(playlist_id)));
                }
                return Task::batch(tasks);
            }
            Message::OpenAddToPlaylistMenu(track_id) => {
                self.add_to_playlist_menu = if self.add_to_playlist_menu == Some(track_id) {
                    None
                } else {
                    Some(track_id)
                };
            }
            Message::CloseAddToPlaylistMenu(track_id) => {
                if self.add_to_playlist_menu == Some(track_id) {
                    self.add_to_playlist_menu = None;
                }
            }
            Message::CreatePlaylistWithTrack(track_id) => {
                self.add_to_playlist_menu = None;
                let db = self.database.clone();
                let title = fl!("new-playlist");
                return Task::perform(
                    async move {
                        let Some(db) = db else { return None };
                        tokio::task::spawn_blocking(move || {
                            let playlist_id = db.create_playlist(title, None).ok()?;
                            let _ = db.add_track_to_playlist(playlist_id, track_id);
                            Some(playlist_id)
                        })
                        .await
                        .ok()
                        .flatten()
                    },
                    |playlist_id| {
                        playlist_id.map_or(Message::LoadPlaylists, Message::PlaylistTracksChanged)
                    },
                )
                .map(cosmic::Action::App);
            }
            Message::AddTrackToPlaylist(playlist_id, track_id) => {
                self.add_to_playlist_menu = None;
                let db = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ = tokio::task::spawn_blocking(move || {
                                db.add_track_to_playlist(playlist_id, track_id)
                            })
                            .await;
                        }
                    },
                    move |()| Message::PlaylistTracksChanged(playlist_id),
                )
                .map(cosmic::Action::App);
            }
            Message::RemoveTrackFromPlaylist(playlist_id, position) => {
                let db = self.database.clone();
                return Task::perform(
                    async move {
                        if let Some(db) = db {
                            let _ = tokio::task::spawn_blocking(move || {
                                db.remove_track_from_playlist(playlist_id, position)
                            })
                            .await;
                        }
                    },
                    move |()| Message::PlaylistTracksChanged(playlist_id),
                )
                .map(cosmic::Action::App);
            }
            _ => {}
        }
        Task::none()
    }

    /// Handle file/folder picker and selection messages
    #[allow(clippy::too_many_lines)]
    fn update_file_ops(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::OpenFilePicker => {
                return Task::perform(
                    async {
                        use cosmic::dialog::file_chooser::{self, FileFilter};

                        let filter = FileFilter::new("Audio Files")
                            .glob("*.mp3")
                            .glob("*.flac")
                            .glob("*.ogg")
                            .glob("*.wav")
                            .glob("*.m4a")
                            .glob("*.aac");

                        let result = file_chooser::open::Dialog::new()
                            .title("Selecionar Música") // TODO: Use fl! macro
                            .filter(filter)
                            .open_file()
                            .await;

                        match result {
                            Ok(response) => response.url().to_file_path().ok(),
                            Err(_) => None,
                        }
                    },
                    Message::FileSelected,
                )
                .map(cosmic::Action::App);
            }
            Message::OpenFolderPicker => {
                return Task::perform(
                    async {
                        use cosmic::dialog::file_chooser;

                        let result: Result<file_chooser::open::FileResponse, file_chooser::Error> =
                            file_chooser::open::Dialog::new()
                                .title("Selecionar Pasta de Músicas") // TODO: Use fl! macro
                                .open_folder()
                                .await;

                        match result {
                            Ok(response) => response.url().to_file_path().ok(),
                            Err(_) => None,
                        }
                    },
                    Message::FolderSelected,
                )
                .map(cosmic::Action::App);
            }
            Message::FolderSelected(path_opt) => {
                println!("Telemetry: Message::FolderSelected received: {path_opt:?}");
                if let Some(path) = path_opt {
                    let path_str = path.to_string_lossy().to_string();
                    if !self.config.music_folders.contains(&path_str) {
                        self.config.add_music_folder(path_str.clone());
                        println!(
                            "Telemetry: Automatically added and persisting folder: {path_str}"
                        );

                        if let Ok(config_context) =
                            cosmic_config::Config::new(APP_ID, Config::VERSION)
                        {
                            let _ = self.config.write_entry(&config_context);
                        }
                    }

                    let db_clone = self.database.clone();
                    return Task::perform(
                        async move {
                            let files = crate::utils::metadata::scan_directory(&path);
                            println!("Telemetry: Found {} audio files in {:?}", files.len(), path);

                            if let Some(db) = db_clone {
                                let mut batch = Vec::new();
                                for (i, file_path) in files.iter().enumerate() {
                                    if let Some(metadata) =
                                        crate::utils::metadata::extract_metadata_fast(file_path)
                                    {
                                        batch.push((file_path.clone(), metadata));
                                    }

                                    if batch.len() >= 50 {
                                        let _ = db.save_tracks_batch(&batch);
                                        println!(
                                            "Telemetry: Processed {}/{} files (Batch Save)",
                                            i + 1,
                                            files.len()
                                        );
                                        batch.clear();
                                    }
                                }

                                if !batch.is_empty() {
                                    let _ = db.save_tracks_batch(&batch);
                                    println!("Telemetry: Processed all {} files", files.len());
                                }

                                db.get_all_tracks().unwrap_or_default()
                            } else {
                                Vec::new()
                            }
                        },
                        Message::LibraryLoaded,
                    )
                    .map(cosmic::Action::App);
                }
            }
            Message::FileSelected(path_opt) => {
                println!("Telemetry: Message::FileSelected received: {path_opt:?}");
                if let Some(path) = path_opt {
                    let path_str = path.to_string_lossy().to_string();
                    self.current_track_path = Some(path.clone());

                    // Reset position
                    self.player_state.position = 0;

                    // Only clear metadata/cover if we are loading a DIFFERENT track path
                    let should_clear_track =
                        if let Some(ref current) = self.player_state.current_track {
                            current.uri != *path
                        } else {
                            true
                        };

                    if should_clear_track {
                        self.player_state.current_track = None;
                    }

                    // Create audio backend if not exists
                    if self.audio_backend.is_none() {
                        let (tx, rx) = mpsc::unbounded_channel();
                        let mut backend = AudioBackend::new(tx);
                        backend.set_volume(self.player_state.volume);
                        self.audio_backend = Some(Arc::new(Mutex::new(backend)));
                        self.event_rx = Some(Arc::new(tokio::sync::Mutex::new(rx)));
                    }

                    // Load and play the file
                    if let Some(ref backend) = self.audio_backend
                        && let Ok(mut backend) = backend.lock()
                    {
                        backend.load_file(&path_str);
                        self.player_state.playback_state = PlaybackState::Playing;
                    }

                    // Also extract metadata for theme
                    let path_clone = path.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::utils::metadata::extract_metadata(path_clone)
                            })
                            .await
                            .unwrap_or(None)
                        },
                        Message::MusicLoaded,
                    )
                    .map(cosmic::Action::App);
                }
            }
            _ => {}
        }
        Task::none()
    }
}
