//! `javax.microedition.lcdui.game`: GameCanvas, Layer, Sprite, TiledLayer
//! and LayerManager.

pub mod model;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    Vec::new()
}
