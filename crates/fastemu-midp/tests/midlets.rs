//! Real MIDlets through the launcher: each test compiles small Java
//! programs (see `tests/support`), packs them into a JAR, launches one
//! with `MidpLauncher`, drives it with keys and pointer events, and checks
//! its frames pixel by pixel and its log.

mod support;

use std::path::PathBuf;
use std::time::Duration;

use fastemu_core::{
    InputEvent, Instance, InstanceStatus, Key, KeyAction, Launcher, PointerAction, builtin_profiles,
};
use fastemu_midp::MidpLauncher;
use support::{Running, classes, midlet, rgb, scratch, write_suite};

/// The test suite's JAR in a fresh folder, or `None` without a JDK.
fn suite(name: &str) -> Option<PathBuf> {
    let classes = classes()?;
    Some(write_suite(classes, &scratch(name), "tests.jar", true))
}

/// The bar colour of the default theme.
const BAR: u32 = 0x20_2020;

#[test]
fn a_canvas_draws_shapes_text_and_images() {
    let Some(jar) = suite("shapes") else { return };
    let mut run = Running::launch(&jar, midlet::SHAPES);
    let line = run.wait_log("painted ");
    assert_eq!(line, "painted 240x320 font true clip 240x320");
    assert!(run.has("missing: IOException"), "{}", run.dump());
    assert!(!run.has("dot:"), "{}", run.dump());
    let frame = run.wait_frame(|f| rgb(f, 0, 0) == 0xFF_0000);
    assert_eq!((frame.width(), frame.height()), (240, 320));
    assert_eq!(rgb(&frame, 15, 15), 0x00_FF00, "fillRect");
    assert_eq!(rgb(&frame, 5, 50), 0x00_00FF, "drawLine");
    assert_eq!(rgb(&frame, 5, 51), 0xFF_0000, "one pixel thick");
    assert_eq!(rgb(&frame, 201, 201), 0x12_3456, "a mutable image");
    assert_eq!(rgb(&frame, 150, 150), 0xFF_0000, "the PNG's red pixel");
    assert_eq!(rgb(&frame, 151, 150), 0x00_FF00, "the PNG's green pixel");
    assert_eq!(rgb(&frame, 150, 151), 0x00_00FF, "the PNG's blue pixel");
    let blended = rgb(&frame, 151, 151);
    assert_eq!(blended >> 16, 0xFF, "half white over red keeps red");
    assert!((0x7E..=0x81).contains(&(blended & 0xFF)), "{blended:06X}");
    // Text is anti-aliased: count pixels the white glyphs lightened.
    let text_pixels = (100..140)
        .flat_map(|x| (100..120).map(move |y| (x, y)))
        .filter(|&(x, y)| rgb(&frame, x, y) & 0xFF > 0x80)
        .count();
    assert!(text_pixels > 10, "the text drew {text_pixels} light pixels");
    assert_eq!(rgb(&frame, 99, 99), 0xFF_0000, "nothing above the text");
    assert_eq!(run.instance.status(), InstanceStatus::Running);
}

#[test]
fn keys_and_pointer_events_reach_the_canvas() {
    let Some(jar) = suite("keys") else { return };
    let touch = builtin_profiles()
        .into_iter()
        .find(|p| p.touch)
        .expect("a touch profile");
    let mut run = Running::launch_on(&jar, midlet::KEYS, touch);
    run.wait_log("shown pointer true");
    // The first press throws; the event thread logs it and carries on.
    run.tap(Key::Num5);
    run.wait_log("Exception in keyPressed: java.lang.NullPointerException");
    run.wait_log("released 53");
    run.tap(Key::Fire);
    run.wait_log("pressed -5 action 8 name SELECT");
    run.wait_log("released -5");
    run.tap(Key::Num2);
    run.wait_log("pressed 50 action 1 name 2");
    run.key(Key::Up, KeyAction::Press);
    std::thread::sleep(Duration::from_millis(700));
    run.key(Key::Up, KeyAction::Release);
    run.wait_log("released -1");
    let lines = run.lines();
    let pressed = lines.iter().position(|l| l.starts_with("pressed -1"));
    let repeated = lines.iter().position(|l| l == "repeated -1");
    assert!(
        pressed.is_some() && repeated > pressed,
        "a held key repeats after its press:\n{}",
        lines.join("\n")
    );
    let pointer = |action, x, y| InputEvent::Pointer { action, x, y };
    run.instance.send(pointer(PointerAction::Press, 10, 20));
    run.instance.send(pointer(PointerAction::Drag, 15, 25));
    run.instance.send(pointer(PointerAction::Drag, 15, 25));
    run.instance.send(pointer(PointerAction::Release, 15, 25));
    run.wait_log("pointerPressed 10 20");
    run.wait_log("pointerReleased 15 25");
    let drags = run
        .lines()
        .iter()
        .filter(|l| l.starts_with("pointerDragged"))
        .count();
    assert_eq!(drags, 1, "{}", run.dump());
    assert_eq!(run.instance.error_count(), 1);
    assert_eq!(run.instance.status(), InstanceStatus::Running);
}

