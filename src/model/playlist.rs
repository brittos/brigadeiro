// SPDX-License-Identifier: GPL-3.0
//
// Playlist model - represents a user-created playlist

use chrono::{DateTime, Utc};

/// Synthetic id used by the built-in "Favorites" collection, which is not stored
/// in the `Playlists` table but is rendered with the playlist detail page.
pub const FAVORITES_PLAYLIST_ID: i64 = -1;

/// Represents a playlist
#[derive(Debug, Clone)]
pub struct Playlist {
    /// Unique identifier
    pub id: i64,
    /// Playlist title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Creation timestamp
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    #[allow(dead_code)] // preenchido pelo banco, a tela ainda não lê
    pub modified_at: DateTime<Utc>,
    /// Track IDs in order
    pub track_ids: Vec<i64>,
}

impl Default for Playlist {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            title: String::new(),
            description: None,
            created_at: now,
            modified_at: now,
            track_ids: Vec::new(),
        }
    }
}

impl Playlist {
    /// Build the synthetic "Favorites" collection
    #[must_use]
    pub fn favorites(title: String) -> Self {
        Self {
            id: FAVORITES_PLAYLIST_ID,
            title,
            ..Default::default()
        }
    }

    /// Whether this is the built-in "Favorites" collection
    #[must_use]
    pub fn is_favorites(&self) -> bool {
        self.id == FAVORITES_PLAYLIST_ID
    }

    /// Get track count
    #[must_use]
    pub fn track_count(&self) -> usize {
        self.track_ids.len()
    }
}
