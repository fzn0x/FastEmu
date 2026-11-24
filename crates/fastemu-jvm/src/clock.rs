//! A virtual machine's run time: a monotonic clock that stands still while
//! the machine is paused.
//!
//! `System.currentTimeMillis` is the wall clock, as games expect. Timing
//! the library keeps for the game (when a `Timer` task is due, how far a
//! silent media player has played) counts run time instead, so a pause
//! is invisible to it: a fixed-rate timer paused for a minute does not
//! run a minute of late tasks in a burst when the machine resumes.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// The pause flag and the times behind the run time.
pub(crate) struct Clock {
    /// Whether the machine is paused. Read without the lock at every
    /// safepoint; changed only with it held, with the times.
    paused: AtomicBool,
    /// `System.currentTimeMillis` when the run time was zero.
    wall_start_millis: i64,
    times: Mutex<Times>,
}

struct Times {
    start: Instant,
    /// The time spent paused before the current pause.
    paused_for: Duration,
    /// When the current pause began.
    paused_since: Option<Instant>,
}

impl Times {
    /// Every pause so far, the current one too.
    fn paused(&self, now: Instant) -> Duration {
        let current = self
            .paused_since
            .map_or(Duration::ZERO, |since| now.saturating_duration_since(since));
        self.paused_for + current
    }
}

impl Clock {
    pub(crate) fn new() -> Self {
        Clock {
            paused: AtomicBool::new(false),
            wall_start_millis: crate::lang::system::current_time_millis(),
            times: Mutex::new(Times {
                start: Instant::now(),
                paused_for: Duration::ZERO,
                paused_since: None,
            }),
        }
    }

    /// The wall time, as `System.currentTimeMillis` counts it, that the
    /// run time `run_millis` stands for: the wall time the machine started
    /// at, plus the run time, plus every pause so far. Times the library
    /// reports to the game (`TimerTask.scheduledExecutionTime`) come from
    /// here, so two run times a period apart map exactly a period apart,
    /// and a run time just reached maps to about the wall time now.
    pub(crate) fn wall_millis(&self, run_millis: i64) -> i64 {
        let paused = self.times.lock().paused(Instant::now());
        self.wall_start_millis
            .saturating_add(run_millis)
            .saturating_add(paused.as_millis() as i64)
    }

    #[inline]
    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Pauses or resumes the machine: sets the flag and stops or starts
    /// the run time with it, so the two never disagree.
    pub(crate) fn set_paused(&self, paused: bool) {
        let mut t = self.times.lock();
        let now = Instant::now();
        match (paused, t.paused_since) {
            (true, None) => t.paused_since = Some(now),
            (false, Some(since)) => {
                t.paused_for += now.saturating_duration_since(since);
                t.paused_since = None;
            }
            _ => {}
        }
        self.paused.store(paused, Ordering::Release);
    }

    fn elapsed_at(&self, now: Instant) -> Duration {
        let t = self.times.lock();
        now.saturating_duration_since(t.start)
            .saturating_sub(t.paused(now))
    }
}

/// A virtual machine's run time: how long it has run since it was made,
/// not counting the time it spent paused ([`crate::Vm::pause`]). It never
/// goes back, and stands still during a pause.
///
/// Use it for timing that must not catch up after a pause, such as when
/// something the game scheduled is due, or how far a player without sound
/// output has played. `System.currentTimeMillis` stays the wall clock.
/// Get one from [`crate::Ctx::clock`] or [`crate::Vm::clock`]; it is
/// cheap to clone, `Send` and `Sync`, and holds no reference to the
/// machine, so a payload or a host thread may keep it.
#[derive(Clone)]
pub struct VmClock(pub(crate) Arc<Clock>);

impl VmClock {
    /// How long the machine has run, pauses excluded.
    pub fn elapsed(&self) -> Duration {
        self.0.elapsed_at(Instant::now())
    }

    /// When, in host time, the run time reaches `at`, if the machine is
    /// not paused before then: a deadline for [`crate::Ctx::wait_until`].
    /// Now for a time already reached; `None` for one too far ahead for
    /// an [`Instant`].
    ///
    /// A pause moves the moment later, so a thread waiting for it checks
    /// [`VmClock::elapsed`] again when it wakes. While the machine is
    /// paused, waits return [`crate::Wake::Paused`] at once; wait for the
    /// resume (with no deadline) rather than for this instant.
    pub fn instant_at(&self, at: Duration) -> Option<Instant> {
        let now = Instant::now();
        now.checked_add(at.saturating_sub(self.0.elapsed_at(now)))
    }

    /// Whether the machine is paused, so the run time stands still.
    pub fn is_paused(&self) -> bool {
        self.0.is_paused()
    }
}

impl fmt::Debug for VmClock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VmClock")
            .field("elapsed", &self.elapsed())
            .field("paused", &self.is_paused())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> VmClock {
        VmClock(Arc::new(Clock::new()))
    }

    #[test]
    fn the_run_time_stands_still_while_paused() {
        let clock = clock();
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(20));
        clock.0.set_paused(true);
        let at_pause = clock.elapsed();
        assert!(at_pause >= Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(clock.elapsed(), at_pause, "no time passes while paused");
        assert!(clock.is_paused());
        // Pausing again does not restart the pause.
        clock.0.set_paused(true);
        clock.0.set_paused(false);
        clock.0.set_paused(false);
        std::thread::sleep(Duration::from_millis(10));
        let after = clock.elapsed();
        assert!(after >= at_pause + Duration::from_millis(10));
        assert!(
            after + Duration::from_millis(30) <= start.elapsed(),
            "the pause is left out: {after:?} of {:?}",
            start.elapsed()
        );
    }

    #[test]
    fn run_times_map_to_wall_times_a_fixed_distance_apart() {
        let clock = Clock::new();
        let now = crate::lang::system::current_time_millis();
        let at = |run| clock.wall_millis(run);
        assert!((at(0) - now).abs() <= 50, "{} and {now}", at(0));
        assert_eq!(at(120) - at(100), 20);
        let before = at(100);
        clock.set_paused(true);
        std::thread::sleep(Duration::from_millis(30));
        clock.set_paused(false);
        // The run time stood still while the wall clock went on.
        assert!(at(100) - before >= 30, "{}", at(100) - before);
        assert_eq!(at(120) - at(100), 20);
    }

    #[test]
    fn instants_line_up_with_the_run_time() {
        let clock = clock();
        let now = clock.elapsed();
        let soon = clock.instant_at(now + Duration::from_millis(50)).unwrap();
        let wait = soon.saturating_duration_since(Instant::now());
        assert!(wait > Duration::from_millis(40) && wait <= Duration::from_millis(50));
        let past = clock.instant_at(Duration::ZERO).unwrap();
        assert!(past <= Instant::now());
        assert_eq!(clock.instant_at(Duration::MAX), None);
    }
}
