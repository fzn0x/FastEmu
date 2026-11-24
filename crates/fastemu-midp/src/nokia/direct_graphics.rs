//! `com.nokia.mid.ui.DirectGraphics`, implemented by MIDP's own
//! `Graphics`.
//!
//! Nokia's `DirectGraphics` is a view of a `Graphics`: colour, clip and
//! translation are shared both ways, and `setARGBColor` sets the colour
//! `Graphics` draws with. Games also cast `(DirectGraphics) g`, which the
//! javadoc warns "may not work" but which worked on the phones. So, as in
//! freej2me-plus, `javax.microedition.lcdui.Graphics` itself implements
//! the interface on profiles with Nokia's UI API ([`implement`]), and
//! `DirectUtils.getDirectGraphics(g)` returns `g`. Every call draws on the
//! `Graphics`' own target with `fastemu-gfx`'s Nokia functions
//! (`docs/architecture/graphics.md`).

use std::sync::Arc;

use fastemu_gfx::nokia::{self, NATIVE_PIXEL_FORMAT};
use fastemu_gfx::{GfxError, Graphics};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use crate::lcdui::graphics::GraphicsData;
use crate::lcdui::image::{SharedSurface, surface_of};
use crate::lcdui::throw_gfx;

/// The interface's internal name.
pub const INTERFACE: &str = "com/nokia/mid/ui/DirectGraphics";
/// MIDP's `Graphics`, which implements it.
pub const GRAPHICS: &str = "javax/microedition/lcdui/Graphics";

const SET_ARGB_COLOR: &str = "(I)V";
const DRAW_IMAGE: &str = "(Ljavax/microedition/lcdui/Image;IIII)V";
const TRIANGLE: &str = "(IIIIIII)V";
const POLYGON: &str = "([II[IIII)V";
const DRAW_PIXELS_INT: &str = "([IZIIIIIIII)V";
const DRAW_PIXELS_BYTE: &str = "([B[BIIIIIIII)V";
const DRAW_PIXELS_SHORT: &str = "([SZIIIIIIII)V";
const GET_PIXELS_INT: &str = "([IIIIIIII)V";
const GET_PIXELS_BYTE: &str = "([B[BIIIIIII)V";
const GET_PIXELS_SHORT: &str = "([SIIIIIII)V";

/// The interface, with its constants.
pub fn interface() -> NativeClass {
    let int = |v| Value::Int(v);
    let mut class = NativeClass::interface(INTERFACE)
        .constant("FLIP_HORIZONTAL", "I", int(nokia::FLIP_HORIZONTAL))
        .constant("FLIP_VERTICAL", "I", int(nokia::FLIP_VERTICAL))
        .constant("ROTATE_90", "I", int(nokia::ROTATE_90))
        .constant("ROTATE_180", "I", int(nokia::ROTATE_180))
        .constant("ROTATE_270", "I", int(nokia::ROTATE_270))
        .constant("TYPE_BYTE_1_GRAY", "I", int(nokia::TYPE_BYTE_1_GRAY))
        .constant(
            "TYPE_BYTE_1_GRAY_VERTICAL",
            "I",
            int(nokia::TYPE_BYTE_1_GRAY_VERTICAL),
        )
        .constant("TYPE_BYTE_2_GRAY", "I", int(nokia::TYPE_BYTE_2_GRAY))
        .constant("TYPE_BYTE_4_GRAY", "I", int(nokia::TYPE_BYTE_4_GRAY))
        .constant("TYPE_BYTE_8_GRAY", "I", int(nokia::TYPE_BYTE_8_GRAY))
        .constant("TYPE_BYTE_332_RGB", "I", int(nokia::TYPE_BYTE_332_RGB))
        .constant(
            "TYPE_USHORT_4444_ARGB",
            "I",
            int(nokia::TYPE_USHORT_4444_ARGB),
        )
        .constant("TYPE_USHORT_444_RGB", "I", int(nokia::TYPE_USHORT_444_RGB))
        .constant("TYPE_USHORT_555_RGB", "I", int(nokia::TYPE_USHORT_555_RGB))
        .constant(
            "TYPE_USHORT_1555_ARGB",
            "I",
            int(nokia::TYPE_USHORT_1555_ARGB),
        )
        .constant("TYPE_USHORT_565_RGB", "I", int(nokia::TYPE_USHORT_565_RGB))
        .constant("TYPE_INT_888_RGB", "I", int(nokia::TYPE_INT_888_RGB))
        .constant("TYPE_INT_8888_ARGB", "I", int(nokia::TYPE_INT_8888_ARGB));
    for (name, desc) in [
        ("setARGBColor", SET_ARGB_COLOR),
        ("drawImage", DRAW_IMAGE),
        ("drawTriangle", TRIANGLE),
        ("fillTriangle", TRIANGLE),
        ("drawPolygon", POLYGON),
        ("fillPolygon", POLYGON),
        ("drawPixels", DRAW_PIXELS_INT),
        ("drawPixels", DRAW_PIXELS_BYTE),
        ("drawPixels", DRAW_PIXELS_SHORT),
        ("getPixels", GET_PIXELS_INT),
        ("getPixels", GET_PIXELS_BYTE),
        ("getPixels", GET_PIXELS_SHORT),
        ("getNativePixelFormat", "()I"),
        ("getAlphaComponent", "()I"),
    ] {
        class = class.abstract_method(name, desc);
    }
    class
}

