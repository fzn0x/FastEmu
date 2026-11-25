//! `com.siemens.mp.color_game.GameCanvas`: MIDP 2.0's `GameCanvas` for
//! Siemens' MIDP 1.0 colour phones, on the runtime's GameCanvas extension
//! points (`docs/architecture/midp.md`, "What GameCanvas uses").
//!
//! The off-screen buffer is a mutable image at the size of the whole
//! screen, kept in a private field. `flushGraphics` copies it before the
//! flush waits for its frame slot, so no image lock is held while the
//! collector may run.

use fastemu_gfx::Surface;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};

use super::layer::{GRAPHICS, IMAGE};
use crate::env::env;
use crate::lcdui::graphics::{GraphicsData, new_graphics};
use crate::lcdui::image::{new_image, reserve_pixels, surface_of};
use crate::lcdui::keys::{self, take_key_states};
use crate::lcdui::{Rect, canvas, display, displayable, throw_gfx};

const GAME_CANVAS: &str = "com/siemens/mp/color_game/GameCanvas";
const BUFFER: &str = "buffer";

/// The off-screen buffer of `this`.
fn buffer(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<ObjRef> {
    let field = ctx.field_id(GAME_CANVAS, BUFFER, IMAGE)?;
    match ctx.get_field(this, field).obj() {
        Some(image) => Ok(image),
        None => ctx.throw("java/lang/IllegalStateException", "no off-screen buffer"),
    }
}

fn init(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let (w, h) = {
        let profile = &env(ctx).profile;
        (profile.screen_width as i32, profile.screen_height as i32)
    };
    reserve_pixels(ctx, w.into(), h.into())?;
    let surface = Surface::new_mutable(w, h).map_err(|e| throw_gfx(ctx, &e))?;
    let image = new_image(ctx, surface)?;
    let field = ctx.field_id(GAME_CANVAS, BUFFER, IMAGE)?;
    ctx.set_field(this, field, image.into());
    displayable::data_mut(ctx, this)?.set_suppress_game_keys(a[1].boolean());
    Ok(Value::Void)
}

/// Shows `region` of the buffer on the canvas.
fn flush(ctx: &mut Ctx<'_>, this: Value, region: Option<Rect>) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let image = buffer(ctx, this)?;
    let copy = surface_of(ctx, image)?.lock().clone();
    let region =
        region.unwrap_or_else(|| Rect::new(0, 0, copy.width() as i32, copy.height() as i32));
    if region.width >= 1 && region.height >= 1 {
        canvas::flush(ctx, this, &copy, region)?;
    }
    Ok(Value::Void)
}

/// `paint(g)`: the buffer at the top left.
fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let g = ctx.non_null(a[1])?;
    let image = buffer(ctx, this)?;
    let source = surface_of(ctx, image)?;
    let d = ctx.payload::<GraphicsData>(g)?;
    let result = {
        let source = source.lock();
        d.state.draw_image(&mut d.target.lock(), &source, 0, 0, 0)
    };
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// The class.
pub(super) fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    let pressed = |action: i32| int(1 << action);
    NativeClass::new(GAME_CANVAS)
        .extends("javax/microedition/lcdui/Canvas")
        .abstract_class()
        .constant("UP_PRESSED", "I", pressed(keys::UP))
        .constant("DOWN_PRESSED", "I", pressed(keys::DOWN))
        .constant("LEFT_PRESSED", "I", pressed(keys::LEFT))
        .constant("RIGHT_PRESSED", "I", pressed(keys::RIGHT))
        .constant("FIRE_PRESSED", "I", pressed(keys::FIRE))
        .constant("GAME_A_PRESSED", "I", pressed(keys::GAME_A))
        .constant("GAME_B_PRESSED", "I", pressed(keys::GAME_B))
        .constant("GAME_C_PRESSED", "I", pressed(keys::GAME_C))
        .constant("GAME_D_PRESSED", "I", pressed(keys::GAME_D))
        .field_with(BUFFER, IMAGE, access::PRIVATE)
        .method_with("<init>", "(Z)V", access::PROTECTED, init)
        .method("flushGraphics", "()V", |ctx, a| flush(ctx, a[0], None))
        .method("flushGraphics", "(IIII)V", |ctx, a| {
            let region = Rect::new(a[1].int(), a[2].int(), a[3].int(), a[4].int());
            flush(ctx, a[0], Some(region))
        })
        .method_with(
            "getGraphics",
            &format!("(){GRAPHICS}"),
            access::PROTECTED,
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let image = buffer(ctx, this)?;
                let target = surface_of(ctx, image)?;
                Ok(new_graphics(ctx, target)?.into())
            },
        )
        .method("getKeyStates", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            if !display::is_shown(ctx, this) {
                return Ok(Value::Int(0));
            }
            Ok(Value::Int(take_key_states(ctx)))
        })
        .method("paint", &format!("({GRAPHICS})V"), paint)
}
