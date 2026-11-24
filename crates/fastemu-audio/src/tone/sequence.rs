//! The JSR-135 `ToneControl` sequence format.
//!
//! A sequence is a byte array (Java bytes, so signed) read as:
//!
//! ```text
//! sequence      := VERSION 1 [TEMPO t] [RESOLUTION r] (block | event)*
//! block         := BLOCK_START n event+ BLOCK_END n
//! event         := note duration | SILENCE duration | PLAY_BLOCK n
//!                | SET_VOLUME v | REPEAT m (note | SILENCE) duration
//! ```
//!
//! A note lasts `duration * 60000 * 4 / (resolution * tempo)` ms, where
//! the tempo is `t * 4` beats per minute (120 unless set) and the
//! resolution is 64 unless set. Blocks are named 0 to 127 and must be
//! defined before they are played.

use crate::error::AudioError;
use crate::tone::ToneEvent;

const VERSION: i8 = -2;
const TEMPO: i8 = -3;
const RESOLUTION: i8 = -4;
const BLOCK_START: i8 = -5;
const BLOCK_END: i8 = -6;
const PLAY_BLOCK: i8 = -7;
const SET_VOLUME: i8 = -8;
const REPEAT: i8 = -9;
const SILENCE: i8 = -1;

const DEFAULT_TEMPO_BPM: u64 = 120;
const DEFAULT_RESOLUTION: u64 = 64;

/// A sequence that unrolls to more tones than this is refused: blocks
/// that play blocks can grow without bound. Even at the shortest note
/// length this is minutes of sound; real sequences hold a few hundred.
const MAX_EVENTS: usize = 1 << 16;

/// Reads a tone sequence into the tones it plays, blocks and repeats
/// unrolled.
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<ToneEvent>, AudioError> {
    Parser::new(bytes).sequence()
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    tempo_bpm: u64,
    resolution: u64,
    volume: u8,
    /// The unrolled tones of each defined block.
    blocks: Vec<Option<Vec<Event>>>,
}

/// A tone before volume is applied: blocks keep these, since a block
/// plays at whatever volume is set when it is played.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Event {
    Note { note: Option<u8>, duration_us: u64 },
    Volume(u8),
}

