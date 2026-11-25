//! The mix: every group's voices resampled to the output rate, summed in
//! f32 with headroom, and passed through a soft limiter.

use std::collections::HashMap;

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

    fn mix_voices(&mut self, chunk: &mut [f32], events: &EventQueue) {
        let slew = self.slew();
        let rate = self.rate;
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
        }
    }
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
    use super::*;

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
}
