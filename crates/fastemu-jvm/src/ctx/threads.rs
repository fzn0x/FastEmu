//! Threads, waiting and posted work, for natives.

use std::time::{Duration, Instant};

use fastemu_core::LogLevel;

use super::{Ctx, JResult, Throw};
use crate::thread;
use crate::value::ObjRef;

/// Why [`Ctx::wait_until`] returned. Wakeups are not precise: callers
/// check their own condition again, as with any condition variable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    /// Another thread changed something: released a monitor, notified,
    /// ended, posted a task, or called [`Ctx::notify_waiters`].
    Notified,
    /// The deadline passed.
    TimedOut,
    /// The thread was interrupted. The flag stays set.
    Interrupted,
    /// The virtual machine is paused.
    Paused,
}

impl Ctx<'_> {
    /// Releases the VM lock and waits for a change, until `deadline` if
    /// given. Returns `Err(Throw::Stop)` when the virtual machine stops.
    /// Rooted references stay valid; unrooted ones may not.
    pub fn wait_until(&mut self, deadline: Option<Instant>) -> JResult<Wake> {
        if self.is_interrupted() {
            return Ok(Wake::Interrupted);
        }
        if self.is_paused() {
            return Ok(Wake::Paused);
        }
        let timed_out = self.wait_raw(deadline)?;
        Ok(if self.is_interrupted() {
            Wake::Interrupted
        } else if self.is_paused() {
            Wake::Paused
        } else if timed_out {
            Wake::TimedOut
        } else {
            Wake::Notified
        })
    }

    /// Wakes every thread in [`Ctx::wait_until`] and in Java waits, so
    /// they check their conditions again.
    pub fn notify_waiters(&self) {
        self.shared.cond.notify_all();
    }

    /// `Thread.sleep`: waits `duration`, throwing `InterruptedException`
    /// (and clearing the flag) if the thread is interrupted.
    pub fn sleep(&mut self, duration: Duration) -> JResult<()> {
        let deadline = Instant::now() + duration;
        loop {
            if self.take_interrupt() {
                return Err(
                    self.new_exception("java/lang/InterruptedException", Some("sleep interrupted"))
                );
            }
            if Instant::now() >= deadline {
                break;
            }
            self.wait_raw(Some(deadline))?;
        }
        while self.is_paused() {
            self.wait_raw(None)?;
        }
        Ok(())
    }

    /// Lets other threads run, as `Thread.yield` does; also a safepoint.
    pub fn yield_now(&mut self) -> JResult<()> {
        self.safepoint()
    }

    /// Whether this thread's interrupt flag is set.
    pub fn is_interrupted(&self) -> bool {
        self.state
            .threads
            .get(&self.tid)
            .is_some_and(|e| e.interrupted)
    }

    /// Clears the interrupt flag, returning whether it was set.
    pub fn take_interrupt(&mut self) -> bool {
        self.state
            .threads
            .get_mut(&self.tid)
            .is_some_and(|e| std::mem::take(&mut e.interrupted))
    }

    /// The `java.lang.Thread` object of this thread.
    pub fn current_thread(&mut self) -> ObjRef {
        let obj = self.state.threads[&self.tid].java_thread;
        self.root(obj)
    }

    /// Starts a Java thread whose body is `body`, as MIDP's event thread
    /// and `Timer` threads are, and returns its `Thread` object. An
    /// exception the body returns is reported as uncaught. The closure
    /// must not capture [`ObjRef`]s; pass [`crate::GlobalRef`]s.
    pub fn spawn_thread(
        &mut self,
        name: &str,
        body: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static,
    ) -> JResult<ObjRef> {
        match thread::spawn_with_new_object(self.shared, &mut self.state, name, Box::new(body)) {
            Ok(obj) => Ok(self.root(obj)),
            Err(message) => Err(self.new_exception("java/lang/OutOfMemoryError", Some(&message))),
        }
    }

    /// Runs every task [`crate::Vm::post`] queued, in order, and returns
    /// how many ran. An exception a task throws is reported as uncaught
    /// and the next task runs.
    pub fn run_posted(&mut self) -> JResult<usize> {
        let mut ran = 0;
        while let Some(task) = self.state.posted.pop_front() {
            ran += 1;
            let result = self.local_frame(|ctx| task(ctx));
            match result {
                Ok(()) => {}
                Err(Throw::Stop) => return Err(Throw::Stop),
                Err(Throw::Exception(e)) => {
                    self.log(LogLevel::Err, "Exception in a posted task:");
                    crate::lang::throwable::print_stack_trace(self, e, LogLevel::Err);
                }
            }
        }
        Ok(ran)
    }

    /// Queues `task` for [`Ctx::run_posted`], as [`crate::Vm::post`] does.
    /// Natives use this one: `Vm::post` takes the VM lock, which the
    /// calling thread already holds.
    pub fn post(&mut self, task: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static) {
        self.state.posted.push_back(Box::new(task));
        self.shared.cond.notify_all();
    }

    /// Asks every thread to stop, as [`crate::Vm::stop`] does, from native
    /// code (which holds the VM lock `Vm::stop` would wait for). The
    /// calling native should then return `Err(Throw::Stop)`.
    pub fn stop_vm(&mut self) {
        self.shared
            .stopping
            .store(true, std::sync::atomic::Ordering::Release);
        self.shared.cond.notify_all();
    }

    /// Waits until a task is posted or `deadline` passes, and returns
    /// whether tasks are waiting.
    pub fn wait_posted(&mut self, deadline: Option<Instant>) -> JResult<bool> {
        while self.state.posted.is_empty() {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                return Ok(false);
            }
            self.wait_raw(deadline)?;
        }
        Ok(true)
    }
}