/// Makes MIDP's `Graphics` class implement `DirectGraphics`.
pub fn implement(graphics: NativeClass) -> NativeClass {
    graphics
        .implements(INTERFACE)
        .method("setARGBColor", SET_ARGB_COLOR, |ctx, a| {
            let argb = a[1].int();
            state_mut(ctx, a[0])?.set_argb_color(argb);
            Ok(Value::Void)
        })
        .method("getAlphaComponent", "()I", |ctx, a| {
            Ok(Value::Int(graphics_of(ctx, a[0])?.state.alpha_component()))
        })
        .method("getNativePixelFormat", "()I", |ctx, a| {
            ctx.non_null(a[0])?;
            Ok(Value::Int(NATIVE_PIXEL_FORMAT))
        })
        .method("drawImage", DRAW_IMAGE, draw_image)
        .method("drawTriangle", TRIANGLE, |ctx, a| triangle(ctx, a, false))
        .method("fillTriangle", TRIANGLE, |ctx, a| triangle(ctx, a, true))
        .method("drawPolygon", POLYGON, |ctx, a| polygon(ctx, a, false))
        .method("fillPolygon", POLYGON, |ctx, a| polygon(ctx, a, true))
        .method("drawPixels", DRAW_PIXELS_INT, draw_pixels_int)
        .method("drawPixels", DRAW_PIXELS_BYTE, draw_pixels_byte)
        .method("drawPixels", DRAW_PIXELS_SHORT, draw_pixels_short)
        .method("getPixels", GET_PIXELS_INT, get_pixels_int)
        .method("getPixels", GET_PIXELS_BYTE, get_pixels_byte)
        .method("getPixels", GET_PIXELS_SHORT, get_pixels_short)
}

/// The payload of the `Graphics` `this`.
fn graphics_of<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a GraphicsData> {
    let this = ctx.non_null(this)?;
    ctx.payload::<GraphicsData>(this)
}

fn state_mut<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut Graphics> {
    let this = ctx.non_null(this)?;
    Ok(&mut ctx.payload_mut::<GraphicsData>(this)?.state)
}

/// A copy of the drawing state and the target of `this`, for calls that
/// need the `Ctx` mutably while they draw or read.
fn detached(ctx: &mut Ctx<'_>, this: Value) -> JResult<(Graphics, SharedSurface)> {
    let d = graphics_of(ctx, this)?;
    Ok((d.state.clone(), Arc::clone(&d.target)))
}

fn checked(ctx: &mut Ctx<'_>, result: Result<(), GfxError>) -> JResult<Value> {
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// The `int` arguments `a[from..to]`.
fn ints(a: &[Value], from: usize, to: usize) -> Vec<i32> {
    a[from..to].iter().map(|v| v.int()).collect()
}

fn draw_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let image = ctx.non_null(a[1])?;
    let v = ints(a, 2, 6);
    let source = surface_of(ctx, image)?;
    let d = ctx.payload::<GraphicsData>(this)?;
    let result = if Arc::ptr_eq(&source, &d.target) {
        // An image drawn onto itself: draw a copy, as Graphics.drawImage.
        let mut target = d.target.lock();
        let copy = target.clone();
        d.state
            .draw_image_manipulated(&mut target, &copy, v[0], v[1], v[2], v[3])
    } else {
        let source = source.lock();
        d.state
            .draw_image_manipulated(&mut d.target.lock(), &source, v[0], v[1], v[2], v[3])
    };
    checked(ctx, result)
}

