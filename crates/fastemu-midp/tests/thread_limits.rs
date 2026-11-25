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
//!
//! Each MIDlet is one of the shared test programs, in a suite of its own.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fastemu_core::{Instance, InstanceStatus};
use fastemu_jvm::VmLimits;
use support::{Running, classes, scratch, suite_entries, zip};

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Threads\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: PauseFlood, , t.PauseFlood\r\n\
MIDlet-2: SelfPause, , t.SelfPause\r\n";

/// `MIDlet-2`: pauses itself once and waits.
const SELF_PAUSE: usize = 1;

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
