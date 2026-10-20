// SPDX-License-Identifier: GPL-3.0
//
// Track model - represents a single audio track in the library

pub use cosmic::iced::Color;
use std::path::PathBuf;

/// Represents a single audio track
#[derive(Debug, Clone, Default)]
pub struct Track {
    /// Unique identifier
    pub id: i64,
    /// Track title
    pub title: String,
    /// Artist name
    pub artist: String,
    /// Album artist (may differ from track artist)
    pub album_artist: String,
    /// Album name
    pub album: String,
    /// Genre
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub genre: Option<String>,
    /// Release date/year
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub date: Option<String>,
    /// Track number within the disc
    pub track_number: Option<u32>,
    /// Disc number
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub disc_number: Option<u32>,
    /// Duration in seconds
    pub duration: f64,
    /// File path
    pub uri: PathBuf,
    /// File type/extension (e.g., ".mp3", ".flac")
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub file_type: String,
    /// Reference to album ID
    pub album_id: Option<i64>,
    /// Reference to artist ID
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub artist_id: Option<i64>,
    /// Reference to cover art ID
    /// Reference to cover art ID
    pub cover_art_id: Option<i64>,
    /// Accent color extracted from cover
    pub accent_color: Option<Color>,
    /// Secondary color extracted from cover
    pub secondary_color: Option<Color>,
    /// Tertiary color extracted from cover
    pub tertiary_color: Option<Color>,
    /// Whether the user marked this track as a favorite
    pub favorite: bool,
}

impl Track {
    /// Get the display artist (`album_artist` if available, otherwise artist)
    #[must_use]
    pub fn display_artist(&self) -> &str {
        if self.album_artist.is_empty() {
            &self.artist
        } else {
            &self.album_artist
        }
    }
}
