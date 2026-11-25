//! What happened to a player or a Nokia sound, on its way to the game's
//! listener, and the channel that carries it to the media event thread.
//!
//! The channel is bounded by a count, not by its size: a sender never
//! waits, because the audio thread sends. A game that makes events faster
//! than its listener takes them (a loop of a few frames, a volume that
//! flips, a start and a stop in a loop) would otherwise fill the host's
//! memory while its Java heap stayed empty. Past [`MAX_PENDING`] notes
//! waiting, the events a listener can do without are dropped, and past
//! twice that the ends of runs too; only a close always passes.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use fastemu_audio::{EventClass, PlayerEvent, PlayerEventKind};

/// How many events wait for the event thread before the repeats among
/// them are dropped: a game gets 1024, and the engine's own queue ahead
/// of it holds 4096 for all games.
pub(crate) const MAX_PENDING: usize = 1024;

/// A player event, with the media time in microseconds where it has one.
///
/// It is the engine's [`PlayerEventKind`] plus the one event the MIDP
/// layer makes itself: `STOPPED_AT_TIME`, for `StopTimeControl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Started(i64),
    Stopped(i64),
    StoppedAtTime(i64),
    /// The end of the media; `last` is false when another pass of a loop
    /// follows.
    EndOfMedia {
        time_us: i64,
        last: bool,
    },
    VolumeChanged,
    Closed,
    Error(String),
}

impl Event {
    /// Whether the event ends a run of the player (and so its hold).
    pub fn ends_a_run(&self) -> bool {
        matches!(
            self,
            Self::Stopped(_)
                | Self::StoppedAtTime(_)
                | Self::EndOfMedia { last: true, .. }
                | Self::Closed
        )
    }
}

impl Event {
    /// What a listener that has fallen behind loses by not hearing of the
    /// event, as [`PlayerEventKind::class`] says for the engine's own.
    pub fn class(&self) -> EventClass {
        match self {
            Self::Started(_) | Self::VolumeChanged | Self::EndOfMedia { last: false, .. } => {
                EventClass::Repeat
            }
            Self::Stopped(_)
            | Self::StoppedAtTime(_)
            | Self::EndOfMedia { last: true, .. }
            | Self::Error(_) => EventClass::RunEnd,
            Self::Closed => EventClass::Close,
        }
    }
}

impl From<PlayerEventKind> for Event {
    fn from(kind: PlayerEventKind) -> Self {
        match kind {
            PlayerEventKind::Started { media_time_us } => Self::Started(media_time_us),
            PlayerEventKind::Stopped { media_time_us } => Self::Stopped(media_time_us),
            PlayerEventKind::EndOfMedia {
                media_time_us,
                last,
            } => Self::EndOfMedia {
                time_us: media_time_us,
                last,
            },
            PlayerEventKind::VolumeChanged { .. } => Self::VolumeChanged,
            PlayerEventKind::Closed => Self::Closed,
            PlayerEventKind::Error { message } => Self::Error(message),
        }
    }
}

/// A message to the media event thread.
#[derive(Debug)]
pub(crate) enum Note {
    /// An event of an engine player, from the group's listener on any
    /// thread (often the audio thread).
    Engine(PlayerEvent),
    /// An event the MIDP layer made for the player or sound `serial`: from
    /// a silent player, a closed unrealized player, or a stop time.
    Midp(u64, Event),
    /// The Java object of `serial` was collected.
    Gone(u64),
    /// A timer changed; the event thread works out when to wake again.
    Wake,
}

/// What the two ends of the channel share.
#[derive(Default)]
struct Shared {
    /// Notes sent and not yet taken up by the event thread.
    pending: AtomicUsize,
    /// Whether a [`Note::Wake`] waits in the channel: one is enough.
    wake_queued: AtomicBool,
    /// Events dropped because the thread was too far behind.
    dropped: AtomicU64,
}

