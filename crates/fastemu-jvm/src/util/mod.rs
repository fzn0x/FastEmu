//! The CLDC 1.1 `java.util` classes, implemented in Rust: the
//! collections and their `Enumeration`, `Random`, `Date`, `TimeZone`
//! and `Calendar`, and MIDP's `Timer` and `TimerTask`.

mod calendar;
mod date;
mod hashtable;
mod keys;
mod random;
mod time;
mod timer;
mod timezone;
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
    all.extend(random::classes());
    all.extend(date::classes());
    all.extend(timezone::classes());
    all.extend(calendar::classes());
    all.extend(timer::classes());
    all
}
