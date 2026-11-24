//! `java.io.PrintStream`, the class of `System.out` and `System.err`.
//! Text goes out in the game's encoding, `microedition.encoding`.
//!
//! As in Java, a print stream never throws `IOException`: a failure of
//! the stream under it sets the flag `checkError` reports, and an
//! `InterruptedIOException` interrupts the thread instead.

use fastemu_core::LogLevel;

use super::access::{write_all, write_array, write_byte};
use super::stream::is_io_exception;
use crate::classfile::access::PROTECTED;
use crate::ctx::{Ctx, FieldId, JResult, Throw};
use crate::heap::Payload;
use crate::lang::float_fmt::{double_to_string, float_to_string};
use crate::lang::string::default_charset;
use crate::lang::system::write_std;
use crate::lang::{check_range, this};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const PS: &str = "java/io/PrintStream";

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

/// What a `PrintStream` keeps beside its `out` field.
#[derive(Default)]
pub(crate) struct PrintState {
    target: Target,
    /// The error flag `checkError` returns.
    trouble: bool,
    /// Set by `close`; later writes only set the error flag.
    closed: bool,
}

impl Payload for PrintState {}

fn out_field(ctx: &mut Ctx<'_>) -> JResult<FieldId> {
    ctx.field_id(PS, "out", "Ljava/io/OutputStream;")
}

fn state<'a>(ctx: &'a mut Ctx<'_>, ps: ObjRef) -> JResult<&'a mut PrintState> {
    ctx.payload_mut::<PrintState>(ps)
}

/// The log level of a print stream that writes to the log, `None` for one
/// over a Java stream.
fn log_level(target: Target) -> Option<LogLevel> {
    match target {
        Target::Out => Some(LogLevel::Out),
        Target::Err => Some(LogLevel::Err),
        Target::Stream => None,
    }
}

/// Records the outcome of a write to the stream under `ps`: an
/// `IOException` sets the error flag instead of reaching the caller, and
/// an `InterruptedIOException` interrupts this thread instead.
fn absorb(ctx: &mut Ctx<'_>, ps: ObjRef, result: JResult<()>) -> JResult<()> {
    match result {
        Err(Throw::Exception(e)) if is_io_exception(ctx, e) => {
            let interrupted = ctx
                .find_class("java/io/InterruptedIOException")
                .is_ok_and(|c| ctx.is_instance(e, c));
            if interrupted {
                ctx.interrupt_self();
            } else {
                state(ctx, ps)?.trouble = true;
            }
            Ok(())
        }
        other => other,
    }
}

/// The open stream under `ps`; `None`, with the error flag set, once the
/// print stream is closed.
fn open_stream(ctx: &mut Ctx<'_>, ps: ObjRef) -> JResult<Option<ObjRef>> {
    let f = out_field(ctx)?;
    let out = ctx.get_field(ps, f).obj();
    let st = state(ctx, ps)?;
    if st.closed || out.is_none() {
        st.trouble = true;
        return Ok(None);
    }
    Ok(out)
}

/// Writes raw bytes, as `write(byte[], int, int)` does.
fn write_bytes(ctx: &mut Ctx<'_>, ps: ObjRef, bytes: &[u8]) -> JResult<()> {
    let st = state(ctx, ps)?;
    if let Some(level) = log_level(st.target) {
        if st.closed {
            st.trouble = true;
            return Ok(());
        }
        write_std(ctx.shared, &mut ctx.state, level, bytes);
        return Ok(());
    }
    let Some(out) = open_stream(ctx, ps)? else {
        return Ok(());
    };
    let result = write_all(ctx, out, bytes);
    absorb(ctx, ps, result)
}

