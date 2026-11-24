//! Nokia's sound API: Smart Messaging ringing tones
//! (`com.nokia.mid.sound.Sound.FORMAT_TONE`) and plain beeps.
//!
//! A ringing tone is a bit stream, most significant bit first:
//!
//! ```text
//! ringing-tone := command-count:8 command*
//! command      := 0100101 filler          ringing tone programming
//!               | 0100010 filler          titles are UCS-2
//!               | 0011101 song-type:3 song   sound
//! song         := title-length:4 char* pattern-count:8 pattern*
//! pattern      := 000 id:2 loop:4 count:8 instruction*count
//! instruction  := 001 note:4 duration:3 dots:2
//!               | 010 scale:2 | 011 style:2 | 100 tempo:5 | 101 volume:4
//! ```
//!
//! A pattern with a count of zero plays the pattern defined earlier with
//! the same id. Notes 1 to 12 are C to B in the current scale, whose A is
//! 440 Hz times 1, 2 (the default), 4 or 8; note 0 is a rest.

use crate::error::AudioError;
use crate::format::Format;
use crate::tone::ToneEvent;

const COMMAND_PROGRAMMING: u32 = 0b010_0101;
const COMMAND_UNICODE: u32 = 0b010_0010;
const COMMAND_SOUND: u32 = 0b001_1101;
const SONG_BASIC: u32 = 0b001;
const SONG_TEMPORARY: u32 = 0b010;

const TEMPOS_BPM: [u32; 32] = [
    25, 28, 31, 35, 40, 45, 50, 56, 63, 70, 80, 90, 100, 112, 125, 140, 160, 180, 200, 225, 250,
    285, 320, 355, 400, 450, 500, 565, 635, 715, 800, 900,
];

/// A pattern may loop at most this many extra times; the loop value 15
/// means forever, which the `Sound` object's own loop count covers, so it
/// plays once.
const MAX_PATTERN_LOOPS: u32 = 14;

/// Reads a Smart Messaging ringing tone into tones.
pub(crate) fn parse_ringtone(data: &[u8]) -> Result<Vec<ToneEvent>, AudioError> {
    let malformed = |reason: &str| AudioError::Malformed {
        format: Format::NokiaRingtone,
        reason: reason.into(),
    };
    let mut bits = BitReader { data, at: 0 };
    let commands = bits.read(8).ok_or_else(|| malformed("empty"))?;
    let mut unicode = false;
    for _ in 0..commands {
        match bits
            .read(7)
            .ok_or_else(|| malformed("ends inside a command"))?
        {
            COMMAND_PROGRAMMING => bits.align(),
            COMMAND_UNICODE => {
                unicode = true;
                bits.align();
            }
            COMMAND_SOUND => {
                return song(&mut bits, unicode).ok_or_else(|| malformed("ends inside the song"));
            }
            _ => break,
        }
    }
    Err(malformed("no sound command"))
}

/// A beep for `Sound(frequency, duration)`. A frequency of zero is a rest.
pub(crate) fn beep(frequency_hz: u32, duration_ms: u64) -> ToneEvent {
    ToneEvent {
        frequency: frequency_hz as f32,
        duration_us: duration_ms.saturating_mul(1000),
        volume: 100,
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    /// The next bit to read, counted from the first byte's top bit.
    at: usize,
}

impl BitReader<'_> {
    fn read(&mut self, count: usize) -> Option<u32> {
        let mut value = 0;
        for _ in 0..count {
            let byte = *self.data.get(self.at / 8)?;
            let bit = (byte >> (7 - self.at % 8)) & 1;
            value = (value << 1) | u32::from(bit);
            self.at += 1;
        }
        Some(value)
    }

    fn align(&mut self) {
        self.at = self.at.div_ceil(8) * 8;
    }
}

/// The state instructions change, kept across patterns.
#[derive(Clone, Copy)]
struct Voice {
    /// 0 to 3: A is 440 Hz times 2 to this power.
    scale: u32,
    style: Style,
    tempo_bpm: u32,
    /// 0 (off) to 15.
    volume: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    /// A short gap after each note.
    Natural,
    Continuous,
    /// Notes sound for half their length.
    Staccato,
}

