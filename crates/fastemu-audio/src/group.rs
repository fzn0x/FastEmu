//! Groups: the players of one instance, with one volume and mute, closed
//! together when the instance ends.

use std::fmt;
use std::sync::{Arc, mpsc};

use crate::engine::Engine;
use crate::error::AudioError;
use crate::event::{EventCallback, PlayerEvent, PlayerEventKind, PlayerId};
use crate::format::Format;
use crate::gain::SharedGain;
use crate::media;
use crate::mixer::{GroupId, GroupState, Mixer, Slot};
use crate::nokia;
use crate::pcm::MAX_DECODED_BYTES;
use crate::player::Player;
use crate::tone::ToneEvent;
use crate::voice::{PlayerState, Voice};

/// A group keeps at most this many `playTone` tones at once; a new one
/// replaces the oldest. Games call `playTone` in tight loops.
const MAX_TONES: usize = 16;

/// What one game's sound may take, set per group with
/// [`Group::set_limits`]. Games on a phone made a few dozen players and
/// kept a few megabytes of sound.
///
/// Both limits count what the group holds now, so closing a player makes
/// room for another. They bound the host's memory for a game whose Java
/// heap never sees its sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupLimits {
    /// Bytes of sound the group's players hold together: the decoded
    /// samples of a WAV or AMR file, the file an MP3 is decoded from as
    /// it plays, the events and time signatures of a MIDI file, the tones
    /// of a tone sequence.
    pub max_bytes: usize,
    /// Players the group holds, playing or not. `playTone` tones are
    /// capped on their own.
    pub max_players: usize,
}

impl GroupLimits {
    /// 32 MiB of sound and 128 players: several times what a phone held,
    /// so a game written for one never reaches it.
    pub const DEFAULT: Self = Self {
        max_bytes: 32 * 1024 * 1024,
        max_players: 128,
    };
}

impl Default for GroupLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The bytes of sound `group` holds now, and its players, `playTone`
/// tones left out.
fn held(mixer: &Mixer, group: GroupId) -> (usize, usize) {
    mixer
        .slots
        .values()
        .filter(|slot| slot.group == group)
        .fold((0, 0), |(bytes, players), slot| {
            (
                bytes + slot.voice.source.memory_bytes(),
                players + usize::from(!slot.voice.one_shot),
            )
        })
}

/// Checks that `group` has room for `adding` more bytes of sound, with
/// `freeing` of what it holds given back (a source being replaced), and
/// for one more player when `new_player`.
///
/// A sound bigger than the whole budget, or than a decoder may make
/// ([`MAX_DECODED_BYTES`]), is [`AudioError::TooLarge`]: it can never fit.
/// Anything else the group has no room for is [`AudioError::OutOfMemory`],
/// which a closed or collected player can cure.
pub(crate) fn admit(
    mixer: &Mixer,
    group: GroupId,
    adding: usize,
    freeing: usize,
    new_player: bool,
) -> Result<(), AudioError> {
    let limits = mixer.groups.get(&group).ok_or(AudioError::Closed)?.limits;
    // The decoders refuse what decodes past the cap, but a MIDI file or a
    // tone list is not decoded to samples: its events are counted here, so
    // that no one sound passes the cap, whatever its format.
    let most = limits.max_bytes.min(MAX_DECODED_BYTES);
    if adding > most {
        return Err(AudioError::TooLarge(format!(
            "a sound of {adding} bytes, when one sound may hold {most} bytes"
        )));
    }
    let (bytes, players) = held(mixer, group);
    if new_player && players >= limits.max_players {
        return Err(AudioError::OutOfMemory(format!(
            "a game may hold {} players",
            limits.max_players
        )));
    }
    if bytes.saturating_sub(freeing).saturating_add(adding) > limits.max_bytes {
        return Err(AudioError::OutOfMemory(format!(
            "a game holds {bytes} bytes of sound already, and may hold {}",
            limits.max_bytes
        )));
    }
    Ok(())
}

/// The players of one instance. Cloning it is cheap; when the last clone
/// is dropped, or [`Group::close`] is called, every player in it stops
/// and closes, so a game that ends leaves no sound behind.
#[derive(Clone)]
pub struct Group {
    inner: Arc<Inner>,
}

struct Inner {
    engine: Engine,
    id: GroupId,
    /// The volume the mix reads, set here without the mixer lock.
    volume: Arc<SharedGain>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        close_group(&self.engine, self.id);
    }
}

