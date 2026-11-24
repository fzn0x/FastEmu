//! Plays a parsed MIDI file through a synthesizer, frame by frame.

use std::sync::Arc;

use crate::midi::Synth;
use crate::midi::smf::Sequence;
use crate::source::Source;

/// Frames rendered per synthesizer call.
const CHUNK: usize = 256;

/// A MIDI file as a [`Source`]: messages go to the synthesizer at the
/// frame their time falls on.
pub(crate) struct MidiSource {
    sequence: Arc<Sequence>,
    /// The frame of each event at `rate`.
    frames: Vec<u64>,
    total: u64,
    synth: Box<dyn Synth>,
    rate: u32,
    pos: u64,
    /// The next event to send.
    next: usize,
    left: Vec<f32>,
    right: Vec<f32>,
}

fn us_to_frames(us: u64, rate: u32) -> u64 {
    ((u128::from(us) * u128::from(rate) + 500_000) / 1_000_000) as u64
}

impl MidiSource {
    pub fn new(sequence: Arc<Sequence>, synth: Box<dyn Synth>, rate: u32) -> Self {
        let frames = sequence
            .events
            .iter()
            .map(|e| us_to_frames(e.time_us, rate))
            .collect();
        let total = us_to_frames(sequence.duration_us, rate);
        Self {
            sequence,
            frames,
            total,
            synth,
            rate,
            pos: 0,
            next: 0,
            left: vec![0.0; CHUNK],
            right: vec![0.0; CHUNK],
        }
    }

    fn send_due(&mut self) {
        while self.next < self.frames.len() && self.frames[self.next] <= self.pos {
            let event = self.sequence.events[self.next];
            self.synth.message(event.status, event.data1, event.data2);
            self.next += 1;
        }
    }
}

impl Source for MidiSource {
    fn rate(&self) -> u32 {
        self.rate
    }

    fn frames(&self) -> u64 {
        self.total
    }

    fn render(&mut self, out: &mut [f32]) -> usize {
        let wanted =
            (out.len() / 2).min(usize::try_from(self.total - self.pos).unwrap_or(usize::MAX));
        let mut done = 0;
        while done < wanted {
            self.send_due();
            // Render up to the next event, the chunk size, or the end.
            let until_event = self
                .frames
                .get(self.next)
                .map_or(u64::MAX, |&frame| frame - self.pos);
            let count = (wanted - done)
                .min(CHUNK)
                .min(usize::try_from(until_event).unwrap_or(usize::MAX));
            let (left, right) = (&mut self.left[..count], &mut self.right[..count]);
            self.synth.render(left, right);
            let frames = out[done * 2..(done + count) * 2].as_chunks_mut::<2>().0;
            for (frame, (&l, &r)) in frames.iter_mut().zip(left.iter().zip(right.iter())) {
                *frame = [l, r];
            }
            self.pos += count as u64;
            done += count;
        }
        done
    }

    fn seek(&mut self, frame: u64) {
        self.pos = frame.min(self.total);
        self.synth.reset();
        // Chase: replay every program change, controller and pitch bend
        // before the new position so the instruments are right, without
        // the notes.
        self.next = self.frames.partition_point(|&f| f < self.pos);
        for event in &self.sequence.events[..self.next] {
            if matches!(event.status & 0xF0, 0xB0 | 0xC0 | 0xE0) {
                self.synth.message(event.status, event.data1, event.data2);
            }
        }
    }

    fn midi_message(&mut self, status: u8, data1: u8, data2: u8) -> bool {
        self.synth.message(status, data1, data2);
        true
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::midi::smf::{self, tests::smf};

    /// A synthesizer that writes down what it is sent and when, and
    /// outputs a constant so rendering can be counted.
    #[derive(Clone, Default)]
    pub(crate) struct Recorder {
        pub log: Arc<Mutex<Vec<(u64, u8, u8)>>>,
        pub resets: Arc<Mutex<usize>>,
        rendered: u64,
    }

    impl Synth for Recorder {
        fn message(&mut self, status: u8, data1: u8, _data2: u8) {
            self.log
                .lock()
                .unwrap()
                .push((self.rendered, status, data1));
        }

        fn reset(&mut self) {
            *self.resets.lock().unwrap() += 1;
        }

        fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
            left.fill(0.5);
            right.fill(-0.5);
            self.rendered += left.len() as u64;
        }
    }

    fn file() -> Arc<Sequence> {
        // 96 ticks per quarter at 120 bpm; a program change, then notes
        // at 0, 0.5 s and 1 s; the track ends at 1.5 s.
        let track = [
            0x00, 0xC0, 10, //
            0x00, 0x90, 60, 100, //
            0x60, 0x80, 60, 0, //
            0x00, 0x90, 62, 100, //
            0x00, 0xB0, 7, 90, //
            0x60, 0x80, 62, 0, //
            0x60, 0xFF, 0x2F, 0x00,
        ];
        Arc::new(smf::parse(&smf(0, 96, &[&track])).unwrap())
    }

    #[test]
    fn sends_messages_on_their_frames() {
        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let mut source = MidiSource::new(file(), Box::new(recorder), 1000);
        assert_eq!(source.frames(), 1500);
        let mut out = vec![0.0; 2 * 4000];
        assert_eq!(source.render(&mut out), 1500);
        assert_eq!(&out[..2], &[0.5, -0.5]);
        let log = log.lock().unwrap();
        let onsets: Vec<(u64, u8)> = log.iter().map(|&(t, s, _)| (t, s)).collect();
        assert_eq!(
            onsets,
            vec![
                (0, 0xC0),
                (0, 0x90),
                (500, 0x80),
                (500, 0x90),
                (500, 0xB0),
                (1000, 0x80)
            ]
        );
    }

    #[test]
    fn small_buffers_keep_the_timing() {
        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let mut source = MidiSource::new(file(), Box::new(recorder), 1000);
        let mut out = vec![0.0; 2 * 7];
        let mut total = 0;
        loop {
            let n = source.render(&mut out);
            total += n;
            if n < 7 {
                break;
            }
        }
        assert_eq!(total, 1500);
        assert_eq!(log.lock().unwrap()[5].0, 1000);
    }

    #[test]
    fn seeking_chases_controllers_but_not_notes() {
        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let resets = recorder.resets.clone();
        let mut source = MidiSource::new(file(), Box::new(recorder), 1000);
        source.seek(700);
        assert_eq!(*resets.lock().unwrap(), 1);
        {
            let log = log.lock().unwrap();
            let chased: Vec<u8> = log.iter().map(|&(_, s, _)| s).collect();
            assert_eq!(chased, vec![0xC0, 0xB0]);
        }
        let mut out = vec![0.0; 2 * 4000];
        assert_eq!(source.render(&mut out), 800);
        // The note off at 1 s comes 300 frames after the seek.
        assert_eq!(log.lock().unwrap().last(), Some(&(300, 0x80, 62)));
        source.seek(5000);
        assert_eq!(source.render(&mut out), 0);
    }

    #[test]
    fn short_messages_reach_the_synthesizer() {
        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let mut source = MidiSource::new(file(), Box::new(recorder), 1000);
        assert!(source.midi_message(0xB3, 7, 0));
        assert_eq!(log.lock().unwrap().as_slice(), &[(0, 0xB3, 7)]);
    }
}