#[test]
fn service_repaints_on_a_game_thread_paints_before_returning() {
    let Some(jar) = suite("threads") else { return };
    let mut run = Running::launch(&jar, midlet::THREADS);
    run.wait_log("thread done 30");
    assert!(!run.has("returned early"), "{}", run.dump());
    run.wait_frame(|f| rgb(f, 0, 0) == 240 << 16);
}

#[test]
fn service_repaints_waits_for_a_paint_already_under_way() {
    let Some(jar) = suite("slow-paint") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::SLOW_PAINT);
    let line = run.wait_log("slow paint done");
    assert_eq!(line, "slow paint done early 0", "{}", run.dump());
}

#[test]
fn call_serially_queues_and_lets_paints_through() {
    let Some(jar) = suite("serial") else { return };
    let mut run = Running::launch(&jar, midlet::SERIAL);
    let line = run.wait_log("serial done 2000");
    let paints: u32 = line
        .rsplit(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a paint count");
    assert!(paints >= 4, "{line}");
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn a_game_looping_in_start_app_still_gets_events() {
    let Some(jar) = suite("looper") else { return };
    let mut run = Running::launch(&jar, midlet::LOOPER);
    run.wait_frame(|f| f.serial() >= 3);
    run.tap(Key::Fire);
    run.wait_log("key in loop -5");
    run.wait_log("loop sees key");
    run.instance.stop();
    run.wait_status(|s| *s == InstanceStatus::Exited);
    assert!(run.has("destroyApp true"), "{}", run.dump());
}

#[test]
fn one_command_shows_in_the_bar_and_takes_both_soft_keys() {
    let Some(jar) = suite("one-command") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::COMMANDS);
    let size = run.wait_log("size ");
    let height: i32 = size
        .trim_start_matches("size 240x")
        .parse()
        .expect("a height");
    assert!(height < 320, "{size}");
    assert!(
        run.has(&format!("sizeChanged 240x{height}")),
        "{}",
        run.dump()
    );
    let frame = run.wait_frame(|f| rgb(f, 0, 0) == 0x00_FF00);
    assert_eq!(rgb(&frame, 0, height as u32 - 1), 0x00_FF00);
    assert_eq!(rgb(&frame, 0, height as u32), BAR, "the soft key bar");
    assert_eq!(rgb(&frame, 0, 319), BAR);
    let label = (0..60)
        .flat_map(|x| (height as u32..320).map(move |y| (x, y)))
        .filter(|&(x, y)| rgb(&frame, x, y) != BAR)
        .count();
    assert!(label > 10, "the label drew {label} pixels");
    run.tap(Key::SoftRight);
    run.wait_log("command Exit");
    run.wait_status(|s| *s == InstanceStatus::Exited);
    assert!(!run.has("pressed -7"), "{}", run.dump());
}

#[test]
fn more_commands_open_a_menu() {
    let Some(jar) = suite("menu") else { return };
    let mut run = Running::launch(&jar, midlet::MANY_COMMANDS);
    let size = run.wait_log("size ");
    let height: u32 = size
        .trim_start_matches("size 240x")
        .parse()
        .expect("a height");
    let before = run.wait_frame(|f| rgb(f, 0, 0) == 0x00_FF00);
    run.tap(Key::SoftLeft);
    let menu = run.wait_frame(|f| f.serial() > before.serial());
    let covered = (0..240)
        .filter(|&x| rgb(&menu, x, height - 2) != 0x00_FF00)
        .count();
    assert!(covered > 200, "the menu covers the canvas above the bar");
    run.tap(Key::Down);
    run.tap(Key::Fire);
    run.wait_log("command Alpha");
    assert!(
        !run.has("pressed"),
        "keys went to the menu:\n{}",
        run.dump()
    );
    run.tap(Key::SoftRight);
    run.wait_log("command Exit");
    run.wait_status(|s| *s == InstanceStatus::Exited);
}

#[test]
fn full_screen_commands_leave_the_right_soft_key_to_the_game() {
    let Some(jar) = suite("full-command") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::FULL_COMMANDS);
    run.wait_log("size 240x320");
    let before = run.wait_frame(|f| rgb(f, 0, 319) == 0x00_FF00);
    run.tap(Key::SoftRight);
    run.wait_log("pressed -7");
    run.tap(Key::SoftLeft);
    run.wait_frame(|f| f.serial() > before.serial() && rgb(f, 0, 319) == BAR);
    run.tap(Key::Fire);
    run.wait_log("command Alpha");
    assert!(!run.has("pressed -6"), "{}", run.dump());
    assert!(!run.has("pressed -5"), "{}", run.dump());
}

