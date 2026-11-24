//! `javax.microedition.lcdui.CustomItem`: an item the game measures and
//! draws itself, with key, pointer and traversal callbacks.
//!
//! The game paints into a surface of the item's content size, through a
//! `Graphics` the item keeps; the form calls `render` for each shown
//! custom item before it locks its own buffer, and then copies the
//! surface into place. Every callback runs on the event thread, inside
//! the callback lock; an exception one throws is logged and the form
//! carries on.

use std::sync::Arc;

use fastemu_core::PointerAction;
use fastemu_gfx::Surface;
use fastemu_jvm::{ArrayType, Ctx, JResult, NativeClass, ObjRef, Throw, Tracer, Value};
use parking_lot::Mutex;

use super::{Handled, ItemData, Kind, MAX_SIDE, Needs, changed, data, data_mut};
use crate::env::env;
use crate::lcdui::Rect;
use crate::lcdui::display;
use crate::lcdui::graphics::{GraphicsData, new_graphics};
use crate::lcdui::highlevel::draw::Painter;
use crate::lcdui::image::SharedSurface;
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase};
use crate::lcdui::state::{report, state};

const CLASS: &str = "javax/microedition/lcdui/CustomItem";

/// `CustomItem.TRAVERSE_HORIZONTAL`.
const TRAVERSE_HORIZONTAL: i32 = 1;
/// `CustomItem.TRAVERSE_VERTICAL`.
const TRAVERSE_VERTICAL: i32 = 2;
/// `CustomItem.KEY_PRESS`, `KEY_RELEASE` and `KEY_REPEAT`.
const KEYS: i32 = 4 | 8 | 16;
/// `CustomItem.POINTER_PRESS`, `POINTER_RELEASE` and `POINTER_DRAG`.
const POINTER: i32 = 32 | 64 | 128;

/// What a `CustomItem` keeps.
#[derive(Default)]
pub struct CustomState {
    /// The pixels the game last painted.
    surface: Option<SharedSurface>,
    /// The `Graphics` it paints with, made with the surface.
    graphics: Option<ObjRef>,
    /// The content size `sizeChanged` last reported.
    size: Option<(i32, i32)>,
    /// Whether the last `traverse` kept the traversal inside the item.
    pub internal: bool,
}

impl CustomState {
    pub(super) fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.graphics);
    }
}

fn custom_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut CustomState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Custom(s) => Ok(s),
        _ => unreachable!("a CustomItem's payload is made by its class"),
    }
}

fn custom<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a CustomState> {
    match &data(ctx, item)?.kind {
        Kind::Custom(s) => Ok(s),
        _ => unreachable!("a CustomItem's payload is made by its class"),
    }
}

/// Calls one of the game's methods, logging an exception it throws as
/// `Exception in <name>`; the call then counts as returning `None`.
fn call(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    name: &str,
    desc: &str,
    args: &[Value],
) -> JResult<Option<Value>> {
    match ctx.call_virtual(item, name, desc, args) {
        Ok(v) => Ok(Some(v)),
        Err(Throw::Exception(e)) => {
            report(ctx, name, Err(Throw::Exception(e)))?;
            Ok(None)
        }
        Err(stop) => Err(stop),
    }
}

fn call_int(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    name: &str,
    desc: &str,
    args: &[Value],
) -> JResult<i32> {
    Ok(call(ctx, item, name, desc, args)?.map_or(0, |v| v.int().clamp(0, MAX_SIDE)))
}

/// The game's sizes: at least its minimum, its preferred width within
/// `max`, and its preferred height for that width.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, max: i32) -> JResult<Needs> {
    let min_w = call_int(ctx, item, "getMinContentWidth", "()I", &[])?;
    let min_h = call_int(ctx, item, "getMinContentHeight", "()I", &[])?;
    let pref_w = call_int(ctx, item, "getPrefContentWidth", "(I)I", &[Value::Int(-1)])?;
    let width = pref_w.max(min_w).min(max);
    let pref_h = call_int(
        ctx,
        item,
        "getPrefContentHeight",
        "(I)I",
        &[Value::Int(width)],
    )?;
    Ok(Needs {
        min_width: min_w.min(max),
        width,
        height: pref_h.max(min_h),
    })
}

