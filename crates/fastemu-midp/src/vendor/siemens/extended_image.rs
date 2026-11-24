//! `com.siemens.mp.game.ExtendedImage`: pixel access to the black and
//! white (or black, white and transparent) pictures of Siemens games, and
//! drawing one straight to the display.
//!
//! The wrapped image is the game's own when it is mutable, so drawing on
//! either shows in both; an immutable image is copied into a mutable one.
//! Siemens names pixels by colour index: an image without transparent
//! pixels is one bit per pixel (0 white, 1 black), one with them is two
//! bits per pixel (0 transparent, 1 white, 2 black), as in freej2me-plus.
//! Packed pixels follow the bitmaps of [`super::bitmap`].

use fastemu_gfx::{Graphics, Surface, SurfaceKind};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::NATIVE_MEM;
use crate::lcdui::canvas;
use crate::lcdui::display;
use crate::lcdui::displayable::{self, Kind};
use crate::lcdui::font::default_font;
use crate::lcdui::image::{SharedSurface, new_image, surface_of};
use crate::lcdui::{Rect, throw_gfx};

pub(crate) const CLASS: &str = "com/siemens/mp/game/ExtendedImage";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";

/// The colours of the one-bit indexes.
const PALETTE_1: [u32; 2] = [0xFFFF_FFFF, 0xFF00_0000];
/// The colours of the two-bit indexes.
const PALETTE_2: [u32; 4] = [0x00FF_FFFF, 0xFFFF_FFFF, 0xFF00_0000, 0xFF00_0000];

/// What an `ExtendedImage` holds.
#[derive(Default)]
pub(crate) struct ExtendedData {
    /// The wrapped MIDP image.
    pub image: Option<ObjRef>,
    /// Whether pixels are two bits (with transparency) rather than one.
    two_bit: bool,
}

impl Payload for ExtendedData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.image);
    }
}

/// The MIDP image an `ExtendedImage` wraps.
pub(crate) fn image_of(ctx: &mut Ctx<'_>, extended: ObjRef) -> JResult<ObjRef> {
    match ctx.payload::<ExtendedData>(extended)?.image {
        Some(image) => Ok(image),
        None => ctx.throw("java/lang/IllegalStateException", "an empty ExtendedImage"),
    }
}

/// The pixels and the pixel size of `this`.
fn pixels(ctx: &mut Ctx<'_>, this: Value) -> JResult<(SharedSurface, bool)> {
    let this = ctx.non_null(this)?;
    let two_bit = ctx.payload::<ExtendedData>(this)?.two_bit;
    let image = image_of(ctx, this)?;
    Ok((surface_of(ctx, image)?, two_bit))
}

fn colour(two_bit: bool, index: i32) -> u32 {
    if two_bit {
        PALETTE_2[(index & 3) as usize]
    } else {
        PALETTE_1[(index & 1) as usize]
    }
}

fn index_of(two_bit: bool, pixel: u32) -> i32 {
    let white = pixel & 0x00FF_FFFF == 0x00FF_FFFF;
    match (two_bit, pixel >> 24 == 0, white) {
        (true, true, _) => 0,
        (true, false, true) => 1,
        (true, false, false) => 2,
        (false, _, true) => 0,
        (false, _, false) => 1,
    }
}

fn construct(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let image = ctx.non_null(a[1])?;
    let shared = surface_of(ctx, image)?;
    let (width, mutable, opaque) = {
        let s = shared.lock();
        (s.width(), s.is_mutable(), s.is_opaque())
    };
    if width % 8 != 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, "the width is not a multiple of 8");
    }
    let image = if mutable {
        image
    } else {
        let copy = shared.lock().clone().into_kind(SurfaceKind::Mutable);
        new_image(ctx, copy)?
    };
    let data = ctx.payload_mut::<ExtendedData>(this)?;
    data.image = Some(image);
    data.two_bit = !opaque;
    Ok(Value::Void)
}

/// Checks the packed-pixel region arguments of `setPixels` and
/// `getPixelBytes`.
fn check_region(ctx: &mut Ctx<'_>, x: i32, width: i32, height: i32) -> JResult<()> {
    if x % 8 != 0 || width % 8 != 0 || width < 0 || height < 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, "x and width must be multiples of 8");
    }
    Ok(())
}

/// The array index and shift of pixel `n` of a packed region.
fn packed(two_bit: bool, n: usize) -> (usize, u32) {
    if two_bit {
        (n / 4, 6 - 2 * (n % 4) as u32)
    } else {
        (n / 8, 7 - (n % 8) as u32)
    }
}

fn set_pixels(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let array = ctx.non_null(a[1])?;
    let [x, y, w, h] = [a[2].int(), a[3].int(), a[4].int(), a[5].int()];
    check_region(ctx, x, w, h)?;
    let (shared, two_bit) = pixels(ctx, a[0])?;
    let bytes = ctx.array::<i8>(array)?.to_vec();
    let mut surface = shared.lock();
    let (sw, sh) = (surface.width() as i64, surface.height() as i64);
    let mask = if two_bit { 3 } else { 1 };
    surface.edit_pixels(|to| {
        for row in 0..i64::from(h) {
            for col in 0..i64::from(w) {
                let (px, py) = (i64::from(x) + col, i64::from(y) + row);
                let (index, shift) = packed(two_bit, (row * i64::from(w) + col) as usize);
                if px < 0 || py < 0 || px >= sw || py >= sh || index >= bytes.len() {
                    continue;
                }
                let value = (i32::from(bytes[index] as u8) >> shift) & mask;
                to[(py * sw + px) as usize] = colour(two_bit, value);
            }
        }
    });
    Ok(Value::Void)
}

