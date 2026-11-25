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

use common::{constant_wav, controller_midi, engine, wav};
use fastemu_audio::{Engine, MAX_QUEUED, PlayerEventKind};

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

/// Renders `seconds` of engine time on another thread than the one that
/// is stuck in the listener.
fn run_for(engine: &Engine, seconds: f64) {
    engine.advance((f64::from(engine.sample_rate()) * seconds) as usize);
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

/// A one frame sound that loops makes an event for every pass, and the
/// game's listener may take them slowly. What waits for it is bounded.
#[test]
fn events_behind_a_blocked_listener_are_bounded() {
    let engine = engine(48_000);
    let group = engine.create_group();
    let listener = blocked_listener(&group);
    let player = group.create_player(&one_frame(), None).unwrap();
    player.set_loop_count(-1).unwrap();
    let starter = thread::spawn(move || {
        player.start().unwrap();
        player
    });
    listener.entered.recv().unwrap();
    // A quarter of a second: some 190,000 events when nothing limits them.
    run_for(&engine, 0.25);
    listener.release.send(()).unwrap();
    let player = starter.join().unwrap();
    player.stop().unwrap();
    // Everything queued has been delivered by now.
    let seen = listener.seen.load(Ordering::SeqCst);
    assert!(
        seen <= MAX_QUEUED + 16,
        "{seen} events reached the listener"
    );
    assert!(engine.dropped_events() > 0);
    // The ones that end a run are never among the dropped.
    assert!(matches!(
        *listener.last.lock().unwrap(),
        Some(PlayerEventKind::Stopped { .. })
    ));
}
