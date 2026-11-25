//! How much a game may write to the disk, and how fast.
//!
//! The quota of a suite limits the bytes its stores hold, not the bytes it
//! writes: every change to a store writes the whole store file and flushes
//! it, so a game that rewrites one big record in a loop writes hundreds of
//! megabytes a second for as long as it runs. A [`WriteBudget`] is a token
//! bucket over those bytes. It starts full, so a game's first saves, and
//! any burst of saves, pass at the speed of the disk; a game that goes on
//! writing faster than the refill rate waits.
//!
//! The budget does no timing of its own. Callers give it the time, as the
//! run time of a [`Clock`], so a test drives it with a clock that sleeps
//! no time at all.

use std::time::{Duration, Instant};

/// What a game may write at once, from a full budget: 32 MiB.
pub const WRITE_BURST: u64 = 32 << 20;

/// What a game may write each second once the burst is spent: 4 MiB.
pub const WRITE_RATE: u64 = 4 << 20;

/// The longest a wait sleeps before the game's thread looks at the stop
/// flag again.
pub(crate) const SLICE: Duration = Duration::from_millis(50);

/// What a write costs at least: one block of a file system. A file of 40
/// bytes takes a block on the disk and a journal entry besides.
const MIN_COST: u64 = 4096;

/// A debt of less than this is no debt: it keeps the arithmetic of a wait
/// from ending in sleeps of a few nanoseconds.
const NEGLIGIBLE: f64 = 1.0;

/// A clock the budget is read against, and the sleep that goes with it.
pub(crate) trait Clock: Send + Sync {
    /// The time since the clock started. Never goes back.
    fn now(&self) -> Duration;

    /// Blocks the calling thread for `how_long`.
    fn sleep(&self, how_long: Duration);
}

/// The system's monotonic clock.
pub(crate) struct SystemClock {
    start: Instant,
}

impl SystemClock {
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep(&self, how_long: Duration) {
        std::thread::sleep(how_long);
    }
}

/// A token bucket of bytes: `burst` bytes at most, refilled at `rate` bytes
/// a second. A write is paid for after it is made, and may leave the
/// budget in debt; the next write waits for the debt to be paid.
pub(crate) struct WriteBudget {
    burst: f64,
    rate: f64,
    /// The bytes that may be written now; negative while in debt.
    tokens: f64,
    /// The time `tokens` was last brought up to date.
    at: Duration,
}

impl WriteBudget {
    /// A full budget at time `now`. A `rate` of 0 counts as 1 byte a
    /// second.
    pub(crate) fn new(burst: u64, rate: u64, now: Duration) -> Self {
        Self {
            burst: burst as f64,
            rate: rate.max(1) as f64,
            tokens: burst as f64,
            at: now,
        }
    }

    /// The bytes a second the budget refills at.
    pub(crate) fn rate(&self) -> u64 {
        self.rate as u64
    }

    fn refill(&mut self, now: Duration) {
        let elapsed = now.saturating_sub(self.at);
        self.at = self.at.max(now);
        self.tokens = (self.tokens + elapsed.as_secs_f64() * self.rate).min(self.burst);
    }

    /// How long a write has to wait at `now`: nothing while the budget is
    /// not in debt, and the time the refill takes to pay the debt after it.
    pub(crate) fn wait(&mut self, now: Duration) -> Duration {
        self.refill(now);
        if self.tokens > -NEGLIGIBLE {
            Duration::ZERO
        } else {
            Duration::from_secs_f64((-self.tokens / self.rate).min(86_400.0))
        }
    }

