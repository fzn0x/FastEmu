//! Malformed bytecode, as a damaged or hostile class file could hold it,
//! throws `VerifyError` when the method is first called instead of
//! crashing the virtual machine.

mod support;

use std::sync::Arc;

use fastemu_jvm::{Throw, Vm, VmConfig};
use support::{ClassBuilder, Handler, PUBLIC, STATIC, idx};

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
    let source = support::source(&[("Bad", c.build())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    vm.attach("test", |ctx| {
        match ctx.call_static("Bad", "f", "()V", &[]) {
            Ok(_) => "returned".to_owned(),
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            Err(Throw::Stop) => "stopped".to_owned(),
        }
    })
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
