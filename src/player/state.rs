// SPDX-License-Identifier: GPL-3.0
//
// Player state - holds current playback state for UI binding

use std::sync::Arc;

use crate::model::track::Track;

use super::audio_backend::PlaybackState;
use super::queue::RepeatMode;

/// Player state for UI binding
#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    /// Current repeat mode
    pub repeat_mode: RepeatMode,
    /// Whether shuffle is enabled
    pub shuffle: bool,
    /// Current playback state
    pub playback_state: PlaybackState,
    /// Current position in seconds
    pub position: u64,
    /// Current track
    pub current_track: Option<Arc<Track>>,
    /// Current volume (0.0 - 1.0)
    pub volume: f64,
    /// Whether queue is empty
    pub is_empty: bool,
    /// Queue title (e.g., album name)
    pub queue_title: String,
    /// Total remaining time in queue
    pub queue_time_remaining: f64,
}

impl PlayerState {
    /// Create a new player state
    #[must_use]
    pub fn new() -> Self {
        Self {
            volume: 1.0,
            is_empty: true,
            ..Default::default()
        }
    }

    /// Check if currently playing
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playback_state == PlaybackState::Playing
    }

    /// Get current track title
    #[must_use]
    pub fn title(&self) -> Option<&str> {
        self.current_track.as_ref().map(|t| t.title.as_str())
    }

    /// Get current track artist
    #[must_use]
    pub fn artist(&self) -> Option<&str> {
        self.current_track.as_ref().map(|t| t.display_artist())
    }

    /// Get current track album
    #[must_use]
    pub fn album(&self) -> Option<&str> {
        self.current_track.as_ref().map(|t| t.album.as_str())
    }

    /// Get current track duration
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.current_track.as_ref().map_or(0.0, |t| t.duration)
    }

    /// Get current track cover art ID
    #[must_use]
    pub fn cover_art_id(&self) -> Option<i64> {
        self.current_track.as_ref().and_then(|t| t.cover_art_id)
    }

    /// Format position as "M:SS"
    #[must_use]
    pub fn position_string(&self) -> String {
        let minutes = self.position / 60;
        let seconds = self.position % 60;
        format!("{minutes}:{seconds:02}")
    }

    /// Format duration as "M:SS"
    #[must_use]
    pub fn duration_string(&self) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_secs = self.duration().max(0.0) as u64; // Safe: duration is non-negative
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes}:{seconds:02}")
    }

    /// Format remaining queue time
    #[must_use]
    pub fn queue_time_remaining_string(&self) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let total_secs = self.queue_time_remaining.max(0.0) as u64; // Safe: time is non-negative
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;

        if hours > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{minutes} min")
        }
    }
}
