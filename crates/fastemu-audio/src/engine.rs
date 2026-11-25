//! The engine: one per app, feeding one output stream, holding a group of
//! players for each running instance.

use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{Mutex, MutexGuard};

use crate::event::EventQueue;
use crate::gain::{SharedGain, unit_gain};
use crate::group::{Group, GroupLimits};
use crate::midi::soundfont::SoundFont;
use crate::mixer::{GroupId, GroupState, Mixer};
use crate::voice::PlayerState;

/// The rates the engine mixes at.
const RATES: std::ops::RangeInclusive<u32> = 8_000..=384_000;

/// Frames mixed under one hold of the mixer lock. A buffer of a device is
/// rendered in slices of this size, with the lock free between them, so
/// another game's call waits for one slice at most, however big the buffer
/// and however many players the audio thread mixes.
const SLICE_FRAMES: usize = 128;

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

    /// Locks the mixer. The lock is fair: a caller that has waited a
    /// moment is next, so a thread that takes it again and again cannot
    /// keep the audio thread or another game's calls out. It has no
    /// poison either: a panic while it was held leaves the mixer usable
    /// rather than silencing every game.
    pub(crate) fn lock(&self) -> MutexGuard<'_, Mixer> {
        self.shared.mixer.lock()
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
        *self.shared.soundfont.lock() = soundfont;
    }

    pub fn soundfont(&self) -> Option<SoundFont> {
        self.shared.soundfont.lock().clone()
    }

    /// Makes a group for one instance's players. Dropping every clone of
    /// the group, or calling [`Group::close`], closes its players.
    pub fn create_group(&self) -> Group {
        let id = GroupId(self.next_id());
        let volume = Arc::new(SharedGain::new(1.0));
        self.lock().groups.insert(
            id,
            GroupState {
                volume: Arc::clone(&volume),
                muted: false,
                paused: false,
                listener: None,
                limits: GroupLimits::default(),
            },
        );
        Group::new(self.clone(), id, volume)
    }

    /// Mixes the next `out.len() / 2` frames of interleaved stereo at the
    /// output rate, advancing every playing player, and delivers the
    /// events that playback caused.
    ///
    /// It never unwinds: audio threads run it, and a panic in a decoder
    /// or a listener must not end them. A voice that panics is closed, with
    /// an `Error` event; anything else that panics while mixing leaves a
    /// slice of silence.
    pub fn render(&self, out: &mut [f32]) {
        for slice in out.chunks_mut(SLICE_FRAMES * 2) {
            let mixed = catch_unwind(AssertUnwindSafe(|| {
                self.lock().render(slice, &self.shared.events);
            }));
            if mixed.is_err() {
                slice.fill(0.0);
            }
            // A listener that panics loses its events, not the audio.
            let _ = catch_unwind(AssertUnwindSafe(|| self.deliver()));
        }
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

    /// How many events were dropped because listeners were far behind: the
    /// queue holds at most [`MAX_QUEUED`](crate::MAX_QUEUED) of them.
    pub fn dropped_events(&self) -> u64 {
        self.shared.events.dropped()
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
