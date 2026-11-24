//! `java.io.PrintStream`, the class of `System.out` and `System.err`.
//! Text goes out in the game's encoding, `microedition.encoding`.

use fastemu_core::LogLevel;

use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::lang::float_fmt::{double_to_string, float_to_string};
use crate::lang::string::default_charset;
use crate::lang::system::write_std;
use crate::lang::this;
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

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

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![print_stream()]
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
            let (off, n) = crate::lang::check_range(
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

/// A `PrintStream` writing to `target`, for `System.out` and
/// `System.err`.
pub(crate) fn std_stream(ctx: &mut Ctx<'_>, target: Target) -> JResult<Value> {
    let class = ctx.find_class("java/io/PrintStream")?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(obj, target);
    Ok(obj.into())
}
