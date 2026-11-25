//! The mix: every group's voices resampled to the output rate, summed in
//! f32 with headroom, and passed through a soft limiter.

use std::any::Any;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::event::{EventCallback, EventQueue, PlayerEvent, PlayerEventKind, PlayerId};
use crate::group::GroupLimits;
use crate::voice::{PlayerState, Reached, Voice};

/// Starts, stops and volume changes slide over this long.
pub(crate) const RAMP_SECONDS: f32 = 0.003;
/// The mix is scaled by this before limiting, so a few sounds at full
/// level sum without hitting the limiter.
const HEADROOM: f32 = 0.7;
/// The limiter keeps peaks under this, and the soft clip above it keeps
/// the output inside -1 to 1.
const THRESHOLD: f32 = 0.9;
const LIMITER_RELEASE_SECONDS: f32 = 0.15;
/// Fade-outs waiting to be heard; past this, new ones are dropped rather
/// than letting closing hundreds of players cost memory.
const MAX_TAILS: usize = 64;
/// Frames mixed per pass, so the scratch buffer stays small.
const CHUNK: usize = 512;
/// The longest message of a panic that an `Error` event carries.
const MAX_PANIC_MESSAGE: usize = 200;

/// Identifies a group (one per instance) for the life of the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GroupId(pub u64);

pub(crate) struct GroupState {
    pub volume: f32,
    pub muted: bool,
    pub paused: bool,
    pub listener: Option<EventCallback>,
    /// How many players and how much sound the group may hold.
    pub limits: GroupLimits,
}

impl GroupState {
    fn gain(&self) -> f32 {
        if self.muted { 0.0 } else { self.volume }
    }
}

/// A voice inside the mix, with the group it belongs to.
pub(crate) struct Slot {
    pub group: GroupId,
    pub voice: Voice,
}

/// A short fade-out still to be mixed.
struct Tail {
    samples: Vec<f32>,
    pos: usize,
}

pub(crate) struct Mixer {
    rate: u32,
    pub master: f32,
    pub groups: HashMap<GroupId, GroupState>,
    pub slots: HashMap<PlayerId, Slot>,
    tails: Vec<Tail>,
    scratch: Vec<f32>,
    limiter: Limiter,
}