#[test]
fn pause_resume_and_stop_drive_the_lifecycle() {
    let Some(jar) = suite("lifecycle") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::LIFECYCLE);
    run.wait_log("startApp 1");
    run.wait_log("property 3 Tests null");
    let platform = run.wait_log("platform ");
    assert!(platform.starts_with("platform Nokia6300"), "{platform}");
    assert!(platform.ends_with(" os null"), "{platform}");
    run.wait_log("showNotify");
    run.wait_frame(|f| rgb(f, 0, 0) == 0x00_00FF);
    run.instance.pause();
    run.wait_status(|s| *s == InstanceStatus::Paused);
    assert!(
        run.has("hideNotify") && run.has("pauseApp"),
        "{}",
        run.dump()
    );
    run.instance.resume();
    run.wait_log("startApp 2");
    run.wait_status(|s| *s == InstanceStatus::Running);
    let start = std::time::Instant::now();
    while run.lines().iter().filter(|l| *l == "showNotify").count() < 2 {
        assert!(start.elapsed() < support::TIMEOUT, "{}", run.dump());
        std::thread::sleep(Duration::from_millis(5));
    }
    run.instance.stop();
    run.wait_status(|s| *s == InstanceStatus::Exited);
    assert!(run.has("destroyApp true"), "{}", run.dump());
}

#[test]
fn stopping_a_paused_instance_still_destroys_the_midlet() {
    let Some(jar) = suite("stop-paused") else {
        return;
    };
    let mut run = Running::launch(&jar, midlet::LIFECYCLE);
    run.wait_log("showNotify");
    run.instance.pause();
    run.wait_status(|s| *s == InstanceStatus::Paused);
    // Input sent while paused is dropped, not delivered on resume.
    run.tap(Key::Fire);
    run.instance.stop();
    run.wait_status(|s| *s == InstanceStatus::Exited);
    assert!(run.has("destroyApp true"), "{}", run.dump());
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}

#[test]
fn a_crash_in_start_app_ends_the_instance() {
    let Some(jar) = suite("crash") else { return };
    let mut run = Running::launch(&jar, midlet::CRASH);
    let status = run.wait_status(InstanceStatus::is_finished);
    match status {
        InstanceStatus::Crashed { message } => {
            assert!(message.contains("startApp"), "{message}");
            assert!(message.contains("boom"), "{message}");
        }
        other => panic!("expected a crash, got {other:?}"),
    }
    assert!(run.has("destroyApp after crash"), "{}", run.dump());
}

#[test]
fn a_jar_without_a_central_directory_still_runs() {
    let Some(classes) = classes() else { return };
    let jar = write_suite(classes, &scratch("broken"), "broken.jar", false);
    let mut run = Running::launch(&jar, midlet::SHAPES);
    run.wait_log("painted 240x320");
    run.wait_frame(|f| rgb(f, 15, 15) == 0x00_FF00);
}

#[test]
fn a_jad_with_odd_formatting_names_its_jar_and_overrides_the_manifest() {
    let Some(classes) = classes() else { return };
    let dir = scratch("jad");
    std::fs::create_dir_all(dir.join("files")).unwrap();
    write_suite(classes, &dir.join("files"), "tests.jar", true);
    let jad = dir.join("game.jad");
    std::fs::write(
        &jad,
        b"\xEF\xBB\xBFMIDlet-Name: JAD Tests\r\n\
MIDlet-Jar-URL: files/tests.jar\r\n\
MIDlet-Jar-Size: 1\r\n\
MIDlet-1: Life, , t.Life\r\n cycle\r\n\
\r\n\
Game-Speed:5\r\n",
    )
    .unwrap();
    let info = MidpLauncher::new(None)
        .inspect(&jad)
        .expect("inspecting the JAD");
    assert_eq!(info.name, "JAD Tests");
    assert_eq!(info.midlets[0].class_name, "t.Lifecycle");
    assert_eq!(
        info.vendor, "FastEmu",
        "the manifest fills what the JAD lacks"
    );
    let mut run = Running::launch(&jad, 0);
    run.wait_log("property 5 JAD Tests null");
    assert!(
        run.has("the JAD says MIDlet-Name"),
        "the mismatch is logged:\n{}",
        run.dump()
    );
}

#[test]
fn launching_a_missing_midlet_or_suite_fails_cleanly() {
    let Some(jar) = suite("missing") else { return };
    let launcher = MidpLauncher::new(None);
    let request = fastemu_core::LaunchRequest {
        midlet: 42,
        ..fastemu_core::LaunchRequest::new(&jar, jar.with_extension("data"))
    };
    assert!(matches!(
        launcher.launch(request),
        Err(fastemu_core::LaunchError::NoSuchMidlet(42))
    ));
    let nowhere = fastemu_core::LaunchRequest::new(jar.with_extension("nope"), "unused");
    assert!(matches!(
        launcher.launch(nowhere),
        Err(fastemu_core::LaunchError::Io(_))
    ));
}
