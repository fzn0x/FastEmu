//! The fair lock of the mixer: a game that keeps calling into the mixer
//! cannot keep the audio thread or another game out of it. This test runs
//! in real time, alone in its own test binary. Run beside the CPU-heavy
//! tests of limits.rs on a runner of three shared cores, it was once left
//! about one core and failed with no change to the engine (#100). Its
//! bounds are far looser than a healthy engine needs.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use common::{constant_wav, controller_midi, engine};

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
