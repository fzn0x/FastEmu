//! The CLDC 1.1 `java.lang` classes, implemented in Rust as
//! [`NativeClass`]es, and the helpers the other library modules share.
//!
//! Where CLDC and Java SE differ in small ways, these classes follow what
//! `javac --release 8` output expects, so the same programs run here and
//! on a desktop JVM: `StringBuilder` exists, `Integer` extends `Number`,
//! `String` implements `CharSequence`.

pub(crate) mod class;
mod dtoa;
pub(crate) mod float_fmt;
pub(crate) mod math;
pub(crate) mod number;
pub(crate) mod object;
pub(crate) mod strbuf;
pub(crate) mod string;
pub(crate) mod system;
pub(crate) mod thread;
pub(crate) mod throwable;
pub(crate) mod weak;

use crate::ctx::{Ctx, JResult};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

/// Every `java.lang` class.
pub(crate) fn classes() -> Vec<NativeClass> {
    let mut all = Vec::new();
    all.extend(object::classes());
    all.extend(class::classes());
    all.extend(string::classes());
    all.extend(strbuf::classes());
    all.extend(number::classes());
    all.extend(math::classes());
    all.extend(system::classes());
    all.extend(thread::classes());
    all.extend(throwable::classes());
    all.extend(weak::classes());
    all
}

/// `this` of an instance native. The interpreter null-checks receivers,
/// so it is always present.
#[inline]
pub(crate) fn this(args: &[Value]) -> ObjRef {
    args[0].obj().expect("instance methods have a receiver")
}

/// Throws `class` with `message`.
pub(crate) fn throw<T>(ctx: &mut Ctx<'_>, class: &str, message: &str) -> JResult<T> {
    Err(ctx.new_exception(class, Some(message)))
}

/// A Java `String` made from `s`, as a return value.
pub(crate) fn string_value(ctx: &mut Ctx<'_>, s: &str) -> JResult<Value> {
    Ok(ctx.new_string(s)?.into())
}

/// Checks `[offset, offset + len)` against an array or string of length
/// `size`, throwing `exception` when it does not fit.
pub(crate) fn check_range(
    ctx: &mut Ctx<'_>,
    exception: &str,
    offset: i32,
    len: i32,
    size: usize,
) -> JResult<(usize, usize)> {
    if offset < 0 || len < 0 || offset as i64 + len as i64 > size as i64 {
        let message = format!("offset {offset}, count {len}, length {size}");
        return throw(ctx, exception, &message);
    }
    Ok((offset as usize, len as usize))
}
