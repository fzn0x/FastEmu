//! Players (`docs/compat/notes/07-sound.md`): WAV, MIDI, a tone sequence,
//! MP3, AMR and data that is no sound at all, each from a stream with its
//! content type and with none, and two locators, all playing at once in
//! an instance without an audio device. Each must report the format its
//! bytes show, its length, and the listener events `started`,
//! `endOfMedia` and `closed`; data FastEmu cannot play must still end.

use crate::conformance_support::{Jar, compile, manifest, scratch, tagged};
use crate::support::Running;

/// The sample rate of the test WAV.
const RATE: u32 = 8000;

/// A 16-bit mono WAV of a 440 Hz sine, `ms` long.
fn wav(ms: u32) -> Vec<u8> {
    let data: Vec<u8> = (0..RATE * ms / 1000)
        .map(|i| {
            let phase = f64::from(i) * 440.0 / f64::from(RATE) * std::f64::consts::TAU;
            (phase.sin() * 12000.0) as i16
        })
        .flat_map(i16::to_le_bytes)
        .collect();
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// A MIDI file of one middle C, one beat at the default tempo.
fn midi() -> Vec<u8> {
    let track = [
        0x00, 0xC0, 0x00, // piano
        0x00, 0x90, 60, 100, // note on
        0x83, 0x60, 0x80, 60, 0, // 480 ticks later, note off
        0x00, 0xFF, 0x2F, 0x00, // end of track
    ];
    let mut out = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    out.extend_from_slice(&480u16.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

/// A tone sequence: version 1, 120 beats a minute, one quarter note.
const TONE: [u8; 6] = [0xFE, 1, 0xFD, 30, 60, 16];

/// One second of AMR-NB at 12.2 kbit/s: 50 frames of silence-like bits.
fn amr() -> Vec<u8> {
    let mut out = b"#!AMR\n".to_vec();
    for _ in 0..50 {
        out.push(0x3C);
        out.extend_from_slice(&[0; 31]);
    }
    out
}

/// What one player must report.
struct Want {
    label: &'static str,
    content_type: &'static str,
    /// The exact duration in microseconds, or `None` for any length.
    duration: Option<i64>,
}

const fn want(label: &'static str, content_type: &'static str, duration: Option<i64>) -> Want {
    Want {
        label,
        content_type,
        duration,
    }
}

const WANT: [Want; 14] = [
    want("beep.wav as audio/x-wav", "audio/x-wav", Some(200_000)),
    want("beep.wav as null", "audio/x-wav", Some(200_000)),
    want("tune.mid as audio/midi", "audio/midi", None),
    want("tune.mid as null", "audio/midi", None),
    want(
        "tone.jts as audio/x-tone-seq",
        "audio/x-tone-seq",
        Some(500_000),
    ),
    want("tone.jts as null", "audio/x-tone-seq", Some(500_000)),
    want("sine.mp3 as audio/mpeg", "audio/mpeg", None),
    want("sine.mp3 as null", "audio/mpeg", None),
    want("speech.amr as audio/amr", "audio/amr", Some(1_000_000)),
    want("speech.amr as null", "audio/amr", Some(1_000_000)),
    // Data FastEmu cannot play: a silent second with the type the game
    // gave, or none.
    want(
        "junk.bin as audio/x-unknown",
        "audio/x-unknown",
        Some(1_000_000),
    ),
    want(
        "junk.bin as null",
        "application/octet-stream",
        Some(1_000_000),
    ),
    want("resource:/beep.wav", "audio/x-wav", Some(200_000)),
    want("/tune.mid", "audio/midi", None),
];

#[test]
fn every_format_plays_to_its_end_with_and_without_a_type() {
    let Some(classes) = compile("media") else {
        return;
    };
    let dir = scratch("media");
    let jar = Jar::new()
        .manifest(manifest("p.Players", ""))
        .classes(&classes)
        .deflated("beep.wav", wav(200))
        .stored("tune.mid", midi())
        .stored("tone.jts", TONE.to_vec())
        .stored(
            "sine.mp3",
            include_bytes!("../../../fastemu-audio/tests/data/sine-440hz-1s.mp3").to_vec(),
        )
        .stored("speech.amr", amr())
        .stored("junk.bin", b"this is not a sound at all".to_vec())
        .write(&dir.join("media.jar"));
    let mut run = Running::launch(&jar, 0);
    run.wait_log("players done");
    let log = run.lines();
    let players = tagged(&log, "player ");
    let dump = || log.join("\n");
    assert_eq!(players.len(), WANT.len(), "{}", dump());
    for (line, want) in players.iter().zip(&WANT) {
        let rest = line
            .strip_prefix(want.label)
            .and_then(|r| r.strip_prefix(": type "))
            .unwrap_or_else(|| panic!("{line:?} is not {}\n{}", want.label, dump()));
        let (content_type, rest) = rest.split_once(" duration ").expect("a duration");
        let (duration, events) = rest.split_once(" events ").expect("events");
        assert_eq!(content_type, want.content_type, "{line}\n{}", dump());
        let duration: i64 = duration.parse().expect("a number of microseconds");
        match want.duration {
            Some(exact) => assert_eq!(duration, exact, "{line}"),
            None => assert!(duration > 0, "{line}"),
        }
        assert_eq!(events, "started endOfMedia closed", "{line}\n{}", dump());
    }
    assert_eq!(run.instance.error_count(), 0, "{}", dump());
}
