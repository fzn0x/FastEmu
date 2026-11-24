//! MIDI through the engine: note timing measured on rendered sound, loops,
//! seeking, and SoundFont playback.

mod common;

use common::{engine, kinds, midi, render, soundfont};
use fastemu_audio::{AudioError, Engine, Format, Group, Player, PlayerEventKind, SoundFont};

const RATE: u32 = 16_000;

/// Flute (a sine with a 30 ms attack): note on at 0, 0.5 and 1.0 s,
/// each a quarter second long, and the track ends at 1.5 s.
fn three_notes() -> Vec<u8> {
    midi(
        96,
        &[
            0x00, 0xC0, 73, //
            0x00, 0x90, 60, 100, //
            0x30, 0x80, 60, 0, //
            0x30, 0x90, 64, 100, //
            0x30, 0x80, 64, 0, //
            0x30, 0x90, 67, 100, //
            0x30, 0x80, 67, 0, //
            0x30, 0xFF, 0x01, 0x00, // an empty text event to mark the end
        ],
    )
}

/// The RMS level of each 10 ms window.
fn levels(samples: &[f32]) -> Vec<f32> {
    samples
        .chunks(RATE as usize / 100)
        .map(|w| (w.iter().map(|v| v * v).sum::<f32>() / w.len() as f32).sqrt())
        .collect()
}

/// The times in milliseconds where the level rises from quiet to loud.
fn onsets_ms(samples: &[f32]) -> Vec<usize> {
    let levels = levels(samples);
    let loudest = levels.iter().fold(0.0f32, |m, &v| m.max(v));
    let mut onsets = Vec::new();
    let mut quiet = true;
    for (i, &level) in levels.iter().enumerate() {
        if quiet && level > loudest * 0.2 {
            onsets.push(i * 10);
            quiet = false;
        } else if level < loudest * 0.02 {
            quiet = true;
        }
    }
    onsets
}

/// Plays `data` from the start and renders two seconds.
fn play(engine: &Engine, data: &[u8]) -> (Group, Player, Vec<f32>) {
    let group = engine.create_group();
    let player = group.create_player(data, Some("audio/midi")).unwrap();
    assert_eq!(player.format(), Format::Midi);
    player.start().unwrap();
    let heard = render(engine, RATE as usize * 2);
    (group, player, heard)
}

#[test]
fn notes_start_on_time() {
    let engine = engine(RATE);
    let (_group, player, heard) = play(&engine, &three_notes());
    assert_eq!(player.duration().unwrap(), 1_500_000);
    let onsets = onsets_ms(&heard);
    assert_eq!(onsets.len(), 3, "{onsets:?}");
    for (onset, want) in onsets.iter().zip([0, 500, 1000]) {
        assert!(onset.abs_diff(want) <= 30, "{onsets:?}");
    }
    // Nothing sounds after the end of the media.
    let after = &heard[RATE as usize * 3 / 2 + 160..];
    assert!(after.iter().all(|&v| v == 0.0));
}

#[test]
fn tempo_changes_move_the_notes() {
    // 120 bpm, then 60 bpm from tick 96: the third note moves from 1.0 s
    // to 1.5 s.
    let data = midi(
        96,
        &[
            0x00, 0xC0, 73, //
            0x00, 0x90, 60, 100, //
            0x30, 0x80, 60, 0, //
            0x30, 0xFF, 0x51, 0x03, 0x0F, 0x42, 0x40, //
            0x00, 0x90, 64, 100, //
            0x18, 0x80, 64, 0, //
            0x48, 0x90, 67, 100, //
            0x18, 0x80, 67, 0,
        ],
    );
    let engine = engine(RATE);
    let group = engine.create_group();
    let player = group.create_player(&data, None).unwrap();
    assert_eq!(player.duration().unwrap(), 1_750_000);
    player.start().unwrap();
    let onsets = onsets_ms(&render(&engine, RATE as usize * 2));
    assert_eq!(onsets.len(), 3, "{onsets:?}");
    assert!(onsets[1].abs_diff(500) <= 30, "{onsets:?}");
    assert!(onsets[2].abs_diff(1500) <= 30, "{onsets:?}");
}

