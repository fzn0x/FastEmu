//! `java.lang.System`, `java.lang.Runtime` and `java.io.PrintStream`.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use fastemu_core::LogLevel;

use super::float_fmt::{double_to_string, float_to_string};
use super::string::default_charset;
use super::{string_value, this, throw};
use crate::class::ClassKind;
use crate::ctx::{Ctx, JResult, Throw};
use crate::heap::{Body, Payload, identity_hash};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};
use crate::vm::{VmShared, VmState};

/// Where a `PrintStream` writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Target {
    /// `System.out`: the log sink, as [`LogLevel::Out`].
    Out,
    /// `System.err`: the log sink, as [`LogLevel::Err`].
    Err,
    /// A Java `OutputStream` in the `out` field.
    #[default]
    Stream,
}

impl Payload for Target {}

/// Writes `bytes` to `System.out` or `System.err`, sending each finished
/// line to the log sink.
pub(crate) fn write_std(shared: &VmShared, state: &mut VmState, level: LogLevel, bytes: &[u8]) {
    for &b in bytes {
        if b == b'\n' {
            let line = std::mem::take(line_buffer(state, level));
            emit(shared, state, level, &line);
        } else {
            line_buffer(state, level).push(b);
        }
    }
}

fn line_buffer(state: &mut VmState, level: LogLevel) -> &mut Vec<u8> {
    if level == LogLevel::Out {
        &mut state.out_line
    } else {
        &mut state.err_line
    }
}

fn emit(shared: &VmShared, state: &VmState, level: LogLevel, line: &[u8]) {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let charset = state
        .properties
        .get("microedition.encoding")
        .and_then(|name| super::charset::Charset::lookup(name))
        .unwrap_or(super::charset::Charset::Utf8);
    shared.log(level, String::from_utf16_lossy(&charset.decode(line)));
}

/// Sends what `System.out` and `System.err` hold past their last line
/// break to the log sink.
pub(crate) fn flush_lines(shared: &VmShared, state: &mut VmState) {
    for level in [LogLevel::Out, LogLevel::Err] {
        let line = std::mem::take(line_buffer(state, level));
        if !line.is_empty() {
            emit(shared, state, level, &line);
        }
    }
}

/// Prints a line of text to `System.err`.
pub(crate) fn err_line(ctx: &mut Ctx<'_>, text: &str) {
    let mut bytes = default_charset(ctx).encode(&text.encode_utf16().collect::<Vec<_>>());
    bytes.push(b'\n');
    write_std(ctx.shared, &mut ctx.state, LogLevel::Err, &bytes);
}

/// Writes `chars` to the print stream `stream`.
fn print_chars(ctx: &mut Ctx<'_>, stream: ObjRef, chars: &[u16]) -> JResult<()> {
    let bytes = default_charset(ctx).encode(chars);
    write_bytes(ctx, stream, &bytes)
}

fn write_bytes(ctx: &mut Ctx<'_>, stream: ObjRef, bytes: &[u8]) -> JResult<()> {
    match *ctx.payload::<Target>(stream)? {
        Target::Out => write_std(ctx.shared, &mut ctx.state, LogLevel::Out, bytes),
        Target::Err => write_std(ctx.shared, &mut ctx.state, LogLevel::Err, bytes),
        Target::Stream => {
            let field = ctx.field_id("java/io/PrintStream", "out", "Ljava/io/OutputStream;")?;
            let Some(out) = ctx.get_field(stream, field).obj() else {
                return Ok(());
            };
            let arr = ctx.new_byte_array(bytes)?;
            ctx.call_virtual(
                out,
                "write",
                "([BII)V",
                &[arr.into(), Value::Int(0), Value::Int(bytes.len() as i32)],
            )?;
        }
    }
    Ok(())
}

/// The text `print(Object)` writes.
fn object_text(ctx: &mut Ctx<'_>, v: Value) -> JResult<Vec<u16>> {
    let Some(obj) = v.obj() else {
        return Ok("null".encode_utf16().collect());
    };
    let s = ctx.call_virtual(obj, "toString", "()Ljava/lang/String;", &[])?;
    match s.obj() {
        Some(s) => Ok(ctx.string_utf16(s)?.to_vec()),
        None => Ok("null".encode_utf16().collect()),
    }
}

