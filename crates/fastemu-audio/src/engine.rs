//! The engine: one per app, feeding one output stream, holding a group of
//! players for each running instance.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::event::EventQueue;
use crate::gain::unit_gain;
use crate::group::{Group, GroupLimits};
use crate::midi::soundfont::SoundFont;
use crate::mixer::{GroupId, GroupState, Mixer};
use crate::voice::PlayerState;

/// The rates the engine mixes at.
const RATES: std::ops::RangeInclusive<u32> = 8_000..=384_000;

/// How the engine starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    /// The output rate in frames per second. An output stream sets the
    /// device's rate when it opens.
    pub sample_rate: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
        }
    }
}

/// The sound engine. Cloning it is cheap and every clone is the same
/// engine.
///
/// The engine does no work on its own: whoever owns the output calls
/// [`Engine::render`] for each buffer, which mixes every playing player.
/// With the `output` feature, `AudioOutput` does that from the audio
/// device's thread; [`NullOutput`](crate::NullOutput) does it in real
/// time without a device; tests call it directly.
#[derive(Clone)]
pub struct Engine {
    pub(crate) shared: Arc<Shared>,
}

pub(crate) struct Shared {
    mixer: Mutex<Mixer>,
    pub events: EventQueue,
    soundfont: Mutex<Option<SoundFont>>,
    next_id: AtomicU64,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        let rate = config.sample_rate.clamp(*RATES.start(), *RATES.end());
        Self {
            shared: Arc::new(Shared {
                mixer: Mutex::new(Mixer::new(rate)),
                events: EventQueue::default(),
                soundfont: Mutex::new(None),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    /// Locks the mixer. A panic while it was held (a bug) leaves it
    /// usable rather than silencing every game.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Mixer> {
        self.shared
            .mixer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn next_id(&self) -> u64 {
        self.shared.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Delivers the events queued so far. Call with the mixer unlocked.
    pub(crate) fn deliver(&self) {
        self.shared.events.deliver();
    }

    pub fn sample_rate(&self) -> u32 {
        self.lock().rate()
    }

    /// Changes the output rate, as when the audio device changes. Players
    /// keep playing; each is resampled to the new rate.
    pub fn set_sample_rate(&self, rate: u32) {
        self.lock()
            .set_rate(rate.clamp(*RATES.start(), *RATES.end()));
    }

    /// The gain of the whole mix, 1.0 by default.
    pub fn master_volume(&self) -> f32 {
        self.lock().master
    }

    /// Sets the gain of the whole mix, from 0.0 (silent) to 1.0.
    pub fn set_master_volume(&self, volume: f32) {
        self.lock().master = unit_gain(volume);
    }

    /// Sets the SoundFont MIDI players created from now on use, or `None`
    /// for the built-in synthesizer. Players already created keep theirs.
    pub fn set_soundfont(&self, soundfont: Option<SoundFont>) {
        *self
            .shared
            .soundfont
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = soundfont;
    }

    pub fn soundfont(&self) -> Option<SoundFont> {
        self.shared
            .soundfont
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Makes a group for one instance's players. Dropping every clone of
    /// the group, or calling [`Group::close`], closes its players.
    pub fn create_group(&self) -> Group {
        let id = GroupId(self.next_id());
        self.lock().groups.insert(
            id,
            GroupState {
                volume: 1.0,
                muted: false,
                paused: false,
                listener: None,
                limits: GroupLimits::default(),
            },
        );
        Group::new(self.clone(), id)
    }

    /// Mixes the next `out.len() / 2` frames of interleaved stereo at the
    /// output rate, advancing every playing player, and delivers the
    /// events that playback caused.
    pub fn render(&self, out: &mut [f32]) {
        self.lock().render(out, &self.shared.events);
        self.deliver();
    }

    /// Moves time forward by `frames` without keeping the sound: for a
    /// runner with no output that still needs players to end.
    pub fn advance(&self, frames: usize) {
        let mut buffer = [0.0f32; 1024];
        let mut left = frames;
        while left > 0 {
            let count = left.min(buffer.len() / 2);
            self.render(&mut buffer[..count * 2]);
            left -= count;
        }
    }

    /// How many players exist, playing or not, across every group.
    pub fn player_count(&self) -> usize {
        self.lock()
            .slots
            .values()
            .filter(|slot| !slot.voice.one_shot)
            .count()
    }

    /// How many players and tones are playing now.
    pub fn playing_count(&self) -> usize {
        self.lock()
            .slots
            .values()
            .filter(|slot| slot.voice.state == PlayerState::Started)
            .count()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new(EngineConfig::default())
    }
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mixer = self.lock();
        f.debug_struct("Engine")
            .field("sample_rate", &mixer.rate())
            .field("groups", &mixer.groups.len())
            .field("players", &mixer.slots.len())
            .finish()
    }
}
