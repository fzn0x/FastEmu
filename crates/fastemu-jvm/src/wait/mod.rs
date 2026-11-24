//! The virtual machine's condition variable, with deadlines precise to
//! about a millisecond on every platform.
//!
//! Every blocked Java thread (in `sleep`, `wait`, `join`, a contended
//! monitor, `Ctx::wait_until`) and a host thread in `Vm::wait_finished`
//! wait on one [`WaitQueue`] per virtual machine, and every change wakes
//! them all. It works as a condition variable does, but each thread
//! waits on a [`Signal`] of its own, so that on Windows the wait can take
//! a high-resolution timer instead of the 15.6 ms system tick that
//! condition variables time out on there. Elsewhere a signal is a mutex
//! and a condition variable, whose timeouts are already precise.

mod portable;
// The Win32 event and timer calls are foreign functions.
#[cfg(windows)]
#[allow(unsafe_code)]
mod windows;

use std::sync::Arc;
use std::time::Instant;

use parking_lot::{Mutex, MutexGuard};

/// A flag one thread waits on and others raise.
enum Signal {
    #[cfg(windows)]
    Windows(windows::Signal),
    Portable(portable::Signal),
}

impl Signal {
    fn new() -> Self {
        #[cfg(windows)]
        if let Some(signal) = windows::Signal::new() {
            return Signal::Windows(signal);
        }
        Signal::Portable(portable::Signal::default())
    }

    fn reset(&self) {
        match self {
            #[cfg(windows)]
            Signal::Windows(s) => s.reset(),
            Signal::Portable(s) => s.reset(),
        }
    }

    fn raise(&self) {
        match self {
            #[cfg(windows)]
            Signal::Windows(s) => s.raise(),
            Signal::Portable(s) => s.raise(),
        }
    }

    /// Returns whether `deadline` passed before a raise.
    fn wait(&self, deadline: Option<Instant>) -> bool {
        match self {
            #[cfg(windows)]
            Signal::Windows(s) => s.wait(deadline),
            Signal::Portable(s) => s.wait(deadline),
        }
    }
}

thread_local! {
    /// The calling OS thread's signal, made on its first wait.
    static SIGNAL: Arc<Signal> = Arc::new(Signal::new());
}

/// Threads waiting for a change to state behind a mutex, as with a
/// condition variable: [`WaitQueue::wait`] releases the lock and
/// [`WaitQueue::notify_all`] wakes every waiter.
///
/// A waiter joins the queue before it releases the lock, so a change made
/// under the lock and then notified always wakes it. Wakeups are not
/// precise: a woken thread checks its condition again.
#[derive(Default)]
pub(crate) struct WaitQueue {
    /// The signals of the threads waiting now. Taken only briefly, never
    /// while taking the lock a waiter released.
    waiters: Mutex<Vec<Arc<Signal>>>,
}

impl WaitQueue {
    /// Releases the lock `guard` holds until [`WaitQueue::notify_all`]
    /// or `deadline`, then takes it again. Returns whether the deadline
    /// passed. May also return early, as condition variables may.
    pub(crate) fn wait<T>(&self, guard: &mut MutexGuard<'_, T>, deadline: Option<Instant>) -> bool {
        let signal = SIGNAL.with(Arc::clone);
        signal.reset();
        self.waiters.lock().push(Arc::clone(&signal));
        let timed_out = MutexGuard::unlocked(guard, || signal.wait(deadline));
        let mut waiters = self.waiters.lock();
        if let Some(i) = waiters.iter().position(|s| Arc::ptr_eq(s, &signal)) {
            waiters.swap_remove(i);
        }
        timed_out
    }

    /// Wakes every thread in [`WaitQueue::wait`].
    pub(crate) fn notify_all(&self) {
        for signal in self.waiters.lock().iter() {
            signal.raise();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// One signal of each kind this platform has.
    fn signals() -> Vec<Signal> {
        vec![
            Signal::Portable(portable::Signal::default()),
            #[cfg(windows)]
            Signal::Windows(windows::Signal::new().expect("an event")),
        ]
    }

    #[test]
    fn a_raise_before_the_wait_is_kept_until_reset() {
        for signal in signals() {
            signal.raise();
            assert!(!signal.wait(Some(Instant::now() + Duration::from_secs(5))));
            signal.raise();
            signal.reset();
            assert!(signal.wait(Some(Instant::now() + Duration::from_millis(1))));
            assert!(signal.wait(Some(Instant::now())), "a past deadline");
        }
    }

    /// How long each of ten waits for `wait` took, sorted.
    fn timed(signal: &Signal, wait: Duration) -> Vec<Duration> {
        let mut took: Vec<Duration> = (0..10)
            .map(|_| {
                let start = Instant::now();
                assert!(signal.wait(Some(start + wait)), "nothing raised it");
                start.elapsed()
            })
            .collect();
        took.sort();
        took
    }

    #[test]
    fn deadlines_never_pass_much_early() {
        for signal in signals() {
            let took = timed(&signal, Duration::from_millis(2));
            assert!(took[0] >= Duration::from_micros(1500), "{took:?}");
        }
    }

    #[test]
    fn deadlines_pass_close_to_on_time() {
        let signal = Signal::new();
        // Generous for loaded machines, and measured again while late, as
        // other tests share the machine; the Windows tick is 15.6 ms, so
        // waits on it are late every time.
        let medians: Vec<Duration> = (0..3)
            .map(|_| timed(&signal, Duration::from_millis(5))[5])
            .take_while(|&median| median >= Duration::from_millis(9))
            .collect();
        assert!(medians.len() < 3, "late every time: {medians:?}");
    }

    #[test]
    fn notify_all_wakes_every_waiter() {
        let state = Arc::new((Mutex::new(0), WaitQueue::default()));
        let threads: Vec<_> = (0..3)
            .map(|_| {
                let state = Arc::clone(&state);
                std::thread::spawn(move || {
                    let (lock, queue) = &*state;
                    let mut guard = lock.lock();
                    *guard += 1;
                    while *guard < 10 {
                        queue.wait(&mut guard, None);
                    }
                })
            })
            .collect();
        let (lock, queue) = &*state;
        loop {
            let mut guard = lock.lock();
            if *guard == 3 {
                *guard = 10;
                break;
            }
            queue.wait(&mut guard, Some(Instant::now() + Duration::from_millis(1)));
        }
        queue.notify_all();
        for t in threads {
            t.join().unwrap();
        }
        assert!(state.1.waiters.lock().is_empty());
    }
}
