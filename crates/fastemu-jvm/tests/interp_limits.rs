//! What a game's bytecode can make the interpreter do without bound: a
//! handler that throws again forms a loop with no branch and no call, a
//! frame declares as many slots as it likes, and `multianewarray` builds
//! many arrays at once.
//!
//! The classes are assembled by hand, because each needs code that no
//! compiler writes: a loop that is only exception transfers, a method of
//! 4,000 slots that uses one.

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use fastemu_jvm::{HeapStats, NativeClass, Throw, Value, Vm, VmConfig, VmLimits};
use support::{ClassBuilder, STATIC, idx};

/// How long a test waits for something that takes milliseconds: far longer
/// than that, so that a busy machine waits instead of failing.
const LONG: Duration = Duration::from_secs(10);

/// A small heap: the smallest the heap allows is 64 KiB.
const HEAP: usize = 256 * 1024;

/// `class Loops`: static methods, each of which tells `Probe.entered()`
/// that it is about to start its loop, which has no backward branch and
/// no call, so the only way out of it is a safepoint of the exception
/// transfers.
///
/// - `spin()`: `aconst_null; athrow; athrow`, caught by a handler over
///   both throws that targets the second: the same exception, for ever.
/// - `fresh()`: a handler that drops the exception and throws a null:
///   a new `NullPointerException` on every pass.
/// - `hoard()`: a handler that keeps every pass's array in a chain, until
///   the heap is full, and then goes on throwing `OutOfMemoryError`.
/// - `keep()`: like `spin()`, but its handler catches only
///   `NullPointerException`, so a thread that lost the exception to a
///   collection, or found another object in its place, ends.
/// - `churn()`: a loop of `new Object[10]` that fills the heap with garbage
///   for ever. It is no throw and catch loop, and it does not tell the probe.
/// - `count(n)`: a loop that throws and catches `n` times and returns `n`.
fn loops() -> Vec<u8> {
    let mut c = ClassBuilder::new("Loops", "java/lang/Object", 49, 0);
    let entered = c.method_ref("demo/Probe", "entered", "()V");
    let object = c.class("java/lang/Object");
    let null_pointer = c.class("java/lang/NullPointerException");
    let [eh, el] = idx(entered);
    let [oh, ol] = idx(object);
    #[rustfmt::skip]
    let spin = [
        0xb8, eh, el,       // 0: invokestatic Probe.entered
        0x01,               // 3: aconst_null
        0xbf,               // 4: athrow
        0xbf,               // 5: athrow (the handler)
    ];
    #[rustfmt::skip]
    let churn = [
        0x10, 0x0a,         // 0: bipush 10
        0xbd, oh, ol,       // 2: anewarray Object
        0x57,               // 5: pop
        0xa7, 0xff, 0xfa,   // 6: goto 0
    ];
    #[rustfmt::skip]
    let fresh = [
        0xb8, eh, el,       // 0: invokestatic Probe.entered
        0x01, 0xbf,         // 3: aconst_null athrow
        0x57,               // 5: pop (the handler)
        0x01, 0xbf,         // 6: aconst_null athrow
    ];
    #[rustfmt::skip]
    let hoard = [
        0xb8, eh, el,       // 0: invokestatic Probe.entered
        0x01, 0x4b,         // 3: aconst_null astore_0
        0x01, 0xbf,         // 5: aconst_null athrow
        0x57,               // 7: pop (the handler)
        0x05,               // 8: iconst_2
        0xbd, oh, ol,       // 9: anewarray Object
        0x59, 0x03, 0x2a,   // 12: dup iconst_0 aload_0
        0x53, 0x4b,         // 15: aastore astore_0
        0x01, 0xbf,         // 17: aconst_null athrow
    ];
    #[rustfmt::skip]
    let count = [
        0x03, 0x3c,                 // 0: iconst_0 istore_1
        0x1b, 0x1a,                 // 2: iload_1 iload_0
        0xa2, 0x00, 0x0c,           // 4: if_icmpge 16
        0x01, 0xbf,                 // 7: aconst_null athrow
        0x4d,                       // 9: astore_2 (the handler)
        0x84, 0x01, 0x01,           // 10: iinc 1 1
        0xa7, 0xff, 0xf5,           // 13: goto 2
        0x1b, 0xac,                 // 16: iload_1 ireturn
    ];
    c.method(STATIC, "spin", "()V", 1, 0, &spin, &[(3, 6, 5, 0)]);
    c.method(
        STATIC,
        "keep",
        "()V",
        1,
        0,
        &spin,
        &[(3, 6, 5, null_pointer)],
    );
    c.method(STATIC, "churn", "()V", 1, 0, &churn, &[]);
    c.method(STATIC, "fresh", "()V", 1, 0, &fresh, &[(3, 8, 5, 0)]);
    c.method(STATIC, "hoard", "()V", 4, 1, &hoard, &[(5, 19, 7, 0)]);
    c.method(STATIC, "count", "(I)I", 2, 3, &count, &[(7, 9, 9, 0)]);
    c.build()
}

