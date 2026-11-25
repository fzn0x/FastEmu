//! `javax.microedition.lcdui.Image` over `fastemu-gfx` surfaces.
//!
//! An image's pixels sit behind an `Arc<Mutex<Surface>>` in its payload,
//! so a `Graphics` drawing on a mutable image and a `Graphics` drawing the
//! image elsewhere reach the same pixels, and natives can hold the pixels
//! while they borrow other parts of the VM.

use std::sync::Arc;

use fastemu_gfx::{Surface, SurfaceKind};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value};
use parking_lot::Mutex;

use super::graphics::new_graphics;
use super::throw_gfx;
use crate::env::env;
use crate::suite::source::normalize;

const CLASS: &str = "javax/microedition/lcdui/Image";

/// Pixels shared between an image and the `Graphics` objects that draw on
/// it.
pub type SharedSurface = Arc<Mutex<Surface>>;

/// What a Java `Image` holds.
pub struct ImageData {
    pub surface: SharedSurface,
}

impl Payload for ImageData {
    fn heap_bytes(&self) -> usize {
        let surface = self.surface.lock();
        surface.width() as usize * surface.height() as usize * 4
    }
}

/// A new Java `Image` holding `surface`.
pub fn new_image(ctx: &mut Ctx<'_>, surface: Surface) -> JResult<ObjRef> {
    let class = ctx.find_class(CLASS)?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(
        obj,
        ImageData {
            surface: Arc::new(Mutex::new(surface)),
        },
    );
    Ok(obj)
}

/// The pixels of a Java `Image`.
pub fn surface_of(ctx: &Ctx<'_>, image: ObjRef) -> JResult<SharedSurface> {
    Ok(Arc::clone(&ctx.payload::<ImageData>(image)?.surface))
}

/// Asks the heap for the pixels of a `width` by `height` image before any
/// of them is made, so one that the heap could never hold is an
/// `OutOfMemoryError` and not a host allocation of up to 128 MiB. A size
/// that is not positive passes: the surface refuses it with the
/// `IllegalArgumentException` MIDP names.
pub(crate) fn reserve_pixels(ctx: &mut Ctx<'_>, width: i64, height: i64) -> JResult<()> {
    if width <= 0 || height <= 0 {
        return Ok(());
    }
    let bytes = (width as u64)
        .saturating_mul(height as u64)
        .saturating_mul(4);
    ctx.check_alloc(usize::try_from(bytes).unwrap_or(usize::MAX))
}

/// [`reserve_pixels`] for the image `data` declares in its header, read
/// without decoding it. A file whose size cannot be read goes to the
/// decoder, which refuses it for the same reason. A small file can declare
/// an image of 128 MiB, so the decoder must not be the first to look.
pub(crate) fn reserve_decoded(ctx: &mut Ctx<'_>, data: &[u8]) -> JResult<()> {
    match fastemu_gfx::probe_size(data) {
        Some((width, height)) => reserve_pixels(ctx, width.into(), height.into()),
        None => Ok(()),
    }
}

/// [`reserve_decoded`] for the part of `data` that `Image.createImage(byte[],
/// int, int)` reads. A range outside the array passes: the decoder refuses
/// it with the exception MIDP names.
pub(crate) fn reserve_decoded_range(
    ctx: &mut Ctx<'_>,
    data: &[u8],
    offset: i32,
    length: i32,
) -> JResult<()> {
    let range = usize::try_from(offset)
        .ok()
        .zip(usize::try_from(length).ok())
        .and_then(|(offset, length)| data.get(offset..offset.checked_add(length)?));
    match range {
        Some(range) => reserve_decoded(ctx, range),
        None => Ok(()),
    }
}

/// Turns a decode or size error of `createImage(String)` and
/// `createImage(InputStream)` into the `IOException` MIDP names, keeping
/// `OutOfMemoryError` as it is.
fn as_io_error(ctx: &mut Ctx<'_>, e: &fastemu_gfx::GfxError) -> fastemu_jvm::Throw {
    match e {
        fastemu_gfx::GfxError::OutOfMemory => throw_gfx(ctx, e),
        _ => ctx.new_exception("java/io/IOException", Some(&e.to_string())),
    }
}

