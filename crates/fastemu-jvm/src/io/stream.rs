//! `java.io.InputStream` and `java.io.OutputStream`, the abstract bases
//! every stream extends. Their methods reach the subclass's own `read`
//! and `write` through virtual dispatch, so a game's stream works
//! wherever a library stream does.

use crate::class::ArrayType;
use crate::ctx::{Ctx, JResult, Throw};
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

/// The most `InputStream.skip` reads at once, as in Java.
const SKIP_BUFFER: i64 = 2048;

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![input_stream(), output_stream()]
}

/// Whether the throwable `e` is a `java.io.IOException`.
pub(crate) fn is_io_exception(ctx: &mut Ctx<'_>, e: ObjRef) -> bool {
    ctx.find_class("java/io/IOException")
        .is_ok_and(|io| ctx.is_instance(e, io))
}

fn input_stream() -> NativeClass {
    NativeClass::new("java/io/InputStream")
        .abstract_class()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .abstract_method("read", "()I")
        .method("read", "([B)I", |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)? as i32;
            ctx.call_virtual(
                this(a),
                "read",
                "([BII)I",
                &[arr.into(), Value::Int(0), Value::Int(len)],
            )
        })
        .method("read", "([BII)I", |ctx, a| {
            let arr = ctx.non_null(a[1])?;
            let size = ctx.array_len(arr)?;
            let (off, len) = check_range(
                ctx,
                "java/lang/IndexOutOfBoundsException",
                a[2].int(),
                a[3].int(),
                size,
            )?;
            if len == 0 {
                return Ok(Value::Int(0));
            }
            let first = ctx.call_virtual(this(a), "read", "()I", &[])?.int();
            if first < 0 {
                return Ok(Value::Int(-1));
            }
            ctx.array_mut::<i8>(arr)?[off] = first as i8;
            let mut n = 1;
            while n < len {
                // As in Java, an IOException after the first byte ends the
                // read early instead of losing the bytes already read.
                let b = match ctx.call_virtual(this(a), "read", "()I", &[]) {
                    Ok(b) => b.int(),
                    Err(Throw::Exception(e)) if is_io_exception(ctx, e) => break,
                    Err(e) => return Err(e),
                };
                if b < 0 {
                    break;
                }
                ctx.array_mut::<i8>(arr)?[off + n] = b as i8;
                n += 1;
            }
            Ok(Value::Int(n as i32))
        })
        .method("skip", "(J)J", |ctx, a| {
            // Java skips by reading into a scratch buffer, through the
            // stream's own read(byte[], int, int).
            let n = a[1].long();
            if n <= 0 {
                return Ok(Value::Long(0));
            }
            let size = n.min(SKIP_BUFFER) as i32;
            let scratch = ctx.new_array(ArrayType::Byte, size)?;
            let mut remaining = n;
            while remaining > 0 {
                let chunk = Value::Int(remaining.min(i64::from(size)) as i32);
                let args = [scratch.into(), Value::Int(0), chunk];
                let read = ctx.call_virtual(this(a), "read", "([BII)I", &args)?.int();
                if read < 0 {
                    break;
                }
                remaining -= i64::from(read);
            }
            Ok(Value::Long(n - remaining))
        })
        .method("available", "()I", |_, _| Ok(Value::Int(0)))
        .method("close", "()V", |_, _| Ok(Value::Void))
        .method("mark", "(I)V", |_, _| Ok(Value::Void))
        .method("reset", "()V", |ctx, _| {
            throw(ctx, "java/io/IOException", "mark/reset not supported")
        })
        .method("markSupported", "()Z", |_, _| Ok(Value::from(false)))
}

