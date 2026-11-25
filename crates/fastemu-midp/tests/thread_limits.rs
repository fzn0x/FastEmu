//! Games that pause themselves and ask to be resumed.
//!
//! `t.PauseFlood` does it in a loop from a `startApp` that never returns,
//! which makes the app manager start one lifecycle thread for each call,
//! and each waits for its turn behind `startApp`. They are the host's
//! threads, and the machine counts them against its limit on threads:
//! `max_threads` for the game and 16 more for the host. An instance keeps
//! at most 8 of them alive and drops later calls, so the host still has
//! room for `destroyApp`.
//!
//! `t.SelfPause` pauses itself once and waits. The app's pause still
//! freezes it, without a `pauseApp`, and the app's resume resumes it.
//! `t.ResumeInPause` asks to be resumed from its `pauseApp`, which does
//! not undo the app's pause.
//!
//! `t.ResumeFlood` calls `notifyPaused` and `resumeRequest` in a tight
//! loop from `startApp`. The app's stop, pause and resume must still get
//! through, and the pause must hold. These tests count the game's
//! requests, not time: a slow machine slows the game and the app alike,
//! and the timeout only bounds a failure.
//!
//! Each MIDlet is one of the shared test programs, in a suite of its own.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fastemu_core::{Instance, InstanceStatus};
use fastemu_jvm::VmLimits;
use support::{Running, classes, rgb, scratch, suite_entries, zip};

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Threads\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: PauseFlood, , t.PauseFlood\r\n\
MIDlet-2: SelfPause, , t.SelfPause\r\n\
MIDlet-3: ResumeInPause, , t.ResumeInPause\r\n\
MIDlet-4: ResumeFlood, , t.ResumeFlood\r\n";

/// `MIDlet-2`: pauses itself once and waits.
const SELF_PAUSE: usize = 1;

/// `MIDlet-3`: asks to be resumed from its `pauseApp`.
const RESUME_IN_PAUSE: usize = 2;

/// `MIDlet-4`: pauses itself and asks to be resumed in a tight loop.
const RESUME_FLOOD: usize = 3;

/// The requests `t.ResumeFlood` makes before its screen turns green, and
/// after which it gives up.
const SHOWN: u32 = 1_000_000;
const MOST: u32 = 4_000_000;

fn suite(name: &str) -> Option<PathBuf> {
    let classes = classes()?;
    let path = scratch(name).join("threads.jar");
    let jar = zip(&suite_entries(classes, MANIFEST), true);
    std::fs::write(&path, jar).expect("writing the JAR");
    Some(path)
}

#[test]
fn three_hundred_pause_and_resume_requests_leave_at_most_eighty_threads() {
    let Some(jar) = suite("pause-flood") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    let limits = VmLimits::default();
    // The game's own threads and the host's reserve of 16.
    let most = limits.max_threads + 16;
    assert_eq!(most, 80);
    let line = run.wait_log("threads ");
    let threads: usize = line["threads ".len()..].parse().expect("a count");
    println!("the flood left {threads} threads, most {most}");
    assert!(threads <= most, "{threads} threads\n{}", run.dump());
    // The game lives on. Stopping it still gives the MIDlet its
    // destroyApp, which needs a thread of its own, and ends every thread.
    assert!(!run.instance.status().is_finished(), "{}", run.dump());
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    assert!(run.has("destroyed true"), "{}", run.dump());
}

