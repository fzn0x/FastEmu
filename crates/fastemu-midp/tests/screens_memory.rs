//! What the high-level screens keep natively counts against the game's
//! heap. A list or choice group that takes elements without end, text
//! boxes and fields made from one large string, and a text box that takes
//! one string again and again make almost no Java objects, so a game
//! could once hold gigabytes this way. Each test runs a small MIDlet from
//! `tests/screens_java` on a phone with a 1 MiB heap, the smallest there
//! is, and checks that the game gets `OutOfMemoryError` and that code
//! which gives its memory back, or never keeps it, does not.

mod screens_support;
mod support;

use fastemu_core::{DeviceProfile, default_profile};
use screens_support::{midlet, suite};
use support::Running;

/// The phone: a heap of 1 MiB.
fn small_heap() -> DeviceProfile {
    let mut profile = default_profile();
    profile.heap_bytes = 1 << 20;
    profile
}

/// Runs `midlet` of the suite `name` until it prints `done`, and returns
/// what it printed, or `None` (after saying why) without a JDK.
fn play(name: &str, midlet: usize, done: &str) -> Option<Vec<String>> {
    let jar = suite(name)?;
    let mut run = Running::launch_on(&jar, midlet, small_heap());
    run.wait_log(done);
    Some(run.lines())
}

/// The line of `lines` that starts with `prefix`.
fn said<'a>(lines: &'a [String], prefix: &str) -> &'a str {
    lines
        .iter()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| {
            panic!(
                "no line starts with {prefix:?}; the log:\n{}",
                lines.join("\n")
            )
        })
}

/// The number a line ends with, after `prefix`: how far a loop got.
fn reached(lines: &[String], prefix: &str, what: &str) -> usize {
    let line = said(lines, prefix);
    let rest = line
        .strip_prefix(prefix)
        .expect("it starts with the prefix");
    assert!(
        rest.starts_with(what),
        "expected {prefix}{what} ..., got {line:?}"
    );
    rest[what.len()..].trim().parse().expect("a number")
}

#[test]
fn elements_added_without_end_end_in_out_of_memory() {
    let Some(lines) = play("memory-lists", midlet::MEM_LISTS, "lists done") else {
        return;
    };
    // One String, appended again and again: 100000 calls is far more than
    // a 1 MiB heap holds, and far more than a game does.
    for kind in ["list: ", "group: "] {
        let calls = reached(&lines, kind, "oom after ");
        assert!(
            (1000..100_000).contains(&calls),
            "{kind}ran out after {calls} calls"
        );
    }
    // Deleting the elements gives their room back; lists nobody keeps are
    // collected with their elements.
    assert_eq!(said(&lines, "reuse: "), "reuse: ok 0");
    assert_eq!(said(&lines, "dropped: "), "dropped: ok 200");
}

#[test]
fn text_boxes_made_from_one_large_string_end_in_out_of_memory() {
    let Some(lines) = play("memory-texts", midlet::MEM_TEXTS, "texts done") else {
        return;
    };
    // 50 boxes of 20000 characters are 2 MB of text on a 1 MiB heap.
    for kind in ["boxes: ", "fields: "] {
        let kept = reached(&lines, kind, "oom after ");
        assert!((2..50).contains(&kept), "{kind}ran out after {kept}");
    }
    // The same 500 boxes, each dropped, are collected as they go; the text
    // of a box that is set again replaces its old text.
    assert_eq!(said(&lines, "dropped boxes: "), "dropped boxes: ok 500");
    assert_eq!(said(&lines, "set again: "), "set again: ok 20000");
}

#[test]
fn a_text_box_with_the_largest_maximum_size_ends_in_out_of_memory() {
    let Some(lines) = play("memory-insert", midlet::MEM_INSERT, "inserts done") else {
        return;
    };
    assert_eq!(said(&lines, "max size: "), "max size: 2147483647");
    // 1000 inserts of 1000 characters are 2 MB of text.
    for kind in ["box: ", "field: "] {
        let calls = reached(&lines, kind, "oom after ");
        assert!((10..1000).contains(&calls), "{kind}ran out after {calls}");
    }
    // Text put in and taken out again stays small.
    assert_eq!(said(&lines, "steady: "), "steady: ok 0");
}
