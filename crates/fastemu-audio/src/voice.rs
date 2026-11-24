//! One player inside the mixer: its source, the resampler that brings the
//! source to the output rate, and the bookkeeping for loops, media time
//! and the end of media.
//!
//! The source is read ahead a block at a time, but loop and end events
//! fire when the playhead (the frame being heard) reaches them, so media
//! time and events agree to the frame.

use std::collections::VecDeque;

use crate::gain::level_gain;
use crate::source::Source;

/// Source frames read at a time.
const BLOCK: usize = 256;

/// The MMAPI player states a voice can be in. Unrealized players live in
/// the MIDP layer; closed ones are gone from the mixer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerState {
    /// Created and checked, holding no playback resources.
    Realized,
    /// Ready to start at once.
    Prefetched,
    /// Playing.
    Started,
    /// Closed; every call but `state` fails.
    Closed,
}

impl PlayerState {
    /// The value of the matching `javax.microedition.media.Player`
    /// constant.
    pub fn mmapi_code(self) -> i32 {
        match self {
            Self::Realized => 200,
            Self::Prefetched => 300,
            Self::Started => 400,
            Self::Closed => 0,
        }
    }
}

/// How many times a run starts over from the beginning for a loop count:
/// one less than the count, or -1 (forever) for -1.
fn wraps_for(loop_count: i32) -> i32 {
    if loop_count < 0 { -1 } else { loop_count - 1 }
}

/// Something the playhead reached while rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Reached {
    /// The end of the media, with more loops to go: playback carries on
    /// from the start.
    Loop,
    /// The end of the media, with no loops left: the voice stopped. The
    /// source's error, if it stopped early.
    End { error: Option<String> },
}

pub(crate) struct Voice {
    pub source: Box<dyn Source>,
    pub state: PlayerState,
    /// The MMAPI volume level, 0 to 100.
    pub level: u8,
    pub muted: bool,
    /// MMAPI's loop count: -1 forever, else how many times to play.
    pub loop_count: i32,
    /// Fire-and-forget voices (`Manager.playTone`) have no player handle,
    /// send no events and leave the mixer when they end.
    pub one_shot: bool,
    /// Whether the last run reached the end; the next start begins again
    /// from the start.
    pub at_end: bool,
    /// The gain heard now; it slides toward the target so changes do not
    /// click.
    pub gain: f32,
    playhead: Playhead,
}

/// The resampler and the position bookkeeping.
struct Playhead {
    /// Four source frames around the playhead: one before, the playhead
    /// frame, two after.
    window: [[f32; 2]; 4],
    /// Where between `window[1]` and `window[2]` the next output frame
    /// falls.
    fraction: f64,
    primed: bool,
    block: Vec<f32>,
    block_len: usize,
    block_pos: usize,
    /// Every frame read from the source since the last reposition is
    /// numbered; `head` is the number of `window[1]`.
    head: u64,
    /// Frames read from the source since the last reposition.
    read: u64,
    /// The media frame is `head + offset`.
    offset: i64,
    /// Loops left to start in this run: -1 forever.
    wraps: i32,
    /// The numbers at which a new loop starts, read but not yet heard.
    loop_starts: VecDeque<u64>,
    /// The number after the last frame, once the source has ended.
    end_at: Option<u64>,
    /// Frames read since the last wrap: a wrap with none between means
    /// the source has nothing to play.
    since_wrap: u64,
}

impl Playhead {
    fn new(frame: u64, wraps: i32) -> Self {
        Self {
            window: [[0.0; 2]; 4],
            fraction: 0.0,
            primed: false,
            block: vec![0.0; BLOCK * 2],
            block_len: 0,
            block_pos: 0,
            head: 0,
            read: 0,
            offset: frame as i64,
            wraps,
            loop_starts: VecDeque::new(),
            end_at: None,
            since_wrap: 1,
        }
    }

    /// Loops left counting the ones read ahead but not yet heard.
    fn wraps_at_head(&self) -> i32 {
        if self.wraps < 0 {
            -1
        } else {
            self.wraps + self.loop_starts.len() as i32
        }
    }

    fn refill(&mut self, source: &mut dyn Source) {
        let mut filled = 0;
        while filled < BLOCK && self.end_at.is_none() {
            let count = source.render(&mut self.block[filled * 2..]);
            filled += count;
            self.read += count as u64;
            self.since_wrap += count as u64;
            if filled < BLOCK {
                if self.wraps != 0 && self.since_wrap > 0 && source.frames() > 0 {
                    if self.wraps > 0 {
                        self.wraps -= 1;
                    }
                    source.seek(0);
                    self.loop_starts.push_back(self.read);
                    self.since_wrap = 0;
                } else {
                    self.end_at = Some(self.read);
                }
            }
        }
        self.block_len = filled;
        self.block_pos = 0;
    }

