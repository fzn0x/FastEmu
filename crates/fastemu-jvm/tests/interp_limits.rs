//! What a game's bytecode can make the interpreter do without bound: a
//! handler that throws again forms a loop with no branch and no call.
//!
//! The classes are assembled by hand, because each needs code that no
//! compiler writes: a loop that is only exception transfers.

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use fastemu_jvm::{NativeClass, Throw, Value, Vm, VmConfig, VmLimits};
use support::{ClassBuilder, STATIC, idx};

/// How long a test waits for something that takes milliseconds: far longer
/// than that, so that a busy machine waits instead of failing.
const LONG: Duration = Duration::from_secs(5);

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
/// - `count(n)`: a loop that throws and catches `n` times and returns `n`.
fn loops() -> Vec<u8> {
    let mut c = ClassBuilder::new("Loops", "java/lang/Object", 49, 0);
    let entered = c.method_ref("demo/Probe", "entered", "()V");
    let object = c.class("java/lang/Object");
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
    c.method(STATIC, "fresh", "()V", 1, 0, &fresh, &[(3, 8, 5, 0)]);
    c.method(STATIC, "hoard", "()V", 4, 1, &hoard, &[(5, 19, 7, 0)]);
    c.method(STATIC, "count", "(I)I", 2, 3, &count, &[(7, 9, 9, 0)]);
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
    let source = support::source(&[("Loops", loops())]);
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

/// Calls `Loops.<method>` of `vm` from the calling thread, as an attached
/// Java thread.
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
    let host = vm.clone();
    let stats = within_long(move || host.heap_stats()).expect("host calls return");
    assert!(
        stats.used_bytes < 3 * HEAP,
        "the heap counts {} bytes of {HEAP}",
        stats.used_bytes
    );
    assert!(stats.collections > 0, "the garbage was collected");
    assert!(stops(&vm), "the loop ends when the VM stops");
}

#[test]
fn a_handler_loop_that_keeps_arrays_stops_at_the_heap_limit() {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(HEAP, VmLimits::default(), vec![probe(Arc::clone(&entered))]);
    start_loop(&vm, &entered, "hoard");
    let host = vm.clone();
    let stats = within_long(move || host.heap_stats()).expect("host calls return");
    assert!(
        stats.used_bytes < 3 * HEAP,
        "the heap counts {} bytes of {HEAP}",
        stats.used_bytes
    );
    assert!(
        stats.used_bytes > HEAP / 2,
        "the chain filled the heap: {} bytes",
        stats.used_bytes
    );
    assert!(stops(&vm), "the loop ends when the VM stops");
}
