//! `javax.microedition.lcdui.Display`, and the functions the rest of the
//! runtime uses to ask what is shown, switch it and repaint it.
//!
//! `setCurrent` is asynchronous: it records the latest request, which the
//! event thread applies between callbacks (`sizeChanged` first if the size
//! changed since the displayable last heard, then `hideNotify` on the old
//! one, `showNotify` on the new one, and a full repaint). `getCurrent`
//! returns the latest request at once, since games check it in loops.

use std::sync::Arc;

use fastemu_gfx::Graphics;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::canvas;
use super::displayable::{self, Kind, data, data_mut};
use super::frame;
use super::screen::ScreenTarget;
use super::state::{self, Internal, Rect, report, state};
use crate::env::env;

const CLASS: &str = "javax/microedition/lcdui/Display";
const ALERT: &str = "javax/microedition/lcdui/Alert";

/// `Display.LIST_ELEMENT`, `CHOICE_GROUP_ELEMENT` and `ALERT`.
const IMAGE_TYPES: std::ops::RangeInclusive<i32> = 1..=3;

/// The `Displayable` on the screen, if any.
pub fn current(ctx: &mut Ctx<'_>) -> Option<ObjRef> {
    state::current(ctx)
}

/// Whether `obj` is on the screen and the instance is in the foreground:
/// `Displayable.isShown()`.
pub fn is_shown(ctx: &mut Ctx<'_>, obj: ObjRef) -> bool {
    !state(ctx).background && current(ctx) == Some(obj)
}

/// Asks the event thread to show `next`, replacing any request it has not
/// applied yet: `Display.setCurrent(next)`.
pub fn set_current(ctx: &mut Ctx<'_>, next: ObjRef) {
    let global = Arc::new(ctx.new_global(next));
    state(ctx).requested = Some(global);
    ctx.notify_waiters();
}

/// Marks a region of a shown displayable for painting; the event thread
/// paints it. Regions of displayables that are not shown are dropped.
pub fn request_paint(ctx: &mut Ctx<'_>, obj: ObjRef, region: Rect) -> JResult<()> {
    if !is_shown(ctx, obj) {
        return Ok(());
    }
    let (w, h) = displayable::size(ctx, obj)?;
    let region = region.intersect(Rect::new(0, 0, w, h));
    if region.is_empty() {
        return Ok(());
    }
    let st = state(ctx);
    let newly = st.dirty.is_none();
    st.dirty = Some(st.dirty.map_or(region, |d| d.union(region)));
    if newly {
        ctx.notify_waiters();
    }
    Ok(())
}

/// Repaints all of a shown displayable, as a high-level screen does after
/// its content changes.
pub fn repaint(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    request_paint(ctx, obj, Rect::new(0, 0, i32::MAX, i32::MAX))
}

/// Tells the event thread that `obj`'s size or soft keys may have changed
/// (its commands, title, ticker or full-screen mode). Only the shown
/// displayable needs it: the others are laid out when they are shown.
pub fn relayout(ctx: &mut Ctx<'_>, obj: ObjRef) {
    if current(ctx) != Some(obj) {
        return;
    }
    let global = Arc::new(ctx.new_global(obj));
    state(ctx).internal.push_back(Internal::Relayout(global));
    ctx.notify_waiters();
}

/// Paints `dirty` of the shown displayable and presents a frame. The
/// caller holds the callback lock.
pub(crate) fn paint(ctx: &mut Ctx<'_>, obj: ObjRef, dirty: Rect) -> JResult<()> {
    let outer = std::mem::replace(&mut state(ctx).painting, true);
    let painted = match data(ctx, obj)?.kind() {
        Kind::Canvas => canvas::paint(ctx, obj, dirty),
        Kind::Screen => paint_screen(ctx, obj),
    };
    state(ctx).painting = outer;
    painted?;
    frame::present(ctx)
}

/// Lets a high-level screen paint its content area.
fn paint_screen(ctx: &mut Ctx<'_>, screen: ObjRef) -> JResult<()> {
    let Some(handler) = displayable::handler(ctx, screen)? else {
        return Ok(());
    };
    let content = displayable::layout(ctx, screen)?.content;
    let prepared = handler.prepare(ctx, screen, content.width, content.height);
    report(ctx, "preparing a screen", prepared)?;
    let buffer = canvas::buffer(ctx, screen)?;
    let (fonts, theme) = {
        let st = state(ctx);
        (Arc::clone(&st.fonts), st.theme)
    };
    let mut surface = buffer.lock();
    if let Ok(mut g) = Graphics::new(&surface, fonts.default_font()) {
        g.set_color(theme.background);
        g.fill_rect(&mut surface, 0, 0, content.width, content.height);
    }
    let mut target = ScreenTarget {
        surface: &mut surface,
        width: content.width,
        height: content.height,
        fonts: &fonts,
        theme: &theme,
    };
    let result = handler.paint(ctx, screen, &mut target);
    drop(surface);
    report(ctx, "painting a screen", result)
}

/// Sends `sizeChanged` to `obj` if its size differs from the last one it
/// was told, as MIDP asks before a displayable is first shown and after
/// its size changes. The caller holds the callback lock.
pub(crate) fn notify_size(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let size = displayable::size(ctx, obj)?;
    let d = data_mut(ctx, obj)?;
    if d.last_size == Some(size) {
        return Ok(());
    }
    d.last_size = Some(size);
    let result = ctx
        .call_virtual(
            obj,
            "sizeChanged",
            "(II)V",
            &[Value::Int(size.0), Value::Int(size.1)],
        )
        .map(drop);
    report(ctx, "sizeChanged", result)
}