    fn pull(&mut self, source: &mut dyn Source) -> [f32; 2] {
        if self.block_pos == self.block_len {
            if self.end_at.is_some() {
                return [0.0; 2];
            }
            self.refill(source);
            if self.block_len == 0 {
                return [0.0; 2];
            }
        }
        let at = self.block_pos * 2;
        self.block_pos += 1;
        [self.block[at], self.block[at + 1]]
    }

    fn advance(&mut self, source: &mut dyn Source) {
        let next = self.pull(source);
        self.window = [self.window[1], self.window[2], self.window[3], next];
        self.head += 1;
    }

    fn prime(&mut self, source: &mut dyn Source) {
        let first = self.pull(source);
        let second = self.pull(source);
        let third = self.pull(source);
        self.window = [[0.0; 2], first, second, third];
        self.primed = true;
    }

    /// Catmull-Rom interpolation at `fraction` between the playhead frame
    /// and the next. At a fraction of zero it returns the playhead frame
    /// exactly, so same-rate playback passes samples through untouched.
    fn interpolate(&self) -> [f32; 2] {
        let t = self.fraction as f32;
        let [a, b, c, d] = self.window;
        let mut out = [0.0; 2];
        for (side, value) in out.iter_mut().enumerate() {
            let (x0, x1, x2, x3) = (a[side], b[side], c[side], d[side]);
            let c1 = 0.5 * (x2 - x0);
            let c2 = x0 - 2.5 * x1 + 2.0 * x2 - 0.5 * x3;
            let c3 = 0.5 * (x3 - x0) + 1.5 * (x1 - x2);
            *value = ((c3 * t + c2) * t + c1) * t + x1;
        }
        out
    }

    /// Takes the loop starts and the end the playhead has reached.
    fn check(&mut self) -> Option<Reached> {
        if let Some(&start) = self.loop_starts.front()
            && self.head >= start
        {
            self.loop_starts.pop_front();
            self.offset = -(start as i64);
            return Some(Reached::Loop);
        }
        match self.end_at {
            Some(end) if self.head >= end => Some(Reached::End { error: None }),
            _ => None,
        }
    }
}

impl Voice {
    pub fn new(source: Box<dyn Source>, one_shot: bool) -> Self {
        Self {
            source,
            state: PlayerState::Realized,
            level: 100,
            muted: false,
            loop_count: 1,
            one_shot,
            at_end: false,
            gain: 0.0,
            playhead: Playhead::new(0, 0),
        }
    }

    /// How far into the media the listener is, in source frames: the
    /// frame the next output frame starts from.
    pub fn media_frame(&self) -> u64 {
        let frames = self.source.frames();
        if self.at_end {
            return frames;
        }
        let playhead = &self.playhead;
        let frame = playhead.head as i64 + playhead.fraction.floor() as i64 + playhead.offset;
        (frame.max(0) as u64).min(frames)
    }

    fn frames_to_us(&self, frames: u64) -> i64 {
        (u128::from(frames) * 1_000_000 / u128::from(self.source.rate().max(1))) as i64
    }

    pub fn media_time_us(&self) -> i64 {
        self.frames_to_us(self.media_frame())
    }

    pub fn duration_us(&self) -> i64 {
        self.frames_to_us(self.source.frames())
    }

