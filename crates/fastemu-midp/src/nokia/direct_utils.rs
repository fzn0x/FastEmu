//! `com.nokia.mid.ui.DirectUtils`: mutable images from bytes or filled
//! with an ARGB colour, and the `DirectGraphics` of a `Graphics`.

use fastemu_gfx::{Surface, SurfaceKind};
use fastemu_jvm::{Ctx, JResult, NativeClass, Value};

use crate::lcdui::image::new_image;
use crate::lcdui::throw_gfx;

const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";

/// `createImage(byte[] imageData, int imageOffset, int imageLength)`:
/// decodes like `Image.createImage(byte[], int, int)` (with its
/// exceptions) but returns a mutable image, alpha kept.
fn create_from_bytes(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let data = ctx.non_null(a[0])?;
    let (offset, length) = (a[1].int(), a[2].int());
    let bytes: Vec<u8> = ctx.array::<i8>(data)?.iter().map(|&b| b as u8).collect();
    let decoded = ctx.without_lock(move || Surface::decode_range(&bytes, offset, length));
    match decoded {
        Ok(surface) => Ok(new_image(ctx, surface.into_kind(SurfaceKind::Mutable))?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

/// `createImage(int width, int height, int ARGBcolor)`: a mutable image
/// filled with the colour, which may be translucent or fully transparent;
/// `IllegalArgumentException` for a size of zero or less.
fn create_filled(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    match Surface::new_mutable_filled(a[0].int(), a[1].int(), a[2].int()) {
        Ok(surface) => Ok(new_image(ctx, surface)?.into()),
        Err(e) => Err(throw_gfx(ctx, &e)),
    }
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new("com/nokia/mid/ui/DirectUtils")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method(
            "getDirectGraphics",
            "(Ljavax/microedition/lcdui/Graphics;)Lcom/nokia/mid/ui/DirectGraphics;",
            // MIDP's Graphics is the DirectGraphics: the same context.
            |_, a| Ok(a[0]),
        )
        .static_method("createImage", &format!("([BII){IMAGE}"), create_from_bytes)
        .static_method("createImage", &format!("(III){IMAGE}"), create_filled)
}
