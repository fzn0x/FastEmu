//! `javax.microedition.lcdui.Canvas`: the screen games draw themselves,
//! its repaint requests and its back buffer.
//!
//! Each canvas keeps a screen-sized back buffer that survives between
//! paints, since games repaint only what changed. `repaint` grows one
//! dirty rectangle; the event thread paints it with the clip set to that
//! rectangle and then presents a frame. `serviceRepaints` paints at once
//! on the event thread and, on any other thread, takes the callback lock
//! and paints on the calling thread, as J2ME Loader does; when the lock is
//! not free within a second it logs a warning and paints anyway.

use std::sync::Arc;
use std::time::Duration;

use fastemu_gfx::Surface;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};
use parking_lot::Mutex;

use super::display::{self, is_shown};
use super::displayable::{self, DisplayableData, data, data_mut};
use super::event::{acquire, release};
use super::frame;
use super::graphics::{GraphicsData, new_graphics};
use super::image::SharedSurface;
use super::keys;
use super::state::{Rect, on_event_thread, report, state, warn_once};
use crate::env::env;

const CLASS: &str = "javax/microedition/lcdui/Canvas";
const PAINT: &str = "(Ljavax/microedition/lcdui/Graphics;)V";

/// How long `serviceRepaints` on a game thread waits for the event thread
/// to finish a callback before it paints anyway.
const SERVICE_TIMEOUT: Duration = Duration::from_secs(1);

