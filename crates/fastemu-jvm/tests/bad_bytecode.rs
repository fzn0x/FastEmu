//! Malformed bytecode, as a damaged or hostile class file could hold it,
//! throws `VerifyError` when the method is first called instead of
//! crashing the virtual machine.

mod support;

use fastemu_core::LogLevel;
use std::sync::Arc;
use std::time::Duration;

use fastemu_jvm::{Throw, Value, Vm, VmConfig};
use support::{ClassBuilder, Handler, PUBLIC, STATIC, idx};

/// Calls `static void Bad.f()` of the given classes and returns what it
/// threw.
fn run_classes(classes: &[(&str, Vec<u8>)]) -> String {
    let source = support::source(classes);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    vm.attach("test", |ctx| {
        match ctx.call_static("Bad", "f", "()V", &[]) {
            Ok(_) => "returned".to_owned(),
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            Err(Throw::Stop) => "stopped".to_owned(),
        }
    })
}

/// Calls `static void Bad.f()`, whose code is `code`, and returns what it
/// threw.
fn run(max_stack: u16, max_locals: u16, code: &[u8], handlers: &[Handler]) -> String {
    let mut c = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    c.method(
        PUBLIC | STATIC,
        "f",
        "()V",
        max_stack,
        max_locals,
        code,
        handlers,
    );
    run_classes(&[("Bad", c.build())])
}

/// The code of a `static void` method that calls method `callee` (a
/// constant pool index) a thousand times, and `discard` after each call
/// to drop the result.
fn call_in_a_loop(callee: u16, discard: &[u8]) -> Vec<u8> {
    let [h, l] = idx(callee);
    // 0: sipush 1000; 3: istore_0; 4: iload_0; 5: ifeq end; 8: invokestatic
    let mut code = vec![0x11, 0x03, 0xe8, 0x3b, 0x1a, 0x99, 0, 0, 0xb8, h, l];
    code.extend(discard);
    // iinc 0 -1; goto 4; return
    code.extend([0x84, 0, 0xff]);
    let goto = code.len();
    code.extend([0xa7, 0, 0]);
    let end = code.len();
    code.push(0xb1);
    code[6..8].copy_from_slice(&((end - 5) as i16).to_be_bytes());
    code[goto + 1..goto + 3].copy_from_slice(&((4 - goto as i32) as i16).to_be_bytes());
    code
}

/// Calls `Bad.v`, whose descriptor is `desc` and code is `code`, a thousand
/// times from `Bad.f`, which drops each result with `discard`.
fn run_returning(desc: &str, max_stack: u16, code: &[u8], discard: &[u8]) -> String {
    let mut c = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    let v = c.method_ref("Bad", "v", desc);
    c.method(PUBLIC | STATIC, "v", desc, max_stack, 2, code, &[]);
    c.method(
        PUBLIC | STATIC,
        "f",
        "()V",
        2,
        1,
        &call_in_a_loop(v, discard),
        &[],
    );
    run_classes(&[("Bad", c.build())])
}

fn assert_verify_error(result: &str, detail: &str) {
    assert!(
        result.starts_with("java.lang.VerifyError") && result.contains(detail),
        "expected a VerifyError about {detail:?}, got {result}"
    );
}

#[test]
fn code_that_falls_off_its_end_is_refused() {
    assert_verify_error(&run(1, 0, &[0x00], &[]), "falls off the end");
    // iconst_0; ifeq 0 is a loop that may fall through past the end.
    assert_verify_error(
        &run(1, 0, &[0x03, 0x99, 0xff, 0xff], &[]),
        "falls off the end",
    );
}

#[test]
fn constant_pool_operands_out_of_range_are_refused() {
    let [h, l] = idx(0xfff0);
    assert_verify_error(&run(1, 0, &[0xb2, h, l, 0x57, 0xb1], &[]), "constant pool");
    assert_verify_error(&run(1, 0, &[0xbb, h, l, 0x57, 0xb1], &[]), "constant pool");
    // aconst_null; athrow; with a handler naming a missing pool entry.
    let handlers = [(0, 2, 2, 0xfff0)];
    assert_verify_error(
        &run(1, 0, &[0x01, 0xbf, 0x57, 0xb1], &handlers),
        "constant pool",
    );
}

