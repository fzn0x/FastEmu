//! The high-level screens through the launcher: each test runs a small
//! MIDlet from `tests/screens_java` (see `tests/screens_support`), drives
//! it with keys and typed text, and checks what its listeners print, its
//! frames pixel by pixel, and `Instance::wants_text`.

mod screens_support;
mod support;

use std::time::{Duration, Instant};

use fastemu_core::{Frame, InputEvent, Instance, Key, KeyAction, builtin_profiles};
use screens_support::{midlet, suite};
use support::{Running, TIMEOUT, rgb};

/// The theme's colours (`lcdui::theme::Theme::default`).
const BACKGROUND: u32 = 0xFF_FFFF;
const HIGHLIGHT: u32 = 0x1F_5FAF;
const BAR: u32 = 0x20_2020;

/// The runs of rows where column `x` of `frame` is `color`, as
/// `(first, last)` pairs.
fn bands(frame: &Frame, x: u32, color: u32) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for y in 0..frame.height() {
        if rgb(frame, x, y) != color {
            continue;
        }
        match out.last_mut() {
            Some(last) if last.1 + 1 == y => last.1 = y,
            _ => out.push((y, y)),
        }
    }
    out
}

/// How many pixels of `frame` are `color`.
fn count(frame: &Frame, color: u32) -> usize {
    (0..frame.height())
        .flat_map(|y| (0..frame.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| rgb(frame, x, y) == color)
        .count()
}

/// Waits until `test` holds.
fn wait_until(run: &mut Running, what: &str, test: impl Fn(&Running) -> bool) {
    let start = Instant::now();
    while !test(run) {
        assert!(
            start.elapsed() <= TIMEOUT,
            "{what} never happened; the log:\n{}",
            run.dump()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn screens_list_selects_and_fires_its_commands() {
    let Some(jar) = suite("list") else { return };
    let mut run = Running::launch(&jar, midlet::LISTS);
    run.wait_log("flags 2 true -1");
    run.wait_log("exclusive 0 true");
    run.wait_log("index refused");
    run.wait_log("type refused");
    run.wait_log("short array refused");
    // The title bar, then the focused first row.
    let first = run.wait_frame(|f| !bands(f, 230, HIGHLIGHT).is_empty());
    assert_eq!(rgb(&first, 2, 2), BAR, "the title bar");
    let row0 = bands(&first, 230, HIGHLIGHT)[0];
    let title = bands(&first, 230, BAR)[0];
    assert!(row0.0 > title.1, "the row is below the title");
    run.tap(Key::Down);
    let moved = run.wait_frame(|f| {
        bands(f, 230, HIGHLIGHT)
            .first()
            .is_some_and(|b| b.0 > row0.1)
    });
    let row1 = bands(&moved, 230, HIGHLIGHT)[0];
    assert_eq!(
        rgb(&moved, 230, row0.0 + 2),
        BACKGROUND,
        "the old row is plain"
    );
    assert_eq!(row1.1 - row1.0, row0.1 - row0.0, "rows are alike");
    run.tap(Key::Fire);
    run.wait_log("select 1 Beta");
    // Two commands: the exit command on the right, the other on the left.
    run.tap(Key::SoftLeft);
    run.wait_log("command Open 1");
    run.tap(Key::Up);
    run.tap(Key::Up);
    run.tap(Key::Fire);
    run.wait_log("select 2 Gamma");
}

#[test]
fn screens_ticker_scrolls() {
    let Some(jar) = suite("ticker") else { return };
    let mut run = Running::launch(&jar, midlet::TEXTS);
    // The ticker line runs just below the title bar.
    let first = run.wait_frame(|f| rgb(f, 0, 2) == BAR && !bands(f, 0, HIGHLIGHT).is_empty());
    let title = bands(&first, 0, BAR)[0];
    let y = title.1 + 7;
    assert_eq!(rgb(&first, first.width() - 1, title.1 + 1), HIGHLIGHT);
    let line = |f: &Frame| -> Vec<u32> { (0..f.width()).map(|x| rgb(f, x, y)).collect() };
    let before = line(&first);
    let later = run.wait_frame(|f| f.serial() > first.serial() + 3 && line(f) != before);
    assert_ne!(line(&later), before, "the ticker's text moved");
}

#[test]
fn screens_text_box_takes_typing_and_multi_tap() {
    let Some(jar) = suite("text") else { return };
    let mut run = Running::launch(&jar, midlet::TEXTS);
    run.wait_log("too long refused");
    run.wait_log("numeric refused");
    run.wait_log("number 312 3 3");
    run.wait_log("range refused");
    run.wait_log("max 2 31");
    run.wait_log("kept 31");
    wait_until(&mut run, "wants_text", |r| r.instance.wants_text());
    run.instance.send(InputEvent::Text("Bob".into()));
    // Two presses of 2 within the multi-tap time: "a", then "b".
    run.tap(Key::Num2);
    run.tap(Key::Num2);
    run.tap(Key::Num0);
    run.tap(Key::Clear);
    run.tap(Key::Clear);
    run.tap(Key::Num7);
    run.tap(Key::SoftLeft);
    run.wait_log("text [Bobp] caret 4");
    run.wait_log("canvas shown");
    wait_until(&mut run, "wants_text to end", |r| !r.instance.wants_text());
}

#[test]
fn screens_alerts_time_out_and_dismiss() {
    let Some(jar) = suite("alert") else { return };
    let mut run = Running::launch(&jar, midlet::ALERTS);
    run.wait_log("timeout 300 default 2000");
    run.wait_log("zero refused");
    run.wait_log("interactive refused");
    // The timed alert goes by itself and shows the canvas behind it.
    run.wait_log("behind 1");
    // The modal alert has a listener: it waits for the select key.
    run.wait_frame(|f| rgb(f, 2, 40) == BACKGROUND);
    std::thread::sleep(Duration::from_millis(400));
    assert!(!run.has("dismissed"), "{}", run.dump());
    run.tap(Key::Fire);
    run.wait_log("dismissed true");
    run.wait_log("behind 2");
}

#[test]
fn screens_form_traverses_and_tells_its_listener() {
    let Some(jar) = suite("form") else { return };
    let mut run = Running::launch(&jar, midlet::FORMS);
    run.wait_log("owned refused");
    run.wait_log("form 5");
    // The text field is the first item that takes the focus.
    wait_until(&mut run, "wants_text", |r| r.instance.wants_text());
    let frame = run.wait_frame(|f| count(f, HIGHLIGHT) > 20);
    assert_eq!(rgb(&frame, 2, 2), BAR, "the title bar");
    run.instance.send(InputEvent::Text("Ann".into()));
    run.wait_log("changed Name Ann");
    // A held 8 types one letter; its repeats do not move the focus down.
    run.key(Key::Num8, KeyAction::Press);
    std::thread::sleep(Duration::from_millis(700));
    run.key(Key::Num8, KeyAction::Release);
    run.wait_log("changed Name Annt");
    assert!(run.instance.wants_text(), "the text field kept the focus");
    run.tap(Key::Down);
    wait_until(&mut run, "the choice group's focus", |r| {
        !r.instance.wants_text()
    });
    run.tap(Key::Down);
    run.tap(Key::Fire);
    run.wait_log("changed Color 1");
    run.tap(Key::Down);
    run.tap(Key::Down);
    run.tap(Key::Right);
    run.wait_log("changed Level 6");
    run.tap(Key::Down);
    run.tap(Key::Fire);
    run.wait_log("item command Go");
    // The listener moved the focus back to the text field.
    wait_until(&mut run, "wants_text again", |r| r.instance.wants_text());
    run.tap(Key::SoftRight);
    run.wait_log("form command Done");
    assert!(!run.has("Exception"), "{}", run.dump());
}

#[test]
fn screens_form_fits_a_small_phone() {
    let Some(jar) = suite("small") else { return };
    let profile = builtin_profiles()
        .into_iter()
        .find(|p| p.screen_width == 176 && p.screen_height == 220)
        .expect("a 176x220 profile");
    let mut run = Running::launch_on(&jar, midlet::FORMS, profile);
    run.wait_log("form 5");
    let frame = run.wait_frame(|f| count(f, HIGHLIGHT) > 20);
    assert_eq!((frame.width(), frame.height()), (176, 220));
    assert_eq!(rgb(&frame, 2, 2), BAR, "the title bar");
    assert_eq!(rgb(&frame, 2, 219), BAR, "the soft key bar");
}

#[test]
fn screens_popup_emptied_while_open_takes_keys() {
    let Some(jar) = suite("edges-popup") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::EDGES);
    run.wait_log("edges shown");
    run.wait_frame(|f| count(f, HIGHLIGHT) > 0);
    // Open the popup's list, let the game empty it, then move in it.
    run.tap(Key::Fire);
    run.tap(Key::SoftLeft);
    run.wait_log("emptied 1");
    run.tap(Key::Up);
    run.tap(Key::Down);
    run.tap(Key::SoftLeft);
    run.wait_log("emptied 2");
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn screens_items_of_the_largest_size_are_laid_out() {
    let Some(jar) = suite("edges-sizes") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::EDGES);
    run.wait_log("edges shown");
    run.wait_log("tall paint");
    // Scroll through the tall items to the bottom and back.
    for _ in 0..4 {
        run.tap(Key::Down);
    }
    run.tap(Key::Up);
    run.wait_frame(|f| count(f, HIGHLIGHT) > 0);
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn screens_gauge_at_the_largest_maximum_moves() {
    let Some(jar) = suite("edges-gauge") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::EDGES);
    run.wait_log("edges shown");
    run.wait_frame(|f| count(f, HIGHLIGHT) > 0);
    // Down to the gauge, which is full: right stays, left steps back.
    run.tap(Key::Down);
    run.tap(Key::Right);
    run.tap(Key::Left);
    run.wait_log("gauge 2040109465");
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn screens_custom_item_gets_its_callbacks() {
    let Some(jar) = suite("custom") else { return };
    let mut run = Running::launch(&jar, midlet::CUSTOMS);
    run.wait_log("show");
    run.wait_log("size 60x30");
    run.wait_log("paint 60x30");
    run.wait_log("traverse 0 visible 60x30");
    let frame = run.wait_frame(|f| count(f, 0xFF_0000) >= 60 * 30);
    assert_eq!(
        count(&frame, 0xFF_0000),
        60 * 30,
        "exactly the item's pixels"
    );
    run.tap(Key::Num5);
    run.wait_log("key 53");
    run.tap(Key::Down);
    run.wait_log("out");
    wait_until(&mut run, "the text field's focus", |r| {
        r.instance.wants_text()
    });
    run.tap(Key::Up);
    run.wait_log("traverse 1 visible 60x30");
}
