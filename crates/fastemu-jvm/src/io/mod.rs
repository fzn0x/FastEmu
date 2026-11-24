//! The CLDC 1.1 `java.io` classes, implemented in Rust: the streams,
//! their byte array versions and `PrintStream`.

pub(crate) mod bytes;
pub(crate) mod print;
mod stream;

use crate::native::NativeClass;

/// Every `java.io` class but the exceptions, which live with the other
/// throwables.
pub(crate) fn classes() -> Vec<NativeClass> {
    let mut all = stream::classes();
    all.extend(bytes::classes());
    all.extend(print::classes());
    all
}
