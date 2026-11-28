//! The virtual machine: its shared state, its configuration and the handle
//! hosts control it with.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use fastemu_core::{LogLevel, LogLine};
use parking_lot::{Mutex, MutexGuard};

use crate::class::{ClassId, ClassTable, MethodArena};
use crate::clock::{Clock, VmClock};
use crate::ctx::{Ctx, JResult};
use crate::heap::{Heap, HeapStats};
use crate::lang;
use crate::limits::{VmLimits, cut_log_text};
use crate::monitor::Monitor;
use crate::native::NativeClass;
use crate::source::ClassSource;
use crate::thread::{self, Starter, ThreadEntry, ThreadId};
use crate::value::{HandleMap, ObjRef};
use crate::wait::WaitQueue;

/// Receives what the virtual machine prints: `System.out` and
/// `System.err` line by line, uncaught exceptions, and warnings such as
/// diagnostics. It is called with the virtual machine locked, so it must
/// return quickly (push to a queue; do not block).
pub type LogSink = Arc<dyn Fn(LogLine) + Send + Sync>;

/// Receives each [`Diagnostic`] once per virtual machine.
pub type DiagnosticSink = Arc<dyn Fn(&Diagnostic) + Send + Sync>;

/// Something a game needed that FastEmu lacks. The names are exact, so
/// they say what to implement next.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Diagnostic {
    /// A class no source and no native definition provides, by internal
    /// name.
    MissingClass(String),
    /// A method reference that did not resolve.
    MissingMethod {
        /// The class the reference names, such as
        /// `javax/microedition/lcdui/Graphics`.
        class: String,
        name: String,
        /// The descriptor, such as `(II)V`.
        desc: String,
    },
    /// A field reference that did not resolve.
    MissingField {
        /// The class the reference names.
        class: String,
        name: String,
        /// The field's descriptor, such as `I`.
        desc: String,
    },
}

/// The most characters of a name a [`Diagnostic`] keeps, so a game that
/// asks for classes, methods or fields with huge names cannot make the
/// machine keep or log them whole.
pub(crate) const MAX_REPORTED_NAME: usize = 255;

impl Diagnostic {
    /// This diagnostic with every name cut to `max` characters.
    pub(crate) fn cut_names(mut self, max: usize) -> Self {
        let cut = |s: &mut String| {
            if let Some((at, _)) = s.char_indices().nth(max) {
                s.truncate(at);
            }
        };
        match &mut self {
            Diagnostic::MissingClass(name) => cut(name),
            Diagnostic::MissingMethod { class, name, desc }
            | Diagnostic::MissingField { class, name, desc } => {
                cut(class);
                cut(name);
                cut(desc);
            }
        }
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Diagnostic::MissingClass(name) => write!(f, "missing class {name}"),
            Diagnostic::MissingMethod { class, name, desc } => {
                write!(f, "missing method {class}.{name}{desc}")
            }
            Diagnostic::MissingField { class, name, desc } => {
                write!(f, "missing field {class}.{name}:{desc}")
            }
        }
    }
}

/// How a virtual machine is set up.
#[derive(Clone)]
pub struct VmConfig {
    /// The heap size: what `Runtime.totalMemory()` reports and the limit
    /// past which allocation throws `OutOfMemoryError`.
    pub heap_bytes: usize,
    /// Java frames a thread may stack before `StackOverflowError`.
    pub max_frames: usize,
    /// Times native code may call back into Java on one thread, nested,
    /// before `StackOverflowError`; each level uses the OS thread's stack.
    pub max_native_depth: u32,
    /// The OS stack size of each Java thread.
    pub thread_stack_bytes: usize,
    /// System properties for `System.getProperty`, on top of the CLDC
    /// defaults (`microedition.configuration`, `microedition.encoding`).
    /// `microedition.encoding` names the charset of `new String(bytes)`.
    pub properties: Vec<(String, String)>,
    /// Where `System.out`, `System.err` and warnings go.
    pub log: LogSink,
    /// Where diagnostics go, besides the log.
    pub diagnostics: Option<DiagnosticSink>,
    /// Whether `System.exit` stops the virtual machine. When false it
    /// throws `SecurityException`, as MIDP requires of MIDlets.
    pub allow_exit: bool,
    /// The limits on host resources a game can use, besides the heap.
    pub limits: VmLimits,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            heap_bytes: 8 * 1024 * 1024,
            max_frames: 4096,
            max_native_depth: 256,
            thread_stack_bytes: 16 * 1024 * 1024,
            properties: Vec::new(),
            log: Arc::new(|line: LogLine| match line.level {
                LogLevel::Out => println!("{}", line.text),
                _ => eprintln!("{}", line.text),
            }),
            diagnostics: None,
            allow_exit: true,
            limits: VmLimits::default(),
        }
    }
}

