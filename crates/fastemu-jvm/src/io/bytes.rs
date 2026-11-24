//! `java.io.ByteArrayInputStream` and `java.io.ByteArrayOutputStream`:
//! streams over a `byte[]` kept in protected fields, which a subclass may
//! read and change, as Java's do.

use crate::class::ArrayType;
use crate::classfile::access;
use crate::ctx::{Ctx, FieldId, JResult};
use crate::lang::string::default_charset;
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const BAIS: &str = "java/io/ByteArrayInputStream";
const BAOS: &str = "java/io/ByteArrayOutputStream";

/// The fields of a `ByteArrayInputStream`.
struct Bais {
    buf: FieldId,
    pos: FieldId,
    mark: FieldId,
    count: FieldId,
}

fn fields(ctx: &mut Ctx<'_>) -> JResult<Bais> {
    Ok(Bais {
        buf: ctx.field_id(BAIS, "buf", "[B")?,
        pos: ctx.field_id(BAIS, "pos", "I")?,
        mark: ctx.field_id(BAIS, "mark", "I")?,
        count: ctx.field_id(BAIS, "count", "I")?,
    })
}

/// A `ByteArrayInputStream` over a new `byte[]` holding `bytes`.
pub(crate) fn new_byte_array_stream(ctx: &mut Ctx<'_>, bytes: &[u8]) -> JResult<ObjRef> {
    let arr = ctx.new_byte_array(bytes)?;
    ctx.construct(BAIS, "([B)V", &[arr.into()])
}

fn init(ctx: &mut Ctx<'_>, a: &[Value], offset: i32, len: Option<i32>) -> JResult<Value> {
    let arr = ctx.non_null(a[1])?;
    let size = ctx.array_len(arr)? as i32;
    let f = fields(ctx)?;
    let obj = this(a);
    let count = match len {
        Some(len) => (offset.saturating_add(len)).min(size),
        None => size,
    };
    ctx.set_field(obj, f.buf, arr.into());
    ctx.set_field(obj, f.pos, Value::Int(offset));
    ctx.set_field(obj, f.mark, Value::Int(offset));
    ctx.set_field(obj, f.count, Value::Int(count));
    Ok(Value::Void)
}

/// Checks that `n` bytes from `pos` lie in `buf` and returns `pos`. The
/// fields are protected, so a subclass (or the constructor's offset) can
/// point them anywhere; Java then throws `ArrayIndexOutOfBoundsException`.
fn source_range(ctx: &mut Ctx<'_>, buf: ObjRef, pos: i32, n: usize) -> JResult<usize> {
    let len = ctx.array_len(buf)?;
    match usize::try_from(pos) {
        Ok(at) if at.checked_add(n).is_some_and(|end| end <= len) => Ok(at),
        _ => {
            let message = format!("Index {pos} out of bounds for length {len}");
            throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message)
        }
    }
}

/// Takes up to `max` bytes from the stream `obj` into `dst`, as its
/// `read(byte[], int, int)` does: `None` at the end of the data, else the
/// count (0 only when `max` is 0).
fn take(ctx: &mut Ctx<'_>, obj: ObjRef, max: usize, dst: &mut Vec<u8>) -> JResult<Option<usize>> {
    let f = fields(ctx)?;
    let pos = ctx.get_field(obj, f.pos).int();
    let count = ctx.get_field(obj, f.count).int();
    if pos >= count {
        return Ok(None);
    }
    let n = max.min((count - pos) as usize);
    if n == 0 {
        return Ok(Some(0));
    }
    let Some(buf) = ctx.get_field(obj, f.buf).obj() else {
        return Ok(None);
    };
    let at = source_range(ctx, buf, pos, n)?;
    dst.extend(ctx.array::<i8>(buf)?[at..at + n].iter().map(|&b| b as u8));
    ctx.set_field(obj, f.pos, Value::Int(pos + n as i32));
    Ok(Some(n))
}

