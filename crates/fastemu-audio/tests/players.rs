//! Players through the public API: the MMAPI state machine, events,
//! media time, looping and volume.

mod common;

use common::{constant_wav, engine, kinds, peak, render, wav};
use fastemu_audio::{AudioError, Format, PlayerEventKind, PlayerState};

#[test]
fn a_player_goes_through_the_mmapi_states() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 800, 1000), Some("audio/x-wav"))
        .unwrap();
    assert_eq!(player.state(), PlayerState::Realized);
    assert_eq!(player.format(), Format::Wav);
    assert_eq!(player.content_type(), "audio/x-wav");
    assert_eq!(player.duration().unwrap(), 100_000);
    player.prefetch().unwrap();
    assert_eq!(player.state(), PlayerState::Prefetched);
    player.start().unwrap();
    assert_eq!(player.state(), PlayerState::Started);
    // Starting again does nothing.
    player.start().unwrap();
    render(&engine, 400);
    assert_eq!(player.media_time().unwrap(), 50_000);
    player.stop().unwrap();
    assert_eq!(player.state(), PlayerState::Prefetched);
    assert_eq!(player.media_time().unwrap(), 50_000);
    // Time stands still while stopped.
    render(&engine, 400);
    assert_eq!(player.media_time().unwrap(), 50_000);
    player.start().unwrap();
    render(&engine, 1000);
    assert_eq!(player.state(), PlayerState::Prefetched);
    assert_eq!(player.media_time().unwrap(), 100_000);
    player.deallocate().unwrap();
    assert_eq!(player.state(), PlayerState::Realized);
    player.close();
    assert_eq!(player.state(), PlayerState::Closed);
    assert_eq!(player.start(), Err(AudioError::Closed));
    assert_eq!(player.media_time(), Err(AudioError::Closed));
    player.close();
    assert_eq!(
        kinds(&events),
        vec![
            PlayerEventKind::Started { media_time_us: 0 },
            PlayerEventKind::Stopped {
                media_time_us: 50_000
            },
            PlayerEventKind::Started {
                media_time_us: 50_000
            },
            PlayerEventKind::EndOfMedia {
                media_time_us: 100_000,
                last: true,
            },
            PlayerEventKind::Closed,
        ]
    );
}

#[test]
fn starting_after_the_end_plays_from_the_start() {
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group
        .create_player(&constant_wav(8000, 80, 1000), None)
        .unwrap();
    player.start().unwrap();
    render(&engine, 200);
    assert_eq!(player.media_time().unwrap(), 10_000);
    player.start().unwrap();
    assert_eq!(player.media_time().unwrap(), 0);
    let heard = render(&engine, 40);
    assert!(peak(&heard) > 0.0);
}

#[test]
fn loops_post_end_of_media_and_started_each_time() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 100, 8000), None)
        .unwrap();
    assert!(matches!(
        player.set_loop_count(0),
        Err(AudioError::InvalidArgument(_))
    ));
    player.set_loop_count(3).unwrap();
    player.start().unwrap();
    assert!(matches!(
        player.set_loop_count(2),
        Err(AudioError::InvalidState(_))
    ));
    let heard = render(&engine, 400);
    // Three times through, then silence.
    assert!(heard[290..299].iter().all(|&v| v > 0.0));
    assert!(heard[310..].iter().all(|&v| v == 0.0));
    let pass = PlayerEventKind::EndOfMedia {
        media_time_us: 12_500,
        last: false,
    };
    let end = PlayerEventKind::EndOfMedia {
        media_time_us: 12_500,
        last: true,
    };
    let started = PlayerEventKind::Started { media_time_us: 0 };
    assert_eq!(
        kinds(&events),
        vec![
            started.clone(),
            pass.clone(),
            started.clone(),
            pass,
            started,
            end
        ]
    );
    // The loop count holds for the next start.
    player.start().unwrap();
    render(&engine, 400);
    assert_eq!(kinds(&events).len(), 6);
}