/// Decodes `bytes` with the VM lock released, as image decoding is slow
/// and touches nothing of the VM.
fn decode(ctx: &mut Ctx<'_>, bytes: Vec<u8>) -> Result<Surface, fastemu_gfx::GfxError> {
    ctx.without_lock(move || Surface::decode(&bytes))
}

fn create_from_resource(ctx: &mut Ctx<'_>, name: Value) -> JResult<Value> {
    let name = ctx.non_null(name)?;
    let name = ctx.string(name)?;
    let path = normalize(&name);
    let source = Arc::clone(&env(ctx).source);
    let bytes = ctx.without_lock(|| source.resource(&path));
    let Some(bytes) = bytes else {
        return ctx.throw(
            "java/io/IOException",
            &format!("resource not found: {name}"),
        );
    };
    reserve_decoded(ctx, &bytes)?;
    match decode(ctx, bytes) {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(as_io_error(ctx, &e)),
    }
}

/// The most bytes `Image.createImage(InputStream)` reads. The PNG and JPEG
/// files of games are far smaller; the largest in the test corpus is
/// 144 KB.
const MAX_IMAGE_FILE: usize = 16 * 1024 * 1024;

/// How many reads in a row may return no bytes before the stream counts as
/// giving no data. `InputStream.read` blocks until it has a byte, so a read
/// that returns 0 again and again is a stream that is broken.
const MAX_EMPTY_READS: u32 = 1000;

/// Reads a Java `InputStream` to its end through its `read([BII)I`, at most
/// [`MAX_IMAGE_FILE`] bytes: an image stream that goes on is an
/// `IOException`, as a phone's would end in one, and not a Vec that grows
/// until the host is out of memory.
fn read_stream(ctx: &mut Ctx<'_>, stream: ObjRef) -> JResult<Vec<u8>> {
    let buffer = ctx.new_array(fastemu_jvm::ArrayType::Byte, 4096)?;
    let mut bytes = Vec::new();
    let mut empty_reads = 0;
    loop {
        let n = ctx
            .call_virtual(
                stream,
                "read",
                "([BII)I",
                &[buffer.into(), Value::Int(0), Value::Int(4096)],
            )?
            .int();
        if n < 0 {
            break;
        }
        let chunk = ctx.array::<i8>(buffer)?;
        let n = (n as usize).min(chunk.len());
        if n == 0 {
            empty_reads += 1;
            if empty_reads >= MAX_EMPTY_READS {
                return ctx.throw("java/io/IOException", "the stream gives no data");
            }
            continue;
        }
        empty_reads = 0;
        if bytes.len() + n > MAX_IMAGE_FILE {
            return ctx.throw(
                "java/io/IOException",
                "the image data is larger than 16 MiB",
            );
        }
        bytes.extend(chunk[..n].iter().map(|&b| b as u8));
    }
    Ok(bytes)
}

fn create_from_stream(ctx: &mut Ctx<'_>, stream: Value) -> JResult<Value> {
    let stream = ctx.non_null(stream)?;
    let bytes = read_stream(ctx, stream)?;
    reserve_decoded(ctx, &bytes)?;
    match decode(ctx, bytes) {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(as_io_error(ctx, &e)),
    }
}

