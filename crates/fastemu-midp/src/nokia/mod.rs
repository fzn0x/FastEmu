//! Nokia's UI API (`com.nokia.mid.ui`) and sound (`com.nokia.mid.sound`),
//! on the profiles whose phones had them (see
//! [`crate::vendor::Vendor::has_nokia_ui`]).
//!
//! `FullCanvas` is a full-screen `Canvas` whose soft keys reach the game,
//! `DirectUtils` makes mutable and transparent images, `DirectGraphics`
//! is MIDP's `Graphics` itself with Nokia's calls added
//! ([`direct_graphics::implement`]), and `DeviceControl` logs light and
//! vibration requests. `docs/architecture/vendors.md` has the details.

pub mod device_control;
pub mod direct_graphics;
pub mod direct_utils;
pub mod full_canvas;
pub mod sound;

use fastemu_jvm::NativeClass;

/// The classes of this module. MIDP's `Graphics` gets the
/// `DirectGraphics` methods separately, through
/// [`direct_graphics::implement`].
pub fn classes() -> Vec<NativeClass> {
    let mut classes = vec![
        full_canvas::class(),
        direct_graphics::interface(),
        direct_utils::class(),
        device_control::class(),
    ];
    classes.extend(sound::classes());
    classes
}
