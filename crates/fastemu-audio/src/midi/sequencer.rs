//! Plays a parsed MIDI file through a synthesizer, frame by frame.

use std::sync::Arc;

use crate::midi::Synth;
use crate::midi::smf::{Event, Sequence, TimeSignature};
use crate::source::Source;
use crate::voice::MIN_LOOP_US;

/// Frames rendered per synthesizer call.
const CHUNK: usize = 256;

/// Messages one call of [`Source::render`] sends at most. A file can put
/// tens of thousands of events at one instant, and sending them all at once
/// holds the mixer, and every game's sound, for as long as that takes.
/// The rest wait for the next call, a few milliseconds on, so a pile is
/// sent in order over several blocks. A real file has a few hundred events
/// at its start and a few a block after that.
const MAX_MESSAGES_PER_CALL: usize = 512;

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

    /// Sends the events whose frame has come, up to `budget` of them.
    fn send_due(&mut self, budget: &mut usize) {
        while *budget > 0 && self.next < self.frames.len() && self.frames[self.next] <= self.pos {
            let event = self.sequence.events[self.next];
            self.synth.message(event.status, event.data1, event.data2);
            self.next += 1;
            *budget -= 1;
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

    /// Every pass sends the file's events again, so a file of a few
    /// frames plays once, however many loops it is asked for.
    fn min_loop_frames(&self) -> u64 {
        us_to_frames(MIN_LOOP_US, self.rate)
    }

    fn render(&mut self, out: &mut [f32]) -> usize {
        let wanted =
            (out.len() / 2).min(usize::try_from(self.total - self.pos).unwrap_or(usize::MAX));
        let mut done = 0;
        let mut budget = MAX_MESSAGES_PER_CALL;
        while done < wanted {
            self.send_due(&mut budget);
            // Render up to the next event, the chunk size, or the end. An
            // event that is due already is one the budget kept back: it
            // waits for the next call.
            let until_event = match self.frames.get(self.next) {
                Some(&frame) if frame > self.pos => frame - self.pos,
                _ => u64::MAX,
            };
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

    /// The parsed events, the frame each falls on, and the time
    /// signatures, which a file of nothing else can fill the memory with.
    fn memory_bytes(&self) -> usize {
        self.sequence.events.len() * size_of::<Event>()
            + self.sequence.time_signatures.len() * size_of::<TimeSignature>()
            + self.frames.len() * size_of::<u64>()
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
    fn the_events_and_their_frames_are_the_memory() {
        let sequence = file();
        // Six messages, each a 16-byte event and an 8-byte frame.
        assert_eq!(sequence.events.len(), 6);
        let source = MidiSource::new(sequence, Box::new(Recorder::default()), 1000);
        assert_eq!(source.memory_bytes(), 6 * 24);
    }

    #[test]
    fn time_signatures_are_memory_too() {
        // A time signature is eight bytes of file and sixteen of memory,
        // and a file of nothing else holds no events at all.
        let mut track = Vec::new();
        for _ in 0..1000 {
            track.extend_from_slice(&[0x00, 0xFF, 0x58, 0x04, 4, 2, 24, 8]);
        }
        track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let sequence = Arc::new(smf::parse(&smf(0, 96, &[&track])).unwrap());
        assert!(sequence.events.is_empty());
        assert_eq!(sequence.time_signatures.len(), 1000);
        let source = MidiSource::new(sequence, Box::new(Recorder::default()), 1000);
        assert_eq!(source.memory_bytes(), 1000 * size_of::<TimeSignature>());
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

    /// A variable-length quantity, as MIDI files write delta times.
    fn vlq(mut value: u32) -> Vec<u8> {
        let mut out = vec![(value & 0x7F) as u8];
        value >>= 7;
        while value > 0 {
            out.push((value & 0x7F) as u8 | 0x80);
            value >>= 7;
        }
        out.reverse();
        out
    }

    /// A file of `notes` note ons at tick 0 that lasts about 21
    /// microseconds, one frame at 48 kHz: 32767 ticks to the quarter note
    /// at a tempo of one microsecond to the quarter.
    fn blink(notes: usize) -> Arc<Sequence> {
        let mut track = vec![0x00, 0xFF, 0x51, 0x03, 0, 0, 1];
        for i in 0..notes {
            track.extend_from_slice(&[0x00, 0x90 | (i % 16) as u8, (i % 128) as u8, 100]);
        }
        track.extend_from_slice(&vlq(688_128));
        track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
        Arc::new(smf::parse(&smf(0, 0x7FFF, &[&track])).unwrap())
    }

    #[test]
    fn a_pile_of_events_at_one_instant_is_sent_over_several_calls() {
        // 3000 controllers at tick 0, then a note on half a second later.
        let mut track = Vec::new();
        for i in 0..3000 {
            track.extend_from_slice(&[0x00, 0xB0 | (i % 16) as u8, (i % 128) as u8, 1]);
        }
        track.extend_from_slice(&[
            0x60, 0x90, 60, 100, //
            0x60, 0x80, 60, 0, //
            0x60, 0xFF, 0x2F, 0x00,
        ]);
        let sequence = Arc::new(smf::parse(&smf(0, 96, &[&track])).unwrap());
        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let mut source = MidiSource::new(sequence, Box::new(recorder), 8000);
        let mut out = vec![0.0; 2 * 256];
        // The first call sends a budget's worth and renders its frames.
        assert_eq!(source.render(&mut out), 256);
        assert_eq!(log.lock().unwrap().len(), 512);
        // The rest follow, in order, a block each; none is lost.
        while source.render(&mut out) == 256 {}
        let log = log.lock().unwrap();
        assert_eq!(log.len(), 3002);
        for (i, &(_, status, data1)) in log[..3000].iter().enumerate() {
            assert_eq!((status, data1), (0xB0 | (i % 16) as u8, (i % 128) as u8));
        }
        // The pile was sent within a few blocks, and the note on that
        // follows still came on its own frame, half a second in.
        assert!(
            log[2999].0 < 8 * 256,
            "the last controller came at {}",
            log[2999].0
        );
        assert_eq!(log[3000], (4000, 0x90, 60));
    }

    #[test]
    fn a_file_of_a_few_frames_plays_once_however_it_is_looped() {
        use crate::voice::{PlayerState, Reached, Voice};

        let recorder = Recorder::default();
        let log = recorder.log.clone();
        let source = MidiSource::new(blink(400), Box::new(recorder), 48_000);
        assert_eq!(source.frames(), 1);
        let mut voice = Voice::new(Box::new(source), false);
        voice.set_loop_count(-1);
        voice.start();
        voice.gain = 1.0;
        let mut out = vec![0.0; 2 * 1024];
        let mut seen = Vec::new();
        voice.render(&mut out, 1.0, 1.0, 1.0, false, |r, _| seen.push(r));
        // Every note was sent once, not once for each of 1024 passes.
        assert_eq!(log.lock().unwrap().len(), 400);
        assert_eq!(seen, vec![Reached::End { error: None }]);
        assert_eq!(voice.state, PlayerState::Prefetched);
    }

    #[test]
    fn a_file_of_20_ms_or_more_is_looped() {
        use crate::voice::Reached;
        use crate::voice::Voice;

        // 20 ms is the floor: 960 frames at 48 kHz.
        let source = MidiSource::new(file(), Box::new(Recorder::default()), 48_000);
        assert_eq!(source.min_loop_frames(), 960);
        assert!(source.frames() >= source.min_loop_frames());
        let mut voice = Voice::new(Box::new(source), false);
        voice.set_loop_count(2);
        voice.start();
        voice.gain = 1.0;
        let mut out = vec![0.0; 2 * 150_000];
        let mut seen = Vec::new();
        voice.render(&mut out, 1.0, 1.0, 1.0, false, |r, _| seen.push(r));
        assert_eq!(seen, vec![Reached::Loop, Reached::End { error: None }]);
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