/// Lets the game paint the item at `width` by `height`, telling it first
/// when that size changed. Runs outside the screen buffer's lock.
pub(crate) fn render(ctx: &mut Ctx<'_>, item: ObjRef, width: i32, height: i32) -> JResult<()> {
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    if custom(ctx, item)?.size != Some((width, height)) {
        custom_mut(ctx, item)?.size = Some((width, height));
        let args = [Value::Int(width), Value::Int(height)];
        call(ctx, item, "sizeChanged", "(II)V", &args)?;
    }
    let background = state(ctx).theme.background;
    let reuse = custom(ctx, item)?
        .surface
        .as_ref()
        .filter(|s| {
            let s = s.lock();
            (s.width() as i32, s.height() as i32) == (width, height)
        })
        .cloned();
    let (surface, g) = match (reuse, custom(ctx, item)?.graphics) {
        (Some(surface), Some(g)) => (surface, ctx.root(g)),
        _ => {
            let fresh =
                Surface::new_mutable_filled(width, height, background | 0xFF00_0000u32 as i32)
                    .map_err(|e| crate::lcdui::throw_gfx(ctx, &e))?;
            let surface: SharedSurface = Arc::new(Mutex::new(fresh));
            let g = new_graphics(ctx, Arc::clone(&surface))?;
            let s = custom_mut(ctx, item)?;
            s.surface = Some(Arc::clone(&surface));
            s.graphics = Some(g);
            (surface, g)
        }
    };
    {
        let mut pixels = surface.lock();
        pixels.edit_pixels(|p| p.fill(0xFF00_0000 | background as u32));
    }
    {
        let gs = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
        gs.reset();
        gs.set_clip(0, 0, width, height);
    }
    let args = [g.into(), Value::Int(width), Value::Int(height)];
    call(
        ctx,
        item,
        "paint",
        "(Ljavax/microedition/lcdui/Graphics;II)V",
        &args,
    )?;
    Ok(())
}

/// Copies what the game last painted into `area`.
pub(super) fn paint(ctx: &Ctx<'_>, item: ObjRef, p: &mut Painter<'_>, area: Rect) -> JResult<()> {
    if let Some(surface) = &custom(ctx, item)?.surface {
        let pixels = surface.lock();
        p.image(&pixels, area.x, area.y);
    }
    Ok(())
}

/// Keys go to the game's key callbacks; up and down are traversal, which
/// the form handles through [`traverse`].
pub(super) fn key(ctx: &mut Ctx<'_>, item: ObjRef, event: KeyEvent) -> JResult<Handled> {
    if matches!(event.game_action, keys::UP | keys::DOWN) {
        let codes = env(ctx).profile.key_codes;
        if event.code == codes.up || event.code == codes.down {
            return Ok(Handled::No);
        }
    }
    let name = match event.phase {
        KeyPhase::Pressed => "keyPressed",
        KeyPhase::Repeated => "keyRepeated",
        KeyPhase::Released => "keyReleased",
    };
    call(ctx, item, name, "(I)V", &[Value::Int(event.code)])?;
    Ok(Handled::Yes)
}

/// Pointer events in content coordinates go to the game's callbacks.
pub(super) fn pointer(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    action: PointerAction,
    x: i32,
    y: i32,
) -> JResult<Handled> {
    let name = match action {
        PointerAction::Press => "pointerPressed",
        PointerAction::Drag => "pointerDragged",
        PointerAction::Release => "pointerReleased",
    };
    call(ctx, item, name, "(II)V", &[Value::Int(x), Value::Int(y)])?;
    Ok(Handled::Yes)
}

/// `traverse(dir, viewportWidth, viewportHeight, visRect)`: whether the
/// game keeps the traversal inside the item. `visible` is the part of
/// the item in view, in content coordinates.
pub(crate) fn traverse(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    dir: i32,
    viewport: (i32, i32),
    visible: Rect,
) -> JResult<bool> {
    let rect = ctx.new_array(ArrayType::Int, 4)?;
    ctx.array_mut::<i32>(rect)?.copy_from_slice(&[
        visible.x,
        visible.y,
        visible.width,
        visible.height,
    ]);
    let args = [
        Value::Int(dir),
        Value::Int(viewport.0),
        Value::Int(viewport.1),
        rect.into(),
    ];
    let inside = call(ctx, item, "traverse", "(III[I)Z", &args)?.is_some_and(Value::boolean);
    custom_mut(ctx, item)?.internal = inside;
    Ok(inside)
}

