//! The game API through the launcher: `GameCanvas` key states, their
//! latching and suppression, and flushing and its pacing, each run as a
//! MIDlet from `tests/java/midlets/t/Game*.java` and checked by its log
//! and frames.

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fastemu_core::{Key, KeyAction};
use support::{Running, TIMEOUT, classes, rgb, scratch, suite_entries, zip};

/// The suite of the game test MIDlets.
const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Game tests\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: Keys, , t.GameKeys\r\n\
MIDlet-2: Open, , t.GameKeysOpen\r\n\
MIDlet-3: Flush, , t.GameFlush\r\n";

const KEYS: usize = 0;
const OPEN: usize = 1;
const FLUSH: usize = 2;

const RED: u32 = 0xFF_0000;
const GREEN: u32 = 0x00_FF00;
const WHITE: u32 = 0xFF_FFFF;

/// The game test suite's JAR in a fresh folder, or `None` without a JDK.
fn suite(name: &str) -> Option<PathBuf> {
    let classes = classes()?;
    let path = scratch(&format!("game-{name}")).join("game.jar");
    std::fs::write(&path, zip(&suite_entries(classes, MANIFEST), true)).expect("writing the JAR");
    Some(path)
}

/// Waits for a line containing `needle` among the lines after the first
/// `from`, and returns how many lines there are up to and including it.
fn wait_after(run: &mut Running, from: usize, needle: &str) -> usize {
    let start = Instant::now();
    loop {
        let lines = run.lines();
        if let Some(i) = lines.iter().skip(from).position(|l| l.contains(needle)) {
            return from + i + 1;
        }
        assert!(
            start.elapsed() <= TIMEOUT,
            "no line with {needle:?} after line {from}; the log:\n{}",
            run.dump()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The lines after the first `from` that contain `needle`.
fn lines_after(run: &mut Running, from: usize, needle: &str) -> Vec<String> {
    run.lines()
        .into_iter()
        .skip(from)
        .filter(|l| l.contains(needle))
        .collect()
}

/// A few polls of the game thread, which reads the key states every
/// 100 ms.
const POLLS: Duration = Duration::from_millis(400);

#[test]
fn game_key_states_latch_hold_and_suppress_events() {
    let Some(jar) = suite("keys") else { return };
    let mut run = Running::launch(&jar, KEYS);
    let mut at = wait_after(&mut run, 0, "states 0 shown true");
    // A tap between two polls is latched, then read once.
    run.tap(Key::Fire);
    at = wait_after(&mut run, at, "states 256 shown true");
    at = wait_after(&mut run, at, "states 0 shown true");
    run.tap(Key::Num2);
    at = wait_after(&mut run, at, "states 2 shown true");
    at = wait_after(&mut run, at, "states 0 shown true");
    // Keys without a game action still get their callbacks.
    run.tap(Key::Star);
    at = wait_after(&mut run, at, "pressed 42");
    at = wait_after(&mut run, at, "released 42");
    // A held key stays set until it is released.
    run.key(Key::Right, KeyAction::Press);
    at = wait_after(&mut run, at, "states 32 shown true");
    std::thread::sleep(POLLS);
    assert!(
        lines_after(&mut run, at, "states").is_empty(),
        "{}",
        run.dump()
    );
    run.key(Key::Right, KeyAction::Release);
    at = wait_after(&mut run, at, "states 0 shown true");
    // Hidden, the canvas reads 0 even for a key pressed meanwhile, and
    // showing it again forgets that key.
    run.tap(Key::Num0);
    at = wait_after(&mut run, at, "states 0 shown false");
    run.key(Key::Fire, KeyAction::Press);
    at = wait_after(&mut run, at, "elsewhere pressed -5");
    std::thread::sleep(POLLS);
    run.key(Key::Fire, KeyAction::Release);
    run.tap(Key::Num0);
    at = wait_after(&mut run, at, "states 0 shown true");
    std::thread::sleep(POLLS);
    let log = run.dump();
    assert!(lines_after(&mut run, at, "states").is_empty(), "{log}");
    assert!(!log.contains("states 256 shown false"), "{log}");
    for suppressed in [
        "pressed -5",
        "released -5",
        "pressed 50",
        "pressed -4",
        "repeated",
    ] {
        assert!(
            !log.lines().any(|l| l.starts_with(suppressed)),
            "{suppressed}: {log}"
        );
    }
    assert_eq!(run.instance.error_count(), 0, "{log}");
}

#[test]
fn game_keys_reach_the_callbacks_without_suppression() {
    let Some(jar) = suite("open") else { return };
    let mut run = Running::launch(&jar, OPEN);
    let mut at = wait_after(&mut run, 0, "states 0 shown true");
    run.tap(Key::Fire);
    at = wait_after(&mut run, at, "pressed -5");
    wait_after(&mut run, at, "released -5");
    wait_after(&mut run, at, "states 256 shown true");
}

#[test]
fn game_flushes_copy_the_buffer_and_keep_the_frame_rate() {
    let Some(jar) = suite("flush") else { return };
    let mut run = Running::launch(&jar, FLUSH);
    let line = run.wait_log("graphics new");
    assert_eq!(line, "graphics new true clip 0 0 240 320 color 0");
    run.wait_log("stage region");
    let frame = run.wait_frame(|f| rgb(f, 15, 15) == RED);
    for (x, y) in [(10, 10), (29, 29), (10, 29)] {
        assert_eq!(rgb(&frame, x, y), RED, "{x},{y} is in the region");
    }
    for (x, y) in [(9, 10), (10, 9), (30, 15), (15, 30), (100, 100)] {
        assert_eq!(rgb(&frame, x, y), WHITE, "{x},{y} is outside the region");
    }
    // paint draws the buffer, so a repaint without a flush shows it all.
    run.tap(Key::Fire);
    run.wait_log("stage repaint");
    let frame = run.wait_frame(|f| rgb(f, 100, 100) == RED);
    assert_eq!(rgb(&frame, 239, 319), RED);
    let before = frame.serial();
    run.tap(Key::Fire);
    let line = run.wait_log("stage paced ");
    let took: u64 = line["stage paced ".len()..].parse().expect("a time");
    // Thirty flushes at the default 60 frames per second take 29 periods.
    assert!(took >= 420, "thirty flushes took {took} ms");
    let frame = run.wait_frame(|f| rgb(f, 0, 0) == GREEN);
    assert!(frame.serial() >= before + 30, "{} {before}", frame.serial());
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}
