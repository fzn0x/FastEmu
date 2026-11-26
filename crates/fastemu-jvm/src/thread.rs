//! Java threads: each one is an OS thread that runs while it holds the VM
//! lock, with its frames and operand stacks in a [`ThreadStack`].

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;

use fastemu_core::LogLevel;

use crate::class::Method;
use crate::ctx::{Ctx, JResult, Throw};
use crate::lang;
use crate::value::{ObjRef, Value};
use crate::vm::{VmShared, VmState};

/// Identifies a Java thread within one virtual machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ThreadId(pub(crate) u32);

/// One activation on a thread's stack.
pub(crate) struct Frame {
    pub(crate) method: Arc<Method>,
    /// For bytecode: the instruction to run next, so `pc - 1` is the one
    /// running (the call, in a caller's frame).
    pub(crate) pc: u32,
    /// The slot of local 0.
    pub(crate) bp: u32,
    /// One past the top of the operand stack. For a native frame, one
    /// past its arguments, which stay on the stack as roots.
    pub(crate) sp: u32,
    /// The monitor a synchronized method holds.
    pub(crate) sync: Option<ObjRef>,
    pub(crate) native: bool,
}

/// A thread's frames, its slots (locals and operand stacks of every frame,
/// contiguous) and the references its native frames hold.
pub(crate) struct ThreadStack {
    pub(crate) slots: Vec<Value>,
    pub(crate) frames: Vec<Frame>,
    /// References created or returned while native code runs, kept alive
    /// until the native frame that made them returns.
    pub(crate) locals: Vec<ObjRef>,
    /// Native code calling into Java, nested.
    pub(crate) native_depth: u32,
    pub(crate) max_native_depth: u32,
    /// The most slots the thread may have ([`crate::VmLimits::max_stack_slots`]);
    /// `slots`, and the buffer that holds it, never grow past it.
    pub(crate) max_slots: usize,
}

/// A frame asked for more slots than the thread may have.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct StackFull;

impl ThreadStack {
    pub(crate) fn new(max_native_depth: u32, max_slots: usize) -> Self {
        Self {
            slots: vec![Value::Void; 256.min(max_slots)],
            frames: Vec::with_capacity(32),
            locals: Vec::new(),
            native_depth: 0,
            max_native_depth,
            max_slots,
        }
    }

    /// An empty stack that stands in for a running thread's real one.
    pub(crate) fn placeholder() -> Self {
        Self {
            slots: Vec::new(),
            frames: Vec::new(),
            locals: Vec::new(),
            native_depth: 0,
            max_native_depth: 0,
            max_slots: 0,
        }
    }

    /// One past the highest live slot.
    #[inline]
    pub(crate) fn top(&self) -> usize {
        self.frames.last().map_or(0, |f| f.sp as usize)
    }

    /// Makes sure slots up to `needed` exist, or says the thread may not
    /// have that many. The stack keeps what it grew to, so a thread that
    /// once recursed deep holds up to `max_slots` slots (16 bytes each)
    /// until it ends. The stack doubles as it grows, cut at `max_slots`,
    /// and its buffer holds exactly its slots, so the buffer is never more
    /// than those 16 bytes a slot either.
    #[inline]
    pub(crate) fn reserve(&mut self, needed: usize) -> Result<(), StackFull> {
        // `slots` is never longer than `max_slots`, so this is also the
        // answer for a request that is within the limit.
        if self.slots.len() >= needed {
            return Ok(());
        }
        if needed > self.max_slots {
            return Err(StackFull);
        }
        let len = needed.max(self.slots.len() * 2).min(self.max_slots);
        // `resize` alone doubles the buffer, which is twice `max_slots`
        // when the limit cut the length: 4 MiB for a stack of 2 MiB.
        self.slots.reserve_exact(len - self.slots.len());
        self.slots.resize(len, Value::Void);
        Ok(())
    }
}

/// A thread's entry in the thread table.
pub(crate) struct ThreadEntry {
    pub(crate) java_thread: ObjRef,
    pub(crate) name: String,
    /// The thread's stack while it does not hold the lock (so collections
    /// on other threads see its roots), a placeholder while it does.
    pub(crate) stack: Box<ThreadStack>,
    pub(crate) interrupted: bool,
    /// Set by `notify` for a thread in `Object.wait`.
    pub(crate) notified: bool,
}

/// What a Rust-bodied Java thread runs.
pub(crate) type ThreadBody = Box<dyn FnOnce(&mut Ctx<'_>) -> JResult<()> + Send>;

/// Threads the host may start beyond [`crate::VmLimits::max_threads`]: the
/// event thread, the lifecycle threads and the threads the library runs
/// for a game. A game that has used up its own threads cannot starve them.
pub(crate) const HOST_RESERVE: usize = 16;

/// Who asks for a thread. The machine counts every live thread, the host's
/// own among them, and lets a game have [`crate::VmLimits::max_threads`] of
/// them in all; the host may go [`HOST_RESERVE`] past that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Starter {
    /// `Thread.start` and `Timer`.
    Game,
    /// `Vm::spawn` and the threads the library starts for itself.
    Host,
}

