//! The mix: every group's voices resampled to the output rate, summed in
//! f32 with headroom, and passed through a soft limiter.

use std::any::Any;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::budget::{self, Clock, RenderBudget, Share, SystemClock, Turn};
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

/// A started voice to mix this slice. Sorted, the voices of a group come
/// together, the one that started first first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    group: GroupId,
    start: u64,
    id: PlayerId,
    /// The voice's `frame_cost_ps`.
    cost_ps: u64,
}

pub(crate) struct Mixer {
    rate: u32,
    pub master: f32,
    pub groups: HashMap<GroupId, GroupState>,
    pub slots: HashMap<PlayerId, Slot>,
    tails: Vec<Tail>,
    scratch: Vec<f32>,
    limiter: Limiter,
    /// What the sound of each game may spend rendering, if anything.
    pub budget: Option<RenderBudget>,
    /// Where render times are read while there is a budget.
    pub clock: std::sync::Arc<dyn Clock>,
    /// The credit of each group under the budget, in nanoseconds. A group
    /// gets a full burst when it first has sound to play.
    credits: HashMap<GroupId, i64>,
    /// How many times a voice waited a slice for its game's budget.
    pub waits: u64,
    /// Counts starts, so a group's voices play in the order they started.
    starts: u64,
    /// The scratch of each slice, kept to spare allocations: the voices
    /// to mix, and for each group with one its turn, its share and its
    /// dearest voice.
    order: Vec<Entry>,
    turns: Vec<Turn>,
    shares: Vec<Share>,
    dearest: Vec<(GroupId, i64)>,
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
            budget: None,
            clock: std::sync::Arc::new(SystemClock::new()),
            credits: HashMap::new(),
            waits: 0,
            starts: 0,
            order: Vec::new(),
            turns: Vec::new(),
            shares: Vec::new(),
            dearest: Vec::new(),
        }
    }

    /// The next place in the order voices start in.
    pub fn next_start_order(&mut self) -> u64 {
        self.starts += 1;
        self.starts
    }

    /// Sets the render budget, or `None` for none. Every group starts
    /// again from a full burst.
    pub fn set_budget(&mut self, budget: Option<RenderBudget>) {
        self.budget = budget;
        self.credits.clear();
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

    /// Mixes every started voice into `chunk`, a group at a time and in
    /// the order the group's voices started. A voice whose source panics
    /// (a decoder or synthesizer bug on damaged data) is stopped and
    /// closed for good, with an `Error` event and the end of its run, and
    /// the others play on: the audio thread never unwinds for one game's
    /// sound.
    ///
    /// With a render budget, each group's voices are offered to its
    /// [`Turn`]. A voice the turn holds back fades out over the ramp, then
    /// waits silent and in place: it is not rendered, so its media time
    /// stands still and it sends no events. It fades in when it plays
    /// again. The clock is read before the first voice and after each
    /// voice that rendered, and the time between two reads is charged to
    /// the group of the voice that rendered. With no budget, no clock is
    /// read and every voice plays.
    fn mix_voices(&mut self, chunk: &mut [f32], events: &EventQueue) {
        let slew = self.slew();
        let rate = self.rate;
        let frames = chunk.len() / 2;
        let mut order = std::mem::take(&mut self.order);
        order.clear();
        for (&id, slot) in &self.slots {
            if slot.voice.state != PlayerState::Started {
                continue;
            }
            match self.groups.get(&slot.group) {
                Some(group) if !group.paused => {}
                _ => continue,
            }
            order.push(Entry {
                group: slot.group,
                start: slot.voice.start_order,
                id,
                cost_ps: slot.voice.frame_cost_ps,
            });
        }
        order.sort_unstable();
        let budget = self.budget;
        let slice_ns = frames as u64 * 1_000_000_000 / u64::from(rate.max(1));
        let mut turns = std::mem::take(&mut self.turns);
        turns.clear();
        if let Some(budget) = budget {
            self.plan(budget, slice_ns, frames, &order, &mut turns);
        }
        let cap = budget.map_or(0, RenderBudget::cap_ns);
        let mut last_read = budget.map(|_| self.clock.now_ns());
        let mut failed: Vec<(PlayerId, String)> = Vec::new();
        let mut turn = 0;
        let mut previous = None;
        for &Entry {
            group: group_id,
            id,
            ..
        } in &order
        {
            if previous.is_some_and(|previous| previous != group_id) {
                turn += 1;
            }
            previous = Some(group_id);
            let (Some(group), Some(slot)) = (self.groups.get(&group_id), self.slots.get_mut(&id))
            else {
                continue;
            };
            let voice = &mut slot.voice;
            let mut target = voice.own_gain() * group.gain();
            let plays = turns.get_mut(turn).is_none_or(|turn| {
                let credit = self.credits.get(&group_id).copied().unwrap_or(0);
                let estimate = (voice.frame_cost_ps > 0)
                    .then(|| budget::frames_ns(voice.frame_cost_ps, frames));
                turn.offer(estimate, voice.waiting, credit)
            });
            voice.waiting = !plays;
            if !plays {
                if voice.gain <= 0.0 {
                    // Silent and in place; it fades in when it comes back.
                    self.waits += 1;
                    continue;
                }
                // Still heard: it fades out first, as a stop does.
                target = 0.0;
            }
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
            if let Some(before) = last_read {
                let now = self.clock.now_ns();
                last_read = Some(now);
                let cost = now.saturating_sub(before).min(slice_ns);
                if let Some(credit) = self.credits.get_mut(&group_id) {
                    budget::charge(credit, cost, slice_ns, cap);
                }
                let per_frame = cost.saturating_mul(1000) / frames.max(1) as u64;
                voice.frame_cost_ps = budget::average(voice.frame_cost_ps, per_frame);
                if let Some(turn) = turns.get_mut(turn) {
                    turn.played(budget::frames_ns(voice.frame_cost_ps, frames));
                }
            }
            if let Err(payload) = rendered {
                voice.fault();
                failed.push((id, panic_message(payload.as_ref())));
            }
        }
        self.order = order;
        self.turns = turns;
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

    /// Gives each group with a voice in `order` its part of the budget for
    /// `slice_ns` of sound, and pushes its turn onto `turns`, in the order
    /// the groups come. A group with nothing to play gets nothing.
    fn plan(
        &mut self,
        budget: RenderBudget,
        slice_ns: u64,
        frames: usize,
        order: &[Entry],
        turns: &mut Vec<Turn>,
    ) {
        let cap = budget.cap_ns();
        if self.credits.len() > self.groups.len() {
            let groups = &self.groups;
            self.credits.retain(|id, _| groups.contains_key(id));
        }
        let mut shares = std::mem::take(&mut self.shares);
        let mut dearest = std::mem::take(&mut self.dearest);
        shares.clear();
        dearest.clear();
        for entry in order {
            let cost = if entry.cost_ps > 0 {
                budget::frames_ns(entry.cost_ps, frames)
            } else {
                0
            };
            match dearest.last_mut() {
                Some((group, most)) if *group == entry.group => *most = (*most).max(cost),
                _ => {
                    dearest.push((entry.group, cost));
                    let credit = *self.credits.entry(entry.group).or_insert(cap);
                    shares.push(Share { credit, part: 0 });
                }
            }
        }
        budget::refill(&mut shares, budget.allowance_ns(slice_ns), cap);
        for (&(group, most), &share) in dearest.iter().zip(&shares) {
            self.credits.insert(group, share.credit);
            turns.push(Turn::new(budget, share, most));
        }
        self.shares = shares;
        self.dearest = dearest;
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

    mod budget {
        use std::sync::atomic::{AtomicU64, Ordering};
        use std::time::Duration;

        use super::*;
        use crate::budget::{Clock, RenderBudget};

        /// The test rate: a slice of 128 frames is 4 ms of sound, and a
        /// frame 31250 ns.
        const RATE: u32 = 32_000;
        const SLICE: usize = 128;
        const FRAME_NS: u64 = 31_250;
        /// The frames a voice reads from its source at a time: a sound
        /// costs nothing in one slice and two slices' worth in the next.
        const BLOCK: u64 = 256;
        const SECOND_NS: u64 = 1_000_000_000;
        /// The burst of the default budget.
        const BURST_NS: u64 = RenderBudget::DEFAULT.burst.as_nanos() as u64;
        /// 2 s of sound.
        const RUN: usize = 500;

        /// A clock only the test's sounds move.
        #[derive(Default)]
        struct TestClock(AtomicU64);

        impl Clock for TestClock {
            fn now_ns(&self) -> u64 {
                self.0.load(Ordering::Relaxed)
            }
        }

        /// An endless sound of one level that moves the test clock by
        /// `ns_per_frame` for every frame it makes. With `stall_at`, its
        /// call of that number moves it a whole second more, as when the
        /// system stops the audio thread in the middle of a render.
        struct Costly {
            clock: Arc<TestClock>,
            ns_per_frame: u64,
            level: f32,
            calls: u64,
            stall_at: Option<u64>,
        }

        impl Source for Costly {
            fn rate(&self) -> u32 {
                RATE
            }

            fn frames(&self) -> u64 {
                1 << 40
            }

            fn render(&mut self, out: &mut [f32]) -> usize {
                out.fill(self.level);
                let frames = out.len() / 2;
                self.calls += 1;
                let mut cost = frames as u64 * self.ns_per_frame;
                if self.stall_at == Some(self.calls) {
                    cost += SECOND_NS;
                }
                self.clock.0.fetch_add(cost, Ordering::Relaxed);
                frames
            }

            fn seek(&mut self, _frame: u64) {}
        }

        fn engine_with(budget: Option<RenderBudget>) -> (Engine, Arc<TestClock>) {
            let engine = Engine::new(EngineConfig { sample_rate: RATE });
            let clock = Arc::new(TestClock::default());
            engine.set_clock(clock.clone());
            engine.set_render_budget(budget);
            (engine, clock)
        }

        fn ns_per_frame(percent: u64) -> u64 {
            FRAME_NS * percent / 100
        }

        /// Starts a sound in `group`, after every sound started before,
        /// that takes `percent` of real time to render.
        fn costly(
            engine: &Engine,
            group: &Group,
            clock: &Arc<TestClock>,
            id: u64,
            percent: u64,
            level: f32,
        ) -> PlayerId {
            let source = Costly {
                clock: clock.clone(),
                ns_per_frame: ns_per_frame(percent),
                level,
                calls: 0,
                stall_at: None,
            };
            let id = play(engine, group, id, Box::new(source));
            let mut mixer = engine.lock();
            let order = mixer.next_start_order();
            mixer.slots.get_mut(&id).unwrap().voice.start_order = order;
            id
        }

        fn frame(engine: &Engine, id: PlayerId) -> u64 {
            engine.lock().slots[&id].voice.media_frame()
        }

        /// Renders `count` slices, one call each, as the audio thread does,
        /// and returns the left channel.
        fn slices(engine: &Engine, count: usize) -> Vec<f32> {
            let mut out = vec![0.0f32; SLICE * 2];
            let mut left = Vec::with_capacity(count * SLICE);
            for _ in 0..count {
                engine.render(&mut out);
                left.extend(out.iter().step_by(2));
            }
            left
        }

        /// Renders one slice and returns which of `voices` moved.
        fn moved(engine: &Engine, voices: &[PlayerId]) -> Vec<bool> {
            let before: Vec<u64> = voices.iter().map(|&v| frame(engine, v)).collect();
            slices(engine, 1);
            voices
                .iter()
                .zip(before)
                .map(|(&v, at)| frame(engine, v) != at)
                .collect()
        }

        /// The render time the voices took, from how far they played:
        /// each made what it played, and at most one block more.
        fn spent_ns(engine: &Engine, voices: &[PlayerId], percent: u64) -> u64 {
            voices.iter().map(|&v| frame(engine, v)).sum::<u64>() * ns_per_frame(percent)
        }

        #[test]
        fn the_budget_is_off_by_default() {
            let engine = Engine::new(EngineConfig { sample_rate: RATE });
            assert_eq!(engine.render_budget(), None);
            let clock = Arc::new(TestClock::default());
            engine.set_clock(clock.clone());
            let heavy = engine.create_group();
            let voices: Vec<_> = (0..8)
                .map(|i| costly(&engine, &heavy, &clock, i, 25, 0.01))
                .collect();
            slices(&engine, 100);
            for &v in &voices {
                assert_eq!(frame(&engine, v), 100 * SLICE as u64);
            }
            assert_eq!(engine.budget_waits(), 0);
            // No clock is read without a budget.
            assert_eq!(engine.lock().slots[&voices[0]].voice.frame_cost_ps, 0);
        }

        #[test]
        fn a_heavy_game_cannot_take_the_time_of_another() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let heavy = engine.create_group();
            let light = engine.create_group();
            // Eight sounds of a quarter of real time each: twice what the
            // audio thread has.
            let loud: Vec<_> = (0..8)
                .map(|i| costly(&engine, &heavy, &clock, i, 25, 0.01))
                .collect();
            let quiet: Vec<_> = (10..12)
                .map(|i| costly(&engine, &light, &clock, i, 1, 0.01))
                .collect();
            slices(&engine, RUN);
            let all = (RUN * SLICE) as u64;
            // The light game plays every frame, and so does the heavy
            // game's oldest sound.
            for &v in &quiet {
                assert_eq!(frame(&engine, v), all);
            }
            assert_eq!(frame(&engine, loud[0]), all);
            // The heavy game gets the half of real time the light one
            // leaves, and no more: the burst and a block of each sound.
            let spent = spent_ns(&engine, &loud, 25);
            let half = SECOND_NS;
            let slack = BURST_NS + 8 * BLOCK * ns_per_frame(25);
            assert!(spent <= half + slack, "{spent}");
            assert!(spent >= half * 9 / 10, "{spent}");
            assert!(engine.budget_waits() > 0);
        }

        #[test]
        fn a_sound_that_waits_keeps_its_place_and_fades_back_in() {
            // No share at all: once the burst is spent, only the oldest
            // sound of the game plays.
            let (engine, clock) = engine_with(Some(RenderBudget {
                share: 0.0,
                burst: Duration::from_millis(10),
            }));
            let group = engine.create_group();
            let first = costly(&engine, &group, &clock, 1, 25, 0.25);
            let second = costly(&engine, &group, &clock, 2, 25, 0.5);
            slices(&engine, 40);
            let held = frame(&engine, second);
            let played = frame(&engine, first);
            assert!(held > 0 && held < played, "{held} {played}");
            let waits = engine.budget_waits();
            let alone = slices(&engine, 50);
            // The second sound stood still and was silent; the first went on.
            assert_eq!(frame(&engine, second), held);
            assert_eq!(frame(&engine, first), played + 50 * SLICE as u64);
            assert_eq!(engine.budget_waits(), waits + 50);
            for &v in &alone {
                assert!((v - 0.25 * HEADROOM).abs() < 1e-6, "{v}");
            }
            // With room again it goes on from where it stood, fading in.
            engine.set_render_budget(Some(RenderBudget::DEFAULT));
            let back = slices(&engine, 1);
            assert_eq!(frame(&engine, second), held + SLICE as u64);
            assert!(back[0] < 0.26 * HEADROOM, "{}", back[0]);
            assert!(
                (back[SLICE - 1] - 0.75 * HEADROOM).abs() < 1e-3,
                "{}",
                back[SLICE - 1]
            );
        }

        #[test]
        fn a_sound_fades_out_before_it_waits() {
            let (engine, clock) = engine_with(Some(RenderBudget {
                share: 0.0,
                burst: Duration::from_millis(10),
            }));
            let group = engine.create_group();
            costly(&engine, &group, &clock, 1, 25, 0.25);
            let second = costly(&engine, &group, &clock, 2, 25, 0.5);
            let out = slices(&engine, 40);
            assert!(engine.lock().slots[&second].voice.waiting);
            // From the first frame on, no step bigger than the 3 ms ramp
            // makes: a cut to silence would drop by 0.35 at once.
            let step = 0.75 * HEADROOM / (RAMP_SECONDS * RATE as f32);
            for pair in out.windows(2) {
                assert!((pair[1] - pair[0]).abs() <= step * 1.01, "{pair:?}");
            }
        }

        #[test]
        fn a_player_started_later_waits_before_one_started_earlier() {
            // No share and no burst: only the oldest sound of a game plays.
            let (engine, _clock) = engine_with(Some(RenderBudget {
                share: 0.0,
                burst: Duration::ZERO,
            }));
            let group = engine.create_group();
            let wav = crate::wav::tests::pcm16_wav(RATE, 1, &vec![1000; 32_000]);
            let made_first = group.create_player(&wav, None).unwrap();
            let made_second = group.create_player(&wav, None).unwrap();
            made_second.start().unwrap();
            made_first.start().unwrap();
            slices(&engine, 10);
            assert!(made_second.media_time().unwrap() > 0);
            assert_eq!(made_first.media_time().unwrap(), 0);
        }

        #[test]
        fn the_sound_started_last_waits_first() {
            let (engine, clock) = engine_with(Some(RenderBudget {
                share: 0.0,
                burst: Duration::from_millis(10),
            }));
            let group = engine.create_group();
            // Made first, started second.
            let made_first = costly(&engine, &group, &clock, 1, 25, 0.1);
            let made_second = costly(&engine, &group, &clock, 2, 25, 0.1);
            {
                let mut mixer = engine.lock();
                let order = mixer.next_start_order();
                mixer.slots.get_mut(&made_first).unwrap().voice.start_order = order;
            }
            slices(&engine, 100);
            assert_eq!(frame(&engine, made_second), 100 * SLICE as u64);
            assert!(frame(&engine, made_first) < 20 * SLICE as u64);
        }

        #[test]
        fn games_within_the_budget_sound_as_they_do_without_it() {
            let run = |budget| {
                let (engine, clock) = engine_with(budget);
                let mut keep = Vec::new();
                for g in 0..8u64 {
                    let group = engine.create_group();
                    costly(&engine, &group, &clock, g * 10, 1, 0.01 * g as f32);
                    costly(&engine, &group, &clock, g * 10 + 1, 1, -0.02);
                    // A real synthesizer too, playing a chord.
                    let track = [
                        0x00, 0x90, 60, 100, 0x00, 0x90, 64, 100, 0x00, 0x90, 67, 100, //
                        0x83, 0x60, 0x80, 60, 0, 0x00, 0xFF, 0x2F, 0x00,
                    ];
                    let file = crate::midi::smf::tests::smf(0, 96, &[&track]);
                    let midi =
                        crate::media::open(&file, crate::format::Format::Midi, RATE, None).unwrap();
                    play(&engine, &group, g * 10 + 2, midi);
                    keep.push(group);
                }
                let out = slices(&engine, RUN);
                (out, engine.budget_waits(), keep)
            };
            let (with, waits, _a) = run(Some(RenderBudget::DEFAULT));
            let (without, _, _b) = run(None);
            assert_eq!(waits, 0);
            assert!(with.iter().any(|v| v.abs() > 0.01));
            assert_eq!(with, without);
        }

        #[test]
        fn a_render_the_system_stalls_costs_no_more_than_its_slice() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let group = engine.create_group();
            let stalled = Costly {
                clock: clock.clone(),
                ns_per_frame: FRAME_NS / 100,
                level: 0.1,
                calls: 0,
                stall_at: Some(10),
            };
            let first = play(&engine, &group, 1, Box::new(stalled));
            let second = costly(&engine, &group, &clock, 2, 1, 0.1);
            let mut dearest = 0;
            for _ in 0..100 {
                slices(&engine, 1);
                dearest = dearest.max(engine.lock().slots[&first].voice.frame_cost_ps);
            }
            assert!(clock.now_ns() > SECOND_NS);
            assert_eq!(engine.budget_waits(), 0);
            assert_eq!(frame(&engine, second), 100 * SLICE as u64);
            // Its estimate never says a frame costs more than it lasts.
            assert!(dearest <= FRAME_NS * 1000, "{dearest}");
        }

        #[test]
        fn two_heavy_games_share_the_budget_and_a_light_one_plays_on() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let a = engine.create_group();
            let b = engine.create_group();
            let c = engine.create_group();
            let in_a: Vec<_> = (0..8)
                .map(|i| costly(&engine, &a, &clock, i, 25, 0.01))
                .collect();
            let in_b: Vec<_> = (10..18)
                .map(|i| costly(&engine, &b, &clock, i, 25, 0.01))
                .collect();
            let in_c: Vec<_> = (20..22)
                .map(|i| costly(&engine, &c, &clock, i, 1, 0.01))
                .collect();
            slices(&engine, RUN);
            for &v in &in_c {
                assert_eq!(frame(&engine, v), (RUN * SLICE) as u64);
            }
            let (spent_a, spent_b) = (spent_ns(&engine, &in_a, 25), spent_ns(&engine, &in_b, 25));
            // A quarter of the 2 s each, give or take the burst and a
            // block of each sound.
            let slack = BURST_NS + 8 * BLOCK * ns_per_frame(25);
            for spent in [spent_a, spent_b] {
                assert!(spent <= SECOND_NS / 2 + slack, "{spent}");
                assert!(spent >= SECOND_NS * 9 / 20, "{spent}");
            }
        }

        #[test]
        fn a_game_just_over_its_part_does_not_chop_its_sound() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let group = engine.create_group();
            costly(&engine, &group, &clock, 1, 30, 0.1);
            let second = costly(&engine, &group, &clock, 2, 30, 0.1);
            let mut changes = 0;
            let mut waiting = false;
            for _ in 0..RUN {
                slices(&engine, 1);
                let now = engine.lock().slots[&second].voice.waiting;
                changes += usize::from(now != waiting);
                waiting = now;
            }
            // It plays and waits in stretches of many slices, not one
            // slice in, one slice out.
            assert!(engine.budget_waits() > 0);
            assert!(changes <= RUN / 10, "{changes} changes");
        }

        #[test]
        fn a_game_that_stops_and_starts_gains_no_time() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let heavy = engine.create_group();
            let voices: Vec<_> = (0..8)
                .map(|i| costly(&engine, &heavy, &clock, i, 50, 0.01))
                .collect();
            for i in 0..RUN {
                let state = if i % 2 == 0 {
                    PlayerState::Started
                } else {
                    PlayerState::Prefetched
                };
                {
                    let mut mixer = engine.lock();
                    for v in &voices {
                        mixer.slots.get_mut(v).unwrap().voice.state = state;
                    }
                }
                slices(&engine, 1);
            }
            // Playing half the time, it gets half of real time for that
            // half: 0.5 s of the 2 s, give or take the burst and a block.
            let spent = spent_ns(&engine, &voices, 50);
            let slack = BURST_NS + 8 * BLOCK * ns_per_frame(50);
            assert!(spent <= SECOND_NS / 2 + slack, "{spent}");
        }

        #[test]
        fn a_game_over_its_part_keeps_all_but_its_newest_sounds_playing() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let group = engine.create_group();
            // Sixteen sounds of 6% each, against a part of 50%: eight fit.
            let voices: Vec<_> = (0..16)
                .map(|i| costly(&engine, &group, &clock, i, 6, 0.01))
                .collect();
            slices(&engine, 100);
            let mut counts = Vec::new();
            for _ in 0..RUN {
                let playing = moved(&engine, &voices);
                // The ones that play are the oldest.
                let count = playing.iter().filter(|&&p| p).count();
                assert!(playing[..count].iter().all(|&p| p), "{playing:?}");
                counts.push(count);
            }
            // The eight that fit play, and a ninth part of the time on the
            // game's credit: never down to the oldest alone, as when the
            // whole game is held back at once, and one sound in or out at
            // a time.
            assert!(counts.iter().all(|&c| (8..=9).contains(&c)), "{counts:?}");
            assert!(counts.contains(&9), "{counts:?}");
            assert!(
                counts.windows(2).all(|w| w[0].abs_diff(w[1]) <= 1),
                "{counts:?}"
            );
        }

        #[test]
        fn games_started_together_stay_near_their_parts_from_the_first_slice() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let games: Vec<_> = (0..4).map(|_| engine.create_group()).collect();
            for (g, group) in games.iter().enumerate() {
                for i in 0..16 {
                    costly(&engine, group, &clock, g as u64 * 100 + i, 5, 0.001);
                }
            }
            // Two slices at a time, so each sound reads one block of its
            // source in each (two in the first, as it fills its first).
            let mut out = vec![0.0f32; 2 * SLICE * 2];
            let allowance = 2 * SLICE as u64 * FRAME_NS / 2;
            let voice = 2 * BLOCK * ns_per_frame(5);
            for pass in 0..100 {
                let before = clock.now_ns();
                engine.render(&mut out);
                let spent = clock.now_ns() - before;
                // The share, and for each game its oldest sound and one
                // more: never the 64 sounds at once.
                let bound = allowance + 4 * 2 * voice;
                assert!(spent <= bound, "pass {pass}: {spent} of {bound}");
            }
        }

        #[test]
        fn sounds_never_rendered_cannot_all_start_at_once() {
            let (engine, clock) = engine_with(Some(RenderBudget::DEFAULT));
            let group = engine.create_group();
            let cheap = costly(&engine, &group, &clock, 0, 1, 0.01);
            slices(&engine, 50);
            // With its credit full, the game starts 31 sounds that cost a
            // quarter of real time each and were never measured.
            let fresh: Vec<_> = (1..32)
                .map(|i| costly(&engine, &group, &clock, i, 25, 0.01))
                .collect();
            let before = clock.now_ns();
            let playing = moved(&engine, &fresh);
            let spent = clock.now_ns() - before;
            // One fits in the part, counted at the dearest sound measured
            // before it, and one more plays past it on the game's credit.
            // Each read one block of its source; the cheap one at most one.
            let started = playing.iter().filter(|&&p| p).count();
            assert_eq!(started, 2, "{playing:?}");
            let bound = 2 * BLOCK * ns_per_frame(25) + BLOCK * ns_per_frame(1);
            assert!(spent <= bound, "{spent} of {bound}");
            assert!(frame(&engine, cheap) > 0);
        }

        #[test]
        fn a_sound_started_again_keeps_its_cost() {
            // A part of 1.1 ms a slice: a sound of 1 ms fits beside a cheap
            // one while it plays, but not when it must be let in again.
            let (engine, clock) = engine_with(Some(RenderBudget {
                share: 0.275,
                burst: Duration::from_millis(20),
            }));
            let group = engine.create_group();
            costly(&engine, &group, &clock, 0, 1, 0.01);
            let heavy = costly(&engine, &group, &clock, 1, 25, 0.01);
            slices(&engine, 100);
            let cost = engine.lock().slots[&heavy].voice.frame_cost_ps;
            assert!(cost > 0);
            {
                // Stopped and started again, as `Player` does, with no
                // credit left to come back on.
                let mut mixer = engine.lock();
                let order = mixer.next_start_order();
                let voice = &mut mixer.slots.get_mut(&heavy).unwrap().voice;
                voice.state = PlayerState::Prefetched;
                voice.start();
                voice.start_order = order;
                assert_eq!(voice.frame_cost_ps, cost);
                mixer.credits.insert(group.id(), 0);
            }
            let at = frame(&engine, heavy);
            slices(&engine, 1);
            // Counted at its own cost, it waits. Counted as never measured,
            // at the cheap sound's cost, it would have played.
            assert_eq!(frame(&engine, heavy), at);
        }
    }
}
