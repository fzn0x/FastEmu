//! [`Track`]: the sound behind one Java `Player` or Nokia `Sound`, either a
//! player of the instance's audio engine or a [`Silent`] one, with the
//! state the MIDP layer keeps beside it.

use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use fastemu_audio::{AudioError, Format, PlayerState};

use super::event::{Event, Note};
use super::silent::Silent;

/// Where a track's sound comes from.
pub(crate) enum Backend {
    /// A player in the instance's sound group: it plays, and the engine
    /// sends its events.
    Engine(fastemu_audio::Player),
    /// A player that keeps time and makes its own events.
    Silent(Silent),
}

impl Backend {
    /// A silent player whose media lasts `duration_us`.
    pub fn silent(duration_us: i64) -> Self {
        Self::Silent(Silent::new(duration_us))
    }
}

/// The smallest wait the event thread schedules for a stop time, so a
/// paused engine does not make it spin.
const MIN_STOP_POLL: Duration = Duration::from_millis(2);

/// The sound of one Java object, keyed by its `serial` in the media
/// registry (see [`super::events`]).
pub(crate) struct Track {
    serial: u64,
    /// Where silent events and the drop notice go; `None` for a track that
    /// was never registered.
    notes: Option<Sender<Note>>,
    /// `None` until the player is realized (or the sound initialized).
    backend: Option<Backend>,
    /// The format being played, when known: it decides which controls a
    /// player offers.
    format: Option<Format>,
    /// What `getContentType` reports.
    content_type: String,
    /// The volume level and mute, kept here as well so they survive the
    /// engine player closing under the game.
    level: i32,
    muted: bool,
    /// `StopTimeControl`'s stop time in microseconds.
    stop_time_us: Option<i64>,
    /// The media time an engine player was stopped at for its stop time:
    /// the `Stopped` event carrying it reaches listeners as
    /// `STOPPED_AT_TIME`.
    stopped_at_time: Option<i64>,
}

impl Default for Track {
    fn default() -> Self {
        Self::new(0, None)
    }
}

impl Track {
    /// A track with no sound yet.
    pub fn new(serial: u64, notes: Option<Sender<Note>>) -> Self {
        Self {
            serial,
            notes,
            backend: None,
            format: None,
            content_type: String::new(),
            level: 100,
            muted: false,
            stop_time_us: None,
            stopped_at_time: None,
        }
    }

    pub fn serial(&self) -> u64 {
        self.serial
    }

    /// Whether the track has a sound (a realized player, an initialized
    /// Nokia sound).
    pub fn has_backend(&self) -> bool {
        self.backend.is_some()
    }

    /// The engine's id of the player behind the track, if any.
    pub fn engine_id(&self) -> Option<fastemu_audio::PlayerId> {
        match &self.backend {
            Some(Backend::Engine(player)) => Some(player.id()),
            _ => None,
        }
    }

    /// The engine player behind the track, if any.
    pub fn engine(&self) -> Option<&fastemu_audio::Player> {
        match &self.backend {
            Some(Backend::Engine(player)) => Some(player),
            _ => None,
        }
    }

