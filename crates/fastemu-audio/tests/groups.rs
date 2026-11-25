//! Groups: one per instance, with volume, mute, pause, and closing that
//! stops a game's sound.

mod common;

use common::{constant_wav, engine, kinds, peak, render};
use fastemu_audio::{AudioError, GroupLimits, PlayerEventKind, PlayerState};

#[test]
fn muting_one_group_leaves_the_others_playing() {
    let engine = engine(8000);
    let first = engine.create_group();
    let second = engine.create_group();
    let a = first
        .create_player(&constant_wav(8000, 8000, 8000), None)
        .unwrap();
    let b = second
        .create_player(&constant_wav(8000, 8000, 8000), None)
        .unwrap();
    a.start().unwrap();
    b.start().unwrap();
    let both = peak(&render(&engine, 200)[100..]);
    first.set_muted(true);
    assert!(first.is_muted());
    let one = peak(&render(&engine, 200)[100..]);
    assert!((one - both / 2.0).abs() < 1e-3, "{one} {both}");
    // The muted game's player still moves through time.
    assert_eq!(a.media_time().unwrap(), b.media_time().unwrap());
    second.set_muted(true);
    assert_eq!(peak(&render(&engine, 200)[100..]), 0.0);
    first.set_muted(false);
    second.set_muted(false);
    assert!((peak(&render(&engine, 200)[100..]) - both).abs() < 1e-3);
}

#[test]
fn group_volume_scales_its_players() {
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group
        .create_player(&constant_wav(8000, 8000, 16000), None)
        .unwrap();
    player.start().unwrap();
    let full = peak(&render(&engine, 200)[100..]);
    group.set_volume(0.25);
    assert_eq!(group.volume(), 0.25);
    let quarter = peak(&render(&engine, 200)[100..]);
    assert!((quarter - full * 0.25).abs() < 1e-3);
    group.set_volume(7.0);
    assert_eq!(group.volume(), 1.0);
}

/// The volume is kept outside the mixer, so a closed group still
/// answers with the last one set; `is_closed` tells it is closed.
#[test]
fn a_closed_group_keeps_its_last_volume() {
    let engine = engine(8000);
    let group = engine.create_group();
    group.set_volume(0.25);
    group.close();
    assert!(group.is_closed());
    assert_eq!(group.volume(), 0.25);
    // Setting it on a closed group is harmless and changes no sound.
    group.set_volume(0.5);
    assert_eq!(group.volume(), 0.5);
    assert_eq!(peak(&render(&engine, 100)), 0.0);
}

#[test]
fn a_volume_that_is_not_a_number_is_silence() {
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group
        .create_player(&constant_wav(8000, 8000, 16000), None)
        .unwrap();
    player.start().unwrap();
    group.set_volume(f32::NAN);
    assert_eq!(group.volume(), 0.0);
    assert_eq!(peak(&render(&engine, 200)[100..]), 0.0);
    group.set_volume(1.0);
    engine.set_master_volume(f32::NAN);
    assert_eq!(engine.master_volume(), 0.0);
    assert_eq!(peak(&render(&engine, 200)[100..]), 0.0);
}

#[test]
fn pausing_a_group_freezes_its_time() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 800, 8000), None)
        .unwrap();
    player.start().unwrap();
    render(&engine, 200);
    group.set_paused(true);
    assert!(group.is_paused());
    let paused_at = player.media_time().unwrap();
    assert_eq!(paused_at, 25_000);
    let heard = render(&engine, 2000);
    assert_eq!(peak(&heard[100..]), 0.0);
    assert_eq!(player.media_time().unwrap(), paused_at);
    assert_eq!(player.state(), PlayerState::Started);
    group.set_paused(false);
    render(&engine, 1000);
    assert_eq!(player.state(), PlayerState::Prefetched);
    // Pausing sends no events of its own.
    assert_eq!(
        kinds(&events),
        vec![
            PlayerEventKind::Started { media_time_us: 0 },
            PlayerEventKind::EndOfMedia {
                media_time_us: 100_000,
                last: true,
            }
        ]
    );
}

#[test]
fn a_paused_group_stays_silent_when_its_players_change() {
    let engine = engine(8000);
    let group = engine.create_group();
    let players: Vec<_> = (0..3)
        .map(|_| {
            let player = group
                .create_player(&constant_wav(8000, 8000, 8000), None)
                .unwrap();
            player.start().unwrap();
            player
        })
        .collect();
    render(&engine, 200);
    group.set_paused(true);
    render(&engine, 200);
    // Stopping, seeking or closing a paused player has nothing to fade.
    players[0].stop().unwrap();
    players[1].set_media_time(500_000).unwrap();
    players[2].close();
    assert_eq!(peak(&render(&engine, 200)), 0.0);
}

