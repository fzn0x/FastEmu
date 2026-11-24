//! The game API through the launcher: `GameCanvas` key states, their
//! latching and suppression, flushing and its pacing, and the rules of
//! `Sprite`, `TiledLayer` and `LayerManager`, each run as a MIDlet from
//! `tests/java/midlets/t/Game*.java` and checked by its log and frames.

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
MIDlet-3: Flush, , t.GameFlush\r\n\
MIDlet-4: Layers, , t.GameLayers\r\n\
MIDlet-5: Scene, , t.GameScene\r\n\
MIDlet-6: Bars, , t.GameBars\r\n";

const KEYS: usize = 0;
const OPEN: usize = 1;
const FLUSH: usize = 2;
const LAYERS: usize = 3;
const SCENE: usize = 4;
const BARS: usize = 5;

const RED: u32 = 0xFF_0000;
const GREEN: u32 = 0x00_FF00;
const BLUE: u32 = 0x00_00FF;
const WHITE: u32 = 0xFF_FFFF;
const BLACK: u32 = 0x00_0000;

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

/// What `GameLayers` prints, one line per rule.
const LAYER_RULES: &str = "
null image: java.lang.NullPointerException
null sprite: java.lang.NullPointerException
frame width 3: java.lang.IllegalArgumentException
frame height 3: java.lang.IllegalArgumentException
frame width 0: java.lang.IllegalArgumentException
frames 4 4 0 at 0,0 size 4x2 ref 0,0 visible true
frame 4: java.lang.IndexOutOfBoundsException
frame -1: java.lang.IndexOutOfBoundsException
next and prev 3 shows 800402
empty sequence: java.lang.IllegalArgumentException
sequence with frame 4: java.lang.ArrayIndexOutOfBoundsException
sequence 3 0 shows 800402
sequence next shows 800400
default 4 0
moved at 2,6 size 4x2 ref 2,6
live 123456 frozen abcdef
more frames 1 2
fewer frames 0 1
null new image: java.lang.NullPointerException
new frame width 3: java.lang.IllegalArgumentException
before resize at 18,20 size 4x2 ref 20,20
resized at 20,20 size 2x2 ref 20,20
ref 1,0
placed at 9,10 size 4x2 ref 10,10
rot90 at 9,9 size 2x4 ref 10,10 shows 800100
mirror at 8,10 size 4x2 ref 10,10 shows 800100 corner 800000
transform 8: java.lang.IllegalArgumentException
transform -1: java.lang.IllegalArgumentException
redefined at 8,10 size 4x2 ref 11,10
copy at 8,10 size 4x2 ref 11,10 frame 2 visible false
copy shows 800002 hidden original shows ffffff
overlap true true
touching false false
hidden false false
null other: java.lang.NullPointerException
hole true false true
rectangle false true
negative rectangle: java.lang.IllegalArgumentException
ell false true
mirrored ell true false
tile pixel true true
clear pixel true false
empty cell false
animated cell true false
animated tile true
no columns: java.lang.IllegalArgumentException
no rows: java.lang.IllegalArgumentException
null tiles: java.lang.NullPointerException
tile width 3: java.lang.IllegalArgumentException
tile height 0: java.lang.IllegalArgumentException
grid 5x3 cells 2x2 size 10x6 cell 0
column 5: java.lang.IndexOutOfBoundsException
row 3: java.lang.IndexOutOfBoundsException
tile 9: java.lang.IndexOutOfBoundsException
tile -1: java.lang.IndexOutOfBoundsException
get column -1: java.lang.IndexOutOfBoundsException
set 8
animated -1 -2 3
animate tile 9: java.lang.IndexOutOfBoundsException
animate tile -1: java.lang.IndexOutOfBoundsException
changed 5
animated -3: java.lang.IndexOutOfBoundsException
animated to tile 9: java.lang.IndexOutOfBoundsException
get animated 0: java.lang.IndexOutOfBoundsException
filled 0 -1 -1 8
fill -1 columns: java.lang.IllegalArgumentException
fill past the edge: java.lang.IndexOutOfBoundsException
fill tile 9: java.lang.IndexOutOfBoundsException
painted 800002 800103 ffffff 800602
blank ffffff
kept 8 0
cleared 0
deleted animated: java.lang.IndexOutOfBoundsException
null tile set: java.lang.NullPointerException
tile set width 3: java.lang.IllegalArgumentException
resized 20x12
append null: java.lang.NullPointerException
insert at 1 of 0: java.lang.IndexOutOfBoundsException
insert at -1: java.lang.IndexOutOfBoundsException
layer 0 of 0: java.lang.IndexOutOfBoundsException
remove null: java.lang.NullPointerException
order a c b
insert a present layer at the end: java.lang.IndexOutOfBoundsException
moved c b a
appended b a c
removed a c size 2
negative window: java.lang.IllegalArgumentException
window ff0000 ff ff ffffff ffffff clip 0 0 15 15 origin 0
hidden layer ffffff ff
";

