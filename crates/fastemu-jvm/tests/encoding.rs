//! Rule 20 of `docs/compat/emulator-notes.md`: the game's encoding
//! setting, `microedition.encoding`, applies to every conversion between
//! bytes and characters that names no charset, not only to
//! `new String(byte[])`.

use std::sync::{Arc, Mutex};

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{Ctx, MemorySource, ObjRef, Value, Vm, VmConfig};

/// "Жи" in windows-1251.
const CP1251: [u8; 2] = [0xC6, 0xE8];
const TEXT: &str = "\u{416}\u{438}";

fn bytes(ctx: &mut Ctx<'_>, arr: Value) -> Vec<u8> {
    let arr = arr.obj().expect("a byte array");
    ctx.array::<i8>(arr)
        .unwrap()
        .iter()
        .map(|&b| b as u8)
        .collect()
}

fn text(ctx: &mut Ctx<'_>, s: Value) -> String {
    ctx.opt_string(s).unwrap().expect("a string")
}

fn construct(ctx: &mut Ctx<'_>, class: &str, desc: &str, args: &[Value]) -> ObjRef {
    ctx.construct(class, desc, args).unwrap()
}

fn call(ctx: &mut Ctx<'_>, obj: ObjRef, name: &str, desc: &str, args: &[Value]) -> Value {
    ctx.call_virtual(obj, name, desc, args).unwrap()
}

#[test]
fn every_implicit_conversion_uses_the_encoding_setting() {
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        properties: vec![("microedition.encoding".into(), "windows-1251".into())],
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(MemorySource::new()), Vec::new());
    vm.attach("test", |ctx| {
        let encoded = ctx.new_byte_array(&CP1251).unwrap();
        let s = construct(ctx, "java/lang/String", "([B)V", &[encoded.into()]);
        assert_eq!(text(ctx, s.into()), TEXT, "new String(byte[])");
        let back = call(ctx, s, "getBytes", "()[B", &[]);
        assert_eq!(bytes(ctx, back), CP1251, "getBytes()");

        let out = construct(ctx, "java/io/ByteArrayOutputStream", "()V", &[]);
        call(ctx, out, "write", "([B)V", &[encoded.into()]);
        let shown = call(ctx, out, "toString", "()Ljava/lang/String;", &[]);
        assert_eq!(text(ctx, shown), TEXT, "ByteArrayOutputStream.toString()");

        let input = construct(
            ctx,
            "java/io/ByteArrayInputStream",
            "([B)V",
            &[encoded.into()],
        );
        let reader = construct(
            ctx,
            "java/io/InputStreamReader",
            "(Ljava/io/InputStream;)V",
            &[input.into()],
        );
        let first = call(ctx, reader, "read", "()I", &[]);
        assert_eq!(first, Value::Int(0x416), "InputStreamReader");

        let written = construct(ctx, "java/io/ByteArrayOutputStream", "()V", &[]);
        let writer = construct(
            ctx,
            "java/io/OutputStreamWriter",
            "(Ljava/io/OutputStream;)V",
            &[written.into()],
        );
        let s = ctx.new_string(TEXT).unwrap();
        call(ctx, writer, "write", "(Ljava/lang/String;)V", &[s.into()]);
        call(ctx, writer, "flush", "()V", &[]);
        let arr = call(ctx, written, "toByteArray", "()[B", &[]);
        assert_eq!(bytes(ctx, arr), CP1251, "OutputStreamWriter");

        let printed = construct(ctx, "java/io/ByteArrayOutputStream", "()V", &[]);
        let ps = construct(
            ctx,
            "java/io/PrintStream",
            "(Ljava/io/OutputStream;)V",
            &[printed.into()],
        );
        call(ctx, ps, "print", "(Ljava/lang/String;)V", &[s.into()]);
        let arr = call(ctx, printed, "toByteArray", "()[B", &[]);
        assert_eq!(bytes(ctx, arr), CP1251, "PrintStream");

        let field = ctx
            .static_field_id("java/lang/System", "out", "Ljava/io/PrintStream;")
            .unwrap();
        let system_out = ctx.get_static(field).unwrap().obj().unwrap();
        call(
            ctx,
            system_out,
            "println",
            "(Ljava/lang/String;)V",
            &[s.into()],
        );
    });
    let lines = lines.lock().unwrap();
    let out: Vec<&str> = lines
        .iter()
        .filter(|l| l.level == LogLevel::Out)
        .map(|l| l.text.as_str())
        .collect();
    assert_eq!(out, [TEXT], "System.out");
}