#[test]
fn stop_all_stops_every_player_of_the_group() {
    let engine = engine(8000);
    let group = engine.create_group();
    let other = engine.create_group();
    let events = group.subscribe();
    let players: Vec<_> = (0..3)
        .map(|_| {
            let p = group
                .create_player(&constant_wav(8000, 8000, 100), None)
                .unwrap();
            p.start().unwrap();
            p
        })
        .collect();
    let keeps = other
        .create_player(&constant_wav(8000, 8000, 100), None)
        .unwrap();
    keeps.start().unwrap();
    group.play_tone(60, 1000, 100).unwrap();
    render(&engine, 80);
    group.stop_all();
    for p in &players {
        assert_eq!(p.state(), PlayerState::Prefetched);
        assert_eq!(p.media_time().unwrap(), 10_000);
    }
    assert_eq!(keeps.state(), PlayerState::Started);
    assert_eq!(engine.playing_count(), 1);
    let stopped = kinds(&events)
        .into_iter()
        .filter(|k| matches!(k, PlayerEventKind::Stopped { .. }))
        .count();
    assert_eq!(stopped, 3);
}

#[test]
fn closing_a_group_closes_its_players() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 8000, 100), None)
        .unwrap();
    let idle = group
        .create_player(&constant_wav(8000, 8000, 100), None)
        .unwrap();
    player.start().unwrap();
    group.play_tone(70, 500, 50).unwrap();
    assert_eq!(group.player_count(), 2);
    assert_eq!(engine.playing_count(), 2);
    group.close();
    assert!(group.is_closed());
    assert_eq!(engine.player_count(), 0);
    assert_eq!(engine.playing_count(), 0);
    assert_eq!(player.state(), PlayerState::Closed);
    assert_eq!(idle.start(), Err(AudioError::Closed));
    let closed = kinds(&events)
        .into_iter()
        .filter(|k| *k == PlayerEventKind::Closed)
        .count();
    assert_eq!(closed, 2);
    assert!(matches!(
        group.create_player(&constant_wav(8000, 10, 1), None),
        Err(AudioError::Closed)
    ));
    assert_eq!(group.play_tone(60, 100, 100), Err(AudioError::Closed));
    // Closing twice is harmless.
    group.close();
}

#[test]
fn dropping_the_last_group_handle_closes_it() {
    let engine = engine(8000);
    let group = engine.create_group();
    let clone = group.clone();
    let player = group
        .create_player(&constant_wav(8000, 8000, 100), None)
        .unwrap();
    player.start().unwrap();
    drop(group);
    assert_eq!(player.state(), PlayerState::Started);
    drop(clone);
    assert_eq!(player.state(), PlayerState::Closed);
    assert_eq!(engine.player_count(), 0);
}

#[test]
fn play_tone_checks_its_arguments_and_cleans_up() {
    let engine = engine(8000);
    let group = engine.create_group();
    assert!(matches!(
        group.play_tone(128, 100, 100),
        Err(AudioError::InvalidArgument(_))
    ));
    assert!(matches!(
        group.play_tone(-1, 100, 100),
        Err(AudioError::InvalidArgument(_))
    ));
    assert!(matches!(
        group.play_tone(60, 0, 100),
        Err(AudioError::InvalidArgument(_))
    ));
    // Volume is clamped rather than refused.
    group.play_tone(69, 100, 400).unwrap();
    assert_eq!(engine.playing_count(), 1);
    assert_eq!(engine.player_count(), 0);
    // 100 ms is 800 frames; the end is noticed on the frame after.
    let heard = render(&engine, 900);
    assert!(peak(&heard) > 0.1);
    assert_eq!(engine.playing_count(), 0);
    // Tones fired in a tight loop are capped.
    for _ in 0..100 {
        group.play_tone(72, 1000, 100).unwrap();
    }
    assert!(engine.playing_count() <= 16);
    assert!(render(&engine, 1000).iter().all(|v| v.abs() <= 1.0));
}

#[test]
fn listeners_owning_handles_are_dropped_without_deadlock() {
    // Runs on a thread so a deadlock fails the test instead of hanging it.
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let engine = engine(8000);
        let group = engine.create_group();
        let player = group
            .create_player(&constant_wav(8000, 80, 1), None)
            .unwrap();
        let held = std::sync::Mutex::new(Some(player));
        let me = group.clone();
        group.set_listener(Some(std::sync::Arc::new(move |_| {
            let _ = (&held, &me);
        })));
        // Replacing the listener drops the player it owned.
        group.set_listener(None);
        assert_eq!(engine.player_count(), 0);
        // A listener owning the group itself goes when the group closes.
        let me = group.clone();
        group.set_listener(Some(std::sync::Arc::new(move |_| {
            let _ = &me;
        })));
        group.close();
        done.send(()).unwrap();
    });
    assert!(
        finished
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok()
    );
}

/// The limits of a game with room for 64 KiB of sound and 4 players.
fn small() -> GroupLimits {
    GroupLimits {
        max_bytes: 64 * 1024,
        max_players: 4,
    }
}

