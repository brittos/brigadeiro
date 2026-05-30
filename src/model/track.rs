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
    pub genre: Option<String>,
    /// Release date/year
    pub date: Option<String>,
    /// Track number within the disc
    pub track_number: Option<u32>,
    /// Disc number
    pub disc_number: Option<u32>,
    /// Duration in seconds
    pub duration: f64,
    /// File path
    pub uri: PathBuf,
    /// File type/extension (e.g., ".mp3", ".flac")
    pub file_type: String,
    /// Reference to album ID
    pub album_id: Option<i64>,
    /// Reference to artist ID
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
}

impl Track {
    /// Create a new track with the given title and uri
    #[must_use]
    pub fn new(title: String, uri: PathBuf) -> Self {
        Self {
            title,
            uri,
            ..Default::default()
        }
    }

    /// Get formatted duration as "M:SS"
    #[must_use]
    pub fn duration_string(&self) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_secs = self.duration.max(0.0) as u32; // Safe: duration is non-negative
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes}:{seconds:02}")
    }

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
