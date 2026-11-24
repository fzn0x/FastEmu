//! Groups: one per instance, with volume, mute, pause, and closing that
//! stops a game's sound.

mod common;

use common::{constant_wav, engine, kinds, peak, render};
use fastemu_audio::{AudioError, PlayerEventKind, PlayerState};

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
