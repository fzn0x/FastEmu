//! Bytecode as old compilers wrote it: class file version 45.3 without
//! `ACC_SUPER`, and `finally` blocks compiled to `jsr` and `ret`.

mod support;

use std::sync::Arc;

use fastemu_jvm::{Throw, Value, Vm, VmConfig};
use support::{ClassBuilder, PUBLIC, STATIC, idx};

fn legacy_class() -> Vec<u8> {
    let mut c = ClassBuilder::new("Legacy", "java/lang/Object", 45, 3);
    c.access(PUBLIC);
    c.field(STATIC, "counter", "I");
    let counter = c.field_ref("Legacy", "counter", "I");
    let [ch, cl] = idx(counter);

    // static int twice(int x) { try { r = x * 2; } finally { counter++; } return r; }
    #[rustfmt::skip]
    let twice = [
        0x1a, 0x05, 0x68, 0x3c,       // iload_0 iconst_2 imul istore_1
        0xa8, 0x00, 0x09,             // jsr 13
        0x1b, 0xac,                   // iload_1 ireturn
        0x00, 0x00, 0x00, 0x00,       // nop x4
        0x4d,                         // 13: astore_2
        0xb2, ch, cl, 0x04, 0x60,     // getstatic counter, iconst_1, iadd
        0xb3, ch, cl,                 // putstatic counter
        0xa9, 0x02,                   // ret 2
    ];
    c.method(STATIC, "twice", "(I)I", 2, 3, &twice, &[]);

    // static int loop(int n) { for (i = 0; i < n; i++) { finally-sub } return counter; }
    #[rustfmt::skip]
    let looped = [
        0x03, 0x3c,                   // iconst_0 istore_1
        0x1b, 0x1a,                   // 2: iload_1 iload_0
        0xa2, 0x00, 0x0d,             // if_icmpge 17
        0xa8, 0x00, 0x0e,             // jsr 21
        0x84, 0x01, 0x01,             // iinc 1 1
        0xa7, 0xff, 0xf5,             // goto 2
        0x00,                         // nop
        0xb2, ch, cl, 0xac,           // 17: getstatic counter, ireturn
        0x4d,                         // 21: astore_2
        0xb2, ch, cl, 0x08, 0x60,     // getstatic counter, iconst_5, iadd
        0xb3, ch, cl,                 // putstatic counter
        0xa9, 0x02,                   // ret 2
    ];
    c.method(STATIC, "loop", "(I)I", 2, 3, &looped, &[]);

    // static int guarded(int x) { try { return 100 / x; } finally { counter += 1000; } }
    #[rustfmt::skip]
    let guarded = [
        0x10, 0x64, 0x1a, 0x6c, 0x3c, // bipush 100, iload_0, idiv, istore_1
        0xa8, 0x00, 0x0f,             // 5: jsr 20
        0x1b, 0xac,                   // iload_1 ireturn
        0x4c,                         // 10: astore_1 (any exception)
        0xa8, 0x00, 0x09,             // jsr 20
        0x2b, 0xbf,                   // aload_1 athrow
        0x00, 0x00, 0x00, 0x00,       // nop x4
        0x4d,                         // 20: astore_2
        0xb2, ch, cl,                 // getstatic counter
        0x11, 0x03, 0xe8, 0x60,       // sipush 1000, iadd
        0xb3, ch, cl,                 // putstatic counter
        0xa9, 0x02,                   // ret 2
    ];
    c.method(STATIC, "guarded", "(I)I", 2, 3, &guarded, &[(0, 5, 10, 0)]);
    c.build()
}

fn call(vm: &Vm, name: &str, arg: i32) -> Result<i32, String> {
    vm.attach("test", |ctx| {
        match ctx.call_static("Legacy", name, "(I)I", &[Value::Int(arg)]) {
            Ok(v) => Ok(v.int()),
            Err(Throw::Exception(e)) => Err(ctx.describe_exception(e)),
            Err(Throw::Stop) => Err("stopped".to_owned()),
        }
    })
}

fn counter(vm: &Vm) -> i32 {
    vm.attach("test", |ctx| {
        let field = ctx.static_field_id("Legacy", "counter", "I").unwrap();
        ctx.get_static(field).unwrap().int()
    })
}

#[test]
fn jsr_and_ret_run_finally_blocks() {
    let source = support::source(&[("Legacy", legacy_class())]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    assert_eq!(call(&vm, "twice", 21), Ok(42));
    assert_eq!(counter(&vm), 1);
    assert_eq!(call(&vm, "loop", 3), Ok(16));
    assert_eq!(call(&vm, "guarded", 4), Ok(25));
    assert_eq!(counter(&vm), 1016);
    assert_eq!(
        call(&vm, "guarded", 0),
        Err("java.lang.ArithmeticException: / by zero".to_owned())
    );
    assert_eq!(
        counter(&vm),
        2016,
        "the finally block ran on the exception path"
    );
}

#[test]
fn version_45_3_without_acc_super_dispatches_specials_directly() {
    // class Base { int value() { return 1; } }
    let mut base = ClassBuilder::new("Base", "java/lang/Object", 45, 3);
    base.access(PUBLIC);
    base.default_constructor("java/lang/Object");
    base.method(0, "value", "()I", 1, 1, &[0x04, 0xac], &[]);
    // class Mid extends Base { int value() { return 2; } }
    let mut mid = ClassBuilder::new("Mid", "Base", 45, 3);
    mid.access(PUBLIC);
    mid.default_constructor("Base");
    mid.method(0, "value", "()I", 1, 1, &[0x05, 0xac], &[]);
    // class Leaf extends Mid, without ACC_SUPER: invokespecial Base.value
    // runs Base.value itself, not the nearest override.
    let mut leaf = ClassBuilder::new("Leaf", "Mid", 45, 3);
    leaf.access(PUBLIC);
    leaf.default_constructor("Mid");
    let base_value = leaf.method_ref("Base", "value", "()I");
    let [h, l] = idx(base_value);
    leaf.method(0, "viaSpecial", "()I", 1, 1, &[0x2a, 0xb7, h, l, 0xac], &[]);
    let source = support::source(&[
        ("Base", base.build()),
        ("Mid", mid.build()),
        ("Leaf", leaf.build()),
    ]);
    let vm = Vm::new(VmConfig::default(), Arc::new(source), Vec::new());
    let result = vm.attach("test", |ctx| {
        let leaf = ctx.construct("Leaf", "()V", &[]).unwrap();
        let special = ctx
            .call_virtual(leaf, "viaSpecial", "()I", &[])
            .unwrap()
            .int();
        let dispatched = ctx.call_virtual(leaf, "value", "()I", &[]).unwrap().int();
        (special, dispatched)
    });
    assert_eq!(result, (1, 2));
}
