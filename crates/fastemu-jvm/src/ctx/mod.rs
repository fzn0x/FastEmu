//! [`Ctx`]: a Java thread's view of its locked virtual machine, and the
//! API native code works through.

mod calls;
mod objects;
mod roots;
mod threads;

use std::time::Instant;

use parking_lot::MutexGuard;

use crate::thread::{ThreadId, ThreadStack};
use crate::value::ObjRef;
use crate::vm::{VmShared, VmState};

pub use objects::{ArrayElem, FieldId, StaticFieldId};
pub use roots::GlobalRef;
pub use threads::Wake;

/// What unwinds a Java thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Throw {
    /// A Java exception: an instance of `java.lang.Throwable`.
    Exception(ObjRef),
    /// The virtual machine is stopping. Not catchable by Java code;
    /// natives pass it on with `?`.
    Stop,
}

/// The result of anything that can throw.
pub type JResult<T> = Result<T, Throw>;

/// A Java thread running with the VM lock held.
///
/// Native methods get a `&mut Ctx` along with their arguments. Through it
/// they allocate objects, read and write fields and arrays, call Java
/// methods, throw exceptions, wait and start threads. Everything it hands
/// out is valid while this thread holds the lock; see the rooting rules
/// in `docs/architecture/jvm.md` for references kept across calls that
/// can run a collection.
pub struct Ctx<'vm> {
    pub(crate) shared: &'vm VmShared,
    pub(crate) state: MutexGuard<'vm, VmState>,
    pub(crate) thread: Box<ThreadStack>,
    pub(crate) tid: ThreadId,
}

impl<'vm> Ctx<'vm> {
    /// Takes over thread `tid`'s stack from its table entry.
    pub(crate) fn enter(
        shared: &'vm VmShared,
        mut state: MutexGuard<'vm, VmState>,
        tid: ThreadId,
    ) -> Self {
        let mut thread = Box::new(ThreadStack::placeholder());
        let entry = state
            .threads
            .get_mut(&tid)
            .expect("a thread enters after it is registered");
        std::mem::swap(&mut thread, &mut entry.stack);
        Self {
            shared,
            state,
            thread,
            tid,
        }
    }

    /// Moves this thread's stack into its table entry, where a collection
    /// on another thread finds its roots, before the lock is released.
    fn park_stack(&mut self) {
        if let Some(entry) = self.state.threads.get_mut(&self.tid) {
            std::mem::swap(&mut entry.stack, &mut self.thread);
        }
    }

    /// Takes the stack back after the lock is reacquired.
    fn unpark_stack(&mut self) {
        self.park_stack();
    }

    /// Releases the VM lock until the condition variable wakes this thread
    /// or `deadline` passes. Returns whether the deadline passed.
    pub(crate) fn wait_raw(&mut self, deadline: Option<Instant>) -> JResult<bool> {
        if self.shared.is_stopping() {
            return Err(Throw::Stop);
        }
        self.park_stack();
        let timed_out = match deadline {
            Some(d) => self.shared.cond.wait_until(&mut self.state, d).timed_out(),
            None => {
                self.shared.cond.wait(&mut self.state);
                false
            }
        };
        self.unpark_stack();
        if self.shared.is_stopping() {
            return Err(Throw::Stop);
        }
        Ok(timed_out)
    }

    /// A safepoint: stops, pauses or collects as asked, and lets other
    /// threads take the lock for a moment.
    pub(crate) fn safepoint(&mut self) -> JResult<()> {
        if self.shared.is_stopping() {
            return Err(Throw::Stop);
        }
        if self.state.heap.gc_requested() {
            self.collect();
        }
        while self.shared.is_paused() {
            self.wait_raw(None)?;
        }
        self.park_stack();
        MutexGuard::bump(&mut self.state);
        self.unpark_stack();
        if self.shared.is_stopping() {
            return Err(Throw::Stop);
        }
        Ok(())
    }

    /// Runs `f` with the VM lock released, for slow work that does not
    /// touch the virtual machine (file and network I/O, decoding). Other
    /// Java threads run meanwhile and may collect: keep only rooted
    /// references across this call.
    pub fn without_lock<R>(&mut self, f: impl FnOnce() -> R) -> R {
        self.park_stack();
        let r = MutexGuard::unlocked(&mut self.state, f);
        self.unpark_stack();
        r
    }

    /// The thread running this context.
    pub fn thread_id(&self) -> ThreadId {
        self.tid
    }

    /// Whether the virtual machine is stopping; long native loops should
    /// return `Err(Throw::Stop)` when it is.
    pub fn is_stopping(&self) -> bool {
        self.shared.is_stopping()
    }

    pub fn is_paused(&self) -> bool {
        self.shared.is_paused()
    }

    /// A handle to this virtual machine, for host threads.
    pub fn vm(&self) -> crate::Vm {
        crate::Vm {
            shared: self.shared.arc(),
        }
    }
}
