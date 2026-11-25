//! A tool can tell a game that waits on a still screen from a stuck one:
//! `MidpInstance::ping` asks the event thread whether it would take a key,
//! and `MidpInstance::activity` says what the game's threads are doing. A
//! game that waits answers and has every thread waiting; a thread that
//! spins keeps it busy; an event thread stuck in a callback never answers,
//! even while every thread sleeps; and a thread that keeps the VM locked
//! lets no look in.
//!
//! The MIDlets are `t.StillMenu`, `t.StuckKey` and `t.StopLoop` of the
//! shared test programs, in suites of their own. Without a JDK the tests
//! print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is set (CI sets
//! it). No test depends on how fast the machine is: each waits as long as
//! it takes for a state that comes, and checks a state that never changes
//! as often as it likes.

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fastemu_core::{Key, KeyAction};
use support::{Running, TIMEOUT, classes, scratch, suite_entries, zip};

/// A suite whose one MIDlet is `t.<midlet>`, with `extra` manifest lines.
fn suite(name: &str, midlet: &str, extra: &str) -> Option<PathBuf> {
    let classes = classes()?;
    let manifest = format!(
        "Manifest-Version: 1.0\r\n\
         MIDlet-Name: Still\r\n\
         MIDlet-Vendor: FastEmu\r\n\
         MIDlet-Version: 1.0\r\n\
         MIDlet-1: Still, , t.{midlet}\r\n{extra}"
    );
    let path = scratch(name).join("still.jar");
    let jar = zip(&suite_entries(classes, &manifest), true);
    std::fs::write(&path, jar).expect("writing the JAR");
    Some(path)
}

/// Waits until `test` holds.
fn wait_until(run: &mut Running, what: &str, test: impl Fn(&Running) -> bool) {
    let begun = Instant::now();
    while !test(run) {
        assert!(
            begun.elapsed() <= TIMEOUT,
            "{what}: the activity stayed {:?}; the log:\n{}",
            run.instance.activity(),
            run.dump()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_menu_that_waits_for_a_key_answers_and_is_idle() {
    let Some(jar) = suite("still-menu", "StillMenu", "") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    run.wait_frame(|_| true);
    let ping = run.instance.ping();
    wait_until(&mut run, "answered and idle", |run| {
        ping.answered() && run.instance.activity().is_some_and(|a| a.is_idle())
    });
    // It still takes keys.
    run.tap(Key::Num5);
    run.wait_log("key ");
}

#[test]
fn a_thread_that_spins_behind_a_still_screen_keeps_the_game_busy() {
    let Some(jar) = suite("spin-menu", "StillMenu", "Spin: yes\r\n") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    run.wait_frame(|_| true);
    run.wait_log("spinning");
    // The event thread is free, so it answers...
    let ping = run.instance.ping();
    wait_until(&mut run, "answered", |_| ping.answered());
    // ...but the spinning thread never waits, so no look finds the game
    // idle, however the threads are scheduled.
    for _ in 0..200 {
        let activity = run.instance.activity();
        assert!(
            activity.is_none_or(|a| a.running() >= 1 && !a.is_idle()),
            "{activity:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn an_event_thread_stuck_in_a_callback_never_answers() {
    let Some(jar) = suite("stuck-key", "StuckKey", "") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    run.wait_frame(|_| true);
    // A press alone: a release would be one more task that waits for the
    // event thread.
    run.key(Key::Num5, KeyAction::Press);
    run.wait_log("stuck");
    // Every thread sleeps, so the threads alone would call it idle.
    wait_until(&mut run, "idle", |run| {
        run.instance.activity().is_some_and(|a| a.is_idle())
    });
    // The event thread never comes back for the ping, which then waits as
    // a posted task, so the game is not idle either.
    let ping = run.instance.ping();
    for _ in 0..200 {
        assert!(!ping.answered());
        std::thread::sleep(Duration::from_millis(5));
    }
    let activity = run.instance.activity();
    assert!(
        activity.is_none_or(|a| a.posted == 1 && !a.is_idle()),
        "{activity:?}"
    );
}

#[test]
fn a_game_that_keeps_the_machine_locked_lets_no_look_in() {
    let Some(jar) = suite("locked", "StopLoop", "") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    run.wait_log("looping");
    // The loop has no safepoint: once a look finds the lock taken, every
    // later one does.
    wait_until(&mut run, "locked", |run| run.instance.activity().is_none());
    // The ping cannot be posted, so it never answers.
    let ping = run.instance.ping();
    run.wait_log("dropped a task posted");
    assert!(run.instance.activity().is_none());
    assert!(!ping.answered());
}
