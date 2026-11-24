//! `javax.microedition.io`: the Generic Connection Framework.
//!
//! `docs/architecture/connections.md` describes the design.

mod interfaces;

use fastemu_jvm::{NativeClass, ObjRef, Value};

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    interfaces::classes()
}

/// `this` of an instance native. The interpreter null-checks receivers,
/// so it is always present.
pub(crate) fn this(args: &[Value]) -> ObjRef {
    args[0]
        .obj()
        .expect("instance methods always have a receiver")
}