fn display_object(ctx: &mut Ctx<'_>) -> JResult<ObjRef> {
    if let Some(global) = state(ctx).display.clone() {
        return Ok(global.get(ctx));
    }
    let class = ctx.find_class(CLASS)?;
    let obj = ctx.new_object(class)?;
    let global = Arc::new(ctx.new_global(obj));
    state(ctx).display = Some(global);
    Ok(obj)
}

fn set_current_alert(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let alert = ctx.non_null(a[1])?;
    let next = ctx.non_null(a[2])?;
    if let Ok(alert_class) = ctx.find_class(ALERT)
        && ctx.is_instance(next, alert_class)
    {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "the displayable after an Alert cannot be an Alert",
        );
    }
    data_mut(ctx, alert)?.set_after_alert(Some(next));
    set_current(ctx, alert);
    Ok(Value::Void)
}

/// `vibrate` and `flashBacklight`: the emulated phone has a motor and a
/// backlight, so both log the request and return true.
fn feedback(ctx: &mut Ctx<'_>, what: &str, duration: i32) -> JResult<Value> {
    if duration < 0 {
        return ctx.throw("java/lang/IllegalArgumentException", "negative duration");
    }
    ctx.log(
        fastemu_core::LogLevel::Info,
        format!("{what} for {duration} ms"),
    );
    Ok(Value::from(true))
}

fn best_image_size(ctx: &mut Ctx<'_>, kind: i32, width: bool) -> JResult<Value> {
    if !IMAGE_TYPES.contains(&kind) {
        return ctx.throw("java/lang/IllegalArgumentException", "invalid image type");
    }
    let st = state(ctx);
    let line = st.fonts.default_font().height();
    let size = match (kind, width) {
        (3, true) => st.screen_width,
        (3, false) => st.screen_height / 2,
        _ => line,
    };
    Ok(Value::Int(size))
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .constant("LIST_ELEMENT", "I", int(1))
        .constant("CHOICE_GROUP_ELEMENT", "I", int(2))
        .constant("ALERT", "I", int(3))
        .constant("COLOR_BACKGROUND", "I", int(0))
        .constant("COLOR_FOREGROUND", "I", int(1))
        .constant("COLOR_HIGHLIGHTED_BACKGROUND", "I", int(2))
        .constant("COLOR_HIGHLIGHTED_FOREGROUND", "I", int(3))
        .constant("COLOR_BORDER", "I", int(4))
        .constant("COLOR_HIGHLIGHTED_BORDER", "I", int(5))
        .static_method(
            "getDisplay",
            "(Ljavax/microedition/midlet/MIDlet;)Ljavax/microedition/lcdui/Display;",
            |ctx, a| {
                ctx.non_null(a[0])?;
                Ok(display_object(ctx)?.into())
            },
        )
        .method("getColor", "(I)I", |ctx, a| {
            match state(ctx).theme.color(a[1].int()) {
                Some(color) => Ok(Value::Int(color)),
                None => ctx.throw(
                    "java/lang/IllegalArgumentException",
                    "invalid color specifier",
                ),
            }
        })
        .method("getBorderStyle", "(Z)I", |_, _| Ok(Value::Int(0)))
        .method("isColor", "()Z", |ctx, _| {
            Ok(Value::from(env(ctx).profile.colors > 2))
        })
        .method("numColors", "()I", |ctx, _| {
            Ok(Value::Int(
                env(ctx).profile.colors.min(i32::MAX as u32) as i32
            ))
        })
        .method("numAlphaLevels", "()I", |ctx, _| {
            Ok(Value::Int(
                env(ctx).profile.alpha_levels.clamp(2, 256) as i32
            ))
        })
        .method(
            "getCurrent",
            "()Ljavax/microedition/lcdui/Displayable;",
            |ctx, _| {
                let global = {
                    let st = state(ctx);
                    st.requested.clone().or_else(|| st.current.clone())
                };
                Ok(state::get(ctx, global.as_ref()).into())
            },
        )
        .method(
            "setCurrent",
            "(Ljavax/microedition/lcdui/Displayable;)V",
            |ctx, a| {
                // null keeps the current screen.
                if let Some(next) = a[1].obj() {
                    set_current(ctx, next);
                }
                Ok(Value::Void)
            },
        )
        .method(
            "setCurrent",
            "(Ljavax/microedition/lcdui/Alert;Ljavax/microedition/lcdui/Displayable;)V",
            set_current_alert,
        )
        .method("callSerially", "(Ljava/lang/Runnable;)V", |ctx, a| {
            if let Some(runnable) = a[1].obj() {
                let global = Arc::new(ctx.new_global(runnable));
                state(ctx).serial.push_back(global);
                ctx.notify_waiters();
            }
            Ok(Value::Void)
        })
        .method("vibrate", "(I)Z", |ctx, a| {
            feedback(ctx, "vibrate", a[1].int())
        })
        .method("flashBacklight", "(I)Z", |ctx, a| {
            feedback(ctx, "flashBacklight", a[1].int())
        })
        .method("getBestImageWidth", "(I)I", |ctx, a| {
            best_image_size(ctx, a[1].int(), true)
        })
        .method("getBestImageHeight", "(I)I", |ctx, a| {
            best_image_size(ctx, a[1].int(), false)
        })
}