impl fmt::Debug for VmConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VmConfig")
            .field("heap_bytes", &self.heap_bytes)
            .field("max_frames", &self.max_frames)
            .field("max_native_depth", &self.max_native_depth)
            .field("thread_stack_bytes", &self.thread_stack_bytes)
            .field("properties", &self.properties)
            .field("allow_exit", &self.allow_exit)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

/// What a virtual machine's threads were doing at one moment; see
/// [`Vm::activity`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Activity {
    /// Java threads alive.
    pub threads: usize,
    /// Threads waiting for something to happen: a sleep, `Object.wait`,
    /// `Thread.join`, posted work, a pause, or a wait the runtime marks
    /// with [`Ctx::wait_without_lock`], such as the media thread's.
    pub waiting: usize,
    /// Threads waiting for a lock another thread holds: a monitor, or a
    /// class another thread initializes.
    pub blocked: usize,
    /// Tasks posted and not run yet.
    pub posted: usize,
}

impl Activity {
    /// Threads running or ready to run: neither waiting nor blocked.
    pub fn running(&self) -> usize {
        self.threads
            .saturating_sub(self.waiting)
            .saturating_sub(self.blocked)
    }

    /// Whether the machine waits for something to happen: it has threads,
    /// each of them waits, none for a lock, and no task is posted. A
    /// machine whose threads have all ended is not idle.
    pub fn is_idle(&self) -> bool {
        self.threads > 0 && self.waiting == self.threads && self.posted == 0
    }
}

