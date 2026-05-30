// SPDX-License-Identifier: GPL-3.0
//
// Queue management for audio playback

use std::sync::Arc;

use rand::prelude::SliceRandom;
use tokio::sync::mpsc;

use crate::model::track::Track;

/// Repeat mode for the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Normal,
    Loop,
    LoopSong,
}

impl RepeatMode {
    /// Create from integer (for settings persistence)
    #[must_use]
    pub fn from_int(mode: i32) -> Self {
        match mode {
            1 => Self::Loop,
            2 => Self::LoopSong,
            _ => Self::Normal,
        }
    }

    /// Convert to integer (for settings persistence)
    #[must_use]
    pub fn to_int(self) -> i32 {
        match self {
            Self::Normal => 0,
            Self::Loop => 1,
            Self::LoopSong => 2,
        }
    }
}

/// Events emitted by the queue
#[derive(Debug, Clone)]
pub enum QueueEvent {
    /// Queue contents changed
    Updated,
    /// Current position changed
    PositionChanged(usize),
    /// Queue became empty
    Empty,
    /// Queue became non-empty
    NonEmpty,
    /// Repeat mode changed
    RepeatModeChanged(RepeatMode),
    /// Total remaining duration changed
    DurationChanged(f64),
}

/// Audio playback queue
#[derive(Debug)]
pub struct Queue {
    /// Event sender
    event_tx: mpsc::UnboundedSender<QueueEvent>,
    /// Current queue (may be shuffled)
    queue: Vec<Arc<Track>>,
    /// Original queue order (for un-shuffling)
    sequential_queue: Vec<Arc<Track>>,
    /// Current position in queue
    current_position: usize,
    /// Current repeat mode
    repeat_mode: RepeatMode,
    /// Whether to loop when in shuffle mode
    shuffle_loop: bool,
}

impl Queue {
    /// Create a new queue
    pub fn new(event_tx: mpsc::UnboundedSender<QueueEvent>) -> Self {
        Self {
            event_tx,
            queue: Vec::new(),
            sequential_queue: Vec::new(),
            current_position: 0,
            repeat_mode: RepeatMode::default(),
            shuffle_loop: false,
        }
    }

    /// Get current track
    #[must_use]
    pub fn current_track(&self) -> Option<Arc<Track>> {
        self.queue.get(self.current_position).cloned()
    }

    /// Set a single track (clears queue and adds track)
    pub fn set_track(&mut self, track: Arc<Track>) {
        self.clear();
        self.add_track(track);
        self.current_position = 0;
        let _ = self.event_tx.send(QueueEvent::PositionChanged(0));
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.queue.clear();
        self.sequential_queue.clear();
        self.current_position = 0;
        let _ = self.event_tx.send(QueueEvent::Empty);
    }

    /// Add a track to the queue
    pub fn add_track(&mut self, track: Arc<Track>) {
        self.queue.push(track.clone());
        self.sequential_queue.push(track);

        let _ = self.event_tx.send(QueueEvent::NonEmpty);
        let _ = self.event_tx.send(QueueEvent::Updated);
        self.send_duration_update();
    }

    /// Add multiple tracks
    pub fn add_tracks(&mut self, tracks: Vec<Arc<Track>>) {
        for track in tracks {
            self.queue.push(track.clone());
            self.sequential_queue.push(track);
        }

        if self.repeat_mode == RepeatMode::Loop {
            // Already handled or similar logic
        }

        let _ = self.event_tx.send(QueueEvent::NonEmpty);
        let _ = self.event_tx.send(QueueEvent::Updated);
        self.send_duration_update();
    }

    /// Set an album (clears queue and adds all tracks)
    pub fn set_album(&mut self, tracks: Vec<Arc<Track>>) {
        self.clear();
        self.add_tracks(tracks);
        self.current_position = 0;
        let _ = self.event_tx.send(QueueEvent::PositionChanged(0));
    }

    /// Set current position
    pub fn set_position(&mut self, position: usize) {
        if self.queue.is_empty() {
            return;
        }

        if position >= self.queue.len() {
            self.end_queue();
            return;
        }

        self.current_position = position;
        let _ = self.event_tx.send(QueueEvent::PositionChanged(position));
        self.send_duration_update();
    }