/// The text of a `print` argument of type `desc`.
fn text(ctx: &mut Ctx<'_>, desc: &str, v: Value) -> JResult<Vec<u16>> {
    let s = match desc {
        "Z" => (if v.boolean() { "true" } else { "false" }).to_owned(),
        "C" => return Ok(vec![v.char()]),
        "I" => v.int().to_string(),
        "J" => v.long().to_string(),
        "F" => float_to_string(v.float()),
        "D" => double_to_string(v.double()),
        "[C" => {
            let arr = ctx.non_null(v)?;
            return Ok(ctx.array::<u16>(arr)?.to_vec());
        }
        "Ljava/lang/String;" => match v.obj() {
            Some(s) => return Ok(ctx.string_utf16(s)?.to_vec()),
            None => "null".to_owned(),
        },
        _ => return object_text(ctx, v),
    };
    Ok(s.encode_utf16().collect())
}

fn print_stream() -> NativeClass {
    let mut class = NativeClass::new("java/io/PrintStream")
        .extends("java/io/OutputStream")
        .payload::<Target>()
        .field_with(
            "out",
            "Ljava/io/OutputStream;",
            crate::classfile::access::PROTECTED,
        )
        .method("<init>", "(Ljava/io/OutputStream;)V", |ctx, a| {
            let out = ctx.non_null(a[1])?;
            let field = ctx.field_id("java/io/PrintStream", "out", "Ljava/io/OutputStream;")?;
            ctx.set_field(this(a), field, out.into());
            Ok(Value::Void)
        })
        .method("println", "()V", |ctx, a| {
            print_chars(ctx, this(a), &[u16::from(b'\n')])?;
            Ok(Value::Void)
        })
        .method("write", "(I)V", |ctx, a| {
            write_bytes(ctx, this(a), &[a[1].int() as u8])?;
            Ok(Value::Void)
        })
        .method("write", "([BII)V", |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)?;
            let (off, n) = super::check_range(
                ctx,
                "java/lang/IndexOutOfBoundsException",
                a[2].int(),
                a[3].int(),
                len,
            )?;
            let bytes: Vec<u8> = ctx.array::<i8>(arr)?[off..off + n]
                .iter()
                .map(|&b| b as u8)
                .collect();
            write_bytes(ctx, this(a), &bytes)?;
            Ok(Value::Void)
        })
        .method("flush", "()V", |ctx, a| {
            if *ctx.payload::<Target>(this(a))? == Target::Stream {
                let field = ctx.field_id("java/io/PrintStream", "out", "Ljava/io/OutputStream;")?;
                if let Some(out) = ctx.get_field(this(a), field).obj() {
                    ctx.call_virtual(out, "flush", "()V", &[])?;
                }
            }
            Ok(Value::Void)
        })
        .method("close", "()V", |_, _| Ok(Value::Void))
        .method("checkError", "()Z", |_, _| Ok(Value::from(false)));
    for desc in [
        "Z",
        "C",
        "I",
        "J",
        "F",
        "D",
        "[C",
        "Ljava/lang/String;",
        "Ljava/lang/Object;",
    ] {
        let d = desc.to_owned();
        class = class.method("print", &format!("({desc})V"), move |ctx, a| {
            let chars = text(ctx, &d, a[1])?;
            print_chars(ctx, this(a), &chars)?;
            Ok(Value::Void)
        });
        let d = desc.to_owned();
        class = class.method("println", &format!("({desc})V"), move |ctx, a| {
            let mut chars = text(ctx, &d, a[1])?;
            chars.push(u16::from(b'\n'));
            print_chars(ctx, this(a), &chars)?;
            Ok(Value::Void)
        });
    }
    class
}

/// A `PrintStream` writing to `target`.
fn std_stream(ctx: &mut Ctx<'_>, target: Target) -> JResult<Value> {
    let class = ctx.find_class("java/io/PrintStream")?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(obj, target);
    Ok(obj.into())
}

fn array_store_error(ctx: &mut Ctx<'_>, message: String) -> Throw {
    ctx.new_exception("java/lang/ArrayStoreException", Some(&message))
}