#[test]
fn operand_stack_overflow_and_underflow_are_refused() {
    let mut pushes = vec![0x03; 400];
    pushes.push(0xb1);
    assert_verify_error(&run(1, 0, &pushes, &[]), "operand stack");
    assert_verify_error(&run(1, 0, &[0x57, 0xb1], &[]), "operand stack");
    // iconst_0; goto 0: the stack grows on every turn of the loop.
    assert_verify_error(&run(8, 0, &[0x03, 0xa7, 0xff, 0xff], &[]), "operand stack");
}

#[test]
fn a_subroutine_that_returns_with_more_on_the_stack_is_refused() {
    // 0: jsr 4; 3: return; 4: astore_0; 5: iconst_0; 6: ret 0
    let code = [0xa8, 0x00, 0x04, 0xb1, 0x4b, 0x03, 0xa9, 0x00];
    assert_verify_error(&run(2, 1, &code, &[]), "subroutine");
}

#[test]
fn well_formed_code_still_runs() {
    // iconst_1; iconst_2; iadd; pop; return
    assert_eq!(run(2, 0, &[0x04, 0x05, 0x60, 0x57, 0xb1], &[]), "returned");
}

/// One case of `run_returning`: descriptor, `max_stack`, code, and the
/// instruction that drops the result.
type Returning<'a> = (&'a str, u16, &'a [u8], &'a [u8]);

#[test]
fn a_return_of_another_kind_than_the_method_declares_is_refused() {
    // Each is called a thousand times in a loop: before the check, the
    // caller's stack grew by a slot or more on every call.
    let cases: [Returning<'_>; 7] = [
        // iconst_0; ireturn in a void, a long and a double method
        ("()V", 1, &[0x03, 0xac], &[]),
        ("()J", 1, &[0x03, 0xac], &[0x58]),
        ("()D", 1, &[0x03, 0xac], &[0x58]),
        // return in an int method
        ("()I", 0, &[0xb1], &[0x57]),
        // lconst_0; lreturn in an int and in a void method
        ("()I", 2, &[0x09, 0xad], &[0x57]),
        ("()V", 2, &[0x09, 0xad], &[]),
        // dconst_0; dreturn in a float method
        ("()F", 2, &[0x0e, 0xaf], &[0x57]),
    ];
    for (desc, max_stack, code, discard) in cases {
        let result = run_returning(desc, max_stack, code, discard);
        assert_verify_error(&result, "a return of a different kind");
        assert!(result.contains(".v"), "{desc}: names the method: {result}");
    }
}

#[test]
fn a_mismatched_return_nothing_reaches_still_runs() {
    // goto 5; iconst_0; ireturn; return: the ireturn is dead code of a
    // void method, as obfuscators leave it and phones accept it.
    let code = [0xa7, 0, 5, 0x03, 0xac, 0xb1];
    assert_eq!(run_returning("()V", 1, &code, &[]), "returned");
}

#[test]
fn returns_that_match_the_method_run() {
    let cases: [Returning<'_>; 6] = [
        ("()V", 0, &[0xb1], &[]),
        ("()I", 1, &[0x03, 0xac], &[0x57]),
        ("()J", 2, &[0x09, 0xad], &[0x58]),
        ("()F", 1, &[0x0b, 0xae], &[0x57]),
        ("()D", 2, &[0x0e, 0xaf], &[0x58]),
        ("()Ljava/lang/Object;", 1, &[0x01, 0xb0], &[0x57]),
    ];
    for (desc, max_stack, code, discard) in cases {
        assert_eq!(
            run_returning(desc, max_stack, code, discard),
            "returned",
            "{desc}"
        );
    }
}

/// Calls `Bad.f()`, whose code is `code`, with class `A` (a `foo()V`) and
/// class `B` (a `bar` taking 100 longs) beside it; `foo` and `bar` sit at
/// the same vtable index.
fn run_with_a_and_b(code: impl FnOnce(&mut ClassBuilder) -> Vec<u8>) -> String {
    let mut a = ClassBuilder::new("A", "java/lang/Object", 49, 0);
    a.default_constructor("java/lang/Object");
    a.method(PUBLIC, "foo", "()V", 0, 1, &[0xb1], &[]);
    let mut b = ClassBuilder::new("B", "java/lang/Object", 49, 0);
    b.default_constructor("java/lang/Object");
    let desc = format!("({})V", "J".repeat(100));
    b.method(PUBLIC, "bar", &desc, 0, 201, &[0xb1], &[]);
    let mut bad = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    let code = code(&mut bad);
    bad.method(PUBLIC | STATIC, "f", "()V", 3, 0, &code, &[]);
    run_classes(&[("A", a.build()), ("B", b.build()), ("Bad", bad.build())])
}

