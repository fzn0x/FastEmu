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
//! fits in the game's part of the slice. Past the part, the sounds that
//! were playing go on while the game has credit, the render time it saved
//! before, so the game uses its whole part on average. The sounds after
//! them wait, silent and in place, until the game's sound fits again.
//!
//! The costs a game is judged by are running averages, slow to rise. So
//! the clock also keeps a game's pace: what its sounds after the oldest
//! really took. Once they have taken the pace of a slice, every later
//! sound of the game waits, whatever its average says. A game that saved
//! time thus spends it over many slices, not in a few.
//!
//! The mixer reads the time from a [`Clock`]. The apps use the system's
//! clock; tests use one that the sounds themselves move, so the budget
//! does the same thing on every machine.

use std::time::{Duration, Instant};

use crate::engine::SLICE_FRAMES;

/// How much of real time the sound of all games may take to render, set
/// with [`Engine::set_render_budget`](crate::Engine::set_render_budget).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderBudget {
    /// The part of real time the sound of all games together may take to
    /// render on average, from 0.0 to 1.0. Each game that plays sound gets
    /// an equal part of it, and the part a game leaves unused goes to the
    /// others. The oldest sound of each game plays whatever it costs.
    pub share: f32,
    /// The render time a game may save, and the debt it may run up. A game
    /// spends what it saved past its part a little each slice, and a spike
    /// shorter than it holds no sound back.
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

/// What the sounds of a game after its oldest may take by the clock in a
/// slice.
///
/// Each slice the pace grows by the game's part and a quarter more. So a
/// game past its part spends the time it saved a quarter of its part a
/// slice, its oldest sound aside, and the games together take not much
/// more than the share in any slice. It holds at most two whole slices'
/// worth: a sound reads its source two whole slices at a time, so sounds
/// started together cost twice their average in one slice and little in
/// the next. That holds however short the slice is: the last slice of a
/// buffer is often a few frames, and a sound may read a whole block in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pace {
    /// What the sounds may take this slice, in nanoseconds; less than 0
    /// after slices that took more.
    room: i64,
    /// The most the pace holds.
    cap: i64,
    /// Whether the pace was full when the slice began.
    full: bool,
}

impl Pace {
    /// No pace at all: the sounds may take any time.
    const UNLIMITED: Self = Self {
        room: i64::MAX,
        cap: i64::MAX,
        full: true,
    };

    /// The pace of a game whose `part` of this slice of `frames` frames is
    /// given, after the slices before left `left` of it (`i64::MAX` for a
    /// game new to the budget, which starts full).
    pub fn new(left: i64, part: i64, frames: usize) -> Self {
        let part = part.max(0);
        let rate = part.saturating_add(part / 4);
        let whole = match frames {
            1..SLICE_FRAMES => rate.saturating_mul(SLICE_FRAMES as i64) / frames as i64,
            _ => rate,
        };
        let cap = whole.saturating_mul(2);
        let room = left.saturating_add(rate).min(cap);
        Self {
            room,
            cap,
            full: room >= cap,
        }
    }
}

/// One game's turn in a slice: which of its sounds play.
///
/// The mixer offers the game's sounds in the order they started, with
/// [`Turn::offer`], and reports each one that rendered with
/// [`Turn::rendered`]. The first sound always plays. A later one plays
/// when the estimates of the sounds before it and its own fit in the
/// game's part, less a debt. A sound that waited needs an eighth of the
/// part to spare, and a sound that played may run an eighth past it, so a
/// sound at the edge does not flip on every change in the estimates. Past
/// the part, sounds that were playing go on while the game has credit, and
/// one sound that waited may come back once the game has saved half the
/// burst. The first sound that does not play stops the turn: every sound
/// after it waits too.
///
/// Whatever the estimates say, no sound after the first plays once those
/// that played before it took the game's [`Pace`] by the clock, and a
/// sound that waited comes back only when the pace was full. The render
/// that took most past its estimate counts only as far past it as the
/// render that took the second most: when the system pauses the thread,
/// one render takes the pause, while sounds that grow dearer do so many at
/// once. And a sound plays only if the pace holds that second most on top
/// of what was taken, since it may well take as much past its own.
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
    /// What the sounds after the first may take by the clock.
    pace: Pace,
    /// What the sounds after the first that played took by the clock.
    took: i64,
    /// The most one of them took past its estimate.
    spike: i64,
    /// The second most one of them took past its estimate: what the next
    /// sound is expected to take past its own.
    second: i64,
    /// The estimate of the sound just let in after the first, until it
    /// rendered.
    expected: Option<i64>,
    /// Whether the oldest sound is still to come.
    first: bool,
    /// Whether a sound plays past the part.
    past: bool,
    /// Whether a sound waited, so every later one waits.
    closed: bool,
}

