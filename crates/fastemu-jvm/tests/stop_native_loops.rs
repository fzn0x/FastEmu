//! Natives that loop around calls into Java end when the VM stops, even
//! when the Java methods they call hold no backward branch and no call,
//! which would never be a safepoint.
//!
//! The classes are assembled by hand, because each needs exact bytecode.

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use fastemu_jvm::{NativeClass, Value, Vm, VmConfig};
use support::{ClassBuilder, PUBLIC, STATIC, idx};

/// How long a test waits for something that takes milliseconds: far longer
/// than that, so that a busy machine waits instead of failing. A call that
/// has to give up on a locked VM takes about a second.
const LONG: Duration = Duration::from_secs(5);

/// `class Endless extends InputStream`: `read(byte[], int, int)` returns
/// `len` at once, so `InputStream.skip` over it never ends.
fn endless() -> Vec<u8> {
    let mut c = ClassBuilder::new("Endless", "java/io/InputStream", 49, 0);
    c.default_constructor("java/io/InputStream");
    c.method(PUBLIC, "read", "()I", 1, 1, &[0x03, 0xac], &[]); // iconst_0 ireturn
    c.method(PUBLIC, "read", "([BII)I", 1, 4, &[0x1d, 0xac], &[]); // iload_3 ireturn
    c.build()
}

/// `class Skipper extends InputStream`: `skip(long)` returns 1, so
/// `DataInputStream.skipBytes` over it calls it for ever.
fn skipper() -> Vec<u8> {
    let mut c = ClassBuilder::new("Skipper", "java/io/InputStream", 49, 0);
    c.default_constructor("java/io/InputStream");
    c.method(PUBLIC, "read", "()I", 1, 1, &[0x03, 0xac], &[]);
    c.method(PUBLIC, "skip", "(J)J", 2, 3, &[0x0a, 0xad], &[]); // lconst_1 lreturn
    c.build()
}

/// `class EndlessReader extends Reader`: `read(char[], int, int)` returns
/// `len` at once.
fn endless_reader() -> Vec<u8> {
    let mut c = ClassBuilder::new("EndlessReader", "java/io/Reader", 49, 0);
    c.default_constructor("java/io/Reader");
    c.method(PUBLIC, "read", "([CII)I", 1, 4, &[0x1d, 0xac], &[]);
    c.method(PUBLIC, "close", "()V", 0, 1, &[0xb1], &[]);
    c.build()
}