/// Writes `chars` in the game's encoding. Java's print methods send the
/// bytes through the print stream's own `write(byte[], int, int)`, so a
/// game's subclass that overrides it sees them; a plain one writes them
/// straight to its stream.
fn print_chars(ctx: &mut Ctx<'_>, ps: ObjRef, chars: &[u16]) -> JResult<()> {
    let bytes = default_charset(ctx).encode(chars);
    if ctx.class_name(ctx.class_of(ps)) == PS {
        return write_bytes(ctx, ps, &bytes);
    }
    let arr = ctx.new_byte_array(&bytes)?;
    let args = [arr.into(), Value::Int(0), Value::Int(bytes.len() as i32)];
    let result = ctx.call_virtual(ps, "write", "([BII)V", &args).map(|_| ());
    absorb(ctx, ps, result)
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

fn flush(ctx: &mut Ctx<'_>, ps: ObjRef) -> JResult<()> {
    if log_level(state(ctx, ps)?.target).is_some() {
        return Ok(());
    }
    let Some(out) = open_stream(ctx, ps)? else {
        return Ok(());
    };
    let result = ctx.call_virtual(out, "flush", "()V", &[]).map(|_| ());
    absorb(ctx, ps, result)
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![print_stream()]
}

fn print_stream() -> NativeClass {
    let mut class = NativeClass::new(PS)
        .extends("java/io/OutputStream")
        .payload::<PrintState>()
        .field_with("out", "Ljava/io/OutputStream;", PROTECTED)
        .method("<init>", "(Ljava/io/OutputStream;)V", |ctx, a| {
            let out = ctx.non_null(a[1])?;
            let f = out_field(ctx)?;
            ctx.set_field(this(a), f, out.into());
            Ok(Value::Void)
        })
        .method("write", "(I)V", |ctx, a| {
            let ps = this(a);
            if log_level(state(ctx, ps)?.target).is_some() {
                write_bytes(ctx, ps, &[a[1].int() as u8])?;
                return Ok(Value::Void);
            }
            let Some(out) = open_stream(ctx, ps)? else {
                return Ok(Value::Void);
            };
            let result = write_byte(ctx, out, a[1].int());
            absorb(ctx, ps, result)?;
            Ok(Value::Void)
        })
        .method("write", "([BII)V", |ctx, a| {
            let ps = this(a);
            let target = state(ctx, ps)?.target;
            let out = match log_level(target) {
                Some(_) => None,
                None => match open_stream(ctx, ps)? {
                    Some(out) => Some(out),
                    None => return Ok(Value::Void),
                },
            };
            let arr = ctx.non_null(a[1])?;
            let size = ctx.array_len(arr)?;
            let exception = "java/lang/IndexOutOfBoundsException";
            let (off, len) = check_range(ctx, exception, a[2].int(), a[3].int(), size)?;
            match out {
                Some(out) => {
                    let result = write_array(ctx, out, arr, off, len);
                    absorb(ctx, ps, result)?;
                }
                None => {
                    let bytes: Vec<u8> = ctx.array::<i8>(arr)?[off..off + len]
                        .iter()
                        .map(|&b| b as u8)
                        .collect();
                    write_bytes(ctx, ps, &bytes)?;
                }
            }
            Ok(Value::Void)
        })
        .method("flush", "()V", |ctx, a| {
            flush(ctx, this(a))?;
            Ok(Value::Void)
        })
        .method("close", "()V", |ctx, a| {
            let ps = this(a);
            let st = state(ctx, ps)?;
            if st.closed {
                return Ok(Value::Void);
            }
            st.closed = true;
            if log_level(st.target).is_some() {
                return Ok(Value::Void);
            }
            // Java flushes the stream as it closes the encoder in front
            // of it, then closes the stream.
            let f = out_field(ctx)?;
            if let Some(out) = ctx.get_field(ps, f).obj() {
                let flushed = ctx.call_virtual(out, "flush", "()V", &[]).map(|_| ());
                absorb(ctx, ps, flushed)?;
                let result = ctx.call_virtual(out, "close", "()V", &[]).map(|_| ());
                absorb(ctx, ps, result)?;
            }
            ctx.set_field(ps, f, Value::NULL);
            Ok(Value::Void)
        })
        .method("checkError", "()Z", |ctx, a| {
            let ps = this(a);
            if !state(ctx, ps)?.closed {
                flush(ctx, ps)?;
            }
            let f = out_field(ctx)?;
            if let Some(out) = ctx.get_field(ps, f).obj() {
                let print_stream = ctx.find_class(PS)?;
                if ctx.is_instance(out, print_stream) {
                    return ctx.call_virtual(out, "checkError", "()Z", &[]);
                }
            }
            Ok(Value::from(state(ctx, ps)?.trouble))
        })
        .method_with("setError", "()V", PROTECTED, |ctx, a| {
            state(ctx, this(a))?.trouble = true;
            Ok(Value::Void)
        })
        .method("println", "()V", |ctx, a| {
            print_chars(ctx, this(a), &[u16::from(b'\n')])?;
            Ok(Value::Void)
        });
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

/// A `PrintStream` writing to `target`, for `System.out` and
/// `System.err`.
pub(crate) fn std_stream(ctx: &mut Ctx<'_>, target: Target) -> JResult<Value> {
    let class = ctx.find_class(PS)?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(
        obj,
        PrintState {
            target,
            ..PrintState::default()
        },
    );
    Ok(obj.into())
}
