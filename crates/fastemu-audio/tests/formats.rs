//! Every format through the engine, content types that lie, tone
//! sequences, and the Nokia sound API.

mod common;

use amr_nb::{Dtx, Encoder, L_FRAME, Mode, rfc4867};
use common::{constant_wav, engine, peak, render};
use fastemu_audio::{AudioError, Format, PlayerState, SUPPORTED_CONTENT_TYPES};

const MP3: &[u8] = include_bytes!("data/sine-440hz-1s.mp3");

fn rising_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
        .count()
}

#[test]
fn the_bytes_win_over_a_wrong_content_type() {
    let engine = engine(8000);
    let group = engine.create_group();
    let wav = constant_wav(8000, 100, 1000);
    let player = group.create_player(&wav, Some("audio/midi")).unwrap();
    assert_eq!(player.format(), Format::Wav);
    let player = group.create_player(MP3, Some("audio/x-wav")).unwrap();
    assert_eq!(player.format(), Format::Mp3);
    // Without a signature the declared type decides.
    let tones = [0xFE, 0x01, 60, 8];
    let player = group
        .create_player(&tones, Some("audio/x-tone-seq"))
        .unwrap();
    assert_eq!(player.format(), Format::ToneSequence);
    assert!(matches!(
        group.create_player(b"garbage", Some("audio/x-unknown")),
        Err(AudioError::Unsupported(_))
    ));
    assert!(matches!(
        group.create_player(b"garbage", None),
        Err(AudioError::Unsupported(_))
    ));
    assert!(SUPPORTED_CONTENT_TYPES.contains(&"audio/amr"));
}

#[test]
fn broken_data_is_reported_at_creation() {
    let engine = engine(8000);
    let group = engine.create_group();
    let mut wav = constant_wav(8000, 10, 1);
    wav.truncate(30);
    assert!(matches!(
        group.create_player(&wav, None),
        Err(AudioError::Malformed {
            format: Format::Wav,
            ..
        })
    ));
    assert!(matches!(
        group.create_player(b"MThd\0\0\0\x06\0\0\0\0\0\x60", None),
        Err(AudioError::Malformed {
            format: Format::Midi,
            ..
        })
    ));
    assert_eq!(engine.player_count(), 0);
}

#[test]
fn mp3_plays_at_its_pitch() {
    let engine = engine(44_100);
    let group = engine.create_group();
    let player = group.create_player(MP3, Some("audio/mpeg")).unwrap();
    assert_eq!(player.duration().unwrap(), 1_000_000);
    player.start().unwrap();
    let heard = render(&engine, 44_100 + 1000);
    // 0.8 s of 440 Hz.
    let crossings = rising_crossings(&heard[4410..39_690]);
    assert!(crossings.abs_diff(352) <= 3, "{crossings}");
    assert_eq!(player.state(), PlayerState::Prefetched);
    assert_eq!(player.media_time().unwrap(), 1_000_000);
}

#[test]
fn amr_plays_at_its_pitch() {
    let pcm: Vec<i16> = (0..8000)
        .map(|i| ((i as f32 / 8000.0 * 500.0 * std::f32::consts::TAU).sin() * 8000.0) as i16)
        .collect();
    let mut encoder = Encoder::new(Dtx::Off);
    let frames: Vec<rfc4867::Frame> = pcm
        .as_chunks::<L_FRAME>()
        .0
        .iter()
        .map(|chunk| encoder.encode(Mode::MR122, chunk))
        .collect();
    let file = rfc4867::write_storage(&frames).unwrap();
    let engine = engine(16_000);
    let group = engine.create_group();
    let player = group.create_player(&file, Some("audio/amr")).unwrap();
    assert_eq!(player.format(), Format::Amr);
    assert_eq!(player.duration().unwrap(), 1_000_000);
    player.start().unwrap();
    let heard = render(&engine, 16_000);
    let crossings = rising_crossings(&heard[1600..14_400]);
    assert!(crossings.abs_diff(400) <= 6, "{crossings}");
}

#[test]
fn wav_variants_play() {
    let engine = engine(8000);
    let group = engine.create_group();
    // 8-bit unsigned mono, then 16-bit stereo with the sides apart.
    let mut eight = constant_wav(8000, 1, 0);
    eight.truncate(44);
    eight[22] = 1;
    eight[34] = 8;
    eight[32] = 1;
    eight[40..44].copy_from_slice(&100u32.to_le_bytes());
    eight.extend(std::iter::repeat_n(200u8, 100));
    let player = group.create_player(&eight, None).unwrap();
    assert_eq!(player.duration().unwrap(), 12_500);
    let stereo = common::wav(8000, 2, &[8000, -8000].repeat(100));
    let player = group.create_player(&stereo, None).unwrap();
    player.start().unwrap();
    let mut out = vec![0.0; 2 * 100];
    engine.render(&mut out);
    assert!(out[160] > 0.1 && out[161] < -0.1);
}