/// The recursion: `n == 0 ? 0 : callee(n - 1) + 1`.
#[rustfmt::skip]
fn recursion(callee: [u8; 2]) -> [u8; 15] {
    [
        0x1a,                       // 0: iload_0
        0x9a, 0x00, 0x05,           // 1: ifne 6
        0x03, 0xac,                 // 4: iconst_0 ireturn
        0x1a, 0x04, 0x64,           // 6: iload_0 iconst_1 isub
        0xb8, callee[0], callee[1], // 9: invokestatic callee
        0x04, 0x60, 0xac,           // 12: iconst_1 iadd ireturn
    ]
}

/// `try { return callee(n); } catch (StackOverflowError e) { return -1; }`
#[rustfmt::skip]
fn probe_overflow(callee: [u8; 2]) -> [u8; 8] {
    [
        0x1a,                       // 0: iload_0
        0xb8, callee[0], callee[1], // 1: invokestatic callee
        0xac,                       // 4: ireturn
        0x4c, 0x02, 0xac,           // 5: astore_1 iconst_m1 ireturn
    ]
}

/// `class Deep`: recursions with frames of different sizes, and the
/// multi-dimensional arrays.
///
/// - `big(n)` and `small(n)`: `n` calls of themselves that return `n`.
///   A frame of `big` declares 2,000 locals and 2,000 stack slots, one of
///   `small` the 1 local and 3 slots it needs.
/// - `probeBig(n)` and `probeSmall(n)`: the same, `-1` when a
///   `StackOverflowError` came out.
/// - `wide(JJ)V`: takes four argument slots and returns.
/// - `huge()`: `new byte[30000][0]`, or `1` when an `OutOfMemoryError`
///   came out and `0` when it was built.
/// - `shape()`: `new int[3][4][]`, as `10 * a[2].length + a.length`.
fn deep() -> Vec<u8> {
    let mut c = ClassBuilder::new("Deep", "java/lang/Object", 49, 0);
    let big = c.method_ref("Deep", "big", "(I)I");
    let small = c.method_ref("Deep", "small", "(I)I");
    let overflow = c.class("java/lang/StackOverflowError");
    let memory = c.class("java/lang/OutOfMemoryError");
    let bytes2 = c.class("[[B");
    let ints3 = c.class("[[[I");
    let [bh, bl] = idx(big);
    let [sh, sl] = idx(small);
    let [xh, xl] = idx(bytes2);
    let [ih, il] = idx(ints3);
    c.method(STATIC, "big", "(I)I", 2000, 2000, &recursion([bh, bl]), &[]);
    c.method(STATIC, "small", "(I)I", 3, 1, &recursion([sh, sl]), &[]);
    c.method(
        STATIC,
        "probeBig",
        "(I)I",
        1,
        2,
        &probe_overflow([bh, bl]),
        &[(0, 5, 5, overflow)],
    );
    c.method(
        STATIC,
        "probeSmall",
        "(I)I",
        1,
        2,
        &probe_overflow([sh, sl]),
        &[(0, 5, 5, overflow)],
    );
    c.method(STATIC, "wide", "(JJ)V", 0, 4, &[0xb1], &[]);
    #[rustfmt::skip]
    let huge = [
        0x11, 0x75, 0x30,           // 0: sipush 30000
        0x03,                       // 3: iconst_0
        0xc5, xh, xl, 0x02,         // 4: multianewarray [[B 2
        0x57,                       // 8: pop
        0x03, 0xac,                 // 9: iconst_0 ireturn
        0x4b, 0x04, 0xac,           // 11: astore_0 iconst_1 ireturn
    ];
    c.method(STATIC, "huge", "()I", 2, 1, &huge, &[(0, 9, 11, memory)]);
    #[rustfmt::skip]
    let shape = [
        0x06, 0x07,                 // 0: iconst_3 iconst_4
        0xc5, ih, il, 0x02,         // 2: multianewarray [[[I 2
        0x4b,                       // 6: astore_0
        0x2a, 0x05, 0x32, 0xbe,     // 7: aload_0 iconst_2 aaload arraylength
        0x10, 0x0a, 0x68,           // 11: bipush 10 imul
        0x2a, 0xbe, 0x60, 0xac,     // 14: aload_0 arraylength iadd ireturn
    ];
    c.method(STATIC, "shape", "()I", 2, 1, &shape, &[]);
    c.build()
}

