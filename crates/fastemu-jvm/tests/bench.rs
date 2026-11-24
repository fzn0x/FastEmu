//! Interpreter speed. Run in release:
//!
//! ```text
//! cargo test -p fastemu-jvm --release --test bench -- --ignored --nocapture
//! ```

mod support;

use std::sync::Arc;
use std::time::Instant;

use fastemu_jvm::{Value, Vm, VmConfig};
use support::{ClassBuilder, PUBLIC, STATIC, idx};

/// Bytecodes per iteration of `loop`'s body and condition.
const LOOP_BYTECODES: u64 = 12;
/// Bytecodes per iteration of `calls`, counting the callee's four.
const CALL_BYTECODES: u64 = 14;
/// Bytecodes per iteration of `append`, not counting the `setLength` call
/// every 1024th iteration makes.
const APPEND_BYTECODES: u64 = 12;

fn bench_class() -> Vec<u8> {
    let mut c = ClassBuilder::new("Bench", "java/lang/Object", 50, 0);
    // static int loop(int n) {
    //     int sum = 0;
    //     for (int i = 0; i < n; i++) sum = (sum + i) ^ (i >> 3);
    //     return sum;
    // }
    #[rustfmt::skip]
    let looped = [
        0x03, 0x3c, 0x03, 0x3d,       // iconst_0 istore_1 iconst_0 istore_2
        0xa7, 0x00, 0x0e,             // goto 18
        0x1b, 0x1c, 0x60,             // 7: iload_1 iload_2 iadd
        0x1c, 0x06, 0x7a, 0x82,       // iload_2 iconst_3 ishr ixor
        0x3c,                         // istore_1
        0x84, 0x02, 0x01,             // iinc 2 1
        0x1c, 0x1a,                   // 18: iload_2 iload_0
        0xa1, 0xff, 0xf3,             // if_icmplt 7
        0x1b, 0xac,                   // iload_1 ireturn
    ];
    c.method(PUBLIC | STATIC, "loop", "(I)I", 3, 3, &looped, &[]);
    // static int step(int a, int b) { return a + b; }
    c.method(
        PUBLIC | STATIC,
        "step",
        "(II)I",
        2,
        2,
        &[0x1a, 0x1b, 0x60, 0xac],
        &[],
    );
    let step = c.method_ref("Bench", "step", "(II)I");
    let [h, l] = idx(step);
    // static int calls(int n) {
    //     int sum = 0;
    //     for (int i = 0; i < n; i++) sum = step(sum, i) & 0xffff;
    //     return sum;
    // }
    #[rustfmt::skip]
    let calls = [
        0x03, 0x3c, 0x03, 0x3d,       // iconst_0 istore_1 iconst_0 istore_2
        0xa7, 0x00, 0x0f,             // goto 19
        0x1b, 0x1c, 0xb8, h, l,       // 7: iload_1 iload_2 invokestatic step
        0x12, 0x00, 0x7e, 0x3c,       // ldc 0xffff (patched below) iand istore_1
        0x84, 0x02, 0x01,             // iinc 2 1
        0x1c, 0x1a,                   // 19: iload_2 iload_0
        0xa1, 0xff, 0xf2,             // if_icmplt 7
        0x1b, 0xac,                   // iload_1 ireturn
    ];
    let mask = c.integer(0xffff);
    let mut calls = calls.to_vec();
    calls[13] = u8::try_from(mask).expect("the constant pool is small");
    c.method(PUBLIC | STATIC, "calls", "(I)I", 3, 3, &calls, &[]);
    // static int append(int n) {
    //     StringBuffer sb = new StringBuffer();
    //     String s = "0123456789012345678901234567890";
    //     for (int i = 0; i < n; i++) {
    //         sb.append(s);
    //         if ((i & 1023) == 0) sb.setLength(0);
    //     }
    //     return sb.length();
    // }
    let buffer = c.class("java/lang/StringBuffer");
    let init = c.method_ref("java/lang/StringBuffer", "<init>", "()V");
    let add = c.method_ref(
        "java/lang/StringBuffer",
        "append",
        "(Ljava/lang/String;)Ljava/lang/StringBuffer;",
    );
    let clear = c.method_ref("java/lang/StringBuffer", "setLength", "(I)V");
    let length = c.method_ref("java/lang/StringBuffer", "length", "()I");
    let text = c.string("0123456789012345678901234567890");
    let [nh, nl] = idx(buffer);
    let [ih, il] = idx(init);
    let [ah, al] = idx(add);
    let [ch, cl] = idx(clear);
    let [lh, ll] = idx(length);
    #[rustfmt::skip]
    let append = [
        0xbb, nh, nl,                 // 0: new StringBuffer
        0x59,                         // dup
        0xb7, ih, il,                 // invokespecial <init>
        0x4c,                         // astore_1
        0x12, u8::try_from(text).expect("the constant pool is small"), // ldc text
        0x4d,                         // astore_2
        0x03, 0x3e,                   // iconst_0 istore_3
        0xa7, 0x00, 0x19,             // goto 38
        0x2b, 0x2c,                   // 16: aload_1 aload_2
        0xb6, ah, al,                 // invokevirtual append
        0x57,                         // pop
        0x1d, 0x11, 0x03, 0xff, 0x7e, // iload_3 sipush 1023 iand
        0x9a, 0x00, 0x08,             // ifne 35
        0x2b, 0x03,                   // aload_1 iconst_0
        0xb6, ch, cl,                 // invokevirtual setLength
        0x84, 0x03, 0x01,             // 35: iinc 3 1
        0x1d, 0x1a,                   // 38: iload_3 iload_0
        0xa1, 0xff, 0xe8,             // if_icmplt 16
        0x2b,                         // aload_1
        0xb6, lh, ll,                 // invokevirtual length
        0xac,                         // ireturn
    ];
    c.method(PUBLIC | STATIC, "append", "(I)I", 3, 4, &append, &[]);
    c.build()
}

