//! Tones: MMAPI tone sequences, `Manager.playTone`, and the Nokia sound
//! API's ringing tones and beeps all become a list of [`ToneEvent`]s that
//! [`ToneSource`] plays as a softened square wave, the way a phone's tone
//! generator sounds.

pub(crate) mod sequence;

use std::sync::Arc;

use crate::gain::level_gain;
use crate::source::Source;

/// One note or rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ToneEvent {
    /// The pitch in hertz; zero is a rest.
    pub frequency: f32,
    pub duration_us: u64,
    /// The MMAPI volume level, 0 to 100.
    pub volume: u8,
}

impl ToneEvent {
    /// A note given as a MIDI note number, as MMAPI tones are.
    pub fn note(note: u8, duration_us: u64, volume: u8) -> Self {
        Self {
            frequency: note_frequency(note),
            duration_us,
            volume,
        }
    }

    pub fn rest(duration_us: u64) -> Self {
        Self {
            frequency: 0.0,
            duration_us,
            volume: 0,
        }
    }
}

/// The frequency of a MIDI note number: note 69 is A at 440 Hz, and
/// JSR-135 defines its tone notes the same way (60 is middle C).
pub(crate) fn note_frequency(note: u8) -> f32 {
    440.0 * 2f32.powf((f32::from(note) - 69.0) / 12.0)
}

/// How loud a full-volume tone is. A square wave at full scale is far
/// louder than other sounds at the same level.
const TONE_AMPLITUDE: f32 = 0.3;
/// Each note fades in and out over this long so notes do not click.
const EDGE_SECONDS: f32 = 0.002;

/// Plays a list of tones at a fixed sample rate.
pub(crate) struct ToneSource {
    events: Arc<[ToneEvent]>,
    /// The first frame of each event, and the total length at the end.
    starts: Vec<u64>,
    rate: u32,
    pos: u64,
    /// The event `pos` falls in.
    index: usize,
    phase: f32,
}

impl ToneSource {
    pub fn new(events: Arc<[ToneEvent]>, rate: u32) -> Self {
        let mut starts = Vec::with_capacity(events.len() + 1);
        let mut elapsed_us = 0u64;
        starts.push(0);
        for event in events.iter() {
            elapsed_us = elapsed_us.saturating_add(event.duration_us);
            starts.push(us_to_frames(elapsed_us, rate));
        }
        Self {
            events,
            starts,
            rate,
            pos: 0,
            index: 0,
            phase: 0.0,
        }
    }

    fn total(&self) -> u64 {
        self.starts.last().copied().unwrap_or(0)
    }

    fn sample(&mut self, event: ToneEvent, offset: u64, length: u64) -> f32 {
        let nyquist = self.rate as f32 / 2.0;
        if event.frequency <= 0.0 || event.frequency >= nyquist || event.volume == 0 {
            return 0.0;
        }
        let dt = event.frequency / self.rate as f32;
        let mut value = if self.phase < 0.5 { 1.0 } else { -1.0 };
        value += poly_blep(self.phase, dt);
        value -= poly_blep((self.phase + 0.5).fract(), dt);
        self.phase = (self.phase + dt).fract();
        let edge = EDGE_SECONDS * self.rate as f32;
        let from_start = offset as f32 / edge;
        let to_end = (length - offset) as f32 / edge;
        let envelope = from_start.min(to_end).min(1.0);
        value * envelope * TONE_AMPLITUDE * level_gain(event.volume)
    }
}

/// Converts microseconds to frames, rounding to the nearest.
fn us_to_frames(us: u64, rate: u32) -> u64 {
    ((u128::from(us) * u128::from(rate) + 500_000) / 1_000_000) as u64
}

/// The polynomial band-limited step that takes the alias-producing
/// corners off a naive square wave.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

impl Source for ToneSource {
    fn rate(&self) -> u32 {
        self.rate
    }

    fn frames(&self) -> u64 {
        self.total()
    }

    fn render(&mut self, out: &mut [f32]) -> usize {
        let mut written = 0;
        for frame in out.as_chunks_mut::<2>().0 {
            while self.index < self.events.len() && self.pos >= self.starts[self.index + 1] {
                self.index += 1;
                self.phase = 0.0;
            }
            if self.index >= self.events.len() {
                break;
            }
            let start = self.starts[self.index];
            let length = self.starts[self.index + 1] - start;
            let value = self.sample(self.events[self.index], self.pos - start, length);
            *frame = [value; 2];
            self.pos += 1;
            written += 1;
        }
        written
    }