/// `System.arraycopy` with every check Java makes.
fn arraycopy(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let src = ctx.non_null(a[0])?;
    let dst = ctx.non_null(a[2])?;
    let (src_pos, dst_pos, len) = (a[1].int(), a[3].int(), a[4].int());
    let src_class = ctx.class_of(src);
    let dst_class = ctx.class_of(dst);
    let src_kind = ctx.state.classes.get(src_class).kind;
    let dst_kind = ctx.state.classes.get(dst_class).kind;
    let describe = |ctx: &Ctx<'_>, c| ctx.class_name(c).replace('/', ".");
    match (src_kind, dst_kind) {
        (ClassKind::PrimArray(s), ClassKind::PrimArray(d)) if s == d => {}
        (ClassKind::RefArray(_), ClassKind::RefArray(_)) => {}
        (
            ClassKind::PrimArray(_) | ClassKind::RefArray(_),
            ClassKind::PrimArray(_) | ClassKind::RefArray(_),
        ) => {
            let message = format!(
                "arraycopy: type mismatch: can not copy {}[] into {}[]",
                describe(ctx, src_class),
                describe(ctx, dst_class)
            );
            return Err(array_store_error(ctx, message));
        }
        (ClassKind::PrimArray(_) | ClassKind::RefArray(_), _) => {
            let message = format!(
                "arraycopy: destination type {} is not an array",
                describe(ctx, dst_class)
            );
            return Err(array_store_error(ctx, message));
        }
        _ => {
            let message = format!(
                "arraycopy: source type {} is not an array",
                describe(ctx, src_class)
            );
            return Err(array_store_error(ctx, message));
        }
    }
    let src_len = ctx.array_len(src)?;
    let dst_len = ctx.array_len(dst)?;
    if src_pos < 0
        || dst_pos < 0
        || len < 0
        || i64::from(src_pos) + i64::from(len) > src_len as i64
        || i64::from(dst_pos) + i64::from(len) > dst_len as i64
    {
        let message = if len < 0 {
            format!("arraycopy: length {len} is negative")
        } else if src_pos < 0 || i64::from(src_pos) + i64::from(len) > src_len as i64 {
            format!(
                "arraycopy: last source index {} out of bounds for length {src_len}",
                i64::from(src_pos) + i64::from(len)
            )
        } else {
            format!(
                "arraycopy: last destination index {} out of bounds for length {dst_len}",
                i64::from(dst_pos) + i64::from(len)
            )
        };
        return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
    }
    let (sp, dp, n) = (src_pos as usize, dst_pos as usize, len as usize);
    if n == 0 {
        return Ok(Value::Void);
    }
    if let (ClassKind::RefArray(se), ClassKind::RefArray(de)) = (src_kind, dst_kind)
        && !ctx.is_assignable(se, de)
    {
        // Elements are checked one by one; the ones before a bad element
        // are copied, as in Java.
        let elems: Vec<Option<ObjRef>> = ctx.array::<Option<ObjRef>>(src)?[sp..sp + n].to_vec();
        for (i, e) in elems.into_iter().enumerate() {
            if let Some(obj) = e {
                let oc = ctx.class_of(obj);
                if !ctx.is_assignable(oc, de) {
                    let message = format!(
                        "arraycopy: element type mismatch: can not cast one of the elements of {}[] to the type of the destination array, {}",
                        describe(ctx, se),
                        describe(ctx, de)
                    );
                    return Err(array_store_error(ctx, message));
                }
            }
            ctx.array_mut::<Option<ObjRef>>(dst)?[dp + i] = e;
        }
        return Ok(Value::Void);
    }
    if src == dst {
        copy_within(&mut ctx.state.heap.get_mut(src).body, sp, dp, n);
    } else {
        let (from, to) = ctx.state.heap.pair_mut(src, dst);
        copy_between(&from.body, &mut to.body, sp, dp, n);
    }
    Ok(Value::Void)
}

fn copy_within(body: &mut Body, sp: usize, dp: usize, n: usize) {
    match body {
        Body::Bytes(a) => a.copy_within(sp..sp + n, dp),
        Body::Chars(a) => a.copy_within(sp..sp + n, dp),
        Body::Shorts(a) => a.copy_within(sp..sp + n, dp),
        Body::Ints(a) => a.copy_within(sp..sp + n, dp),
        Body::Longs(a) => a.copy_within(sp..sp + n, dp),
        Body::Floats(a) => a.copy_within(sp..sp + n, dp),
        Body::Doubles(a) => a.copy_within(sp..sp + n, dp),
        Body::Refs(a) => a.copy_within(sp..sp + n, dp),
        Body::Fields(_) => {}
    }
}

