//! Player events, and the queue that delivers them in order.
//!
//! Events come from API calls (start, stop, close, volume) and from the
//! audio thread (end of media, loops). They are queued while the mixer
//! is locked, so their order is the order things happened, and delivered
//! after it is unlocked, so a listener may call back into the engine.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError, TryLockError};

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
    /// the next one.
    EndOfMedia { media_time_us: i64 },
    /// `VOLUME_CHANGED`: the level or mute changed.
    VolumeChanged { level: i32, muted: bool },
    /// `CLOSED`: the player, or its whole group, was closed.
    Closed,
    /// `ERROR`: playback hit a problem, such as damaged data partway.
    Error { message: String },
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
}

impl EventQueue {
    pub fn push(&self, callback: EventCallback, event: PlayerEvent) {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push_back((callback, event));
    }

    fn pop(&self) -> Option<(EventCallback, PlayerEvent)> {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pop_front()
    }

    fn is_empty(&self) -> bool {
        self.queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }

    /// Delivers every queued event. If another thread is delivering, it
    /// takes these too and this returns at once.
    pub fn deliver(&self) {
        loop {
            {
                let _guard = match self.delivering.try_lock() {
                    Ok(guard) => guard,
                    // A listener panicked earlier; delivery goes on.
                    Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
                    Err(TryLockError::WouldBlock) => return,
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