/// Waits for a line that starts with `prefix` after the first line that
/// starts with `after`, and returns the lines so far.
fn wait_after(run: &mut Running, after: &str, prefix: &str) -> Vec<String> {
    let start = Instant::now();
    loop {
        let lines = run.lines();
        if let Some(from) = lines.iter().position(|l| l.starts_with(after))
            && lines[from..].iter().any(|l| l.starts_with(prefix))
        {
            return lines;
        }
        assert!(
            start.elapsed() <= support::TIMEOUT,
            "no {prefix:?} after {after:?}:\n{}",
            lines.join("\n")
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn the_app_pauses_and_resumes_a_game_that_paused_itself() {
    let Some(jar) = suite("self-pause") else {
        return;
    };
    let mut run = Running::launch(&jar, SELF_PAUSE);
    run.wait_log("paused itself");
    run.wait_log("showNotify");
    run.wait_status(|s| *s == InstanceStatus::Paused);
    // The app's pause reaches a game that paused itself: its screen is
    // hidden, and the game freezes.
    run.instance.pause();
    run.wait_log("hideNotify");
    // The app's resume shows the screen again and calls startApp.
    run.instance.resume();
    run.wait_log("startApp 2");
    let lines = wait_after(&mut run, "hideNotify", "showNotify");
    run.wait_status(|s| *s == InstanceStatus::Running);
    // MIDP calls pauseApp only on an active MIDlet. Had the pause called
    // it, it would have come before the second startApp, in its turn.
    assert!(
        !lines.iter().any(|l| l == "pauseApp"),
        "{}",
        lines.join("\n")
    );
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    assert!(run.has("destroyApp true"), "{}", run.dump());
}

#[test]
fn a_game_the_app_paused_does_not_resume_itself() {
    let Some(jar) = suite("resume-in-pause") else {
        return;
    };
    let mut run = Running::launch(&jar, RESUME_IN_PAUSE);
    run.wait_log("startApp 1");
    run.wait_status(|s| *s == InstanceStatus::Running);
    run.instance.pause();
    // Its resumeRequest is queued before the line.
    run.wait_log("pauseApp");
    // Had the request resumed the game, this second pause would land on
    // a running game and call pauseApp again.
    run.instance.pause();
    run.instance.resume();
    run.wait_log("startApp 2");
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    let lines = run.lines();
    let count = |line: &str| lines.iter().filter(|l| *l == line).count();
    assert_eq!(count("pauseApp"), 1, "{}", lines.join("\n"));
    assert_eq!(count("startApp 3"), 0, "{}", lines.join("\n"));
    assert_eq!(count("destroyApp true"), 1, "{}", lines.join("\n"));
}

/// The count of requests in the first line that starts with `prefix`.
fn requests_at(run: &mut Running, prefix: &str) -> u32 {
    let line = run.wait_log(prefix);
    line[line.find(prefix).unwrap() + prefix.len()..]
        .parse()
        .expect("a count")
}

/// Launches `t.ResumeFlood` and waits until it has asked to be resumed
/// [`SHOWN`] times.
fn flooding(name: &str) -> Option<Running> {
    let jar = suite(name)?;
    let mut run = Running::launch(&jar, RESUME_FLOOD);
    run.wait_frame(|f| rgb(f, 0, 0) == 0x00_FF00);
    Some(run)
}

#[test]
fn the_app_stops_a_game_that_asks_to_be_resumed_in_a_tight_loop() {
    let Some(mut run) = flooding("resume-flood-stop") else {
        return;
    };
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    let at = requests_at(&mut run, "destroyed at ");
    println!("destroyApp came after {at} requests");
    // The stop waited behind at most one request, not behind every one.
    assert!((SHOWN..MOST).contains(&at), "{}", run.dump());
    // notifyPaused logs a line only when it pauses a running game, which
    // takes one of the control thread's resumes: about one call in a
    // thousand. Were every call logged, the log would keep its most,
    // 10000 lines.
    let paused = run
        .lines()
        .iter()
        .filter(|l| *l == "the MIDlet paused itself")
        .count();
    println!("{paused} lines say the MIDlet paused itself");
    assert!(paused < 5_000, "{paused} lines\n{}", run.dump());
}

#[test]
fn the_app_pauses_a_game_that_asks_to_be_resumed_in_a_tight_loop() {
    let Some(mut run) = flooding("resume-flood-pause") else {
        return;
    };
    run.instance.pause();
    let hidden = requests_at(&mut run, "hidden at ");
    println!("hideNotify came after {hidden} requests");
    assert!(hidden < MOST, "{}", run.dump());
    // The game's own requests do not undo the app's pause: it is not
    // shown again before the stop.
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    let destroyed = requests_at(&mut run, "destroyed at ");
    let lines = run.lines();
    let after = lines
        .iter()
        .position(|l| l.starts_with("hidden at "))
        .unwrap();
    assert!(
        !lines[after..].iter().any(|l| l.starts_with("shown at ")),
        "{}",
        lines.join("\n")
    );
    assert!(destroyed < MOST, "{}", lines.join("\n"));
}

#[test]
fn the_app_resumes_a_game_that_asks_to_be_resumed_in_a_tight_loop() {
    let Some(mut run) = flooding("resume-flood-resume") else {
        return;
    };
    run.instance.pause();
    run.wait_log("hidden at ");
    run.instance.resume();
    let lines = wait_after(&mut run, "hidden at ", "shown at ");
    let hidden = lines
        .iter()
        .position(|l| l.starts_with("hidden at "))
        .unwrap();
    let shown: u32 = lines[hidden..]
        .iter()
        .find_map(|l| l.strip_prefix("shown at "))
        .and_then(|n| n.parse().ok())
        .expect("a count");
    println!("showNotify came after {shown} requests");
    run.instance.stop();
    run.wait_status(InstanceStatus::is_finished);
    let destroyed = requests_at(&mut run, "destroyed at ");
    assert!(shown < MOST && destroyed < MOST, "{}", run.dump());
}
