//! `java.io.ByteArrayInputStream`, the stream `Class.getResourceAsStream`
//! returns, over a `byte[]` in protected fields a subclass may change.

use crate::ctx::{Ctx, FieldId, JResult};
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const BAIS: &str = "java/io/ByteArrayInputStream";

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

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(BAIS)
            .extends("java/io/InputStream")
            .field_with("buf", "[B", crate::classfile::access::PROTECTED)
            .field_with("pos", "I", crate::classfile::access::PROTECTED)
            .field_with("mark", "I", crate::classfile::access::PROTECTED)
            .field_with("count", "I", crate::classfile::access::PROTECTED)
            .method("<init>", "([B)V", |ctx, a| init(ctx, a, 0, None))
            .method("<init>", "([BII)V", |ctx, a| {
                init(ctx, a, a[2].int(), Some(a[3].int()))
            })
            .method("read", "()I", |ctx, a| {
                let f = fields(ctx)?;
                let obj = this(a);
                let pos = ctx.get_field(obj, f.pos).int();
                let count = ctx.get_field(obj, f.count).int();
                if pos >= count {
                    return Ok(Value::Int(-1));
                }
                let Some(buf) = ctx.get_field(obj, f.buf).obj() else {
                    return Ok(Value::Int(-1));
                };
                let at = source_range(ctx, buf, pos, 1)?;
                let b = ctx.array::<i8>(buf)?[at];
                ctx.set_field(obj, f.pos, Value::Int(pos + 1));
                Ok(Value::Int(i32::from(b as u8)))
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
                let f = fields(ctx)?;
                let obj = this(a);
                let pos = ctx.get_field(obj, f.pos).int();
                let count = ctx.get_field(obj, f.count).int();
                if pos >= count {
                    return Ok(Value::Int(-1));
                }
                let n = len.min((count - pos) as usize);
                if n == 0 {
                    return Ok(Value::Int(0));
                }
                let Some(buf) = ctx.get_field(obj, f.buf).obj() else {
                    return Ok(Value::Int(-1));
                };
                let at = source_range(ctx, buf, pos, n)?;
                let src: Vec<i8> = ctx.array::<i8>(buf)?[at..at + n].to_vec();
                ctx.array_mut::<i8>(dst)?[off..off + n].copy_from_slice(&src);
                ctx.set_field(obj, f.pos, Value::Int(pos + n as i32));
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
            .method("close", "()V", |_, _| Ok(Value::Void)),
    ]
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

    /// What `read()` and `read(byte[], 0, 4)` give on a stream over ten
    /// bytes that starts at `offset`.
    fn reads(offset: i32) -> (String, String) {
        let vm = Vm::new(
            VmConfig::default(),
            std::sync::Arc::new(MemorySource::new()),
            Vec::new(),
        );
        vm.attach("test", |ctx| {
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
}
