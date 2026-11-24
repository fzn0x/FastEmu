//! `javax.microedition.lcdui`: the display, its screens and canvases,
//! graphics, images, fonts and commands.

pub mod game;
pub mod highlevel;

use fastemu_jvm::NativeClass;

/// The classes of this module and its submodules.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = Vec::new();
    classes.extend(game::classes());
    classes.extend(highlevel::classes());
    classes
}
