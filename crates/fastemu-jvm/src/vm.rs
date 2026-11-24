//! The virtual machine: its shared state, its configuration and the handle
//! hosts control it with.

use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use fastemu_core::{LogLevel, LogLine};
use parking_lot::{Condvar, Mutex};

use crate::class::{ClassId, ClassTable};
use crate::ctx::{Ctx, JResult};
use crate::heap::{Heap, HeapStats};
use crate::lang;
use crate::monitor::Monitor;
use crate::native::NativeClass;
use crate::source::ClassSource;
use crate::thread::{self, ThreadEntry, ThreadId};
use crate::value::ObjRef;

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
            .finish_non_exhaustive()
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
    pub(crate) monitors: HashMap<ObjRef, Monitor>,
    pub(crate) interned: HashMap<Box<[u16]>, ObjRef>,
    /// `WeakReference` objects and their referents.
    pub(crate) weak: HashMap<ObjRef, Option<ObjRef>>,
    pub(crate) globals: Vec<Option<ObjRef>>,
    pub(crate) free_globals: Vec<u32>,
    pub(crate) posted: VecDeque<Task>,
    pub(crate) extensions: HashMap<TypeId, Box<dyn Any + Send>>,
    pub(crate) properties: HashMap<String, String>,
    /// Bytes printed to `System.out` and `System.err` since the last line
    /// break.
    pub(crate) out_line: Vec<u8>,
    pub(crate) err_line: Vec<u8>,
    /// The `Math.random` generator's state, 0 until first used.
    pub(crate) random: u64,
    pub(crate) reported: HashSet<Diagnostic>,
    pub(crate) wk: WellKnown,
    pub(crate) exit_code: Option<i32>,
    pub(crate) live_threads: usize,
}

/// The parts of a virtual machine every thread shares.
pub(crate) struct VmShared {
    pub(crate) state: Mutex<VmState>,
    pub(crate) cond: Condvar,
    pub(crate) stopping: AtomicBool,
    pub(crate) paused: AtomicBool,
    pub(crate) config: VmConfig,
    /// Global roots whose handles were dropped, cleared at the next
    /// collection.
    pub(crate) released_globals: Mutex<Vec<u32>>,
    pub(crate) this: Weak<VmShared>,
    pub(crate) id: u64,
}

impl VmShared {
    #[inline]
    pub(crate) fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Relaxed)
    }

    #[inline]
    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub(crate) fn arc(&self) -> Arc<VmShared> {
        self.this
            .upgrade()
            .expect("a running thread keeps its virtual machine alive")
    }

    /// Wakes every thread waiting on the VM condition variable. Taking
    /// the lock first makes sure a thread that checked its condition is
    /// already waiting, so the wakeup is not lost.
    pub(crate) fn wake_all(&self) {
        drop(self.state.lock());
        self.cond.notify_all();
    }

    pub(crate) fn log(&self, level: LogLevel, text: impl Into<String>) {
        (self.config.log)(LogLine {
            level,
            text: text.into(),
        });
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
/// reach it: a [`Vm`] there would be a cycle and leak the machine.
#[derive(Clone)]
pub struct WeakVm(Weak<VmShared>);

impl WeakVm {
    /// The machine, if anything still holds it.
    pub fn upgrade(&self) -> Option<Vm> {
        self.0.upgrade().map(|shared| Vm { shared })
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
        let mut library: HashMap<String, NativeClass> = lang::library()
            .into_iter()
            .map(|c| (c.name.clone(), c))
            .collect();
        for class in natives {
            library.insert(class.name.clone(), class);
        }
        let mut classes = ClassTable::new(source, library.into_values().collect());
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
            monitors: HashMap::new(),
            interned: HashMap::new(),
            weak: HashMap::new(),
            globals: Vec::new(),
            free_globals: Vec::new(),
            posted: VecDeque::new(),
            extensions: HashMap::new(),
            properties,
            out_line: Vec::new(),
            err_line: Vec::new(),
            random: 0,
            reported: HashSet::new(),
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
        let shared = Arc::new_cyclic(|this| VmShared {
            state: Mutex::new(state),
            cond: Condvar::new(),
            stopping: AtomicBool::new(false),
            paused: AtomicBool::new(false),
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
    pub fn spawn(
        &self,
        name: &str,
        body: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static,
    ) -> std::io::Result<()> {
        let mut state = self.shared.state.lock();
        thread::spawn_with_new_object(&self.shared, &mut state, name, Box::new(body))
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
    pub fn post(&self, task: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static) {
        self.shared.state.lock().posted.push_back(Box::new(task));
        self.shared.cond.notify_all();
    }

    /// Asks every thread to end: each unwinds at its next safepoint or
    /// wait, without running `finally` blocks. Returns at once; see
    /// [`Vm::wait_finished`].
    pub fn stop(&self) {
        self.shared.stopping.store(true, Ordering::Release);
        self.shared.wake_all();
    }

    /// Parks every thread at its next safepoint until [`Vm::resume`].
    /// Waits and sleeps that end while paused park too.
    pub fn pause(&self) {
        self.shared.paused.store(true, Ordering::Release);
        self.shared.wake_all();
    }

    /// Lets paused threads run again.
    pub fn resume(&self) {
        self.shared.paused.store(false, Ordering::Release);
        self.shared.wake_all();
    }

    /// Whether [`Vm::pause`] is in effect.
    pub fn is_paused(&self) -> bool {
        self.shared.is_paused()
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
    /// given. Returns whether they have.
    pub fn wait_finished(&self, timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|t| Instant::now() + t);
        let mut state = self.shared.state.lock();
        while state.live_threads > 0 {
            match deadline {
                Some(d) => {
                    if self.shared.cond.wait_until(&mut state, d).timed_out() {
                        return state.live_threads == 0;
                    }
                }
                None => self.shared.cond.wait(&mut state),
            }
        }
        true
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
    /// line break.
    pub fn flush_output(&self) {
        let mut state = self.shared.state.lock();
        lang::system::flush_lines(&self.shared, &mut state);
    }
}