/// The back buffer of a canvas (or the content of a screen), made on first
/// use at the size of the whole screen, so a switch to full-screen mode
/// keeps every pixel.
pub fn buffer(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<SharedSurface> {
    if let Some(buffer) = &data(ctx, obj)?.buffer {
        return Ok(Arc::clone(buffer));
    }
    let (w, h) = {
        let st = state(ctx);
        (st.screen_width as u32, st.screen_height as u32)
    };
    let surface = Surface::screen(w, h).map_err(|e| super::throw_gfx(ctx, &e))?;
    let buffer = Arc::new(Mutex::new(surface));
    data_mut(ctx, obj)?.buffer = Some(Arc::clone(&buffer));
    Ok(buffer)
}

/// The `Graphics` a canvas's `paint` gets, made once and reset before
/// each paint.
fn paint_graphics(ctx: &mut Ctx<'_>, canvas: ObjRef) -> JResult<ObjRef> {
    if let Some(g) = data(ctx, canvas)?.graphics {
        return Ok(ctx.root(g));
    }
    let target = buffer(ctx, canvas)?;
    let g = new_graphics(ctx, target)?;
    data_mut(ctx, canvas)?.graphics = Some(g);
    Ok(g)
}

/// Calls the canvas's `paint` for `dirty` with a fresh graphics state and
/// the clip on that region. The caller holds the callback lock; an
/// exception from `paint` is logged.
pub(crate) fn paint(ctx: &mut Ctx<'_>, canvas: ObjRef, dirty: Rect) -> JResult<()> {
    let (w, h) = displayable::size(ctx, canvas)?;
    let clip = dirty.intersect(Rect::new(0, 0, w, h));
    let g = paint_graphics(ctx, canvas)?;
    {
        let state = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
        state.reset();
        state.set_clip(clip.x, clip.y, clip.width, clip.height);
    }
    let result = ctx
        .call_virtual(canvas, "paint", PAINT, &[g.into()])
        .map(drop);
    report(ctx, "paint", result)
}

/// `repaint(x, y, w, h)`: marks a region of a shown canvas for painting.
/// On a game thread with no paint pending it first waits for the next
/// frame slot, which is how the frame rate bound reaches games that loop
/// on `repaint` and `Thread.sleep`.
fn repaint(ctx: &mut Ctx<'_>, canvas: ObjRef, region: Rect) -> JResult<()> {
    if region.is_empty() || !is_shown(ctx, canvas) {
        return Ok(());
    }
    if state(ctx).dirty.is_none() && !on_event_thread(ctx) {
        frame::wait_for_slot(ctx)?;
    }
    display::request_paint(ctx, canvas, region)
}

/// `serviceRepaints()`. Off the event thread it also waits for a paint
/// that is under way, whose region is no longer pending, since the game
/// expects that paint to have finished when the call returns.
fn service_repaints(ctx: &mut Ctx<'_>, canvas: ObjRef) -> JResult<()> {
    let elsewhere = !on_event_thread(ctx);
    let st = state(ctx);
    let busy = st.dirty.is_some() || (st.painting && elsewhere);
    if !busy || !is_shown(ctx, canvas) {
        return Ok(());
    }
    if on_event_thread(ctx) {
        // Inside a callback, which already holds the lock.
        let dirty = state(ctx).dirty.take();
        return match dirty {
            Some(dirty) => display::paint(ctx, canvas, dirty),
            None => Ok(()),
        };
    }
    let locked = acquire(ctx, Some(SERVICE_TIMEOUT))?;
    if !locked {
        warn_once(
            ctx,
            "serviceRepaints",
            "serviceRepaints waited over a second for the event thread; painting anyway",
        );
    }
    let result = match (is_shown(ctx, canvas), state(ctx).dirty) {
        (true, Some(_)) => {
            let dirty = state(ctx).dirty.take().unwrap_or(Rect::new(0, 0, 0, 0));
            display::paint(ctx, canvas, dirty)
        }
        _ => Ok(()),
    };
    if locked {
        release(ctx);
    }
    result
}

/// Copies a region of an off-screen surface onto a shown canvas's back
/// buffer and presents the screen: `GameCanvas.flushGraphics`. Waits for
/// the frame slot first when called off the event thread, so the frame
/// rate bound holds for games that only flush. Does nothing for a canvas
/// that is not shown, or that was hidden while the call waited.
///
/// The source is locked only for the copy, after the wait: the wait
/// releases the VM lock, and the collector locks every image's pixels to
/// count them, so holding them across it could block both threads.
pub fn flush(
    ctx: &mut Ctx<'_>,
    canvas: ObjRef,
    source: &SharedSurface,
    region: Rect,
) -> JResult<()> {
    if !is_shown(ctx, canvas) {
        return Ok(());
    }
    if !on_event_thread(ctx) {
        frame::wait_for_slot(ctx)?;
        if !is_shown(ctx, canvas) {
            return Ok(());
        }
    }
    copy_region(ctx, canvas, &source.lock(), region)?;
    frame::present(ctx)
}

/// Copies `region` of `source` onto the same place of `canvas`'s back
/// buffer, cut to both.
fn copy_region(ctx: &mut Ctx<'_>, canvas: ObjRef, source: &Surface, region: Rect) -> JResult<()> {
    let (w, h) = displayable::size(ctx, canvas)?;
    let region = region.intersect(Rect::new(0, 0, w, h)).intersect(Rect::new(
        0,
        0,
        source.width() as i32,
        source.height() as i32,
    ));
    if !region.is_empty() {
        let target = buffer(ctx, canvas)?;
        let mut target = target.lock();
        let (sw, tw) = (source.width() as usize, target.width() as usize);
        let rows = region.y as usize..(region.y + region.height) as usize;
        let columns = region.x as usize..(region.x + region.width) as usize;
        let from = source.pixels();
        target.edit_pixels(|to| {
            for row in rows {
                let src = &from[row * sw + columns.start..row * sw + columns.end];
                to[row * tw + columns.start..row * tw + columns.end].copy_from_slice(src);
            }
        });
    }
    Ok(())
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it has the shape of a native method body"
)]
fn noop(_: &mut Ctx<'_>, _: &[Value]) -> JResult<Value> {
    Ok(Value::Void)
}