fn create_from_bytes(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let data = ctx.non_null(a[0])?;
    let (offset, length) = (a[1].int(), a[2].int());
    let bytes: Vec<u8> = ctx.array::<i8>(data)?.iter().map(|&b| b as u8).collect();
    reserve_decoded_range(ctx, &bytes, offset, length)?;
    let result = ctx.without_lock(move || Surface::decode_range(&bytes, offset, length));
    match result {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_copy(ctx: &mut Ctx<'_>, source: Value) -> JResult<Value> {
    let source = ctx.non_null(source)?;
    let pixels = surface_of(ctx, source)?;
    let (width, height, mutable) = {
        let surface = pixels.lock();
        (surface.width(), surface.height(), surface.is_mutable())
    };
    if !mutable {
        // MIDP allows handing back an immutable source itself.
        return Ok(source.into());
    }
    reserve_pixels(ctx, width.into(), height.into())?;
    let copy = pixels.lock().to_immutable();
    Ok(new_image(ctx, copy)?.into())
}

fn create_region(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let source = ctx.non_null(a[0])?;
    let pixels = surface_of(ctx, source)?;
    let (x, y, width, height) = (a[1].int(), a[2].int(), a[3].int(), a[4].int());
    let (source_width, source_height) = {
        let surface = pixels.lock();
        (i64::from(surface.width()), i64::from(surface.height()))
    };
    // A region that is not inside the source is refused by `from_region`
    // with the exception MIDP names; one that is can be no larger than the
    // source.
    let inside = x >= 0
        && y >= 0
        && i64::from(x) + i64::from(width) <= source_width
        && i64::from(y) + i64::from(height) <= source_height;
    if inside {
        reserve_pixels(ctx, width.into(), height.into())?;
    }
    let result = Surface::from_region(
        &pixels.lock(),
        a[1].int(),
        a[2].int(),
        a[3].int(),
        a[4].int(),
        a[5].int(),
    );
    match result {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_rgb(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let rgb = ctx.non_null(a[0])?;
    // A size the array cannot fill is refused by `from_rgb` with the
    // exception MIDP names.
    let (width, height) = (i64::from(a[1].int()), i64::from(a[2].int()));
    if width * height <= ctx.array::<i32>(rgb)?.len() as i64 {
        reserve_pixels(ctx, width, height)?;
    }
    let result = Surface::from_rgb(
        ctx.array::<i32>(rgb)?,
        a[1].int(),
        a[2].int(),
        a[3].boolean(),
    );
    match result {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn create_mutable(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    reserve_pixels(ctx, a[0].int().into(), a[1].int().into())?;
    match Surface::new_mutable(a[0].int(), a[1].int()) {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

fn get_rgb(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let out = ctx.non_null(a[1])?;
    let pixels = surface_of(ctx, this)?;
    let surface = pixels.lock();
    let result = surface.get_rgb(
        ctx.array_mut::<i32>(out)?,
        a[2].int(),
        a[3].int(),
        a[4].int(),
        a[5].int(),
        a[6].int(),
        a[7].int(),
    );
    drop(surface);
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

fn get_graphics(ctx: &mut Ctx<'_>, this: Value) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let pixels = surface_of(ctx, this)?;
    if pixels.lock().kind() == SurfaceKind::Immutable {
        return ctx.throw(
            "java/lang/IllegalStateException",
            "an immutable image has no Graphics",
        );
    }
    Ok(new_graphics(ctx, pixels)?.into())
}

fn dimension(f: fn(&Surface) -> u32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        let this = ctx.non_null(a[0])?;
        let pixels = surface_of(ctx, this)?;
        Ok(Value::Int(f(&pixels.lock()) as i32))
    }
}

/// The class.
pub fn class() -> NativeClass {
    const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";
    NativeClass::new(CLASS)
        .static_method("createImage", &format!("(II){IMAGE}"), create_mutable)
        .static_method("createImage", &format!("({IMAGE}){IMAGE}"), |ctx, a| {
            create_copy(ctx, a[0])
        })
        .static_method(
            "createImage",
            &format!("(Ljava/lang/String;){IMAGE}"),
            |ctx, a| create_from_resource(ctx, a[0]),
        )
        .static_method("createImage", &format!("([BII){IMAGE}"), create_from_bytes)
        .static_method(
            "createImage",
            &format!("({IMAGE}IIIII){IMAGE}"),
            create_region,
        )
        .static_method(
            "createImage",
            &format!("(Ljava/io/InputStream;){IMAGE}"),
            |ctx, a| create_from_stream(ctx, a[0]),
        )
        .static_method("createRGBImage", &format!("([IIIZ){IMAGE}"), create_rgb)
        .method(
            "getGraphics",
            "()Ljavax/microedition/lcdui/Graphics;",
            |ctx, a| get_graphics(ctx, a[0]),
        )
        .method("getWidth", "()I", dimension(Surface::width))
        .method("getHeight", "()I", dimension(Surface::height))
        .method("isMutable", "()Z", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let pixels = surface_of(ctx, this)?;
            Ok(Value::from(pixels.lock().kind() != SurfaceKind::Immutable))
        })
        .method("getRGB", "([IIIIIII)V", get_rgb)
}
