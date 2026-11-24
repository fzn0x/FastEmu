//! The CLDC 1.1 `java.io` classes, implemented in Rust: the streams,
//! their byte array versions, the data streams, the readers and writers
//! and `PrintStream`.

mod access;
pub(crate) mod bytes;
mod data;
pub(crate) mod print;
mod stream;
mod text;

use crate::native::NativeClass;

/// Every `java.io` class but the exceptions, which live with the other
/// throwables.
pub(crate) fn classes() -> Vec<NativeClass> {
    let mut all = stream::classes();
    all.extend(bytes::classes());
    all.extend(data::classes());
    all.extend(text::classes());
    all.extend(print::classes());
    all
}
