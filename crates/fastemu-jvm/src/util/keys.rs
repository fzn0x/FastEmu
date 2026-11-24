//! `equals` as the collections call it: through the game's own
//! overrides, but computed in Rust for the elements games use most
//! (strings, boxed numbers, and objects that keep `Object`'s identity),
//! which no game code can change.

use crate::class::ClassId;
use crate::ctx::{Ctx, JResult};
use crate::heap::Body;
use crate::lang::string;
use crate::value::{ObjRef, Value};

/// The value of a boxed primitive, as its `equals` compares it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Boxed {
    /// `Integer`, `Short`, `Byte` and `Character`.
    Int(i32),
    Long(i64),
    Bool(bool),
    /// `Float`, by `floatToIntBits`: one NaN, and 0.0 apart from -0.0.
    Float(u32),
    /// `Double`, by `doubleToLongBits`.
    Double(u64),
}

/// The class and value of `obj` if it is a boxed primitive.
fn boxed(ctx: &Ctx<'_>, obj: ObjRef) -> Option<(ClassId, Boxed)> {
    let class = ctx.class_of(obj);
    let c = ctx.state.classes.get(class);
    let field = |desc: &str| {
        let f = c.declared_field("value", desc)?;
        match &ctx.state.heap.get(obj).body {
            Body::Fields(values) => values.get(f.slot as usize).copied(),
            _ => None,
        }
    };
    let value = match &*c.name {
        "java/lang/Integer" => Boxed::Int(field("I")?.int()),
        "java/lang/Short" => Boxed::Int(field("S")?.int()),
        "java/lang/Byte" => Boxed::Int(field("B")?.int()),
        "java/lang/Character" => Boxed::Int(field("C")?.int()),
        "java/lang/Boolean" => Boxed::Bool(field("Z")?.boolean()),
        "java/lang/Long" => Boxed::Long(field("J")?.long()),
        "java/lang/Float" => {
            let v = field("F")?.float();
            Boxed::Float(if v.is_nan() { 0x7FC0_0000 } else { v.to_bits() })
        }
        "java/lang/Double" => {
            let v = field("D")?.double();
            let bits = if v.is_nan() {
                0x7FF8_0000_0000_0000
            } else {
                v.to_bits()
            };
            Boxed::Double(bits)
        }
        _ => return None,
    };
    Some((class, value))
}

/// Whether `obj` keeps `java.lang.Object`'s own method `name`, which
/// compares and hashes by identity.
fn keeps_object_method(ctx: &Ctx<'_>, obj: ObjRef, name: &str, desc: &str) -> bool {
    let classes = &ctx.state.classes;
    let Some(selector) = classes.find_selector(name, desc) else {
        return false;
    };
    let class = classes.get(ctx.class_of(obj));
    class
        .virtual_method(selector)
        .is_some_and(|m| Some(m.class) == classes.lookup("java/lang/Object"))
}

/// `a.equals(b)`.
pub(crate) fn equals(ctx: &mut Ctx<'_>, a: ObjRef, b: Option<ObjRef>) -> JResult<bool> {
    if let Some(chars) = string::chars(&ctx.state, a) {
        return Ok(b.is_some_and(|b| string::chars(&ctx.state, b) == Some(chars)));
    }
    if let Some((class, value)) = boxed(ctx, a) {
        return Ok(b.and_then(|b| boxed(ctx, b)) == Some((class, value)));
    }
    if keeps_object_method(ctx, a, "equals", "(Ljava/lang/Object;)Z") {
        return Ok(b == Some(a));
    }
    let arg = b.map_or(Value::NULL, Value::from);
    Ok(ctx
        .call_virtual(a, "equals", "(Ljava/lang/Object;)Z", &[arg])?
        .boolean())
}
