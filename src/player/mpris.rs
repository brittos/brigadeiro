// SPDX-License-Identifier: GPL-3.0
//
// MPRIS D-Bus integration for Brigadeiro
//
// Since `mpris_server::Player` is `!Send`/`!Sync` (uses RefCell internally),
// it runs on a dedicated thread with its own tokio LocalSet runtime.
// Communication with the main app loop happens via mpsc channels:
//   - `MprisAction`: D-Bus controller → App (commands)
//   - `MprisStateUpdate`: App → D-Bus Player (state sync)

use mpris_server::{LoopStatus, Metadata, PlaybackStatus, Time};
use tokio::sync::mpsc;

use crate::model::track::Track;
use crate::player::audio_backend::PlaybackState;
use crate::player::queue::RepeatMode;

/// Actions received from MPRIS D-Bus controllers (e.g., GNOME Shell, playerctl)
#[derive(Debug, Clone)]
pub enum MprisAction {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    /// Seek by offset (microseconds, can be negative)
    Seek(i64),
    /// Set absolute position (microseconds)
    SetPosition(i64),
    /// Set volume (0.0 - 1.0)
    SetVolume(f64),
    /// Set loop status from D-Bus
    SetLoopStatus(LoopStatus),
    /// Set shuffle from D-Bus
    SetShuffle(bool),
    Raise,
    Quit,
}

/// State updates sent from the App to the MPRIS Player thread
#[derive(Debug, Clone)]
pub enum MprisStateUpdate {
    /// Playback status changed
    PlaybackStatus(PlaybackStatus),
    /// Track metadata changed
    TrackChanged {
        title: String,
        artist: String,
        album: String,
        length_micros: i64,
    },
    /// Playback position updated (microseconds)
    Position(i64),
    /// Volume changed (0.0 - 1.0)
    Volume(f64),
    /// Loop status changed
    LoopStatus(LoopStatus),
    /// Shuffle changed
    Shuffle(bool),
    /// Capabilities changed
    CanGoNext(bool),
    /// Capabilities changed
    CanGoPrevious(bool),
}

/// Spawns the MPRIS Player on a dedicated thread.
///
/// Returns:
///   - `mpsc::UnboundedReceiver<MprisAction>`: receive commands FROM D-Bus
///   - `mpsc::UnboundedSender<MprisStateUpdate>`: send state updates TO D-Bus
pub fn spawn_mpris_thread() -> (
    mpsc::UnboundedReceiver<MprisAction>,
    mpsc::UnboundedSender<MprisStateUpdate>,
) {
    let (action_tx, action_rx) = mpsc::unbounded_channel::<MprisAction>();
    let (state_tx, state_rx) = mpsc::unbounded_channel::<MprisStateUpdate>();

    std::thread::Builder::new()
        .name("mpris-dbus".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create MPRIS tokio runtime");

            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, async move {
                mpris_main_loop(action_tx, state_rx).await;
            });
        })
        .expect("Failed to spawn MPRIS thread");

    (action_rx, state_tx)
}

/// Main loop running on the dedicated MPRIS thread (`LocalSet` context)
async fn mpris_main_loop(
    action_tx: mpsc::UnboundedSender<MprisAction>,
    mut state_rx: mpsc::UnboundedReceiver<MprisStateUpdate>,
) {
    // Build the MPRIS Player (must be on this thread since it's !Send)
    let player = match mpris_server::Player::builder("brigadeiro")
        .identity("Brigadeiro")
        .desktop_entry("io.github.brittos.brigadeiro")
        .can_play(true)
        .can_pause(true)
        .can_go_next(true)
        .can_go_previous(true)
        .can_seek(true)
        .can_control(true)
        .can_quit(true)
        .can_raise(true)
        .build()
        .await
    {
        Ok(p) => p,
        Err(e) => {
            eprintln!("MPRIS: Failed to build Player: {e}");
            return;
        }
    };

    // Wire up all callbacks
    connect_callbacks(&player, &action_tx);

    // Run the D-Bus server concurrently with processing state updates
    let run_task = player.run();
    tokio::pin!(run_task);

    loop {
        tokio::select! {
            () = &mut run_task => {
                println!("MPRIS: D-Bus server stopped");
                break;
            }
            update = state_rx.recv() => {
                if let Some(update) = update {
                    apply_state_update(&player, update).await;
                } else {
                    println!("MPRIS: State channel closed");
                    break;
                }
            }
        }
    }
}

