//! A player that plays nothing but keeps time: what a game gets for sound
//! FastEmu cannot decode, and for every sound of an instance without an
//! audio engine.
//!
//! It follows the engine's player state machine exactly (realized,
//! prefetched, started, closed; loops; media time; the end of media), with
//! the wall clock standing in for the audio thread. Nothing moves on its
//! own: [`Silent::catch_up`] works out what happened since the last call,
//! and the media event thread calls it when [`Silent::deadline`] passes.

use std::time::{Duration, Instant};

use fastemu_audio::{AudioError, PlayerState};

use super::event::Event;

/// How long a sound FastEmu cannot decode lasts. Games that wait for
/// `END_OF_MEDIA` (an intro jingle, a voice line) carry on after it.
pub(crate) const UNKNOWN_LENGTH_US: i64 = 1_000_000;

/// At most this many loop passes are reported by one catch-up; longer gaps
/// (an instance paused for a minute over a short looping sound) jump ahead
/// instead of flooding the listeners.
const MAX_PASSES_REPORTED: u32 = 4;

/// How many passes start after the current one for a loop count: one less
/// than the count, or -1 (forever) for any negative count.
fn wraps_for(loop_count: i32) -> i32 {
    if loop_count < 0 { -1 } else { loop_count - 1 }
}

fn micros(duration: Duration) -> i64 {
    i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)
}

/// The silent player's clock and state.
#[derive(Debug)]
pub(crate) struct Silent {
    state: PlayerState,
    duration_us: i64,
    loop_count: i32,
    /// Passes to start after the current one; -1 forever.
    wraps: i32,
    /// The media time at `since` while started, or now otherwise.
    position_us: i64,
    /// When the current stretch of playing began.
    since: Option<Instant>,
    /// Whether the last run reached the end; the next start begins again.
    at_end: bool,
}

impl Silent {
    /// A realized player whose media lasts `duration_us`.
    pub fn new(duration_us: i64) -> Self {
        Self {
            state: PlayerState::Realized,
            duration_us: duration_us.max(0),
            loop_count: 1,
            wraps: 0,
            position_us: 0,
            since: None,
            at_end: false,
        }
    }

    fn open(&self) -> Result<(), AudioError> {
        if self.state == PlayerState::Closed {
            Err(AudioError::Closed)
        } else {
            Ok(())
        }
    }

    pub fn state(&self) -> PlayerState {
        self.state
    }

    pub fn duration_us(&self) -> i64 {
        self.duration_us
    }

    /// The media time at `now`.
    pub fn media_time_us(&self, now: Instant) -> i64 {
        match self.since {
            Some(since) => (self.position_us + micros(now.saturating_duration_since(since)))
                .min(self.duration_us),
            None => self.position_us,
        }
    }

    /// When the current pass ends, while started.
    pub fn deadline(&self) -> Option<Instant> {
        let since = self.since?;
        let left = (self.duration_us - self.position_us).max(0);
        Some(since + Duration::from_micros(left as u64))
    }

    /// Applies the loops and the end that passed by `now`, adding their
    /// events to `out`.
    pub fn catch_up(&mut self, now: Instant, out: &mut Vec<Event>) {
        let mut reported = 0;
        while let Some(end) = self.deadline() {
            if now < end {
                return;
            }
            if self.wraps != 0 && self.duration_us > 0 {
                if self.wraps > 0 {
                    self.wraps -= 1;
                }
                if reported < MAX_PASSES_REPORTED {
                    out.push(Event::EndOfMedia {
                        time_us: self.duration_us,
                        last: false,
                    });
                    out.push(Event::Started(0));
                    reported += 1;
                    self.position_us = 0;
                    self.since = Some(end);
                } else {
                    // Far behind: skip the whole passes that went by
                    // unheard, as many as the loops left allow.
                    let mut passes = micros(now - end) / self.duration_us;
                    if self.wraps >= 0 {
                        passes = passes.min(i64::from(self.wraps));
                        self.wraps -= passes as i32;
                    }
                    self.position_us = 0;
                    let skipped = Duration::from_micros((passes * self.duration_us) as u64);
                    self.since = Some(end + skipped);
                }
            } else {
                out.push(Event::EndOfMedia {
                    time_us: self.duration_us,
                    last: true,
                });
                self.state = PlayerState::Prefetched;
                self.at_end = true;
                self.position_us = self.duration_us;
                self.since = None;
                self.wraps = wraps_for(self.loop_count);
            }
        }
    }