fn close_group(engine: &Engine, id: GroupId) {
    let mut mixer = engine.lock();
    if !mixer.groups.contains_key(&id) {
        return;
    }
    let players: Vec<PlayerId> = mixer
        .slots
        .iter()
        .filter(|(_, slot)| slot.group == id)
        .map(|(&player, _)| player)
        .collect();
    for player in players {
        mixer.fade_out(player);
        mixer.emit(&engine.shared.events, player, PlayerEventKind::Closed);
        mixer.slots.remove(&player);
    }
    let state = mixer.groups.remove(&id);
    drop(mixer);
    // The listener may own player or group handles, whose drops take the
    // mixer lock: drop it only after the lock is released.
    drop(state);
    engine.deliver();
}

impl Group {
    pub(crate) fn new(engine: Engine, id: GroupId, volume: Arc<SharedGain>) -> Self {
        Self {
            inner: Arc::new(Inner { engine, id, volume }),
        }
    }

    pub fn id(&self) -> GroupId {
        self.inner.id
    }

    pub fn engine(&self) -> &Engine {
        &self.inner.engine
    }

    /// Runs `f` on the group's state; `None` once the group is closed.
    fn with<R>(&self, f: impl FnOnce(&mut GroupState) -> R) -> Option<R> {
        self.inner
            .engine
            .lock()
            .groups
            .get_mut(&self.inner.id)
            .map(f)
    }

    /// Sets the group's gain, from 0.0 (silent) to 1.0, on top of each
    /// player's own level; a NaN is silence. It never waits for the mixer,
    /// which the audio thread holds while it renders a slice and a game's
    /// threads hold in player calls: the app calls it from its UI thread.
    /// The mix takes it up at its next slice and slides to it over a few
    /// milliseconds.
    pub fn set_volume(&self, volume: f32) {
        self.inner.volume.set(volume);
    }

    /// The last volume set, whether the group is open or closed.
    pub fn volume(&self) -> f32 {
        self.inner.volume.get()
    }

    /// Silences the group without stopping it: players keep their place
    /// in time and still reach their ends.
    pub fn set_muted(&self, muted: bool) {
        self.with(|group| group.muted = muted);
    }

    pub fn is_muted(&self) -> bool {
        self.with(|group| group.muted).unwrap_or(true)
    }

    /// Freezes every player in the group where it is, as when the app
    /// pauses the instance; time stops for them until unpaused. Their
    /// states do not change and no events are sent.
    pub fn set_paused(&self, paused: bool) {
        let mut mixer = self.inner.engine.lock();
        let Some(group) = mixer.groups.get_mut(&self.inner.id) else {
            return;
        };
        if group.paused == paused {
            return;
        }
        group.paused = paused;
        for player in self.player_ids(&mixer) {
            if paused {
                mixer.fade_out(player);
            }
            // Silent while paused, so a stop, seek or close meanwhile
            // has nothing to fade out; ramps up again on resume.
            if let Some(slot) = mixer.slots.get_mut(&player) {
                slot.voice.gain = 0.0;
            }
        }
    }

    pub fn is_paused(&self) -> bool {
        self.with(|group| group.paused).unwrap_or(false)
    }

    /// Sets the function that receives every event of the group's
    /// players, or `None` for none. See [`EventCallback`] for the rules
    /// it must follow.
    pub fn set_listener(&self, listener: Option<EventCallback>) {
        let old = self.with(|group| std::mem::replace(&mut group.listener, listener));
        // Dropped here, with the mixer unlocked: it may own handles whose
        // drops take the lock.
        drop(old);
    }

    /// Sends the group's events into a channel and returns its receiving
    /// end, replacing any listener.
    pub fn subscribe(&self) -> mpsc::Receiver<PlayerEvent> {
        let (sender, receiver) = mpsc::channel();
        self.set_listener(Some(Arc::new(move |event| {
            // The receiver may be gone; then nobody is listening.
            let _ = sender.send(event);
        })));
        receiver
    }

    fn player_ids(&self, mixer: &crate::mixer::Mixer) -> Vec<PlayerId> {
        mixer
            .slots
            .iter()
            .filter(|(_, slot)| slot.group == self.inner.id)
            .map(|(&id, _)| id)
            .collect()
    }

    /// What the group may hold; the default until [`Group::set_limits`].
    pub fn limits(&self) -> GroupLimits {
        self.with(|group| group.limits).unwrap_or_default()
    }

    /// Changes what the group may hold, for the players made from now on:
    /// players it holds already stay, even past a lowered limit. The
    /// limits of one game are the host's to set; a game cannot.
    pub fn set_limits(&self, limits: GroupLimits) {
        self.with(|group| group.limits = limits);
    }

    /// The bytes of sound the group's players hold now, counted as
    /// [`GroupLimits::max_bytes`] counts them.
    pub fn memory_bytes(&self) -> usize {
        held(&self.inner.engine.lock(), self.inner.id).0
    }

    /// Fails now if the group is closed or has no room for another
    /// player, so a sound it could not take is not decoded first.
    fn check_room(&self) -> Result<(), AudioError> {
        admit(&self.inner.engine.lock(), self.inner.id, 0, 0, true)
    }