fn this(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    ctx.non_null(v)
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    let protected = access::PROTECTED;
    let final_method = access::PUBLIC | access::FINAL;
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Displayable")
        .abstract_class()
        .payload_with(|| Box::new(DisplayableData::canvas()))
        .constant("UP", "I", int(keys::UP))
        .constant("DOWN", "I", int(keys::DOWN))
        .constant("LEFT", "I", int(keys::LEFT))
        .constant("RIGHT", "I", int(keys::RIGHT))
        .constant("FIRE", "I", int(keys::FIRE))
        .constant("GAME_A", "I", int(keys::GAME_A))
        .constant("GAME_B", "I", int(keys::GAME_B))
        .constant("GAME_C", "I", int(keys::GAME_C))
        .constant("GAME_D", "I", int(keys::GAME_D))
        .constant("KEY_NUM0", "I", int(48))
        .constant("KEY_NUM1", "I", int(49))
        .constant("KEY_NUM2", "I", int(50))
        .constant("KEY_NUM3", "I", int(51))
        .constant("KEY_NUM4", "I", int(52))
        .constant("KEY_NUM5", "I", int(53))
        .constant("KEY_NUM6", "I", int(54))
        .constant("KEY_NUM7", "I", int(55))
        .constant("KEY_NUM8", "I", int(56))
        .constant("KEY_NUM9", "I", int(57))
        .constant("KEY_STAR", "I", int(42))
        .constant("KEY_POUND", "I", int(35))
        .method_with("<init>", "()V", protected, noop)
        .method("isDoubleBuffered", "()Z", |_, _| Ok(Value::from(true)))
        .method("hasPointerEvents", "()Z", |ctx, _| {
            Ok(Value::from(env(ctx).profile.touch))
        })
        .method("hasPointerMotionEvents", "()Z", |ctx, _| {
            Ok(Value::from(env(ctx).profile.touch))
        })
        .method("hasRepeatEvents", "()Z", |_, _| Ok(Value::from(true)))
        .method("getKeyCode", "(I)I", |ctx, a| {
            match keys::key_code(&env(ctx).profile.key_codes, a[1].int()) {
                Some(code) => Ok(Value::Int(code)),
                None => ctx.throw("java/lang/IllegalArgumentException", "not a game action"),
            }
        })
        .method("getKeyName", "(I)Ljava/lang/String;", |ctx, a| {
            let name = keys::key_name(&env(ctx).profile.key_codes, a[1].int());
            Ok(ctx.new_string(&name)?.into())
        })
        .method("getGameAction", "(I)I", |ctx, a| {
            let codes = env(ctx).profile.key_codes;
            Ok(Value::Int(keys::game_action(&codes, a[1].int())))
        })
        .method("setFullScreenMode", "(Z)V", |ctx, a| {
            let canvas = this(ctx, a[0])?;
            let full = a[1].boolean();
            let d = data_mut(ctx, canvas)?;
            let changed = d.full_screen != full;
            d.full_screen = full;
            if changed {
                display::relayout(ctx, canvas);
            }
            Ok(Value::Void)
        })
        .method_with("keyPressed", "(I)V", protected, noop)
        .method_with("keyRepeated", "(I)V", protected, noop)
        .method_with("keyReleased", "(I)V", protected, noop)
        .method_with("pointerPressed", "(II)V", protected, noop)
        .method_with("pointerReleased", "(II)V", protected, noop)
        .method_with("pointerDragged", "(II)V", protected, noop)
        .method_with("showNotify", "()V", protected, noop)
        .method_with("hideNotify", "()V", protected, noop)
        .method_with("sizeChanged", "(II)V", protected, noop)
        .abstract_method("paint", PAINT)
        .method_with("repaint", "(IIII)V", final_method, |ctx, a| {
            let canvas = this(ctx, a[0])?;
            let region = Rect::new(a[1].int(), a[2].int(), a[3].int(), a[4].int());
            repaint(ctx, canvas, region)?;
            Ok(Value::Void)
        })
        .method_with("repaint", "()V", final_method, |ctx, a| {
            let canvas = this(ctx, a[0])?;
            let (w, h) = displayable::size(ctx, canvas)?;
            repaint(ctx, canvas, Rect::new(0, 0, w, h))?;
            Ok(Value::Void)
        })
        .method_with("serviceRepaints", "()V", final_method, |ctx, a| {
            let canvas = this(ctx, a[0])?;
            service_repaints(ctx, canvas)?;
            Ok(Value::Void)
        })
}
