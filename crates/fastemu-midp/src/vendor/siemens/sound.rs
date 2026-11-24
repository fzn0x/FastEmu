//! Siemens' `com.siemens.mp.game` sound: `Sound.playTone`, and melodies
//! built note by note with `MelodyComposer` and played as `Melody`.
//!
//! A melody becomes a standard MIDI file played by the instance's sound
//! group, following freej2me-plus, which games were tested on: tone
//! `TONE_C0` (0) is MIDI note 24, a quarter note lasts one beat at the
//! composer's tempo, and the lead square-wave instrument (program 80)
//! plays it. `TONE_REPEAT` repeats everything before it, and
//! `TONE_REPEAT_MARK` what follows the last `TONE_MARK`, as many more
//! times as the note's length says; `TONE_STOP` ends the melody. The
//! `REPEV` and `REPON` notes, which no source explains, are skipped. One
//! melody plays at a time; `Melody.stop()` stops it.

use fastemu_audio::{Format, Player};
use fastemu_jvm::{Ctx, JResult, NativeClass, Payload, Value};

use super::NATIVE_MEM;
use crate::vendor::clip;

const MELODY: &str = "com/siemens/mp/game/Melody";
const COMPOSER: &str = "com/siemens/mp/game/MelodyComposer";

/// The note names of `TONE_*`, from `TONE_C0` (0) to `TONE_A4` (57).
const NOTES: [&str; 12] = [
    "C", "CIS", "D", "DIS", "E", "F", "FIS", "G", "GIS", "A", "AIS", "H",
];
const TONE_A4: i32 = 57;
const TONE_PAUSE: i32 = 58;
const NO_TONE: i32 = 59;
const TONE_STOP: i32 = 60;
const TONE_REPEAT: i32 = 61;
const TONE_MARK: i32 = 64;
const TONE_REPEAT_MARK: i32 = 65;
/// The special notes after `TONE_PAUSE`, by value.
const SPECIAL: [&str; 10] = [
    "TONE_PAUSE",
    "NO_TONE",
    "TONE_STOP",
    "TONE_REPEAT",
    "TONE_REPEV",
    "TONE_REPON",
    "TONE_MARK",
    "TONE_REPEAT_MARK",
    "TONE_REPEV_MARK",
    "TONE_REPON_MARK",
];
/// `TONELENGTH_*`, by value.
const LENGTHS: [&str; 14] = [
    "1_1",
    "1_2",
    "1_4",
    "1_8",
    "1_16",
    "1_32",
    "1_64",
    "DOTTED_1_1",
    "DOTTED_1_2",
    "DOTTED_1_4",
    "DOTTED_1_8",
    "DOTTED_1_16",
    "DOTTED_1_32",
    "DOTTED_1_64",
];
/// The tempo a composer starts with, in beats per minute.
const BPM: i32 = 60;
/// How many notes a composer holds, as in freej2me-plus.
const MAX_NOTES: usize = 32_767;
/// MIDI ticks per quarter note: enough for a dotted sixty-fourth.
const TICKS_PER_QUARTER: u32 = 96;
/// The most events one melody expands to, so repeats cannot exhaust the
/// host.
const MAX_EVENTS: usize = 1 << 16;

/// The MIDI note number a frequency in hertz is closest to.
fn note_of(frequency_hz: i32) -> i32 {
    (69.0 + 12.0 * (f64::from(frequency_hz) / 440.0).log2()).round() as i32
}

/// MIDI ticks for a `TONELENGTH_*` value; unknown lengths are quarters.
fn ticks(length: i32) -> u32 {
    let whole = TICKS_PER_QUARTER * 4;
    match length {
        0..=6 => whole >> length,
        7..=13 => (whole >> (length - 7)) * 3 / 2,
        _ => TICKS_PER_QUARTER,
    }
}

/// The notes and rests a list of composer notes plays: a MIDI note or
/// `None` for a rest, and its length in ticks.
fn expand(notes: &[(i32, i32)]) -> Vec<(Option<u8>, u32)> {
    let mut out: Vec<(Option<u8>, u32)> = Vec::new();
    let mut mark = 0;
    for &(note, length) in notes {
        match note {
            0..=TONE_A4 => out.push((Some(note as u8 + 24), ticks(length))),
            TONE_PAUSE | NO_TONE => out.push((None, ticks(length))),
            TONE_STOP => break,
            TONE_MARK => mark = out.len(),
            TONE_REPEAT | TONE_REPEAT_MARK => {
                let from = if note == TONE_REPEAT { 0 } else { mark };
                let part = out[from..].to_vec();
                // Repeating nothing adds nothing, however many times.
                let room = match part.len() {
                    0 => 0,
                    len => (MAX_EVENTS - out.len()) / len,
                };
                for _ in 0..(length.max(0) as usize).min(room) {
                    out.extend_from_slice(&part);
                }
            }
            _ => {}
        }
        if out.len() >= MAX_EVENTS {
            out.truncate(MAX_EVENTS);
            break;
        }
    }
    out
}