impl Mixer {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            master: 1.0,
            groups: HashMap::new(),
            slots: HashMap::new(),
            tails: Vec::new(),
            scratch: vec![0.0; CHUNK * 2],
            limiter: Limiter::new(rate),
        }
    }

    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn set_rate(&mut self, rate: u32) {
        if rate != self.rate {
            self.rate = rate;
            self.tails.clear();
            self.limiter = Limiter::new(rate);
        }
    }

    /// The per-frame gain change that makes a full swing take
    /// [`RAMP_SECONDS`].
    fn slew(&self) -> f32 {
        1.0 / (RAMP_SECONDS * self.rate as f32)
    }

    /// Queues an event for the listener of the player's group, if any.
    pub fn emit(&self, events: &EventQueue, id: PlayerId, kind: PlayerEventKind) {
        let Some(slot) = self.slots.get(&id) else {
            return;
        };
        if slot.voice.one_shot {
            return;
        }
        if let Some(listener) = self
            .groups
            .get(&slot.group)
            .and_then(|g| g.listener.clone())
        {
            events.push(listener, PlayerEvent { player: id, kind });
        }
    }

    /// Fades the player out over a few milliseconds from where it is, if
    /// it is being heard, and leaves its position where it was.
    pub fn fade_out(&mut self, id: PlayerId) {
        let frames = (RAMP_SECONDS * self.rate as f32) as usize;
        let slew = self.slew();
        // With the tails full, skip the fade rather than render it for
        // nothing: closing a game with a hundred players stays cheap.
        if self.tails.len() >= MAX_TAILS {
            return;
        }
        let Some(slot) = self.slots.get_mut(&id) else {
            return;
        };
        if slot.voice.state != PlayerState::Started {
            return;
        }
        let step = f64::from(slot.voice.source.rate()) / f64::from(self.rate);
        let samples = slot.voice.fade_out(frames, step, slew);
        if self.tails.len() < MAX_TAILS && samples.iter().any(|&v| v != 0.0) {
            self.tails.push(Tail { samples, pos: 0 });
        }
    }

    /// Renders `out.len() / 2` stereo frames.
    pub fn render(&mut self, out: &mut [f32], events: &EventQueue) {
        out.fill(0.0);
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let count = (frames - done).min(CHUNK);
            // A panic between taking the scratch buffer and putting it
            // back (a bug somewhere in a mix) must not leave it empty.
            if self.scratch.len() < CHUNK * 2 {
                self.scratch = vec![0.0; CHUNK * 2];
            }
            let mut chunk = std::mem::take(&mut self.scratch);
            chunk[..count * 2].fill(0.0);
            self.mix_tails(&mut chunk[..count * 2]);
            self.mix_voices(&mut chunk[..count * 2], events);
            let gain = self.master * HEADROOM;
            for value in &mut chunk[..count * 2] {
                *value *= gain;
            }
            self.limiter.process(&mut chunk[..count * 2]);
            out[done * 2..(done + count) * 2].copy_from_slice(&chunk[..count * 2]);
            self.scratch = chunk;
            done += count;
        }
        self.slots
            .retain(|_, slot| !(slot.voice.one_shot && slot.voice.at_end));
    }

    fn mix_tails(&mut self, chunk: &mut [f32]) {
        for tail in &mut self.tails {
            let left = &tail.samples[tail.pos..];
            let count = left.len().min(chunk.len());
            for (out, &value) in chunk.iter_mut().zip(&left[..count]) {
                *out += value;
            }
            tail.pos += count;
        }
        self.tails.retain(|tail| tail.pos < tail.samples.len());
    }

    /// Mixes every started voice into `chunk`. A voice whose source panics
    /// (a decoder or synthesizer bug on damaged data) is stopped and
    /// closed for good, with an `Error` event and the end of its run, and
    /// the others play on: the audio thread never unwinds for one game's
    /// sound.
    fn mix_voices(&mut self, chunk: &mut [f32], events: &EventQueue) {
        let slew = self.slew();
        let rate = self.rate;
        let mut failed: Vec<(PlayerId, String)> = Vec::new();
        for (&id, slot) in &mut self.slots {
            let voice = &mut slot.voice;
            if voice.state != PlayerState::Started {
                continue;
            }
            let Some(group) = self.groups.get(&slot.group) else {
                continue;
            };
            if group.paused {
                continue;
            }
            let target = voice.own_gain() * group.gain();
            let step = f64::from(voice.source.rate()) / f64::from(rate);
            let listener = if voice.one_shot {
                None
            } else {
                group.listener.clone()
            };
            let rendered = catch_unwind(AssertUnwindSafe(|| {
                voice.render(chunk, step, target, slew, false, |reached, duration_us| {
                    let Some(listener) = &listener else { return };
                    let push = |kind| {
                        events.push(listener.clone(), PlayerEvent { player: id, kind });
                    };
                    match reached {
                        Reached::Loop => {
                            push(PlayerEventKind::EndOfMedia {
                                media_time_us: duration_us,
                                last: false,
                            });
                            push(PlayerEventKind::Started { media_time_us: 0 });
                        }
                        Reached::End { error } => {
                            if let Some(message) = error {
                                push(PlayerEventKind::Error { message });
                            }
                            push(PlayerEventKind::EndOfMedia {
                                media_time_us: duration_us,
                                last: true,
                            });
                        }
                    }
                });
            }));
            if let Err(payload) = rendered {
                voice.fault();
                failed.push((id, panic_message(payload.as_ref())));
            }
        }
        for (id, message) in failed {
            let message = format!("the sound failed while playing: {message}");
            self.emit(events, id, PlayerEventKind::Error { message });
            self.emit(
                events,
                id,
                PlayerEventKind::EndOfMedia {
                    media_time_us: 0,
                    last: true,
                },
            );
        }
    }
}

/// The text of a panic, cut to a length an event can carry.
fn panic_message(payload: &(dyn Any + Send)) -> String {
    let text = payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a panic with no message".to_owned());
    text.chars().take(MAX_PANIC_MESSAGE).collect()
}

/// A peak limiter with instant attack and a soft clip after it, so the
/// sum of many games never wraps or clips hard.
struct Limiter {
    envelope: f32,
    release: f32,
}

impl Limiter {
    fn new(rate: u32) -> Self {
        Self {
            envelope: 0.0,
            release: (-1.0 / (LIMITER_RELEASE_SECONDS * rate.max(1) as f32)).exp(),
        }
    }

    fn process(&mut self, samples: &mut [f32]) {
        for frame in samples.as_chunks_mut::<2>().0 {
            let peak = frame[0].abs().max(frame[1].abs());
            self.envelope = if peak > self.envelope {
                peak
            } else {
                peak + (self.envelope - peak) * self.release
            };
            let gain = if self.envelope > THRESHOLD {
                THRESHOLD / self.envelope
            } else {
                1.0
            };
            for value in frame {
                *value = soft_clip(*value * gain);
            }
        }
    }
}

