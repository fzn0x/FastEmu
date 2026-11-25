//! A game that pauses itself and asks to be resumed in a loop, from a
//! `startApp` that never returns, makes the app manager start one
//! lifecycle thread for each call, and each waits for its turn behind
//! `startApp`. They are the host's threads, and the machine counts them
//! against its limit on threads: `max_threads` for the game and 16 more
//! for the host. An instance keeps at most 8 of them alive and drops later
//! calls, so the host still has room for `destroyApp`.
//!
//! The MIDlet is `t.PauseFlood` of the shared test programs, in a suite of
//! its own. Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::PathBuf;

use fastemu_core::{Instance, InstanceStatus};
use fastemu_jvm::VmLimits;
use support::{Running, classes, scratch, suite_entries, zip};

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Threads\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: PauseFlood, , t.PauseFlood\r\n";

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