#[derive(Clone, Copy)]
enum Instruction {
    Note {
        value: u32,
        duration: u32,
        dots: u32,
    },
    Scale(u32),
    Style(Style),
    Tempo(u32),
    Volume(u32),
}

fn song(bits: &mut BitReader<'_>, unicode: bool) -> Option<Vec<ToneEvent>> {
    let song_type = bits.read(3)?;
    if song_type == SONG_BASIC {
        let title_length = bits.read(4)? as usize;
        bits.read(title_length * if unicode { 16 } else { 8 })?;
    } else if song_type != SONG_TEMPORARY {
        return Some(Vec::new());
    }
    let pattern_count = bits.read(8)?;
    let mut patterns: [Option<Vec<Instruction>>; 4] = Default::default();
    let mut voice = Voice {
        scale: 1,
        style: Style::Natural,
        tempo_bpm: 63,
        volume: 7,
    };
    let mut tones = Vec::new();
    for _ in 0..pattern_count {
        let Some(header) = bits.read(3) else { break };
        if header != 0 {
            break;
        }
        let id = bits.read(2)? as usize;
        let loops = bits.read(4)?;
        let count = bits.read(8)?;
        if count > 0 {
            let mut instructions = Vec::with_capacity(count as usize);
            for _ in 0..count {
                let Some(instruction) = instruction(bits) else {
                    break;
                };
                instructions.push(instruction);
            }
            patterns[id] = Some(instructions);
        }
        let plays = if loops > MAX_PATTERN_LOOPS {
            1
        } else {
            loops + 1
        };
        if let Some(instructions) = &patterns[id] {
            for _ in 0..plays {
                for &instruction in instructions {
                    play(instruction, &mut voice, &mut tones);
                }
            }
        }
    }
    Some(tones)
}

/// Reads one instruction. `None` ends the pattern: the data ran out or
/// the instruction type is unknown.
fn instruction(bits: &mut BitReader<'_>) -> Option<Instruction> {
    let instruction = match bits.read(3)? {
        0b001 => Instruction::Note {
            value: bits.read(4)?,
            duration: bits.read(3)?,
            dots: bits.read(2)?,
        },
        0b010 => Instruction::Scale(bits.read(2)?),
        0b011 => Instruction::Style(match bits.read(2)? {
            0 => Style::Natural,
            1 => Style::Continuous,
            _ => Style::Staccato,
        }),
        0b100 => Instruction::Tempo(TEMPOS_BPM[bits.read(5)? as usize]),
        0b101 => Instruction::Volume(bits.read(4)?),
        _ => return None,
    };
    Some(instruction)
}

