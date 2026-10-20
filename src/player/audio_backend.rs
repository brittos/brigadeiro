// SPDX-License-Identifier: GPL-3.0
//
// Audio playback backend using rodio/symphonia

use std::fs::File;

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use lofty::file::AudioFile;
use lofty::probe::Probe;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use tokio::sync::mpsc;

/// Playback state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Paused,
    Playing,
}

/// Actions sent from the audio backend to the player controller
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Playback state changed
    StateChanged(PlaybackState),
    /// Position tick update (in seconds)
    Tick(u64),
    /// End of stream reached
    EndOfStream,
    /// Error occurred
    Error(String),
    /// Duration of current track (in seconds)
    Duration(f64),
}

/// Audio backend using rodio for playback
pub struct AudioBackend {
    /// Event sender to notify the player controller
    event_tx: mpsc::UnboundedSender<PlaybackEvent>,
    /// rodio output stream (must be kept alive and provides the mixer)
    stream: Option<MixerDeviceSink>,
    /// The audio sink for playback control
    sink: Option<Player>,
    /// Current playback state
    state: PlaybackState,
    /// Current volume (0.0 - 1.0)
    volume: f64,
    /// Duration of current track in seconds
    duration: Option<f64>,
    /// Current URI being played
    current_uri: Option<String>,
    /// Flag for tick updates (to stop ticking when paused)
    tick_running: Arc<AtomicBool>,
    /// Accumulated playback position in milliseconds (saved on pause/seek)
    current_position_ms: u64,
    /// Timestamp of the last successful play/resume action
    last_play_time: Option<Instant>,
    /// Tick task handle
    tick_handle: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for AudioBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioBackend")
            .field("state", &self.state)
            .field("volume", &self.volume)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl AudioBackend {
    /// Create a new audio backend
    pub fn new(event_tx: mpsc::UnboundedSender<PlaybackEvent>) -> Self {
        // Initialize audio output
        let stream = match DeviceSinkBuilder::open_default_sink() {
            Ok(s) => Some(s),
            Err(e) => {
                let _ = event_tx.send(PlaybackEvent::Error(format!(
                    "Failed to initialize audio output: {e}"
                )));
                None
            }
        };

        let tick_running = Arc::new(AtomicBool::new(false));
        Self {
            event_tx,
            stream,
            sink: None,
            state: PlaybackState::default(),
            volume: 1.0,
            duration: None,
            current_uri: None,
            tick_running,
            current_position_ms: 0,
            last_play_time: None,
            tick_handle: None,
        }
    }

    /// Get or create a new sink
    fn ensure_sink(&mut self) -> bool {
        if self.sink.is_some() {
            return true;
        }

        if let Some(ref stream) = self.stream {
            let sink = Player::connect_new(stream.mixer());
            // Apply cubic curve for consistent volume perception (same as set_volume)
            let cubic = self.volume * self.volume * self.volume;
            #[allow(clippy::cast_possible_truncation)]
            sink.set_volume(cubic as f32);
            self.sink = Some(sink);
            return true;
        }
        false
    }

    /// Set playback state
    pub fn set_state(&mut self, state: PlaybackState) {
        match state {
            PlaybackState::Stopped => {
                self.stop_internal();
            }
            PlaybackState::Paused => {
                if let Some(ref sink) = self.sink {
                    sink.pause();
                    self.tick_running.store(false, Ordering::SeqCst);

                    // Save accumulated time
                    if let Some(last_time) = self.last_play_time.take() {
                        #[allow(clippy::cast_possible_truncation)]
                        let elapsed = last_time.elapsed().as_millis() as u64; // Safe: reasonable elapsed time
                        self.current_position_ms += elapsed;
                    }
                }
                self.state = state;
            }
            PlaybackState::Playing => {
                if let Some(ref sink) = self.sink {
                    sink.play();
                    self.tick_running.store(true, Ordering::SeqCst);
                    self.last_play_time = Some(Instant::now());
                }
                self.state = state;
            }
        }

        let _ = self.event_tx.send(PlaybackEvent::StateChanged(state));
    }

    fn stop_internal(&mut self) {
        self.tick_running.store(false, Ordering::SeqCst);
        self.current_position_ms = 0;
        self.last_play_time = None;

        if let Some(sink) = self.sink.take() {
            sink.stop();
        }

        self.duration = None;
        self.current_uri = None;
        self.state = PlaybackState::Stopped;
    }

    /// Load and play a file
    pub fn load_file(&mut self, uri: &str) {
        // Stop current playback
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }

        // Reset position tracking
        self.current_position_ms = 0;
        self.last_play_time = None;
        self.current_uri = Some(uri.to_string());

        // Ensure we have a sink
        if !self.ensure_sink() {
            let _ = self
                .event_tx
                .send(PlaybackEvent::Error("Failed to create sink".to_string()));
            return;
        }

        // Open and decode the file
        let path = Path::new(uri);

        // 1. Try to get duration using Lofty (more reliable for MP3/VBR)
        let mut extracted_duration = None;
        if let Ok(tagged_file) = Probe::open(path).and_then(lofty::probe::Probe::read) {
            let duration = tagged_file.properties().duration().as_secs_f64();
            if duration > 0.0 {
                extracted_duration = Some(duration);
                println!("Telemetry: Lofty extracted duration: {duration}s");
            }
        }

        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                println!("Telemetry: Failed to open file {uri}: {e}");
                let _ = self
                    .event_tx
                    .send(PlaybackEvent::Error(format!("Failed to open file: {e}")));
                return;
            }
        };

        let source = match Decoder::try_from(file) {
            Ok(s) => s,
            Err(e) => {
                println!("Telemetry: Failed to decode file {uri}: {e}");
                let _ = self
                    .event_tx
                    .send(PlaybackEvent::Error(format!("Failed to decode file: {e}")));
                return;
            }
        };

        // 2. Use rodio duration as fallback if lofty failed
        let duration = extracted_duration.or_else(|| {
            let dur = source.total_duration().map(|d| d.as_secs_f64());
            if let Some(d) = dur {
                println!("Telemetry: Rodio extracted duration: {d}s");
            }
            dur
        });

        self.duration = duration;

        if let Some(dur) = duration {
            let _ = self.event_tx.send(PlaybackEvent::Duration(dur));
        } else {
            println!("Telemetry: WARNING - Duration not found for {uri}");
        }

        // Append to sink
        if let Some(ref sink) = self.sink {
            sink.append(source);
        }

        self.set_state(PlaybackState::Playing);
    }

    /// Play (resume)
    pub fn play(&mut self) {
        if self.state == PlaybackState::Paused {
            self.set_state(PlaybackState::Playing);
        }
    }

    /// Pause
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.set_state(PlaybackState::Paused);
        }
    }

    /// Toggle play/pause
    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.play(),
            PlaybackState::Stopped => {}
        }
    }

    /// Stop playback
    pub fn stop(&mut self) {
        self.set_state(PlaybackState::Stopped);
    }

    /// Set volume (0.0 - 1.0)
    pub fn set_volume(&mut self, volume: f64) {
        let clamped = volume.clamp(0.0, 1.0);
        let adjusted = if clamped <= 0.05 { 0.0 } else { clamped };

        // Apply cubic curve for more natural volume perception
        let cubic = adjusted * adjusted * adjusted;
        self.volume = adjusted;

        if let Some(ref sink) = self.sink {
            #[allow(clippy::cast_possible_truncation)]
            sink.set_volume(cubic as f32);
        }
    }

    /// Seek to position (in seconds)
    pub fn seek(&mut self, seconds: u64) {
        let pos = Duration::from_secs(seconds);

        // Try direct seek first (supported by most Rodio decoders like symphonia)
        if let Some(ref sink) = self.sink {
            if sink.try_seek(pos).is_ok() {
                println!("Telemetry: AudioBackend::seek - try_seek SUCCESSFUL");
                self.current_position_ms = seconds * 1000;
                if self.state == PlaybackState::Playing {
                    self.last_play_time = Some(Instant::now());
                } else {
                    self.last_play_time = None;
                }
                return;
            }
            println!("Telemetry: AudioBackend::seek - try_seek FAILED, using fallback");
        }

        // Fallback: Reopen and skip (slower but reliable)
        let Some(uri) = self.current_uri.clone() else {
            return;
        };

        let was_playing = self.state == PlaybackState::Playing;

        // Stop current playback
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }

        // Reopen the file
        let path = Path::new(&uri);
        let Ok(file) = File::open(path) else {
            return;
        };

        let Ok(source) = Decoder::try_from(file) else {
            return;
        };

        // Skip to position
        let skipped_source = source.skip_duration(pos);

        // Ensure sink and append
        if self.ensure_sink()
            && let Some(ref sink) = self.sink
        {
            sink.append(skipped_source);

            // Update position tracking
            self.current_position_ms = seconds * 1000;

            if was_playing {
                sink.play();
                self.tick_running.store(true, Ordering::SeqCst);
                self.last_play_time = Some(Instant::now());
            } else {
                sink.pause();
                self.last_play_time = None;
            }
        }
    }

    /// Process a tick update (intended to be called periodically from the main loop)
    pub fn tick(&mut self) {
        if self.state == PlaybackState::Playing {
            // Check for EOS
            if let Some(ref sink) = self.sink
                && sink.empty()
            {
                self.set_state(PlaybackState::Stopped);
                let _ = self.event_tx.send(PlaybackEvent::EndOfStream);
                return;
            }

            // Calculate exact elapsed time
            let mut total_ms = self.current_position_ms;
            if let Some(last_time) = self.last_play_time {
                #[allow(clippy::cast_possible_truncation)]
                let elapsed = last_time.elapsed().as_millis() as u64; // Safe: reasonable elapsed time
                total_ms += elapsed;
            }

            let tick_secs = total_ms / 1000;
            let _ = self.event_tx.send(PlaybackEvent::Tick(tick_secs));
        }
    }
}

impl Drop for AudioBackend {
    fn drop(&mut self) {
        if let Some(handle) = self.tick_handle.take() {
            handle.abort();
        }
    }
}
