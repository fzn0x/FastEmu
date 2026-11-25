//! Player events, and the queue that delivers them in order.
//!
//! Events come from API calls (start, stop, close, volume) and from the
//! audio thread (end of media, loops). They are queued while the mixer
//! is locked, so their order is the order things happened, and delivered
//! after it is unlocked, so a listener may call back into the engine.
//!
//! The queue is bounded. A listener that blocks leaves the events behind
//! it queued, and a game can make them faster than any listener reads
//! them, with a loop of a few frames or a volume that flips. Past
//! [`MAX_QUEUED`] events the ones a listener can do without are dropped
//! (see [`EventClass`]), so the queue never grows without bound.

use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

/// How many events wait for delivery before the engine starts dropping
/// the repeats among them. Run ends are kept up to twice as many.
pub const MAX_QUEUED: usize = 4096;

/// Identifies a player for the life of the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlayerId(pub u64);

impl fmt::Display for PlayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "player {}", self.0)
    }
}

/// Something that happened to a player, for MMAPI's `PlayerListener`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerEvent {
    pub player: PlayerId,
    pub kind: PlayerEventKind,
}

/// The events of `javax.microedition.media.PlayerListener`, with the
/// data each carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerEventKind {
    /// `STARTED`: playback began, or began again for another loop.
    Started { media_time_us: i64 },
    /// `STOPPED`: `stop` was called.
    Stopped { media_time_us: i64 },
    /// `END_OF_MEDIA`: the end was reached, after every loop or before
    /// the next one. `last` is true when the player stopped there and
    /// false when another pass of a loop follows, so a listener need not
    /// guess from the player's state, which may have moved on by the time
    /// the event arrives.
    EndOfMedia { media_time_us: i64, last: bool },
    /// `VOLUME_CHANGED`: the level or mute changed.
    VolumeChanged { level: i32, muted: bool },
    /// `CLOSED`: the player, or its whole group, was closed.
    Closed,
    /// `ERROR`: playback hit a problem, such as damaged data partway.
    Error { message: String },
}

impl PlayerEventKind {
    /// How much a listener that has fallen behind can do without the
    /// event. See [`EventClass`].
    pub fn class(&self) -> EventClass {
        match self {
            Self::Started { .. } | Self::VolumeChanged { .. } => EventClass::Repeat,
            Self::EndOfMedia { last: false, .. } => EventClass::Repeat,
            Self::Stopped { .. } | Self::EndOfMedia { last: true, .. } | Self::Error { .. } => {
                EventClass::RunEnd
            }
            Self::Closed => EventClass::Close,
        }
    }
}

/// What losing an event costs a listener, which decides what a queue
/// that is full drops first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClass {
    /// `Started`, `VolumeChanged` and the `EndOfMedia` of a pass that is
    /// followed by another: events a game can make by the million, that
    /// say nothing a later event does not say again.
    Repeat,
    /// `Stopped`, the last `EndOfMedia` and `Error`: they end a run, and
    /// a listener lets go of the player on them. Dropped only when the
    /// listener is far behind, and then the player stays held until its
    /// next run ends or it closes.
    RunEnd,
    /// `Closed`: never dropped, so a player's data can always be freed.
    /// There is one for each player a game made.
    Close,
}

/// Receives the events of one group's players. It runs on whichever
/// thread caused the event (often the audio thread), so it must return
/// quickly and never block; sending into a channel is the usual body.
pub type EventCallback = Arc<dyn Fn(PlayerEvent) + Send + Sync>;

/// Delivers queued events in order, one thread at a time.
#[derive(Default)]
pub(crate) struct EventQueue {
    queue: Mutex<VecDeque<(EventCallback, PlayerEvent)>>,
    /// Held by the thread delivering events, so two threads never
    /// deliver at once and reorder them.
    delivering: Mutex<()>,
    /// Events dropped for want of room, since the engine started.
    dropped: AtomicU64,
}

/// Whether a queue holding `queued` events has room for one of `class`:
/// the repeats stop at [`MAX_QUEUED`], the run ends at twice that, and
/// closes always fit.
fn has_room(queued: usize, class: EventClass) -> bool {
    match class {
        EventClass::Repeat => queued < MAX_QUEUED,
        EventClass::RunEnd => queued < 2 * MAX_QUEUED,
        EventClass::Close => true,
    }
}