#[test]
fn midi_loops_and_seeks_by_media_time() {
    let engine = engine(RATE);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group.create_player(&three_notes(), None).unwrap();
    player.set_loop_count(2).unwrap();
    player.start().unwrap();
    let heard = render(&engine, RATE as usize * 4);
    let onsets = onsets_ms(&heard);
    assert_eq!(onsets.len(), 6, "{onsets:?}");
    assert!(onsets[3].abs_diff(1500) <= 30, "{onsets:?}");
    let end = PlayerEventKind::EndOfMedia {
        media_time_us: 1_500_000,
    };
    assert_eq!(
        kinds(&events),
        vec![
            PlayerEventKind::Started { media_time_us: 0 },
            end.clone(),
            PlayerEventKind::Started { media_time_us: 0 },
            end
        ]
    );
    // Seek to the third note and play from there, once.
    player.set_loop_count(1).unwrap();
    assert_eq!(player.set_media_time(1_000_000).unwrap(), 1_000_000);
    player.start().unwrap();
    let onsets = onsets_ms(&render(&engine, RATE as usize));
    assert!(onsets[0] <= 30, "{onsets:?}");
    assert_eq!(onsets.len(), 1);
}

#[test]
fn midi_control_messages_reach_the_synthesizer() {
    let engine = engine(RATE);
    let group = engine.create_group();
    // One long note.
    let data = midi(
        96,
        &[0x00, 0xC0, 73, 0x00, 0x90, 69, 100, 0x83, 0x00, 0x80, 69, 0],
    );
    let player = group.create_player(&data, None).unwrap();
    // JSR-135 wants the player prefetched first.
    assert!(matches!(
        player.midi_short_event(0xB0, 7, 20),
        Err(AudioError::InvalidState(_))
    ));
    player.start().unwrap();
    let loud = common::peak(&render(&engine, 4000)[2000..]);
    // Channel volume down through MIDIControl.
    player.midi_short_event(0xB0, 7, 20).unwrap();
    let quiet = common::peak(&render(&engine, 4000)[2000..]);
    assert!(quiet < loud * 0.1, "{quiet} {loud}");
    // Any status from 0x80 to 0xFF but the two system exclusive bytes;
    // system messages change nothing here.
    assert!(player.midi_short_event(0xF0, 0, 0).is_err());
    assert!(player.midi_short_event(0xF7, 0, 0).is_err());
    assert!(player.midi_short_event(0x7F, 0, 0).is_err());
    player.midi_short_event(0xF8, 0, 0).unwrap();
    player.midi_short_event(0xFF, 0, 0).unwrap();
    assert!(player.midi_short_event(0x90, 128, 0).is_err());
}

#[test]
fn a_loaded_soundfont_plays_midi() {
    let font = SoundFont::from_bytes(&soundfont()).unwrap();
    assert_eq!(font.name(), "FastEmu test");
    assert_eq!(font.preset_count(), 1);
    let engine = engine(RATE);
    engine.set_soundfont(Some(font));
    let group = engine.create_group();
    // Middle C for a second.
    let data = midi(96, &[0x00, 0x90, 60, 100, 0x81, 0x40, 0x80, 60, 0]);
    let player = group.create_player(&data, None).unwrap();
    player.start().unwrap();
    let heard = render(&engine, RATE as usize);
    let middle = &heard[RATE as usize / 10..RATE as usize * 9 / 10];
    assert!(common::peak(middle) > 0.05);
    // 0.8 s of 261.6 Hz rises through zero about 209 times.
    let crossings = middle
        .windows(2)
        .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
        .count();
    assert!(crossings.abs_diff(209) <= 3, "{crossings}");
    // Players made after clearing it use the built-in synthesizer.
    engine.set_soundfont(None);
    assert!(engine.soundfont().is_none());
}
