//! Groups: the players of one instance, with one volume and mute, closed
//! together when the instance ends.

use std::fmt;
use std::sync::{Arc, mpsc};

use crate::engine::Engine;
use crate::error::AudioError;
use crate::event::{EventCallback, PlayerEvent, PlayerEventKind, PlayerId};
use crate::format::Format;
use crate::media;
use crate::mixer::{GroupId, GroupState, Slot};
use crate::nokia;
use crate::player::Player;
use crate::tone::ToneEvent;
use crate::voice::{PlayerState, Voice};

/// A group keeps at most this many `playTone` tones at once; a new one
/// replaces the oldest. Games call `playTone` in tight loops.
const MAX_TONES: usize = 16;

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
    mixer.groups.remove(&id);
    drop(mixer);
    engine.deliver();
}

impl Group {
    pub(crate) fn new(engine: Engine, id: GroupId) -> Self {
        Self {
            inner: Arc::new(Inner { engine, id }),
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
    /// player's own level. It slides over a few milliseconds.
    pub fn set_volume(&self, volume: f32) {
        self.with(|group| group.volume = volume.clamp(0.0, 1.0));
    }

    pub fn volume(&self) -> f32 {
        self.with(|group| group.volume).unwrap_or(0.0)
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
            } else if let Some(slot) = mixer.slots.get_mut(&player) {
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
        self.with(|group| group.listener = listener);
    }

    /// Sends the group's events into a channel and returns its receiving
    /// end, replacing any listener.
    pub fn subscribe(&self) -> mpsc::Receiver<PlayerEvent> {
        let (sender, receiver) = mpsc::channel();
        let sender = std::sync::Mutex::new(sender);
        self.set_listener(Some(Arc::new(move |event| {
            // The receiver may be gone; then nobody is listening.
            let _ = sender
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .send(event);
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

    /// Adds a voice to the group and returns its id.
    fn insert(&self, voice: Voice) -> Result<PlayerId, AudioError> {
        let engine = &self.inner.engine;
        let mut mixer = engine.lock();
        if !mixer.groups.contains_key(&self.inner.id) {
            return Err(AudioError::Closed);
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