    /// Adds a voice to the group and returns its id. Every player any
    /// call makes goes through here, so the limits hold for them all:
    /// MMAPI players, tone players, and the Nokia, Samsung and Siemens
    /// sounds.
    fn insert(&self, mut voice: Voice) -> Result<PlayerId, AudioError> {
        let engine = &self.inner.engine;
        let mut mixer = engine.lock();
        if voice.one_shot {
            // A `playTone` tone: a few bytes, and at most MAX_TONES.
            if !mixer.groups.contains_key(&self.inner.id) {
                return Err(AudioError::Closed);
            }
        } else {
            admit(&mixer, self.inner.id, voice.source.memory_bytes(), 0, true)?;
        }
        if voice.state == PlayerState::Started {
            // A tone starts as it is made: it takes its place in the order
            // the game's voices started.
            voice.start_order = mixer.next_start_order();
        }
        let id = PlayerId(engine.next_id());
        mixer.slots.insert(
            id,
            Slot {
                group: self.inner.id,
                voice,
            },
        );
        Ok(id)
    }

    /// Creates a player from sound data, as `Manager.createPlayer` with
    /// an input stream and a content type. The format is taken from the
    /// data's signature when it has one, since games often pass the
    /// wrong type, and from `content_type` otherwise. The player starts
    /// out realized.
    pub fn create_player(
        &self,
        data: &[u8],
        content_type: Option<&str>,
    ) -> Result<Player, AudioError> {
        let format = Format::detect(data, content_type)?;
        self.create_player_as(data, format)
    }

    /// Creates a player for data in a known format, such as a Nokia
    /// ringing tone from `com.nokia.mid.sound.Sound`.
    pub fn create_player_as(&self, data: &[u8], format: Format) -> Result<Player, AudioError> {
        self.check_room()?;
        let engine = &self.inner.engine;
        let soundfont = engine.soundfont();
        let source = media::open(data, format, engine.sample_rate(), soundfont.as_ref())?;
        let id = self.insert(Voice::new(source, false))?;
        Ok(Player::new(engine.clone(), id, format))
    }

    /// Creates a tone sequence player with an empty sequence, for
    /// `Manager.TONE_DEVICE_LOCATOR`; give it notes with
    /// [`Player::set_tone_sequence`].
    pub fn create_tone_player(&self) -> Result<Player, AudioError> {
        let rate = self.inner.engine.sample_rate();
        let voice = Voice::new(media::tones(Vec::new(), rate), false);
        let id = self.insert(voice)?;
        Ok(Player::new(
            self.inner.engine.clone(),
            id,
            Format::ToneSequence,
        ))
    }

    /// Creates a player for a single beep, as Nokia's `Sound(frequency,
    /// duration)`: `frequency_hz` up to 20 kHz (0 is a rest) for
    /// `duration_ms` milliseconds.
    pub fn create_beep_player(
        &self,
        frequency_hz: i32,
        duration_ms: i64,
    ) -> Result<Player, AudioError> {
        if !(0..=20_000).contains(&frequency_hz) {
            return Err(AudioError::InvalidArgument(format!(
                "frequency {frequency_hz} Hz"
            )));
        }
        if duration_ms <= 0 {
            return Err(AudioError::InvalidArgument(format!(
                "duration {duration_ms} ms"
            )));
        }
        let rate = self.inner.engine.sample_rate();
        let beep = nokia::beep(frequency_hz as u32, duration_ms as u64);
        let voice = Voice::new(media::tones(vec![beep], rate), false);
        let id = self.insert(voice)?;
        Ok(Player::new(
            self.inner.engine.clone(),
            id,
            Format::ToneSequence,
        ))
    }

    /// Plays a tone and forgets it, as `Manager.playTone`: `note` is a
    /// MIDI note number (0 to 127, 69 is A 440 Hz), `duration_ms` must be
    /// positive, and `volume` (0 to 100) is clamped.
    pub fn play_tone(&self, note: i32, duration_ms: i32, volume: i32) -> Result<(), AudioError> {
        if !(0..=127).contains(&note) {
            return Err(AudioError::InvalidArgument(format!("note {note}")));
        }
        if duration_ms <= 0 {
            return Err(AudioError::InvalidArgument(format!(
                "duration {duration_ms} ms"
            )));
        }
        let engine = &self.inner.engine;
        let tone = ToneEvent::note(
            note as u8,
            duration_ms as u64 * 1000,
            volume.clamp(0, 100) as u8,
        );
        let mut voice = Voice::new(media::tones(vec![tone], engine.sample_rate()), true);
        voice.start();
        let id = self.insert(voice)?;
        let mut mixer = engine.lock();
        let mut tones: Vec<PlayerId> = mixer
            .slots
            .iter()
            .filter(|(_, slot)| slot.group == self.inner.id && slot.voice.one_shot)
            .map(|(&tone, _)| tone)
            .collect();
        if tones.len() > MAX_TONES {
            tones.sort_unstable();
            for oldest in &tones[..tones.len() - MAX_TONES] {
                if *oldest != id {
                    mixer.fade_out(*oldest);
                    mixer.slots.remove(oldest);
                }
            }
        }
        Ok(())
    }