fn error(at: usize, reason: impl Into<String>) -> AudioError {
    AudioError::InvalidToneSequence {
        offset: at,
        reason: reason.into(),
    }
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            at: 0,
            tempo_bpm: DEFAULT_TEMPO_BPM,
            resolution: DEFAULT_RESOLUTION,
            volume: 100,
            blocks: vec![None; 128],
        }
    }

    fn peek(&self) -> Option<i8> {
        self.bytes.get(self.at).map(|&b| b as i8)
    }

    fn next(&mut self, what: &str) -> Result<i8, AudioError> {
        let value = self
            .peek()
            .ok_or_else(|| error(self.at, format!("ends before the {what}")))?;
        self.at += 1;
        Ok(value)
    }

    /// Reads a byte that must fall in `range`.
    fn value(&mut self, what: &str, range: std::ops::RangeInclusive<i8>) -> Result<i8, AudioError> {
        let at = self.at;
        let value = self.next(what)?;
        if range.contains(&value) {
            Ok(value)
        } else {
            Err(error(at, format!("{what} {value} is out of range")))
        }
    }

    fn sequence(mut self) -> Result<Vec<ToneEvent>, AudioError> {
        let at = self.at;
        if self.next("version")? != VERSION {
            return Err(error(at, "a sequence starts with VERSION"));
        }
        let version = self.value("version", 1..=1)?;
        debug_assert_eq!(version, 1);
        // TEMPO and RESOLUTION, in either order, before anything else.
        loop {
            match self.peek() {
                Some(TEMPO) => {
                    self.at += 1;
                    self.tempo_bpm = self.value("tempo", 5..=127)? as u64 * 4;
                }
                Some(RESOLUTION) => {
                    self.at += 1;
                    self.resolution = self.value("resolution", 1..=127)? as u64;
                }
                _ => break,
            }
        }
        let mut events = Vec::new();
        while let Some(command) = self.peek() {
            if command == BLOCK_START {
                self.block()?;
            } else {
                self.event(&mut events)?;
            }
        }
        Ok(self.apply_volume(&events))
    }

    fn block(&mut self) -> Result<(), AudioError> {
        self.at += 1;
        let number = self.value("block number", 0..=127)?;
        let mut events = Vec::new();
        loop {
            let at = self.at;
            match self.peek() {
                None => return Err(error(at, format!("block {number} never ends"))),
                Some(BLOCK_END) => {
                    self.at += 1;
                    let end = self.at;
                    let closing = self.value("block number", 0..=127)?;
                    if closing != number {
                        return Err(error(
                            end,
                            format!("block {number} ends with BLOCK_END {closing}"),
                        ));
                    }
                    break;
                }
                Some(BLOCK_START) => {
                    return Err(error(at, "blocks cannot be defined inside blocks"));
                }
                Some(_) => self.event(&mut events)?,
            }
        }
        if events.is_empty() {
            return Err(error(self.at, format!("block {number} is empty")));
        }
        self.blocks[number as usize] = Some(events);
        Ok(())
    }

    fn event(&mut self, events: &mut Vec<Event>) -> Result<(), AudioError> {
        let at = self.at;
        match self.next("event")? {
            PLAY_BLOCK => {
                let number = self.value("block number", 0..=127)?;
                let block = self.blocks[number as usize]
                    .as_ref()
                    .ok_or_else(|| error(at, format!("block {number} is not defined")))?;
                if events.len() + block.len() > MAX_EVENTS {
                    return Err(error(at, "the sequence unrolls to too many tones"));
                }
                events.extend_from_slice(block);
            }
            SET_VOLUME => {
                let volume = self.value("volume", 0..=100)?;
                events.push(Event::Volume(volume as u8));
            }
            REPEAT => {
                let times = self.value("repeat count", 2..=127)?;
                let note_at = self.at;
                let note = self.next("repeated note")?;
                let tone = self.tone(note_at, note)?;
                if events.len() + times as usize > MAX_EVENTS {
                    return Err(error(at, "the sequence unrolls to too many tones"));
                }
                events.extend(std::iter::repeat_n(tone, times as usize));
            }
            note => {
                let tone = self.tone(at, note)?;
                if events.len() >= MAX_EVENTS {
                    return Err(error(at, "the sequence unrolls to too many tones"));
                }
                events.push(tone);
            }
        }
        Ok(())
    }

    /// Reads the duration after `note` (already read at `at`).
    fn tone(&mut self, at: usize, note: i8) -> Result<Event, AudioError> {
        let note = match note {
            SILENCE => None,
            0.. => Some(note as u8),
            other => return Err(error(at, format!("{other} is not a note or command"))),
        };
        let duration = self.value("duration", 1..=127)? as u64;
        let duration_us = duration * 240_000_000 / (self.resolution * self.tempo_bpm);
        Ok(Event::Note { note, duration_us })
    }

    fn apply_volume(&mut self, events: &[Event]) -> Vec<ToneEvent> {
        let mut tones = Vec::with_capacity(events.len());
        for event in events {
            match *event {
                Event::Volume(volume) => self.volume = volume,
                Event::Note {
                    note: Some(note),
                    duration_us,
                } => tones.push(ToneEvent::note(note, duration_us, self.volume)),
                Event::Note {
                    note: None,
                    duration_us,
                } => tones.push(ToneEvent::rest(duration_us)),
            }
        }
        tones
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C4: i8 = 60;
    const D4: i8 = 62;

    fn bytes(values: &[i8]) -> Vec<u8> {
        values.iter().map(|&v| v as u8).collect()
    }

    fn parse_i8(values: &[i8]) -> Result<Vec<ToneEvent>, AudioError> {
        parse(&bytes(values))
    }

    fn offset_of(result: Result<Vec<ToneEvent>, AudioError>) -> usize {
        match result {
            Err(AudioError::InvalidToneSequence { offset, .. }) => offset,
            other => panic!("expected a tone sequence error, got {other:?}"),
        }
    }

    #[test]
    fn plays_notes_at_the_default_tempo() {
        // A quarter note at 120 bpm is half a second.
        let tones = parse_i8(&[VERSION, 1, C4, 16, SILENCE, 8]).unwrap();
        assert_eq!(tones.len(), 2);
        assert_eq!(tones[0].duration_us, 500_000);
        assert!((tones[0].frequency - 261.6256).abs() < 0.01);
        assert_eq!(tones[0].volume, 100);
        assert_eq!(tones[1], ToneEvent::rest(250_000));
    }

    #[test]
    fn tempo_and_resolution_change_durations() {
        // TEMPO 15 is 60 bpm; RESOLUTION 4 makes a duration of 1 a
        // quarter note: one second.
        let tones = parse_i8(&[VERSION, 1, TEMPO, 15, RESOLUTION, 4, C4, 1]).unwrap();
        assert_eq!(tones[0].duration_us, 1_000_000);
        let tones = parse_i8(&[VERSION, 1, RESOLUTION, 4, TEMPO, 15, C4, 2]).unwrap();
        assert_eq!(tones[0].duration_us, 2_000_000);
    }

    #[test]
    fn blocks_repeat_and_volume() {
        let tones = parse_i8(&[
            VERSION,
            1,
            BLOCK_START,
            3,
            C4,
            8,
            D4,
            8,
            BLOCK_END,
            3,
            SET_VOLUME,
            50,
            PLAY_BLOCK,
            3,
            SET_VOLUME,
            20,
            PLAY_BLOCK,
            3,
            REPEAT,
            3,
            C4,
            4,
        ])
        .unwrap();
        assert_eq!(tones.len(), 7);
        assert_eq!(tones[0].volume, 50);
        assert_eq!(tones[1].volume, 50);
        assert_eq!(tones[2].volume, 20);
        assert_eq!(tones[4], tones[5]);
        assert_eq!(tones[6].duration_us, 125_000);
    }

    #[test]
    fn blocks_can_play_earlier_blocks() {
        let tones = parse_i8(&[
            VERSION,
            1,
            BLOCK_START,
            0,
            C4,
            8,
            BLOCK_END,
            0,
            BLOCK_START,
            1,
            PLAY_BLOCK,
            0,
            PLAY_BLOCK,
            0,
            BLOCK_END,
            1,
            PLAY_BLOCK,
            1,
            PLAY_BLOCK,
            1,
        ])
        .unwrap();
        assert_eq!(tones.len(), 4);
    }

    #[test]
    fn an_empty_body_is_allowed() {
        assert_eq!(parse_i8(&[VERSION, 1]).unwrap(), vec![]);
    }

    #[test]
    fn reports_where_a_sequence_breaks() {
        assert_eq!(offset_of(parse_i8(&[])), 0);
        assert_eq!(offset_of(parse_i8(&[C4, 8])), 0);
        assert_eq!(offset_of(parse_i8(&[VERSION, 2])), 1);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, TEMPO, 4])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, RESOLUTION, 0])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, C4])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, C4, 0])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, -20, 8])), 2);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, SET_VOLUME, 101])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, REPEAT, 1, C4, 8])), 3);
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, PLAY_BLOCK, 5])), 2);
        // A block ended with the wrong number, never ended, or empty.
        assert_eq!(
            offset_of(parse_i8(&[VERSION, 1, BLOCK_START, 1, C4, 8, BLOCK_END, 2])),
            7
        );
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, BLOCK_START, 1, C4, 8])), 6);
        assert_eq!(
            offset_of(parse_i8(&[VERSION, 1, BLOCK_START, 1, BLOCK_END, 1])),
            6
        );
        // A block cannot play itself.
        assert_eq!(
            offset_of(parse_i8(&[
                VERSION,
                1,
                BLOCK_START,
                1,
                PLAY_BLOCK,
                1,
                BLOCK_END,
                1
            ])),
            4
        );
        // TEMPO after the first event is not a note.
        assert_eq!(offset_of(parse_i8(&[VERSION, 1, C4, 8, TEMPO, 30])), 4);
    }

    #[test]
    fn refuses_sequences_that_unroll_without_bound() {
        // Each block plays the one before it 100 times.
        let mut seq = vec![VERSION, 1, BLOCK_START, 0, C4, 1, BLOCK_END, 0];
        for n in 1..5i8 {
            seq.extend_from_slice(&[BLOCK_START, n]);
            for _ in 0..100 {
                seq.extend_from_slice(&[PLAY_BLOCK, n - 1]);
            }
            seq.extend_from_slice(&[BLOCK_END, n]);
        }
        seq.extend_from_slice(&[PLAY_BLOCK, 4]);
        assert!(matches!(
            parse_i8(&seq),
            Err(AudioError::InvalidToneSequence { .. })
        ));
    }
}
