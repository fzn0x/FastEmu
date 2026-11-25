//! Siemens' bitmaps and `com.siemens.mp.ui.Image`, the helpers that make
//! MIDP images from them.
//!
//! Siemens games carry black and white pictures as packed bits, as J2ME
//! Loader and freej2me-plus read them: rows of whole bytes, the left
//! pixel in the high bit.
//!
//! - One bit per pixel: a set bit is black, a clear one white. A mask of
//!   the same layout makes pixels whose bit is clear transparent.
//! - Two bits per pixel (`createTransparentImageFromBitmap`): 0 is
//!   transparent, 1 white, 2 and 3 black.
//! - `createRGBImage` takes a byte per pixel, `RRRGGGBB`; the byte `0xC0`
//!   is transparent, as freej2me-plus found games use it.
//!
//! A mask given as a MIDP image is transparent where it is white.

use fastemu_gfx::{GfxError, MAX_PIXELS, Surface, SurfaceKind, Transform};
use fastemu_jvm::{
    Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Tracer, Value, classfile::access,
};

use super::NATIVE_MEM;
use crate::lcdui::image::{
    SharedSurface, image_from_array, image_from_resource, new_image, reserve_pixels, surface_of,
};
use crate::lcdui::throw_gfx;
use crate::suite::source::normalize;

const UI_IMAGE: &str = "com/siemens/mp/ui/Image";
const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";
/// `COLOR_BMP_8BIT`: a byte per pixel, `RRRGGGBB`.
const COLOR_BMP_8BIT: i32 = 5;
/// The 8-bit colour that stands for a transparent pixel.
const TRANSPARENT_332: u8 = 0xC0;

const WHITE: u32 = 0xFFFF_FFFF;
const BLACK: u32 = 0xFF00_0000;

/// The number of pixels of a `width` by `height` picture, refused when it
/// is empty or larger than any image may be.
fn pixel_count(width: i32, height: i32) -> Result<usize, GfxError> {
    if width <= 0 || height <= 0 {
        return Err(GfxError::IllegalArgument("bitmap size must be positive"));
    }
    let count = i64::from(width) * i64::from(height);
    if count > MAX_PIXELS as i64 {
        return Err(GfxError::OutOfMemory);
    }
    Ok(count as usize)
}

/// The bytes a packed `width` by `height` picture takes, rows of whole
/// bytes. The size must have passed [`pixel_count`].
fn packed_len(width: i32, height: i32, bits_per_pixel: u32) -> usize {
    let per_byte = (8 / bits_per_pixel) as usize;
    (width as usize).div_ceil(per_byte) * height as usize
}

/// Asks the heap for the picture that `bytes` bytes of `bits_per_pixel`
/// packed pixels unpack to, before it is made. A picture takes 32 times
/// the bytes of a one-bit bitmap, so an array that fits the heap can make
/// an image that does not. A size or array the unpacking refuses passes,
/// to be refused with the exception MIDP names.
fn reserve_unpacked(
    ctx: &mut Ctx<'_>,
    bytes: usize,
    width: i32,
    height: i32,
    bits_per_pixel: u32,
) -> JResult<()> {
    if pixel_count(width, height).is_err() || packed_len(width, height, bits_per_pixel) > bytes {
        return Ok(());
    }
    reserve_pixels(ctx, width.into(), height.into())
}

