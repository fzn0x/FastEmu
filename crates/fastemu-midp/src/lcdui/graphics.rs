//! `javax.microedition.lcdui.Graphics` over `fastemu-gfx`.
//!
//! A Java `Graphics` keeps the drawing state ([`fastemu_gfx::Graphics`])
//! and the pixels it draws on in its payload. Following
//! `docs/architecture/graphics.md`, null arguments throw
//! `NullPointerException` before anything is drawn, an image drawn onto
//! itself is copied first, and `drawRegion` of the destination itself
//! throws `IllegalArgumentException`.

use std::sync::Arc;

use fastemu_gfx::{GfxError, Graphics, Surface};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value};

use super::font::{default_font, font_object, font_of};
use super::image::{ImageData, SharedSurface};
use super::throw_gfx;
use crate::env::env;

const CLASS: &str = "javax/microedition/lcdui/Graphics";

/// What a Java `Graphics` holds: MIDP's drawing state and its target.
pub struct GraphicsData {
    /// Translation, clip, colour, stroke and font.
    pub state: Graphics,
    /// The pixels drawing lands on: an image's, a canvas's back buffer or
    /// a `GameCanvas`'s off-screen buffer.
    pub target: SharedSurface,
}

impl Payload for GraphicsData {}

/// A new Java `Graphics` drawing on `target`, in MIDP's initial state with
/// the instance's default font and display depth.
pub fn new_graphics(ctx: &mut Ctx<'_>, target: SharedSurface) -> JResult<ObjRef> {
    let font = default_font(ctx);
    let colors = env(ctx).profile.colors;
    let state = Graphics::new(&target.lock(), font);
    let mut state = state.map_err(|e| throw_gfx(ctx, &e))?;
    state.set_display_colors(colors);
    let class = ctx.find_class(CLASS)?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(obj, GraphicsData { state, target });
    Ok(obj)
}

/// The payload of a `Graphics` argument.
fn data<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a GraphicsData> {
    let this = ctx.non_null(this)?;
    ctx.payload::<GraphicsData>(this)
}

/// Draws with the state and target of `this`.
fn draw(ctx: &mut Ctx<'_>, this: Value, f: impl FnOnce(&Graphics, &mut Surface)) -> JResult<Value> {
    let d = data(ctx, this)?;
    f(&d.state, &mut d.target.lock());
    Ok(Value::Void)
}

/// Changes the state of `this`.
fn edit<R>(ctx: &mut Ctx<'_>, this: Value, f: impl FnOnce(&mut Graphics) -> R) -> JResult<R> {
    let this = ctx.non_null(this)?;
    Ok(f(&mut ctx.payload_mut::<GraphicsData>(this)?.state))
}

/// Reads the state of `this`.
fn query(f: fn(&Graphics) -> i32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| Ok(Value::Int(f(&data(ctx, a[0])?.state)))
}

fn checked(ctx: &mut Ctx<'_>, result: Result<(), GfxError>) -> JResult<Value> {
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// A drawing call with four `int` arguments.
fn shape4(
    f: fn(&Graphics, &mut Surface, i32, i32, i32, i32),
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        let [x, y, w, h] = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
        draw(ctx, a[0], |g, s| f(g, s, x, y, w, h))
    }
}

/// A drawing call with six `int` arguments.
#[allow(clippy::type_complexity)] // the signature of six-int MIDP calls
fn shape6(
    f: fn(&Graphics, &mut Surface, i32, i32, i32, i32, i32, i32),
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        let v: Vec<i32> = a[1..7].iter().map(|v| v.int()).collect();
        draw(ctx, a[0], |g, s| {
            f(g, s, v[0], v[1], v[2], v[3], v[4], v[5]);
        })
    }
}

/// The text of a `String` argument, `NullPointerException` for `null`.
fn text(ctx: &mut Ctx<'_>, value: Value) -> JResult<Vec<u16>> {
    let s = ctx.non_null(value)?;
    Ok(ctx.string_utf16(s)?.to_vec())
}

fn draw_text(
    ctx: &mut Ctx<'_>,
    this: Value,
    f: impl FnOnce(&Graphics, &mut Surface) -> Result<(), GfxError>,
) -> JResult<Value> {
    let d = data(ctx, this)?;
    let result = f(&d.state, &mut d.target.lock());
    checked(ctx, result)
}

