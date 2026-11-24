//! Event order (`docs/compat/notes/02-events-threads.md`): `sizeChanged`
//! before `showNotify` before the first `paint`; `callSerially` runnables
//! after the pending paint; `serviceRepaints` painting before it returns
//! when called from a callback, a timer task and a game thread; and on
//! `setCurrent`, the new canvas's `sizeChanged`, the old one's
//! `hideNotify`, then the new one's `showNotify` and `paint`.

use fastemu_core::{Key, KeyAction};

use crate::conformance_support::{Jar, assert_lines, compile, manifest, scratch, tagged};
use crate::support::{Running, rgb};

#[test]
fn callbacks_come_in_the_order_midp_gives() {
    let Some(classes) = compile("events") else {
        return;
    };
    let dir = scratch("events");
    let jar = Jar::new()
        .manifest(manifest("ev.Order", ""))
        .classes(&classes)
        .write(&dir.join("events.jar"));
    let mut run = Running::launch(&jar, 0);
    run.wait_log("ev ready for keys");
    // The game thread's serviceRepaints presented its paint, the fourth.
    run.wait_frame(|f| rgb(f, 0, 0) == 0x20_4004);
    run.key(Key::Num1, KeyAction::Press);
    run.wait_log("ev c2 paint 1");
    run.wait_frame(|f| rgb(f, 0, 0) == 0x00_00F1);
    let log = run.lines();
    let want = [
        "startApp",
        "c1 sizeChanged 240x320",
        "c1 showNotify",
        "c1 paint 1",
        "serial A",
        "c1 paint 2",
        "serial A done",
        "serial B",
        "timer",
        "c1 paint 3",
        "timer done",
        "thread",
        "c1 paint 4",
        "thread done",
        "ready for keys",
        "c1 keyPressed 49",
        "c1 paint 5",
        "c1 keyPressed served",
        "c1 keyPressed done",
        "c1 paint 6",
        "serial C",
        "c2 sizeChanged 240x320",
        "c1 hideNotify",
        "c2 showNotify",
        "c2 paint 1",
    ];
    assert_lines(&tagged(&log, "ev "), &want, &log);
    assert_eq!(run.instance.error_count(), 0, "{}", log.join("\n"));
}