/// Work a host thread hands to the virtual machine; see [`Vm::post`].
pub type Task = Box<dyn FnOnce(&mut Ctx<'_>) -> JResult<()> + Send>;

/// Classes the virtual machine itself needs at hand.
pub(crate) struct WellKnown {
    pub(crate) string: ClassId,
    pub(crate) class: ClassId,
    pub(crate) thread: ClassId,
    pub(crate) throwable: ClassId,
    /// The instance slot of `Throwable.detailMessage`.
    pub(crate) message_slot: usize,
}

/// Everything the VM lock guards.
pub(crate) struct VmState {
    pub(crate) classes: ClassTable,
    pub(crate) heap: Heap,
    pub(crate) threads: HashMap<ThreadId, ThreadEntry>,
    pub(crate) next_thread: u32,
    /// Numbers the default names `Thread-0`, `Thread-1`, ...
    pub(crate) thread_names: u32,
    pub(crate) monitors: HandleMap<Monitor>,
    pub(crate) interned: HashMap<Box<[u16]>, ObjRef>,
    /// `WeakReference` objects and their referents, and each running
    /// `Timer`'s queue and its timer (see `util::timer::reap`).
    pub(crate) weak: HashMap<ObjRef, Option<ObjRef>>,
    pub(crate) globals: Vec<Option<ObjRef>>,
    pub(crate) free_globals: Vec<u32>,
    /// Tasks posted and not yet run, at most `limits.max_posted`.
    pub(crate) posted: VecDeque<Task>,
    /// Whether a posted task was dropped for lack of room, which is
    /// logged once.
    pub(crate) posted_full: bool,
    pub(crate) extensions: HashMap<TypeId, Box<dyn Any + Send>>,
    pub(crate) properties: HashMap<String, String>,
    /// Bytes printed to `System.out` and `System.err` since the last line
    /// break.
    pub(crate) out_line: Vec<u8>,
    pub(crate) err_line: Vec<u8>,
    /// The `Math.random` generator's state, 0 until first used.
    pub(crate) random: u64,
    /// The diagnostics reported so far, at most `limits.max_reported`.
    pub(crate) reported: HashSet<Diagnostic>,
    /// Whether a diagnostic was left out for lack of room, which is
    /// logged once.
    pub(crate) reported_full: bool,
    pub(crate) wk: WellKnown,
    pub(crate) exit_code: Option<i32>,
    pub(crate) live_threads: usize,
}

impl VmState {
    /// Queues `task` for [`Ctx::run_posted`], unless `limits.max_posted`
    /// tasks wait already. Then the task comes back to the caller to drop,
    /// and the first time that happens a warning goes to the log.
    pub(crate) fn push_posted(&mut self, shared: &VmShared, task: Task) -> Option<Task> {
        if self.posted.len() < shared.config.limits.max_posted {
            self.posted.push_back(task);
            return None;
        }
        if !std::mem::replace(&mut self.posted_full, true) {
            shared.log(
                LogLevel::Warn,
                "too many tasks wait for the virtual machine; later ones are dropped",
            );
        }
        Some(task)
    }
}

/// The parts of a virtual machine every thread shares.
pub(crate) struct VmShared {
    pub(crate) state: Mutex<VmState>,
    /// Where blocked threads wait; every change notifies it.
    pub(crate) cond: WaitQueue,
    pub(crate) stopping: AtomicBool,
    /// The pause flag, and the run time that stops with it.
    pub(crate) clock: Arc<Clock>,
    pub(crate) config: VmConfig,
    /// Global roots whose handles were dropped, cleared at the next
    /// collection.
    pub(crate) released_globals: Mutex<Vec<u32>>,
    pub(crate) this: Weak<VmShared>,
    pub(crate) id: u64,
    /// Every method of the machine, by id: the class table's, outside the
    /// lock, so the interpreter borrows a method without counting it.
    pub(crate) methods: Arc<MethodArena>,
}

/// How long [`Vm::stop`], [`Vm::pause`] and [`Vm::resume`] wait for the VM
/// lock before they wake the waiting threads without it. A thread that
/// holds it for longer is running, and sees the flag at its next call into
/// Java or its next wait.
const WAKE_LOCK_WAIT: Duration = Duration::from_millis(100);

/// How long [`Vm::flush_output`] waits for the VM lock before it skips the
/// flush.
const FLUSH_LOCK_WAIT: Duration = Duration::from_millis(250);

/// How long [`Vm::activity`] waits for the VM lock before it gives up. A
/// thread holds it for microseconds between safepoints; one that holds it
/// this long is running.
const ACTIVITY_LOCK_WAIT: Duration = Duration::from_millis(100);

/// How long [`Vm::spawn`] and [`Vm::post`] wait for the VM lock before
/// they give up. A thread holds it for microseconds at a time; one that
/// holds it for this long is wedged.
const WEDGED_LOCK_WAIT: Duration = Duration::from_secs(1);

impl VmShared {
    #[inline]
    pub(crate) fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn is_paused(&self) -> bool {
        self.clock.is_paused()
    }

    pub(crate) fn arc(&self) -> Arc<VmShared> {
        self.this
            .upgrade()
            .expect("a running thread keeps its virtual machine alive")
    }

    /// Wakes every thread waiting on the VM's wait queue. Taking the lock
    /// first makes sure a thread that checked its condition is already in
    /// the queue, so the wakeup is not lost.
    ///
    /// The lock is waited for at most [`WAKE_LOCK_WAIT`], since a thread
    /// that never reaches a safepoint holds it for ever, and the caller is
    /// a host thread that has to go on, to stop that very thread. Then the
    /// wake goes out without the lock, and a thread that was between its
    /// check and its wait is told not to block (see
    /// [`WaitQueue::notify_all_unlocked`]).
    pub(crate) fn wake_all(&self) {
        if self.state.try_lock_for(WAKE_LOCK_WAIT).is_some() {
            self.cond.notify_all();
        } else {
            self.cond.notify_all_unlocked();
        }
    }

    /// The VM lock, if it is free by `deadline`, or whenever with none.
    fn lock_by(&self, deadline: Option<Instant>) -> Option<MutexGuard<'_, VmState>> {
        match deadline {
            Some(deadline) => self.state.try_lock_until(deadline),
            None => Some(self.state.lock()),
        }
    }

    /// Sends a line to the log sink, cut to `limits.max_log_line_bytes`.
    pub(crate) fn log(&self, level: LogLevel, text: impl Into<String>) {
        let text = cut_log_text(text.into(), self.config.limits.max_log_line_bytes);
        (self.config.log)(LogLine { level, text });
    }
}