fn draw_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let image = ctx.non_null(a[1])?;
    let (x, y, anchor) = (a[2].int(), a[3].int(), a[4].int());
    let source = Arc::clone(&ctx.payload::<ImageData>(image)?.surface);
    let d = data(ctx, a[0])?;
    let result = if Arc::ptr_eq(&source, &d.target) {
        // Undefined in MIDP and impossible with one borrow: draw a copy.
        let mut target = d.target.lock();
        let copy = target.clone();
        d.state.draw_image(&mut target, &copy, x, y, anchor)
    } else {
        let source = source.lock();
        d.state
            .draw_image(&mut d.target.lock(), &source, x, y, anchor)
    };
    checked(ctx, result)
}

fn draw_region(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let image = ctx.non_null(a[1])?;
    let v: Vec<i32> = a[2..10].iter().map(|v| v.int()).collect();
    let source = Arc::clone(&ctx.payload::<ImageData>(image)?.surface);
    if Arc::ptr_eq(&source, &data(ctx, a[0])?.target) {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "drawRegion of the destination image itself",
        );
    }
    let d = data(ctx, a[0])?;
    let result = {
        let source = source.lock();
        d.state.draw_region(
            &mut d.target.lock(),
            &source,
            v[0],
            v[1],
            v[2],
            v[3],
            v[4],
            v[5],
            v[6],
            v[7],
        )
    };
    checked(ctx, result)
}

fn draw_rgb(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let rgb = ctx.non_null(a[1])?;
    let v: Vec<i32> = a[2..8].iter().map(|v| v.int()).collect();
    let alpha = a[8].boolean();
    let this = ctx.non_null(a[0])?;
    let d = ctx.payload::<GraphicsData>(this)?;
    let pixels = ctx.array::<i32>(rgb)?;
    let result = d.state.draw_rgb(
        &mut d.target.lock(),
        pixels,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        alpha,
    );
    checked(ctx, result)
}

fn copy_area(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let v: Vec<i32> = a[1..8].iter().map(|v| v.int()).collect();
    let d = data(ctx, a[0])?;
    let result = d.state.copy_area(
        &mut d.target.lock(),
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
    );
    checked(ctx, result)
}

fn set_font(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let font = match a[1].obj() {
        Some(obj) => Some(font_of(ctx, obj)?),
        None => None,
    };
    edit(ctx, a[0], |g| g.set_font(font))?;
    Ok(Value::Void)
}

fn get_font(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let font = data(ctx, a[0])?.state.font().clone();
    Ok(font_object(ctx, &font)?.into())
}