    /// The gain to slide toward, before the group's gain.
    pub fn own_gain(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            level_gain(self.level)
        }
    }

    /// Loops left in the current run, counting from the playhead.
    fn wraps(&self) -> i32 {
        self.playhead.wraps_at_head()
    }

    /// Moves the playhead to `frame` with `wraps` loops left.
    fn reposition(&mut self, frame: u64, wraps: i32) {
        let frame = frame.min(self.source.frames());
        self.source.seek(frame);
        self.playhead = Playhead::new(frame, wraps);
        self.at_end = false;
    }

    /// Replaces the media, as when a tone player gets a new sequence; the
    /// playhead goes back to the start.
    pub fn set_source(&mut self, source: Box<dyn Source>) {
        self.source = source;
        self.reposition(0, wraps_for(self.loop_count));
    }

    /// Jumps to a media time, as `setMediaTime`; returns the time set.
    pub fn seek_us(&mut self, us: i64) -> i64 {
        let rate = u128::from(self.source.rate());
        let frame = (u128::from(us.max(0) as u64) * rate / 1_000_000) as u64;
        let wraps = if self.at_end {
            wraps_for(self.loop_count)
        } else {
            self.wraps()
        };
        self.reposition(frame, wraps);
        self.media_time_us()
    }

    /// Sets the loop count for the next run, as `setLoopCount`.
    pub fn set_loop_count(&mut self, count: i32) {
        self.loop_count = count;
        let frame = self.media_frame();
        let at_end = self.at_end;
        self.reposition(frame, wraps_for(count));
        self.at_end = at_end;
    }

    /// Starts playing: from the start again if the last run reached the
    /// end. The gain starts from silence.
    pub fn start(&mut self) {
        if self.at_end {
            self.reposition(0, wraps_for(self.loop_count));
        }
        self.state = PlayerState::Started;
        self.gain = 0.0;
    }

    /// Renders and adds `out.len() / 2` stereo frames at the output rate.
    /// `step` is source frames per output frame. `quiet` renders without
    /// changing state, for the fade-out after a stop; reaching the end
    /// then just gives silence.
    pub fn render(
        &mut self,
        out: &mut [f32],
        step: f64,
        target: f32,
        slew: f32,
        quiet: bool,
        mut on: impl FnMut(Reached, i64),
    ) {
        if !self.playhead.primed {
            self.playhead.prime(self.source.as_mut());
            if let Some(reached) = self.playhead.check()
                && self.reached(reached, quiet, &mut on)
            {
                return;
            }
        }
        for frame in out.as_chunks_mut::<2>().0 {
            while self.playhead.fraction >= 1.0 {
                self.playhead.advance(self.source.as_mut());
                self.playhead.fraction -= 1.0;
                if let Some(reached) = self.playhead.check()
                    && self.reached(reached, quiet, &mut on)
                {
                    return;
                }
            }
            let [left, right] = self.playhead.interpolate();
            self.gain = if self.gain < target {
                (self.gain + slew).min(target)
            } else {
                (self.gain - slew).max(target)
            };
            frame[0] += left * self.gain;
            frame[1] += right * self.gain;
            self.playhead.fraction += step;
        }
    }

    /// Handles a loop or the end; returns whether rendering stops.
    fn reached(
        &mut self,
        reached: Reached,
        quiet: bool,
        on: &mut impl FnMut(Reached, i64),
    ) -> bool {
        let at_end = matches!(reached, Reached::End { .. });
        if quiet {
            return at_end;
        }
        match reached {
            Reached::Loop => on(Reached::Loop, self.duration_us()),
            Reached::End { .. } => {
                let error = self.source.take_error();
                self.at_end = true;
                self.state = PlayerState::Prefetched;
                self.playhead.wraps = wraps_for(self.loop_count);
                on(Reached::End { error }, self.duration_us());
            }
        }
        at_end
    }

    /// Renders a short fade-out from where the voice is, then puts the
    /// playhead back, so stopping, pausing and jumping do not click and
    /// media time stays where it was.
    pub fn fade_out(&mut self, frames: usize, step: f64, slew: f32) -> Vec<f32> {
        let mut tail = vec![0.0; frames * 2];
        if self.gain <= 0.0 {
            return tail;
        }
        let (frame, wraps, at_end) = (self.media_frame(), self.wraps(), self.at_end);
        let target = self.gain;
        self.render(&mut tail, step, target, slew, true, |_, _| {});
        let len = frames.max(1) as f32;
        for (i, pair) in tail.as_chunks_mut::<2>().0.iter_mut().enumerate() {
            let fade = 1.0 - i as f32 / len;
            pair[0] *= fade;
            pair[1] *= fade;
        }
        self.reposition(frame, wraps);
        self.at_end = at_end;
        tail
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::pcm::{Pcm, PcmSource};

    /// A voice over a counting ramp: sample n has the value n / 32768.
    fn ramp(frames: usize, rate: u32) -> Voice {
        let samples = (0..frames).map(|i| i as i16).collect();
        let pcm = Arc::new(Pcm {
            rate,
            channels: 1,
            samples,
        });
        let mut voice = Voice::new(Box::new(PcmSource::new(pcm)), false);
        voice.start();
        voice.gain = 1.0;
        voice
    }

    fn play(voice: &mut Voice, frames: usize, step: f64) -> (Vec<f32>, Vec<(Reached, i64)>) {
        let mut out = vec![0.0; frames * 2];
        let mut seen = Vec::new();
        voice.render(&mut out, step, 1.0, 1.0, false, |r, t| seen.push((r, t)));
        (out.iter().step_by(2).map(|v| v * 32768.0).collect(), seen)
    }

    #[test]
    fn same_rate_passes_samples_through() {
        let mut voice = ramp(100, 8000);
        let (out, seen) = play(&mut voice, 50, 1.0);
        let want: Vec<f32> = (0..50).map(|i| i as f32).collect();
        assert_eq!(out, want);
        assert!(seen.is_empty());
        assert_eq!(voice.media_frame(), 50);
    }

    #[test]
    fn reaching_the_end_stops_at_the_last_frame() {
        let mut voice = ramp(100, 8000);
        let (out, seen) = play(&mut voice, 150, 1.0);
        assert_eq!(out[99], 99.0);
        assert!(out[100..].iter().all(|&v| v == 0.0));
        assert_eq!(seen, vec![(Reached::End { error: None }, 12_500)]);
        assert_eq!(voice.state, PlayerState::Prefetched);
        assert!(voice.at_end);
        assert_eq!(voice.media_time_us(), voice.duration_us());
    }

    #[test]
    fn loops_play_the_media_again() {
        let mut voice = ramp(100, 8000);
        voice.set_loop_count(3);
        voice.start();
        voice.gain = 1.0;
        let (out, seen) = play(&mut voice, 350, 1.0);
        assert_eq!(out[100], 0.0);
        assert_eq!(out[150], 50.0);
        assert_eq!(out[299], 99.0);
        assert!(out[300..].iter().all(|&v| v == 0.0));
        assert_eq!(seen.len(), 3);
        assert_eq!(seen[0].0, Reached::Loop);
        assert_eq!(seen[2].0, Reached::End { error: None });
    }

    #[test]
    fn media_time_follows_the_playhead_through_loops() {
        let mut voice = ramp(1000, 1000);
        voice.set_loop_count(-1);
        voice.start();
        voice.gain = 1.0;
        play(&mut voice, 1250, 1.0);
        // 1250 frames in: the second time through, 250 frames along.
        assert_eq!(voice.media_frame(), 250);
        assert_eq!(voice.media_time_us(), 250_000);
        assert!(!voice.at_end);
    }

    #[test]
    fn upsampling_interpolates_between_frames() {
        let mut voice = ramp(100, 4000);
        let (out, _) = play(&mut voice, 40, 0.5);
        // On a straight line, Catmull-Rom gives the points in between.
        for (i, &v) in out.iter().enumerate().skip(4) {
            assert!((v - i as f32 * 0.5).abs() < 1e-3, "{i}: {v}");
        }
        assert_eq!(voice.media_frame(), 20);
    }

    #[test]
    fn seeking_moves_media_time_and_keeps_loops() {
        let mut voice = ramp(1000, 1000);
        voice.set_loop_count(2);
        assert_eq!(voice.seek_us(500_000), 500_000);
        voice.start();
        voice.gain = 1.0;
        let (out, seen) = play(&mut voice, 10, 1.0);
        assert_eq!(out[0], 500.0);
        assert!(seen.is_empty());
        assert_eq!(voice.seek_us(5_000_000), 1_000_000);
        assert_eq!(voice.seek_us(-3), 0);
    }

    #[test]
    fn a_fade_out_leaves_the_position_alone() {
        let mut voice = ramp(1000, 1000);
        play(&mut voice, 100, 1.0);
        let before = voice.media_frame();
        let tail = voice.fade_out(20, 1.0, 1.0);
        assert_eq!(tail.len(), 40);
        assert!(tail[0] > 0.0 && tail[38].abs() < tail[0]);
        assert_eq!(voice.media_frame(), before);
        let (out, _) = play(&mut voice, 1, 1.0);
        assert_eq!(out[0], before as f32);
    }

    #[test]
    fn an_empty_source_ends_at_once_even_when_looping() {
        let mut voice = ramp(0, 1000);
        voice.set_loop_count(-1);
        voice.start();
        let (_, seen) = play(&mut voice, 10, 1.0);
        assert_eq!(seen, vec![(Reached::End { error: None }, 0)]);
    }

    #[test]
    fn starting_after_the_end_begins_again() {
        let mut voice = ramp(10, 1000);
        play(&mut voice, 20, 1.0);
        assert!(voice.at_end);
        voice.start();
        assert_eq!(voice.media_frame(), 0);
        let (out, _) = play(&mut voice, 5, 1.0);
        assert_eq!(out, vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    }
}
