// SPDX-License-Identifier: GPL-3.0
//
// Repeat mode for audio playback

/// Repeat mode for the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Normal,
    Loop,
    LoopSong,
}