/// The fields of a `ByteArrayOutputStream`.
struct Baos {
    buf: FieldId,
    count: FieldId,
}

fn out_fields(ctx: &mut Ctx<'_>) -> JResult<Baos> {
    Ok(Baos {
        buf: ctx.field_id(BAOS, "buf", "[B")?,
        count: ctx.field_id(BAOS, "count", "I")?,
    })
}

/// The buffer and count of the output stream `obj`, read from its fields.
fn out_state(ctx: &mut Ctx<'_>, f: &Baos, obj: ObjRef) -> JResult<(ObjRef, usize)> {
    let buf = ctx.get_field(obj, f.buf);
    let buf = ctx.non_null(buf)?;
    let count = ctx.get_field(obj, f.count).int();
    let len = ctx.array_len(buf)?;
    match usize::try_from(count) {
        Ok(c) if c <= len => Ok((buf, c)),
        _ => {
            let message = format!("Index {count} out of bounds for length {len}");
            throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message)
        }
    }
}

/// Appends `bytes` to the output stream `obj`, growing its buffer as
/// Java's does: to twice its size, or to what is needed if that is more.
fn append(ctx: &mut Ctx<'_>, obj: ObjRef, bytes: &[u8]) -> JResult<()> {
    let f = out_fields(ctx)?;
    let (mut buf, count) = out_state(ctx, &f, obj)?;
    let needed = count + bytes.len();
    let capacity = ctx.array_len(buf)?;
    if needed > i32::MAX as usize {
        return Err(ctx.out_of_memory());
    }
    if needed > capacity {
        let grown = capacity.saturating_mul(2).clamp(needed, i32::MAX as usize);
        let bigger = ctx.new_array(ArrayType::Byte, grown as i32)?;
        let old = ctx.array::<i8>(buf)?[..count].to_vec();
        ctx.array_mut::<i8>(bigger)?[..count].copy_from_slice(&old);
        ctx.set_field(obj, f.buf, bigger.into());
        buf = bigger;
    }
    for (slot, &b) in ctx.array_mut::<i8>(buf)?[count..needed]
        .iter_mut()
        .zip(bytes)
    {
        *slot = b as i8;
    }
    ctx.set_field(obj, f.count, Value::Int(needed as i32));
    Ok(())
}