/// The code of `new <class>; dup; invokespecial <class>.<init>; invokevirtual
/// A.foo()V; return`.
fn call_foo_on_new(bad: &mut ClassBuilder, class: &str) -> Vec<u8> {
    let [ch, cl] = idx(bad.class(class));
    let [ih, il] = idx(bad.method_ref(class, "<init>", "()V"));
    let [fh, fl] = idx(bad.method_ref("A", "foo", "()V"));
    vec![0xbb, ch, cl, 0x59, 0xb7, ih, il, 0xb6, fh, fl, 0xb1]
}

#[test]
fn a_virtual_call_on_an_object_of_another_class_is_refused() {
    // B.bar takes 100 longs: running it for a call of A.foo wrapped the
    // frame pointer and panicked.
    let result = run_with_a_and_b(|bad| call_foo_on_new(bad, "B"));
    assert!(
        result.starts_with("java.lang.IncompatibleClassChangeError")
            && result.contains("Class B does not have the method 'A.foo()V'"),
        "{result}"
    );
    // Object has no method at that index at all.
    let result = run_with_a_and_b(|bad| call_foo_on_new(bad, "java/lang/Object"));
    assert!(
        result.starts_with("java.lang.IncompatibleClassChangeError"),
        "{result}"
    );
}

#[test]
fn a_virtual_call_on_an_object_of_the_class_still_runs() {
    assert_eq!(
        run_with_a_and_b(|bad| call_foo_on_new(bad, "A")),
        "returned"
    );
}