    /// Remove track at position
    pub fn remove_track(&mut self, position: usize) {
        if position >= self.queue.len() {
            return;
        }

        self.queue.remove(position);

        // Adjust current position if needed
        if self.queue.is_empty() {
            self.current_position = 0;
            let _ = self.event_tx.send(QueueEvent::Empty);
        } else if position < self.current_position {
            self.current_position = self.current_position.saturating_sub(1);
        } else if self.current_position >= self.queue.len() {
            self.current_position = self.queue.len().saturating_sub(1);
        }

        let _ = self.event_tx.send(QueueEvent::Updated);
        self.send_duration_update();
    }

    /// Get previous track
    pub fn previous(&mut self) -> Option<Arc<Track>> {
        if self.queue.is_empty() {
            let _ = self.event_tx.send(QueueEvent::Empty);
            return None;
        }

        match self.repeat_mode {
            RepeatMode::Normal => {
                if self.current_position > 0 {
                    self.current_position -= 1;
                }
            }
            RepeatMode::Loop => {
                if self.current_position == 0 {
                    self.current_position = self.queue.len().saturating_sub(1);
                } else {
                    self.current_position -= 1;
                }
            }
            RepeatMode::LoopSong => {
                // Stay on same song
            }
        }

        let _ = self
            .event_tx
            .send(QueueEvent::PositionChanged(self.current_position));
        self.send_duration_update();
        self.current_track()
    }

    /// Get next track
    pub fn next(&mut self) -> Option<Arc<Track>> {
        if self.queue.is_empty() {
            let _ = self.event_tx.send(QueueEvent::Empty);
            return None;
        }

        let next_pos = self.current_position + 1;

        match self.repeat_mode {
            RepeatMode::Normal => {
                if next_pos >= self.queue.len() {
                    self.end_queue();
                    return None;
                }
                self.current_position = next_pos;
            }
            RepeatMode::Loop => {
                self.current_position = if next_pos >= self.queue.len() {
                    0
                } else {
                    next_pos
                };
            }
            RepeatMode::LoopSong => {
                // Stay on same song
            }
        }

        let _ = self
            .event_tx
            .send(QueueEvent::PositionChanged(self.current_position));
        self.send_duration_update();
        self.current_track()
    }

    /// Set repeat mode
    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        let new_mode = if mode == self.repeat_mode {
            RepeatMode::Normal
        } else {
            mode
        };

        // Restore original order if we were shuffled?
        // This part needs rethink if shuffle is separate
        self.queue = self.sequential_queue.clone();

        self.repeat_mode = new_mode;
        let _ = self.event_tx.send(QueueEvent::RepeatModeChanged(new_mode));
    }

    /// Get current repeat mode
    #[must_use]
    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    /// Set shuffle loop mode
    pub fn set_shuffle_loop(&mut self, enabled: bool) {
        self.shuffle_loop = enabled;
    }

    /// Shuffle remaining tracks (after current position)
    fn shuffle_remaining(&mut self) {
        if self.current_position + 1 >= self.queue.len() {
            return;
        }

        let mut remaining: Vec<_> = self.queue.drain(self.current_position + 1..).collect();
        remaining.shuffle(&mut rand::rng());
        self.queue.extend(remaining);
    }

    /// End the queue
    fn end_queue(&mut self) {
        self.clear();
    }

    /// Calculate and send remaining duration
    fn send_duration_update(&self) {
        let remaining_duration: f64 = self.queue[self.current_position..]
            .iter()
            .map(|t| t.duration)
            .sum();
        let _ = self
            .event_tx
            .send(QueueEvent::DurationChanged(remaining_duration));
    }

    /// Get all tracks
    #[must_use]
    pub fn tracks(&self) -> &[Arc<Track>] {
        &self.queue
    }

    /// Get current position
    #[must_use]
    pub fn position(&self) -> usize {
        self.current_position
    }

    /// Check if queue is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Get queue length
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }
}