/// A virtual machine: one game's heap, classes and threads.
///
/// `Vm` is a cheap handle; clones share the machine. Its methods are for
/// host threads (the UI, the launcher): they may take the VM lock, so
/// native code, which runs with that lock held, uses the [`Ctx`]
/// equivalents ([`Ctx::post`], [`Ctx::stop_vm`]) instead. Each FastEmu
/// instance runs its own `Vm`, so instances run in parallel on different
/// cores.
#[derive(Clone)]
pub struct Vm {
    pub(crate) shared: Arc<VmShared>,
}

/// A handle that does not keep its virtual machine alive, for state the
/// machine itself owns (an extension, a native's closure) that needs to
/// reach it: a [`Vm`] there would be a cycle and leak the machine. A host
/// that should let a finished machine go, such as an app's handle on a
/// game, keeps one too.
#[derive(Clone)]
pub struct WeakVm(Weak<VmShared>);

impl WeakVm {
    /// The machine, if anything still holds it.
    ///
    /// The [`Vm`] it returns holds the machine too. If every other holder
    /// lets go meanwhile, the caller's thread is the one that frees it when
    /// it drops that `Vm`, so a thread that must stay quick, such as an
    /// app's UI thread, uses [`WeakVm::is_freed`] instead.
    pub fn upgrade(&self) -> Option<Vm> {
        self.0.upgrade().map(|shared| Vm { shared })
    }

    /// Whether nothing holds the machine any more: no [`Vm`] handle and no
    /// thread of its own. Its memory is then freed, or being freed on the
    /// thread that let go last. Unlike [`WeakVm::upgrade`], it never holds
    /// the machine, so the caller never ends up freeing it. It takes no
    /// lock.
    pub fn is_freed(&self) -> bool {
        self.0.strong_count() == 0
    }
}

impl fmt::Debug for WeakVm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WeakVm")
    }
}

impl fmt::Debug for Vm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vm").field("id", &self.shared.id).finish()
    }
}

