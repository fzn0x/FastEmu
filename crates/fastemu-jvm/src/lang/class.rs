//! `java.lang.Class`: one object per loaded class, made on first use.

use std::sync::Arc;

use super::{string_value, throw};
use crate::class::{ClassId, ClassKind, MAX_CLASS_NAME, MAX_LOADABLE_NAME, short};
use crate::ctx::{Ctx, JResult};
use crate::descriptor::internal_name;
use crate::heap::Payload;
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};
use crate::vm::{Diagnostic, VmState};

/// The payload of a `Class` object: the class it stands for.
pub(crate) struct Mirror(pub(crate) ClassId);

impl Payload for Mirror {}

/// The `Class` object of `class`, made on first use and kept alive by the
/// class table.
pub(crate) fn mirror(state: &mut VmState, class: ClassId) -> ObjRef {
    if let Some(m) = state.classes.mirrors[class.index()] {
        return m;
    }
    let obj = state.alloc_instance(state.wk.class);
    state.heap.get_mut(obj).payload = Some(Box::new(Mirror(class)));
    state.classes.mirrors[class.index()] = Some(obj);
    obj
}

/// The class a `Class` object stands for.
pub(crate) fn mirrored(state: &VmState, obj: ObjRef) -> Option<ClassId> {
    state.heap.get(obj).payload::<Mirror>().map(|m| m.0)
}

fn class_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<ClassId> {
    let obj = ctx.non_null(v)?;
    Ok(mirrored(&ctx.state, obj).expect("a java.lang.Class object has a mirror payload"))
}

/// `Class.getName()`: dots for slashes; arrays keep their descriptors.
fn java_name(ctx: &Ctx<'_>, class: ClassId) -> String {
    ctx.class_name(class).replace('/', ".")
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/Class")
            .final_class()
            .payload_with(|| Box::new(Mirror(ClassId(0))))
            .method("getName", "()Ljava/lang/String;", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                let name = java_name(ctx, c);
                string_value(ctx, &name)
            })
            .method("toString", "()Ljava/lang/String;", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                let kind = if ctx.state.classes.get(c).is_interface() {
                    "interface "
                } else {
                    "class "
                };
                let name = java_name(ctx, c);
                string_value(ctx, &format!("{kind}{name}"))
            })
            .static_method("forName", "(Ljava/lang/String;)Ljava/lang/Class;", for_name)
            .method("newInstance", "()Ljava/lang/Object;", new_instance)
            .method("isInstance", "(Ljava/lang/Object;)Z", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                Ok(Value::from(
                    a[1].obj().is_some_and(|o| ctx.is_instance(o, c)),
                ))
            })
            .method("isAssignableFrom", "(Ljava/lang/Class;)Z", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                let other = class_arg(ctx, a[1])?;
                Ok(Value::from(ctx.is_assignable(other, c)))
            })
            .method("isArray", "()Z", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                Ok(Value::from(ctx.state.classes.get(c).is_array()))
            })
            .method("isInterface", "()Z", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                Ok(Value::from(ctx.state.classes.get(c).is_interface()))
            })
            .method("getSuperclass", "()Ljava/lang/Class;", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                let class = ctx.state.classes.get(c);
                if class.is_interface() {
                    return Ok(Value::NULL);
                }
                match class.superclass {
                    Some(s) => Ok(ctx.class_object(s).into()),
                    None => Ok(Value::NULL),
                }
            })
            .method("getComponentType", "()Ljava/lang/Class;", |ctx, a| {
                let c = class_arg(ctx, a[0])?;
                match ctx.state.classes.get(c).kind {
                    ClassKind::RefArray(elem) => Ok(ctx.class_object(elem).into()),
                    _ => Ok(Value::NULL),
                }
            })
            .method(
                "getResourceAsStream",
                "(Ljava/lang/String;)Ljava/io/InputStream;",
                get_resource_as_stream,
            ),
    ]
}

fn for_name(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let name_obj = ctx.non_null(a[0])?;
    // No class has a name this long, and the game's string is not copied,
    // not even into the message.
    let units = ctx.string_utf16(name_obj)?;
    let limit = if units.first() == Some(&u16::from(b'[')) {
        MAX_LOADABLE_NAME
    } else {
        MAX_CLASS_NAME
    };
    if units.len() > limit {
        let head = short(&String::from_utf16_lossy(&units[..limit]));
        return throw(ctx, "java/lang/ClassNotFoundException", &head);
    }
    let name = ctx.string(name_obj)?;
    let internal = internal_name(&name);
    // Class.forName takes binary names; an internal name with slashes is
    // not one. A class that exists but fails to link throws its linkage
    // error; only a missing one is ClassNotFoundException.
    let class = if name.contains('/') {
        None
    } else {
        match ctx.state.classes.load(&internal) {
            Ok(class) => Some(class),
            Err(e) if e.missing.as_deref() == Some(internal.as_str()) => None,
            Err(e) => return Err(ctx.load_error(e)),
        }
    };
    let Some(class) = class else {
        ctx.report(Diagnostic::MissingClass(internal));
        return throw(ctx, "java/lang/ClassNotFoundException", &short(&name));
    };
    ctx.initialize(class)?;
    Ok(ctx.class_object(class).into())
}

fn new_instance(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let class = class_arg(ctx, a[0])?;
    let c = Arc::clone(ctx.state.classes.get(class));
    if c.is_abstract() || c.is_interface() || c.is_array() {
        let name = c.name.replace('/', ".");
        return throw(ctx, "java/lang/InstantiationException", &name);
    }
    let Some(ctor) = c.declared_method("<init>", "()V").cloned() else {
        let name = c.name.replace('/', ".");
        return throw(ctx, "java/lang/InstantiationException", &name);
    };
    if ctor.is_private() {
        let name = c.name.replace('/', ".");
        return throw(ctx, "java/lang/IllegalAccessException", &name);
    }
    let obj = ctx.new_object(class)?;
    ctx.invoke(&ctor, &[obj.into()])?;
    Ok(obj.into())
}

/// `Class.getResourceAsStream`: a name without a leading `/` is relative
/// to the class's package.
fn get_resource_as_stream(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let class = class_arg(ctx, a[0])?;
    let name_obj = ctx.non_null(a[1])?;
    let name = ctx.string(name_obj)?;
    let path = match name.strip_prefix('/') {
        Some(absolute) => absolute.to_owned(),
        None => {
            let package = ctx.state.classes.get(class).package().to_owned();
            if package.is_empty() {
                name
            } else {
                format!("{package}/{name}")
            }
        }
    };
    let source = Arc::clone(ctx.state.classes.source());
    let bytes = ctx.without_lock(|| source.resource(&path));
    match bytes {
        Some(bytes) => Ok(crate::io::bytes::new_byte_array_stream(ctx, &bytes)?.into()),
        None => Ok(Value::NULL),
    }
}