    pub fn format(&self) -> Option<Format> {
        self.format
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Gives the track its sound, replacing (and dropping) any earlier
    /// one, with the level and mute carried over. Returns the old one so
    /// the caller decides when it is dropped.
    pub fn install(
        &mut self,
        backend: Backend,
        format: Option<Format>,
        content_type: String,
    ) -> Option<Backend> {
        if let Backend::Engine(player) = &backend {
            if self.level != 100 {
                let _ = player.set_level(self.level);
            }
            if self.muted {
                let _ = player.set_muted(true);
            }
        }
        self.format = format;
        self.content_type = content_type;
        self.stopped_at_time = None;
        self.backend.replace(backend)
    }

    /// Takes the sound out of the track, leaving it without one.
    pub fn uninstall(&mut self) -> Option<Backend> {
        self.backend.take()
    }

    /// Sends an event the MIDP layer made, such as the `CLOSED` of a
    /// player that was never realized.
    pub fn announce(&self, event: Event) {
        self.send(vec![event]);
    }

    /// Sends the events a silent player made.
    fn send(&self, events: Vec<Event>) {
        if let Some(notes) = &self.notes {
            for event in events {
                // The event thread is gone only when the VM stops.
                let _ = notes.send(Note::Midp(self.serial, event));
            }
        }
    }

    /// Runs `f` on a silent backend with an event list, then sends the
    /// events; `engine` runs on an engine one.
    fn run<R>(
        &mut self,
        engine: impl FnOnce(&fastemu_audio::Player) -> Result<R, AudioError>,
        silent: impl FnOnce(&mut Silent, &mut Vec<Event>) -> Result<R, AudioError>,
    ) -> Result<R, AudioError> {
        match &mut self.backend {
            None => Err(AudioError::InvalidState("no sound yet".into())),
            Some(Backend::Engine(player)) => engine(player),
            Some(Backend::Silent(clock)) => {
                let mut events = Vec::new();
                let result = silent(clock, &mut events);
                self.send(events);
                result
            }
        }
    }

    /// The state now; `Closed` when the engine closed the player (its
    /// group closed) and `Realized` before there is a sound.
    pub fn state(&mut self, now: Instant) -> PlayerState {
        match &mut self.backend {
            None => PlayerState::Realized,
            Some(Backend::Engine(player)) => player.state(),
            Some(Backend::Silent(clock)) => {
                let mut events = Vec::new();
                clock.catch_up(now, &mut events);
                let state = clock.state();
                self.send(events);
                state
            }
        }
    }

    pub fn is_started(&mut self, now: Instant) -> bool {
        self.state(now) == PlayerState::Started
    }

    pub fn prefetch(&mut self) -> Result<(), AudioError> {
        self.run(fastemu_audio::Player::prefetch, |clock, _| clock.prefetch())
    }

    pub fn start(&mut self, now: Instant) -> Result<(), AudioError> {
        self.run(fastemu_audio::Player::start, |clock, out| {
            clock.start(now, out)
        })
    }

    pub fn stop(&mut self, now: Instant) -> Result<(), AudioError> {
        self.run(fastemu_audio::Player::stop, |clock, out| {
            clock.stop(now, out)
        })
    }

    pub fn deallocate(&mut self, now: Instant) -> Result<(), AudioError> {
        self.run(fastemu_audio::Player::deallocate, |clock, out| {
            clock.deallocate(now, out)
        })
    }

    /// Closes the sound; the engine (or the silent clock) sends `Closed`
    /// once. Without a sound, nothing happens.
    pub fn close(&mut self) {
        let _ = self.run(
            |player| {
                player.close();
                Ok(())
            },
            |clock, out| {
                clock.close(out);
                Ok(())
            },
        );
    }

    pub fn set_loop_count(&mut self, count: i32) -> Result<(), AudioError> {
        self.run(
            |player| player.set_loop_count(count),
            |clock, _| clock.set_loop_count(count),
        )
    }

    pub fn media_time(&mut self, now: Instant) -> Result<i64, AudioError> {
        self.run(fastemu_audio::Player::media_time, |clock, out| {
            clock.catch_up(now, out);
            Ok(clock.media_time_us(now))
        })
    }

    pub fn set_media_time(&mut self, time_us: i64, now: Instant) -> Result<i64, AudioError> {
        self.run(
            |player| player.set_media_time(time_us),
            |clock, out| clock.set_media_time(time_us, now, out),
        )
    }

    pub fn duration(&mut self) -> Result<i64, AudioError> {
        self.run(fastemu_audio::Player::duration, |clock, _| {
            Ok(clock.duration_us())
        })
    }

    /// Sets the volume level, clamped to 0 to 100, and returns it. A
    /// change sends `VOLUME_CHANGED`.
    pub fn set_level(&mut self, level: i32) -> i32 {
        let level = level.clamp(0, 100);
        let changed = level != self.level;
        self.level = level;
        let _ = self.run(
            |player| player.set_level(level).map(|_| ()),
            |_, out| {
                if changed {
                    out.push(Event::VolumeChanged);
                }
                Ok(())
            },
        );
        level
    }

    pub fn level(&self) -> i32 {
        self.level
    }

    pub fn set_muted(&mut self, muted: bool) {
        let changed = muted != self.muted;
        self.muted = muted;
        let _ = self.run(
            |player| player.set_muted(muted),
            |_, out| {
                if changed {
                    out.push(Event::VolumeChanged);
                }
                Ok(())
            },
        );
    }

    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Replaces a tone player's sequence, as `ToneControl.setSequence`.
    /// A silent tone player gets the length `silent_duration` worked out.
    pub fn set_tone_sequence(
        &mut self,
        sequence: &[u8],
        silent_duration: impl FnOnce(&[u8]) -> Result<i64, AudioError>,
    ) -> Result<(), AudioError> {
        self.run(
            |player| player.set_tone_sequence(sequence),
            |clock, _| {
                if clock.state() != PlayerState::Realized {
                    return Err(AudioError::InvalidState(
                        "the sequence can only change while realized".into(),
                    ));
                }
                clock.set_duration(silent_duration(sequence)?);
                Ok(())
            },
        )
    }

    /// `StopTimeControl`'s stop time.
    pub fn stop_time(&self) -> Option<i64> {
        self.stop_time_us
    }

    pub fn set_stop_time(&mut self, time_us: Option<i64>) {
        self.stop_time_us = time_us;
    }

    /// Whether an engine `Stopped` event at `time_us` is the stop at the
    /// stop time; a match is used up.
    pub fn take_stopped_at_time(&mut self, time_us: i64) -> bool {
        let matched = self.stopped_at_time == Some(time_us);
        if matched {
            self.stopped_at_time = None;
        }
        matched
    }

    /// Does what time brought: a silent player's loops and end, and the
    /// stop at the stop time.
    pub fn tick(&mut self, now: Instant) {
        if !self.is_started(now) {
            return;
        }
        let Some(stop_at) = self.stop_time_us else {
            return;
        };
        let Ok(time) = self.media_time(now) else {
            return;
        };
        if time < stop_at {
            return;
        }
        self.stop_time_us = None;
        match &mut self.backend {
            Some(Backend::Engine(player)) => {
                // A stop keeps the media time exactly, so the event carries
                // the time read afterwards.
                if player.stop().is_ok() {
                    self.stopped_at_time = player.media_time().ok();
                }
            }
            Some(Backend::Silent(clock)) => {
                let mut events = Vec::new();
                let _ = clock.stop(now, &mut events);
                for event in &mut events {
                    if let Event::Stopped(time) = *event {
                        *event = Event::StoppedAtTime(time);
                    }
                }
                self.send(events);
            }
            None => {}
        }
    }

    /// When [`Track::tick`] next has something to do, if ever.
    pub fn deadline(&mut self, now: Instant) -> Option<Instant> {
        if !self.is_started(now) {
            return None;
        }
        let silent_end = match &self.backend {
            Some(Backend::Silent(clock)) => clock.deadline(),
            _ => None,
        };
        let stop = self.stop_time_us.and_then(|stop_at| {
            let time = self.media_time(now).ok()?;
            let left = Duration::from_micros(stop_at.saturating_sub(time).max(0) as u64);
            Some(now + left.max(MIN_STOP_POLL))
        });
        match (silent_end, stop) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
}

impl Drop for Track {
    fn drop(&mut self) {
        if let Some(notes) = &self.notes {
            let _ = notes.send(Note::Gone(self.serial));
        }
    }
}
