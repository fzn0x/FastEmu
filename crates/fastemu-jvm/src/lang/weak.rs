//! `java.lang.ref.Reference` and `java.lang.ref.WeakReference`. The
//! referent lives in the VM's weak table, which the collector does not
//! treat as a root and clears when the referent dies.

use super::this;
use crate::native::NativeClass;
use crate::value::Value;

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/ref/Reference")
            .abstract_class()
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .method("get", "()Ljava/lang/Object;", |ctx, a| {
                let referent = ctx.state.weak.get(&this(a)).copied().flatten();
                Ok(match referent {
                    Some(r) => ctx.root(r).into(),
                    None => Value::NULL,
                })
            })
            .method("clear", "()V", |ctx, a| {
                ctx.state.weak.insert(this(a), None);
                Ok(Value::Void)
            }),
        NativeClass::new("java/lang/ref/WeakReference")
            .extends("java/lang/ref/Reference")
            .method("<init>", "(Ljava/lang/Object;)V", |ctx, a| {
                ctx.state.weak.insert(this(a), a[1].obj());
                Ok(Value::Void)
            }),
    ]
}