#[test]
fn a_group_holds_only_so_many_players() {
    let engine = engine(8000);
    let group = engine.create_group();
    assert_eq!(group.limits(), GroupLimits::default());
    assert_eq!(group.limits().max_players, 128);
    assert_eq!(group.limits().max_bytes, 32 * 1024 * 1024);
    group.set_limits(small());
    assert_eq!(group.limits(), small());
    let sound = constant_wav(8000, 100, 1);
    let players: Vec<_> = (0..4)
        .map(|_| group.create_player(&sound, None).unwrap())
        .collect();
    // A fifth player of any kind is refused, and nothing is left of it.
    let refused = group.create_player(&sound, None).unwrap_err();
    assert!(matches!(refused, AudioError::OutOfMemory(_)), "{refused}");
    assert!(matches!(
        group.create_tone_player(),
        Err(AudioError::OutOfMemory(_))
    ));
    assert!(matches!(
        group.create_beep_player(440, 100),
        Err(AudioError::OutOfMemory(_))
    ));
    assert_eq!(engine.player_count(), 4);
    // `playTone` tones have no handle and a cap of their own.
    group.play_tone(69, 100, 100).unwrap();
    // Another game in the engine has its own room.
    let other = engine.create_group();
    let _others = other.create_player(&sound, None).unwrap();
    // Closing one player makes room for one.
    players[0].close();
    let _again = group.create_player(&sound, None).unwrap();
    assert_eq!(group.player_count(), 4);
}

#[test]
fn a_group_holds_only_so_much_sound_and_closing_frees_it() {
    let engine = engine(8000);
    let group = engine.create_group();
    group.set_limits(small());
    // 20000 samples are 40000 bytes of the 65536.
    let sound = constant_wav(8000, 20_000, 1);
    let first = group.create_player(&sound, None).unwrap();
    assert_eq!(group.memory_bytes(), 40_000);
    let refused = group.create_player(&sound, None).unwrap_err();
    assert!(matches!(refused, AudioError::OutOfMemory(_)), "{refused}");
    assert_eq!(group.memory_bytes(), 40_000);
    assert_eq!(group.player_count(), 1);
    // A sound that could never fit says so.
    let huge = constant_wav(8000, 40_000, 1);
    let refused = group.create_player(&huge, None).unwrap_err();
    assert!(matches!(refused, AudioError::TooLarge(_)), "{refused}");
    // Closing the first gives its bytes back, and the second then fits.
    first.close();
    assert_eq!(group.memory_bytes(), 0);
    let second = group.create_player(&sound, None).unwrap();
    assert_eq!(group.memory_bytes(), 40_000);
    // So does dropping a player, or closing the whole group.
    drop(second);
    assert_eq!(group.memory_bytes(), 0);
    let third = group.create_player(&sound, None).unwrap();
    group.close();
    assert_eq!(
        group.create_player(&sound, None).unwrap_err(),
        AudioError::Closed
    );
    drop(third);
}

#[test]
fn every_kind_of_sound_counts_what_it_holds() {
    let engine = engine(8000);
    let group = engine.create_group();
    let mp3 = include_bytes!("data/sine-440hz-1s.mp3");
    let player = group.create_player(mp3, None).unwrap();
    // The file stays, and the frames are decoded as they play.
    assert!(
        group.memory_bytes() >= mp3.len() / 2,
        "{}",
        group.memory_bytes()
    );
    drop(player);
    let events = [0x00, 0x90, 60, 100, 0x60, 0x80, 60, 0];
    let player = group
        .create_player(&common::midi(96, &events), None)
        .unwrap();
    assert!(group.memory_bytes() > 0);
    drop(player);
    let player = group
        .create_player(&[0xFE, 1, 69, 16, 71, 16], None)
        .unwrap();
    assert!(group.memory_bytes() > 0);
    drop(player);
    assert_eq!(group.memory_bytes(), 0);
}

#[test]
fn a_new_tone_sequence_counts_in_place_of_the_old() {
    let engine = engine(8000);
    let group = engine.create_group();
    group.set_limits(GroupLimits {
        max_bytes: 4096,
        max_players: 8,
    });
    // VERSION 1, then `notes` notes of a sixteenth.
    let sequence = |notes: usize| {
        let mut bytes = vec![0xFE, 1];
        bytes.extend((0..notes).flat_map(|_| [69, 1]));
        bytes
    };
    let first = group.create_tone_player().unwrap();
    first.set_tone_sequence(&sequence(100)).unwrap();
    let held = group.memory_bytes();
    assert!((1000..=4096).contains(&held), "{held}");
    // Replacing it with another of the same size takes no more room.
    first.set_tone_sequence(&sequence(100)).unwrap();
    assert_eq!(group.memory_bytes(), held);
    // A second player cannot take a sequence that no longer fits beside it.
    let second = group.create_tone_player().unwrap();
    let refused = second.set_tone_sequence(&sequence(100)).unwrap_err();
    assert!(matches!(refused, AudioError::OutOfMemory(_)), "{refused}");
    // And one that is bigger than the whole budget can never.
    let refused = first.set_tone_sequence(&sequence(300)).unwrap_err();
    assert!(matches!(refused, AudioError::TooLarge(_)), "{refused}");
    // The first sequence is still the one in place.
    assert_eq!(group.memory_bytes(), held + 8);
    first.close();
    second.set_tone_sequence(&sequence(100)).unwrap();
}
