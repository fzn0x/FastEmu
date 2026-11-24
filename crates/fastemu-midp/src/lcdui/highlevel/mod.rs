//! The high-level screens and items of `javax.microedition.lcdui`: Form
//! and its items, List, TextBox, Alert and Ticker.

pub mod draw;
pub mod text;
pub mod ticker;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    vec![ticker::class()]
}
