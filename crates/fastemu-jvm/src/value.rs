//! Values as the virtual machine stores them: in local variables, on the
//! operand stack, in fields and as arguments to native methods.

use std::fmt;
use std::num::NonZeroU32;

/// A reference to an object on one virtual machine's heap.
///
/// It is a handle (an index into the heap's slab), never a pointer, so it is
/// `Copy`, `Send` and cheap to store. It stays valid while the object is
/// reachable from a garbage collection root; see the rooting rules in
/// `docs/architecture/jvm.md`. A handle is meaningful only to the virtual
/// machine that made it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjRef(NonZeroU32);

impl ObjRef {
    pub(crate) fn from_index(index: usize) -> Self {
        let raw = u32::try_from(index + 1).expect("heap slab index fits in u32");
        Self(NonZeroU32::new(raw).expect("index + 1 is never zero"))
    }

    pub(crate) fn index(self) -> usize {
        self.0.get() as usize - 1
    }

    /// The raw handle, for host-side maps keyed by object.
    pub fn to_raw(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Debug for ObjRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjRef({})", self.0)
    }
}

/// One JVM value.
///
/// Locals and the operand stack hold one `Value` per slot, and a `long` or
/// `double` takes two slots, as the JVM specification counts them: the
/// value itself followed by a [`Value::Void`]. Fields, array elements and
/// native method arguments hold one `Value` per Java value.
///
/// `boolean`, `byte`, `char` and `short` are carried as [`Value::Int`], as
/// the JVM does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Value {
    /// No value: the second slot of a `long` or `double`, an unset local,
    /// or the result of a `void` method.
    #[default]
    Void,
    /// An `int`, or a `boolean`, `byte`, `char` or `short`.
    Int(i32),
    /// A `long`.
    Long(i64),
    /// A `float`.
    Float(f32),
    /// A `double`.
    Double(f64),
    /// A reference, `None` for `null`.
    Ref(Option<ObjRef>),
    /// A `jsr` return address: the instruction index to continue at.
    RetAddr(u32),
}

impl Value {
    /// The `null` reference.
    pub const NULL: Value = Value::Ref(None);

    /// The `int` this value holds, or 0 when it holds something else
    /// (verified bytecode never asks for the wrong type).
    #[inline]
    pub fn int(self) -> i32 {
        match self {
            Value::Int(v) => v,
            _ => mismatch(self, 0),
        }
    }

    /// The `long` this value holds, or 0.
    #[inline]
    pub fn long(self) -> i64 {
        match self {
            Value::Long(v) => v,
            _ => mismatch(self, 0),
        }
    }

    /// The `float` this value holds, or 0.
    #[inline]
    pub fn float(self) -> f32 {
        match self {
            Value::Float(v) => v,
            _ => mismatch(self, 0.0),
        }
    }

    /// The `double` this value holds, or 0.
    #[inline]
    pub fn double(self) -> f64 {
        match self {
            Value::Double(v) => v,
            _ => mismatch(self, 0.0),
        }
    }

    /// The reference this value holds; `None` for `null` and for values
    /// that are not references.
    #[inline]
    pub fn obj(self) -> Option<ObjRef> {
        match self {
            Value::Ref(r) => r,
            _ => mismatch(self, None),
        }
    }

    /// The `int` this value holds, or 0, with no check in debug builds:
    /// for `iload` and `istore`, which crafted code can point at a slot
    /// of another type.
    #[inline]
    pub(crate) fn int_or_zero(self) -> i32 {
        if let Value::Int(v) = self { v } else { 0 }
    }

    /// The `long` this value holds, or 0, as [`Value::int_or_zero`].
    #[inline]
    pub(crate) fn long_or_zero(self) -> i64 {
        if let Value::Long(v) = self { v } else { 0 }
    }

    /// The `float` this value holds, or 0, as [`Value::int_or_zero`].
    #[inline]
    pub(crate) fn float_or_zero(self) -> f32 {
        if let Value::Float(v) = self { v } else { 0.0 }
    }

    /// The `double` this value holds, or 0, as [`Value::int_or_zero`].
    #[inline]
    pub(crate) fn double_or_zero(self) -> f64 {
        if let Value::Double(v) = self { v } else { 0.0 }
    }

    /// A `boolean` argument: any non-zero `int` is true.
    #[inline]
    pub fn boolean(self) -> bool {
        self.int() != 0
    }

    /// A `char` argument.
    #[inline]
    pub fn char(self) -> u16 {
        self.int() as u16
    }

    /// Whether this value is the `null` reference.
    pub fn is_null(self) -> bool {
        matches!(self, Value::Ref(None))
    }

    /// Whether this value takes two slots on the operand stack.
    pub fn is_wide(self) -> bool {
        matches!(self, Value::Long(_) | Value::Double(_))
    }
}

#[inline]
#[track_caller]
fn mismatch<T>(value: Value, fallback: T) -> T {
    debug_assert!(
        matches!(value, Value::Void),
        "value of the wrong type: {value:?}"
    );
    fallback
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::Long(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Self {
        Value::Float(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Double(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Int(i32::from(v))
    }
}

impl From<ObjRef> for Value {
    fn from(r: ObjRef) -> Self {
        Value::Ref(Some(r))
    }
}

impl From<Option<ObjRef>> for Value {
    fn from(r: Option<ObjRef>) -> Self {
        Value::Ref(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_round_trip_their_index() {
        let r = ObjRef::from_index(41);
        assert_eq!(r.index(), 41);
        assert_eq!(r.to_raw(), 42);
        assert_eq!(std::mem::size_of::<Option<ObjRef>>(), 4);
    }

    #[test]
    fn values_stay_small() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
    }

    #[test]
    fn accessors_read_their_type() {
        assert_eq!(Value::from(7).int(), 7);
        assert_eq!(Value::from(true).int(), 1);
        assert!(Value::from(-3i64).is_wide());
        assert!(Value::NULL.is_null());
        assert_eq!(Value::NULL.obj(), None);
        assert_eq!(Value::Void.int(), 0);
    }
}