fn fallible_edit(
    ctx: &mut Ctx<'_>,
    this: Value,
    f: impl FnOnce(&mut Graphics) -> Result<(), GfxError>,
) -> JResult<Value> {
    let result = edit(ctx, this, f)?;
    checked(ctx, result)
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .constant("HCENTER", "I", int(1))
        .constant("VCENTER", "I", int(2))
        .constant("LEFT", "I", int(4))
        .constant("RIGHT", "I", int(8))
        .constant("TOP", "I", int(16))
        .constant("BOTTOM", "I", int(32))
        .constant("BASELINE", "I", int(64))
        .constant("SOLID", "I", int(0))
        .constant("DOTTED", "I", int(1))
        .method("translate", "(II)V", |ctx, a| {
            let (x, y) = (a[1].int(), a[2].int());
            edit(ctx, a[0], |g| g.translate(x, y))?;
            Ok(Value::Void)
        })
        .method("getTranslateX", "()I", query(Graphics::translate_x))
        .method("getTranslateY", "()I", query(Graphics::translate_y))
        .method("getColor", "()I", query(Graphics::color))
        .method("getRedComponent", "()I", query(Graphics::red_component))
        .method("getGreenComponent", "()I", query(Graphics::green_component))
        .method("getBlueComponent", "()I", query(Graphics::blue_component))
        .method("getGrayScale", "()I", query(Graphics::gray_scale))
        .method("setColor", "(III)V", |ctx, a| {
            let (r, g, b) = (a[1].int(), a[2].int(), a[3].int());
            fallible_edit(ctx, a[0], |gr| gr.set_color_rgb(r, g, b))
        })
        .method("setColor", "(I)V", |ctx, a| {
            let rgb = a[1].int();
            edit(ctx, a[0], |g| g.set_color(rgb))?;
            Ok(Value::Void)
        })
        .method("setGrayScale", "(I)V", |ctx, a| {
            let v = a[1].int();
            fallible_edit(ctx, a[0], |g| g.set_gray_scale(v))
        })
        .method("getFont", "()Ljavax/microedition/lcdui/Font;", get_font)
        .method("setFont", "(Ljavax/microedition/lcdui/Font;)V", set_font)
        .method("setStrokeStyle", "(I)V", |ctx, a| {
            let style = a[1].int();
            fallible_edit(ctx, a[0], |g| g.set_stroke_style(style))
        })
        .method("getStrokeStyle", "()I", query(Graphics::stroke_style))
        .method("getClipX", "()I", query(Graphics::clip_x))
        .method("getClipY", "()I", query(Graphics::clip_y))
        .method("getClipWidth", "()I", query(Graphics::clip_width))
        .method("getClipHeight", "()I", query(Graphics::clip_height))
        .method("clipRect", "(IIII)V", |ctx, a| {
            let [x, y, w, h] = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            edit(ctx, a[0], |g| g.clip_rect(x, y, w, h))?;
            Ok(Value::Void)
        })
        .method("setClip", "(IIII)V", |ctx, a| {
            let [x, y, w, h] = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            edit(ctx, a[0], |g| g.set_clip(x, y, w, h))?;
            Ok(Value::Void)
        })
        .method("drawLine", "(IIII)V", shape4(Graphics::draw_line))
        .method("fillRect", "(IIII)V", shape4(Graphics::fill_rect))
        .method("drawRect", "(IIII)V", shape4(Graphics::draw_rect))
        .method(
            "drawRoundRect",
            "(IIIIII)V",
            shape6(Graphics::draw_round_rect),
        )
        .method(
            "fillRoundRect",
            "(IIIIII)V",
            shape6(Graphics::fill_round_rect),
        )
        .method("fillArc", "(IIIIII)V", shape6(Graphics::fill_arc))
        .method("drawArc", "(IIIIII)V", shape6(Graphics::draw_arc))
        .method("fillTriangle", "(IIIIII)V", shape6(Graphics::fill_triangle))
        .method("drawString", "(Ljava/lang/String;III)V", |ctx, a| {
            let s = text(ctx, a[1])?;
            let [x, y, anchor] = [a[2].int(), a[3].int(), a[4].int()];
            draw_text(ctx, a[0], |g, t| g.draw_string(t, &s, x, y, anchor))
        })
        .method("drawSubstring", "(Ljava/lang/String;IIIII)V", |ctx, a| {
            let s = text(ctx, a[1])?;
            let v: Vec<i32> = a[2..7].iter().map(|v| v.int()).collect();
            draw_text(ctx, a[0], |g, t| {
                g.draw_substring(t, &s, v[0], v[1], v[2], v[3], v[4])
            })
        })
        .method("drawChar", "(CIII)V", |ctx, a| {
            let c = a[1].char();
            let [x, y, anchor] = [a[2].int(), a[3].int(), a[4].int()];
            draw_text(ctx, a[0], |g, t| g.draw_char(t, c, x, y, anchor))
        })
        .method("drawChars", "([CIIIII)V", |ctx, a| {
            let chars = ctx.non_null(a[1])?;
            let units = ctx.array::<u16>(chars)?.to_vec();
            let v: Vec<i32> = a[2..7].iter().map(|v| v.int()).collect();
            draw_text(ctx, a[0], |g, t| {
                g.draw_chars(t, &units, v[0], v[1], v[2], v[3], v[4])
            })
        })
        .method(
            "drawImage",
            "(Ljavax/microedition/lcdui/Image;III)V",
            draw_image,
        )
        .method(
            "drawRegion",
            "(Ljavax/microedition/lcdui/Image;IIIIIIII)V",
            draw_region,
        )
        .method("copyArea", "(IIIIIII)V", copy_area)
        .method("drawRGB", "([IIIIIIIZ)V", draw_rgb)
        .method("getDisplayColor", "(I)I", |ctx, a| {
            let color = a[1].int();
            Ok(Value::Int(data(ctx, a[0])?.state.display_color(color)))
        })
}