    pub fn prefetch(&mut self) -> Result<(), AudioError> {
        self.open()?;
        if self.state == PlayerState::Realized {
            self.state = PlayerState::Prefetched;
        }
        Ok(())
    }

    /// Starts from the media time, or from the start after the end.
    pub fn start(&mut self, now: Instant, out: &mut Vec<Event>) -> Result<(), AudioError> {
        self.open()?;
        self.catch_up(now, out);
        if self.state == PlayerState::Started {
            return Ok(());
        }
        if self.at_end {
            self.position_us = 0;
            self.at_end = false;
            self.wraps = wraps_for(self.loop_count);
        }
        self.state = PlayerState::Started;
        self.since = Some(now);
        out.push(Event::Started(self.position_us));
        Ok(())
    }

    /// Stops and keeps the media time; nothing happens if not started.
    pub fn stop(&mut self, now: Instant, out: &mut Vec<Event>) -> Result<(), AudioError> {
        self.open()?;
        self.catch_up(now, out);
        if self.state != PlayerState::Started {
            return Ok(());
        }
        self.position_us = self.media_time_us(now);
        self.since = None;
        self.state = PlayerState::Prefetched;
        out.push(Event::Stopped(self.position_us));
        Ok(())
    }

    /// Goes back to realized, stopping first if started.
    pub fn deallocate(&mut self, now: Instant, out: &mut Vec<Event>) -> Result<(), AudioError> {
        self.stop(now, out)?;
        self.state = PlayerState::Realized;
        Ok(())
    }

    /// Closes the player; closing twice adds nothing.
    pub fn close(&mut self, out: &mut Vec<Event>) {
        if self.state != PlayerState::Closed {
            self.state = PlayerState::Closed;
            self.since = None;
            out.push(Event::Closed);
        }
    }

    /// Sets the loop count for the next run: -1 or any negative forever.
    pub fn set_loop_count(&mut self, count: i32) -> Result<(), AudioError> {
        self.open()?;
        if count == 0 {
            return Err(AudioError::InvalidArgument("a loop count of 0".into()));
        }
        if self.state == PlayerState::Started {
            return Err(AudioError::InvalidState(
                "the loop count cannot change while playing".into(),
            ));
        }
        self.loop_count = count.max(-1);
        self.wraps = wraps_for(self.loop_count);
        Ok(())
    }

    /// Jumps to a media time, clamped to the media, and returns it.
    pub fn set_media_time(
        &mut self,
        time_us: i64,
        now: Instant,
        out: &mut Vec<Event>,
    ) -> Result<i64, AudioError> {
        self.open()?;
        self.catch_up(now, out);
        if self.at_end {
            self.wraps = wraps_for(self.loop_count);
        }
        self.at_end = false;
        self.position_us = time_us.clamp(0, self.duration_us);
        if self.since.is_some() {
            self.since = Some(now);
        }
        Ok(self.position_us)
    }

