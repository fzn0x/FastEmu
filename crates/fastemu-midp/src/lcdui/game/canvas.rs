//! `javax.microedition.lcdui.game.GameCanvas`: a canvas with an
//! off-screen buffer of the whole screen's size and polled key states.
//!
//! The buffer is a mutable `Image`, white at first, kept in a private
//! field so the collector counts and keeps it; `Canvas` owns the payload.
//! `getGraphics` hands out a new `Graphics` on it at every call,
//! `flushGraphics` copies it onto the canvas's back buffer and presents a
//! frame (waiting for the frame slot on a game thread), and `paint` draws
//! it, so a repaint shows what was last drawn. `getKeyStates` reads the
//! latched key states and is 0 while the canvas is not shown.

use std::sync::Arc;

use fastemu_gfx::Surface;
use fastemu_gfx::anchor::{LEFT, TOP};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};

use super::layer::graphics_arg;
use crate::lcdui::canvas::flush_shared;
use crate::lcdui::display::is_shown;
use crate::lcdui::displayable::data_mut;
use crate::lcdui::graphics::{GraphicsData, new_graphics};
use crate::lcdui::image::{SharedSurface, new_image, surface_of};
use crate::lcdui::keys::{self, take_key_states};
use crate::lcdui::state::state;
use crate::lcdui::{Rect, throw_gfx};

const CLASS: &str = "javax/microedition/lcdui/game/GameCanvas";
const GRAPHICS: &str = "Ljavax/microedition/lcdui/Graphics;";
const BUFFER: &str = "buffer";
const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";

/// The off-screen buffer of `canvas`.
fn buffer(ctx: &mut Ctx<'_>, canvas: ObjRef) -> JResult<SharedSurface> {
    let field = ctx.field_id(CLASS, BUFFER, IMAGE)?;
    match ctx.get_field(canvas, field).obj() {
        Some(image) => surface_of(ctx, image),
        None => ctx.throw(
            "java/lang/IllegalStateException",
            "the GameCanvas constructor has not run",
        ),
    }
}

/// `GameCanvas(suppressKeyEvents)`.
fn construct(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    data_mut(ctx, this)?.set_suppress_game_keys(a[1].boolean());
    let (width, height) = {
        let st = state(ctx);
        (st.screen_width, st.screen_height)
    };
    let surface = Surface::new_mutable(width, height).map_err(|e| throw_gfx(ctx, &e))?;
    let image = new_image(ctx, surface)?;
    let field = ctx.field_id(CLASS, BUFFER, IMAGE)?;
    ctx.set_field(this, field, image.into());
    Ok(Value::Void)
}

/// `paint(g)`: the buffer at the top left of `g`, through its
/// translation and clip.
fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let g = graphics_arg(ctx, a[1])?;
    let source = buffer(ctx, this)?;
    let d = ctx.payload::<GraphicsData>(g)?;
    let result = if Arc::ptr_eq(&source, &d.target) {
        // `paint(getGraphics())` draws the buffer on itself: use a copy.
        let mut target = d.target.lock();
        let copy = target.clone();
        d.state.draw_image(&mut target, &copy, 0, 0, TOP | LEFT)
    } else {
        let source = source.lock();
        d.state
            .draw_image(&mut d.target.lock(), &source, 0, 0, TOP | LEFT)
    };
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// `flushGraphics(x, y, width, height)`; nothing for an empty region.
fn flush_region(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let region = Rect::new(a[1].int(), a[2].int(), a[3].int(), a[4].int());
    if region.is_empty() {
        return Ok(Value::Void);
    }
    let source = buffer(ctx, this)?;
    flush_shared(ctx, this, &source, region)?;
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    let pressed = |action: i32| Value::Int(1 << action);
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Canvas")
        .abstract_class()
        .field_with(BUFFER, IMAGE, access::PRIVATE)
        .constant("UP_PRESSED", "I", pressed(keys::UP))
        .constant("DOWN_PRESSED", "I", pressed(keys::DOWN))
        .constant("LEFT_PRESSED", "I", pressed(keys::LEFT))
        .constant("RIGHT_PRESSED", "I", pressed(keys::RIGHT))
        .constant("FIRE_PRESSED", "I", pressed(keys::FIRE))
        .constant("GAME_A_PRESSED", "I", pressed(keys::GAME_A))
        .constant("GAME_B_PRESSED", "I", pressed(keys::GAME_B))
        .constant("GAME_C_PRESSED", "I", pressed(keys::GAME_C))
        .constant("GAME_D_PRESSED", "I", pressed(keys::GAME_D))
        .method_with("<init>", "(Z)V", access::PROTECTED, construct)
        .method_with(
            "getGraphics",
            &format!("(){GRAPHICS}"),
            access::PROTECTED,
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let target = buffer(ctx, this)?;
                Ok(new_graphics(ctx, target)?.into())
            },
        )
        .method("getKeyStates", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            if !is_shown(ctx, this) {
                return Ok(Value::Int(0));
            }
            Ok(Value::Int(take_key_states(ctx)))
        })
        .method("paint", &format!("({GRAPHICS})V"), paint)
        .method("flushGraphics", "(IIII)V", flush_region)
        .method("flushGraphics", "()V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let source = buffer(ctx, this)?;
            flush_shared(ctx, this, &source, Rect::new(0, 0, i32::MAX, i32::MAX))?;
            Ok(Value::Void)
        })
}
