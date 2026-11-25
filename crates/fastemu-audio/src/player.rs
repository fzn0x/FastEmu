//! Players: the engine side of MMAPI's `Player`, `VolumeControl`,
//! `ToneControl` and `MIDIControl`.

use std::fmt;

use crate::engine::Engine;
use crate::error::AudioError;
use crate::event::{PlayerEventKind, PlayerId};
use crate::format::Format;
use crate::media;
use crate::mixer::{GroupId, Mixer};
use crate::tone::sequence;
use crate::voice::{PlayerState, Voice};

/// How many players one game may have started at once. A phone played a
/// few sounds at a time; each started player takes a share of every
/// buffer the audio thread mixes, which all games wait for.
pub const MAX_STARTED_PER_GROUP: usize = 32;

/// How many players all games together may have started at once.
pub const MAX_STARTED_TOTAL: usize = 128;

/// Fails if `group` or the engine already plays as many players as it
/// may. Players that ended, stopped or never started do not count, and
/// neither do the tones of `playTone`, which a group caps on its own.
fn check_started_room(mixer: &Mixer, group: GroupId) -> Result<(), AudioError> {
    let (mut here, mut all) = (0, 0);
    for slot in mixer.slots.values() {
        if slot.voice.state == PlayerState::Started && !slot.voice.one_shot {
            all += 1;
            here += usize::from(slot.group == group);
        }
    }
    if here >= MAX_STARTED_PER_GROUP {
        return Err(AudioError::Busy(format!(
            "a game may have {MAX_STARTED_PER_GROUP} players started at once"
        )));
    }
    if all >= MAX_STARTED_TOTAL {
        return Err(AudioError::Busy(format!(
            "all games together may have {MAX_STARTED_TOTAL} players started at once"
        )));
    }
    Ok(())
}

/// One sound a game can start, stop, loop and seek. Created by a
/// [`Group`](crate::Group); dropping it closes it, which frees its data.
///
/// Every method returns quickly and may be called from any thread. Calls
/// on a closed player (closed directly, or through its group) fail with
/// [`AudioError::Closed`], except [`Player::state`] and
/// [`Player::close`].
pub struct Player {
    engine: Engine,
    id: PlayerId,
    format: Format,
}

impl Player {
    pub(crate) fn new(engine: Engine, id: PlayerId, format: Format) -> Self {
        Self { engine, id, format }
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }

    pub fn format(&self) -> Format {
        self.format
    }