#[test]
fn looping_forever_keeps_going_until_stopped() {
    let engine = engine(8000);
    let group = engine.create_group();
    let player = group
        .create_player(&constant_wav(8000, 100, 8000), None)
        .unwrap();
    player.set_loop_count(-1).unwrap();
    player.start().unwrap();
    let heard = render(&engine, 10_000);
    assert!(heard[9_990..].iter().all(|&v| v > 0.0));
    assert_eq!(player.state(), PlayerState::Started);
    render(&engine, 50);
    assert_eq!(player.media_time().unwrap(), 6_250);
    player.stop().unwrap();
}

#[test]
fn media_time_can_be_set_while_stopped_or_playing() {
    let engine = engine(8000);
    let group = engine.create_group();
    // A ramp, so the sample heard tells where playback is.
    let ramp: Vec<i16> = (0..8000).map(|i| i as i16 * 4).collect();
    let player = group.create_player(&wav(8000, 1, &ramp), None).unwrap();
    assert_eq!(player.set_media_time(500_000).unwrap(), 500_000);
    assert_eq!(player.set_media_time(-5).unwrap(), 0);
    assert_eq!(player.set_media_time(9_000_000).unwrap(), 1_000_000);
    player.set_media_time(250_000).unwrap();
    player.start().unwrap();
    render(&engine, 100);
    assert_eq!(player.media_time().unwrap(), 262_500);
    player.set_media_time(750_000).unwrap();
    assert_eq!(player.media_time().unwrap(), 750_000);
    let heard = render(&engine, 100);
    // After the short ramp in, the ramp's values around frame 6000.
    let expected = f32::from(6050i16 * 4) / 32768.0;
    assert!(
        (heard[50] * (1.0 / 0.7) - expected).abs() < 0.02,
        "{}",
        heard[50]
    );
    assert_eq!(player.media_time().unwrap(), 762_500);
}

#[test]
fn volume_levels_follow_the_loudness_scale_and_post_events() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 8000, 16384), None)
        .unwrap();
    player.set_loop_count(-1).unwrap();
    player.start().unwrap();
    let full = peak(&render(&engine, 400)[200..]);
    assert_eq!(player.set_level(50).unwrap(), 50);
    assert_eq!(player.set_level(50).unwrap(), 50);
    let half = peak(&render(&engine, 400)[200..]);
    let db = 20.0 * (half / full).log10();
    assert!((db + 10.0).abs() < 0.2, "{db}");
    assert_eq!(player.set_level(500).unwrap(), 100);
    player.set_muted(true).unwrap();
    assert!(player.is_muted().unwrap());
    let before = player.media_time().unwrap();
    assert_eq!(peak(&render(&engine, 400)[200..]), 0.0);
    // Muted players keep playing.
    assert!(player.media_time().unwrap() > before);
    player.set_muted(false).unwrap();
    assert_eq!(player.level().unwrap(), 100);
    let changes: Vec<PlayerEventKind> = kinds(&events)
        .into_iter()
        .filter(|k| matches!(k, PlayerEventKind::VolumeChanged { .. }))
        .collect();
    assert_eq!(
        changes,
        vec![
            PlayerEventKind::VolumeChanged {
                level: 50,
                muted: false
            },
            PlayerEventKind::VolumeChanged {
                level: 100,
                muted: false
            },
            PlayerEventKind::VolumeChanged {
                level: 100,
                muted: true
            },
            PlayerEventKind::VolumeChanged {
                level: 100,
                muted: false
            },
        ]
    );
}

#[test]
fn starts_and_stops_ramp_instead_of_clicking() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let player = group
        .create_player(&constant_wav(48_000, 48_000, 30_000), None)
        .unwrap();
    player.start().unwrap();
    let heard = render(&engine, 1000);
    // The first frame starts near silence and rises over about 3 ms.
    assert!(heard[0].abs() < 0.01);
    assert!(heard[60] < heard[120]);
    let steady = heard[900];
    assert!(steady > 0.5);
    player.stop().unwrap();
    let after = render(&engine, 400);
    assert!(after[0] > steady * 0.8, "{}", after[0]);
    assert!(after[100] < after[0]);
    assert_eq!(after[300], 0.0);
}

