//! `javax.microedition.lcdui`: the display, its screens and canvases,
//! graphics, images, fonts and commands.

pub mod font;
pub mod game;
pub mod graphics;
pub mod highlevel;
pub mod image;
mod state;

use fastemu_gfx::GfxError;
use fastemu_jvm::{Ctx, NativeClass, Throw};

/// The exception a failed graphics call names, ready to return.
pub(crate) fn throw_gfx(ctx: &mut Ctx<'_>, e: &GfxError) -> Throw {
    ctx.new_exception(e.java_class(), e.message())
}

/// The classes of this module and its submodules.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = vec![graphics::class(), image::class(), font::class()];
    classes.extend(game::classes());
    classes.extend(highlevel::classes());
    classes
}