impl Vm {
    /// Creates a virtual machine that loads classes from `source`.
    ///
    /// `natives` adds classes implemented in Rust (MIDP and the vendor
    /// APIs) to the CLDC library; one with the name of a library class
    /// replaces it.
    ///
    /// # Panics
    ///
    /// When `natives` replaces a core class (`java/lang/Object`, `String`,
    /// `Class`, `Thread`, `Throwable`) with one that cannot link.
    pub fn new(config: VmConfig, source: Arc<dyn ClassSource>, natives: Vec<NativeClass>) -> Vm {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let mut library: HashMap<String, NativeClass> = crate::library()
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        for class in natives {
            library.insert(class.name.clone(), class);
        }
        let mut classes = ClassTable::new(
            source,
            library.into_values().collect(),
            config.limits.max_class_bytes,
        );
        let mut load = |name: &str| {
            classes
                .load(name)
                .unwrap_or_else(|e| panic!("core class {name} does not link: {}", e.message))
        };
        load("java/lang/Object");
        let string = load("java/lang/String");
        let class = load("java/lang/Class");
        let thread = load("java/lang/Thread");
        let throwable = load("java/lang/Throwable");
        let message_slot = classes
            .get(throwable)
            .declared_field("detailMessage", "Ljava/lang/String;")
            .expect("Throwable declares detailMessage")
            .slot as usize;
        let mut properties: HashMap<String, String> = [
            ("microedition.configuration", "CLDC-1.1"),
            ("microedition.encoding", "UTF-8"),
            ("microedition.platform", "FastEmu"),
            ("microedition.locale", "en-US"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
        properties.extend(config.properties.iter().cloned());
        let state = VmState {
            classes,
            heap: Heap::new(config.heap_bytes),
            threads: HashMap::new(),
            next_thread: 1,
            thread_names: 0,
            monitors: HandleMap::default(),
            interned: HashMap::new(),
            weak: HashMap::new(),
            globals: Vec::new(),
            free_globals: Vec::new(),
            posted: VecDeque::new(),
            posted_full: false,
            extensions: HashMap::new(),
            properties,
            out_line: Vec::new(),
            err_line: Vec::new(),
            random: 0,
            reported: HashSet::new(),
            reported_full: false,
            wk: WellKnown {
                string,
                class,
                thread,
                throwable,
                message_slot,
            },
            exit_code: None,
            live_threads: 0,
        };
        let methods = Arc::clone(state.classes.arena());
        let shared = Arc::new_cyclic(|this| VmShared {
            methods,
            state: Mutex::new(state),
            cond: WaitQueue::default(),
            stopping: AtomicBool::new(false),
            clock: Arc::new(Clock::new()),
            config,
            released_globals: Mutex::new(Vec::new()),
            this: this.clone(),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        });
        Vm { shared }
    }

    /// A handle that does not keep the machine alive.
    pub fn downgrade(&self) -> WeakVm {
        WeakVm(Arc::downgrade(&self.shared))
    }

    /// Runs `f` on the calling OS thread as a Java thread named `name`,
    /// with the virtual machine locked, and returns what it returns.
    ///
    /// Use it to set the machine up and for short work. `f` runs on the
    /// caller's stack, so deep calls into Java belong on a thread from
    /// [`Vm::spawn`]. Blocks while other threads hold the lock.
    pub fn attach<R>(&self, name: &str, f: impl FnOnce(&mut Ctx<'_>) -> R) -> R {
        thread::attach(&self.shared, name, f)
    }

    /// Starts a Java thread named `name` whose body is the Rust closure
    /// `body`; MIDP's event thread is one. An exception the body returns
    /// is reported as uncaught.
    ///
    /// Fails with [`std::io::ErrorKind::TimedOut`] when a thread holds the
    /// VM lock for a second without a safepoint, as one wedged in a native
    /// does. It fails too when the machine has as many threads as
    /// [`VmLimits::max_threads`] plus a reserve of 16 for the host, so a
    /// game that has used up its own threads cannot keep the host from
    /// starting the ones it needs.
    pub fn spawn(
        &self,
        name: &str,
        body: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static,
    ) -> std::io::Result<()> {
        let Some(mut state) = self.shared.state.try_lock_for(WEDGED_LOCK_WAIT) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "the virtual machine is busy: a thread holds its lock",
            ));
        };
        thread::spawn_with_new_object(
            &self.shared,
            &mut state,
            name,
            Box::new(body),
            Starter::Host,
        )
        .map(|_| ())
        .map_err(std::io::Error::other)
    }

