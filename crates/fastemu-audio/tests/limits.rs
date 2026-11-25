//! What keeps one game's sound from starving another's: the fair lock,
//! bounded events, bounded MIDI work, the cap on started players and the
//! guard around rendering. Everything runs on the engine's own clock,
//! with no device; only the fairness test uses real time, and its bounds
//! are far looser than a healthy engine needs.

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use common::{constant_wav, controller_midi, engine, kinds, midi, wav};
use fastemu_audio::{AudioError, MAX_QUEUED, PlayerEventKind, PlayerState};

/// A listener that blocks on its first event until released, and counts
/// every event it gets. The thread that started the player is stuck
/// inside it, delivering, so everything the engine makes meanwhile stays
/// queued: what a slow Java listener does to the events behind it.
struct BlockedListener {
    entered: mpsc::Receiver<()>,
    release: mpsc::Sender<()>,
    seen: Arc<AtomicUsize>,
    last: Arc<Mutex<Option<PlayerEventKind>>>,
}

fn blocked_listener(group: &fastemu_audio::Group) -> BlockedListener {
    let (entered_tx, entered) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let seen = Arc::new(AtomicUsize::new(0));
    let last = Arc::new(Mutex::new(None));
    let (count, kept) = (Arc::clone(&seen), Arc::clone(&last));
    group.set_listener(Some(Arc::new(move |event| {
        *kept.lock().unwrap() = Some(event.kind);
        if count.fetch_add(1, Ordering::SeqCst) == 0 {
            entered_tx.send(()).unwrap();
            let _ = release_rx.lock().unwrap().recv();
        }
    })));
    BlockedListener {
        entered,
        release,
        seen,
        last,
    }
}

/// A WAV of one frame at 384 kHz: looped, every output frame passes its
/// end and starts again.
fn one_frame() -> Vec<u8> {
    wav(384_000, 1, &[1000])
}

/// One thread seeks a big MIDI file again and again, as fast as it can,
/// while the audio thread renders buffers and another game starts and
/// stops a player. Each call holds the mixer for a moment; none may keep
/// the others out for long.
#[test]
fn a_thread_that_keeps_seeking_cannot_starve_the_others() {
    let engine = engine(48_000);
    let noisy = engine.create_group();
    let quiet = engine.create_group();
    // The most events a file may hold, nearly: every seek replays them.
    let big = noisy.create_player(&controller_midi(65_000), None).unwrap();
    big.start().unwrap();
    let victim = quiet
        .create_player(&constant_wav(8000, 800, 1000), None)
        .unwrap();
    let window = Duration::from_millis(1500);

    let stop = Arc::new(AtomicBool::new(false));
    let seeker = {
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut seeks = 0u64;
            while !stop.load(Ordering::Relaxed) {
                let _ = big.set_media_time(300_000 + (seeks % 1000) as i64);
                seeks += 1;
            }
            seeks
        })
    };
    let audio = {
        let engine = engine.clone();
        thread::spawn(move || {
            let mut buffer = vec![0.0f32; 1024];
            let (mut renders, mut worst) = (0u32, Duration::ZERO);
            let end = Instant::now() + window;
            while Instant::now() < end {
                let started = Instant::now();
                engine.render(&mut buffer);
                worst = worst.max(started.elapsed());
                renders += 1;
                thread::sleep(Duration::from_millis(1));
            }
            (renders, worst)
        })
    };
    let (mut pairs, mut worst_pair) = (0u32, Duration::ZERO);
    let end = Instant::now() + window;
    while Instant::now() < end {
        let started = Instant::now();
        victim.start().unwrap();
        victim.stop().unwrap();
        worst_pair = worst_pair.max(started.elapsed());
        pairs += 1;
        thread::sleep(Duration::from_millis(2));
    }
    let (renders, worst_render) = audio.join().unwrap();
    stop.store(true, Ordering::Relaxed);
    let seeks = seeker.join().unwrap();

    let report = format!(
        "{renders} renders (worst {worst_render:?}), {pairs} start/stop pairs \
         (worst {worst_pair:?}), {seeks} seeks"
    );
    println!("{report}");
    // A healthy engine renders hundreds of buffers and runs hundreds of
    // pairs in the window. With an unfair lock the seeker took it back
    // each time and a render or a pair waited hundreds of milliseconds.
    assert!(renders >= 30, "the audio thread was starved: {report}");
    assert!(pairs >= 30, "the other game was starved: {report}");
    assert!(worst_render < Duration::from_secs(1), "{report}");
    assert!(worst_pair < Duration::from_secs(1), "{report}");
}