/// Reads `bits_per_pixel` packed pixels, rows of whole bytes, high bits
/// first, and turns each value into a colour.
fn unpack(
    bytes: &[i8],
    width: i32,
    height: i32,
    bits_per_pixel: u32,
    colour: impl Fn(u32, usize, u32) -> u32,
) -> Result<Vec<u32>, GfxError> {
    let count = pixel_count(width, height)?;
    let per_byte = (8 / bits_per_pixel) as usize;
    let (w, h) = (width as usize, height as usize);
    if packed_len(width, height, bits_per_pixel) > bytes.len() {
        return Err(GfxError::ArrayIndexOutOfBounds);
    }
    let stride = w.div_ceil(per_byte);
    let mut pixels = Vec::with_capacity(count);
    let field = (1 << bits_per_pixel) - 1;
    for y in 0..h {
        for x in 0..w {
            let index = y * stride + x / per_byte;
            let shift = 8 - bits_per_pixel * (x % per_byte + 1) as u32;
            let value = (u32::from(bytes[index] as u8) >> shift) & field;
            pixels.push(colour(value, index, shift));
        }
    }
    Ok(pixels)
}

/// An immutable picture of `pixels`.
fn surface(pixels: Vec<u32>, width: i32, height: i32) -> Result<Surface, GfxError> {
    Surface::from_pixels(width as u32, height as u32, pixels, SurfaceKind::Immutable)
}

/// A one-bit bitmap, with an optional one-bit mask of the same layout.
pub(crate) fn from_1bpp(
    bits: &[i8],
    mask: Option<&[i8]>,
    width: i32,
    height: i32,
) -> Result<Surface, GfxError> {
    pixel_count(width, height)?;
    if mask.is_some_and(|m| m.len() < packed_len(width, height, 1)) {
        return Err(GfxError::ArrayIndexOutOfBounds);
    }
    let opaque = |index: usize, shift: u32| mask.is_none_or(|m| (m[index] as u8 >> shift) & 1 == 1);
    let pixels = unpack(bits, width, height, 1, |value, index, shift| {
        if !opaque(index, shift) {
            0
        } else if value == 1 {
            BLACK
        } else {
            WHITE
        }
    })?;
    surface(pixels, width, height)
}

/// A two-bit bitmap: transparent, white or black.
pub(crate) fn from_2bpp(bits: &[i8], width: i32, height: i32) -> Result<Surface, GfxError> {
    let pixels = unpack(bits, width, height, 2, |value, _, _| match value {
        0 => 0x00FF_FFFF,
        1 => WHITE,
        _ => BLACK,
    })?;
    surface(pixels, width, height)
}

/// A byte per pixel, `RRRGGGBB`, with `0xC0` transparent.
fn from_332(bytes: &[i8], width: i32, height: i32) -> Result<Surface, GfxError> {
    let count = pixel_count(width, height)?;
    let bytes = bytes.get(..count).ok_or(GfxError::ArrayIndexOutOfBounds)?;
    let scale = |v: u32, max: u32| v * 255 / max;
    let pixels = bytes
        .iter()
        .map(|&b| {
            let b = b as u8;
            if b == TRANSPARENT_332 {
                return 0;
            }
            let b = u32::from(b);
            0xFF00_0000 | scale(b >> 5, 7) << 16 | scale((b >> 2) & 7, 7) << 8 | scale(b & 3, 3)
        })
        .collect();
    surface(pixels, width, height)
}

/// `image` with the pixels that are white in `mask` made transparent.
/// The two must have the same size.
pub(crate) fn apply_mask(image: &Surface, mask: &Surface) -> Result<Surface, GfxError> {
    let pixels = image
        .pixels()
        .iter()
        .zip(mask.pixels())
        .map(|(&p, &m)| if m & 0x00FF_FFFF == 0x00FF_FFFF { 0 } else { p })
        .collect();
    Surface::from_pixels(
        image.width(),
        image.height(),
        pixels,
        SurfaceKind::Immutable,
    )
}

/// A copy of the pixels of a Java image, `None` for `null`, so no lock is
/// held afterwards.
pub(crate) fn copy_of(ctx: &Ctx<'_>, image: Option<ObjRef>) -> JResult<Option<Surface>> {
    match image {
        Some(image) => Ok(Some(surface_of(ctx, image)?.lock().clone())),
        None => Ok(None),
    }
}

