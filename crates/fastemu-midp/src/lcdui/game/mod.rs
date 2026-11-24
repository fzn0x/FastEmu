//! `javax.microedition.lcdui.game`: GameCanvas, Layer, Sprite, TiledLayer
//! and LayerManager.
//!
//! The rules of the layers live in [`model`], plain Rust with no virtual
//! machine; the classes here keep a model in each object's payload and
//! draw through `fastemu-gfx`'s `drawRegion`. `docs/architecture/midp.md`
//! describes how they fit the runtime.

mod canvas;
mod layer;
mod manager;
pub mod model;
mod sprite;
mod tiled;

use fastemu_jvm::NativeClass;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    vec![
        canvas::class(),
        layer::class(),
        sprite::class(),
        tiled::class(),
        manager::class(),
    ]
}
