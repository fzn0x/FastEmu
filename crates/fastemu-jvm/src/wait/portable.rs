//! A wakeup signal from a mutex and a condition variable. Its timeouts are
//! as precise as the OS's: well under a millisecond on Linux, Android and
//! macOS, whose waits take nanosecond deadlines. On Windows it is only
//! the fallback, as there they wake on the system timer tick.

use std::time::Instant;

use parking_lot::{Condvar, Mutex};

/// A flag one thread waits on and others raise.
#[derive(Default)]
pub(super) struct Signal {
    raised: Mutex<bool>,
    cond: Condvar,
}

impl Signal {
    /// Lowers the flag, forgetting a raise no wait took.
    pub(super) fn reset(&self) {
        *self.raised.lock() = false;
    }

    /// Raises the flag: wakes the waiting thread, or makes its next wait
    /// return at once.
    pub(super) fn raise(&self) {
        *self.raised.lock() = true;
        self.cond.notify_one();
    }

    /// Waits until the flag is raised, and lowers it, or until `deadline`
    /// passes. Returns whether the deadline passed first.
    pub(super) fn wait(&self, deadline: Option<Instant>) -> bool {
        let mut raised = self.raised.lock();
        loop {
            if std::mem::take(&mut *raised) {
                return false;
            }
            match deadline {
                None => self.cond.wait(&mut raised),
                Some(d) => {
                    if self.cond.wait_until(&mut raised, d).timed_out() {
                        return !std::mem::take(&mut *raised);
                    }
                }
            }
        }
    }
}