    /// Starts the thread `main` running `public static void
    /// main(String[])` of class `class` (a binary or internal name), for
    /// tests and command-line programs.
    pub fn run_main(&self, class: &str, args: &[&str]) -> std::io::Result<()> {
        let class = crate::descriptor::internal_name(class);
        let args: Vec<String> = args.iter().map(|&a| a.to_owned()).collect();
        self.spawn("main", move |ctx| {
            let cid = ctx.find_class(&class)?;
            let string = ctx.find_class("java/lang/String")?;
            let array = ctx.new_object_array(string, args.len() as i32)?;
            for (i, arg) in args.iter().enumerate() {
                let s = ctx.new_string(arg)?;
                ctx.array_mut::<Option<ObjRef>>(array)?[i] = Some(s);
            }
            ctx.call_static_in(cid, "main", "([Ljava/lang/String;)V", &[array.into()])?;
            Ok(())
        })
    }

    /// Queues `task` to run on a Java thread that drains posted tasks
    /// ([`Ctx::run_posted`]); MIDP's event thread does. Returns after
    /// queueing; takes the VM lock briefly.
    ///
    /// The task runs later, so it must not carry [`ObjRef`]s: hand it
    /// [`crate::GlobalRef`]s instead.
    ///
    /// When a thread holds the VM lock for a second without a safepoint,
    /// as one wedged in a native does, the task is dropped without running
    /// and a warning goes to the log: waiting longer would only hold up
    /// the host thread, which is often the one that has to stop the
    /// machine. So is a task posted while [`crate::VmLimits::max_posted`] others
    /// wait, which only a thread that never drains them lets happen; the
    /// first one dropped for that logs a warning too.
    pub fn post(&self, task: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static) {
        let rejected = match self.shared.state.try_lock_for(WEDGED_LOCK_WAIT) {
            Some(mut state) => state.push_posted(&self.shared, Box::new(task)),
            None => {
                self.shared.log(
                    LogLevel::Warn,
                    "dropped a task posted to a virtual machine that a thread holds locked",
                );
                return;
            }
        };
        // A task that found no room is dropped here, with the lock free.
        if rejected.is_none() {
            self.shared.cond.notify_all();
        }
    }

    /// Asks every thread to end: each unwinds at its next safepoint,
    /// wait or call from native code into Java, without running `finally`
    /// blocks. Returns at once (after 100 ms at most when a thread holds
    /// the VM lock); see [`Vm::wait_finished`].
    pub fn stop(&self) {
        self.shared.stopping.store(true, Ordering::Release);
        self.shared.wake_all();
    }

    /// Parks every thread at its next safepoint until [`Vm::resume`].
    /// Waits and sleeps that end while paused park too, and the run time
    /// ([`Vm::clock`]) stands still.
    pub fn pause(&self) {
        self.shared.clock.set_paused(true);
        self.shared.wake_all();
    }

    /// Lets paused threads run again.
    pub fn resume(&self) {
        self.shared.clock.set_paused(false);
        self.shared.wake_all();
    }

    /// Whether [`Vm::pause`] is in effect.
    pub fn is_paused(&self) -> bool {
        self.shared.is_paused()
    }

    /// The machine's run time, which leaves out the time it spent paused.
    pub fn clock(&self) -> VmClock {
        VmClock(Arc::clone(&self.shared.clock))
    }

    /// Whether [`Vm::stop`] (or `System.exit`) was called.
    pub fn is_stopping(&self) -> bool {
        self.shared.is_stopping()
    }

    /// Java threads still running.
    pub fn thread_count(&self) -> usize {
        self.shared.state.lock().live_threads
    }

