//! The render budget: how much of the audio thread's time the sound of
//! each game may take.
//!
//! The audio thread mixes every game's sound into one buffer, and the
//! device needs that buffer before it has played the last one. One game
//! with dozens of MIDI players can take most of that time, and then every
//! game's sound breaks up. With a budget, the games that play sound share
//! a part of real time: each gets an equal part, and what a game leaves
//! unused goes to the games that need more.
//!
//! Each slice, a game's sounds are offered in the order they started. Its
//! oldest sound always plays. The others play while their measured cost
//! fits in the game's part of the slice. One sound more may play past the
//! part while the game has credit, the render time it saved before, so
//! the game uses its whole part on average. The sounds after it wait,
//! silent and in place, until the game's sound fits again.
//!
//! The mixer reads the time from a [`Clock`]. The apps use the system's
//! clock; tests use one that the sounds themselves move, so the budget
//! does the same thing on every machine.

use std::time::{Duration, Instant};

/// How much of real time the sound of all games may take to render, set
/// with [`Engine::set_render_budget`](crate::Engine::set_render_budget).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBudget {
    /// The part of real time the sound of all games together may take to
    /// render on average, from 0.0 to 1.0. Each game that plays sound gets
    /// an equal part of it, and the part a game leaves unused goes to the
    /// others. The oldest sound of each game plays whatever it costs.
    pub share: f32,
    /// The render time a game may save, and the debt it may run up: a
    /// game that saved this much lets one more sound play past its part
    /// for a while, and a spike shorter than it holds no sound back.
    pub burst: Duration,
}

impl RenderBudget {
    /// Half of real time for all games, and 20 ms of render time saved.
    /// The heaviest game music measured plays in about 1% of real time, so
    /// a game that a phone could play stays far inside its part.
    pub const DEFAULT: Self = Self {
        share: 0.5,
        burst: Duration::from_millis(20),
    };

    /// The most credit a game may hold, and the most debt, in nanoseconds
    /// of render time.
    pub(crate) fn cap_ns(self) -> i64 {
        i64::try_from(self.burst.as_nanos()).map_or(i64::MAX / 4, |ns| ns.min(i64::MAX / 4))
    }

    /// What all games together may spend on `slice_ns` of sound.
    pub(crate) fn allowance_ns(self, slice_ns: u64) -> i64 {
        let share = if self.share.is_nan() {
            0.0
        } else {
            f64::from(self.share.clamp(0.0, 1.0))
        };
        (slice_ns as f64 * share) as i64
    }
}

impl Default for RenderBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Where the mixer reads the time it spends rendering.
pub(crate) trait Clock: Send + Sync {
    /// Nanoseconds since a fixed point.
    fn now_ns(&self) -> u64;
}

/// The system's monotonic clock.
pub(crate) struct SystemClock {
    origin: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

/// One game's account in a slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Share {
    /// The render time the game may still spend, in nanoseconds: what it
    /// saved, or a debt when negative.
    pub credit: i64,
    /// The part of this slice's allowance the game was given.
    pub part: i64,
}

/// Shares `allowance` nanoseconds out among `shares`, one for each game
/// with sound to render this slice: an equal part each, and what would
/// take a game's credit past `cap` goes in equal parts to the games under
/// it. A game's `part` is what it was given, before its own cap.
pub(crate) fn refill(shares: &mut [Share], allowance: i64, cap: i64) {
    if shares.is_empty() {
        return;
    }
    let part = allowance / shares.len() as i64;
    let mut spill = 0i64;
    for share in shares.iter_mut() {
        share.part = part;
        share.credit = share.credit.saturating_add(part);
        if share.credit > cap {
            spill = spill.saturating_add(share.credit - cap);
            share.credit = cap;
        }
    }
    let needy = shares.iter().filter(|s| s.credit < cap).count() as i64;
    if needy > 0 && spill > 0 {
        let each = spill / needy;
        for share in shares.iter_mut().filter(|s| s.credit < cap) {
            share.part = share.part.saturating_add(each);
            share.credit = share.credit.saturating_add(each).min(cap);
        }
    }
}

/// Takes `cost_ns` from `credit`: never more than `slice_ns`, the real
/// length of the slice, so a thread the system stopped for a while in the
/// middle of a render charges no more than the slice, and never below
/// `-cap`, so a game comes back within a bounded time.
pub(crate) fn charge(credit: &mut i64, cost_ns: u64, slice_ns: u64, cap: i64) {
    let cost = i64::try_from(cost_ns.min(slice_ns)).unwrap_or(i64::MAX);
    *credit = credit.saturating_sub(cost).max(-cap);
}

/// The running average of what a frame of a sound costs, in picoseconds,
/// after a slice that cost `latest`: an eighth of the way from `average`,
/// or `latest` itself for a sound never measured (an `average` of 0). It
/// is never 0 once measured.
pub(crate) fn average(average: u64, latest: u64) -> u64 {
    let next = if average == 0 {
        latest
    } else {
        average.saturating_mul(7).saturating_add(latest) / 8
    };
    next.max(1)
}