    fn seek(&mut self, frame: u64) {
        self.pos = frame.min(self.total());
        // The event whose span holds `pos`: the last start at or before it.
        self.index = self.starts[1..].partition_point(|&end| end <= self.pos);
        self.phase = 0.0;
    }

    fn memory_bytes(&self) -> usize {
        self.events.len() * size_of::<ToneEvent>() + self.starts.len() * size_of::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_all(source: &mut ToneSource) -> Vec<f32> {
        let mut out = vec![0.0; 2 * 1024];
        let mut all = Vec::new();
        loop {
            let n = source.render(&mut out);
            all.extend_from_slice(&out[..n * 2]);
            if n < 1024 {
                return all;
            }
        }
    }

    #[test]
    fn notes_follow_midi_numbering() {
        assert!((note_frequency(69) - 440.0).abs() < 1e-3);
        assert!((note_frequency(60) - 261.6256).abs() < 1e-2);
        assert!((note_frequency(81) - 880.0).abs() < 1e-2);
    }

    #[test]
    fn the_events_and_their_starts_are_the_memory() {
        let notes: Arc<[ToneEvent]> = vec![ToneEvent::note(69, 1000, 100); 10].into();
        let source = ToneSource::new(notes, 8000);
        // Ten events and eleven starts of 8 bytes.
        assert_eq!(source.memory_bytes(), 10 * size_of::<ToneEvent>() + 11 * 8);
        assert_eq!(ToneSource::new(Arc::from([]), 8000).memory_bytes(), 8);
    }

    #[test]
    fn plays_each_tone_for_its_duration() {
        let events: Arc<[ToneEvent]> = vec![
            ToneEvent::note(69, 100_000, 100),
            ToneEvent::rest(50_000),
            ToneEvent::note(81, 100_000, 100),
        ]
        .into();
        let mut source = ToneSource::new(events, 8000);
        assert_eq!(source.frames(), 2000);
        let all = render_all(&mut source);
        assert_eq!(all.len(), 4000);
        // The rest is silent.
        assert!(all[2 * 850..2 * 1150].iter().all(|&v| v == 0.0));
        // 440 Hz for 0.1 s has 44 cycles: count rising zero crossings.
        let left: Vec<f32> = all.iter().step_by(2).copied().collect();
        let rising = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        assert!((43..=45).contains(&rising(&left[..800])));
        assert!((87..=89).contains(&rising(&left[1200..])));
        assert!(left.iter().all(|v| v.abs() <= TONE_AMPLITUDE * 1.1));
    }

    #[test]
    fn notes_fade_in_and_out() {
        let events: Arc<[ToneEvent]> = vec![ToneEvent::note(69, 10_000, 100)].into();
        let mut source = ToneSource::new(events, 48000);
        let all = render_all(&mut source);
        assert!(all[0].abs() < 0.01);
        assert!(all[all.len() - 1].abs() < 0.01);
    }

    #[test]
    fn volume_scales_the_tone() {
        let peak = |volume| {
            let events: Arc<[ToneEvent]> = vec![ToneEvent::note(69, 20_000, volume)].into();
            render_all(&mut ToneSource::new(events, 8000))
                .iter()
                .fold(0.0f32, |m, v| m.max(v.abs()))
        };
        assert!(peak(50) < peak(100) * 0.4);
        assert_eq!(peak(0), 0.0);
    }

    #[test]
    fn seeks_into_the_middle_of_a_tone() {
        let events: Arc<[ToneEvent]> =
            vec![ToneEvent::rest(100_000), ToneEvent::note(69, 100_000, 100)].into();
        let mut source = ToneSource::new(events, 8000);
        source.seek(1200);
        assert_eq!(source.index, 1);
        let all = render_all(&mut source);
        assert_eq!(all.len(), 2 * 400);
        source.seek(400);
        assert_eq!(source.index, 0);
        source.seek(10_000);
        assert_eq!(render_all(&mut source).len(), 0);
    }
}
