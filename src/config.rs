// SPDX-License-Identifier: GPL-3.0
#![allow(clippy::float_cmp)]

//! Configuration module for Brigadeiro
//!
//! This module handles all user preferences that persist between sessions.
//! Uses `cosmic_config` for storage in `~/.config/cosmic/com.github.brittos.brigadeiro/v1/`
//!
//! ## Architecture
//!
//! - **Config**: User preferences (UI, playback settings)
//! - **Database (`SQLite`)**: Music data (albums, artists, tracks)
//!
//! These are intentionally separate - config is for preferences, DB is for content.

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

/// Sort order options for album grid
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlbumSort {
    #[default]
    Title,
    Artist,
    ReleaseDate,
    Duration,
    TrackCount,
}

/// Sort order options for artist grid
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtistSort {
    #[default]
    Name,
    AlbumCount,
    TrackCount,
}

/// Sort order options for track list
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackSort {
    #[default]
    Title,
    Album,
    Artist,
    ReleaseDate,
    Duration,
}

/// Main configuration struct for Brigadeiro
///
/// All fields are automatically persisted to disk via `cosmic_config`.
/// Changes trigger UI updates through the subscription system.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, CosmicConfigEntry, PartialEq, Serialize, Deserialize)]
#[version = 1]
pub struct Config {
    // ═══════════════════════════════════════════════════════════════════════
    // Library Settings (Fase 2)
    // ═══════════════════════════════════════════════════════════════════════
    /// Music library folders to scan
    pub music_folders: Vec<String>,

    // ═══════════════════════════════════════════════════════════════════════
    // UI Settings - Album Grid (Fase 3)
    // ═══════════════════════════════════════════════════════════════════════
    /// Default sort order for album grid
    pub album_sort: AlbumSort,

    /// Show album title and artist labels on cards
    pub album_show_labels: bool,

    // ═══════════════════════════════════════════════════════════════════════
    // UI Settings - Artist Grid (Fase 3)
    // ═══════════════════════════════════════════════════════════════════════
    /// Default sort order for artist grid
    pub artist_sort: ArtistSort,

    // ═══════════════════════════════════════════════════════════════════════
    // UI Settings - Track List (Fase 3)
    // ═══════════════════════════════════════════════════════════════════════
    /// Default sort order for track list
    pub track_sort: TrackSort,

    /// Show search bar by default on track page
    pub track_show_search: bool,

    // ═══════════════════════════════════════════════════════════════════════
    // UI Settings - Queue/Sidebar
    // ═══════════════════════════════════════════════════════════════════════
    /// Open queue sidebar by default on app start
    pub queue_open_default: bool,

    // ═══════════════════════════════════════════════════════════════════════
    // Playback Settings (Fase 4)
    // ═══════════════════════════════════════════════════════════════════════
    /// Default playback volume (0.0 - 1.0)
    pub default_volume: f64,

    /// Shuffle mode loops at end instead of stopping
    pub shuffle_loops: bool,

    /// Playing a track from album view plays entire album
    pub album_from_track: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Library
            music_folders: vec![],

            // Album Grid
            album_sort: AlbumSort::default(),
            album_show_labels: true,

            // Artist Grid
            artist_sort: ArtistSort::default(),

            // Track List
            track_sort: TrackSort::default(),
            track_show_search: true,

            // Queue
            queue_open_default: false,

            // Playback
            default_volume: 0.5,
            shuffle_loops: true,
            album_from_track: true,
        }
    }
}

impl Config {
    /// Add a music folder to the configuration
    pub fn add_music_folder(&mut self, folder: String) {
        if !self.music_folders.contains(&folder) {
            self.music_folders.push(folder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.music_folders.is_empty());
        assert_eq!(config.album_sort, AlbumSort::Title);
        assert!(config.album_show_labels);
        assert_eq!(config.default_volume, 0.5);
    }

    #[test]
    fn test_add_music_folder() {
        let mut config = Config::default();
        config.add_music_folder("/home/user/Music".to_string());
        assert_eq!(config.music_folders.len(), 1);

        // Adding duplicate should not add
        config.add_music_folder("/home/user/Music".to_string());
        assert_eq!(config.music_folders.len(), 1);
    }
}
