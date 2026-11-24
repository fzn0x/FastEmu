//! `javax.microedition.rms`: record stores, saved in the instance's
//! folder.

pub mod disk;
pub mod format;
pub mod name;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    Vec::new()
}