    /// The content type `Player.getContentType` reports.
    pub fn content_type(&self) -> &'static str {
        self.format.content_type()
    }

    /// Runs `f` on the player's voice with the mixer locked, then delivers
    /// the events it queued.
    fn with<R>(
        &self,
        f: impl FnOnce(&mut Mixer, &crate::event::EventQueue) -> Result<R, AudioError>,
    ) -> Result<R, AudioError> {
        let mut mixer = self.engine.lock();
        if !mixer.slots.contains_key(&self.id) {
            return Err(AudioError::Closed);
        }
        let result = f(&mut mixer, &self.engine.shared.events);
        drop(mixer);
        self.engine.deliver();
        result
    }

    fn voice<'m>(&self, mixer: &'m mut Mixer) -> &'m mut Voice {
        &mut mixer
            .slots
            .get_mut(&self.id)
            .expect("checked by with")
            .voice
    }

    pub fn state(&self) -> PlayerState {
        self.engine
            .lock()
            .slots
            .get(&self.id)
            .map_or(PlayerState::Closed, |slot| slot.voice.state)
    }

    /// Moves from realized to prefetched. Decoding happened when the
    /// player was created, so this only changes the state.
    pub fn prefetch(&self) -> Result<(), AudioError> {
        self.with(|mixer, _| {
            let voice = self.voice(mixer);
            if voice.state == PlayerState::Realized {
                voice.state = PlayerState::Prefetched;
            }
            Ok(())
        })
    }

    /// Starts playing from the media time, or from the start if the last
    /// run reached the end. Does nothing if already started. Fails with
    /// [`AudioError::Busy`] while the game has [`MAX_STARTED_PER_GROUP`]
    /// players started, or all games together [`MAX_STARTED_TOTAL`].
    pub fn start(&self) -> Result<(), AudioError> {
        self.with(|mixer, events| {
            let voice = self.voice(mixer);
            if voice.state == PlayerState::Started {
                return Ok(());
            }
            let group = mixer.slots.get(&self.id).expect("checked by with").group;
            check_started_room(mixer, group)?;
            let voice = self.voice(mixer);
            voice.start();
            let media_time_us = voice.media_time_us();
            mixer.emit(events, self.id, PlayerEventKind::Started { media_time_us });
            Ok(())
        })
    }

    /// Stops playing and keeps the media time, with a short fade so the
    /// stop does not click. Does nothing if not started.
    pub fn stop(&self) -> Result<(), AudioError> {
        self.with(|mixer, events| {
            self.stop_locked(mixer, events);
            Ok(())
        })
    }

    fn stop_locked(&self, mixer: &mut Mixer, events: &crate::event::EventQueue) {
        if self.voice(mixer).state != PlayerState::Started {
            return;
        }
        mixer.fade_out(self.id);
        let voice = self.voice(mixer);
        voice.state = PlayerState::Prefetched;
        let media_time_us = voice.media_time_us();
        mixer.emit(events, self.id, PlayerEventKind::Stopped { media_time_us });
    }

    /// Goes back to realized, stopping first if started.
    pub fn deallocate(&self) -> Result<(), AudioError> {
        self.with(|mixer, events| {
            self.stop_locked(mixer, events);
            self.voice(mixer).state = PlayerState::Realized;
            Ok(())
        })
    }

    /// Stops the player, frees everything it holds and sends `Closed`.
    /// Closing twice is harmless.
    pub fn close(&self) {
        let _ = self.with(|mixer, events| {
            mixer.fade_out(self.id);
            mixer.emit(events, self.id, PlayerEventKind::Closed);
            mixer.slots.remove(&self.id);
            Ok(())
        });
    }

    /// Sets how many times the media plays: -1 (or any negative) loops
    /// forever, 0 is refused, and the player must not be started.
    pub fn set_loop_count(&self, count: i32) -> Result<(), AudioError> {
        if count == 0 {
            return Err(AudioError::InvalidArgument("a loop count of 0".into()));
        }
        self.with(|mixer, _| {
            let voice = self.voice(mixer);
            if voice.state == PlayerState::Started {
                return Err(AudioError::InvalidState(
                    "the loop count cannot change while playing".into(),
                ));
            }
            voice.set_loop_count(count.max(-1));
            Ok(())
        })
    }

    /// The media time in microseconds.
    pub fn media_time(&self) -> Result<i64, AudioError> {
        self.with(|mixer, _| Ok(self.voice(mixer).media_time_us()))
    }

    /// Moves to a media time in microseconds, clamped to the media, and
    /// returns the time actually set. A playing player jumps without a
    /// click.
    pub fn set_media_time(&self, time_us: i64) -> Result<i64, AudioError> {
        self.with(|mixer, _| {
            let playing = self.voice(mixer).state == PlayerState::Started;
            if playing {
                mixer.fade_out(self.id);
            }
            let voice = self.voice(mixer);
            let set = voice.seek_us(time_us);
            if playing {
                voice.gain = 0.0;
            }
            Ok(set)
        })
    }

    /// The length of the media in microseconds.
    pub fn duration(&self) -> Result<i64, AudioError> {
        self.with(|mixer, _| Ok(self.voice(mixer).duration_us()))
    }

    /// Sets the volume level, clamped to 0 to 100, and returns it, as
    /// `VolumeControl.setLevel`. Level 50 sounds half as loud as 100.
    pub fn set_level(&self, level: i32) -> Result<i32, AudioError> {
        let level = level.clamp(0, 100) as u8;
        self.with(|mixer, events| {
            let voice = self.voice(mixer);
            if voice.level != level {
                voice.level = level;
                let muted = voice.muted;
                mixer.emit(
                    events,
                    self.id,
                    PlayerEventKind::VolumeChanged {
                        level: i32::from(level),
                        muted,
                    },
                );
            }
            Ok(i32::from(level))
        })
    }

    pub fn level(&self) -> Result<i32, AudioError> {
        self.with(|mixer, _| Ok(i32::from(self.voice(mixer).level)))
    }

    /// Mutes or unmutes the player, keeping its level.
    pub fn set_muted(&self, muted: bool) -> Result<(), AudioError> {
        self.with(|mixer, events| {
            let voice = self.voice(mixer);
            if voice.muted != muted {
                voice.muted = muted;
                let level = i32::from(voice.level);
                mixer.emit(
                    events,
                    self.id,
                    PlayerEventKind::VolumeChanged { level, muted },
                );
            }
            Ok(())
        })
    }

    pub fn is_muted(&self) -> Result<bool, AudioError> {
        self.with(|mixer, _| Ok(self.voice(mixer).muted))
    }

    /// Replaces the notes of a tone sequence player, as
    /// `ToneControl.setSequence`. The player must be realized; a bad
    /// sequence is refused with the byte where it breaks.
    pub fn set_tone_sequence(&self, sequence_bytes: &[u8]) -> Result<(), AudioError> {
        if self.format != Format::ToneSequence {
            return Err(AudioError::InvalidState("not a tone player".into()));
        }
        let tones = sequence::parse(sequence_bytes)?;
        let rate = self.engine.sample_rate();
        let source = media::tones(tones, rate);
        self.with(|mixer, _| {
            let slot = mixer.slots.get(&self.id).expect("checked by with");
            if slot.voice.state != PlayerState::Realized {
                return Err(AudioError::InvalidState(
                    "the sequence can only change while realized".into(),
                ));
            }
            // The new notes replace the old ones, which count no more. A
            // sequence of 65536 tones is 1.5 MiB, and a game can make 128
            // players that each take one.
            let (group, old) = (slot.group, slot.voice.source.memory_bytes());
            crate::group::admit(mixer, group, source.memory_bytes(), old, false)?;
            self.voice(mixer).set_source(source);
            Ok(())
        })
    }

    /// Sends a short MIDI message to a MIDI player's synthesizer, as
    /// `MIDIControl.shortMidiEvent`: a status byte from 0x80 to 0xFF
    /// except the system exclusive bytes 0xF0 and 0xF7, and two data
    /// bytes up to 127. Channel messages reach the synthesizer; system
    /// common and real-time messages are accepted and change nothing.
    /// The player must be prefetched or started.
    pub fn midi_short_event(&self, status: i32, data1: i32, data2: i32) -> Result<(), AudioError> {
        if self.format != Format::Midi {
            return Err(AudioError::InvalidState("not a MIDI player".into()));
        }
        if !(0x80..=0xFF).contains(&status)
            || status == 0xF0
            || status == 0xF7
            || !(0..=127).contains(&data1)
            || !(0..=127).contains(&data2)
        {
            return Err(AudioError::InvalidArgument(format!(
                "MIDI message {status:#X} {data1} {data2}"
            )));
        }
        self.with(|mixer, _| {
            let voice = self.voice(mixer);
            if voice.state == PlayerState::Realized {
                return Err(AudioError::InvalidState(
                    "MIDI messages need a prefetched player".into(),
                ));
            }
            if status < 0xF0 {
                voice
                    .source
                    .midi_message(status as u8, data1 as u8, data2 as u8);
            }
            Ok(())
        })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.close();
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Player")
            .field("id", &self.id)
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}
