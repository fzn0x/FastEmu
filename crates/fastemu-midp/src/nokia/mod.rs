//! Nokia's UI API (`com.nokia.mid.ui`) and sound (`com.nokia.mid.sound`).

pub mod sound;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = Vec::new();
    classes.extend(sound::classes());
    classes
}