/// Whether one more thread fits for `starter`.
fn check_room(shared: &VmShared, state: &VmState, starter: Starter) -> Result<(), String> {
    let max = shared.config.limits.max_threads;
    let limit = match starter {
        Starter::Game => max,
        Starter::Host => max.saturating_add(HOST_RESERVE),
    };
    if state.live_threads >= limit {
        return Err(format!(
            "unable to create a thread: too many threads (limit {limit})"
        ));
    }
    Ok(())
}

/// The name the OS gets for a thread: Java's name without control
/// characters, cut to 32 characters. `std` panics on a NUL in a thread
/// name, and the name is only a label for debuggers; the Java side keeps
/// the name it was given.
fn os_name(name: &str) -> String {
    let label: String = name.chars().filter(|c| !c.is_control()).take(32).collect();
    format!("java {label}")
}

/// Adds a thread for `java_thread` to the table and starts its OS thread.
/// Fails, with nothing added, when `starter` has no room for another
/// thread.
pub(crate) fn spawn(
    shared: &VmShared,
    state: &mut VmState,
    java_thread: ObjRef,
    name: String,
    body: ThreadBody,
    starter: Starter,
) -> Result<ThreadId, String> {
    check_room(shared, state, starter)?;
    let tid = register(shared, state, java_thread, name.clone());
    let arc = shared.arc();
    let result = std::thread::Builder::new()
        .name(os_name(&name))
        .stack_size(shared.config.thread_stack_bytes)
        .spawn(move || run(&arc, tid, body));
    match result {
        Ok(_) => Ok(tid),
        Err(e) => {
            unregister(state, tid);
            Err(format!("unable to create a thread: {e}"))
        }
    }
}

/// Makes a `java.lang.Thread` object named `name` and starts it with
/// `body`. Without room for the thread, no object is made.
pub(crate) fn spawn_with_new_object(
    shared: &VmShared,
    state: &mut VmState,
    name: &str,
    body: ThreadBody,
    starter: Starter,
) -> Result<ObjRef, String> {
    check_room(shared, state, starter)?;
    let object = lang::thread::new_thread_object(state, name);
    if let Some(data) = state
        .heap
        .get_mut(object)
        .payload_mut::<lang::thread::ThreadData>()
    {
        data.started = true;
    }
    spawn(shared, state, object, name.to_owned(), body, starter)?;
    Ok(object)
}

fn register(shared: &VmShared, state: &mut VmState, java_thread: ObjRef, name: String) -> ThreadId {
    let tid = ThreadId(state.next_thread);
    state.next_thread += 1;
    state.threads.insert(
        tid,
        ThreadEntry {
            java_thread,
            name,
            stack: Box::new(ThreadStack::new(
                shared.config.max_native_depth,
                shared.config.limits.max_stack_slots,
            )),
            interrupted: false,
            notified: false,
        },
    );
    state.live_threads += 1;
    if let Some(data) = state
        .heap
        .get_mut(java_thread)
        .payload_mut::<lang::thread::ThreadData>()
    {
        data.tid = Some(tid);
        data.alive = true;
    }
    tid
}

fn unregister(state: &mut VmState, tid: ThreadId) {
    if let Some(entry) = state.threads.remove(&tid) {
        if let Some(data) = state
            .heap
            .get_mut(entry.java_thread)
            .payload_mut::<lang::thread::ThreadData>()
        {
            data.alive = false;
            data.tid = None;
        }
        state.live_threads -= 1;
    }
}

/// The body of every spawned OS thread.
fn run(shared: &Arc<VmShared>, tid: ThreadId, body: ThreadBody) {
    let guard = shared.state.lock();
    let mut ctx = Ctx::enter(shared, guard, tid);
    let mut outcome = catch_unwind(AssertUnwindSafe(|| body(&mut ctx)));
    if let Ok(Err(Throw::Exception(e))) = outcome {
        // Bytecode can throw an object that is not a Throwable, which the
        // report cannot read. That ends the thread like any other panic.
        outcome = catch_unwind(AssertUnwindSafe(|| {
            lang::throwable::report_uncaught(&mut ctx, e);
            Ok(())
        }));
    }
    if let Err(panic) = outcome {
        let message = panic_message(&*panic);
        shared.log(
            LogLevel::Err,
            format!("internal error in the virtual machine: {message}; stopping it"),
        );
        shared
            .stopping
            .store(true, std::sync::atomic::Ordering::Release);
    }
    // Always: it releases the monitors the thread holds and wakes whoever
    // waits for it.
    ctx.finish();
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_owned()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "a panic".to_owned()
    }
}