/// A new Java `Image` holding `result`, or the exception its error names.
pub(crate) fn image_or_throw(
    ctx: &mut Ctx<'_>,
    result: Result<Surface, GfxError>,
) -> JResult<ObjRef> {
    match result {
        Ok(surface) => new_image(ctx, surface),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

/// The bytes of a `byte[]` argument, `NullPointerException` for `null`.
fn bytes(ctx: &mut Ctx<'_>, value: Value) -> JResult<Vec<i8>> {
    let array = ctx.non_null(value)?;
    Ok(ctx.array::<i8>(array)?.to_vec())
}

/// Loads the image resource `name`, as `Image.createImage(String)`:
/// `IOException` when it is missing or does not decode.
fn load(ctx: &mut Ctx<'_>, name: Value) -> JResult<ObjRef> {
    let name = ctx.non_null(name)?;
    let name = ctx.string(name)?;
    let path = normalize(&name);
    match image_from_resource(ctx, &path)? {
        Some(Ok(image)) => Ok(image),
        Some(Err(GfxError::OutOfMemory)) => Err(throw_gfx(ctx, &GfxError::OutOfMemory)),
        Some(Err(e)) => Err(io_error(ctx, &e.to_string())),
        None => Err(io_error(ctx, &format!("resource not found: {name}"))),
    }
}

fn io_error(ctx: &mut Ctx<'_>, message: &str) -> Throw {
    ctx.new_exception("java/io/IOException", Some(message))
}

/// `createRGBImage(imageData, width, height, BitmapType)`.
fn rgb_image(
    ctx: &mut Ctx<'_>,
    data: Value,
    width: i32,
    height: i32,
    kind: i32,
) -> JResult<ObjRef> {
    let data = bytes(ctx, data)?;
    if width <= 0 || height <= 0 {
        return ctx.throw("java/lang/ArrayIndexOutOfBoundsException", "size");
    }
    if kind != COLOR_BMP_8BIT {
        return Err(io_error(
            ctx,
            &format!("bitmap type {kind} is not supported"),
        ));
    }
    reserve_unpacked(ctx, data.len(), width, height, 8)?;
    image_or_throw(ctx, from_332(&data, width, height))
}

/// Mirrors an image in place.
fn mirror(ctx: &mut Ctx<'_>, image: Value, transform: Transform) -> JResult<Value> {
    let image = ctx.non_null(image)?;
    let pixels: SharedSurface = surface_of(ctx, image)?;
    let mut surface = pixels.lock();
    let (w, h) = (surface.width() as i64, surface.height() as i64);
    let source = surface.pixels().to_vec();
    surface.edit_pixels(|to| {
        for y in 0..h {
            for x in 0..w {
                let (tx, ty) = transform.apply(x, y, w, h);
                to[(ty * w + tx) as usize] = source[(y * w + x) as usize];
            }
        }
    });
    Ok(Value::Void)
}

/// What a `com.siemens.mp.ui.Image` holds: the MIDP image it stands for.
#[derive(Default)]
struct UiImage {
    image: Option<ObjRef>,
}

impl Payload for UiImage {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.image);
    }
}

/// Makes `this` stand for the MIDP image `image`.
fn wrap(ctx: &mut Ctx<'_>, this: Value, image: ObjRef) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<UiImage>(this)?.image = Some(image);
    Ok(Value::Void)
}

fn wrapped(ctx: &mut Ctx<'_>, this: Value) -> JResult<Option<ObjRef>> {
    let this = ctx.non_null(this)?;
    Ok(ctx.payload::<UiImage>(this)?.image)
}

fn dimension(ctx: &mut Ctx<'_>, this: Value, f: fn(&Surface) -> u32) -> JResult<Value> {
    Ok(Value::Int(match wrapped(ctx, this)? {
        Some(image) => f(&surface_of(ctx, image)?.lock()) as i32,
        None => 0,
    }))
}