    /// Stops every player in the group, as if `stop` were called on each,
    /// and silences its tones.
    pub fn stop_all(&self) {
        let engine = &self.inner.engine;
        let mut mixer = engine.lock();
        for player in self.player_ids(&mixer) {
            let Some(slot) = mixer.slots.get(&player) else {
                continue;
            };
            if slot.voice.state != PlayerState::Started {
                continue;
            }
            let one_shot = slot.voice.one_shot;
            mixer.fade_out(player);
            if one_shot {
                mixer.slots.remove(&player);
                continue;
            }
            let media_time_us = match mixer.slots.get_mut(&player) {
                Some(slot) => {
                    slot.voice.state = PlayerState::Prefetched;
                    slot.voice.media_time_us()
                }
                None => continue,
            };
            mixer.emit(
                &engine.shared.events,
                player,
                PlayerEventKind::Stopped { media_time_us },
            );
        }
        drop(mixer);
        engine.deliver();
    }

    /// Closes every player in the group and the group itself; creating
    /// players fails from then on. Each open player gets a `Closed`
    /// event.
    pub fn close(&self) {
        close_group(&self.inner.engine, self.inner.id);
    }

    pub fn is_closed(&self) -> bool {
        self.with(|_| ()).is_none()
    }

    /// How many players the group holds, playing or not; `playTone`
    /// tones are not counted.
    pub fn player_count(&self) -> usize {
        let mixer = self.inner.engine.lock();
        mixer
            .slots
            .values()
            .filter(|slot| slot.group == self.inner.id && !slot.voice.one_shot)
            .count()
    }
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Group").field("id", &self.inner.id).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::EngineConfig;
    use crate::pcm::{Pcm, PcmSource};
    use crate::source::Source;

    /// A sound of `bytes` bytes of silence, whose pages are never touched.
    fn silence(bytes: usize) -> Box<dyn Source> {
        Box::new(PcmSource::new(Arc::new(Pcm {
            rate: 8000,
            channels: 1,
            samples: vec![0; bytes / 2],
        })))
    }

    #[test]
    fn no_one_sound_is_bigger_than_a_decoder_may_make() {
        let engine = Engine::new(EngineConfig { sample_rate: 8000 });
        let group = engine.create_group();
        // A budget of 40 MiB is no reason for one sound of 17.
        group.set_limits(GroupLimits {
            max_bytes: 40 * 1024 * 1024,
            max_players: 8,
        });
        group
            .insert(Voice::new(silence(MAX_DECODED_BYTES), false))
            .unwrap();
        let refused = group
            .insert(Voice::new(silence(MAX_DECODED_BYTES + 2), false))
            .unwrap_err();
        assert!(matches!(refused, AudioError::TooLarge(_)), "{refused}");
        assert_eq!(
            refused.to_string(),
            "sound too large: a sound of 16777218 bytes, when one sound may hold 16777216 bytes"
        );
        assert_eq!(group.player_count(), 1);
        // The budget still counts what the group holds in all.
        group
            .insert(Voice::new(silence(MAX_DECODED_BYTES), false))
            .unwrap();
        let full = group
            .insert(Voice::new(silence(MAX_DECODED_BYTES), false))
            .unwrap_err();
        assert!(matches!(full, AudioError::OutOfMemory(_)), "{full}");
    }

    /// The app sets a game's volume from its UI thread, so the call must
    /// not wait for the mixer: the audio thread holds it while it renders
    /// a slice, and a game's threads in every player call. Here the test
    /// holds it for as long as it likes.
    #[test]
    fn set_volume_does_not_wait_for_the_mixer() {
        let engine = Engine::new(EngineConfig { sample_rate: 8000 });
        let group = engine.create_group();
        let held = engine.lock();
        let (done, returned) = mpsc::channel();
        let setter = {
            let group = group.clone();
            std::thread::spawn(move || {
                group.set_volume(0.25);
                let _ = done.send(());
            })
        };
        // The call takes microseconds. The bound only ends the test when
        // the call waits for the lock, which it then never gets while the
        // test holds it; the lock is let go before the join, so the test
        // fails rather than hangs.
        let in_time = returned
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_ok();
        drop(held);
        setter.join().unwrap();
        assert!(in_time, "set_volume waited for the mixer lock");
        assert_eq!(group.volume(), 0.25);
    }
}