/// The sending end of the channel to the media event thread. Cloning it is
/// cheap; every clone feeds the same thread.
#[derive(Clone)]
pub(crate) struct Notes {
    sender: Sender<Note>,
    shared: Arc<Shared>,
}

/// The receiving end, on the event thread.
pub(crate) struct NoteReceiver {
    receiver: Receiver<Note>,
    shared: Arc<Shared>,
}

/// Makes the channel to a media event thread.
pub(crate) fn channel() -> (Notes, NoteReceiver) {
    let (sender, receiver) = mpsc::channel();
    let shared = Arc::new(Shared::default());
    (
        Notes {
            sender,
            shared: Arc::clone(&shared),
        },
        NoteReceiver { receiver, shared },
    )
}

/// Whether `pending` waiting notes leave room for one of `class`.
fn has_room(pending: usize, class: EventClass) -> bool {
    match class {
        EventClass::Repeat => pending < MAX_PENDING,
        EventClass::RunEnd => pending < 2 * MAX_PENDING,
        EventClass::Close => true,
    }
}

impl Notes {
    /// Sends `note` unless the thread is too far behind for its class.
    /// Never blocks.
    fn send(&self, note: Note, class: EventClass) {
        let pending = self.shared.pending.fetch_add(1, Ordering::AcqRel);
        if !has_room(pending, class) {
            self.shared.pending.fetch_sub(1, Ordering::AcqRel);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        // The thread is gone only when the VM stops.
        let _ = self.sender.send(note);
    }

    /// An event of an engine player, from the group's listener on any
    /// thread, often the audio thread.
    pub fn engine(&self, event: PlayerEvent) {
        let class = event.kind.class();
        self.send(Note::Engine(event), class);
    }

    /// An event the MIDP layer made for the player or sound `serial`.
    pub fn midp(&self, serial: u64, event: Event) {
        let class = event.class();
        self.send(Note::Midp(serial, event), class);
    }

    /// The Java object of `serial` was collected. Always sent: there is
    /// one for each object the game made.
    pub fn gone(&self, serial: u64) {
        self.send(Note::Gone(serial), EventClass::Close);
    }

    /// Wakes the thread to work out its timers again. A wake that is
    /// waiting already does the same, so a game that starts and stops a
    /// player in a loop leaves one.
    pub fn wake(&self) {
        if !self.shared.wake_queued.swap(true, Ordering::AcqRel) {
            let _ = self.sender.send(Note::Wake);
        }
    }

    /// How many notes wait for the thread now.
    #[cfg(test)]
    pub fn pending(&self) -> usize {
        self.shared.pending.load(Ordering::Acquire)
    }

    /// How many events were dropped so far.
    #[cfg(test)]
    pub fn dropped(&self) -> u64 {
        self.shared.dropped.load(Ordering::Relaxed)
    }
}

impl NoteReceiver {
    /// Waits up to `wait` for a note.
    pub fn recv_timeout(&self, wait: Duration) -> Result<Note, RecvTimeoutError> {
        self.receiver.recv_timeout(wait)
    }

    /// Takes the notes that wait now.
    pub fn take_waiting(&self) -> Vec<Note> {
        self.receiver.try_iter().collect()
    }