impl Turn {
    /// A turn for a game given `share`, whose dearest sound measured so
    /// far costs `dearest` nanoseconds this slice (0 for none), with no
    /// pace until [`Turn::with_pace`].
    pub fn new(budget: RenderBudget, share: Share, dearest: i64) -> Self {
        let limit = share.part.saturating_add(share.credit.min(0) / 8);
        Self {
            limit,
            margin: limit.max(0) / 8,
            half: budget.cap_ns() / 2,
            spent: 0,
            dearest,
            pace: Pace::UNLIMITED,
            took: 0,
            spike: 0,
            second: 0,
            expected: None,
            first: true,
            past: false,
            closed: false,
        }
    }

    /// The same turn, held to `pace`.
    pub fn with_pace(self, pace: Pace) -> Self {
        Self { pace, ..self }
    }

    /// Whether the next sound plays. `estimate` is what it should cost
    /// this slice, `None` if it was never measured; `waiting` says that
    /// it did not play the last slice (or has not played since it
    /// started); `credit` is the game's credit now.
    pub fn offer(&mut self, estimate: Option<i64>, waiting: bool, credit: i64) -> bool {
        self.expected = None;
        if self.closed {
            return false;
        }
        if self.first {
            self.first = false;
            return true;
        }
        // The next sound may take as much past its estimate as the sound
        // that took the second most past its own, so the pace must hold
        // that much more.
        let taken = self.taken().saturating_add(self.second);
        if taken > self.pace.room || (waiting && !self.pace.full) {
            self.closed = true;
            return false;
        }
        let cost = estimate.unwrap_or(self.dearest);
        let room = if waiting {
            self.limit - self.margin
        } else {
            self.limit + self.margin
        };
        let plays = self.spent.saturating_add(cost) <= room || {
            let plays = credit > 0 && (!waiting || (!self.past && credit >= self.half));
            self.past |= plays;
            plays
        };
        self.closed |= !plays;
        if plays {
            self.expected = Some(cost);
        }
        plays
    }

    /// Counts a sound that rendered: at `estimate`, its estimate after
    /// this slice, and, if it was let in after the first sound, at
    /// `took`, what it took by the clock.
    pub fn rendered(&mut self, estimate: i64, took: i64) {
        self.spent = self.spent.saturating_add(estimate);
        self.dearest = self.dearest.max(estimate);
        if let Some(expected) = self.expected.take() {
            self.took = self.took.saturating_add(took);
            let over = took.saturating_sub(expected);
            if over > self.spike {
                self.second = self.spike;
                self.spike = over;
            } else {
                self.second = self.second.max(over);
            }
        }
    }

    /// What the sounds after the first took by the clock, counting the one
    /// that ran most past its estimate only as far past it as the next.
    fn taken(&self) -> i64 {
        self.took.saturating_sub(self.spike - self.second)
    }

    /// Counts a sound that rendered and took its estimate.
    #[cfg(test)]
    pub fn played(&mut self, estimate: i64) {
        self.rendered(estimate, estimate);
    }

