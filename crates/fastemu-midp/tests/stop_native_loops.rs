//! A game stuck in a loop inside the runtime: `InputStream.skip` over a
//! stream that never ends. It holds the VM lock for ever, since no call in
//! the loop reaches a safepoint, but closing its tile still ends it, and
//! so does a pause that comes before.
//!
//! The MIDlet is `t.StopLoop` of the shared test programs, in a suite of
//! its own. Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fastemu_core::{Instance, InstanceStatus};
use support::{Running, classes, scratch, suite_entries, zip};

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Stop\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: StopLoop, , t.StopLoop\r\n";

/// How long a stop may take: the control thread gives up on the VM lock
/// for the lifecycle call after a second, and the loop then ends.
const STOP_WITHIN: Duration = Duration::from_secs(5);

fn suite(name: &str) -> Option<PathBuf> {
    let classes = classes()?;
    let path = scratch(name).join("stop.jar");
    let jar = zip(&suite_entries(classes, MANIFEST), true);
    std::fs::write(&path, jar).expect("writing the JAR");
    Some(path)
}

/// Launches the MIDlet and waits until it is in its loop.
fn looping(name: &str) -> Option<Running> {
    let jar = suite(name)?;
    let mut run = Running::launch(&jar, 0);
    run.wait_log("looping");
    // The loop starts a few instructions after the line.
    std::thread::sleep(Duration::from_millis(50));
    Some(run)
}

#[test]
fn closing_the_tile_of_a_game_in_a_native_loop_ends_it() {
    let Some(mut run) = looping("stop-loop") else {
        return;
    };
    let asked = Instant::now();
    run.instance.stop();
    run.wait_status(|s| *s == InstanceStatus::Exited);
    let took = asked.elapsed();
    assert!(took < STOP_WITHIN, "the stop took {took:?}");
    assert!(!run.has("skip returned"), "{}", run.dump());
}

#[test]
fn a_game_in_a_native_loop_can_be_paused_and_then_closed() {
    let Some(mut run) = looping("pause-loop") else {
        return;
    };
    // The pause cannot park the loop, but it ends: the lifecycle calls it
    // makes give up on the lock instead of waiting for it.
    run.instance.pause();
    run.wait_status(|s| *s == InstanceStatus::Paused);
    let asked = Instant::now();
    run.instance.stop();
    run.wait_status(|s| *s == InstanceStatus::Exited);
    let took = asked.elapsed();
    assert!(took < STOP_WITHIN, "the stop took {took:?}");
}