    /// Says the thread is about to handle `note`, which stops counting as
    /// waiting. Call it for every note taken, before handling it, so the
    /// count covers a batch the thread is working through.
    pub fn settle(&self, note: &Note) {
        match note {
            Note::Wake => self.shared.wake_queued.store(false, Ordering::Release),
            _ => {
                self.shared.pending.fetch_sub(1, Ordering::AcqRel);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use fastemu_audio::PlayerId;

    use super::*;

    fn engine_event(kind: PlayerEventKind) -> PlayerEvent {
        PlayerEvent {
            player: PlayerId(1),
            kind,
        }
    }

    fn waiting(receiver: &NoteReceiver) -> usize {
        receiver.take_waiting().len()
    }

    #[test]
    fn events_have_the_class_the_engines_have() {
        let pairs = [
            (
                Event::Started(0),
                PlayerEventKind::Started { media_time_us: 0 },
            ),
            (
                Event::Stopped(0),
                PlayerEventKind::Stopped { media_time_us: 0 },
            ),
            (
                Event::EndOfMedia {
                    time_us: 0,
                    last: false,
                },
                PlayerEventKind::EndOfMedia {
                    media_time_us: 0,
                    last: false,
                },
            ),
            (
                Event::EndOfMedia {
                    time_us: 0,
                    last: true,
                },
                PlayerEventKind::EndOfMedia {
                    media_time_us: 0,
                    last: true,
                },
            ),
            (
                Event::VolumeChanged,
                PlayerEventKind::VolumeChanged {
                    level: 1,
                    muted: false,
                },
            ),
            (Event::Closed, PlayerEventKind::Closed),
            (
                Event::Error(String::new()),
                PlayerEventKind::Error {
                    message: String::new(),
                },
            ),
        ];
        for (event, kind) in pairs {
            assert_eq!(event.class(), kind.class(), "{event:?}");
        }
        assert_eq!(Event::StoppedAtTime(0).class(), EventClass::RunEnd);
    }

    #[test]
    fn a_hundred_thousand_loop_events_behind_a_blocked_listener_stay_bounded() {
        let (notes, receiver) = channel();
        // Nobody takes notes: the Java listener is blocked.
        for i in 0..100_000u32 {
            let kind = if i % 2 == 0 {
                PlayerEventKind::EndOfMedia {
                    media_time_us: 1,
                    last: false,
                }
            } else {
                PlayerEventKind::Started { media_time_us: 0 }
            };
            notes.engine(engine_event(kind));
            assert!(notes.pending() <= MAX_PENDING);
        }
        assert_eq!(notes.pending(), MAX_PENDING);
        assert_eq!(notes.dropped(), 100_000 - MAX_PENDING as u64);
        assert_eq!(waiting(&receiver), MAX_PENDING);
    }

    #[test]
    fn the_ends_of_runs_go_further_and_a_close_always_passes() {
        let (notes, receiver) = channel();
        for _ in 0..3 * MAX_PENDING {
            notes.engine(engine_event(PlayerEventKind::Stopped { media_time_us: 0 }));
        }
        assert_eq!(notes.pending(), 2 * MAX_PENDING);
        notes.engine(engine_event(PlayerEventKind::Closed));
        notes.midp(7, Event::Closed);
        notes.gone(7);
        assert_eq!(notes.pending(), 2 * MAX_PENDING + 3);
        // The thread works through them and makes room.
        let batch = receiver.take_waiting();
        assert_eq!(batch.len(), 2 * MAX_PENDING + 3);
        for note in &batch {
            receiver.settle(note);
        }
        assert_eq!(notes.pending(), 0);
        notes.midp(7, Event::Started(0));
        assert_eq!(notes.pending(), 1);
    }

    #[test]
    fn a_batch_being_handled_still_counts_until_each_note_is_taken_up() {
        let (notes, receiver) = channel();
        for _ in 0..10 {
            notes.midp(1, Event::VolumeChanged);
        }
        let batch = receiver.take_waiting();
        assert_eq!(notes.pending(), 10);
        receiver.settle(&batch[0]);
        assert_eq!(notes.pending(), 9);
    }

    #[test]
    fn one_wake_is_enough() {
        let (notes, receiver) = channel();
        for _ in 0..50_000 {
            notes.wake();
        }
        assert_eq!(notes.pending(), 0);
        let batch = receiver.take_waiting();
        assert_eq!(batch.len(), 1);
        assert!(matches!(batch[0], Note::Wake));
        // Once the thread has taken it up, the next wake is sent.
        receiver.settle(&batch[0]);
        notes.wake();
        assert_eq!(receiver.take_waiting().len(), 1);
    }
}
