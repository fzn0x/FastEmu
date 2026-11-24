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

#[cfg(test)]
mod tests {
    use super::*;

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