/// `demo/Probe.entered()`: tells the test, without the VM lock, that the
/// thread is about to start its loop.
fn probe(entered: Arc<AtomicBool>) -> NativeClass {
    NativeClass::new("demo/Probe").static_method("entered", "()V", move |_, _| {
        entered.store(true, Ordering::SeqCst);
        Ok(Value::Void)
    })
}

fn vm_with(heap: usize, limits: VmLimits, natives: Vec<NativeClass>) -> Vm {
    let source = support::source(&[("Loops", loops()), ("Deep", deep())]);
    let config = VmConfig {
        heap_bytes: heap,
        limits,
        log: Arc::new(|_| {}),
        ..VmConfig::default()
    };
    Vm::new(config, Arc::new(source), natives)
}

/// Starts `Loops.<method>` on a Java thread and returns once it is about
/// to loop, and a little after, so that it is in the loop.
fn start_loop(vm: &Vm, entered: &Arc<AtomicBool>, method: &'static str) {
    vm.spawn("main", move |ctx| {
        ctx.call_static("Loops", method, "()V", &[]).map(|_| ())
    })
    .expect("the thread starts");
    let start = Instant::now();
    while !entered.load(Ordering::SeqCst) {
        assert!(start.elapsed() < LONG, "the loop in {method} started");
        std::thread::sleep(Duration::from_millis(1));
    }
    std::thread::sleep(Duration::from_millis(50));
}

/// Runs `f` on a thread of its own and waits `LONG` for its result. A
/// call that waits for a VM lock that a loop never gives up does not
/// return, so the test fails instead of hanging on it.
fn within_long<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> Option<R> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(LONG).ok()
}

/// Stops `vm` from a thread of its own and says whether its threads ended.
fn stops(vm: &Vm) -> bool {
    let stopper = vm.clone();
    let stopped = within_long(move || {
        stopper.stop();
        stopper.wait_finished(Some(LONG))
    });
    stopped == Some(true)
}

/// Reads the heap counters of `vm` from the host, again and again, until
/// `done` says they show what the test waits for. The heap must stay
/// within three times its limit at every reading. A loop on a loaded
/// machine takes as long as it takes to get there, so the test waits
/// instead of guessing how long that is.
fn watch_heap(vm: &Vm, done: impl Fn(&HeapStats) -> bool) -> HeapStats {
    let start = Instant::now();
    loop {
        let host = vm.clone();
        let stats = within_long(move || host.heap_stats()).expect("host calls return");
        assert!(
            stats.used_bytes < 3 * HEAP,
            "the heap counts {} bytes of {HEAP}",
            stats.used_bytes
        );
        if done(&stats) {
            return stats;
        }
        assert!(
            start.elapsed() < LONG,
            "the heap never showed it: {stats:?}"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Calls `Loops.count` and `Deep.<method>` of `vm` from the calling
/// thread, as an attached Java thread.
fn call_int(vm: &Vm, class: &'static str, method: &str, arg: Option<i32>) -> i32 {
    vm.attach("test", |ctx| {
        let (desc, args) = match arg {
            Some(n) => ("(I)I", vec![Value::Int(n)]),
            None => ("()I", Vec::new()),
        };
        match ctx.call_static(class, method, desc, &args) {
            Ok(v) => v.int(),
            Err(Throw::Exception(e)) => {
                panic!("{class}.{method} threw {}", ctx.describe_exception(e))
            }
            Err(Throw::Stop) => panic!("{class}.{method} was stopped"),
        }
    })
}

#[test]
fn a_throw_and_catch_loop_ends_when_the_vm_stops() {
    // A loop that throws and catches a finite number of times, many more
    // than a safepoint of the handler budget covers, still ends and
    // counts them all.
    let counting = vm_with(HEAP, VmLimits::default(), Vec::new());
    assert_eq!(call_int(&counting, "Loops", "count", Some(0)), 0);
    assert_eq!(call_int(&counting, "Loops", "count", Some(50_000)), 50_000);
    for method in ["spin", "fresh"] {
        let entered = Arc::new(AtomicBool::new(false));
        let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
        start_loop(&vm, &entered, method);
        assert!(stops(&vm), "the throw and catch loop of {method} ends");
    }
}

#[test]
fn a_throw_and_catch_loop_leaves_the_vm_lock_to_the_host() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "spin");
    // Each of these takes the VM lock and waits for it as long as it takes:
    // a loop that never reached a safepoint would hold it for ever.
    let host = vm.clone();
    let seen = within_long(move || (host.thread_count(), host.heap_stats()));
    let (threads, stats) = seen.expect("host calls return while the loop runs");
    assert_eq!(threads, 1);
    assert!(stats.used_bytes < 3 * HEAP);
    assert!(stops(&vm), "the loop ends when the VM stops");
}

#[test]
fn a_paused_vm_parks_a_throw_and_catch_loop() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "spin");
    vm.pause();
    // A parked thread waits without the VM lock; one that never reached a
    // safepoint holds it.
    let host = vm.clone();
    assert_eq!(
        within_long(move || host.thread_count()),
        Some(1),
        "the loop parks"
    );
    vm.resume();
    assert!(stops(&vm), "the loop ends when the VM stops");
}