/// A game can make volume events as fast as it can call `setLevel`, and
/// the game's listener may take them slowly. What waits for it is bounded.
#[test]
fn events_behind_a_blocked_listener_are_bounded() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let listener = blocked_listener(&group);
    let player = group
        .create_player(&constant_wav(8000, 800, 1000), None)
        .unwrap();
    let player = Arc::new(player);
    let starter = {
        let player = Arc::clone(&player);
        thread::spawn(move || player.start().unwrap())
    };
    listener.entered.recv().unwrap();
    // The thread that started it is stuck in the listener now, so these
    // events stay queued: 20,000 of them when nothing limits them.
    for i in 0..20_000 {
        player.set_level(if i % 2 == 0 { 10 } else { 90 }).unwrap();
    }
    listener.release.send(()).unwrap();
    starter.join().unwrap();
    player.stop().unwrap();
    // Everything queued has been delivered by now.
    let seen = listener.seen.load(Ordering::SeqCst);
    assert!(
        seen <= MAX_QUEUED + 16,
        "{seen} events reached the listener"
    );
    assert!(engine.dropped_events() > 15_000);
    // The ones that end a run are never among the dropped.
    assert!(matches!(
        *listener.last.lock().unwrap(),
        Some(PlayerEventKind::Stopped { .. })
    ));
}

/// The same sound with a listener that keeps up: a loop of one frame is
/// still heard, but it is reported about fifty times a second.
#[test]
fn a_one_frame_loop_is_reported_a_few_times_a_second() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group.create_player(&one_frame(), None).unwrap();
    player.set_loop_count(-1).unwrap();
    player.start().unwrap();
    // A second of sound, 384,000 passes of the frame.
    let mut sound = vec![0.0; 2 * 48_000];
    engine.render(&mut sound);
    let heard = sound.iter().filter(|v| v.abs() > 0.001).count();
    assert!(heard > 2 * 40_000, "the loop went quiet: {heard} samples");
    let events = events.try_iter().count();
    assert!(
        (90..=110).contains(&events),
        "{events} events in a second of a loop of one frame"
    );
}

/// A MIDI file of one note that lasts `ticks` ticks of about a millisecond.
fn short_midi(ticks: u8) -> Vec<u8> {
    midi(480, &[0x00, 0x90, 60, 100, ticks, 0x80, 60, 0])
}

/// A file of a few milliseconds sends all its events again each time it
/// wraps, so it plays once, as a phone's sequencer would.
#[test]
fn a_midi_shorter_than_20_ms_plays_once_however_it_is_looped() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group.create_player(&short_midi(10), None).unwrap();
    assert_eq!(player.duration().unwrap(), 10_416);
    player.set_loop_count(-1).unwrap();
    player.start().unwrap();
    engine.advance(48_000);
    assert_eq!(player.state(), PlayerState::Prefetched);
    let kinds = kinds(&events);
    assert_eq!(kinds.len(), 2, "{kinds:?}");
    assert!(matches!(
        kinds[1],
        PlayerEventKind::EndOfMedia { last: true, .. }
    ));
}

#[test]
fn a_midi_of_30_ms_is_looped() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group.create_player(&short_midi(29), None).unwrap();
    assert_eq!(player.duration().unwrap(), 30_208);
    player.set_loop_count(-1).unwrap();
    player.start().unwrap();
    engine.advance(24_000);
    assert_eq!(player.state(), PlayerState::Started);
    let loops = kinds(&events)
        .iter()
        .filter(|kind| matches!(kind, PlayerEventKind::EndOfMedia { last: false, .. }))
        .count();
    assert!((14..=17).contains(&loops), "{loops} passes in half a second");
}

/// A MIDI file of `n` note ons at tick 0 and a rest of half a second.
fn note_ons(n: usize) -> Vec<u8> {
    let mut events = Vec::with_capacity(n * 4 + 8);
    for i in 0..n {
        events.extend_from_slice(&[0x00, 0x90 | (i % 16) as u8, (24 + i % 90) as u8, 100]);
    }
    events.extend_from_slice(&[0x83, 0x60, 0x80, 60, 0]);
    midi(480, &events)
}

#[test]
fn a_midi_of_more_than_65536_events_is_refused_as_too_large() {
    let engine = engine(48_000);
    let group = engine.create_group();
    // 200,000 events: a file of 800 KB that would take every pass of a
    // loop to send again.
    let refused = group.create_player(&note_ons(200_000), None).unwrap_err();
    assert!(matches!(refused, AudioError::TooLarge(_)), "{refused:?}");
    assert!(refused.is_size_limit());
    assert_eq!(group.player_count(), 0);
    // The limit is the file's events, not its bytes: the most it may hold
    // still plays.
    assert!(group.create_player(&note_ons(65_534), None).is_ok());
}

/// The note ons of a file are sent a block at a time, so a file with
/// 65,000 of them at one instant sounds, plays to its end, and never holds
/// the mixer for all of them at once.
#[test]
fn a_midi_with_a_pile_of_notes_at_the_start_still_plays_through() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let events = group.subscribe();
    let player = group.create_player(&note_ons(65_000), None).unwrap();
    player.start().unwrap();
    let mut buffer = vec![0.0f32; 2 * 1024];
    let (mut loudest, mut calls) = (0.0f32, 0);
    while player.state() == PlayerState::Started {
        engine.render(&mut buffer);
        loudest = buffer.iter().fold(loudest, |m, v| m.max(v.abs()));
        calls += 1;
        assert!(calls < 100, "the file never ended");
    }
    assert!(loudest > 0.05, "the pile made no sound: {loudest}");
    assert!(matches!(
        kinds(&events).last(),
        Some(PlayerEventKind::EndOfMedia { last: true, .. })
    ));
}