/// The bytes written to the output stream `obj` so far.
fn written(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Vec<u8>> {
    let f = out_fields(ctx)?;
    let (buf, count) = out_state(ctx, &f, obj)?;
    Ok(ctx.array::<i8>(buf)?[..count]
        .iter()
        .map(|&b| b as u8)
        .collect())
}

fn out_init(ctx: &mut Ctx<'_>, a: &[Value], size: i32) -> JResult<Value> {
    if size < 0 {
        let message = format!("Negative initial size: {size}");
        return throw(ctx, "java/lang/IllegalArgumentException", &message);
    }
    let buf = ctx.new_array(ArrayType::Byte, size)?;
    let f = out_fields(ctx)?;
    ctx.set_field(this(a), f.buf, buf.into());
    Ok(Value::Void)
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![input_stream(), output_stream()]
}

fn input_stream() -> NativeClass {
    NativeClass::new(BAIS)
        .extends("java/io/InputStream")
        .field_with("buf", "[B", access::PROTECTED)
        .field_with("pos", "I", access::PROTECTED)
        .field_with("mark", "I", access::PROTECTED)
        .field_with("count", "I", access::PROTECTED)
        .method("<init>", "([B)V", |ctx, a| init(ctx, a, 0, None))
        .method("<init>", "([BII)V", |ctx, a| {
            init(ctx, a, a[2].int(), Some(a[3].int()))
        })
        .method("read", "()I", |ctx, a| {
            let mut byte = Vec::with_capacity(1);
            let read = take(ctx, this(a), 1, &mut byte)?;
            Ok(Value::Int(match (read, byte.first()) {
                (Some(_), Some(&b)) => i32::from(b),
                _ => -1,
            }))
        })
        .method("read", "([BII)I", |ctx, a| {
            let dst = ctx.non_null(a[1])?;
            let size = ctx.array_len(dst)?;
            let (off, len) = check_range(
                ctx,
                "java/lang/IndexOutOfBoundsException",
                a[2].int(),
                a[3].int(),
                size,
            )?;
            let mut bytes = Vec::with_capacity(len.min(64 * 1024));
            let Some(n) = take(ctx, this(a), len, &mut bytes)? else {
                return Ok(Value::Int(-1));
            };
            for (slot, &b) in ctx.array_mut::<i8>(dst)?[off..off + n]
                .iter_mut()
                .zip(&bytes)
            {
                *slot = b as i8;
            }
            Ok(Value::Int(n as i32))
        })
        .method("skip", "(J)J", |ctx, a| {
            let f = fields(ctx)?;
            let obj = this(a);
            let pos = ctx.get_field(obj, f.pos).int();
            let count = ctx.get_field(obj, f.count).int();
            let n = a[1].long().clamp(0, i64::from(count - pos).max(0));
            ctx.set_field(obj, f.pos, Value::Int(pos + n as i32));
            Ok(Value::Long(n))
        })
        .method("available", "()I", |ctx, a| {
            let f = fields(ctx)?;
            let obj = this(a);
            let pos = ctx.get_field(obj, f.pos).int();
            let count = ctx.get_field(obj, f.count).int();
            Ok(Value::Int((count - pos).max(0)))
        })
        .method("markSupported", "()Z", |_, _| Ok(Value::from(true)))
        .method("mark", "(I)V", |ctx, a| {
            let f = fields(ctx)?;
            let obj = this(a);
            let pos = ctx.get_field(obj, f.pos);
            ctx.set_field(obj, f.mark, pos);
            Ok(Value::Void)
        })
        .method("reset", "()V", |ctx, a| {
            let f = fields(ctx)?;
            let obj = this(a);
            let mark = ctx.get_field(obj, f.mark);
            ctx.set_field(obj, f.pos, mark);
            Ok(Value::Void)
        })
        .method("close", "()V", |_, _| Ok(Value::Void))
}

fn output_stream() -> NativeClass {
    NativeClass::new(BAOS)
        .extends("java/io/OutputStream")
        .field_with("buf", "[B", access::PROTECTED)
        .field_with("count", "I", access::PROTECTED)
        .method("<init>", "()V", |ctx, a| out_init(ctx, a, 32))
        .method("<init>", "(I)V", |ctx, a| out_init(ctx, a, a[1].int()))
        .method("write", "(I)V", |ctx, a| {
            append(ctx, this(a), &[a[1].int() as u8])?;
            Ok(Value::Void)
        })
        .method("write", "([BII)V", |ctx, a| {
            let src = ctx.non_null(a[1])?;
            let size = ctx.array_len(src)?;
            let (off, len) = check_range(
                ctx,
                "java/lang/IndexOutOfBoundsException",
                a[2].int(),
                a[3].int(),
                size,
            )?;
            let bytes: Vec<u8> = ctx.array::<i8>(src)?[off..off + len]
                .iter()
                .map(|&b| b as u8)
                .collect();
            append(ctx, this(a), &bytes)?;
            Ok(Value::Void)
        })
        .method("reset", "()V", |ctx, a| {
            let f = out_fields(ctx)?;
            ctx.set_field(this(a), f.count, Value::Int(0));
            Ok(Value::Void)
        })
        .method("toByteArray", "()[B", |ctx, a| {
            let bytes = written(ctx, this(a))?;
            Ok(ctx.new_byte_array(&bytes)?.into())
        })
        .method("size", "()I", |ctx, a| {
            let f = out_fields(ctx)?;
            Ok(ctx.get_field(this(a), f.count))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let bytes = written(ctx, this(a))?;
            let chars = default_charset(ctx).decode(&bytes);
            Ok(ctx.new_string_utf16(chars)?.into())
        })
        .method("close", "()V", |_, _| Ok(Value::Void))
}

#[cfg(test)]
mod tests {
    use crate::ctx::Throw;
    use crate::value::Value;
    use crate::{ArrayType, MemorySource, Vm, VmConfig};

    fn outcome(ctx: &mut crate::Ctx<'_>, result: crate::JResult<Value>) -> String {
        match result {
            Ok(v) => format!("{v:?}"),
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            Err(Throw::Stop) => "stopped".to_owned(),
        }
    }

    fn vm() -> Vm {
        Vm::new(
            VmConfig::default(),
            std::sync::Arc::new(MemorySource::new()),
            Vec::new(),
        )
    }

    /// What `read()` and `read(byte[], 0, 4)` give on a stream over ten
    /// bytes that starts at `offset`.
    fn reads(offset: i32) -> (String, String) {
        vm().attach("test", |ctx| {
            let buf = ctx.new_array(ArrayType::Byte, 10).unwrap();
            let args = [buf.into(), Value::Int(offset), Value::Int(10)];
            let stream = ctx
                .construct("java/io/ByteArrayInputStream", "([BII)V", &args)
                .unwrap();
            let dst = ctx.new_array(ArrayType::Byte, 4).unwrap();
            let one = ctx.call_virtual(stream, "read", "()I", &[]);
            let one = outcome(ctx, one);
            let args = [dst.into(), Value::Int(0), Value::Int(4)];
            let many = ctx.call_virtual(stream, "read", "([BII)I", &args);
            (one, outcome(ctx, many))
        })
    }

    #[test]
    fn a_negative_position_throws_instead_of_reading() {
        let (one, many) = reads(-5);
        assert!(
            one.starts_with("java.lang.ArrayIndexOutOfBoundsException"),
            "{one}"
        );
        assert!(
            many.starts_with("java.lang.ArrayIndexOutOfBoundsException"),
            "{many}"
        );
        assert_eq!(reads(2), ("Int(0)".to_owned(), "Int(4)".to_owned()));
    }

    #[test]
    fn the_output_buffer_grows_as_javas_does() {
        let (capacities, bytes) = vm().attach("test", |ctx| {
            let out = ctx
                .construct("java/io/ByteArrayOutputStream", "(I)V", &[Value::Int(2)])
                .unwrap();
            let f = super::out_fields(ctx).unwrap();
            let mut capacities = Vec::new();
            for b in 0..7 {
                ctx.call_virtual(out, "write", "(I)V", &[Value::Int(b)])
                    .unwrap();
                let buf = ctx.get_field(out, f.buf).obj().unwrap();
                capacities.push(ctx.array_len(buf).unwrap());
            }
            let arr = ctx.call_virtual(out, "toByteArray", "()[B", &[]).unwrap();
            let bytes = ctx.array::<i8>(arr.obj().unwrap()).unwrap().to_vec();
            (capacities, bytes)
        });
        assert_eq!(capacities, [2, 2, 4, 4, 8, 8, 8]);
        assert_eq!(bytes, [0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn a_count_past_the_buffer_throws() {
        let thrown = vm().attach("test", |ctx| {
            let out = ctx
                .construct("java/io/ByteArrayOutputStream", "()V", &[])
                .unwrap();
            let f = super::out_fields(ctx).unwrap();
            ctx.set_field(out, f.count, Value::Int(100));
            let result = ctx.call_virtual(out, "toByteArray", "()[B", &[]);
            outcome(ctx, result)
        });
        assert!(
            thrown.starts_with("java.lang.ArrayIndexOutOfBoundsException"),
            "{thrown}"
        );
    }
}
