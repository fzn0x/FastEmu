//! `javax.microedition.lcdui`: the display, its screens and canvases,
//! graphics, images, fonts and commands.
//!
//! `docs/architecture/midp.md` describes the design: the event thread,
//! repaints and frames, input, soft keys, and the extension point the
//! high-level screens use ([`screen::ScreenHandler`]).

pub mod canvas;
pub mod command;
pub mod display;
pub mod displayable;
pub mod event;
pub mod font;
mod frame;
pub mod game;
pub mod graphics;
pub mod highlevel;
pub mod image;
pub mod input;
pub mod keys;
pub mod screen;
mod softkeys;
pub mod state;
pub mod theme;

use fastemu_gfx::GfxError;
use fastemu_jvm::{Ctx, NativeClass, Throw};

pub use state::Rect;

/// The exception a failed graphics call names, ready to return.
pub(crate) fn throw_gfx(ctx: &mut Ctx<'_>, e: &GfxError) -> Throw {
    ctx.new_exception(e.java_class(), e.message())
}

/// The classes of this module and its submodules.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = vec![
        displayable::class(),
        canvas::class(),
        screen::class(),
        display::class(),
        graphics::class(),
        image::class(),
        font::class(),
    ];
    classes.extend(command::classes());
    classes.extend(game::classes());
    classes.extend(highlevel::classes());
    classes
}
