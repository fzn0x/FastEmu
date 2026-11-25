//! Builders for sound files and helpers shared by the integration tests.

#![allow(dead_code)] // Each test file uses its own subset.

use std::sync::mpsc::Receiver;

use fastemu_audio::{Engine, EngineConfig, PlayerEvent, PlayerEventKind};

/// A 16-bit PCM WAV file.
pub fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2 * u32::from(channels)).to_le_bytes());
    out.extend_from_slice(&(2 * channels).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// A mono WAV holding one value for `frames` frames.
pub fn constant_wav(rate: u32, frames: usize, value: i16) -> Vec<u8> {
    wav(rate, 1, &vec![value; frames])
}

/// A MIDI file of one track from raw events (delta times included); the
/// end-of-track event is added.
pub fn midi(division: u16, events: &[u8]) -> Vec<u8> {
    let mut track = events.to_vec();
    track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    let mut out = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    out.extend_from_slice(&division.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

/// A MIDI file of `controllers` volume changes at tick 0, then one note
/// half a second long, at 480 ticks per quarter and 120 beats a minute.
pub fn controller_midi(controllers: usize) -> Vec<u8> {
    let mut events = Vec::with_capacity(controllers * 4 + 16);
    for i in 0..controllers {
        events.extend_from_slice(&[0x00, 0xB0 | (i % 16) as u8, 7, (i % 128) as u8]);
    }
    events.extend_from_slice(&[0x00, 0x90, 60, 100, 0x83, 0x60, 0x80, 60, 0]);
    midi(480, &events)
}

pub fn engine(rate: u32) -> Engine {
    Engine::new(EngineConfig { sample_rate: rate })
}

/// Renders `frames` frames and returns the left channel.
pub fn render(engine: &Engine, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    engine.render(&mut out);
    out.iter().step_by(2).copied().collect()
}

pub fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

pub fn kinds(events: &Receiver<PlayerEvent>) -> Vec<PlayerEventKind> {
    events.try_iter().map(|e| e.kind).collect()
}

/// A minimal SoundFont 2 bank: one preset (bank 0, program 0) whose one
/// instrument loops a sine sample at middle C over every key.
pub fn soundfont() -> Vec<u8> {
    fn chunk(id: [u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = id.to_vec();
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }
    fn list(kind: [u8; 4], chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut body = kind.to_vec();
        for c in chunks {
            body.extend_from_slice(c);
        }
        chunk(*b"LIST", &body)
    }
    fn name(text: &str, len: usize) -> Vec<u8> {
        let mut out = text.as_bytes().to_vec();
        out.resize(len, 0);
        out
    }
    let le16 = |v: u16| v.to_le_bytes().to_vec();
    let le32 = |v: u32| v.to_le_bytes().to_vec();

    // 100 samples of one sine cycle at 26163 Hz plays middle C (261.63 Hz).
    let cycle = 100u32;
    let rate = 26163u32;
    let mut samples: Vec<i16> = (0..cycle * 10)
        .map(|i| ((f64::from(i) / f64::from(cycle) * std::f64::consts::TAU).sin() * 20000.0) as i16)
        .collect();
    let end = samples.len() as u32;
    samples.extend(std::iter::repeat_n(0, 46));
    let smpl: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

    let info = list(
        *b"INFO",
        &[
            chunk(*b"ifil", &[le16(2), le16(1)].concat()),
            chunk(*b"isng", b"EMU8000\0"),
            chunk(*b"INAM", b"FastEmu test\0\0"),
        ],
    );
    let sdta = list(*b"sdta", &[chunk(*b"smpl", &smpl)]);

    let phdr = [
        [
            name("Sine", 20),
            le16(0),
            le16(0),
            le16(0),
            le32(0),
            le32(0),
            le32(0),
        ]
        .concat(),
        [
            name("EOP", 20),
            le16(0),
            le16(0),
            le16(1),
            le32(0),
            le32(0),
            le32(0),
        ]
        .concat(),
    ]
    .concat();
    let pbag = [le16(0), le16(0), le16(1), le16(0)].concat();
    let pmod = vec![0u8; 10];
    // Preset zone: instrument 0.
    let pgen = [le16(41), le16(0), le16(0), le16(0)].concat();
    let inst = [
        [name("Sine", 20), le16(0)].concat(),
        [name("EOI", 20), le16(1)].concat(),
    ]
    .concat();
    let ibag = [le16(0), le16(0), le16(2), le16(0)].concat();
    let imod = vec![0u8; 10];
    // Instrument zone: loop continuously, then the sample (always last).
    let igen = [le16(54), le16(1), le16(53), le16(0), le16(0), le16(0)].concat();
    let shdr = [
        [
            name("Sine", 20),
            le32(0),
            le32(end),
            le32(0),
            le32(end),
            le32(rate),
            vec![60, 0],
            le16(0),
            le16(1),
        ]
        .concat(),
        [name("EOS", 20), vec![0; 26]].concat(),
    ]
    .concat();
    let pdta = list(
        *b"pdta",
        &[
            chunk(*b"phdr", &phdr),
            chunk(*b"pbag", &pbag),
            chunk(*b"pmod", &pmod),
            chunk(*b"pgen", &pgen),
            chunk(*b"inst", &inst),
            chunk(*b"ibag", &ibag),
            chunk(*b"imod", &imod),
            chunk(*b"igen", &igen),
            chunk(*b"shdr", &shdr),
        ],
    );
    let mut body = b"sfbk".to_vec();
    body.extend_from_slice(&info);
    body.extend_from_slice(&sdta);
    body.extend_from_slice(&pdta);
    chunk(*b"RIFF", &body)
}
