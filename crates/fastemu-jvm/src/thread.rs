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
}

impl ThreadStack {
    pub(crate) fn new(max_native_depth: u32) -> Self {
        Self {
            slots: vec![Value::Void; 256],
            frames: Vec::with_capacity(32),
            locals: Vec::new(),
            native_depth: 0,
            max_native_depth,
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
        }
    }

    /// One past the highest live slot.
    #[inline]
    pub(crate) fn top(&self) -> usize {
        self.frames.last().map_or(0, |f| f.sp as usize)
    }

    /// Makes sure slots up to `needed` exist.
    #[inline]
    pub(crate) fn reserve(&mut self, needed: usize) {
        if self.slots.len() < needed {
            let len = needed.max(self.slots.len() * 2);
            self.slots.resize(len, Value::Void);
        }
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

/// Adds a thread for `java_thread` to the table and starts its OS thread.
pub(crate) fn spawn(
    shared: &VmShared,
    state: &mut VmState,
    java_thread: ObjRef,
    name: String,
    body: ThreadBody,
) -> Result<ThreadId, String> {
    let tid = register(shared, state, java_thread, name.clone());
    let arc = shared.arc();
    let result = std::thread::Builder::new()
        .name(format!("java {name}"))
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
/// `body`.
pub(crate) fn spawn_with_new_object(
    shared: &VmShared,
    state: &mut VmState,
    name: &str,
    body: ThreadBody,
) -> Result<ObjRef, String> {
    let object = lang::thread::new_thread_object(state, name);
    if let Some(data) = state
        .heap
        .get_mut(object)
        .payload_mut::<lang::thread::ThreadData>()
    {
        data.started = true;
    }
    spawn(shared, state, object, name.to_owned(), body)?;
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
            stack: Box::new(ThreadStack::new(shared.config.max_native_depth)),
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
    let outcome = catch_unwind(AssertUnwindSafe(|| body(&mut ctx)));
    match outcome {
        Ok(Ok(()) | Err(Throw::Stop)) => {}
        Ok(Err(Throw::Exception(e))) => lang::throwable::report_uncaught(&mut ctx, e),
        Err(panic) => {
            let message = panic_message(&*panic);
            shared.log(
                LogLevel::Err,
                format!("internal error in the virtual machine: {message}; stopping it"),
            );
            shared
                .stopping
                .store(true, std::sync::atomic::Ordering::Release);
        }
    }
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
