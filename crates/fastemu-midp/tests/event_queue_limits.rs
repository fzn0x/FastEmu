//! The queue of `Display.callSerially` has a limit, as the queue of work
//! posted to the machine has: a game that keeps asking while the event
//! thread is busy gets `OutOfMemoryError`, as it would on a phone, and
//! what it asked for before that still runs.
//!
//! The MIDlet is `t.CallFlood` of the shared test programs, in a suite of
//! its own. The limit of the relayout queue is tested where it is, in
//! `lcdui/display.rs`, since the queue is not visible from here. Without a
//! JDK the tests print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is
//! set (CI sets it).

mod support;

use std::path::PathBuf;

use fastemu_core::{Instance, InstanceStatus};
use fastemu_jvm::VmLimits;
use support::{Running, classes, scratch, suite_entries, zip};

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Queues\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: CallFlood, , t.CallFlood\r\n";

fn suite(name: &str) -> Option<PathBuf> {
    let classes = classes()?;
    let path = scratch(name).join("queues.jar");
    let jar = zip(&suite_entries(classes, MANIFEST), true);
    std::fs::write(&path, jar).expect("writing the JAR");
    Some(path)
}

#[test]
fn twenty_thousand_call_serially_calls_behind_a_busy_event_thread_stop_at_the_limit() {
    let Some(jar) = suite("call-flood") else {
        return;
    };
    let mut run = Running::launch(&jar, 0);
    let max = VmLimits::default().max_posted;
    let taken = run.wait_log("taken ");
    assert_eq!(
        taken,
        format!(
            "taken {max} refused {} too many callSerially calls are waiting",
            20_000 - max
        ),
        "{}",
        run.dump()
    );
    // What the game was allowed to queue ran, in the end, and the game
    // lived through the refusals.
    let ran = run.wait_log("ran ");
    assert_eq!(ran, format!("ran {max}"), "{}", run.dump());
    assert_eq!(run.instance.status(), InstanceStatus::Running);
    assert_eq!(run.instance.error_count(), 0, "{}", run.dump());
}