/// Connect all D-Bus method callbacks to send `MprisAction` via the channel
fn connect_callbacks(player: &mpris_server::Player, tx: &mpsc::UnboundedSender<MprisAction>) {
    let tx_c = tx.clone();
    player.connect_play(move |_| {
        let _ = tx_c.send(MprisAction::Play);
    });

    let tx_c = tx.clone();
    player.connect_pause(move |_| {
        let _ = tx_c.send(MprisAction::Pause);
    });

    let tx_c = tx.clone();
    player.connect_play_pause(move |_| {
        let _ = tx_c.send(MprisAction::PlayPause);
    });

    let tx_c = tx.clone();
    player.connect_stop(move |_| {
        let _ = tx_c.send(MprisAction::Stop);
    });

    let tx_c = tx.clone();
    player.connect_next(move |_| {
        let _ = tx_c.send(MprisAction::Next);
    });

    let tx_c = tx.clone();
    player.connect_previous(move |_| {
        let _ = tx_c.send(MprisAction::Previous);
    });

    let tx_c = tx.clone();
    player.connect_seek(move |_, offset| {
        let _ = tx_c.send(MprisAction::Seek(offset.as_micros()));
    });

    let tx_c = tx.clone();
    player.connect_set_position(move |_, _track_id, position| {
        let _ = tx_c.send(MprisAction::SetPosition(position.as_micros()));
    });

    let tx_c = tx.clone();
    player.connect_set_volume(move |_, volume| {
        let _ = tx_c.send(MprisAction::SetVolume(volume));
    });

    let tx_c = tx.clone();
    player.connect_set_loop_status(move |_, loop_status| {
        let _ = tx_c.send(MprisAction::SetLoopStatus(loop_status));
    });

    let tx_c = tx.clone();
    player.connect_set_shuffle(move |_, shuffle| {
        let _ = tx_c.send(MprisAction::SetShuffle(shuffle));
    });

    let tx_c = tx.clone();
    player.connect_raise(move |_| {
        let _ = tx_c.send(MprisAction::Raise);
    });

    let tx_c = tx.clone();
    player.connect_quit(move |_| {
        let _ = tx_c.send(MprisAction::Quit);
    });
}

/// Apply a state update to the MPRIS Player (runs on the MPRIS thread)
async fn apply_state_update(player: &mpris_server::Player, update: MprisStateUpdate) {
    let result = match update {
        MprisStateUpdate::PlaybackStatus(status) => player.set_playback_status(status).await,
        MprisStateUpdate::TrackChanged {
            title,
            artist,
            album,
            length_micros,
        } => {
            let metadata = Metadata::builder()
                .title(title)
                .artist([artist])
                .album(album)
                .length(Time::from_micros(length_micros))
                .build();
            player.set_metadata(metadata).await
        }
        MprisStateUpdate::Position(micros) => {
            player.set_position(Time::from_micros(micros));
            Ok(())
        }
        MprisStateUpdate::Volume(vol) => player.set_volume(vol).await,
        MprisStateUpdate::LoopStatus(status) => player.set_loop_status(status).await,
        MprisStateUpdate::Shuffle(shuffle) => player.set_shuffle(shuffle).await,
        MprisStateUpdate::CanGoNext(can) => player.set_can_go_next(can).await,
        MprisStateUpdate::CanGoPrevious(can) => player.set_can_go_previous(can).await,
    };

    if let Err(e) = result {
        eprintln!("MPRIS: Failed to update state: {e}");
    }
}

// =============================================================================
// Helper functions for the app side (conversion utilities)
// =============================================================================

/// Convert our `PlaybackState` to MPRIS `PlaybackStatus`
#[must_use]
pub fn to_mpris_status(state: PlaybackState) -> PlaybackStatus {
    match state {
        PlaybackState::Playing => PlaybackStatus::Playing,
        PlaybackState::Paused => PlaybackStatus::Paused,
        PlaybackState::Stopped | PlaybackState::Loading => PlaybackStatus::Stopped,
    }
}

/// Convert our `RepeatMode` to MPRIS `LoopStatus`
#[must_use]
pub fn to_mpris_loop_status(mode: RepeatMode) -> LoopStatus {
    match mode {
        RepeatMode::Normal => LoopStatus::None,
        RepeatMode::Loop => LoopStatus::Playlist,
        RepeatMode::LoopSong => LoopStatus::Track,
    }
}

/// Convert MPRIS `LoopStatus` to our `RepeatMode`
#[must_use]
pub fn from_mpris_loop_status(status: LoopStatus) -> RepeatMode {
    match status {
        LoopStatus::None => RepeatMode::Normal,
        LoopStatus::Playlist => RepeatMode::Loop,
        LoopStatus::Track => RepeatMode::LoopSong,
    }
}

/// Build a `MprisStateUpdate::TrackChanged` from a `Track`
#[must_use]
pub fn track_to_state_update(track: &Track) -> MprisStateUpdate {
    #[allow(clippy::cast_possible_truncation)]
    let length_micros = (track.duration * 1_000_000.0) as i64;
    MprisStateUpdate::TrackChanged {
        title: track.title.clone(),
        artist: track.display_artist().to_string(),
        album: track.album.clone(),
        length_micros,
    }
}

/// Convert seconds to microseconds for MPRIS
#[must_use]
pub fn secs_to_micros(secs: u64) -> i64 {
    #[allow(clippy::cast_possible_wrap)]
    let micros = (secs as i64) * 1_000_000; // Safe: reasonable playback position
    micros
}
