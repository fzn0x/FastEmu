//! [`VmLimits`]: the limits a virtual machine puts on what a game can
//! make it use, in one struct.

/// Limits on the host resources a game can make its virtual machine use,
/// other than the Java heap, which [`crate::VmConfig::heap_bytes`] sizes.
///
/// Every limit has a default that no game in the test corpus comes near.
/// The struct is plain, so a test writes `VmLimits { max_threads: 8,
/// ..VmLimits::default() }` to lower one. Each field says which change
/// enforces it; the fields are read only, never changed, while the
/// machine runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VmLimits {
    /// Java threads a game may have alive at once, counting its main
    /// thread. `Thread.start` and `Timer` throw `OutOfMemoryError` past
    /// it. Enforced by #36.
    pub max_threads: usize,
    /// Slots (locals and operand stack entries) one thread's stack may
    /// hold, so deep recursion ends in `StackOverflowError` before it
    /// holds much memory. Enforced by #28.
    pub max_stack_slots: usize,
    /// Bytes of class data (parsed classes and their code) one machine
    /// may hold; loading more throws `OutOfMemoryError`. Enforced by #35.
    pub max_class_bytes: usize,
    /// Longest line, in bytes, the machine hands the log sink: a longer
    /// line from `System.out`, `System.err` or the machine itself is split
    /// or cut. Enforced by #30.
    pub max_log_line_bytes: usize,
    /// Different diagnostics (missing classes, methods and fields) the
    /// machine reports; after that it says once that it stops. Enforced
    /// by #30.
    pub max_reported: usize,
    /// Tasks posted to the machine and not yet run (`Display.callSerially`
    /// and `Vm::post`); later ones are dropped. Enforced by #30.
    pub max_posted: usize,
}

impl Default for VmLimits {
    fn default() -> Self {
        Self {
            max_threads: 64,
            max_stack_slots: 131_072,
            max_class_bytes: 128 * 1024 * 1024,
            max_log_line_bytes: 4096,
            max_reported: 256,
            max_posted: 10_000,
        }
    }
}

/// Room a cut line leaves for its marker, in bytes.
const MARKER_ROOM: usize = 48;

/// Cuts `text` to at most `max_bytes` bytes, at a character boundary, and
/// ends it with a marker that says how many characters went: ` ... (12
/// more characters)`. A text that fits is returned as it is, and a cut
/// text fits, so cutting twice changes nothing the second time. A limit
/// too small to hold a marker (under 96 bytes) cuts without one.
///
/// The log sink's lines are cut this way (see
/// [`VmLimits::max_log_line_bytes`]), and so are the lines a host adds.
pub fn cut_log_text(mut text: String, max_bytes: usize) -> String {
    use std::fmt::Write;

    if text.len() <= max_bytes {
        return text;
    }
    let marker = max_bytes >= 2 * MARKER_ROOM;
    let mut at = if marker {
        max_bytes - MARKER_ROOM
    } else {
        max_bytes
    };
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let more = text[at..].chars().count();
    text.truncate(at);
    if marker {
        let _ = write!(text, " ... ({more} more characters)");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_that_fits_is_not_cut() {
        assert_eq!(cut_log_text("hello".into(), 5), "hello");
        assert_eq!(cut_log_text(String::new(), 0), "");
    }

    #[test]
    fn a_long_text_is_cut_to_the_limit_with_a_marker() {
        let cut = cut_log_text("x".repeat(10_000), 4096);
        let marker = " ... (5952 more characters)";
        assert_eq!(cut.len(), 4096 - MARKER_ROOM + marker.len());
        assert!(cut.len() <= 4096);
        assert!(cut.ends_with(marker), "{cut:?}");
        assert!(cut.starts_with("xxxx"));
    }

    #[test]
    fn a_cut_never_splits_a_character() {
        // Every character takes three bytes, so the cut lands inside one.
        let cut = cut_log_text("\u{20ac}".repeat(5000), 4096);
        assert!(cut.len() <= 4096);
        let (kept, marker) = cut.split_once(" ... (").unwrap();
        assert!(marker.ends_with(" more characters)"));
        assert!(kept.chars().all(|c| c == '\u{20ac}'));
    }

    #[test]
    fn cutting_twice_changes_nothing() {
        let once = cut_log_text("y".repeat(9000), 4096);
        assert_eq!(cut_log_text(once.clone(), 4096), once);
    }

    #[test]
    fn a_tiny_limit_cuts_without_a_marker() {
        assert_eq!(cut_log_text("abcdefgh".into(), 3), "abc");
        assert_eq!(cut_log_text("\u{20ac}\u{20ac}".into(), 4), "\u{20ac}");
    }

    #[test]
    fn defaults_leave_room_for_real_games() {
        let limits = VmLimits::default();
        assert_eq!(limits.max_threads, 64);
        assert_eq!(limits.max_stack_slots, 131_072);
        assert_eq!(limits.max_class_bytes, 128 << 20);
        assert_eq!(limits.max_log_line_bytes, 4096);
        assert_eq!(limits.max_reported, 256);
        assert_eq!(limits.max_posted, 10_000);
    }

    #[test]
    fn one_limit_can_change_alone() {
        let limits = VmLimits {
            max_threads: 8,
            ..VmLimits::default()
        };
        assert_eq!(limits.max_threads, 8);
        assert_eq!(limits.max_posted, VmLimits::default().max_posted);
    }
}