/// A MIDI variable-length quantity.
fn put_varint(out: &mut Vec<u8>, mut value: u32) {
    let mut bytes = vec![(value & 0x7F) as u8];
    value >>= 7;
    while value > 0 {
        bytes.push((value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
    out.extend(bytes.iter().rev());
}

/// A standard MIDI file (format 0) playing `notes` at `bpm` beats per
/// minute.
fn midi(bpm: i32, notes: &[(Option<u8>, u32)]) -> Vec<u8> {
    // A MIDI tempo has 24 bits, so below 4 bpm the slowest one plays.
    let quarter_us = (60_000_000 / bpm.max(1) as u32).min(0xFF_FFFF);
    let mut track = vec![0x00, 0xFF, 0x51, 0x03];
    track.extend_from_slice(&quarter_us.to_be_bytes()[1..]);
    track.extend_from_slice(&[0x00, 0xC0, 80]);
    let mut delay = 0;
    for &(note, length) in notes {
        match note {
            None => delay += length,
            Some(note) => {
                put_varint(&mut track, delay);
                track.extend_from_slice(&[0x90, note, 100]);
                put_varint(&mut track, length);
                track.extend_from_slice(&[0x80, note, 0]);
                delay = 0;
            }
        }
    }
    put_varint(&mut track, delay);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);
    let mut file = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    file.extend_from_slice(&(TICKS_PER_QUARTER as u16).to_be_bytes());
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&(track.len() as u32).to_be_bytes());
    file.extend_from_slice(&track);
    file
}

/// What a `MelodyComposer` holds.
struct Composer {
    notes: Vec<(i32, i32)>,
    bpm: i32,
}

impl Default for Composer {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            bpm: BPM,
        }
    }
}

impl Payload for Composer {
    fn heap_bytes(&self) -> usize {
        self.notes.len() * 8
    }
}

/// What a `Melody` holds: its MIDI file.
#[derive(Default)]
struct MelodyData {
    midi: Vec<u8>,
}

impl Payload for MelodyData {
    fn heap_bytes(&self) -> usize {
        self.midi.len()
    }
}

/// The melody playing now, stopped when another starts.
#[derive(Default)]
struct Playing(Option<Player>);

fn composer<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut Composer> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<Composer>(this)
}

fn compose(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let notes = ctx.non_null(a[1])?;
    let notes: Vec<i32> = ctx.array::<i32>(notes)?.to_vec();
    let bpm = a[2].int();
    let c = composer(ctx, a[0])?;
    c.bpm = bpm;
    c.notes = notes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[note, length]| (note, length))
        .take(MAX_NOTES)
        .collect();
    Ok(Value::Void)
}

fn append_note(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (note, length) = (a[1].int(), a[2].int());
    let c = composer(ctx, a[0])?;
    if c.notes.len() >= MAX_NOTES {
        return ctx.throw("java/lang/Exception", "the melody is full");
    }
    c.notes.push((note, length));
    Ok(Value::Void)
}

fn get_melody(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let c = composer(ctx, a[0])?;
    let file = midi(c.bpm, &expand(&c.notes));
    let class = ctx.find_class(MELODY)?;
    let melody = ctx.new_object(class)?;
    ctx.set_payload(melody, MelodyData { midi: file });
    Ok(melody.into())
}

fn play(ctx: &mut Ctx<'_>, this: Value) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let bytes = ctx.payload::<MelodyData>(this)?.midi.clone();
    stop(ctx);
    let Some(player) = clip::open(ctx, bytes, Some(Format::Midi)) else {
        return Ok(Value::Void);
    };
    // A refused start (the instance closing its sound) is silence.
    let _ = player.start();
    ctx.set_extension(Playing(Some(player)));
    Ok(Value::Void)
}

fn stop(ctx: &mut Ctx<'_>) {
    if let Some(Playing(player)) = ctx.extension_mut::<Playing>() {
        // Dropping the player closes it.
        player.take();
    }
}

/// `Melody`, `MelodyComposer` and `Sound`.
pub fn classes() -> Vec<NativeClass> {
    vec![melody(), composer_class(), sound()]
}

fn melody() -> NativeClass {
    NativeClass::new(MELODY)
        .extends(NATIVE_MEM)
        .payload::<MelodyData>()
        .method("play", "()V", |ctx, a| play(ctx, a[0]))
        .static_method("stop", "()V", |ctx, _| {
            stop(ctx);
            Ok(Value::Void)
        })
}