impl EventQueue {
    /// Queues an event, or drops it when the queue is too full for its
    /// class. It never blocks, so the audio thread can call it.
    pub fn push(&self, callback: EventCallback, event: PlayerEvent) {
        let mut queue = self.queue.lock();
        if has_room(queue.len(), event.kind.class()) {
            queue.push_back((callback, event));
        } else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn pop(&self) -> Option<(EventCallback, PlayerEvent)> {
        self.queue.lock().pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }

    /// How many events were dropped because the queue was full.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Delivers every queued event. If another thread is delivering, it
    /// takes these too and this returns at once.
    pub fn deliver(&self) {
        loop {
            {
                let Some(_guard) = self.delivering.try_lock() else {
                    return;
                };
                while let Some((callback, event)) = self.pop() {
                    callback(event);
                }
            }
            // An event pushed while the guard was being released would
            // otherwise wait for the next delivery.
            if self.is_empty() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn event(n: u64) -> PlayerEvent {
        PlayerEvent {
            player: PlayerId(n),
            kind: PlayerEventKind::Closed,
        }
    }

    #[test]
    fn delivers_in_order() {
        let queue = EventQueue::default();
        let (sender, receiver) = mpsc::channel();
        let callback: EventCallback = Arc::new(move |e| sender.send(e).unwrap());
        for n in 0..5 {
            queue.push(callback.clone(), event(n));
        }
        queue.deliver();
        let got: Vec<u64> = receiver.try_iter().map(|e| e.player.0).collect();
        assert_eq!(got, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_listener_may_queue_more_while_delivering() {
        let queue = Arc::new(EventQueue::default());
        let (sender, receiver) = mpsc::channel();
        let inner = Arc::clone(&queue);
        let sender2 = sender.clone();
        let echo: EventCallback = Arc::new(move |e| sender2.send(e).unwrap());
        let callback: EventCallback = Arc::new(move |e: PlayerEvent| {
            if e.player.0 == 0 {
                inner.push(echo.clone(), event(9));
                // Delivery is busy on this thread: this returns at once
                // and the outer loop delivers event 9.
                inner.deliver();
            }
            sender.send(e).unwrap();
        });
        queue.push(callback, event(0));
        queue.deliver();
        let got: Vec<u64> = receiver.try_iter().map(|e| e.player.0).collect();
        assert_eq!(got, vec![0, 9]);
    }

    fn event_of(kind: PlayerEventKind) -> PlayerEvent {
        PlayerEvent {
            player: PlayerId(1),
            kind,
        }
    }

    fn repeat() -> PlayerEvent {
        event_of(PlayerEventKind::Started { media_time_us: 0 })
    }

    fn run_end() -> PlayerEvent {
        event_of(PlayerEventKind::Stopped { media_time_us: 0 })
    }

    #[test]
    fn events_have_the_class_their_loss_costs() {
        use PlayerEventKind::*;
        let pass = |last| EndOfMedia {
            media_time_us: 5,
            last,
        };
        for repeat in [
            Started { media_time_us: 0 },
            VolumeChanged {
                level: 3,
                muted: false,
            },
            pass(false),
        ] {
            assert_eq!(repeat.class(), EventClass::Repeat, "{repeat:?}");
        }
        for end in [
            Stopped { media_time_us: 0 },
            pass(true),
            Error {
                message: String::new(),
            },
        ] {
            assert_eq!(end.class(), EventClass::RunEnd, "{end:?}");
        }
        assert_eq!(Closed.class(), EventClass::Close);
    }

    #[test]
    fn a_full_queue_drops_repeats_first_and_never_a_close() {
        let queue = EventQueue::default();
        let (sender, receiver) = mpsc::channel();
        let callback: EventCallback = Arc::new(move |e| sender.send(e).unwrap());
        for _ in 0..MAX_QUEUED + 10 {
            queue.push(callback.clone(), repeat());
        }
        assert_eq!(queue.queue.lock().len(), MAX_QUEUED);
        assert_eq!(queue.dropped(), 10);
        // The ends of runs still fit, up to twice as many events.
        for _ in 0..MAX_QUEUED + 10 {
            queue.push(callback.clone(), run_end());
        }
        assert_eq!(queue.queue.lock().len(), 2 * MAX_QUEUED);
        assert_eq!(queue.dropped(), 20);
        // A close always fits.
        queue.push(callback.clone(), event(7));
        assert_eq!(queue.queue.lock().len(), 2 * MAX_QUEUED + 1);
        assert_eq!(queue.dropped(), 20);
        queue.deliver();
        let got: Vec<PlayerEvent> = receiver.try_iter().collect();
        assert_eq!(got.len(), 2 * MAX_QUEUED + 1);
        assert_eq!(got.last().unwrap().player, PlayerId(7));
        // Delivery made room again.
        queue.push(callback, repeat());
        assert_eq!(queue.queue.lock().len(), 1);
    }

    #[test]
    fn a_panicking_listener_does_not_stop_delivery() {
        let queue = Arc::new(EventQueue::default());
        let panicky: EventCallback = Arc::new(|_| panic!("listener failed"));
        queue.push(panicky, event(0));
        let q = Arc::clone(&queue);
        assert!(std::thread::spawn(move || q.deliver()).join().is_err());
        let (sender, receiver) = mpsc::channel();
        queue.push(Arc::new(move |e| sender.send(e).unwrap()), event(1));
        queue.deliver();
        assert_eq!(receiver.try_iter().count(), 1);
    }
}
