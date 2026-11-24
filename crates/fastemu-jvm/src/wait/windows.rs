//! A wakeup signal from a Win32 event and a high-resolution waitable
//! timer.
//!
//! Condition variables, `WaitOnAddress` and `Sleep` time out on the system
//! timer tick, 15.6 ms unless some process raised the timer resolution,
//! so a 10 ms `Thread.sleep` took 16 ms and a game loop that sleeps each
//! frame ran at 52 frames a second instead of 60. A waitable timer made
//! with `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` (Windows 10 1803 and
//! later) fires within about half a millisecond of its due time without
//! raising the resolution for the whole system; Rust's own
//! `thread::sleep` uses one too. The thread waits on the timer and an
//! event at once, so a raise still wakes it at once.

use std::ptr;
use std::time::Instant;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, INFINITE,
    ResetEvent, SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS, WaitForMultipleObjects,
    WaitForSingleObject,
};

/// A kernel object handle, closed when dropped.
struct Handle(HANDLE);

// SAFETY: a kernel handle is a process-wide index, which any thread may
// wait on, signal and close.
unsafe impl Send for Handle {}
// SAFETY: as above; the calls made on it are thread-safe in Win32.
unsafe impl Sync for Handle {}

impl Handle {
    fn new(handle: HANDLE) -> Option<Self> {
        (!handle.is_null()).then_some(Handle(handle))
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle is open and owned by this value alone.
        unsafe { CloseHandle(self.0) };
    }
}

/// A flag one thread waits on and others raise.
pub(super) struct Signal {
    /// An auto-reset event: the flag.
    event: Handle,
    /// The timer for deadlines; `None` if Windows made none, and waits
    /// then time out on the system tick.
    timer: Option<Handle>,
}

impl Signal {
    /// A lowered signal, or `None` if Windows has no event to give.
    pub(super) fn new() -> Option<Self> {
        // SAFETY: no security attributes and no name; an auto-reset event
        // that starts lowered. Returns null on failure.
        let event = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
        let event = Handle::new(event)?;
        // Windows before 10 1803 refuses the high-resolution flag.
        let timer = [CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, 0]
            .into_iter()
            .find_map(|flags| {
                // SAFETY: no attributes and no name; returns null on failure.
                let timer = unsafe {
                    CreateWaitableTimerExW(ptr::null(), ptr::null(), flags, TIMER_ALL_ACCESS)
                };
                Handle::new(timer)
            });
        Some(Signal { event, timer })
    }

    /// Lowers the flag, forgetting a raise no wait took.
    pub(super) fn reset(&self) {
        // SAFETY: the event handle is open for as long as `self` lives.
        unsafe { ResetEvent(self.event.0) };
    }

    /// Raises the flag: wakes the waiting thread, or makes its next wait
    /// return at once.
    pub(super) fn raise(&self) {
        // SAFETY: as in `reset`.
        unsafe { SetEvent(self.event.0) };
    }

    /// Waits until the flag is raised, and lowers it, or until `deadline`
    /// passes. Returns whether the deadline passed first.
    pub(super) fn wait(&self, deadline: Option<Instant>) -> bool {
        let Some(deadline) = deadline else {
            // SAFETY: as in `reset`.
            unsafe { WaitForSingleObject(self.event.0, INFINITE) };
            return false;
        };
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return true;
        }
        if let Some(timer) = &self.timer {
            // A due time relative to now is negative, in 100 ns units.
            let due = i64::try_from(left.as_nanos().div_ceil(100)).map_or(i64::MIN, |t| -t);
            // SAFETY: the timer handle is open, `due` outlives the call,
            // and there is no completion routine. Setting the timer also
            // lowers it, should an earlier deadline have fired unseen.
            let armed =
                unsafe { SetWaitableTimer(timer.0, &raw const due, 0, None, ptr::null(), 0) };
            if armed != 0 {
                let handles = [self.event.0, timer.0];
                // SAFETY: both handles are open; the array outlives the
                // call. The event comes first, so a raise wins a tie.
                let woke = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, INFINITE) };
                return woke != WAIT_OBJECT_0;
            }
        }
        // No timer: whole milliseconds on the system tick, rounded up.
        let millis = u32::try_from(left.as_nanos().div_ceil(1_000_000)).unwrap_or(INFINITE - 1);
        // SAFETY: as in `reset`.
        let woke = unsafe { WaitForSingleObject(self.event.0, millis) };
        woke != WAIT_OBJECT_0
    }
}