fn measure(vm: &Vm, method: &str, n: i32, per_iteration: u64) -> f64 {
    let start = Instant::now();
    let result = vm.attach("bench", |ctx| {
        ctx.call_static("Bench", method, "(I)I", &[Value::Int(n)])
            .expect("the benchmark runs")
            .int()
    });
    let seconds = start.elapsed().as_secs_f64();
    let rate = (n as u64 * per_iteration) as f64 / seconds;
    println!(
        "{method}: {n} iterations in {seconds:.3} s, {:.1} million bytecodes per second (result {result})",
        rate / 1e6
    );
    rate
}

#[test]
#[ignore = "a benchmark; run it in release with --ignored --nocapture"]
fn bytecodes_per_second() {
    let source = support::source(&[("Bench", bench_class())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    measure(&vm, "loop", 1_000_000, LOOP_BYTECODES);
    measure(&vm, "calls", 1_000_000, CALL_BYTECODES);
    let loop_rate = measure(&vm, "loop", 100_000_000, LOOP_BYTECODES);
    measure(&vm, "calls", 30_000_000, CALL_BYTECODES);
    measure(&vm, "append", 1_000_000, APPEND_BYTECODES);
    measure(&vm, "append", 20_000_000, APPEND_BYTECODES);
    assert!(loop_rate > 0.0);
}

#[test]
fn benchmark_methods_compute_the_right_values() {
    let source = support::source(&[("Bench", bench_class())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    let (looped, calls, appended) = vm.attach("check", |ctx| {
        let l = ctx
            .call_static("Bench", "loop", "(I)I", &[Value::Int(1000)])
            .unwrap()
            .int();
        let c = ctx
            .call_static("Bench", "calls", "(I)I", &[Value::Int(1000)])
            .unwrap()
            .int();
        let a = ctx
            .call_static("Bench", "append", "(I)I", &[Value::Int(2000)])
            .unwrap()
            .int();
        (l, c, a)
    });
    let mut sum = 0i32;
    for i in 0..1000 {
        sum = sum.wrapping_add(i) ^ (i >> 3);
    }
    assert_eq!(looped, sum);
    let mut sum = 0i32;
    for i in 0..1000 {
        sum = (sum + i) & 0xffff;
    }
    assert_eq!(calls, sum);
    // 2000 appends of 31 characters; the buffer was emptied at i = 0 and
    // i = 1024, so it holds the 975 appends from i = 1025 to i = 1999.
    assert_eq!(appended, 975 * 31);
}