fn play(instruction: Instruction, voice: &mut Voice, tones: &mut Vec<ToneEvent>) {
    match instruction {
        Instruction::Scale(scale) => voice.scale = scale,
        Instruction::Style(style) => voice.style = style,
        Instruction::Tempo(bpm) => voice.tempo_bpm = bpm,
        Instruction::Volume(volume) => voice.volume = volume,
        Instruction::Note {
            value,
            duration,
            dots,
        } => {
            // A whole note is four beats; durations halve from there.
            let whole_us = 4.0 * 60_000_000.0 / f64::from(voice.tempo_bpm);
            let mut length_us = whole_us / f64::from(1u32 << duration.min(5));
            length_us *= match dots {
                1 => 1.5,
                2 => 1.75,
                3 => 2.0 / 3.0,
                _ => 1.0,
            };
            let length_us = length_us.round() as u64;
            if !(1..=12).contains(&value) || voice.volume == 0 {
                tones.push(ToneEvent::rest(length_us));
                return;
            }
            let a = 440.0 * f64::from(1u32 << voice.scale);
            let frequency = a * 2f64.powf((f64::from(value) - 10.0) / 12.0);
            let sounding_us = match voice.style {
                Style::Natural => length_us * 7 / 8,
                Style::Continuous => length_us,
                Style::Staccato => length_us / 2,
            };
            tones.push(ToneEvent {
                frequency: frequency as f32,
                duration_us: sounding_us,
                // Volume 1 to 15 maps onto MMAPI levels 44 to 100.
                volume: (40 + voice.volume * 4) as u8,
            });
            if sounding_us < length_us {
                tones.push(ToneEvent::rest(length_us - sounding_us));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct BitWriter {
        bytes: Vec<u8>,
        bits: usize,
    }

    impl BitWriter {
        fn put(&mut self, value: u32, count: usize) {
            for i in (0..count).rev() {
                if self.bits.is_multiple_of(8) {
                    self.bytes.push(0);
                }
                let bit = ((value >> i) & 1) as u8;
                *self.bytes.last_mut().unwrap() |= bit << (7 - self.bits % 8);
                self.bits += 1;
            }
        }

        fn align(&mut self) {
            self.bits = self.bits.div_ceil(8) * 8;
        }
    }

    fn header(w: &mut BitWriter, title: &str, patterns: u32) {
        w.put(2, 8);
        w.put(COMMAND_PROGRAMMING, 7);
        w.align();
        w.put(COMMAND_SOUND, 7);
        w.put(SONG_BASIC, 3);
        w.put(title.len() as u32, 4);
        for byte in title.bytes() {
            w.put(u32::from(byte), 8);
        }
        w.put(patterns, 8);
    }

    fn note(w: &mut BitWriter, value: u32, duration: u32, dots: u32) {
        w.put(0b001, 3);
        w.put(value, 4);
        w.put(duration, 3);
        w.put(dots, 2);
    }

    #[test]
    fn reads_a_basic_song() {
        let mut w = BitWriter::default();
        header(&mut w, "AB", 1);
        // Pattern A, no loop, four instructions.
        w.put(0, 3);
        w.put(0, 2);
        w.put(0, 4);
        w.put(4, 8);
        // Tempo 100 bpm (index 12), style continuous, then A and a rest.
        w.put(0b100, 3);
        w.put(12, 5);
        w.put(0b011, 3);
        w.put(1, 2);
        note(&mut w, 10, 2, 0);
        note(&mut w, 0, 2, 1);
        let tones = parse_ringtone(&w.bytes).unwrap();
        assert_eq!(tones.len(), 2);
        // A quarter note at 100 bpm is 0.6 s; A in the default scale is 880 Hz.
        assert_eq!(tones[0].duration_us, 600_000);
        assert!((tones[0].frequency - 880.0).abs() < 0.01);
        assert_eq!(tones[0].volume, 68);
        assert_eq!(tones[1], ToneEvent::rest(900_000));
        assert_eq!(Format::sniff(&w.bytes), Some(Format::NokiaRingtone));
    }

    #[test]
    fn natural_style_leaves_a_gap_and_patterns_repeat() {
        let mut w = BitWriter::default();
        header(&mut w, "", 2);
        // Pattern B loops once more: scale 1 (A = 440), C, at 63 bpm.
        w.put(0, 3);
        w.put(1, 2);
        w.put(1, 4);
        w.put(2, 8);
        w.put(0b010, 3);
        w.put(0, 2);
        note(&mut w, 1, 0, 0);
        // Pattern B again by reference.
        w.put(0, 3);
        w.put(1, 2);
        w.put(0, 4);
        w.put(0, 8);
        let tones = parse_ringtone(&w.bytes).unwrap();
        // Three plays of C, each a note and its gap.
        assert_eq!(tones.len(), 6);
        let whole = (4.0 * 60_000_000.0 / 63.0f64).round() as u64;
        assert_eq!(tones[0].duration_us + tones[1].duration_us, whole);
        assert!((tones[0].frequency - 261.63).abs() < 0.01);
        assert_eq!(tones[1].frequency, 0.0);
    }

    #[test]
    fn a_truncated_song_keeps_what_it_read_or_fails() {
        let mut w = BitWriter::default();
        header(&mut w, "X", 1);
        w.put(0, 3);
        w.put(0, 2);
        w.put(0, 4);
        w.put(5, 8);
        note(&mut w, 10, 3, 0);
        // The pattern promised five instructions but the data ends.
        let tones = parse_ringtone(&w.bytes).unwrap();
        assert_eq!(tones.len(), 2);
        assert!(parse_ringtone(&w.bytes[..3]).is_err());
        assert!(parse_ringtone(&[]).is_err());
        assert!(parse_ringtone(&[1, 0]).is_err());
    }

    #[test]
    fn beeps_are_single_tones() {
        assert_eq!(beep(1000, 250).duration_us, 250_000);
        assert_eq!(beep(1000, 250).frequency, 1000.0);
    }
}
