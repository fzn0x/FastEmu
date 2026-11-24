//! `javax.microedition.media` (MMAPI, JSR-135): players and controls on
//! the instance's sound group.

mod api;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    api::classes()
}
