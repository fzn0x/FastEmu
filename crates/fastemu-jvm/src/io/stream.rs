//! `java.io.InputStream` and `java.io.OutputStream`, the abstract bases
//! every stream extends. Their methods reach the subclass's own `read`
//! and `write` through virtual dispatch, so a game's stream works
//! wherever a library stream does.

use crate::ctx::JResult;
use crate::lang::{check_range, this, throw};
use crate::native::NativeClass;
use crate::value::Value;

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![input_stream(), output_stream()]
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
            let mut n = 0;
            while n < len {
                let b = ctx.call_virtual(this(a), "read", "()I", &[])?.int();
                if b < 0 {
                    break;
                }
                ctx.array_mut::<i8>(arr)?[off + n] = b as i8;
                n += 1;
            }
            Ok(Value::Int(if n == 0 { -1 } else { n as i32 }))
        })
        .method("skip", "(J)J", |ctx, a| {
            let mut skipped = 0i64;
            while skipped < a[1].long() {
                if ctx.call_virtual(this(a), "read", "()I", &[])?.int() < 0 {
                    break;
                }
                skipped += 1;
            }
            Ok(Value::Long(skipped))
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
