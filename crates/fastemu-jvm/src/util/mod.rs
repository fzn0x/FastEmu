//! The CLDC 1.1 `java.util` classes, implemented in Rust: the
//! collections and their `Enumeration`.

mod hashtable;
mod keys;
mod vector;

use crate::native::NativeClass;

/// Every `java.util` class but the exceptions, which live with the other
/// throwables.
pub(crate) fn classes() -> Vec<NativeClass> {
    let mut all = vec![
        NativeClass::interface("java/util/Enumeration")
            .abstract_method("hasMoreElements", "()Z")
            .abstract_method("nextElement", "()Ljava/lang/Object;"),
    ];
    all.extend(vector::classes());
    all.extend(hashtable::classes());
    all
}