/// What `frames` frames cost at `per_frame_ps` picoseconds a frame, in
/// nanoseconds and at least 1: no sound is free.
pub(crate) fn frames_ns(per_frame_ps: u64, frames: usize) -> i64 {
    let ns = u128::from(per_frame_ps) * frames as u128 / 1000;
    i64::try_from(ns).unwrap_or(i64::MAX).max(1)
}

/// One game's turn in a slice: which of its sounds play.
///
/// The mixer offers the game's sounds in the order they started, with
/// [`Turn::offer`], and reports each one that rendered with
/// [`Turn::played`]. The first sound always plays. A later one plays when
/// the estimates of the sounds before it and its own fit in the game's
/// part, less a debt. A sound that waited needs an eighth of the part to
/// spare, and a sound that played may run an eighth past it, so a sound
/// at the edge does not flip on every change in the estimates. Past the
/// part, sounds that were playing go on while the game has credit, and
/// one sound that waited may come back once the game has saved half the
/// burst. The first sound that does not play stops the turn: every sound
/// after it waits too.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Turn {
    /// What the game may spend this slice: its part, less an eighth of
    /// its debt.
    limit: i64,
    /// An eighth of the limit.
    margin: i64,
    /// Half the burst: the credit a sound that waited needs to come back
    /// past the part.
    half: i64,
    /// The estimates of the sounds that played so far.
    spent: i64,
    /// The dearest estimate known in the game: a sound never measured is
    /// counted at it.
    dearest: i64,
    /// Whether the oldest sound is still to come.
    first: bool,
    /// Whether a sound plays past the part.
    past: bool,
    /// Whether a sound waited, so every later one waits.
    closed: bool,
}

impl Turn {
    /// A turn for a game given `share`, whose dearest sound measured so
    /// far costs `dearest` nanoseconds this slice (0 for none).
    pub fn new(budget: RenderBudget, share: Share, dearest: i64) -> Self {
        let limit = share.part.saturating_add(share.credit.min(0) / 8);
        Self {
            limit,
            margin: limit.max(0) / 8,
            half: budget.cap_ns() / 2,
            spent: 0,
            dearest,
            first: true,
            past: false,
            closed: false,
        }
    }

    /// Whether the next sound plays. `estimate` is what it should cost
    /// this slice, `None` if it was never measured; `waiting` says that
    /// it did not play the last slice (or has not played since it
    /// started); `credit` is the game's credit now.
    pub fn offer(&mut self, estimate: Option<i64>, waiting: bool, credit: i64) -> bool {
        if self.closed {
            return false;
        }
        if self.first {
            self.first = false;
            return true;
        }
        let cost = estimate.unwrap_or(self.dearest);
        let room = if waiting {
            self.limit - self.margin
        } else {
            self.limit + self.margin
        };
        if self.spent.saturating_add(cost) <= room {
            return true;
        }
        let plays = credit > 0 && (!waiting || (!self.past && credit >= self.half));
        self.past |= plays;
        self.closed |= !plays;
        plays
    }