fn composer_class() -> NativeClass {
    let mut class = NativeClass::new(COMPOSER).payload::<Composer>();
    let mut value = 0;
    for octave in 0..5 {
        for name in NOTES {
            if value > TONE_A4 {
                break;
            }
            class = class.constant(&format!("TONE_{name}{octave}"), "I", Value::Int(value));
            value += 1;
        }
    }
    for (i, name) in SPECIAL.iter().enumerate() {
        class = class.constant(name, "I", Value::Int(TONE_PAUSE + i as i32));
    }
    for (i, name) in LENGTHS.iter().enumerate() {
        class = class.constant(&format!("TONELENGTH_{name}"), "I", Value::Int(i as i32));
    }
    class
        .constant("BPM", "I", Value::Int(BPM))
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method("<init>", "([II)V", compose)
        .method("setBPM", "(I)V", |ctx, a| {
            let bpm = a[1].int();
            composer(ctx, a[0])?.bpm = bpm;
            Ok(Value::Void)
        })
        .method("appendNote", "(II)V", append_note)
        .method("getMelody", "()Lcom/siemens/mp/game/Melody;", get_melody)
        .method("resetMelody", "()V", |ctx, a| {
            *composer(ctx, a[0])? = Composer::default();
            Ok(Value::Void)
        })
        .method("length", "()I", |ctx, a| {
            Ok(Value::Int(composer(ctx, a[0])?.notes.len() as i32))
        })
        .static_method("maxLength", "()I", |_, _| Ok(Value::Int(MAX_NOTES as i32)))
}

fn sound() -> NativeClass {
    NativeClass::new("com/siemens/mp/game/Sound")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("playTone", "(II)V", |ctx, a| {
            let (frequency, duration) = (a[0].int(), a[1].int());
            if frequency > 0 && duration > 0 {
                clip::tone(ctx, note_of(frequency), duration);
            }
            Ok(Value::Void)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequencies_map_to_the_nearest_midi_note() {
        assert_eq!(note_of(440), 69);
        assert_eq!(note_of(262), 60);
        assert_eq!(note_of(1000), 83);
    }

    #[test]
    fn lengths_count_ticks_of_a_quarter_of_96() {
        assert_eq!(ticks(0), 384);
        assert_eq!(ticks(2), 96);
        assert_eq!(ticks(6), 6);
        assert_eq!(ticks(9), 144);
        assert_eq!(ticks(13), 9);
        assert_eq!(ticks(99), 96);
    }

    #[test]
    fn repeats_marks_and_stops_expand() {
        let notes = [
            (0, 2),
            (TONE_MARK, 0),
            (9, 3),
            (TONE_PAUSE, 3),
            (TONE_REPEAT_MARK, 1),
            (TONE_REPEAT, 1),
            (TONE_STOP, 0),
            (5, 2),
        ];
        let a = (Some(33), 48);
        let rest = (None, 48);
        assert_eq!(
            expand(&notes),
            [
                (Some(24), 96),
                a,
                rest,
                a,
                rest,
                (Some(24), 96),
                a,
                rest,
                a,
                rest
            ]
        );
        assert!(expand(&[(0, 0), (TONE_REPEAT, i32::MAX)]).len() <= MAX_EVENTS);
    }

    #[test]
    fn a_melody_is_a_midi_file_with_its_tempo() {
        let file = midi(120, &[(None, 96), (Some(60), 200)]);
        assert_eq!(&file[..4], b"MThd");
        assert_eq!(&file[12..14], &96u16.to_be_bytes());
        let track = &file[22..];
        // 500 000 us per quarter note at 120 bpm.
        assert_eq!(&track[..7], &[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
        assert_eq!(&track[7..10], &[0x00, 0xC0, 80]);
        // The rest delays the note on; 200 ticks is two varint bytes.
        assert_eq!(
            &track[10..],
            &[
                0x60, 0x90, 60, 100, 0x81, 0x48, 0x80, 60, 0, 0x00, 0xFF, 0x2F, 0x00
            ]
        );
        let length = u32::from_be_bytes(file[18..22].try_into().unwrap()) as usize;
        assert_eq!(length, track.len());
        let mut varint = Vec::new();
        put_varint(&mut varint, 0x0FFF_FFFF);
        assert_eq!(varint, [0xFF, 0xFF, 0xFF, 0x7F]);
    }

    #[test]
    fn a_tempo_below_four_bpm_is_the_slowest_midi_tempo() {
        let file = midi(1, &[]);
        assert_eq!(&file[22..29], &[0x00, 0xFF, 0x51, 0x03, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn repeating_nothing_returns_at_once() {
        let started = std::time::Instant::now();
        assert!(expand(&[(TONE_REPEAT, i32::MAX)]).is_empty());
        let marked = [(0, 0), (TONE_MARK, 0), (TONE_REPEAT_MARK, i32::MAX)];
        assert_eq!(expand(&marked).len(), 1);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}