#[test]
fn players_resample_to_the_output_rate() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let events = group.subscribe();
    // 800 frames at 8 kHz is 100 ms: 4800 frames at 48 kHz.
    let player = group
        .create_player(&constant_wav(8000, 800, 8000), None)
        .unwrap();
    player.start().unwrap();
    let heard = render(&engine, 6000);
    assert!(heard[4790] > 0.0);
    assert!(heard[4810..].iter().all(|&v| v == 0.0));
    assert!(kinds(&events).contains(&PlayerEventKind::EndOfMedia {
        media_time_us: 100_000,
        last: true,
    }));
}

#[test]
fn closing_frees_the_player_and_dropping_closes_it() {
    let engine = engine(8000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group
        .create_player(&constant_wav(8000, 800, 100), None)
        .unwrap();
    let id = player.id();
    player.start().unwrap();
    assert_eq!(engine.player_count(), 1);
    drop(player);
    assert_eq!(engine.player_count(), 0);
    let closed: Vec<_> = events.try_iter().filter(|e| e.player == id).collect();
    assert_eq!(closed.last().unwrap().kind, PlayerEventKind::Closed);
}

#[test]
fn many_players_open_and_close_without_leaks() {
    let engine = engine(22_050);
    let group = engine.create_group();
    let sound = constant_wav(8000, 400, 1000);
    for round in 0..20 {
        let players: Vec<_> = (0..50)
            .map(|_| group.create_player(&sound, Some("audio/x-wav")).unwrap())
            .collect();
        for (i, player) in players.iter().enumerate() {
            if (i + round) % 2 == 0 {
                player.start().unwrap();
            }
        }
        assert_eq!(engine.player_count(), 50);
        render(&engine, 256);
        for player in &players {
            player.close();
        }
        assert_eq!(engine.player_count(), 0);
        assert_eq!(engine.playing_count(), 0);
    }
    // Playing hundreds at once stays inside full scale.
    let players: Vec<_> = (0..200)
        .map(|_| {
            let player = group
                .create_player(&constant_wav(8000, 4000, 30_000), None)
                .unwrap();
            player.start().unwrap();
            player
        })
        .collect();
    let heard = render(&engine, 2000);
    assert!(heard.iter().all(|v| v.abs() <= 1.0));
    assert!(peak(&heard) > 0.8);
    drop(players);
    assert_eq!(engine.player_count(), 0);
}

#[test]
fn listeners_may_call_back_into_the_engine() {
    let engine = engine(8000);
    let group = engine.create_group();
    let inner = engine.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    let sender = std::sync::Mutex::new(sender);
    group.set_listener(Some(std::sync::Arc::new(move |event| {
        // Taking the mixer lock here would deadlock if events were
        // delivered while it was held.
        let count = inner.player_count();
        sender.lock().unwrap().send((event.kind, count)).unwrap();
    })));
    let player = group
        .create_player(&constant_wav(8000, 80, 100), None)
        .unwrap();
    player.start().unwrap();
    render(&engine, 200);
    let got: Vec<_> = receiver.try_iter().collect();
    assert_eq!(got.len(), 2);
    assert_eq!(got[1].1, 1);
}

#[test]
fn players_work_from_many_threads_while_rendering() {
    let engine = engine(8000);
    let group = engine.create_group();
    let sound = constant_wav(8000, 200, 1000);
    let renderer = {
        let engine = engine.clone();
        std::thread::spawn(move || {
            for _ in 0..200 {
                render(&engine, 64);
            }
        })
    };
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let group = group.clone();
            let sound = sound.clone();
            std::thread::spawn(move || {
                for _ in 0..50 {
                    let player = group.create_player(&sound, None).unwrap();
                    player.start().unwrap();
                    let _ = player.media_time().unwrap();
                    player.set_level(40).unwrap();
                    player.stop().unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    renderer.join().unwrap();
    assert_eq!(engine.player_count(), 0);
}