fn triangle(ctx: &mut Ctx<'_>, a: &[Value], fill: bool) -> JResult<Value> {
    let v = ints(a, 1, 8);
    let d = graphics_of(ctx, a[0])?;
    let mut target = d.target.lock();
    if fill {
        d.state
            .fill_triangle_argb(&mut target, v[0], v[1], v[2], v[3], v[4], v[5], v[6]);
    } else {
        d.state
            .draw_triangle_argb(&mut target, v[0], v[1], v[2], v[3], v[4], v[5], v[6]);
    }
    Ok(Value::Void)
}

fn polygon(ctx: &mut Ctx<'_>, a: &[Value], fill: bool) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let xs = ctx.non_null(a[1])?;
    let ys = ctx.non_null(a[3])?;
    let (x_offset, y_offset, n, argb) = (a[2].int(), a[4].int(), a[5].int(), a[6].int());
    let d = ctx.payload::<GraphicsData>(this)?;
    let (x_points, y_points) = (ctx.array::<i32>(xs)?, ctx.array::<i32>(ys)?);
    let mut target = d.target.lock();
    let result = if fill {
        d.state
            .fill_polygon(&mut target, x_points, x_offset, y_points, y_offset, n, argb)
    } else {
        d.state
            .draw_polygon(&mut target, x_points, x_offset, y_points, y_offset, n, argb)
    };
    drop(target);
    checked(ctx, result)
}

fn draw_pixels_int(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let transparency = a[2].boolean();
    let v = ints(a, 3, 11);
    let d = ctx.payload::<GraphicsData>(this)?;
    let pixels = ctx.array::<i32>(pixels)?;
    let result = d.state.draw_pixels_int(
        &mut d.target.lock(),
        pixels,
        transparency,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
        v[7],
    );
    checked(ctx, result)
}

fn draw_pixels_short(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let transparency = a[2].boolean();
    let v = ints(a, 3, 11);
    let d = ctx.payload::<GraphicsData>(this)?;
    let pixels = ctx.array::<i16>(pixels)?;
    let result = d.state.draw_pixels_short(
        &mut d.target.lock(),
        pixels,
        transparency,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
        v[7],
    );
    checked(ctx, result)
}

fn draw_pixels_byte(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let mask = a[2].obj();
    let v = ints(a, 3, 11);
    let d = ctx.payload::<GraphicsData>(this)?;
    let pixels = ctx.array::<i8>(pixels)?;
    let mask = match mask {
        Some(mask) => Some(ctx.array::<i8>(mask)?),
        None => None,
    };
    let result = d.state.draw_pixels_byte(
        &mut d.target.lock(),
        pixels,
        mask,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
        v[7],
    );
    checked(ctx, result)
}

fn get_pixels_int(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let v = ints(a, 2, 9);
    let (state, target) = detached(ctx, a[0])?;
    let source = target.lock();
    let result = state.get_pixels_int(
        &source,
        ctx.array_mut::<i32>(pixels)?,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
    );
    drop(source);
    checked(ctx, result)
}

fn get_pixels_short(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let v = ints(a, 2, 9);
    let (state, target) = detached(ctx, a[0])?;
    let source = target.lock();
    let result = state.get_pixels_short(
        &source,
        ctx.array_mut::<i16>(pixels)?,
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
    );
    drop(source);
    checked(ctx, result)
}

/// `getPixels(byte[], byte[], ...)`. Two Java arrays cannot be borrowed
/// mutably at once, so both are read into copies and written back when
/// the call succeeds; a failed call leaves them unchanged, as it must.
fn get_pixels_byte(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    ctx.non_null(a[0])?;
    let pixels = ctx.non_null(a[1])?;
    let mask = a[2].obj();
    let v = ints(a, 3, 10);
    let (state, target) = detached(ctx, a[0])?;
    let mut pixel_copy = ctx.array::<i8>(pixels)?.to_vec();
    let mut mask_copy = match mask {
        Some(mask) => Some(ctx.array::<i8>(mask)?.to_vec()),
        None => None,
    };
    let result = state.get_pixels_byte(
        &target.lock(),
        &mut pixel_copy,
        mask_copy.as_deref_mut(),
        v[0],
        v[1],
        v[2],
        v[3],
        v[4],
        v[5],
        v[6],
    );
    if result.is_ok() {
        write_back(ctx, pixels, &pixel_copy)?;
        if let (Some(mask), Some(copy)) = (mask, mask_copy) {
            write_back(ctx, mask, &copy)?;
        }
    }
    checked(ctx, result)
}

fn write_back(ctx: &mut Ctx<'_>, array: ObjRef, bytes: &[i8]) -> JResult<()> {
    ctx.array_mut::<i8>(array)?.copy_from_slice(bytes);
    Ok(())
}
