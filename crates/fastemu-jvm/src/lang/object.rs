//! `java.lang.Object` and the small interfaces of `java.lang`.

use std::time::{Duration, Instant};

use super::{string_value, this, throw};
use crate::classfile::access;
use crate::ctx::{Ctx, JResult};
use crate::heap::{Body, HeapObject, identity_hash};
use crate::native::NativeClass;
use crate::value::Value;

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/Object")
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .method("hashCode", "()I", |_, a| {
                Ok(Value::Int(identity_hash(this(a))))
            })
            .method("equals", "(Ljava/lang/Object;)Z", |_, a| {
                Ok(Value::from(a[1].obj() == Some(this(a))))
            })
            .method("toString", "()Ljava/lang/String;", to_string)
            .method("getClass", "()Ljava/lang/Class;", |ctx, a| {
                let class = ctx.class_of(this(a));
                Ok(ctx.class_object(class).into())
            })
            .method("notify", "()V", |ctx, a| {
                ctx.monitor_notify(this(a), false)?;
                Ok(Value::Void)
            })
            .method("notifyAll", "()V", |ctx, a| {
                ctx.monitor_notify(this(a), true)?;
                Ok(Value::Void)
            })
            .method("wait", "()V", |ctx, a| wait(ctx, a, 0, 0))
            .method("wait", "(J)V", |ctx, a| wait(ctx, a, a[1].long(), 0))
            .method("wait", "(JI)V", |ctx, a| {
                wait(ctx, a, a[1].long(), a[2].int())
            })
            .method_with("clone", "()Ljava/lang/Object;", access::PROTECTED, clone)
            .method_with("finalize", "()V", access::PROTECTED, |_, _| Ok(Value::Void)),
        NativeClass::interface("java/lang/Runnable").abstract_method("run", "()V"),
        NativeClass::interface("java/lang/Cloneable"),
        NativeClass::interface("java/lang/CharSequence")
            .abstract_method("length", "()I")
            .abstract_method("charAt", "(I)C")
            .abstract_method("toString", "()Ljava/lang/String;"),
    ]
}

fn to_string(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let obj = this(a);
    let name = ctx.class_name(ctx.class_of(obj)).replace('/', ".");
    let hash = ctx.call_virtual(obj, "hashCode", "()I", &[])?.int();
    string_value(ctx, &format!("{name}@{:x}", hash as u32))
}

fn wait(ctx: &mut Ctx<'_>, a: &[Value], millis: i64, nanos: i32) -> JResult<Value> {
    if millis < 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "timeout value is negative",
        );
    }
    if !(0..=999_999).contains(&nanos) {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "nanosecond timeout value out of range",
        );
    }
    let millis = if nanos > 0 && millis < i64::MAX {
        millis + 1
    } else {
        millis
    };
    let deadline = (millis > 0).then(|| Instant::now() + Duration::from_millis(millis as u64));
    ctx.monitor_wait(this(a), deadline)?;
    Ok(Value::Void)
}

/// `Object.clone`: arrays copy their elements; other objects copy their
/// fields when their class implements `Cloneable`.
fn clone(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let obj = this(a);
    let class = ctx.class_of(obj);
    let source = ctx.state.heap.get(obj);
    let is_array = source.body.array_len().is_some();
    if !is_array {
        let cloneable = ctx.find_class("java/lang/Cloneable")?;
        let source = ctx.state.heap.get(obj);
        if !ctx.is_assignable(class, cloneable) || source.payload.is_some() {
            let name = ctx.class_name(class).replace('/', ".");
            return throw(ctx, "java/lang/CloneNotSupportedException", &name);
        }
    }
    let bytes = ctx.state.heap.get(obj).body.bytes();
    ctx.check_alloc(bytes)?;
    let body = match &ctx.state.heap.get(obj).body {
        Body::Fields(f) => Body::Fields(f.clone()),
        other => other.duplicate(),
    };
    let copy = ctx.state.heap.alloc(HeapObject::new(class, body, None));
    Ok(ctx.root(copy).into())
}