    /// Counts a sound that rendered, at its estimate after this slice.
    pub fn played(&mut self, estimate: i64) {
        self.spent = self.spent.saturating_add(estimate);
        self.dearest = self.dearest.max(estimate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credits(shares: &[Share]) -> Vec<i64> {
        shares.iter().map(|s| s.credit).collect()
    }

    fn parts(shares: &[Share]) -> Vec<i64> {
        shares.iter().map(|s| s.part).collect()
    }

    fn with_credits(credits: &[i64]) -> Vec<Share> {
        credits
            .iter()
            .map(|&credit| Share { credit, part: 0 })
            .collect()
    }

    #[test]
    fn each_game_gets_an_equal_part() {
        let mut shares = with_credits(&[0, 0, 0, 0]);
        refill(&mut shares, 400, 1000);
        assert_eq!(credits(&shares), [100, 100, 100, 100]);
        assert_eq!(parts(&shares), [100, 100, 100, 100]);
    }

    #[test]
    fn what_a_full_game_leaves_goes_to_the_others() {
        // The first two are full: their parts go to the two that are not.
        let mut shares = with_credits(&[1000, 1000, 0, -500]);
        refill(&mut shares, 400, 1000);
        assert_eq!(credits(&shares), [1000, 1000, 200, -300]);
        assert_eq!(parts(&shares), [100, 100, 200, 200]);
    }

    #[test]
    fn no_game_passes_the_cap() {
        let mut shares = with_credits(&[990, 0]);
        refill(&mut shares, 4000, 1000);
        assert_eq!(credits(&shares), [1000, 1000]);
        // Both are full, so what they spill goes to no one.
        assert_eq!(parts(&shares), [2000, 2000]);
        let mut none: [Share; 0] = [];
        refill(&mut none, 4000, 1000);
    }

    #[test]
    fn a_charge_is_at_most_the_slice_and_stops_at_minus_the_cap() {
        let mut credit = 500;
        charge(&mut credit, 1_000_000_000, 4_000, 10_000);
        assert_eq!(credit, -3_500);
        charge(&mut credit, 4_000, 4_000, 10_000);
        charge(&mut credit, 4_000, 4_000, 10_000);
        assert_eq!(credit, -10_000);
    }

    #[test]
    fn the_cost_of_a_sound_is_a_running_average() {
        assert_eq!(average(0, 800), 800);
        assert_eq!(average(800, 0), 700);
        assert_eq!(average(800, 1600), 900);
        // Measured once, never "never measured" again.
        assert_eq!(average(0, 0), 1);
        assert_eq!(average(1, 0), 1);
        assert_eq!(frames_ns(2_000, 128), 256);
        assert_eq!(frames_ns(1, 128), 1);
    }

    #[test]
    fn a_nan_share_gives_nothing_and_a_huge_burst_is_capped() {
        let budget = RenderBudget {
            share: f32::NAN,
            burst: Duration::MAX,
        };
        assert_eq!(budget.allowance_ns(4000), 0);
        assert_eq!(budget.cap_ns(), i64::MAX / 4);
        let over = RenderBudget {
            share: 7.0,
            ..RenderBudget::DEFAULT
        };
        assert_eq!(over.allowance_ns(4000), 4000);
        assert_eq!(RenderBudget::DEFAULT.allowance_ns(4000), 2000);
    }

    /// A turn with a part of 800 ns, no debt, and a burst of 1000 ns.
    fn turn(credit: i64) -> Turn {
        let budget = RenderBudget {
            share: 0.5,
            burst: Duration::from_nanos(1000),
        };
        Turn::new(budget, Share { credit, part: 800 }, 0)
    }

    /// Offers sounds of `cost` each, playing or waiting as `waiting` says,
    /// until one waits; returns how many played.
    fn offer_all(turn: &mut Turn, cost: i64, waiting: &[bool], credit: i64) -> usize {
        let mut played = 0;
        for &w in waiting {
            if !turn.offer(Some(cost), w, credit) {
                break;
            }
            turn.played(cost);
            played += 1;
        }
        played
    }

    #[test]
    fn sounds_play_while_they_fit_in_the_part() {
        // A part of 800: sounds of 200 that played fit up to 900 (an
        // eighth over), sounds that waited up to 700 (an eighth under).
        assert_eq!(offer_all(&mut turn(0), 200, &[false; 8], 0), 4);
        assert_eq!(offer_all(&mut turn(0), 200, &[true; 8], 0), 3);
        // The oldest plays whatever it costs.
        assert_eq!(offer_all(&mut turn(0), 5000, &[true; 8], 0), 1);
    }

    #[test]
    fn past_the_part_sounds_that_played_go_on_while_there_is_credit() {
        assert_eq!(offer_all(&mut turn(1), 200, &[false; 8], 1), 8);
        assert_eq!(offer_all(&mut turn(0), 200, &[false; 8], 0), 4);
        assert_eq!(offer_all(&mut turn(-1), 200, &[false; 8], -1), 4);
    }

    #[test]
    fn one_sound_that_waited_comes_back_past_the_part_at_half_the_burst() {
        // At half the burst one comes back, and only one.
        assert_eq!(offer_all(&mut turn(500), 200, &[true; 8], 500), 4);
        assert_eq!(offer_all(&mut turn(499), 200, &[true; 8], 499), 3);
        // Not after another sound already plays past the part.
        let waiting = [false, false, false, false, false, true, true];
        assert_eq!(offer_all(&mut turn(1000), 200, &waiting, 1000), 5);
    }

    #[test]
    fn the_first_sound_that_waits_holds_back_every_later_one() {
        let mut t = turn(0);
        assert!(t.offer(Some(100), false, 0));
        t.played(100);
        assert!(!t.offer(Some(5000), true, 0));
        // A cheap sound after it waits too: sounds wait newest first.
        assert!(!t.offer(Some(1), false, 0));
    }

    #[test]
    fn a_sound_never_measured_is_counted_at_the_dearest() {
        let mut t = turn(0);
        assert!(t.offer(None, true, 0));
        t.played(500);
        // Counted at 500, it does not fit beside the first.
        assert!(!t.offer(None, true, 0));
        // A game whose dearest sound is known before the turn.
        let budget = RenderBudget {
            share: 0.5,
            burst: Duration::from_nanos(1000),
        };
        let mut t = Turn::new(
            budget,
            Share {
                credit: 0,
                part: 800,
            },
            750,
        );
        assert!(t.offer(Some(10), false, 0));
        t.played(10);
        assert!(!t.offer(None, true, 0));
    }

    #[test]
    fn a_debt_takes_its_eighth_off_the_part() {
        let budget = RenderBudget {
            share: 0.5,
            burst: Duration::from_nanos(1000),
        };
        let mut t = Turn::new(
            budget,
            Share {
                credit: -800,
                part: 800,
            },
            0,
        );
        // A limit of 700, and an eighth of that, 87, over it.
        assert_eq!(offer_all(&mut t, 200, &[false; 8], -800), 3);
    }
}