/// Calls `static int Bad.f()`, which pushes 5, calls `Bad.v()I` whose body
/// is `code`, drops the result and returns the 5: what `f` returns shows
/// whether the call left exactly one slot behind.
fn call_around_a_return(code: &[u8]) -> Value {
    let mut c = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    let v = c.method_ref("Bad", "v", "()I");
    let [vh, vl] = idx(v);
    c.method(PUBLIC | STATIC, "v", "()I", 2, 2, code, &[]);
    // iconst_5; invokestatic v; pop; ireturn
    let f = [0x08, 0xb8, vh, vl, 0x57, 0xac];
    c.method(PUBLIC | STATIC, "f", "()I", 2, 0, &f, &[]);
    let source = support::source(&[("Bad", c.build())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    vm.attach("test", |ctx| {
        match ctx.call_static("Bad", "f", "()I", &[]) {
            Ok(v) => v,
            Err(Throw::Exception(e)) => panic!("{}", ctx.describe_exception(e)),
            Err(Throw::Stop) => panic!("stopped"),
        }
    })
}

#[test]
fn a_return_leaves_the_slots_the_method_declares() {
    // lconst_1; lstore_0; iload_0; ireturn: the long itself, as one slot,
    // where an int is expected. The caller took it for two slots.
    let long_as_one = [0x0a, 0x3f, 0x1a, 0xac];
    assert_eq!(call_around_a_return(&long_as_one), Value::Int(5));
    // lconst_1; lstore_0; iload_1; ireturn: the second half of the long,
    // which holds no value. The caller took it for none.
    let second_half = [0x0a, 0x3f, 0x1b, 0xac];
    assert_eq!(call_around_a_return(&second_half), Value::Int(5));
    // iconst_3; ireturn
    assert_eq!(call_around_a_return(&[0x06, 0xac]), Value::Int(5));
}

/// Calls `static int Bad.v()`, whose code `build` makes, and returns what
/// it returned.
fn call_v(build: impl FnOnce(&mut ClassBuilder) -> Vec<u8>) -> Value {
    let mut c = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    let code = build(&mut c);
    c.method(PUBLIC | STATIC, "v", "()I", 4, 2, &code, &[]);
    let source = support::source(&[("Bad", c.build())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    vm.attach("test", |ctx| {
        match ctx.call_static("Bad", "v", "()I", &[]) {
            Ok(v) => v,
            Err(Throw::Exception(e)) => panic!("{}", ctx.describe_exception(e)),
            Err(Throw::Stop) => panic!("stopped"),
        }
    })
}

#[test]
fn typed_loads_and_stores_read_another_type_as_zero() {
    // ldc_w "s"; istore_0; iload_0; ireturn: a reference stored as an int
    // reads back as 0, never as the reference.
    let reference = call_v(|c| {
        let [h, l] = idx(c.string("s"));
        vec![0x13, h, l, 0x3b, 0x1a, 0xac]
    });
    assert_eq!(reference, Value::Int(0));
    // lconst_1; lstore_0; iload_0; iload_1; iadd; ireturn: a long read as
    // two ints.
    let halves = call_v(|_| vec![0x0a, 0x3f, 0x1a, 0x1b, 0x60, 0xac]);
    assert_eq!(halves, Value::Int(0));
    // fconst_2; fstore_0; iload_0; ireturn: a float read as an int.
    assert_eq!(call_v(|_| vec![0x0d, 0x44, 0x1a, 0xac]), Value::Int(0));
    // iconst_5; istore_0; iload_0; ireturn: an int reads back as itself.
    assert_eq!(call_v(|_| vec![0x08, 0x3b, 0x1a, 0xac]), Value::Int(5));
}

#[test]
fn athrow_of_an_object_that_is_not_a_throwable_is_refused() {
    // new Object; dup; invokespecial Object.<init>; athrow
    let mut c = ClassBuilder::new("Bad", "java/lang/Object", 49, 0);
    let [oh, ol] = idx(c.class("java/lang/Object"));
    let [ih, il] = idx(c.method_ref("java/lang/Object", "<init>", "()V"));
    let code = [0xbb, oh, ol, 0x59, 0xb7, ih, il, 0xbf];
    c.method(PUBLIC | STATIC, "f", "()V", 2, 0, &code, &[]);
    assert_verify_error(&run_classes(&[("Bad", c.build())]), "not a Throwable");
    // iconst_1; newarray int; athrow: an array is no Throwable either.
    assert_verify_error(&run(1, 0, &[0x04, 0xbc, 10, 0xbf], &[]), "not a Throwable");
}

#[test]
fn a_thread_whose_uncaught_exception_cannot_be_reported_ends() {
    // new RuntimeException; dup; invokespecial <init>; dup; new Object;
    // dup; invokespecial Object.<init>; invokevirtual initCause; pop;
    // athrow. Nothing checks the type of an argument, so the cause is a
    // plain Object, and the report of the uncaught exception panics when
    // it reads the cause's own cause from fields the object does not have.
    let mut c = ClassBuilder::new("Thr", "java/lang/Object", 49, 0);
    let [rh, rl] = idx(c.class("java/lang/RuntimeException"));
    let [ch, cl] = idx(c.method_ref("java/lang/RuntimeException", "<init>", "()V"));
    let [oh, ol] = idx(c.class("java/lang/Object"));
    let [ih, il] = idx(c.method_ref("java/lang/Object", "<init>", "()V"));
    let [xh, xl] = idx(c.method_ref(
        "java/lang/Throwable",
        "initCause",
        "(Ljava/lang/Throwable;)Ljava/lang/Throwable;",
    ));
    let code = [
        0xbb, rh, rl, 0x59, 0xb7, ch, cl, 0x59, 0xbb, oh, ol, 0x59, 0xb7, ih, il, 0xb6, xh, xl,
        0x57, 0xbf,
    ];
    c.method(
        PUBLIC | STATIC,
        "main",
        "([Ljava/lang/String;)V",
        4,
        1,
        &code,
        &[],
    );
    let (log, lines) = support::capture();
    let config = VmConfig {
        log,
        ..VmConfig::default()
    };
    let vm = Vm::new(
        config,
        Arc::new(support::source(&[("Thr", c.build())])),
        Vec::new(),
    );
    vm.run_main("Thr", &[]).unwrap();
    assert!(
        vm.wait_finished(Some(Duration::from_secs(3))),
        "the thread is still registered"
    );
    assert_eq!(vm.thread_count(), 0);
    assert!(vm.is_stopping(), "the game is not stopped");
    let errors = support::lines_at(&lines, LogLevel::Err);
    assert!(
        errors
            .iter()
            .any(|l| l.starts_with("internal error in the virtual machine")),
        "{errors:?}"
    );
}