/// Leaves samples under the threshold alone and bends the rest smoothly
/// toward 1.
fn soft_clip(x: f32) -> f32 {
    let magnitude = x.abs();
    if magnitude <= THRESHOLD {
        return x;
    }
    let room = 1.0 - THRESHOLD;
    let bent = THRESHOLD + room * ((magnitude - THRESHOLD) / room).tanh();
    bent.copysign(x)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::engine::{Engine, EngineConfig};
    use crate::group::Group;
    use crate::pcm::{Pcm, PcmSource};
    use crate::source::Source;

    #[test]
    fn the_soft_clip_stays_inside_full_scale() {
        assert_eq!(soft_clip(0.5), 0.5);
        assert_eq!(soft_clip(-0.9), -0.9);
        assert!(soft_clip(3.0) <= 1.0 && soft_clip(3.0) > 0.95);
        assert!(soft_clip(-30.0) >= -1.0);
        assert!(soft_clip(0.95) > soft_clip(0.92));
    }

    #[test]
    fn the_limiter_tames_loud_sums_and_recovers() {
        let mut limiter = Limiter::new(1000);
        let mut loud = vec![4.0; 200];
        limiter.process(&mut loud);
        assert!(loud.iter().all(|v| v.abs() < 1.0));
        // A quiet passage after the loud one comes back to full level.
        let mut quiet = vec![0.2; 4000];
        limiter.process(&mut quiet);
        assert!((quiet[3998] - 0.2).abs() < 1e-3);
    }

    /// A decoder with a bug: it panics when asked for sound.
    struct Bomb;

    impl Source for Bomb {
        fn rate(&self) -> u32 {
            8000
        }

        fn frames(&self) -> u64 {
            80_000
        }

        fn render(&mut self, _out: &mut [f32]) -> usize {
            panic!("decoder bug on damaged data");
        }

        fn seek(&mut self, _frame: u64) {}

        fn memory_bytes(&self) -> usize {
            1000
        }
    }

    /// Puts a started voice over `source` in the group, and returns its id.
    fn play(engine: &Engine, group: &Group, id: u64, source: Box<dyn Source>) -> PlayerId {
        let mut voice = Voice::new(source, false);
        voice.start();
        let id = PlayerId(1_000_000 + id);
        engine.lock().slots.insert(
            id,
            Slot {
                group: group.id(),
                voice,
            },
        );
        id
    }

    fn constant(frames: usize) -> Box<dyn Source> {
        Box::new(PcmSource::new(Arc::new(Pcm {
            rate: 8000,
            channels: 1,
            samples: vec![8000; frames],
        })))
    }

    #[test]
    fn a_voice_that_panics_is_closed_and_the_others_play_on() {
        let engine = Engine::new(EngineConfig { sample_rate: 8000 });
        let group = engine.create_group();
        let events = group.subscribe();
        let healthy = play(&engine, &group, 1, constant(8000));
        let bomb = play(&engine, &group, 2, Box::new(Bomb));
        let mut out = vec![0.0f32; 2 * 1000];
        // The panic stays inside the voice.
        engine.render(&mut out);
        assert!(
            out.iter().any(|v| v.abs() > 0.05),
            "the healthy voice is silent"
        );
        {
            let mixer = engine.lock();
            let voice = &mixer.slots[&bomb].voice;
            assert_eq!(voice.state, PlayerState::Prefetched);
            assert!(voice.at_end);
            assert_eq!(voice.source.memory_bytes(), 0);
            assert_eq!(mixer.slots[&healthy].voice.state, PlayerState::Started);
        }
        let seen: Vec<_> = events.try_iter().filter(|e| e.player == bomb).collect();
        assert_eq!(seen.len(), 2, "{seen:?}");
        match &seen[0].kind {
            PlayerEventKind::Error { message } => {
                assert!(message.contains("decoder bug on damaged data"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            seen[1].kind,
            PlayerEventKind::EndOfMedia { last: true, .. }
        ));
        // It is not asked again: no more panics, no more events.
        engine.render(&mut out);
        assert_eq!(events.try_iter().filter(|e| e.player == bomb).count(), 0);
        // A start that follows plays nothing and ends at once.
        engine.lock().slots.get_mut(&bomb).unwrap().voice.start();
        engine.render(&mut out);
        let ended = events
            .try_iter()
            .filter(|e| e.player == bomb)
            .map(|e| e.kind)
            .collect::<Vec<_>>();
        assert!(matches!(
            ended.as_slice(),
            [PlayerEventKind::EndOfMedia { last: true, .. }]
        ));
    }

    #[test]
    fn a_listener_that_panics_does_not_end_the_render() {
        let engine = Engine::new(EngineConfig { sample_rate: 8000 });
        let group = engine.create_group();
        group.set_listener(Some(Arc::new(|_| panic!("listener bug"))));
        play(&engine, &group, 1, constant(200));
        play(&engine, &group, 2, constant(8000));
        let mut out = vec![0.0f32; 2 * 800];
        // The first voice ends in this render and its listener panics.
        engine.render(&mut out);
        assert!(out.iter().any(|v| v.abs() > 0.05));
        // The mix of the next render is whole too.
        engine.render(&mut out);
        assert!(out.iter().any(|v| v.abs() > 0.05));
        // Closing the group tells the listener, on this thread.
        group.set_listener(None);
    }
}
