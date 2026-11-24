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
    assert!(loop_rate > 0.0);
}

#[test]
fn benchmark_methods_compute_the_right_values() {
    let source = support::source(&[("Bench", bench_class())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    let (looped, calls) = vm.attach("check", |ctx| {
        let l = ctx
            .call_static("Bench", "loop", "(I)I", &[Value::Int(1000)])
            .unwrap()
            .int();
        let c = ctx
            .call_static("Bench", "calls", "(I)I", &[Value::Int(1000)])
            .unwrap()
            .int();
        (l, c)
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
}