/// `com.siemens.mp.ui.Image`.
pub fn class() -> NativeClass {
    let ui = format!("L{UI_IMAGE};");
    NativeClass::new(UI_IMAGE)
        .extends(NATIVE_MEM)
        .payload::<UiImage>()
        .constant("COLOR_BMP_8BIT", "I", Value::Int(COLOR_BMP_8BIT))
        .method_with("<init>", "()V", access::PROTECTED, |_, _| Ok(Value::Void))
        .method("<init>", "(II)V", |ctx, a| {
            reserve_pixels(ctx, a[1].int().into(), a[2].int().into())?;
            let image = image_or_throw(ctx, Surface::new_mutable(a[1].int(), a[2].int()))?;
            wrap(ctx, a[0], image)
        })
        .method("<init>", &format!("({ui})V"), |ctx, a| {
            let other = ctx.non_null(a[1])?;
            let copy = match ctx.payload::<UiImage>(other)?.image {
                Some(image) => {
                    let pixels = surface_of(ctx, image)?;
                    let (width, height) = {
                        let surface = pixels.lock();
                        (surface.width(), surface.height())
                    };
                    reserve_pixels(ctx, width.into(), height.into())?;
                    let copy = pixels.lock().clone();
                    Some(copy)
                }
                None => None,
            };
            match copy {
                Some(copy) => {
                    let image = new_image(ctx, copy)?;
                    wrap(ctx, a[0], image)
                }
                None => Ok(Value::Void),
            }
        })
        .method("<init>", "([BII)V", |ctx, a| {
            let bits = bytes(ctx, a[1])?;
            reserve_unpacked(ctx, bits.len(), a[2].int(), a[3].int(), 1)?;
            let image = image_or_throw(ctx, from_1bpp(&bits, None, a[2].int(), a[3].int()))?;
            wrap(ctx, a[0], image)
        })
        .method("<init>", "([BIIZ)V", |ctx, a| {
            let bits = bytes(ctx, a[1])?;
            let (w, h) = (a[2].int(), a[3].int());
            reserve_unpacked(ctx, bits.len(), w, h, if a[4].boolean() { 2 } else { 1 })?;
            let surface = if a[4].boolean() {
                from_2bpp(&bits, w, h)
            } else {
                from_1bpp(&bits, None, w, h)
            };
            let image = image_or_throw(ctx, surface)?;
            wrap(ctx, a[0], image)
        })
        .method("<init>", "(Ljava/lang/String;Z)V", |ctx, a| {
            // Scaling to the screen is not done: the picture keeps its size.
            let image = load(ctx, a[1])?;
            wrap(ctx, a[0], image)
        })
        .method("<init>", "([B)V", |ctx, a| {
            let data = ctx.non_null(a[1])?;
            let image = match image_from_array(ctx, data, None, SurfaceKind::Immutable)? {
                Ok(image) => image,
                Err(e) => return Err(throw_gfx(ctx, &e)),
            };
            wrap(ctx, a[0], image)
        })
        .method("<init>", "([BIII)V", |ctx, a| {
            let image = rgb_image(ctx, a[1], a[2].int(), a[3].int(), a[4].int())?;
            wrap(ctx, a[0], image)
        })
        .method("getWidth", "()I", |ctx, a| {
            dimension(ctx, a[0], Surface::width)
        })
        .method("getHeight", "()I", |ctx, a| {
            dimension(ctx, a[0], Surface::height)
        })
        .static_method(
            "createImageWithScaling",
            &format!("(Ljava/lang/String;){IMAGE}"),
            |ctx, a| Ok(load(ctx, a[0])?.into()),
        )
        .static_method(
            "createImageWithoutScaling",
            &format!("(Ljava/lang/String;){IMAGE}"),
            |ctx, a| Ok(load(ctx, a[0])?.into()),
        )
        .static_method(
            "createImageFromBitmap",
            &format!("([BII){IMAGE}"),
            |ctx, a| {
                let bits = bytes(ctx, a[0])?;
                reserve_unpacked(ctx, bits.len(), a[1].int(), a[2].int(), 1)?;
                let surface = from_1bpp(&bits, None, a[1].int(), a[2].int());
                Ok(image_or_throw(ctx, surface)?.into())
            },
        )
        .static_method("createRGBImage", &format!("([BIII){IMAGE}"), |ctx, a| {
            Ok(rgb_image(ctx, a[0], a[1].int(), a[2].int(), a[3].int())?.into())
        })
        .static_method(
            "createTransparentImageFromBitmap",
            &format!("([BII){IMAGE}"),
            |ctx, a| {
                let bits = bytes(ctx, a[0])?;
                reserve_unpacked(ctx, bits.len(), a[1].int(), a[2].int(), 2)?;
                let surface = from_2bpp(&bits, a[1].int(), a[2].int());
                Ok(image_or_throw(ctx, surface)?.into())
            },
        )
        .static_method(
            "mirrorImageHorizontally",
            &format!("({IMAGE})V"),
            |ctx, a| mirror(ctx, a[0], Transform::Mirror),
        )
        .static_method("mirrorImageVertically", &format!("({IMAGE})V"), |ctx, a| {
            mirror(ctx, a[0], Transform::MirrorRot180)
        })
        .method_with(
            "setNativeImage",
            &format!("({IMAGE}{ui})V"),
            access::PROTECTED | access::STATIC,
            |ctx, a| {
                let image = ctx.non_null(a[0])?;
                wrap(ctx, a[1], image)
            },
        )
        .static_method("getNativeImage", &format!("({IMAGE}){ui}"), |ctx, a| {
            let image = ctx.non_null(a[0])?;
            let class = ctx.find_class(UI_IMAGE)?;
            let ui = ctx.new_object(class)?;
            ctx.payload_mut::<UiImage>(ui)?.image = Some(image);
            Ok(ui.into())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_bit_bitmaps_are_black_where_set() {
        let s = from_1bpp(&[0b1010_0000_u8 as i8, 0x01], None, 9, 1).unwrap();
        assert_eq!(&s.pixels()[..4], &[BLACK, WHITE, BLACK, WHITE]);
        assert_eq!(s.pixels()[8], WHITE);
        assert!(s.is_opaque());
        let s = from_1bpp(&[0b1100_0000_u8 as i8], Some(&[0b1000_0000_u8 as i8]), 2, 1).unwrap();
        assert_eq!(s.pixels(), &[BLACK, 0]);
        assert_eq!(
            from_1bpp(&[0], None, 8, 2).unwrap_err(),
            GfxError::ArrayIndexOutOfBounds
        );
        assert!(from_1bpp(&[0], None, 0, 1).is_err());
    }

    #[test]
    fn two_bit_bitmaps_have_transparent_white_and_black() {
        let s = from_2bpp(&[0b00_01_10_11], 4, 1).unwrap();
        assert_eq!(s.pixels(), &[0x00FF_FFFF, WHITE, BLACK, BLACK]);
    }

    #[test]
    fn rgb332_spreads_each_channel_and_keeps_c0_transparent() {
        let s = from_332(&[0xE0_u8 as i8, 0x1C, 0x03, 0xC0_u8 as i8], 2, 2).unwrap();
        assert_eq!(s.pixels(), &[0xFFFF_0000, 0xFF00_FF00, 0xFF00_00FF, 0]);
        assert!(from_332(&[0; 3], 2, 2).is_err());
    }

    #[test]
    fn white_mask_pixels_are_transparent() {
        let image = from_1bpp(&[0b1100_0000_u8 as i8], None, 2, 1).unwrap();
        let mask = from_1bpp(&[0b1000_0000_u8 as i8], None, 2, 1).unwrap();
        assert_eq!(apply_mask(&image, &mask).unwrap().pixels(), &[BLACK, 0]);
    }
}
