// SPDX-License-Identifier: GPL-3.0
//
// Database management - SQLite backend

use chrono::Utc;
pub use cosmic::iced::Color;
use rusqlite::{Connection, OptionalExtension, Result, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database manager for the application
#[derive(Clone, Debug)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open a connection to the database at the given path
    pub fn open(path: PathBuf) -> Result<Self> {
        let conn = Connection::open(path)?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.setup_db_tables()?;
        Ok(db)
    }

    /// Open an in-memory database (mainly for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.setup_db_tables()?;
        Ok(db)
    }

    /// Create all necessary tables if they don't exist
    fn setup_db_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // 1. Music_Folders
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Music_Folders (
                id  INTEGER PRIMARY KEY,
                uri TEXT NOT NULL
            );",
            [],
        )?;

        // 2. File_URIs
        conn.execute(
            "CREATE TABLE IF NOT EXISTS File_URIs (
                id  INTEGER PRIMARY KEY,
                uri TEXT NOT NULL,
                last_modified TIMESTAMP
            );",
            [],
        )?;

        // 3. Folder_File_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Folder_File_Junction (
                id  INTEGER PRIMARY KEY,
                folder_id INTEGER NOT NULL,
                file_id INTEGER NOT NULL,
                FOREIGN KEY (folder_id) REFERENCES Music_Folders(id),
                FOREIGN KEY (file_id) REFERENCES File_URIs(id)
            );",
            [],
        )?;

        // 4. Genres
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Genres (
                id  INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );",
            [],
        )?;

        // 5. Artists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Artists (
                id  INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                image_fetched INTEGER NOT NULL
            );",
            [],
        )?;

        // 6. Albums
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Albums (
                id  INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                artist TEXT NOT NULL,
                date TEXT NOT NULL,
                genre TEXT NOT NULL,
                cover_art_id INTEGER,
                accent_color TEXT,
                FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
            );",
            [],
        )?;

        // 7. Cover_Art
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Cover_Art (
                id  INTEGER PRIMARY KEY,
                data BLOB NOT NULL,
                thing INTEGER
            );",
            [],
        )?;

        // 8. Tracks
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Tracks (
                id  INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                filetype TEXT NOT NULL,
                album_name TEXT NOT NULL,
                date TEXT NOT NULL,
                duration REAL NOT NULL,
                track_number INTEGER NOT NULL,
                disc_number INTEGER NOT NULL,
                album_artist TEXT NOT NULL,
                file_uri_id INTEGER NOT NULL,
                cover_art_id INTEGER,
                accent_color TEXT,
                secondary_color TEXT,
                tertiary_color TEXT,
                FOREIGN KEY (file_uri_id) REFERENCES File_URIs(id),
                FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
            );",
            [],
        )?;

        // 9. Plays
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Plays (
                id  INTEGER PRIMARY KEY,
                playtime TIMESTAMP,
                track_id INTEGER NOT NULL,
                album_id INTEGER NOT NULL,
                album_artist_id INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES Tracks(id),
                FOREIGN KEY (album_id) REFERENCES Albums(id),
                FOREIGN KEY (album_artist_id) REFERENCES Artists(id)
            );",
            [],
        )?;

        // 10. Playlists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Playlists (
                id  INTEGER PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                creation_time TIMESTAMP,
                modify_time TIMESTAMP
            );",
            [],
        )?;

        // 11. Playlist_Entries
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Playlist_Entries (
                id  INTEGER PRIMARY KEY,
                playlist_position INTEGER NOT NULL
            );",
            [],
        )?;

        // 12. Playlist_Entry_Track_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Playlist_Entry_Track_Junction (
                id  INTEGER PRIMARY KEY,
                playlist_entry_id INTEGER NOT NULL,
                track_id INTEGER NOT NULL,
                FOREIGN KEY (playlist_entry_id) REFERENCES Playlist_Entries(id),
                FOREIGN KEY (track_id) REFERENCES Tracks(id)
            );",
            [],
        )?;

        // 13. Playlist_Entry_Playlist_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Playlist_Entry_Playlist_Junction (
                id  INTEGER PRIMARY KEY,
                playlist_entry_id INTEGER NOT NULL,
                playlist_id INTEGER NOT NULL,
                FOREIGN KEY (playlist_entry_id) REFERENCES Playlist_Entries(id),
                FOREIGN KEY (playlist_id) REFERENCES Playlists(id)
            );",
            [],
        )?;

        // 14. Track_Folder_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Track_Folder_Junction (
                id  INTEGER PRIMARY KEY,
                track_id INTEGER NOT NULL,
                folder_id INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES Tracks(id),
                FOREIGN KEY (folder_id) REFERENCES Music_Folders(id)
            );",
            [],
        )?;

        // 15. Track_Album_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Track_Album_Junction (
                id  INTEGER PRIMARY KEY,
                track_id INTEGER NOT NULL,
                album_id INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES Tracks(id),
                FOREIGN KEY (album_id) REFERENCES Albums(id)
            );",
            [],
        )?;

        // 16. Track_Artist_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Track_Artist_Junction (
                id  INTEGER PRIMARY KEY,
                track_id INTEGER NOT NULL,
                artist_id INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES Tracks(id),
                FOREIGN KEY (artist_id) REFERENCES Artists(id)
            );",
            [],
        )?;

        // 17. Track_Cover_Art_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Track_Cover_Art_Junction (
                id  INTEGER PRIMARY KEY,
                track_id INTEGER NOT NULL,
                cover_art_id INTEGER NOT NULL,
                FOREIGN KEY (track_id) REFERENCES Tracks(id),
                FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
            );",
            [],
        )?;

        // 18. Album_Cover_Art_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Album_Cover_Art_Junction (
                id  INTEGER PRIMARY KEY,
                album_id INTEGER NOT NULL,
                cover_art_id INTEGER NOT NULL,
                FOREIGN KEY (album_id) REFERENCES Albums(id),
                FOREIGN KEY (cover_art_id) REFERENCES Cover_Art(id)
            );",
            [],
        )?;

        // 19. Album_Artist_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Album_Artist_Junction (
                id  INTEGER PRIMARY KEY,
                album_id INTEGER NOT NULL,
                artist_id INTEGER NOT NULL,
                FOREIGN KEY (album_id) REFERENCES Albums(id),
                FOREIGN KEY (artist_id) REFERENCES Artists(id)
            );",
            [],
        )?;

        // 20. Album_Genre_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Album_Genre_Junction (
                id  INTEGER PRIMARY KEY,
                album_id INTEGER NOT NULL,
                genre_id INTEGER NOT NULL,
                FOREIGN KEY (album_id) REFERENCES Albums(id),
                FOREIGN KEY (genre_id) REFERENCES Genres(id)
            );",
            [],
        )?;

        // 21. Discog_Artist_Image
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Discog_Artist_Image (
                id  INTEGER PRIMARY KEY,
                url TEXT NOT NULL,
                data BLOB NOT NULL,
                thing INTEGER
            );",
            [],
        )?;

        // 22. Artist_Discog_Artist_Image_Junction
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Artist_Discog_Artist_Image_Junction (
                id  INTEGER PRIMARY KEY,
                artist_id INTEGER NOT NULL,
                image_id INTEGER NOT NULL,
                FOREIGN KEY (artist_id) REFERENCES Artists(id),
                FOREIGN KEY (image_id) REFERENCES Discog_Artist_Image(id)
            );",
            [],
        )?;

        // Migration: Add missing columns for visual metadata (if they don't exist)
        // We use let _ = to ignore "duplicate column" errors
        let _ = conn.execute(
            "ALTER TABLE Albums ADD COLUMN cover_art_id INTEGER REFERENCES Cover_Art(id)",
            [],
        );
        let _ = conn.execute("ALTER TABLE Albums ADD COLUMN accent_color TEXT", []);

        let _ = conn.execute(
            "ALTER TABLE Tracks ADD COLUMN cover_art_id INTEGER REFERENCES Cover_Art(id)",
            [],
        );
        let _ = conn.execute("ALTER TABLE Tracks ADD COLUMN accent_color TEXT", []);
        let _ = conn.execute("ALTER TABLE Tracks ADD COLUMN secondary_color TEXT", []);
        let _ = conn.execute("ALTER TABLE Tracks ADD COLUMN tertiary_color TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE Tracks ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0",
            [],
        );

        Ok(())
    }

    /// Columns shared by every query that materializes a [`Track`](crate::model::track::Track).
    const TRACK_COLUMNS: &'static str =
        "t.id, t.title, t.album_artist, t.album_name, t.date, t.duration,
                    t.track_number, t.disc_number, f.uri, t.filetype,
                    t.accent_color, t.secondary_color, t.tertiary_color, t.cover_art_id,
                    t.favorite";

    /// Build a [`Track`](crate::model::track::Track) from a row selected with [`Self::TRACK_COLUMNS`].
    fn track_from_row(row: &rusqlite::Row<'_>) -> Result<crate::model::track::Track> {
        let accent_hex: Option<String> = row.get(10)?;
        let secondary_hex: Option<String> = row.get(11)?;
        let tertiary_hex: Option<String> = row.get(12)?;

        let parse_color = |hex: Option<String>| -> Option<Color> {
            hex.and_then(|h| {
                if h.starts_with('#') && h.len() == 7 {
                    let r = u8::from_str_radix(&h[1..3], 16).ok()?;
                    let g = u8::from_str_radix(&h[3..5], 16).ok()?;
                    let b = u8::from_str_radix(&h[5..7], 16).ok()?;
                    Some(Color::from_rgb8(r, g, b))
                } else {
                    None
                }
            })
        };

        Ok(crate::model::track::Track {
            id: row.get(0)?,
            title: row.get(1)?,
            artist: row.get(2)?,
            album_artist: row.get(2)?,
            album: row.get(3)?,
            date: Some(row.get(4)?),
            duration: row.get(5)?,
            track_number: Some(row.get(6)?),
            disc_number: Some(row.get(7)?),
            uri: PathBuf::from(row.get::<_, String>(8)?),
            file_type: row.get(9)?,
            accent_color: parse_color(accent_hex),
            secondary_color: parse_color(secondary_hex),
            tertiary_color: parse_color(tertiary_hex),
            cover_art_id: row.get(13)?,
            favorite: row.get::<_, i64>(14)? != 0,
            ..Default::default()
        })
    }

    /// Save full track metadata to the database
    pub fn save_track_metadata(
        &self,
        path: &PathBuf,
        metadata: &crate::utils::metadata::AudioMetadata,
    ) -> Result<i64> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let id = Self::save_track_metadata_inner(&tx, path, metadata)?;
        tx.commit()?;
        Ok(id)
    }

    /// Save multiple tracks in a single transaction (MUCH faster for scanning)
    pub fn save_tracks_batch(
        &self,
        tracks: &[(PathBuf, crate::utils::metadata::AudioMetadata)],
    ) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        for (path, metadata) in tracks {
            let _ = Self::save_track_metadata_inner(&tx, path, metadata)?;
        }

        tx.commit()?;
        Ok(())
    }

    fn save_track_metadata_inner(
        conn: &rusqlite::Connection,
        path: &PathBuf,
        metadata: &crate::utils::metadata::AudioMetadata,
    ) -> Result<i64> {
        let file_uri = path.to_string_lossy().to_string();

        // 1. Get or create File_URI
        let file_id = if let Ok(id) = conn.query_row(
            "SELECT id FROM File_URIs WHERE uri = ?",
            [&file_uri],
            |row| row.get::<_, i64>(0),
        ) {
            id
        } else {
            conn.execute(
                "INSERT INTO File_URIs (uri, last_modified) VALUES (?, ?)",
                params![file_uri, 0], // FIXME: add actual timestamp
            )?;
            conn.last_insert_rowid()
        };

        // 2. Get or create Artist
        let artist_name = metadata.artist.as_deref().unwrap_or("Unknown Artist");
        let artist_id = if let Ok(id) = conn.query_row(
            "SELECT id FROM Artists WHERE name = ?",
            [artist_name],
            |row| row.get::<_, i64>(0),
        ) {
            id
        } else {
            conn.execute(
                "INSERT INTO Artists (name, image_fetched) VALUES (?, ?)",
                params![artist_name, 0],
            )?;
            conn.last_insert_rowid()
        };

        // 3. Get or create Album
        let album_name = metadata.album.as_deref().unwrap_or("Unknown Album");
        let album_id = if let Ok(id) = conn.query_row(
            "SELECT id FROM Albums WHERE title = ? AND artist = ?",
            [album_name, artist_name],
            |row| row.get::<_, i64>(0),
        ) {
            id
        } else {
            conn.execute(
                "INSERT INTO Albums (title, artist, date, genre) VALUES (?, ?, ?, ?)",
                params![
                    album_name,
                    artist_name,
                    metadata.date.as_deref().unwrap_or(""),
                    metadata.genre.as_deref().unwrap_or("")
                ],
            )?;
            conn.last_insert_rowid()
        };

        // 4. Create Track (or update if exists - for simplicity we just insert for now)
        // Check if track already exists for this file_id
        let track_id = if let Ok(id) = conn.query_row(
            "SELECT id FROM Tracks WHERE file_uri_id = ?",
            [file_id],
            |row| row.get::<_, i64>(0),
        ) {
            id
        } else {
            // Handle Cover Art
            let cover_art_id = if let Some(data) = &metadata.picture_data {
                let data = data.as_slice();
                // Check if cover already exists (deduplication by content)
                let existing_id: Option<i64> = conn
                    .query_row("SELECT id FROM Cover_Art WHERE data = ?", [data], |row| {
                        row.get(0)
                    })
                    .ok();

                if let Some(id) = existing_id {
                    Some(id)
                } else {
                    conn.execute("INSERT INTO Cover_Art (data) VALUES (?)", [data])
                        .ok();
                    Some(conn.last_insert_rowid())
                }
            } else {
                None
            };

            conn.execute(
                "INSERT INTO Tracks (title, filetype, album_name, date, duration, track_number, disc_number, album_artist, file_uri_id, cover_art_id) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    metadata.title.as_deref().unwrap_or("Unknown Track"),
                    metadata.file_type,
                    album_name,
                    metadata.date.as_deref().unwrap_or(""),
                    metadata.duration,
                    metadata.track_number.unwrap_or(0),
                    metadata.disc_number.unwrap_or(0),
                    metadata.album_artist.as_deref().unwrap_or(artist_name),
                    file_id,
                    cover_art_id
                ],
            )?;
            conn.last_insert_rowid()
        };

        // Auto-fix: Update cover_art_id for existing tracks if missing
        if let Ok(current_cover_id) = conn.query_row::<Option<i64>, _, _>(
            "SELECT cover_art_id FROM Tracks WHERE id = ?",
            [track_id],
            |row| row.get(0),
        ) && current_cover_id.is_none()
            && let Some(data) = &metadata.picture_data
        {
            let data = data.as_slice();
            // Deduplication check
            let cover_id = if let Ok(id) = conn.query_row::<i64, _, _>(
                "SELECT id FROM Cover_Art WHERE data = ?",
                [data],
                |row| row.get(0),
            ) {
                id
            } else {
                conn.execute("INSERT INTO Cover_Art (data) VALUES (?)", [data])
                    .unwrap_or(0);
                conn.last_insert_rowid()
            };

            conn.execute(
                "UPDATE Tracks SET cover_art_id = ? WHERE id = ?",
                params![cover_id, track_id],
            )
            .ok();
        }

        // 5. Create Junctions if they don't exist

        // Track_Artist_Junction
        conn.execute(
            "INSERT OR IGNORE INTO Track_Artist_Junction (track_id, artist_id) VALUES (?, ?)",
            params![track_id, artist_id],
        )?;

        // Track_Album_Junction
        conn.execute(
            "INSERT OR IGNORE INTO Track_Album_Junction (track_id, album_id) VALUES (?, ?)",
            params![track_id, album_id],
        )?;

        // Album_Artist_Junction
        conn.execute(
            "INSERT OR IGNORE INTO Album_Artist_Junction (album_id, artist_id) VALUES (?, ?)",
            params![album_id, artist_id],
        )?;

        Ok(track_id)
    }

    /// Get all tracks from the database
    pub fn get_all_tracks(&self) -> Result<Vec<crate::model::track::Track>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {}
             FROM Tracks t
             JOIN File_URIs f ON t.file_uri_id = f.id",
            Self::TRACK_COLUMNS
        ))?;

        let track_iter = stmt.query_map([], Self::track_from_row)?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }

        Ok(tracks)
    }

    /// Get all tracks marked as favorite, ordered by title
    pub fn get_favorite_tracks(&self) -> Result<Vec<crate::model::track::Track>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {}
             FROM Tracks t
             JOIN File_URIs f ON t.file_uri_id = f.id
             WHERE t.favorite <> 0
             ORDER BY t.title COLLATE NOCASE ASC",
            Self::TRACK_COLUMNS
        ))?;

        let track_iter = stmt.query_map([], Self::track_from_row)?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }

        Ok(tracks)
    }

    /// Mark or unmark a track as favorite
    pub fn set_track_favorite(&self, track_id: i64, favorite: bool) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE Tracks SET favorite = ? WHERE id = ?",
            params![i64::from(favorite), track_id],
        )?;
        Ok(())
    }

    /// Update visual metadata for a track
    pub fn update_track_visuals(
        &self,
        track_id: i64,
        accent: Option<Color>,
        secondary: Option<Color>,
        tertiary: Option<Color>,
        cover_art_id: Option<i64>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let hex = |c: Color| {
            format!(
                "#{:02X}{:02X}{:02X}",
                (c.r * 255.0) as u8,
                (c.g * 255.0) as u8,
                (c.b * 255.0) as u8
            )
        };

        conn.execute(
            "UPDATE Tracks SET accent_color = ?, secondary_color = ?, tertiary_color = ?, cover_art_id = ? WHERE id = ?",
            params![
                accent.map(hex),
                secondary.map(hex),
                tertiary.map(hex),
                cover_art_id,
                track_id
            ],
        )?;

        Ok(())
    }

    /// Retrieve cover art data (BLOB) by ID
    pub fn get_cover_art(&self, id: i64) -> Result<Vec<u8>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM Cover_Art WHERE id = ?")?;
        let data: Vec<u8> = stmt.query_row([id], |row| row.get(0))?;
        Ok(data)
    }

    /// Clear the entire library (for rescanning)
    pub fn clear_library(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // Delete order matters due to foreign keys
        // Junctions first
        conn.execute("DELETE FROM Track_Artist_Junction", [])?;
        conn.execute("DELETE FROM Track_Album_Junction", [])?;
        conn.execute("DELETE FROM Album_Artist_Junction", [])?;
        conn.execute("DELETE FROM Playlist_Entry_Track_Junction", [])?;
        conn.execute("DELETE FROM Playlist_Entry_Playlist_Junction", [])?;

        // Entities
        conn.execute("DELETE FROM Playlist_Entries", [])?;
        conn.execute("DELETE FROM Playlists", [])?;
        conn.execute("DELETE FROM Tracks", [])?;
        conn.execute("DELETE FROM Albums", [])?;
        conn.execute("DELETE FROM Artists", [])?;
        conn.execute("DELETE FROM Cover_Art", [])?;

        // Note: We might want to keep File_URIs to avoid full re-hash if we were smart,
        // but for a "Fresh Rescan" it's better to clear everything except maybe Music_Folders?
        // Let's clear File_URIs too to be safe.
        conn.execute("DELETE FROM File_URIs", [])?;

        Ok(())
    }
    // --- Playlist Management ---

    /// Create a new playlist
    pub fn create_playlist(&self, title: String, description: Option<String>) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        conn.execute(
            "INSERT INTO Playlists (title, description, creation_time, modify_time) VALUES (?, ?, ?, ?)",
            params![
                title,
                description.unwrap_or_default(),
                now,
                now
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Delete a playlist and all its entries
    pub fn delete_playlist(&self, playlist_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // 1. Get all playlist entries for this playlist
        let mut stmt = conn.prepare(
            "SELECT playlist_entry_id FROM Playlist_Entry_Playlist_Junction WHERE playlist_id = ?",
        )?;
        let entry_ids: Vec<i64> = stmt
            .query_map([playlist_id], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        // 2. Delete junctions and entries
        for entry_id in entry_ids {
            conn.execute(
                "DELETE FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = ?",
                [entry_id],
            )?;
            conn.execute(
                "DELETE FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = ?",
                [entry_id],
            )?;
            conn.execute("DELETE FROM Playlist_Entries WHERE id = ?", [entry_id])?;
        }

        // 3. Delete playlist itself
        conn.execute("DELETE FROM Playlists WHERE id = ?", [playlist_id])?;

        Ok(())
    }

    /// Update playlist metadata (title, description)
    pub fn update_playlist_metadata(
        &self,
        playlist_id: i64,
        title: String,
        description: Option<String>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        conn.execute(
            "UPDATE Playlists SET title = ?, description = ?, modify_time = ? WHERE id = ?",
            params![title, description.unwrap_or_default(), now, playlist_id],
        )?;
        Ok(())
    }

    /// Get all playlists
    pub fn get_all_playlists(&self) -> Result<Vec<crate::model::playlist::Playlist>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, title, description, creation_time, modify_time FROM Playlists ORDER BY title ASC")?;

        let mut playlists: Vec<crate::model::playlist::Playlist> = stmt
            .query_map([], |row| {
                Ok(crate::model::playlist::Playlist {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: Some(row.get(2)?),
                    created_at: row.get(3)?,
                    modified_at: row.get(4)?,
                    track_ids: Vec::new(),
                })
            })?
            .filter_map(Result::ok)
            .collect();

        let mut ids_stmt = conn.prepare(
            "SELECT petj.track_id
             FROM Playlist_Entries pe
             JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
             JOIN Playlist_Entry_Track_Junction petj ON pe.id = petj.playlist_entry_id
             WHERE pepj.playlist_id = ?
             ORDER BY pe.playlist_position ASC",
        )?;
        for playlist in &mut playlists {
            playlist.track_ids = ids_stmt
                .query_map([playlist.id], |row| row.get(0))?
                .filter_map(Result::ok)
                .collect();
        }

        Ok(playlists)
    }

    /// Add a track to a playlist
    pub fn add_track_to_playlist(&self, playlist_id: i64, track_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        // 1. Determine new position (append at end)
        // We need to join everything to find the max position for this playlist.
        // MAX() over an empty set yields a single NULL row, hence Option<i64>.
        let max_pos: Option<i64> = tx
            .query_row(
                "SELECT MAX(pe.playlist_position) 
             FROM Playlist_Entries pe
             JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
             WHERE pepj.playlist_id = ?",
                [playlist_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten();

        let next_pos = max_pos.unwrap_or(0) + 1;

        // 2. Create Playlist_Entry
        tx.execute(
            "INSERT INTO Playlist_Entries (playlist_position) VALUES (?)",
            [next_pos],
        )?;
        let entry_id = tx.last_insert_rowid();

        // 3. Link Entry -> Playlist
        tx.execute(
            "INSERT INTO Playlist_Entry_Playlist_Junction (playlist_entry_id, playlist_id) VALUES (?, ?)",
            params![entry_id, playlist_id],
        )?;

        // 4. Link Entry -> Track
        tx.execute(
            "INSERT INTO Playlist_Entry_Track_Junction (playlist_entry_id, track_id) VALUES (?, ?)",
            params![entry_id, track_id],
        )?;

        // 5. Update playlist modified time
        let now = Utc::now();
        tx.execute(
            "UPDATE Playlists SET modify_time = ? WHERE id = ?",
            params![now, playlist_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Remove a track from a playlist by position
    pub fn remove_track_from_playlist(&self, playlist_id: i64, position: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;

        // 1. Find the entry ID at this position for this playlist
        let entry_id: i64 = tx.query_row(
            "SELECT pe.id 
             FROM Playlist_Entries pe
             JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
             WHERE pepj.playlist_id = ? AND pe.playlist_position = ?",
            params![playlist_id, position],
            |row| row.get(0),
        )?;

        // 2. Delete Junctions
        tx.execute(
            "DELETE FROM Playlist_Entry_Playlist_Junction WHERE playlist_entry_id = ?",
            [entry_id],
        )?;
        tx.execute(
            "DELETE FROM Playlist_Entry_Track_Junction WHERE playlist_entry_id = ?",
            [entry_id],
        )?;

        // 3. Delete Entry
        tx.execute("DELETE FROM Playlist_Entries WHERE id = ?", [entry_id])?;

        // 4. Reorder remaining entries
        tx.execute(
            "UPDATE Playlist_Entries 
             SET playlist_position = playlist_position - 1
             WHERE id IN (
                 SELECT pe.id 
                 FROM Playlist_Entries pe
                 JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
                 WHERE pepj.playlist_id = ? AND pe.playlist_position > ?
             )",
            params![playlist_id, position],
        )?;

        // Update modified time
        let now = Utc::now();
        tx.execute(
            "UPDATE Playlists SET modify_time = ? WHERE id = ?",
            params![now, playlist_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Get all tracks for a playlist in order
    pub fn get_playlist_tracks(&self, playlist_id: i64) -> Result<Vec<crate::model::track::Track>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(&format!(
            "SELECT {}
             FROM Tracks t
             JOIN File_URIs f ON t.file_uri_id = f.id
             JOIN Playlist_Entry_Track_Junction petj ON t.id = petj.track_id
             JOIN Playlist_Entries pe ON petj.playlist_entry_id = pe.id
             JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
             WHERE pepj.playlist_id = ?
             ORDER BY pe.playlist_position ASC",
            Self::TRACK_COLUMNS
        ))?;

        let track_iter = stmt.query_map([playlist_id], Self::track_from_row)?;

        let mut tracks = Vec::new();
        for track in track_iter {
            tracks.push(track?);
        }
        Ok(tracks)
    }

    /// Get cover art IDs for the first N tracks of a playlist (for collage)
    pub fn get_playlist_cover_art_ids(&self, playlist_id: i64, limit: usize) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT DISTINCT t.cover_art_id
             FROM Tracks t
             JOIN Playlist_Entry_Track_Junction petj ON t.id = petj.track_id
             JOIN Playlist_Entries pe ON petj.playlist_entry_id = pe.id
             JOIN Playlist_Entry_Playlist_Junction pepj ON pe.id = pepj.playlist_entry_id
             WHERE pepj.playlist_id = ? AND t.cover_art_id IS NOT NULL
             ORDER BY pe.playlist_position ASC
             LIMIT ?",
        )?;

        let ids = stmt
            .query_map(params![playlist_id, limit], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();

        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::utils::metadata::AudioMetadata;
    use std::path::PathBuf;

    fn insert_track(db: &Database, title: &str) -> i64 {
        let metadata = AudioMetadata {
            title: Some(title.to_string()),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            duration: 120.0,
            file_type: "mp3".to_string(),
            ..Default::default()
        };
        let path = PathBuf::from(format!("/music/{title}.mp3"));
        db.save_track_metadata(&path, &metadata)
            .expect("track should be saved")
    }

    #[test]
    fn tracks_are_not_favorite_by_default() {
        let db = Database::open_in_memory().unwrap();
        insert_track(&db, "a");

        let tracks = db.get_all_tracks().unwrap();
        assert_eq!(tracks.len(), 1);
        assert!(!tracks[0].favorite);
        assert!(db.get_favorite_tracks().unwrap().is_empty());
    }

    #[test]
    fn set_track_favorite_round_trips() {
        let db = Database::open_in_memory().unwrap();
        let a = insert_track(&db, "a");
        let b = insert_track(&db, "b");

        db.set_track_favorite(a, true).unwrap();

        let favorites = db.get_favorite_tracks().unwrap();
        assert_eq!(favorites.iter().map(|t| t.id).collect::<Vec<_>>(), vec![a]);
        assert!(favorites[0].favorite);

        // The flag is also visible through the regular library query
        let all = db.get_all_tracks().unwrap();
        assert!(all.iter().find(|t| t.id == a).unwrap().favorite);
        assert!(!all.iter().find(|t| t.id == b).unwrap().favorite);

        db.set_track_favorite(a, false).unwrap();
        assert!(db.get_favorite_tracks().unwrap().is_empty());
    }

    #[test]
    fn favorite_tracks_are_sorted_by_title() {
        let db = Database::open_in_memory().unwrap();
        let zeta = insert_track(&db, "Zeta");
        let alpha = insert_track(&db, "alpha");
        db.set_track_favorite(zeta, true).unwrap();
        db.set_track_favorite(alpha, true).unwrap();

        let ids: Vec<i64> = db
            .get_favorite_tracks()
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![alpha, zeta]);
    }

    #[test]
    fn add_and_remove_tracks_from_playlist_updates_count_and_order() {
        let db = Database::open_in_memory().unwrap();
        let a = insert_track(&db, "a");
        let b = insert_track(&db, "b");
        let c = insert_track(&db, "c");
        let playlist = db.create_playlist("Mix".to_string(), None).unwrap();

        // Empty playlist reports zero tracks
        let playlists = db.get_all_playlists().unwrap();
        assert_eq!(playlists.len(), 1);
        assert_eq!(playlists[0].track_count(), 0);

        db.add_track_to_playlist(playlist, a).unwrap();
        db.add_track_to_playlist(playlist, b).unwrap();
        db.add_track_to_playlist(playlist, c).unwrap();

        let playlists = db.get_all_playlists().unwrap();
        assert_eq!(playlists[0].track_count(), 3);
        assert_eq!(playlists[0].track_ids, vec![a, b, c]);

        let ids: Vec<i64> = db
            .get_playlist_tracks(playlist)
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![a, b, c]);

        // Remove the middle entry (positions are 1-based) and check reordering
        db.remove_track_from_playlist(playlist, 2).unwrap();

        let ids: Vec<i64> = db
            .get_playlist_tracks(playlist)
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![a, c]);
        assert_eq!(db.get_all_playlists().unwrap()[0].track_count(), 2);

        // Removing position 2 again now removes `c`, proving positions were compacted
        db.remove_track_from_playlist(playlist, 2).unwrap();
        let ids: Vec<i64> = db
            .get_playlist_tracks(playlist)
            .unwrap()
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, vec![a]);
    }

    #[test]
    fn same_track_can_appear_twice_in_a_playlist() {
        let db = Database::open_in_memory().unwrap();
        let a = insert_track(&db, "a");
        let playlist = db.create_playlist("Mix".to_string(), None).unwrap();

        db.add_track_to_playlist(playlist, a).unwrap();
        db.add_track_to_playlist(playlist, a).unwrap();

        assert_eq!(db.get_all_playlists().unwrap()[0].track_count(), 2);
    }

    #[test]
    fn playlist_tracks_carry_the_favorite_flag() {
        let db = Database::open_in_memory().unwrap();
        let a = insert_track(&db, "a");
        let playlist = db.create_playlist("Mix".to_string(), None).unwrap();
        db.add_track_to_playlist(playlist, a).unwrap();
        db.set_track_favorite(a, true).unwrap();

        let tracks = db.get_playlist_tracks(playlist).unwrap();
        assert_eq!(tracks.len(), 1);
        assert!(tracks[0].favorite);
    }
}