/// Runs `f` on the calling OS thread as a temporary Java thread.
pub(crate) fn attach<R>(shared: &VmShared, name: &str, f: impl FnOnce(&mut Ctx<'_>) -> R) -> R {
    let mut guard = shared.state.lock();
    let object = lang::thread::new_thread_object(&mut guard, name);
    let tid = register(shared, &mut guard, object, name.to_owned());
    let mut ctx = Ctx::enter(shared, guard, tid);
    // An attached thread runs on a host stack of unknown size.
    ctx.thread.max_native_depth = ctx.thread.max_native_depth.min(16);
    let outcome = catch_unwind(AssertUnwindSafe(|| f(&mut ctx)));
    ctx.finish();
    match outcome {
        Ok(r) => r,
        Err(panic) => resume_unwind(panic),
    }
}

impl Ctx<'_> {
    /// Ends the current thread: flushes nothing, wakes joiners and drops
    /// it from the table.
    pub(crate) fn finish(mut self) {
        self.thread.frames.clear();
        self.thread.locals.clear();
        let tid = self.tid;
        // Monitors the thread still holds (a body that returned early)
        // are released so other threads do not hang on them.
        let held: Vec<ObjRef> = self
            .state
            .monitors
            .iter()
            .filter(|(_, m)| m.owner == Some(tid))
            .map(|(&obj, _)| obj)
            .collect();
        for obj in held {
            if let Some(m) = self.state.monitors.get_mut(&obj) {
                m.owner = None;
                m.count = 0;
            }
        }
        unregister(&mut self.state, tid);
        if self.state.live_threads == 0 {
            lang::system::flush_lines(self.shared, &mut self.state);
        }
        self.shared.cond.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_doubles_the_stack_up_to_the_limit() {
        let mut stack = ThreadStack::new(16, 1000);
        assert_eq!(stack.slots.len(), 256);
        assert_eq!(stack.reserve(200), Ok(()));
        assert_eq!(stack.slots.len(), 256);
        assert_eq!(stack.reserve(300), Ok(()));
        assert_eq!(stack.slots.len(), 512);
        // Doubling would make 1024: the limit cuts it.
        assert_eq!(stack.reserve(600), Ok(()));
        assert_eq!(stack.slots.len(), 1000);
        assert_eq!(stack.reserve(1000), Ok(()));
        assert_eq!(stack.reserve(1001), Err(StackFull));
        assert_eq!(stack.slots.len(), 1000);
    }

    #[test]
    fn the_buffer_of_a_stack_never_outgrows_its_limit() {
        // Doubling 512 slots makes 1024, which the limit cuts to 1000: the
        // buffer must be cut too.
        let mut stack = ThreadStack::new(16, 1000);
        assert_eq!(stack.reserve(300), Ok(()));
        assert_eq!(stack.reserve(600), Ok(()));
        assert_eq!(stack.slots.len(), 1000);
        assert!(stack.slots.capacity() <= 1000, "{}", stack.slots.capacity());
        // A frame of all the slots but one, then one more slot: the buffer
        // would double to 1998.
        let mut stack = ThreadStack::new(16, 1000);
        assert_eq!(stack.reserve(999), Ok(()));
        assert_eq!(stack.reserve(1000), Ok(()));
        assert!(stack.slots.capacity() <= 1000, "{}", stack.slots.capacity());
        // The default limit: 131071 slots, then 131072, made a buffer of
        // 262142 slots, 4 MiB, for a stack of 2 MiB.
        let mut stack = ThreadStack::new(16, 131_072);
        assert_eq!(stack.reserve(131_071), Ok(()));
        assert_eq!(stack.reserve(131_072), Ok(()));
        let capacity = stack.slots.capacity();
        assert!(capacity <= 131_072, "{capacity}");
        // Below the limit the buffer follows the length, doubling.
        let mut stack = ThreadStack::new(16, 131_072);
        for (needed, len) in [(300, 512), (600, 1024), (5000, 5000), (5001, 10_000)] {
            assert_eq!(stack.reserve(needed), Ok(()));
            assert_eq!((stack.slots.len(), stack.slots.capacity()), (len, len));
        }
    }

    #[test]
    fn a_stack_never_holds_more_slots_than_its_limit() {
        let mut stack = ThreadStack::new(16, 3);
        assert_eq!(stack.slots.len(), 3);
        assert_eq!(stack.reserve(3), Ok(()));
        assert_eq!(stack.reserve(4), Err(StackFull));
        assert_eq!(ThreadStack::placeholder().reserve(1), Err(StackFull));
    }
}