fn get_pixel_bytes(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let array = ctx.non_null(a[1])?;
    let [x, y, w, h] = [a[2].int(), a[3].int(), a[4].int(), a[5].int()];
    check_region(ctx, x, w, h)?;
    let (shared, two_bit) = pixels(ctx, a[0])?;
    let mut bytes = ctx.array::<i8>(array)?.to_vec();
    {
        let surface = shared.lock();
        for row in 0..h {
            for col in 0..w {
                let n = (i64::from(row) * i64::from(w) + i64::from(col)) as usize;
                let (index, shift) = packed(two_bit, n);
                if index >= bytes.len() {
                    continue;
                }
                let value = u32::try_from(x + col)
                    .ok()
                    .zip(u32::try_from(y + row).ok())
                    .and_then(|(px, py)| surface.pixel(px, py))
                    .map_or(0, |p| index_of(two_bit, p));
                let field = if two_bit { 3 } else { 1 };
                let byte = (i32::from(bytes[index] as u8) & !(field << shift)) | (value << shift);
                bytes[index] = byte as u8 as i8;
            }
        }
    }
    ctx.array_mut::<i8>(array)?.copy_from_slice(&bytes);
    Ok(Value::Void)
}

/// `blitToScreen(x, y)`: draws the image onto the shown canvas at once,
/// outside `paint`, and presents the screen.
fn blit_to_screen(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (x, y) = (a[1].int(), a[2].int());
    let (shared, _) = pixels(ctx, a[0])?;
    let Some(canvas) = display::current(ctx) else {
        return Ok(Value::Void);
    };
    if displayable::data(ctx, canvas)?.kind() != Kind::Canvas {
        return Ok(Value::Void);
    }
    let font = default_font(ctx);
    let buffer = canvas::buffer(ctx, canvas)?;
    let mut screen = buffer.lock().clone();
    let source: Surface = shared.lock().clone();
    let drawn =
        Graphics::new(&screen, font).and_then(|g| g.draw_image(&mut screen, &source, x, y, 0));
    if let Err(e) = drawn {
        return Err(throw_gfx(ctx, &e));
    }
    let region = Rect::new(x, y, source.width() as i32, source.height() as i32);
    canvas::flush(ctx, canvas, &screen, region)?;
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .extends(NATIVE_MEM)
        .payload::<ExtendedData>()
        .method("<init>", "(Ljavax/microedition/lcdui/Image;)V", construct)
        .method("getPixel", "(II)I", |ctx, a| {
            let (shared, two_bit) = pixels(ctx, a[0])?;
            let pixel = u32::try_from(a[1].int())
                .ok()
                .zip(u32::try_from(a[2].int()).ok())
                .and_then(|(x, y)| shared.lock().pixel(x, y));
            Ok(Value::Int(pixel.map_or(0, |p| index_of(two_bit, p))))
        })
        .method("setPixel", "(IIB)V", |ctx, a| {
            let (shared, two_bit) = pixels(ctx, a[0])?;
            let (x, y) = (a[1].int(), a[2].int());
            let mut surface = shared.lock();
            let (w, h) = (surface.width() as i32, surface.height() as i32);
            if (0..w).contains(&x) && (0..h).contains(&y) {
                let c = colour(two_bit, a[3].int());
                surface.edit_pixels(|p| p[(y * w + x) as usize] = c);
            }
            Ok(Value::Void)
        })
        .method("setPixels", "([BIIII)V", set_pixels)
        .method("getPixelBytes", "([BIIII)V", get_pixel_bytes)
        .method("clear", "(B)V", |ctx, a| {
            let (shared, two_bit) = pixels(ctx, a[0])?;
            let c = colour(two_bit, a[1].int());
            shared.lock().edit_pixels(|p| p.fill(c));
            Ok(Value::Void)
        })
        .method("blitToScreen", "(II)V", blit_to_screen)
        .method(
            "getImage",
            "()Ljavax/microedition/lcdui/Image;",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                Ok(image_of(ctx, this)?.into())
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_and_colours_round_trip() {
        for two_bit in [false, true] {
            let count = if two_bit { 3 } else { 2 };
            for index in 0..count {
                assert_eq!(index_of(two_bit, colour(two_bit, index)), index);
            }
        }
        assert_eq!(colour(true, 3), 0xFF00_0000);
        assert_eq!(index_of(false, 0x1234_5678), 1);
    }

    #[test]
    fn packed_pixels_start_at_the_high_bits() {
        assert_eq!(packed(false, 0), (0, 7));
        assert_eq!(packed(false, 9), (1, 6));
        assert_eq!(packed(true, 0), (0, 6));
        assert_eq!(packed(true, 5), (1, 4));
    }
}
