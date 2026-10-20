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
}