/// `traverseOut()`, when the focus leaves the item.
pub(crate) fn traverse_out(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    custom_mut(ctx, item)?.internal = false;
    call(ctx, item, "traverseOut", "()V", &[])?;
    Ok(())
}

/// Whether the last `traverse` kept the traversal inside.
pub(crate) fn internal(ctx: &Ctx<'_>, item: ObjRef) -> JResult<bool> {
    Ok(custom(ctx, item)?.internal)
}

/// `showNotify` or `hideNotify`, as the form appears or goes.
pub(crate) fn notify_shown(ctx: &mut Ctx<'_>, item: ObjRef, shown: bool) -> JResult<()> {
    let name = if shown { "showNotify" } else { "hideNotify" };
    call(ctx, item, name, "()V", &[])?;
    Ok(())
}

fn repaint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let item = ctx.non_null(a[0])?;
    let d = data(ctx, item)?;
    if let Some(owner) = d.owner {
        display::repaint(ctx, owner)?;
    }
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    use fastemu_jvm::classfile::access::{FINAL, PROTECTED};
    let int = |v| Value::Int(v);
    let empty = |_: &mut Ctx<'_>, _: &[Value]| Ok(Value::Void);
    // MIDP declares the constants protected; javac inlines them either way.
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .abstract_class()
        .payload_with(|| Box::new(ItemData::new(Kind::Custom(CustomState::default()))))
        .constant("TRAVERSE_HORIZONTAL", "I", int(TRAVERSE_HORIZONTAL))
        .constant("TRAVERSE_VERTICAL", "I", int(TRAVERSE_VERTICAL))
        .constant("KEY_PRESS", "I", int(4))
        .constant("KEY_RELEASE", "I", int(8))
        .constant("KEY_REPEAT", "I", int(16))
        .constant("POINTER_PRESS", "I", int(32))
        .constant("POINTER_RELEASE", "I", int(64))
        .constant("POINTER_DRAG", "I", int(128))
        .constant("NONE", "I", int(0))
        .method_with("<init>", "(Ljava/lang/String;)V", PROTECTED, |ctx, a| {
            let item = ctx.non_null(a[0])?;
            data_mut(ctx, item)?.label = a[1].obj();
            Ok(Value::Void)
        })
        .method("getGameAction", "(I)I", |ctx, a| {
            let codes = env(ctx).profile.key_codes;
            Ok(Value::Int(keys::game_action(&codes, a[1].int())))
        })
        .method_with("getInteractionModes", "()I", PROTECTED | FINAL, |ctx, _| {
            let touch = env(ctx).profile.touch;
            Ok(Value::Int(
                TRAVERSE_VERTICAL | KEYS | if touch { POINTER } else { 0 },
            ))
        })
        .abstract_method_with("getMinContentWidth", "()I", PROTECTED)
        .abstract_method_with("getMinContentHeight", "()I", PROTECTED)
        .abstract_method_with("getPrefContentWidth", "(I)I", PROTECTED)
        .abstract_method_with("getPrefContentHeight", "(I)I", PROTECTED)
        .abstract_method_with(
            "paint",
            "(Ljavax/microedition/lcdui/Graphics;II)V",
            PROTECTED,
        )
        .method_with("sizeChanged", "(II)V", PROTECTED, empty)
        .method_with("invalidate", "()V", PROTECTED | FINAL, |ctx, a| {
            let item = ctx.non_null(a[0])?;
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method_with("repaint", "()V", PROTECTED | FINAL, repaint)
        .method_with("repaint", "(IIII)V", PROTECTED | FINAL, repaint)
        .method_with("traverse", "(III[I)Z", PROTECTED, |_, _| {
            Ok(Value::from(false))
        })
        .method_with("traverseOut", "()V", PROTECTED, empty)
        .method_with("keyPressed", "(I)V", PROTECTED, empty)
        .method_with("keyReleased", "(I)V", PROTECTED, empty)
        .method_with("keyRepeated", "(I)V", PROTECTED, empty)
        .method_with("pointerPressed", "(II)V", PROTECTED, empty)
        .method_with("pointerReleased", "(II)V", PROTECTED, empty)
        .method_with("pointerDragged", "(II)V", PROTECTED, empty)
        .method_with("showNotify", "()V", PROTECTED, empty)
        .method_with("hideNotify", "()V", PROTECTED, empty)
}