#[test]
fn tone_sequences_play_and_report_bad_bytes() {
    let engine = engine(8000);
    let group = engine.create_group();
    // VERSION 1, TEMPO 30 (120 bpm), A4 for a quarter, a rest, A5.
    let sequence = [0xFE, 1, 0xFD, 30, 69, 16, 0xFF, 16, 81, 16];
    let player = group
        .create_player(&sequence, Some("audio/x-tone-seq"))
        .unwrap();
    assert_eq!(player.duration().unwrap(), 1_500_000);
    player.start().unwrap();
    let heard = render(&engine, 12_400);
    assert!(rising_crossings(&heard[400..3600]).abs_diff(176) <= 2);
    assert_eq!(peak(&heard[4400..7600]), 0.0);
    assert!(rising_crossings(&heard[8400..11_600]).abs_diff(352) <= 2);
    // A bad sequence says where it breaks.
    assert_eq!(
        group
            .create_player(&[0xFE, 1, 69, 0], Some("audio/x-tone-seq"))
            .unwrap_err(),
        AudioError::InvalidToneSequence {
            offset: 3,
            reason: "duration 0 is out of range".into()
        }
    );
}

#[test]
fn the_tone_device_takes_a_sequence_while_realized() {
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group.create_tone_player().unwrap();
    assert_eq!(player.duration().unwrap(), 0);
    assert!(matches!(
        player.set_tone_sequence(&[0xFE, 1, 200, 8]),
        Err(AudioError::InvalidToneSequence { offset: 2, .. })
    ));
    player.set_tone_sequence(&[0xFE, 1, 60, 64]).unwrap();
    assert_eq!(player.duration().unwrap(), 2_000_000);
    player.prefetch().unwrap();
    assert!(matches!(
        player.set_tone_sequence(&[0xFE, 1, 60, 8]),
        Err(AudioError::InvalidState(_))
    ));
    let wav = group
        .create_player(&constant_wav(8000, 10, 0), None)
        .unwrap();
    assert!(matches!(
        wav.set_tone_sequence(&[0xFE, 1, 60, 8]),
        Err(AudioError::InvalidState(_))
    ));
    assert!(matches!(
        wav.midi_short_event(0x90, 60, 100),
        Err(AudioError::InvalidState(_))
    ));
}

#[test]
fn nokia_ringing_tones_and_beeps_play() {
    // Two commands: ringing tone programming, then a basic song "N" of one
    // pattern holding one A at the default scale (880 Hz), a quarter note
    // at the default 63 bpm, continuous style.
    let mut bits = String::new();
    let mut put = |value: u32, count: usize| {
        for i in (0..count).rev() {
            bits.push(if (value >> i) & 1 == 1 { '1' } else { '0' });
        }
    };
    put(2, 8);
    put(0b010_0101, 7);
    put(0, 1);
    put(0b001_1101, 7);
    put(0b001, 3);
    put(1, 4);
    put(u32::from(b'N'), 8);
    put(1, 8);
    put(0, 3);
    put(0, 2);
    put(0, 4);
    put(2, 8);
    put(0b011, 3);
    put(1, 2);
    put(0b001, 3);
    put(10, 4);
    put(2, 3);
    put(0, 2);
    while !bits.len().is_multiple_of(8) {
        bits.push('0');
    }
    let data: Vec<u8> = bits
        .as_bytes()
        .chunks(8)
        .map(|byte| u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 2).unwrap())
        .collect();
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group
        .create_player_as(&data, Format::NokiaRingtone)
        .unwrap();
    // A quarter note at 63 bpm.
    assert_eq!(player.duration().unwrap(), 952_375);
    player.start().unwrap();
    let heard = render(&engine, 8000);
    assert!(rising_crossings(&heard[800..4800]).abs_diff(440) <= 3);

    let beep = group.create_beep_player(1000, 250).unwrap();
    assert_eq!(beep.duration().unwrap(), 250_000);
    assert!(group.create_beep_player(-1, 250).is_err());
    assert!(group.create_beep_player(1000, 0).is_err());
}