#[test]
fn game_layers_follow_the_spec() {
    let Some(jar) = suite("layers") else { return };
    let mut run = Running::launch(&jar, LAYERS);
    run.wait_log("layers done");
    let lines = run.lines();
    let printed: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .take_while(|&l| l != "layers done")
        .collect();
    let expected: Vec<&str> = LAYER_RULES.trim().lines().collect();
    for (i, (got, want)) in printed.iter().zip(&expected).enumerate() {
        assert_eq!(got, want, "line {i}; the log:\n{}", lines.join("\n"));
    }
    assert_eq!(printed.len(), expected.len(), "{}", lines.join("\n"));
    assert_eq!(run.instance.error_count(), 0);
}

#[test]
fn game_flushes_stop_at_the_soft_key_bar() {
    let Some(jar) = suite("bars") else { return };
    let mut run = Running::launch(&jar, BARS);
    let line = run.wait_log("bars height ");
    let height: u32 = line["bars height ".len()..]
        .split(' ')
        .next()
        .and_then(|h| h.parse().ok())
        .expect("a height");
    assert!(height < 320, "the bar takes some height: {line}");
    assert!(line.ends_with(" buffer 240x320"), "{line}");
    let frame = run.wait_frame(|f| rgb(f, 10, 10) == RED);
    assert_eq!(rgb(&frame, 239, height - 1), RED, "the canvas's last row");
    for y in height..320 {
        assert_ne!(rgb(&frame, 120, y), RED, "row {y} is the soft key bar");
    }
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn game_scene_draws_layers_through_the_view_window() {
    let Some(jar) = suite("scene") else { return };
    let mut run = Running::launch(&jar, SCENE);
    let line = run.wait_log("scene 1 ");
    assert_eq!(line, "scene 1 tiles true true dot false true");
    let frame = run.wait_frame(|f| rgb(f, 110, 110) == RED);
    assert_eq!(rgb(&frame, 0, 0), BLUE, "tile 1");
    assert_eq!(rgb(&frame, 0, 85), GREEN, "the animated row shows tile 2");
    assert_eq!(rgb(&frame, 101, 101), BLUE, "the sprite's clear corner");
    assert_eq!(rgb(&frame, 230, 10), BLACK, "right of the tiled layer");
    run.tap(Key::Fire);
    let line = run.wait_log("scene 2 ");
    assert_eq!(line, "scene 2 at 101,100 tiles true true dot true false");
    let frame = run.wait_frame(|f| rgb(f, 0, 85) == BLUE);
    assert_eq!(
        rgb(&frame, 85, 101),
        RED,
        "the turned sprite, moved by the window"
    );
    assert_eq!(rgb(&frame, 98, 101), BLUE, "its clear corner turned too");
    assert_eq!(rgb(&frame, 200, 10), BLUE);
    assert_eq!(rgb(&frame, 210, 10), BLACK, "the window moved 16 pixels");
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}