#[test]
fn a_handler_loop_that_makes_exceptions_stays_in_its_heap() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "fresh");
    // The garbage of the exceptions was collected, and the heap stayed
    // small while it was.
    watch_heap(&vm, |stats| stats.collections > 0);
    assert!(stops(&vm), "the loop ends when the VM stops");
}

#[test]
fn a_handler_loop_that_keeps_arrays_stops_at_the_heap_limit() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "hoard");
    // The chain fills the heap, and the heap does not grow past it.
    watch_heap(&vm, |stats| stats.used_bytes > HEAP / 2);
    assert!(stops(&vm), "the loop ends when the VM stops");
}

#[test]
fn a_collection_by_another_thread_keeps_the_exception_of_a_handler_loop() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "keep");
    // The loop waits at its safepoint with nothing but its operand stack
    // to hold the exception, while this thread fills the heap and
    // collects it again and again.
    vm.spawn("churn", |ctx| {
        ctx.call_static("Loops", "churn", "()V", &[]).map(|_| ())
    })
    .expect("the thread starts");
    let start = Instant::now();
    loop {
        let host = vm.clone();
        let (threads, stats) = within_long(move || (host.thread_count(), host.heap_stats()))
            .expect("host calls return");
        assert_eq!(threads, 2, "the handler loop kept running");
        if stats.collections >= 100 {
            break;
        }
        assert!(start.elapsed() < LONG, "the heap was collected: {stats:?}");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(stops(&vm), "both threads end when the VM stops");
}

#[test]
fn a_frame_past_the_slot_limit_throws_stack_overflow_error() {
    let limits = VmLimits {
        max_stack_slots: 4096,
        ..VmLimits::default()
    };
    let vm = vm_with(HEAP, limits, Vec::new());
    // One frame of 4,002 slots fits; a second does not.
    assert_eq!(call_int(&vm, "Deep", "probeBig", Some(0)), 0);
    assert_eq!(call_int(&vm, "Deep", "probeBig", Some(1)), -1);
    // The thread goes on: frames that need little still recurse deep.
    assert_eq!(call_int(&vm, "Deep", "probeSmall", Some(500)), 500);
    assert_eq!(call_int(&vm, "Deep", "probeBig", Some(1)), -1);
}

#[test]
fn the_default_slot_limit_allows_frames_of_a_real_game_and_not_gigabytes() {
    let vm = vm_with(HEAP, VmLimits::default(), Vec::new());
    // A frame of this method moves the stack on by about 2,000 slots, so
    // 131,072 slots hold about 63 of them, and a recursion of small frames
    // reaches the limit on frames (4,096) first.
    assert_eq!(call_int(&vm, "Deep", "probeBig", Some(20)), 20);
    assert_eq!(call_int(&vm, "Deep", "probeBig", Some(100)), -1);
    assert_eq!(call_int(&vm, "Deep", "probeSmall", Some(3000)), 3000);
    assert_eq!(call_int(&vm, "Deep", "probeSmall", Some(100_000)), -1);
}

#[test]
fn a_native_that_calls_into_java_past_the_slot_limit_gets_stack_overflow_error() {
    let limits = VmLimits {
        max_stack_slots: 3,
        ..VmLimits::default()
    };
    let vm = vm_with(HEAP, limits, Vec::new());
    let text = vm.attach("test", |ctx| {
        // Four argument slots do not fit in three.
        match ctx.call_static("Deep", "wide", "(JJ)V", &[Value::Long(1), Value::Long(2)]) {
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            Err(Throw::Stop) => panic!("stopped"),
            Ok(_) => String::new(),
        }
    });
    assert!(
        text.contains("StackOverflowError"),
        "the call threw {text:?}"
    );
}

#[test]
fn multianewarray_counts_each_sub_array_against_the_heap() {
    let vm = vm_with(HEAP, VmLimits::default(), Vec::new());
    // What fits is built, with the sizes it was asked for: new int[3][4][].
    assert_eq!(call_int(&vm, "Deep", "shape", None), 43);
    // 30,000 arrays of 16 bytes, and the 120,000 bytes that hold them, do
    // not fit in 256 KiB; the outer array alone does.
    assert_eq!(call_int(&vm, "Deep", "huge", None), 1);
    let stats = vm.heap_stats();
    assert!(
        stats.freed_objects < 1000,
        "no sub-array was built: {} objects freed",
        stats.freed_objects
    );
    assert!(stats.live_objects < 1000, "{} live", stats.live_objects);
}