    /// Waits until every Java thread has ended, at most `timeout` if
    /// given, the wait for the VM lock included. Returns whether they
    /// have.
    pub fn wait_finished(&self, timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|t| Instant::now() + t);
        loop {
            let Some(state) = self.shared.lock_by(deadline) else {
                return false;
            };
            if state.live_threads == 0 {
                return true;
            }
            if self.shared.cond.wait_released(state, deadline) {
                // The time is up: one more look.
                return self
                    .shared
                    .lock_by(deadline)
                    .is_some_and(|state| state.live_threads == 0);
            }
        }
    }

    /// What every thread is doing now, for tools that tell a game waiting
    /// on a still screen from a stuck one. `None` when a thread holds the
    /// VM lock for 100 ms without a safepoint, as one wedged in a native
    /// does: that thread is running.
    ///
    /// A thread counts as waiting from the moment it waits until it has
    /// the lock back, so one look can find a thread asleep that wakes and
    /// works a moment later. A thread that never waits, such as one that
    /// spins, is never counted as waiting.
    pub fn activity(&self) -> Option<Activity> {
        let state = self.shared.state.try_lock_for(ACTIVITY_LOCK_WAIT)?;
        let mut activity = Activity {
            threads: state.threads.len(),
            posted: state.posted.len(),
            ..Activity::default()
        };
        for entry in state.threads.values() {
            match entry.doing {
                thread::Doing::Running => {}
                thread::Doing::Waiting => activity.waiting += 1,
                thread::Doing::Blocked => activity.blocked += 1,
            }
        }
        Some(activity)
    }

    /// The status `System.exit` was called with, if it was.
    pub fn exit_code(&self) -> Option<i32> {
        self.shared.state.lock().exit_code
    }

    /// What the heap holds now.
    pub fn heap_stats(&self) -> HeapStats {
        self.shared.state.lock().heap.stats()
    }

    /// Writes out what `System.out` and `System.err` hold past their last
    /// line break. Skips it when a thread holds the VM lock for a quarter
    /// of a second, as one that never reaches a safepoint does.
    pub fn flush_output(&self) {
        if let Some(mut state) = self.shared.state.try_lock_for(FLUSH_LOCK_WAIT) {
            lang::system::flush_lines(&self.shared, &mut state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_weak_handle_sees_the_machine_freed_without_holding_it() {
        let config = VmConfig {
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(crate::MemorySource::new()), Vec::new());
        let weak = vm.downgrade();
        let clone = vm.clone();
        assert!(!weak.is_freed());
        drop(vm);
        assert!(!weak.is_freed(), "a clone still holds it");
        assert!(weak.upgrade().is_some());
        // The upgraded handle is gone again: the clone is the last holder.
        assert!(!weak.is_freed());
        drop(clone);
        assert!(weak.is_freed());
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn a_freed_machine_frees_its_methods() {
        let config = VmConfig {
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(crate::MemorySource::new()), Vec::new());
        let arena = Arc::downgrade(&vm.shared.methods);
        let object = {
            let state = vm.shared.state.lock();
            let id = state.classes.lookup("java/lang/Object").expect("loaded");
            let method = Arc::clone(&state.classes.get(id).methods[0]);
            assert!(Arc::ptr_eq(vm.shared.methods.get(method.id), &method));
            Arc::downgrade(&method)
        };
        drop(vm);
        assert!(arena.upgrade().is_none(), "the arena went with the machine");
        assert!(object.upgrade().is_none(), "and its methods with it");
    }

    #[test]
    fn a_diagnostic_keeps_255_characters_of_each_name() {
        let long = "\u{20ac}".repeat(300);
        let cut = Diagnostic::MissingMethod {
            class: long.clone(),
            name: "short".into(),
            desc: long.clone(),
        }
        .cut_names(MAX_REPORTED_NAME);
        assert_eq!(
            cut,
            Diagnostic::MissingMethod {
                class: "\u{20ac}".repeat(255),
                name: "short".into(),
                desc: "\u{20ac}".repeat(255),
            }
        );
        let field = Diagnostic::MissingField {
            class: "a".into(),
            name: long.clone(),
            desc: "I".into(),
        };
        assert!(matches!(
            field.cut_names(MAX_REPORTED_NAME),
            Diagnostic::MissingField { name, .. } if name.chars().count() == 255
        ));
        assert_eq!(
            Diagnostic::MissingClass(long).cut_names(10),
            Diagnostic::MissingClass("\u{20ac}".repeat(10))
        );
        assert_eq!(
            Diagnostic::MissingClass("exact".into()).cut_names(5),
            Diagnostic::MissingClass("exact".into())
        );
    }
}