fn output_stream() -> NativeClass {
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
        .method("write", "([BII)V", |ctx, a| -> JResult<Value> {
            let arr = ctx.non_null(a[1])?;
            let len = ctx.array_len(arr)?;
            let (off, n) = check_range(
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
        .method("close", "()V", |_, _| Ok(Value::Void))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::ctx::Throw;
    use crate::{ArrayType, MemorySource, NativeClass, Value, Vm, VmConfig};

    /// A stream of the bytes 1, 2, 3, ... that throws `IOException` in
    /// place of byte `fail_at`, and counts the `read(byte[], int, int)`
    /// calls `skip` makes.
    fn counting_stream(fail_at: i32) -> NativeClass {
        NativeClass::new("test/Counting")
            .extends("java/io/InputStream")
            .field("next", "I")
            .field("bulk", "I")
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .method("read", "()I", move |ctx, a| {
                let obj = a[0].obj().unwrap();
                let f = ctx.field_id("test/Counting", "next", "I")?;
                let next = ctx.get_field(obj, f).int() + 1;
                ctx.set_field(obj, f, Value::Int(next));
                if next == fail_at {
                    return ctx.throw("java/io/IOException", "broken");
                }
                Ok(Value::Int(next))
            })
            .method("read", "([BII)I", |ctx, a| {
                let obj = a[0].obj().unwrap();
                let f = ctx.field_id("test/Counting", "bulk", "I")?;
                let calls = ctx.get_field(obj, f).int();
                ctx.set_field(obj, f, Value::Int(calls + 1));
                let args = [a[1], a[2], a[3]];
                ctx.call_special("java/io/InputStream", obj, "read", "([BII)I", &args)
            })
    }

    #[test]
    fn an_io_exception_after_the_first_byte_ends_the_read() {
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, vec![counting_stream(3)]);
        let (n, bytes, again) = vm.attach("test", |ctx| {
            let stream = ctx.construct("test/Counting", "()V", &[]).unwrap();
            let buf = ctx.new_array(ArrayType::Byte, 5).unwrap();
            let args = [buf.into(), Value::Int(0), Value::Int(5)];
            let n = ctx.call_virtual(stream, "read", "([BII)I", &args).unwrap();
            let bytes = ctx.array::<i8>(buf).unwrap().to_vec();
            // The next read starts after the failed byte.
            let again = ctx.call_virtual(stream, "read", "()I", &[]).unwrap();
            (n, bytes, again)
        });
        assert_eq!(n, Value::Int(2));
        assert_eq!(bytes, [1, 2, 0, 0, 0]);
        assert_eq!(again, Value::Int(4));
    }

    #[test]
    fn an_io_exception_on_the_first_byte_is_thrown() {
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, vec![counting_stream(1)]);
        let thrown = vm.attach("test", |ctx| {
            let stream = ctx.construct("test/Counting", "()V", &[]).unwrap();
            let buf = ctx.new_array(ArrayType::Byte, 5).unwrap();
            let args = [buf.into(), Value::Int(0), Value::Int(5)];
            match ctx.call_virtual(stream, "read", "([BII)I", &args) {
                Err(Throw::Exception(e)) => ctx.describe_exception(e),
                other => format!("{other:?}"),
            }
        });
        assert_eq!(thrown, "java.io.IOException: broken");
    }

    #[test]
    fn skip_reads_through_the_bulk_read_in_chunks() {
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, vec![counting_stream(-1)]);
        let (skipped, calls, next) = vm.attach("test", |ctx| {
            let stream = ctx.construct("test/Counting", "()V", &[]).unwrap();
            let skipped = ctx
                .call_virtual(stream, "skip", "(J)J", &[Value::Long(5000)])
                .unwrap();
            let f = ctx.field_id("test/Counting", "bulk", "I").unwrap();
            let calls = ctx.get_field(stream, f);
            let none = ctx
                .call_virtual(stream, "skip", "(J)J", &[Value::Long(-3)])
                .unwrap();
            assert_eq!(none, Value::Long(0));
            let next = ctx.call_virtual(stream, "read", "()I", &[]).unwrap();
            (skipped, calls, next)
        });
        assert_eq!(skipped, Value::Long(5000));
        assert_eq!(calls, Value::Int(3));
        assert_eq!(next, Value::Int(5001));
    }
}