    /// What is left of the pace for the next slice: never less than
    /// minus what it holds, so a game comes back within a few slices.
    pub fn pace_left(&self) -> i64 {
        self.pace
            .room
            .saturating_sub(self.taken())
            .max(self.pace.cap.saturating_neg())
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

    #[test]
    fn the_pace_grows_by_a_part_and_a_quarter_up_to_two_slices() {
        // A game new to the budget starts with a full pace.
        let new = Pace::new(i64::MAX, 800, SLICE_FRAMES);
        assert_eq!((new.room, new.cap, new.full), (2000, 2000, true));
        // After a slice that took all of it, a part and a quarter comes
        // back; after slices that took more, less.
        let after = Pace::new(0, 800, SLICE_FRAMES);
        assert_eq!((after.room, after.full), (1000, false));
        assert_eq!(Pace::new(-1500, 800, SLICE_FRAMES).room, -500);
        assert!(Pace::new(1000, 800, SLICE_FRAMES).full);
        assert_eq!(Pace::new(0, 0, SLICE_FRAMES).room, 0);
    }

    #[test]
    fn a_short_slice_holds_two_whole_slices_of_pace() {
        // The last 2 frames of a buffer: a sixty-fourth of the part, but
        // the pace still holds two whole slices' worth, and a whole slice
        // after it starts full.
        let short = Pace::new(i64::MAX, 800 * 2 / 128, 2);
        assert_eq!((short.room, short.cap, short.full), (1920, 1920, true));
        let next = Pace::new(short.room, 800, SLICE_FRAMES);
        assert!(next.full);
    }

    /// Offers sounds that each cost 100 by their estimate but take `took`
    /// by the clock, the oldest first, until one waits; returns how many
    /// played. The game has credit and its estimates fit in its part.
    fn offer_paced(turn: &mut Turn, took: &[i64]) -> usize {
        let mut played = 0;
        for &took in took {
            if !turn.offer(Some(100), false, 1000) {
                break;
            }
            turn.rendered(100, took);
            played += 1;
        }
        played
    }

    #[test]
    fn no_sound_plays_once_those_before_it_took_the_pace() {
        // A pace of 2000. Every sound takes 600 by the clock, 500 past its
        // estimate, and the oldest is not counted at all. Alone, the first
        // sound after it counts its estimate, 100; once two have played,
        // each counts 600. So the sounds after the oldest took 100, 1200
        // and 1800, and the fifth sound, which may take 500 past its
        // estimate as they did, finds 1800 and 500 more past the pace.
        let full = Pace::new(i64::MAX, 800, SLICE_FRAMES);
        let mut t = turn(1000).with_pace(full);
        assert_eq!(offer_paced(&mut t, &[600; 8]), 4);
        assert_eq!(t.pace_left(), 200);
        // With no pace, the estimates alone let all eight play.
        assert_eq!(offer_paced(&mut turn(1000), &[600; 8]), 8);
    }

    #[test]
    fn the_render_that_took_most_past_its_estimate_counts_as_far_as_the_next() {
        // The system pauses the thread in the third sound's render: it
        // takes 5000 by the clock, 4900 past its estimate, while the others
        // take their estimates. The sounds after it play on.
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        let took = [100, 100, 5000, 100, 100, 100];
        assert_eq!(offer_paced(&mut t, &took), 6);
        assert_eq!(t.pace_left(), 2000 - 500);
        // The next render takes 1000 past its estimate: the pause counts
        // that far past its own too, and the pace is taken.
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        let took = [100, 100, 5000, 1100, 100];
        assert_eq!(offer_paced(&mut t, &took), 4);
        assert_eq!(t.pace_left(), 2000 - 2300);
        // Two renders run past their estimates, by 500 and then by 1000:
        // the one that ran most counts as far as the one before it.
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        assert_eq!(offer_paced(&mut t, &[100, 600, 1100, 100]), 4);
        assert_eq!(t.pace_left(), 2000 - 1300);
        // Two such pauses: the second counts, and the pace is taken.
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        let took = [100, 5000, 5000, 100, 100];
        assert_eq!(offer_paced(&mut t, &took), 3);
    }

    #[test]
    fn a_sound_that_waited_comes_back_only_on_a_full_pace() {
        // The pace is short of full after a slice that took it all: a
        // sound that played goes on, one that waited does not come back,
        // though both fit in the part.
        let short = Pace::new(0, 800, SLICE_FRAMES);
        let mut t = turn(1000).with_pace(short);
        assert_eq!(
            offer_all(&mut t, 100, &[false, false, true, false], 1000),
            2
        );
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        assert_eq!(
            offer_all(&mut t, 100, &[false, false, true, false], 1000),
            4
        );
    }

    #[test]
    fn a_pace_left_stops_at_minus_what_it_holds() {
        let mut t = turn(1000).with_pace(Pace::new(i64::MAX, 800, SLICE_FRAMES));
        // The oldest, then one sound that takes far more than its pace
        // twice over, with a second one so the first is not forgiven.
        assert_eq!(offer_paced(&mut t, &[100, 9000, 9000]), 3);
        assert_eq!(t.pace_left(), -2000);
    }
}
