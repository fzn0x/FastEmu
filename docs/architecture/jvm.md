# The virtual machine

The `fastemu-jvm` crate is the Java virtual machine every FastEmu instance
runs its game in: class file parsing, linking, the bytecode interpreter,
the heap and its collector, Java threads, and the CLDC 1.1 class library
(`java.lang`, `java.io`, `java.util`, with MIDP's `Timer`) implemented in
Rust. It runs real J2ME games: class versions 45.3
(javac 1.1) to 52 (Java 8), ProGuard-obfuscated, preverified (StackMap
attributes are skipped), with `jsr` and `ret`.

This page is for the people building on it, the MIDP crate first. It
describes the design, the threading and garbage collection rules native
code must follow, and how to define a Java class in Rust.

## Shape

```text
  host thread (UI, launcher)              Java threads (OS threads)
        |  Vm::new, attach, spawn,             |  hold the VM lock while they run,
        |  post, pause, resume, stop           |  give it up at safepoints and waits
        v                                      v
  +-------------------------- VmShared ------------------------------+
  |  Mutex<VmState> (the VM lock)   Condvar   stop and pause flags   |
  |  VmState:                                                        |
  |    ClassTable  classes, statics, init states, Class objects      |
  |    Heap        slab of objects addressed by handles              |
  |    threads     each thread's entry and its parked stack          |
  |    monitors    interned strings  weak refs  global roots         |
  |    posted tasks  extensions  system properties  output buffers   |
  +------------------------------------------------------------------+
        ^
        |  every native method gets a Ctx: the locked state plus the
        |  calling thread's own stack (frames, slots, native locals)
```

- **`Vm`**: a cheap, cloneable handle to one virtual machine. Each
  FastEmu instance has its own, so instances run in parallel on different
  cores and share nothing.
- **`Ctx`**: a Java thread's view of its locked machine. Native methods
  receive `&mut Ctx` and do everything through it.
- **`NativeClass`**: a Java class defined in Rust. Bytecode classes and
  native classes share one class table, so a game class can extend
  `javax/microedition/lcdui/Canvas` (native) and override `paint`, and
  the native `Display` code reaches that override through ordinary
  virtual dispatch.
- **`ClassSource`**: where class files and resources come from. This
  crate has `DirSource` (a directory, as `javac -d` writes it) and
  `MemorySource` (tests); the MIDP crate provides the JAR source.

## Modules

| Module | What it holds |
| --- | --- |
| `classfile` | The class file parser and modified UTF-8. Public. |
| `descriptor` | Field and method descriptors. Public. |
| `class` | Runtime classes, fields and methods; the class table; linking (field layout, vtables, interface maps). |
| `interp` | Decoding bytecode to instructions, lazy constant pool resolution, the interpreter loop. |
| `heap`, `gc` | The object slab, payloads, the mark and sweep collector and its roots. |
| `thread`, `monitor`, `init` | Java threads, monitors and `wait`/`notify`, class initialization. |
| `ctx` | The `Ctx` API for natives. |
| `native` | `NativeClass`, the builder for classes defined in Rust. |
| `vm` | `Vm`, `VmConfig`, the shared state. |
| `limits` | `VmLimits`, the limits on what a game can make the VM use. |
| `lang` | `java.lang` in Rust, and the helpers the other library modules share. |
| `io` | `java.io` in Rust. |
| `util` | `java.util` in Rust, with MIDP's `Timer` and `TimerTask`. |
| `charset` | The character encodings every conversion between bytes and text uses. |

## Using a virtual machine from the host

```rust
use std::sync::Arc;
use fastemu_jvm::{Vm, VmConfig};

let config = VmConfig {
    heap_bytes: profile.heap_bytes as usize,
    properties: vec![
        ("microedition.platform".into(), profile.platform.clone()),
        ("microedition.encoding".into(), settings.encoding.clone()),
        ("microedition.locale".into(), settings.locale.clone()),
    ],
    log: Arc::new(move |line| log_queue.push(line)),
    diagnostics: Some(Arc::new(|d| eprintln!("{d}"))),
    allow_exit: false, // MIDP: System.exit throws SecurityException
    ..VmConfig::default()
};
let vm = Vm::new(config, Arc::new(jar_source), midp::classes(&profile));
vm.spawn("main", |ctx| {
    // load the MIDlet class, construct it, call startApp ...
    Ok(())
})?;
```

`VmConfig` fields:

- `heap_bytes`: what `Runtime.totalMemory()` reports and the limit past
  which allocation throws `OutOfMemoryError`. Objects are counted at
  about the size a phone's VM would give them.
- `max_frames` (4096): Java frames per thread before `StackOverflowError`.
  The slots the frames declare are limited as well (see
  [Limits](#limits)).
- `max_native_depth` (256): how many times native code may call back into
  Java on one thread, nested, before `StackOverflowError`. A native that
  calls another native (`Vector.toString` on a vector that holds a
  vector) counts as well as one that calls bytecode. Each level
  uses the OS thread's stack (about 40 KB in a debug build, a few KB in
  release), so this keeps a runaway recursion through natives from
  overflowing it.
- `thread_stack_bytes` (16 MiB): the OS stack of each Java thread.
- `properties`: system properties on top of `microedition.configuration`
  (`CLDC-1.1`), `microedition.encoding` (`UTF-8`), `microedition.platform`
  and `microedition.locale`. `microedition.encoding` is the charset of
  every conversion that names none (see the class library below); one
  FastEmu lacks means ISO-8859-1. No host property (`os.name`,
  `user.home` and the like) is set, so a game reads them as `null`.
- `log`: receives `System.out` and `System.err` line by line
  (`LogLevel::Out`, `LogLevel::Err`, uncaught exceptions included) and
  warnings (`LogLevel::Warn`, diagnostics among them). A line is at most
  `limits.max_log_line_bytes` (4096) bytes: `System.out` text without a
  line break comes out as lines of that size, and any other line is cut
  with a note of how many characters went. It is called with
  the VM lock held: push to a queue and return.
- `diagnostics`: receives each `Diagnostic` once per VM, the first
  `limits.max_reported` (256) different ones (see below).
- `allow_exit`: whether `System.exit` stops the VM (`Vm::exit_code` then
  holds the status) or throws `SecurityException`, as MIDP requires.
- `limits`: a `VmLimits`, the limits on what a game can make the VM use
  besides the heap (see [Limits](#limits)).

`Vm` methods:

| Method | What it does |
| --- | --- |
| `Vm::new(config, source, natives)` | Creates the machine. `natives` adds classes (a class named like a library class replaces it). |
| `spawn(name, body)` | Starts a Java thread whose body is a Rust closure. An exception it returns is printed as uncaught. |
| `run_main(class, args)` | Starts thread `main` running `static void main(String[])`, for tests and tools. |
| `attach(name, f)` | Runs `f` on the calling OS thread as a temporary Java thread, with the lock held. For setup and short work; the host stack is small, so it allows only 16 nested native-to-Java calls. |
| `post(task)` | Queues a task for a thread that drains posted tasks (MIDP's event thread). At most `limits.max_posted` wait; later ones are dropped. |
| `pause()`, `resume()` | Parks every thread at its next safepoint, and lets them go. The run time stands still meanwhile. |
| `stop()` | Every thread unwinds at its next safepoint or wait. `finally` blocks do not run. |
| `wait_finished(timeout)` | Waits until every Java thread has ended. |
| `clock()` | The machine's run time, a `VmClock` (see [Run time](#run-time)). |
| `thread_count`, `exit_code`, `heap_stats`, `flush_output` | Status. |
| `downgrade()` | A `WeakVm`, which does not keep the machine alive. |

`Vm` methods take the VM lock, so they are for host threads only. Native
code already holds the lock; a native that called `Vm::post` or
`Vm::stop` would deadlock. Natives use `Ctx::post` and `Ctx::stop_vm`.
A thread that never reaches a safepoint holds the lock for ever, so the
methods a host thread uses to stop the machine wait for it only so long:
`stop`, `pause` and `resume` 100 ms, after which they wake the waiting
threads without the lock; `flush_output` 250 ms, then it skips the
flush; `spawn` and `post` one second, then `spawn` fails with
`ErrorKind::TimedOut` and `post` drops its task and logs a warning (it
drops it too, with a warning the first time, when `max_posted` tasks wait
already);
`wait_finished` counts the wait for the lock into its timeout and
returns `false` when it runs out. `attach`, `thread_count`,
`exit_code` and `heap_stats` wait for as long as it takes.
`Vm::clock`, `Vm::is_stopping`, `Vm::is_paused`, `Vm::downgrade` and
`Ctx::clock` take no lock. A host thread that must not wait, such as an
app's UI thread, should not call the ones that take it: a Java thread
keeps the lock until its next safepoint or wait, and longer when its OS
thread is not scheduled. MIDP's instances post the app's input from a
control thread of their own (`docs/architecture/midp.md`).

## Values and references

`Value` is one JVM value: `Int` (also `boolean`, `byte`, `char`,
`short`), `Long`, `Float`, `Double`, `Ref(Option<ObjRef>)` (`None` is
`null`), `RetAddr` (for `jsr`) and `Void` (no value: the second slot of a
`long` or `double`, or a `void` result).

`ObjRef` is a handle to an object: a 32-bit index into the heap's slab,
never a pointer. It is `Copy` and `Send`. It is valid only while the
object is reachable from a root; the collector does not move objects, so
a rooted handle never changes.

Native methods receive their arguments one `Value` per parameter (a
`long` is one value, not two slots), `this` first for instance methods,
and return a `Value` (`Value::Void` for `void`). `Value::int()`,
`long()`, `float()`, `double()`, `obj()`, `boolean()` and `char()` read
them.

## Threads and the VM lock

Every Java thread is an OS thread. A VM has one mutex over its state (the
VM lock) and one wait queue, which works as a condition variable
(`src/wait`).

- A thread runs Java only while it holds the lock, so at most one thread
  of a VM runs at a time and natives never see another thread change the
  heap under them.
- The interpreter gives other threads a turn every 4096 backward
  branches or calls (a safepoint) with a fair handoff of the lock. An
  exception that a handler catches counts as 16 of them, so a loop of
  nothing but throws and handlers, which has no branch and no call,
  reaches a safepoint every 256 passes.
- Blocking operations wait on the wait queue, which releases the lock:
  `Thread.sleep`, `Object.wait`, `Thread.join`, a contended
  `monitorenter`, class initialization another thread is running, and
  natives that call `Ctx::wait_until`, `Ctx::sleep`, `Ctx::wait_posted`.
- Deadlines are precise to about a millisecond on every platform, so a
  game loop that sleeps 10 ms a frame gets its frame rate. Each waiting
  thread waits on a signal of its own: on Windows an event and a
  high-resolution waitable timer, since condition variables there time
  out on the 15.6 ms system tick (a 10 ms sleep took 16 ms); elsewhere a
  mutex and condition variable, whose timeouts are already precise.
  Nothing spins, and a notify, an interrupt or a stop still wakes a
  waiter at once.
- `Ctx::without_lock(f)` runs `f` with the lock released, for slow work
  that does not touch the VM: file and network I/O, image decoding.
- Wakeups are not precise: every change (a monitor released, a notify, a
  thread ended, a task posted) wakes every waiting thread, and each checks
  its own condition again.

Monitors have an owner, a recursion count, threads waiting to enter and
a wait set. `wait`, `notify` and `notifyAll` without owning the monitor
throw `IllegalMonitorStateException`; `Thread.interrupt` wakes `sleep`,
`wait` and `join` with `InterruptedException`.

**Safepoints** are where a thread checks the stop and pause flags, lets
other threads in, and runs a requested collection: backward branches and
calls (every 4096), exceptions caught by a handler (every 256),
allocation, the return of every native call, waits, and
`Ctx::yield_now`. A stopping VM makes each thread return
`Err(Throw::Stop)` from its next safepoint or wait; that unwinds every
frame without running handlers. A paused VM parks threads at safepoints;
waits and sleeps that end while it is paused park too.

Every call from native code into Java (`Ctx::call_virtual`,
`call_static`, `construct`) checks the stop flag first and returns
`Err(Throw::Stop)` when it is set, so a native that loops around such
calls, as `InputStream.skip` does around `read`, ends when the VM stops
even if the Java methods it calls hold no backward branch. A native that
never calls back into Java holds the lock until it returns, and a loop of
natives that calls Java and never lets go of the lock starves the VM's
other threads until it ends; natives that loop for long must check
`Ctx::is_stopping` themselves.

## The heap and the collector

The heap is a slab of objects addressed by handles. An object has its
class, its body (instance fields, one `Value` each, or an array's
elements: `boolean` and `byte` as `i8`, `char` as `u16`, `short`, `int`,
`long`, `float`, `double`, or references) and an optional **payload**: a
Rust value attached to the object for classes implemented in Rust (a
string's characters, a buffer's contents, a thread's state, later an
image's pixels).

The collector is a precise, non-moving mark and sweep. Its roots:

- every thread's frames: the locals and operand stacks of each frame,
  and the monitor a synchronized frame holds;
- every thread's **native locals**: each reference created or returned
  while a native frame is active, dropped when that native returns;
- static fields, interned strings and `Class` objects;
- global roots (`GlobalRef`), thread objects, and objects whose monitor
  is owned or waited on.

Payloads report the references they hold through `Payload::trace`.
`WeakReference` referents are not roots; a collection clears the ones
that died.

**Allocation never collects.** It counts the object against the limit
and asks for a collection when the heap fills up; the collection runs at
the next safepoint, where every reference the thread holds is in a root.
The interpreter's allocation instructions are safepoints: they collect
first when the heap is full and throw `OutOfMemoryError` if it still has
no room. `multianewarray` asks for the arrays of every level at once:
the outer array and each array below it, at the length and element size
of its level, so `new byte[1000000][0]` is refused on a small heap before
it builds the first of a million arrays. `System.gc()` and `Ctx::gc()`
collect at once.

**Natives follow a deferred rule**, because a native may hold references
the collector cannot see, so it cannot collect in the middle:

1. *Counted at once.* What a native makes counts as it is made: a string
   counts its characters however it is made, `Ctx::set_payload` counts
   the change in the object's size, and `Ctx::recount(obj)` counts a
   payload that changed size afterwards (a buffer that grew, a stack trace
   that was stored, a thread's name). Every heap object remembers the
   size it was counted at, so `recount` adds only the difference. Call it
   where the size changes, not on every call: `StringBuffer` calls it once
   per doubling. A payload that changes size and skips it is counted only
   at the next collection, and a loop that allocates nothing else never
   reaches one.
2. *Big requests are refused first.* `Ctx::check_alloc(bytes)` throws
   `OutOfMemoryError` when `bytes` is more than the whole heap, or when
   it and what the heap counts now come to more than twice the heap.
   `new_array`, `new_object_array`, `new_byte_array`, `new_string` and
   `new_string_utf16` call it. A native that builds a big host buffer
   (`String.concat`, a growing `StringBuffer`) calls it before it builds
   the buffer. The bound is twice the heap, not the heap, because the
   count includes garbage that waits for a collection and a native cannot
   collect.
3. *Judged after the native returns.* The invoke instruction collects
   when the heap asked for it, and if the live data still does not fit in
   the limit it throws `OutOfMemoryError` (message `Java heap space`)
   into the thread whose native grew the count, as on a phone. A native
   grew it when the heap counts more as it returns than it did when the
   interpreter called it, not counting what other threads added while
   this one gave up the lock. So a native that allocated nothing gets no
   error, even while another thread's data keeps the heap over its limit.
   What a native allocated before it called other natives from Rust
   still counts as its own. An exception that a native throws, or the
   interpreter makes (a bad index, a null receiver, a number that does
   not parse), skips that return, so the interpreter judges the heap the
   same way as it throws one and throws `OutOfMemoryError` in its place.
   Without that, a game could keep such exceptions and their stack
   traces in an array it made first, and nothing would check the heap.

A game that fills memory through natives alone (`s = s.concat(s)`,
appending to a `StringBuffer`, keeping exceptions, resource streams or
`toCharArray` results) therefore gets `OutOfMemoryError` after at most a
few times the heap limit, and the host never holds much more than that.
`Throwable` keeps at most 256 stack frames (16 bytes each) and a
`Thread` name is cut to 255 characters, so neither is a way around the
count. The `OutOfMemoryError` itself is made without a check, so it can
always be thrown.

### Limits

`VmConfig::limits` is a `VmLimits`: one struct for every limit on what a
game can make the VM use besides the heap. It is a plain public struct
with a `Default`, so a test lowers one with `VmLimits { max_threads: 8,
..VmLimits::default() }`. Each limit has its own change that enforces it;
the fields are only read while the VM runs.

| Field | Default | What it limits | Enforced by |
| --- | --- | --- | --- |
| `max_threads` | 64 | Java threads alive at once | #36 |
| `max_stack_slots` | 131072 | slots in one thread's stack | #28 |
| `max_class_bytes` | 128 MiB | class data of one machine | #35 |
| `max_log_line_bytes` | 4096 | bytes of one log line | #30 |
| `max_reported` | 256 | different diagnostics reported | #30 |
| `max_posted` | 10000 | posted tasks not yet run | #30 |

`max_stack_slots` limits how far a thread's stack reaches. A frame starts
at its arguments, which sit on top of its caller's operand stack, and
reserves the `max_locals` and `max_stack` that the class file declares,
and 2 more, whether the method uses them or not. Each slot is 16 bytes
and a thread keeps what it grew to until it ends, so the default of
131072 slots is at most 2 MiB a thread. That is 4096 frames that add 32
slots each, where the largest method in the test corpus declares about a
hundred in all. A call that would pass the limit throws
`StackOverflowError`, which a game can catch, whether the caller is
bytecode or a native that calls into Java. A frame that declares 65535
locals and 65535 stack slots needs 131072 slots, the whole limit, so it
fits only when nothing is below it on the thread's stack.

A game chooses the text of its output, its log lines and the names it asks
for, and none of that is on the Java heap, so these three bound what the
host keeps for it:

- **Output lines.** `System.out` and `System.err` keep the text since the
  last line break in a buffer of at most `max_log_line_bytes`. When it
  is full the buffer goes to the log as a line and starts again, so a
  game that prints without a line break in a loop makes 4 KiB lines and
  never a large buffer. A cut inside a multi byte character garbles that
  one boundary. `flush_output` and the end of a thread still send what is
  left.
- **Log lines.** `cut_log_text` cuts every line the machine writes to
  the log sink at a character boundary and ends it with ` ... (N more
  characters)`. The cut line fits the limit, so a host that cuts again
  (MIDP's `HostLink` does) changes nothing. It is public, for hosts that
  add lines of their own.
- **Diagnostics.** The names in a `Diagnostic` are cut to 255 characters.
  After `max_reported` different ones the machine reports no more, and
  logs once that it stops; the game still gets its errors. One that is
  reported already is not new.
- **Posted tasks.** `Vm::post` and `Ctx::post` drop a task when
  `max_posted` wait, and log one warning the first time. A native that
  keeps a queue of its own reads the limit with `Ctx::limits`: MIDP's
  `callSerially` throws `OutOfMemoryError` at the same number.

### Payloads and the heap

Every payload that owns data a game can make large either counts it
(`Payload::heap_bytes` and `Ctx::recount`, or `Ctx::check_alloc` before
building it) or is bounded by a named limit in another change. Payloads
that hold only references, flags and numbers are fixed in size and need
neither. A change that adds a payload adds a row.

| Payload | Owns | Counted or bounded by |
| --- | --- | --- |
| `JString` | the characters | `heap_bytes`, counted when the string is made; `new_string*` and `String.concat` check first |
| `StrBuf` (`StringBuffer`, `StringBuilder`) | the text | `heap_bytes` at the capacity; `edit` checks and recounts when the capacity changes |
| `ThrowableData` | the stack trace | `heap_bytes` (16 bytes a frame, at most 256); recounted when stored |
| `ThreadData` | the name | `heap_bytes`; name cut to 255 characters; recounted when set |
| `Table`, `Entry`, `Cursor`, `Elements` | references into heap arrays | fixed; the arrays are heap objects |
| `Queue`, `TimerData`, `Task` (`Timer`) | one slot per scheduled `TimerTask` | bounded by the task objects, which can be scheduled only once |
| `ReaderState`, `WriterState`, `PrintState` | decoder state, a pending character | fixed |
| `Mirror`, `Instant`, `Seed`, `Zone`, calendar `State` | numbers and short ids | fixed |
| `ImageData`, the buffer of `DisplayableData`, the surface of a `CustomItem`, `LayerData` (tiled layers, sprites) | pixels, tile grids, animated tiles, frame sequences | `check_alloc` before building and `recount` on change (#32); a displayable buffer and an item surface are made on the event thread, so they are counted and not refused |
| screen state of `DisplayableData` (`ListState`, `TextBoxState`), `ItemData` (`ChoiceGroup`, `TextField`) | the elements of a choice and the text of an editor | `heap_bytes` at the room of the vector (16 bytes an element, 2 a unit of text); recounted when elements or text change, and a vector that lost items gives room back; a constructor or edit that makes a text longer calls `check_alloc` first |
| `PlayerData` | the sound bytes kept until `realize` | `heap_bytes`; recounted when `realize` or `close` drops them; `check_alloc` before the copy; a stream read asks the heap per chunk |
| `PlayerData`, `SoundData`, `Clip` (decoded audio in the engine) | decoded samples, MP3 files, MIDI events | not on the heap: the instance's sound group allows 32 MiB and 128 players, 16 MiB for one sound ([audio.md](audio.md)) |
| `HttpPayload`, `SocketPayload`, `ServerPayload`, `UdpPayload`, `InputPayload`, `OutputPayload` | socket and body buffers | #38, with its own caps |
| `StoreHandle`, `EnumState` | listeners and the ids of an enumeration | `heap_bytes`; the size of a record store by #40 |
| `GraphicData` (Siemens sprites and backgrounds) | frames and tiles | `heap_bytes`; the constructors call `check_alloc` before cutting the pictures and recount |
| `UiImage`, `ExtendedData`, `LayerData` (Siemens) | pictures, tile grids and sprite sequences | bitmaps, copies, tile layers and sprite sequences: `check_alloc` before building, `recount` on change (#32) |
| `Composer`, `MelodyData` (Siemens) | notes and MIDI bytes | `heap_bytes`; a note added to a `Composer` is not recounted yet |
| `ManagerData` (`LayerManager`), `Manager` (Siemens layers) | a list of layer references | bounded: a layer is in the list once |
| `Manager` (Siemens graphic objects) | a list of object references | `heap_bytes` at the room of the list, recounted on each change; `addObject` keeps an object once, `insertObject` may repeat one and counts it |
| `CommandData`, `FontData`, `GraphicsData`, `AlertTypeData`, `TickerData`, `ControlData`, `Message`, `InfoPayload`, `ConnectionPayload` | references, flags, numbers; `GraphicsData` shares the surface of its target | fixed |

## Rules for native code

These keep the collector precise and the threads safe. The borrow checker
enforces some of them; the rest are on the native's author.

1. **References the `Ctx` hands out are rooted until the native returns.**
   Arguments are rooted (they stay on the caller's operand stack), and
   every `ObjRef` a `Ctx` method creates or returns (`new_string`,
   `new_object`, `construct`, `get_field`, `get_static`, `call_*`
   results, exceptions from calls, `current_thread`, `global`) is added
   to the thread's native locals.
2. **References read from array slices are not rooted.** `ctx.array::<
   Option<ObjRef>>(arr)` gives raw handles; pass any you keep across a
   call that can collect to `ctx.root(obj)`.
3. **A native that calls Java or waits must not hold unrooted references
   across the call.** A collection can run inside `call_virtual`,
   `call_static`, `call_special`, `construct`, `initialize`,
   `wait_until`, `sleep`, `wait_posted`, `without_lock`, `monitor_enter`,
   `monitor_wait`, `yield_now`, `run_posted` and `gc`, on this thread or,
   while the lock is released, on another one. Rooted references survive
   (objects never move); unrooted ones may name freed or reused slots.
4. **Never keep an `ObjRef` beyond the native call** in Rust state: not in
   a `static`, a closure, an extension, a host structure, a thread body or
   a posted task. Keep a `GlobalRef` (`ctx.new_global(obj)`), which is
   `Send` and `Sync` and frees its root when dropped (the next
   `new_global` uses its slot, no collection needed), or store the
   reference in a Java field or in a payload that traces it.
5. **Payloads trace what they hold.** A payload with `ObjRef`s implements
   `Payload::trace` and marks each one; otherwise the collector frees the
   objects.
6. **Long Rust loops bound their locals.** A thread body that loops
   forever (an event loop) wraps each iteration in
   `ctx.local_frame(|ctx| ...)`, which drops the locals made inside it.
   `run_posted` does this for each task.
7. **Do not block while holding the lock.** Wait with `ctx.wait_until`,
   `ctx.sleep` or `ctx.wait_posted`; do slow work in `ctx.without_lock`.
   A native that blocks the OS thread freezes every Java thread of its VM.
8. **Pass `Throw::Stop` on.** Use `?` on every `Ctx` call that returns a
   `JResult`; a long native loop checks `ctx.is_stopping()` and returns
   `Err(Throw::Stop)`.
9. **Use `Ctx`, not `Vm`, from natives** (`Ctx::post`, `Ctx::stop_vm`);
   `Vm` methods take the lock the native holds. State the VM owns (an
   extension, a native's closure, a payload) must not hold a `Vm` either:
   that is a cycle that keeps the machine alive after it stops. Hold a
   `WeakVm` (`vm.downgrade()`) and `upgrade()` it when needed.
10. **A panic is a bug.** It ends the thread, logs an internal error and
    stops the VM. Report Java-visible failures as exceptions instead:
    `return ctx.throw("java/lang/IllegalArgumentException", "why")`.
11. **Count what a game can make large.** A payload that holds data a
    game sizes reports it in `heap_bytes`, calls `ctx.recount(obj)` when
    it changes size, and calls `ctx.check_alloc(bytes)` before it builds a
    big host buffer. A native that makes the host hold memory the heap
    does not count lets a loop of such calls use up the machine.

## Defining a class in Rust

A `NativeClass` describes a class as a class file would. Register it by
passing it to `Vm::new`; it links on first use like any class.

```rust
use fastemu_jvm::{NativeClass, Payload, Value};

/// What a Rust-backed Image keeps beside its Java fields.
#[derive(Default)]
struct ImageData {
    width: i32,
    height: i32,
    pixels: Vec<u32>,
}

impl Payload for ImageData {
    fn heap_bytes(&self) -> usize {
        self.pixels.len() * 4
    }
}

fn image_class() -> NativeClass {
    NativeClass::new("javax/microedition/lcdui/Image")
        .payload::<ImageData>()                       // every instance gets one
        .static_method(
            "createImage",
            "(II)Ljavax/microedition/lcdui/Image;",
            |ctx, args| {
                let (w, h) = (args[0].int(), args[1].int());
                if w <= 0 || h <= 0 {
                    return ctx.throw("java/lang/IllegalArgumentException", "size");
                }
                let class = ctx.find_class("javax/microedition/lcdui/Image")?;
                let image = ctx.new_object(class)?;    // rooted until we return
                let data = ctx.payload_mut::<ImageData>(image)?;
                data.width = w;
                data.height = h;
                data.pixels = vec![0xFFFF_FFFF; (w * h) as usize];
                Ok(image.into())
            },
        )
        .method("getWidth", "()I", |ctx, args| {
            let this = ctx.non_null(args[0])?;
            Ok(Value::Int(ctx.payload::<ImageData>(this)?.width))
        })
}
```

The builder:

| Call | Adds |
| --- | --- |
| `NativeClass::new(name)` | A public class extending `java/lang/Object`. Names are internal (`a/b/C`). |
| `NativeClass::interface(name)` | A public interface. |
| `.extends(super)`, `.implements(iface)` | The superclass, an interface. |
| `.abstract_class()`, `.final_class()`, `.access(flags)` | Class flags (`classfile::access`). |
| `.field(name, desc)`, `.field_with(name, desc, access)` | An instance field (public by default). |
| `.static_field(name, desc)` | A public static field. |
| `.constant(name, desc, value)`, `.string_constant(name, text)` | A `public static final` with a constant value. |
| `.method(name, desc, f)`, `.static_method(...)`, `.method_with(name, desc, access, f)` | A method implemented by `f`. Add `access::SYNCHRONIZED` to hold the monitor of `this` (or the class) around it. |
| `.abstract_method(name, desc)`, `.abstract_method_with(name, desc, access)` | An abstract method for subclasses to implement: public, or with the given flags, such as `PROTECTED`. |
| `.static_init(f)` | The static initializer, run once on first active use. |
| `.payload::<T>()`, `.payload_with(f)` | A payload for every new instance of the class and its subclasses. |

Constructors are methods named `<init>` with a `V` return. A Java
subclass's constructor calls the native one through `invokespecial`, so
define every constructor the game may call (MIDP's `Canvas()` and friends).
`new` in bytecode allocates the object (fields at their defaults, payload
from the factory) before the constructor runs.

### Game classes extending native ones

Bytecode can extend a native class and override its methods. Native code
reaches the override through virtual dispatch:

```rust
NativeClass::new("javax/microedition/lcdui/Canvas")
    .extends("javax/microedition/lcdui/Displayable")
    .abstract_class()
    .method("<init>", "()V", |_, _| Ok(Value::Void))
    .abstract_method("paint", "(Ljavax/microedition/lcdui/Graphics;)V")
    .method("keyPressed", "(I)V", |_, _| Ok(Value::Void)) // games override it
```

and later, on the event thread:

```rust
ctx.call_virtual(canvas, "paint", "(Ljavax/microedition/lcdui/Graphics;)V", &[g.into()])?;
ctx.call_virtual(canvas, "keyPressed", "(I)V", &[Value::Int(code)])?;
```

`call_virtual` runs the game's override if it has one, the native body
otherwise; `call_interface` is the same call for a method of an
interface (a `Runnable`, a `CommandListener`). `call_special(class, obj,
name, desc, args)` calls one class's
method without dispatch (a `super` call), `call_static(class, name, desc,
args)` a static method (initializing the class first), and
`construct(class, desc, args)` makes an object and runs its constructor.
Calls return `Err(Throw::Exception(e))` when Java throws; the exception is
rooted, so the native can inspect it (`ctx.describe_exception(e)`,
`ctx.stack_trace(e)`), return it to rethrow, or drop it to swallow it.

When the Java original of a native calls another overridable method of
its own class (`Stack.pop` calls `size`, `peek` and `removeElementAt`),
the native makes the same call, so a game's override sees it, but keeps
its Rust code for the objects whose class does not override it.
`ctx.is_overridden(obj, class, name, desc)` says whether a virtual call
on `obj` would run anything other than what `class` has for that method
(it answers `true` when it cannot tell); `ctx.call_if_overridden(obj,
class, name, desc, args)` makes that call and returns `Some(result)` if
so, `None` for the native to go on in Rust:

```rust
if ctx.call_if_overridden(ht, "java/util/Hashtable", "rehash", "()V", &[])?.is_none() {
    rehash(ctx, ht)?; // nothing overrides it: the library's own code
}
```

### Fields, arrays and strings

- `ctx.field_id(class, name, desc)` resolves an instance field once
  (inherited fields too); `get_field` and `set_field` use the `FieldId`.
  `static_field_id`, `get_static` and `set_static` do the same for static
  fields and initialize the class first.
- `ctx.new_array(ArrayType::Int, n)`, `new_object_array(elem_class, n)`,
  `new_byte_array(&bytes)` allocate arrays; `array::<T>(arr)` and
  `array_mut::<T>(arr)` view them as slices (`i8`, `u16`, `i16`, `i32`,
  `i64`, `f32`, `f64`, `Option<ObjRef>`); `array_len` gives the length.
  A slice borrows the `Ctx`, so copy out what you need before calling
  anything else.
- `ctx.new_string(&str)`, `new_string_utf16(Vec<u16>)` and `intern`
  make strings; `string(obj)` reads one as Rust text, `string_utf16(obj)`
  borrows its UTF-16, `opt_string(value)` handles `null`.
- `ctx.payload::<T>(obj)`, `payload_mut`, `try_payload` and `set_payload`
  reach a payload.
- `ctx.check_alloc(bytes)` throws `OutOfMemoryError` for a request the
  heap could not hold, and `ctx.recount(obj)` counts a payload that
  changed size. The arrays and strings above call the first themselves;
  see [The heap and the collector](#the-heap-and-the-collector) for when
  a native calls either.

### Exceptions

`ctx.new_exception(class, message)` makes a `Throwable` of a class
defined in Rust, with this thread's stack trace, and returns it as a
`Throw`; `ctx.throw(class, message)` returns it as `Err` directly. Every
`java.lang` exception and error CLDC 1.1 lists exists, plus
`IllegalStateException`, `UnsupportedOperationException`, the linkage
errors and `java.io.IOException` with its CLDC subclasses. Natives in
other crates register their own exception classes as subclasses
(`NativeClass::new("javax/microedition/media/MediaException")
.extends("java/lang/Exception")` with `<init>()V` and
`<init>(Ljava/lang/String;)V` calling `call_special` on the superclass).

### Threads, waiting and posted work

MIDP's event thread and `Timer` threads are Java threads with Rust
bodies:

```rust
let thread = ctx.spawn_thread("event", |ctx| loop {
    // Sleep until the host posts work, then run it, one local frame each.
    ctx.wait_posted(None)?;
    ctx.run_posted()?;
})?;
```

The host thread posts with `vm.post(move |ctx| { ... Ok(()) })`; inside a
native, use `ctx.post`. Tasks run in order. A task that throws is printed
as uncaught and the next one runs. Tasks and thread bodies must not
capture `ObjRef`s; give them `GlobalRef`s. A post is dropped when
`max_posted` tasks wait. A native that posts once for each call a game
makes must be able to lose some when a game floods it.

`ctx.wait_until(deadline)` releases the lock and returns a `Wake`
(`Notified`, `TimedOut`, `Interrupted`, `Paused`) or `Err(Throw::Stop)`.
`ctx.notify_waiters()` wakes every waiter. `ctx.sleep(duration)` is
`Thread.sleep`. `ctx.monitor_enter`, `monitor_exit`, `monitor_wait` and
`monitor_notify` give natives Java's monitors. Deadlines are host time
(`Instant`): a sleep or wait that ends during a pause parks until the
resume and then returns.

### Run time

`ctx.clock()` (or `vm.clock()` on a host thread) is the machine's run
time, a `VmClock`: how long the machine has run since `Vm::new`, not
counting the time it spent paused. It never goes back and stands still
during a pause. Timing the library keeps for the game uses it, so a pause
is invisible to it: `Timer` schedules on it, so a fixed-rate timer paused
for a minute does not run a minute of late tasks when the machine
resumes. A media player that keeps its own time (one with no sound
output) should count its media time on it too. `System.currentTimeMillis`
stays the wall clock, which games show and compare with dates.

```rust
let clock = ctx.clock();               // cheap to clone; Send and Sync
let due = clock.elapsed() + Duration::from_millis(500);
while clock.elapsed() < due {
    // instant_at gives the host deadline the run time reaches `due` at
    // if no pause comes first; a pause wakes the wait with Wake::Paused,
    // and yield_now, a safepoint, parks until the resume.
    match ctx.wait_until(clock.instant_at(due))? {
        Wake::Paused => ctx.yield_now()?,
        // The flag stays set, so a wait would return at once and the
        // loop would spin with the lock held: stop waiting, or call
        // take_interrupt() and go on, as Timer's thread does.
        Wake::Interrupted => break,
        Wake::Notified | Wake::TimedOut => {}
    }
}
```

`VmClock` holds no reference to the machine, so a payload, an extension
or a host thread may keep one. `clock.elapsed()` is a `Duration`;
`clock.instant_at(t)` is `None` for a time too far ahead for an
`Instant` (wait without a deadline then), and `clock.is_paused()` says
whether the clock stands still.

### Host data

`ctx.set_extension(value)`, `extension::<T>()` and `extension_mut`
keep one value per type in the VM, for state the natives share (the
current `Displayable` as a `GlobalRef`, the frame buffer, input queues).
Extensions must not hold `ObjRef`s.

## Class loading and initialization

A class name resolves, in order, to an already loaded class, an array
class (made on demand), a native class definition, or a class file from
the source. Superclasses and interfaces load first. Fields are laid out
after the inherited ones; static fields live in the class table. The
vtable copies the superclass's and replaces overridden entries (a
package-private method is overridden only from its own package);
interface methods a class does not implement get an abstract entry, so
calling one throws `AbstractMethodError`.

Methods decode on their first call into an instruction array with
resolved branch targets; constants the pool can give are inlined.
Decoding also checks the operand stack, as a small part of a verifier:
every reachable instruction must have one stack height however it is
reached, stay within `max_stack`, never pop an empty stack, never run
past the end of the code, and name constant pool entries of the right
kind. A return must also leave as many slots as the method's descriptor
says: none for `void`, two for `long` and `double`, one for the rest.
Code that fails throws `VerifyError` on its first call; javac and
ProGuard output always passes. Code that no path reaches is not
checked, as phones accept the dead code that obfuscators leave. An
exception handler counts as reached, whatever its range holds. There is
no type verifier beyond that; see "What the interpreter runs".
Constant pool references resolve lazily, on first execution, into
per-class caches; a failed resolution is not cached and throws again.
`invokeinterface` caches the last receiver class per constant pool
entry.

Initialization follows JVMS 5.5: on first active use (`new`, a static
field, a static call, `Class.forName`, a subclass's initialization), the
superclass first, constant fields set, then `<clinit>` once. A recursive
request from the initializing thread returns at once; other threads wait.
An exception from `<clinit>` that is not an `Error` is wrapped in
`ExceptionInInitializerError`; later uses throw `NoClassDefFoundError`.

**Limits.** Names and hierarchies come from games and from files, and
loading recurses once per class of a hierarchy. So they have limits, set
in `class/table.rs`, as a real VM has. A class name over 255 bytes throws
`NoClassDefFoundError`. An array of over 255 dimensions and a hierarchy
of over 256 classes (`Object` and the class itself included) throw
`ClassFormatError`. Real games stay far below all three. The longest
class name of the test games has 57 bytes, and their chains of game
classes have 5 classes at most.

- Array classes are made in a loop, from the innermost dimension out.
  The stack a name needs does not grow with its length.
- A hierarchy is bounded twice, because a chain loads in two ways. A class
  loaded before its superclass or interfaces nests one load per class, and
  the load stops at 256 nested classes. A class loaded after its whole
  chain nests nothing, so `define` checks the height of the class: the
  most classes on any path up to `Object` through superclasses and
  interfaces. That bounds initialization, which recurses once per
  superclass, and the flat list of all interfaces of a class, which grows
  with a chain of interfaces. Loading and initialization are depth
  bounded for every load order.
- Field resolution searches each class once, in the order of JVMS
  5.4.3.2, so it takes time linear in the size of the interface graph.
  A graph of layers that each extend the two before is no harder than a
  chain.
- A name a game chose is cut to 100 characters in an error message.
  `Class.forName` refuses a name over the limit, counted in characters
  and in bytes, with `ClassNotFoundException`, and does not copy a string
  that is too long.

**Class memory.** A loaded class stays for as long as its machine runs:
classes are never unloaded. None of it is on the Java heap, so a game
could fill the host with classes the heap meter never sees: a pool of
65535 constants is 327 KB of file, 600 bytes in a JAR and 6.3 MiB of
tables, and each subclass copies the method table of its superclass.
`VmLimits::max_class_bytes` (128 MiB) bounds the class data of one
machine. A class that would take the machine past it does not load, and
the game gets `OutOfMemoryError` with the message `class memory limit
reached`, which it can catch. The classes that loaded stay, and a smaller
class can still load.

The classes a game can make count: those read from class files, and the
array classes of any name it asks for (an array of 255 dimensions of every
class is 255 classes). The classes of the library do not, and neither do
the eight arrays of primitives, which `new int[n]` needs. The size of a
class is an estimate (`class/budget.rs`), worked out from its definition
before anything of it is built, so a class that does not fit costs little
to refuse and leaves nothing behind. These are the parts, on a 64 bit
machine:

| Part | Bytes |
| --- | --- |
| a class | 512 and the length of its name |
| a constant pool entry | 104 (the entry and the slot that caches what it resolves to), and the text of a `Utf8` entry with its 16 byte header |
| a field | 64, and 16 for its static value or its slot in the instance template |
| the instance slots a subclass copies | 16 each, inherited ones included |
| a string constant of a static field | 16 and 2 per character, once however many fields name it |
| a method | 216 and the lengths of its name and descriptor |
| a method with code, besides | 184, 21 per byte of code, 24 per exception handler, 4 per line number |
| an entry of a method table | 32: the superclass's entries, a slot for each method a subclass could override, and the methods of interfaces the superclass does not implement |
| a superclass or interface of the class | 4 each, so a chain of interfaces loaded from its root takes memory by the square of its length |
| an array class | 512 and the length of its name, and 32 per entry of the method table of `Object` |

A byte of code is counted at 21 because the decoded instruction takes 16
bytes and its offset 4, and the shortest instruction is one byte. Real
code has two or three bytes to an instruction, so the model counts about
twice what real code takes. The code of every method is counted when the
class links, whether the method ever runs, so the interpreter needs no
hook.

The estimate is not exact. Methods that all have names of their own fill
the table of selectors fastest, and a set of classes of 60000 such
methods each held about a fifth more than the estimate says.

By this model the JARs of the test game folder cost 3 KB to 29 MiB. The
largest game (327 classes) costs 7.3 MiB. The 29 MiB is a desktop library
of 1046 classes that is no game, and the largest single class, with
thousands of fields, costs 2.8 MiB. The limit is far above any game and
stops a hostile set of classes: a class with 65535 constants takes 6.5
MiB, a method of 64 KiB of code 1.3 MiB, and a method table of 30000
methods 960 KB for each subclass, so about 20, 100 and 130 of them
load.

What the loader builds before it knows the size stays within a small
multiple of the class file and of the classes already loaded, so a class
that is then refused costs little. A method keeps two numbers of its descriptor and not its parse, since a descriptor can name
tens of thousands of parameters and every method can share it. A text
constant is copied to UTF-16 once for each text of the pool, however many
string entries and fields name it. An interface a class names more than
once is added once. The file itself, which the zip reader caps at 64 MiB
for an entry, is parsed before any of this, and a refused class holds a
few times its size while it is.

**Diagnostics.** Something a game needs that FastEmu lacks throws the
usual error and is reported once to the diagnostics hook and the log, by
exact internal name, so it says what to implement next:

```text
missing class com/nokia/mid/ui/FullCanvas
missing method javax/microedition/lcdui/Graphics.drawFoo(II)V
missing field javax/microedition/lcdui/Canvas.FOO:I
```

A catch clause naming a missing class catches nothing (no thrown object
can be of a class that does not exist) and reports it.

## What the interpreter runs

Every instruction of class versions 45.3 to 52, including `wide`,
`tableswitch`, `lookupswitch`, `multianewarray`, `jsr`, `ret`, monitors,
and `checkcast`/`instanceof` for arrays and interfaces. Arithmetic is
Java's exactly: integer division by zero throws `ArithmeticException`,
`MIN_VALUE / -1` wraps, shift counts are masked, float-to-int
conversions saturate with NaN to 0, `fcmpl`/`fcmpg` follow the NaN rules,
and float math is IEEE single and double. `invokedynamic` and method
handle constants (lambdas) throw `IncompatibleClassChangeError`; J2ME
code has none. Exception messages follow HotSpot's where it matters
(`/ by zero`, `Index 5 out of bounds for length 3`), and stack traces
print as HotSpot prints them, with line numbers from `LineNumberTable`.

A thread's frames may hold 131072 slots in all (see [Limits](#limits)),
so a recursion of large frames ends in `StackOverflowError` long before
`max_frames`. A handler that catches an exception counts toward the
safepoint budget (see [Threads and the VM lock](#threads-and-the-vm-lock)).

Speed, from the benchmark (`cargo test -p fastemu-jvm --release --test
bench -- --ignored --nocapture`): about 590 million bytecodes per second
on a tight arithmetic loop and 190 million with a static call in every
iteration, on the development machine. Phones ran 1 to 20 million.

What a crafted class can do is limited, as there is no type verifier.
Decoding checks what the interpreter counts on. A reachable return that
leaves another number of slots than the descriptor says throws
`VerifyError` at the first call. A return gives the caller the slots the
descriptor declares, whatever came back, so the caller's operand stack
never drifts from what the stack walk counted. `invokevirtual` runs the
entry of the receiver's vtable only when its selector (the name and the
descriptor) is the one the call names. A receiver that does not have the
method throws `IncompatibleClassChangeError`. A call takes its arguments
off the stack with a checked subtraction.

`athrow` throws `VerifyError` when its object is not a `Throwable`, so
a handler and the report of an uncaught exception always get one. Values
are not typed otherwise. Bytecode can hand a native an object or an
array of another class than it reads, and hundreds of natives read their
arguments with accessors that panic on the wrong type, so this stays a
known limit: a crafted class can stop its own game with an internal
error. The panic ends the thread that ran it, logs `internal error in
the virtual machine` and stops that game, which the host shows as
crashed. No other game is affected. The report of an uncaught exception
has its own guard, so a thread whose exception has a plain `Object` as
its cause still ends, releases its monitors and wakes the threads that
join it.

## The class library

Every class, constant, constructor and method that
`docs/compat/api/cldc-java-lang.md`, `cldc-java-util.md` and
`cldc-java-io.md` list exists, with the listed descriptor and flags; a
unit test (`src/inventory.rs`) checks all 665 members against those files.
The behaviour follows the JDK wherever games can see it, and the
differential tests compare it with the JDK.

- **`java.lang`** (`src/lang`): `Object`, `Class` (`getName`, `forName`,
  `newInstance`, `isInstance`, `isAssignableFrom`, `isArray`,
  `isInterface`, `getResourceAsStream`), `String`, `StringBuffer` and
  `StringBuilder`, `Math`, `Integer`, `Long`, `Short`, `Byte`, `Boolean`,
  `Character`, `Float`, `Double` (parsing as Java does, and printing with
  the `FloatingDecimal` algorithm of Java before 19, which phones ran:
  `1e23` prints as `9.999999999999999E22`, where Java 19 and later print
  `1.0E23`), `Number`, `System`, `Runtime`, `Thread`, `Runnable`,
  `Throwable` and the exceptions, `ref.Reference` and `ref.WeakReference`.
  A few Java SE members that `javac --release 8` output or common code
  uses are there too (`StringBuilder`, `CharSequence`,
  `Integer.valueOf(int)` for autoboxing, `Math.pow`). `Thread` has no
  `stop`, `suspend` or `resume`, so a game's `Thread` subclass may declare
  its own.
- **`java.io`** (`src/io`): `InputStream`, `OutputStream`,
  `ByteArrayInputStream`, `ByteArrayOutputStream`, `DataInput`,
  `DataOutput`, `DataInputStream`, `DataOutputStream` (big-endian
  primitives, modified UTF-8 with the JDK's `UTFDataFormatException`
  messages), `Reader`, `Writer`, `InputStreamReader`,
  `OutputStreamWriter` and `PrintStream`.
- **`java.util`** (`src/util`): `Vector`, `Stack`, `Enumeration`,
  `Hashtable`, `Random`, `Date`, `TimeZone`, `Calendar`, and MIDP's
  `Timer` and `TimerTask`.

What natives and games can rely on:

- **Subclasses work.** Classes keep their protected fields where Java
  does (`Vector.elementData`, `ByteArrayOutputStream.buf`,
  `DataInputStream.in`, `Calendar.fields`), and every method reads them
  afresh. Wrappers (the data streams, readers, writers, `PrintStream`)
  call the wrapped stream's methods through virtual dispatch, and base
  classes call their own overridable methods as the JDK does
  (`InputStream.skip` reads through `read(byte[], int, int)`,
  `Random.nextInt` through `next(int)`, `PrintStream.print` through
  `write(byte[], int, int)` and `println(x)` through `print(x)` and then
  a write of its own for the line break, `Stack.push`, `pop`, `peek`,
  `empty` and `search` through `addElement`, `size`, `peek`, `elementAt`,
  `removeElementAt` and `lastIndexOf`, `Vector.removeElement` through
  `indexOf` and `removeElementAt`, `contains` and `indexOf(Object)`
  through `indexOf(Object, int)`, `Hashtable.put` through `rehash`), so a
  game's own stream, generator, list, table or calendar sees the same
  calls as on Java. An object of exactly the library's own class (a plain
  `ByteArrayInputStream` under a `DataInputStream`, a plain `Random`), or
  of a subclass that does not override the method called, takes a Rust
  fast path instead; `src/io/access.rs` is the byte-level access the
  wrappers share.
- **Collections.** The methods Java declares `synchronized` on `Vector`,
  `Stack` and `Hashtable` hold the object's monitor. `Hashtable` is the
  JDK's design bucket for bucket (11 buckets, chains with new entries
  first, rehash to `2n + 1` past a load of 0.75, enumeration from the last
  bucket down), so keys come out in Java's order and removing while
  enumerating behaves as there. Keys and elements are hashed and compared
  through their own `hashCode` and `equals`; strings, boxed numbers and
  objects that keep `Object`'s methods are hashed and compared in Rust
  (`src/util/keys.rs`).
- **Encodings** (`src/charset`), under the names, aliases and historical
  names Java accepts (`SJIS`, `EUC_JP`, `UnicodeBig`, `ISO8859_2`,
  `Cp1251`, ...), ignoring case, `-`, `_` and `:`:
  - UTF-8, ISO-8859-1, US-ASCII, windows-1250, windows-1251,
    windows-1252, KOI8-R, the ISO-8859 parts Java has (2 to 9, 13, 15
    and 16; the JDK has no 10, 11 or 14 under those names) and UTF-16
    (`UTF-16` with a byte order mark, `UTF-16BE`, `UTF-16LE`,
    `x-UTF-16LE-BOM`) convert exactly as the JDK's charset does:
    unassigned bytes decode to U+FFFD, what a charset cannot encode
    becomes `?` (U+FFFD in UTF-16), and malformed UTF-8 and UTF-16 become
    as many U+FFFD as in Java (a surrogate in three bytes, or a sequence
    cut off at the end, is one). The ISO-8859 tables after part 2 come
    from encoding_rs, except part 9, which the WHATWG standard reads as
    windows-1254.
  - The multi-byte charsets of East Asia go through encoding_rs, whose
    WHATWG tables are the Microsoft code pages: `Shift_JIS` and
    `windows-31j` (code page 932), `EUC-JP`, `ISO-2022-JP`, `GBK` and
    `GB2312` (code page 936), `GB18030`, `Big5` (with the Hong Kong
    additions) and `EUC-KR` (code page 949). Text of the national
    standards converts as in Java, which the `Charsets` differential
    program checks; the vendor rows a strict Java charset rejects (NEC's
    circled numbers in `Shift_JIS`, the extra hangul of code page 949 in
    `EUC-KR`) decode instead of becoming U+FFFD, malformed bytes give the
    standard's count of U+FFFD, `GBK` encodes the euro sign as 0x80, and
    `ISO-2022-JP` writes halfwidth katakana as fullwidth ones, where Java
    switches to JIS X 0201. What a charset cannot encode becomes `?`.

  Every conversion that names no charset uses `microedition.encoding`:
  `new String(byte[])`, `getBytes()`, `InputStreamReader`,
  `OutputStreamWriter`, `PrintStream`, `ByteArrayOutputStream.toString()`
  and `System.out` itself. A name that is not one of these throws
  `UnsupportedEncodingException` where the API declares it.
- **Readers and writers** behave as on CLDC phones rather than as Java
  SE's buffered ones: `InputStreamReader` takes bytes from its stream
  only as it needs them for the characters asked for (and passes `mark`
  and `reset` to it), and `OutputStreamWriter` writes through at once.
  Each keeps its charset's state from one call to the next, as Java's
  do: a character split between reads waits for its last byte, `UTF-16`
  reads and writes its byte order mark once, and `ISO-2022-JP` keeps its
  shift state until the writer closes.
- **`PrintStream`** never throws `IOException`: a failure of the stream
  under it sets the flag `checkError` returns. `println(x)` holds the
  stream's monitor across its text and its line break, as Java's does,
  so lines two threads print never mix.
- **Time.** `System.currentTimeMillis`, `Date`, `Random()` and `Calendar`
  read the wall clock through `lang::system::current_time_millis`;
  `Timer` schedules on the run time (see [Run time](#run-time)).
  `Random` is the javadoc's 48-bit generator, so seeded sequences match
  every phone. `Calendar` behaves as Java's lenient `GregorianCalendar`
  (proleptic: no Julian dates before 1582), with weeks from Sunday as in
  the US locale. `TimeZone` has GMT,
  UTC and custom `GMT+hh:mm` zones, gives GMT for an ID it does not know,
  and defaults to the host's zone, with the host's daylight saving rules
  (from the jiff crate), under the `GMT+hh:mm` ID of its standard offset
  so the host's zone name never reaches the game.
- **`Timer`** runs its tasks on a Java thread of its own, `Timer-n`, with
  a Rust body. A task that throws is reported as uncaught and cancelled,
  and the timer goes on with its other tasks, as J2ME Loader does; on
  Java SE the whole timer would end. Tasks are due on the run time, so a
  pause neither runs them nor leaves late runs to catch up on: a
  fixed-rate task goes on at its rate after the resume. A `Date` passed
  to `schedule` counts from the wall time now, and
  `scheduledExecutionTime` is wall time (the wall time the machine
  started at, plus the run time, plus the pauses so far), so the
  javadoc's check `currentTimeMillis() - scheduledExecutionTime()` stays
  small after a pause.

## Testing

- Unit tests sit next to the code.
- `tests/differential.rs` compiles the programs in `tests/java/` with
  `javac --release 8`, runs each on the installed JDK and on FastEmu, and
  compares their output. Without a JDK the tests pass with a note,
  unless `FASTEMU_REQUIRE_JAVA=1` is set. The class library has one
  program per area (`Vectors`, `Hashtables`, `RandomSequences`,
  `CalendarFields`, `Timers`, `DataStreams`, `Readers`, `PrintStreams`,
  `Charsets`), each with game subclasses of the library classes. Java
  source files are UTF-8.
- `src/inventory.rs` checks the class library against the API inventory
  in `docs/compat/api`, and `tests/encoding.rs` that the encoding setting
  reaches every implicit conversion.
- `tests/old_bytecode.rs` runs `jsr`/`ret` and a version 45.3 class built
  with the small assembler in `tests/support`.
- `tests/bad_bytecode.rs` runs methods a damaged or crafted class could
  hold: bad stack heights and constant pool operands, returns of the
  wrong width (called a thousand times, so a leak shows), virtual calls
  on an object of another class, and a thread that throws a plain
  `Object`.
- `tests/class_limits.rs` loads deep array names, long chains of classes
  and of interfaces in both load orders and a 40 layer interface diamond,
  each under a deadline, on a VM thread.
- `tests/native_api.rs` covers this page's API: native classes extended
  by bytecode, threads, posted tasks, global roots, waiting, pause,
  resume and stop, diagnostics.
- `tests/timing.rs` measures how late sleeps and timed waits wake (run
  it with `--nocapture`) and how quickly an interrupt or a stop ends
  them.
- `tests/heap_limits.rs` runs the programs in `tests/heap_java/` on a
  256 KiB heap. Each fills memory through natives (`String.concat`,
  `StringBuffer.append`, kept exceptions, including ones a native or
  the interpreter throws, resource streams, arrays, interned strings,
  thread names) and must end in `OutOfMemoryError`
  within a few times the heap, in the thread that ran the native. Three
  cases with two threads keep data over the limit and check that a
  thread that allocates nothing, or only sleeps and wakes, never gets
  the error.
- `tests/output_limits.rs` runs the programs in `tests/output_java/`
  and checks output without a line break, long log lines, the diagnostic
  limit and its name cut, and the posted task limit, from a host thread
  and from a native.
- `tests/bench.rs` is the benchmark: a plain loop, static calls and
  `StringBuffer.append`.