    /// Takes the bytes of a write that was made at `now` from the budget.
    /// A write costs at least [`MIN_COST`].
    pub(crate) fn spend(&mut self, bytes: u64, now: Duration) {
        self.refill(now);
        self.tokens -= bytes.max(MIN_COST) as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn budget() -> WriteBudget {
        WriteBudget::new(WRITE_BURST, WRITE_RATE, Duration::ZERO)
    }

    fn near(actual: Duration, expected: Duration) {
        let off = actual.abs_diff(expected);
        assert!(
            off < Duration::from_millis(1),
            "{actual:?} is not {expected:?}"
        );
    }

    #[test]
    fn the_burst_passes_without_a_wait() {
        let mut budget = budget();
        let mib = 1 << 20;
        // 32 writes of 1 MiB at the same instant: the whole burst.
        for _ in 0..32 {
            assert_eq!(budget.wait(Duration::ZERO), Duration::ZERO);
            budget.spend(mib, Duration::ZERO);
        }
        // Spent to the last byte, and not in debt.
        assert_eq!(budget.wait(Duration::ZERO), Duration::ZERO);
        // The next write is made, and the one after it waits.
        budget.spend(mib, Duration::ZERO);
        assert!(budget.wait(Duration::ZERO) > Duration::ZERO);
    }

    #[test]
    fn a_debt_waits_for_the_refill_to_pay_it() {
        let mut budget = budget();
        budget.spend(WRITE_BURST + WRITE_RATE, Duration::ZERO);
        // 4 MiB over at 4 MiB a second is one second.
        near(budget.wait(Duration::ZERO), SECOND);
        near(budget.wait(SECOND / 4), SECOND * 3 / 4);
        // Paid after the second; nothing is left over to save up.
        assert_eq!(budget.wait(SECOND), Duration::ZERO);
        budget.spend(WRITE_RATE, SECOND);
        near(budget.wait(SECOND), SECOND);
    }

    #[test]
    fn idle_time_restores_the_burst_and_no_more() {
        let mut budget = budget();
        budget.spend(WRITE_BURST, Duration::ZERO);
        // Ten seconds refill 40 MiB, which the burst cuts to 32 MiB.
        let later = SECOND * 10;
        budget.spend(WRITE_BURST, later);
        assert_eq!(budget.wait(later), Duration::ZERO);
        budget.spend(WRITE_RATE, later);
        near(budget.wait(later), SECOND);
    }

    #[test]
    fn a_clock_that_goes_back_gives_nothing() {
        let mut budget = budget();
        budget.spend(WRITE_BURST + WRITE_RATE, SECOND * 5);
        near(budget.wait(SECOND * 5), SECOND);
        near(budget.wait(SECOND), SECOND);
    }

    #[test]
    fn a_small_write_costs_a_block() {
        let mut budget = WriteBudget::new(8192, 4096, Duration::ZERO);
        budget.spend(40, Duration::ZERO);
        budget.spend(1, Duration::ZERO);
        // Two blocks gone, and a third makes a debt of one block.
        assert_eq!(budget.wait(Duration::ZERO), Duration::ZERO);
        budget.spend(0, Duration::ZERO);
        near(budget.wait(Duration::ZERO), SECOND);
    }

    #[test]
    fn a_rate_of_zero_still_refills() {
        let mut budget = WriteBudget::new(0, 0, Duration::ZERO);
        assert_eq!(budget.rate(), 1);
        budget.spend(MIN_COST, Duration::ZERO);
        near(budget.wait(Duration::ZERO), SECOND * MIN_COST as u32);
    }

    /// A game that adds a thousand records of 100 bytes one at a time
    /// writes a store that grows to 112 KB, with a name of 32 characters: 56 MB
    /// in all.
    #[test]
    fn a_thousand_small_records_wait_a_few_seconds_at_most() {
        let mut budget = budget();
        let mut now = Duration::ZERO;
        let mut waited = Duration::ZERO;
        for records in 1..=1000u64 {
            let wait = budget.wait(now);
            waited += wait;
            now += wait;
            budget.spend(34 + 64 + 112 * records, now);
        }
        assert!(
            waited > Duration::ZERO && waited < Duration::from_secs(6),
            "{waited:?}"
        );
    }

    /// Saves that stay under the rate never wait, however long they go on.
    #[test]
    fn saves_under_the_rate_are_never_slowed() {
        let mut budget = budget();
        let mut now = Duration::ZERO;
        for _ in 0..10_000 {
            // A 100 KiB save every 100 ms is 1 MiB a second.
            assert_eq!(budget.wait(now), Duration::ZERO);
            budget.spend(100 << 10, now);
            now += Duration::from_millis(100);
        }
    }
}