fn copy_between(from: &Body, to: &mut Body, sp: usize, dp: usize, n: usize) {
    match (from, to) {
        (Body::Bytes(s), Body::Bytes(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Chars(s), Body::Chars(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Shorts(s), Body::Shorts(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Ints(s), Body::Ints(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Longs(s), Body::Longs(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Floats(s), Body::Floats(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Doubles(s), Body::Doubles(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        (Body::Refs(s), Body::Refs(d)) => d[dp..dp + n].copy_from_slice(&s[sp..sp + n]),
        _ => {}
    }
}

fn exit(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    if !ctx.shared.config.allow_exit {
        return throw(
            ctx,
            "java/lang/SecurityException",
            "MIDlets may not call System.exit",
        );
    }
    ctx.state.exit_code = Some(a[0].int());
    flush_lines(ctx.shared, &mut ctx.state);
    ctx.shared
        .stopping
        .store(true, std::sync::atomic::Ordering::Release);
    ctx.shared.cond.notify_all();
    Err(Throw::Stop)
}

fn epoch_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn process_nanos() -> i64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as i64
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/System")
            .final_class()
            .static_field("out", "Ljava/io/PrintStream;")
            .static_field("err", "Ljava/io/PrintStream;")
            .static_init(|ctx, _| {
                let out = std_stream(ctx, Target::Out)?;
                let err = std_stream(ctx, Target::Err)?;
                let out_field =
                    ctx.static_field_id("java/lang/System", "out", "Ljava/io/PrintStream;")?;
                let err_field =
                    ctx.static_field_id("java/lang/System", "err", "Ljava/io/PrintStream;")?;
                ctx.state.classes.statics[out_field.index as usize] = out;
                ctx.state.classes.statics[err_field.index as usize] = err;
                Ok(Value::Void)
            })
            .static_method("currentTimeMillis", "()J", |_, _| {
                Ok(Value::Long(epoch_millis()))
            })
            .static_method("nanoTime", "()J", |_, _| Ok(Value::Long(process_nanos())))
            .static_method(
                "arraycopy",
                "(Ljava/lang/Object;ILjava/lang/Object;II)V",
                arraycopy,
            )
            .static_method("identityHashCode", "(Ljava/lang/Object;)I", |_, a| {
                Ok(Value::Int(a[0].obj().map_or(0, identity_hash)))
            })
            .static_method(
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                |ctx, a| get_property(ctx, a[0], Value::NULL),
            )
            .static_method(
                "getProperty",
                "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                |ctx, a| get_property(ctx, a[0], a[1]),
            )
            .static_method("gc", "()V", |ctx, _| {
                ctx.gc();
                Ok(Value::Void)
            })
            .static_method("exit", "(I)V", exit),
        NativeClass::new("java/lang/Runtime")
            .static_field("runtime", "Ljava/lang/Runtime;")
            .static_method("getRuntime", "()Ljava/lang/Runtime;", |ctx, _| {
                let field =
                    ctx.static_field_id("java/lang/Runtime", "runtime", "Ljava/lang/Runtime;")?;
                let current = ctx.get_static(field)?;
                if !current.is_null() {
                    return Ok(current);
                }
                let class = ctx.find_class("java/lang/Runtime")?;
                let obj = ctx.new_object(class)?;
                ctx.set_static(field, obj.into())?;
                Ok(obj.into())
            })
            .method("totalMemory", "()J", |ctx, _| {
                Ok(Value::Long(ctx.heap_usage().0 as i64))
            })
            .method("maxMemory", "()J", |ctx, _| {
                Ok(Value::Long(ctx.heap_usage().0 as i64))
            })
            .method("freeMemory", "()J", |ctx, _| {
                let (limit, used) = ctx.heap_usage();
                Ok(Value::Long(limit.saturating_sub(used) as i64))
            })
            .method("gc", "()V", |ctx, _| {
                ctx.gc();
                Ok(Value::Void)
            })
            .method("exit", "(I)V", |ctx, a| exit(ctx, &a[1..])),
        NativeClass::new("java/io/OutputStream")
            .abstract_class()
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .abstract_method("write", "(I)V")
            .method("write", "([B)V", |ctx, a| {
                let arr = ctx.non_null(a[1])?;
                let len = ctx.array_len(arr)? as i32;
                ctx.call_virtual(
                    this(a),
                    "write",
                    "([BII)V",
                    &[arr.into(), Value::Int(0), Value::Int(len)],
                )?;
                Ok(Value::Void)
            })
            .method("write", "([BII)V", |ctx, a| {
                let arr = ctx.non_null(a[1])?;
                let len = ctx.array_len(arr)?;
                let (off, n) = super::check_range(
                    ctx,
                    "java/lang/IndexOutOfBoundsException",
                    a[2].int(),
                    a[3].int(),
                    len,
                )?;
                for i in off..off + n {
                    let b = ctx.array::<i8>(arr)?[i];
                    ctx.call_virtual(this(a), "write", "(I)V", &[Value::Int(i32::from(b))])?;
                }
                Ok(Value::Void)
            })
            .method("flush", "()V", |_, _| Ok(Value::Void))
            .method("close", "()V", |_, _| Ok(Value::Void)),
        print_stream(),
    ]
}

fn get_property(ctx: &mut Ctx<'_>, key: Value, default: Value) -> JResult<Value> {
    let key_obj = ctx.non_null(key)?;
    let key = ctx.string(key_obj)?;
    if key.is_empty() {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "key can't be empty",
        );
    }
    match ctx.property(&key).map(str::to_owned) {
        Some(v) => string_value(ctx, &v),
        None => Ok(default),
    }
}