    /// Replaces the media with one of `duration_us`, as a tone player's new
    /// sequence does; time goes back to the start.
    pub fn set_duration(&mut self, duration_us: i64) {
        self.duration_us = duration_us.max(0);
        self.position_us = 0;
        self.at_end = false;
        self.wraps = wraps_for(self.loop_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn pass(time_us: i64) -> Event {
        Event::EndOfMedia {
            time_us,
            last: false,
        }
    }

    fn end(time_us: i64) -> Event {
        Event::EndOfMedia {
            time_us,
            last: true,
        }
    }

    #[test]
    fn plays_to_the_end_and_starts_again() {
        let t0 = Instant::now();
        let mut player = Silent::new(500_000);
        let mut out = Vec::new();
        player.prefetch().unwrap();
        player.start(t0, &mut out).unwrap();
        assert_eq!(out, vec![Event::Started(0)]);
        assert_eq!(player.media_time_us(t0 + ms(200)), 200_000);
        assert_eq!(player.deadline(), Some(t0 + ms(500)));
        out.clear();
        player.catch_up(t0 + ms(499), &mut out);
        assert!(out.is_empty());
        player.catch_up(t0 + ms(800), &mut out);
        assert_eq!(out, vec![end(500_000)]);
        assert_eq!(player.state(), PlayerState::Prefetched);
        assert_eq!(player.media_time_us(t0 + ms(900)), 500_000);
        out.clear();
        player.start(t0 + ms(1000), &mut out).unwrap();
        assert_eq!(out, vec![Event::Started(0)]);
    }

    #[test]
    fn loops_report_each_pass() {
        let t0 = Instant::now();
        let mut player = Silent::new(100_000);
        player.set_loop_count(3).unwrap();
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        player.catch_up(t0 + ms(350), &mut out);
        assert_eq!(
            out,
            vec![
                Event::Started(0),
                pass(100_000),
                Event::Started(0),
                pass(100_000),
                Event::Started(0),
                end(100_000),
            ]
        );
        assert_eq!(player.state(), PlayerState::Prefetched);
    }

    #[test]
    fn a_long_gap_over_endless_loops_jumps_ahead() {
        let t0 = Instant::now();
        let mut player = Silent::new(10_000);
        player.set_loop_count(-1).unwrap();
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        out.clear();
        player.catch_up(t0 + Duration::from_secs(60) + ms(3), &mut out);
        assert_eq!(out.len(), 2 * MAX_PASSES_REPORTED as usize);
        assert_eq!(player.state(), PlayerState::Started);
        assert_eq!(
            player.media_time_us(t0 + Duration::from_secs(60) + ms(3)),
            3_000
        );
    }

    #[test]
    fn stop_keeps_the_time_and_is_quiet_when_not_started() {
        let t0 = Instant::now();
        let mut player = Silent::new(1_000_000);
        let mut out = Vec::new();
        player.stop(t0, &mut out).unwrap();
        assert!(out.is_empty());
        player.start(t0, &mut out).unwrap();
        player.stop(t0 + ms(300), &mut out).unwrap();
        assert_eq!(out, vec![Event::Started(0), Event::Stopped(300_000)]);
        assert_eq!(player.media_time_us(t0 + ms(900)), 300_000);
        out.clear();
        player.start(t0 + ms(1000), &mut out).unwrap();
        assert_eq!(out, vec![Event::Started(300_000)]);
    }

    #[test]
    fn refuses_what_the_engine_refuses() {
        let t0 = Instant::now();
        let mut player = Silent::new(1_000);
        assert!(matches!(
            player.set_loop_count(0),
            Err(AudioError::InvalidArgument(_))
        ));
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        assert!(matches!(
            player.set_loop_count(2),
            Err(AudioError::InvalidState(_))
        ));
        player.close(&mut out);
        player.close(&mut out);
        assert_eq!(out.iter().filter(|e| **e == Event::Closed).count(), 1);
        assert_eq!(player.prefetch(), Err(AudioError::Closed));
        assert_eq!(player.start(t0, &mut out), Err(AudioError::Closed));
    }

    #[test]
    fn seeking_clamps_and_moves_the_deadline() {
        let t0 = Instant::now();
        let mut player = Silent::new(1_000_000);
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        assert_eq!(player.set_media_time(-5, t0, &mut out), Ok(0));
        assert_eq!(player.set_media_time(600_000, t0, &mut out), Ok(600_000));
        assert_eq!(player.deadline(), Some(t0 + ms(400)));
        assert_eq!(
            player.set_media_time(9_000_000, t0, &mut out),
            Ok(1_000_000)
        );
        // At the end, the next look finds the run over.
        player.catch_up(t0, &mut out);
        assert_eq!(out.last(), Some(&end(1_000_000)));
        assert_eq!(player.state(), PlayerState::Prefetched);
    }

    #[test]
    fn a_long_gap_over_counted_loops_still_ends() {
        let t0 = Instant::now();
        let mut player = Silent::new(10_000);
        player.set_loop_count(100).unwrap();
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        out.clear();
        player.catch_up(t0 + Duration::from_secs(5), &mut out);
        assert_eq!(player.state(), PlayerState::Prefetched);
        assert_eq!(out.len(), 2 * MAX_PASSES_REPORTED as usize + 1);
        assert_eq!(out.last(), Some(&end(10_000)));
    }

    #[test]
    fn an_empty_media_ends_at_once_even_when_looping() {
        let t0 = Instant::now();
        let mut player = Silent::new(0);
        player.set_loop_count(-1).unwrap();
        let mut out = Vec::new();
        player.start(t0, &mut out).unwrap();
        player.catch_up(t0, &mut out);
        assert_eq!(out, vec![Event::Started(0), end(0)]);
    }
}
