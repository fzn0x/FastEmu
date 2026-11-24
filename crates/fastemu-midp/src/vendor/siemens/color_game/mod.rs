//! `com.siemens.mp.color_game`: Siemens' copy of MIDP 2.0's game package
//! for its MIDP 1.0 colour phones. It behaves as MIDP's `GameCanvas`,
//! `Layer`, `LayerManager`, `Sprite` and `TiledLayer` do, without the
//! sprites' reference pixel and transforms, and lets games subclass
//! `Layer`.

mod game_canvas;
mod layer;
mod layer_manager;
mod sprite;
mod tiled_layer;

use fastemu_jvm::NativeClass;

/// The classes of the package.
pub fn classes() -> Vec<NativeClass> {
    vec![
        game_canvas::class(),
        layer::class(),
        layer_manager::class(),
        sprite::class(),
        tiled_layer::class(),
    ]
}