/// `class Loops`: one static method per native loop, each of which tells
/// `Probe.entered()` that it is about to start it.
///
/// - `stream()`: `new Endless().skip(1L << 62)`
/// - `reader()`: `new EndlessReader().skip(1L << 62)`
/// - `bytes()`: `new DataInputStream(new Skipper()).skipBytes(Integer.MAX_VALUE)`
fn loops() -> Vec<u8> {
    let mut c = ClassBuilder::new("Loops", "java/lang/Object", 49, 0);
    let entered = c.method_ref("demo/Probe", "entered", "()V");
    let endless_class = c.class("Endless");
    let endless_init = c.method_ref("Endless", "<init>", "()V");
    let skip_stream = c.method_ref("java/io/InputStream", "skip", "(J)J");
    let reader_class = c.class("EndlessReader");
    let reader_init = c.method_ref("EndlessReader", "<init>", "()V");
    let skip_reader = c.method_ref("java/io/Reader", "skip", "(J)J");
    let data_class = c.class("java/io/DataInputStream");
    let data_init = c.method_ref(
        "java/io/DataInputStream",
        "<init>",
        "(Ljava/io/InputStream;)V",
    );
    let skipper_class = c.class("Skipper");
    let skipper_init = c.method_ref("Skipper", "<init>", "()V");
    let skip_bytes = c.method_ref("java/io/DataInputStream", "skipBytes", "(I)I");
    let max = c.integer(i32::MAX);
    let [eh, el] = idx(entered);
    let [ech, ecl] = idx(endless_class);
    let [eih, eil] = idx(endless_init);
    let [ssh, ssl] = idx(skip_stream);
    let [rch, rcl] = idx(reader_class);
    let [rih, ril] = idx(reader_init);
    let [srh, srl] = idx(skip_reader);
    let [dch, dcl] = idx(data_class);
    let [dih, dil] = idx(data_init);
    let [kch, kcl] = idx(skipper_class);
    let [kih, kil] = idx(skipper_init);
    let [sbh, sbl] = idx(skip_bytes);
    let [mh, ml] = idx(max);
    #[rustfmt::skip]
    let stream = [
        0xb8, eh, el,           // invokestatic Probe.entered
        0xbb, ech, ecl, 0x59,   // new Endless, dup
        0xb7, eih, eil,         // invokespecial <init>
        0x0a, 0x10, 62, 0x79,   // lconst_1 bipush 62 lshl
        0xb6, ssh, ssl,         // invokevirtual InputStream.skip
        0x58, 0xb1,             // pop2 return
    ];
    #[rustfmt::skip]
    let reader = [
        0xb8, eh, el,
        0xbb, rch, rcl, 0x59,
        0xb7, rih, ril,
        0x0a, 0x10, 62, 0x79,
        0xb6, srh, srl,
        0x58, 0xb1,
    ];
    #[rustfmt::skip]
    let bytes = [
        0xb8, eh, el,
        0xbb, dch, dcl, 0x59,   // new DataInputStream, dup
        0xbb, kch, kcl, 0x59,   // new Skipper, dup
        0xb7, kih, kil,         // invokespecial Skipper.<init>
        0xb7, dih, dil,         // invokespecial DataInputStream.<init>
        0x13, mh, ml,           // ldc_w Integer.MAX_VALUE
        0xb6, sbh, sbl,         // invokevirtual skipBytes
        0x57, 0xb1,             // pop return
    ];
    c.method(STATIC, "stream", "()V", 4, 0, &stream, &[]);
    c.method(STATIC, "reader", "()V", 4, 0, &reader, &[]);
    c.method(STATIC, "bytes", "()V", 4, 0, &bytes, &[]);
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

fn vm_with(natives: Vec<NativeClass>, config: VmConfig) -> Vm {
    let source = support::source(&[
        ("Endless", endless()),
        ("Skipper", skipper()),
        ("EndlessReader", endless_reader()),
        ("Loops", loops()),
    ]);
    Vm::new(config, Arc::new(source), natives)
}

/// Starts `method` of `Loops` on a Java thread, waits until it is about to
/// loop, and stops the VM from another thread. Returns how long the stop
/// took to take effect: until `stop` returned and every thread had ended.
fn stopped_loop(method: &'static str) -> Duration {
    let entered = Arc::new(AtomicBool::new(false));
    let vm = vm_with(vec![probe(Arc::clone(&entered))], VmConfig::default());
    vm.spawn("main", move |ctx| {
        ctx.call_static("Loops", method, "()V", &[]).map(|_| ())
    })
    .expect("the thread starts");
    let start = Instant::now();
    while !entered.load(Ordering::SeqCst) {
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "the loop started"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    std::thread::sleep(Duration::from_millis(50));
    // On a VM that cannot be stopped `stop` does not return, so it runs on
    // a thread of its own and the test waits for it.
    let (tx, rx) = mpsc::channel();
    let stopper = vm.clone();
    let asked = Instant::now();
    std::thread::spawn(move || {
        stopper.stop();
        let _ = tx.send(());
    });
    assert!(
        rx.recv_timeout(LONG).is_ok(),
        "stop returns while the loop runs"
    );
    assert!(
        vm.wait_finished(Some(LONG)),
        "the loop in {method} ends when the VM stops"
    );
    asked.elapsed()
}

#[test]
fn skip_over_an_endless_stream_ends_when_the_vm_stops() {
    let took = stopped_loop("stream");
    assert!(took < LONG, "{took:?}");
}

#[test]
fn skip_over_an_endless_reader_ends_when_the_vm_stops() {
    let took = stopped_loop("reader");
    assert!(took < LONG, "{took:?}");
}

#[test]
fn skip_bytes_over_an_endless_skip_ends_when_the_vm_stops() {
    let took = stopped_loop("bytes");
    assert!(took < LONG, "{took:?}");
}
