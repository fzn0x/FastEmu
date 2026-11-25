//! Sound for FastEmu: the engine behind MMAPI (JSR-135) and the vendor
//! sound APIs, with no dependency on the virtual machine.
//!
//! One [`Engine`] serves the whole app and feeds one output stream. Each
//! running instance gets a [`Group`] with its own volume and mute, so the
//! app can silence one game, and closing the group stops and frees all
//! of that game's sound. A group makes [`Player`]s from sound data (WAV,
//! MIDI, MP3, AMR, MMAPI tone sequences, Nokia ringing tones) shaped like
//! MMAPI players: realize, prefetch, start, stop, close, loop counts,
//! media time in microseconds, volume levels, and [`PlayerEvent`]s for
//! `PlayerListener`. `Manager.playTone` is [`Group::play_tone`].
//!
//! ```
//! use fastemu_audio::{Engine, EngineConfig, PlayerEventKind};
//!
//! let engine = Engine::new(EngineConfig { sample_rate: 8000 });
//! let game = engine.create_group();
//! let events = game.subscribe();
//! // A tone sequence: VERSION 1, then A4 for a quarter note.
//! let player = game.create_player(&[0xFE, 1, 69, 16], Some("audio/x-tone-seq"))?;
//! player.start()?;
//! let mut buffer = vec![0.0; 2 * 8000];
//! engine.render(&mut buffer);
//! let kinds: Vec<_> = events.try_iter().map(|e| e.kind).collect();
//! assert!(matches!(kinds.last(), Some(PlayerEventKind::EndOfMedia { .. })));
//! # Ok::<(), fastemu_audio::AudioError>(())
//! ```
//!
//! Every player is resampled to the output rate, summed in f32 with
//! headroom, and passed through a soft limiter; starts, stops and volume
//! changes ramp over a few milliseconds so they do not click.
//!
//! Without the `output` feature the engine renders only when
//! [`Engine::render`] is called, for tests and the headless runner;
//! [`NullOutput`] renders in real time and discards the sound. With it,
//! `AudioOutput` plays through the system's audio device.

mod amr;
mod engine;
pub mod error;
mod event;
pub mod format;
mod gain;
mod group;
mod media;
mod midi;
mod mixer;
mod mp3;
mod nokia;
mod output;
mod pcm;
mod player;
mod source;
mod tone;
mod voice;
mod wav;

pub use engine::{Engine, EngineConfig};
pub use error::AudioError;
pub use event::{EventCallback, PlayerEvent, PlayerEventKind, PlayerId};
pub use format::{Format, SUPPORTED_CONTENT_TYPES};
pub use group::{Group, GroupLimits};
pub use midi::soundfont::SoundFont;
pub use mixer::GroupId;
pub use output::NullOutput;
#[cfg(feature = "output")]
pub use output::{AudioOutput, OutputError, OutputInfo};
pub use pcm::MAX_DECODED_BYTES;
pub use player::Player;
pub use voice::PlayerState;
