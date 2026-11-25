//! `com.siemens.mp.color_game.Sprite`: MIDP 2.0's `Sprite` without the
//! reference pixel and the transforms. Frames are cut from the image left
//! to right, then top to bottom; a frame sequence picks among them, and
//! collisions test the collision rectangles and, when asked, the opaque
//! pixels.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};

use super::layer::{GRAPHICS, IMAGE, Kind, LAYER, LayerData, draw_regions, image_size, layer};
use crate::lcdui::image::surface_of;

const SPRITE: &str = "com/siemens/mp/color_game/Sprite";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const INDEX_OUT_OF_BOUNDS: &str = "java/lang/IndexOutOfBoundsException";

/// A sprite's frames and collision rectangle.
#[derive(Clone)]
pub(in crate::vendor::siemens) struct SpriteState {
    frame_width: i32,
    frame_height: i32,
    /// Frames across the image.
    columns: i32,
    raw_frames: i32,
    /// The frame sequence the game set, or `None` for every raw frame in
    /// order.
    sequence: Option<Vec<i32>>,
    index: i32,
    /// x, y, width and height, relative to the sprite.
    collision: [i32; 4],
}

impl SpriteState {
    pub(super) fn heap_bytes(&self) -> usize {
        self.sequence.as_ref().map_or(0, |s| s.len() * 4)
    }

    fn sequence_len(&self) -> i32 {
        self.sequence
            .as_ref()
            .map_or(self.raw_frames, |s| s.len() as i32)
    }

    /// The raw frame shown now.
    fn raw_frame(&self) -> i32 {
        match &self.sequence {
            Some(sequence) => sequence[self.index as usize],
            None => self.index,
        }
    }

    /// Where the frame shown now starts in the image.
    fn origin(&self) -> (i32, i32) {
        let frame = self.raw_frame();
        (
            frame % self.columns * self.frame_width,
            frame / self.columns * self.frame_height,
        )
    }
}

fn illegal<T>(ctx: &mut Ctx<'_>, why: &str) -> JResult<T> {
    ctx.throw(ILLEGAL_ARGUMENT, why)
}

/// Checks a frame size against an image and returns the sprite state for
/// it.
fn frames_of(ctx: &mut Ctx<'_>, image: ObjRef, fw: i32, fh: i32) -> JResult<SpriteState> {
    let (iw, ih) = image_size(ctx, image)?;
    if fw < 1 || fh < 1 {
        return illegal(ctx, "frame size below 1");
    }
    if iw % fw != 0 || ih % fh != 0 {
        return illegal(ctx, "the image is not a whole number of frames");
    }
    Ok(SpriteState {
        frame_width: fw,
        frame_height: fh,
        columns: iw / fw,
        raw_frames: (iw / fw) * (ih / fh),
        sequence: None,
        index: 0,
        collision: [0, 0, fw, fh],
    })
}

/// Makes `this` a sprite of `image` cut into `fw` by `fh` frames.
fn init(ctx: &mut Ctx<'_>, this: Value, image: Value, fw: Option<(i32, i32)>) -> JResult<Value> {
    let image = ctx.non_null(image)?;
    let (fw, fh) = match fw {
        Some(size) => size,
        None => image_size(ctx, image)?,
    };
    let state = frames_of(ctx, image, fw, fh)?;
    let l = layer(ctx, this)?;
    (l.width, l.height, l.image) = (fw, fh, Some(image));
    l.kind = Kind::Sprite(state);
    Ok(Value::Void)
}

fn sprite<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut SpriteState> {
    if !matches!(layer(ctx, this)?.kind, Kind::Sprite(_)) {
        return ctx.throw("java/lang/IllegalStateException", "not a sprite");
    }
    match &mut layer(ctx, this)?.kind {
        Kind::Sprite(s) => Ok(s),
        _ => unreachable!("the kind was checked above"),
    }
}

/// The opaque pixels of a sprite's current frame inside its collision
/// rectangle, as a rectangle on the screen and a mask, or `None` when it
/// has none.
struct Shape {
    /// Left, top, right and bottom on the screen.
    bounds: [i64; 4],
    /// Whether each pixel of `bounds` is opaque, row by row; `None` when
    /// only the rectangle counts.
    opaque: Option<Vec<bool>>,
}

impl Shape {
    fn opaque_at(&self, x: i64, y: i64) -> bool {
        let [left, top, right, _] = self.bounds;
        match &self.opaque {
            None => true,
            Some(mask) => mask[((y - top) * (right - left) + (x - left)) as usize],
        }
    }
}

/// The opaque pixels of `image` in `area` (left, top, right, bottom in
/// image coordinates), each translated by `(dx, dy)` to the screen.
fn image_shape(
    ctx: &Ctx<'_>,
    image: ObjRef,
    area: [i64; 4],
    (dx, dy): (i64, i64),
    pixels: bool,
) -> JResult<Shape> {
    let [left, top, right, bottom] = area;
    let bounds = [left + dx, top + dy, right + dx, bottom + dy];
    if !pixels || right <= left || bottom <= top {
        return Ok(Shape {
            bounds,
            opaque: None,
        });
    }
    let surface = surface_of(ctx, image)?;
    let surface = surface.lock();
    let mut opaque = Vec::with_capacity(((right - left) * (bottom - top)) as usize);
    for y in top..bottom {
        for x in left..right {
            let p = surface.pixel(x as u32, y as u32).unwrap_or(0);
            opaque.push(p >> 24 != 0);
        }
    }
    Ok(Shape {
        bounds,
        opaque: Some(opaque),
    })
}

/// The collision shape of the visible sprite `this`, `None` when hidden.
fn sprite_shape(ctx: &mut Ctx<'_>, this: Value, pixels: bool) -> JResult<Option<Shape>> {
    let l = layer(ctx, this)?;
    let (x, y, visible, image) = (l.x, l.y, l.visible, l.image);
    let Kind::Sprite(s) = &l.kind else {
        return ctx.throw("java/lang/IllegalStateException", "not a sprite");
    };
    if !visible {
        return Ok(None);
    }
    let [cx, cy, cw, ch] = s.collision.map(i64::from);
    let (fw, fh) = (i64::from(s.frame_width), i64::from(s.frame_height));
    let (ox, oy) = s.origin();
    let (ox, oy) = (i64::from(ox), i64::from(oy));
    // Pixel tests stay inside the frame.
    let area = if pixels {
        [cx.max(0), cy.max(0), (cx + cw).min(fw), (cy + ch).min(fh)]
    } else {
        [cx, cy, cx + cw, cy + ch]
    };
    let image_area = [area[0] + ox, area[1] + oy, area[2] + ox, area[3] + oy];
    let Some(image) = image else {
        return Ok(None);
    };
    let shift = (i64::from(x) - ox, i64::from(y) - oy);
    Ok(Some(image_shape(ctx, image, image_area, shift, pixels)?))
}

/// Whether two shapes share a pixel (or overlap, without pixel masks).
fn overlap(a: &Shape, b: &Shape) -> bool {
    let left = a.bounds[0].max(b.bounds[0]);
    let top = a.bounds[1].max(b.bounds[1]);
    let right = a.bounds[2].min(b.bounds[2]);
    let bottom = a.bounds[3].min(b.bounds[3]);
    if left >= right || top >= bottom {
        return false;
    }
    if a.opaque.is_none() && b.opaque.is_none() {
        return true;
    }
    (top..bottom).any(|y| (left..right).any(|x| a.opaque_at(x, y) && b.opaque_at(x, y)))
}

fn collides_with_sprite(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let other = ctx.non_null(a[1])?;
    let pixels = a[2].boolean();
    let Some(ours) = sprite_shape(ctx, a[0], pixels)? else {
        return Ok(Value::from(false));
    };
    let Some(theirs) = sprite_shape(ctx, other.into(), pixels)? else {
        return Ok(Value::from(false));
    };
    Ok(Value::from(overlap(&ours, &theirs)))
}

fn collides_with_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let image = ctx.non_null(a[1])?;
    let (x, y, pixels) = (i64::from(a[2].int()), i64::from(a[3].int()), a[4].boolean());
    let Some(ours) = sprite_shape(ctx, a[0], pixels)? else {
        return Ok(Value::from(false));
    };
    let (w, h) = image_size(ctx, image)?;
    let theirs = image_shape(ctx, image, [0, 0, w.into(), h.into()], (x, y), pixels)?;
    Ok(Value::from(overlap(&ours, &theirs)))
}

fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let g = ctx.non_null(a[1])?;
    let l = layer(ctx, a[0])?;
    let (x, y, visible, image) = (l.x, l.y, l.visible, l.image);
    let Kind::Sprite(s) = &l.kind else {
        return Ok(Value::Void);
    };
    let ((sx, sy), size) = (s.origin(), (s.frame_width, s.frame_height));
    match image {
        Some(image) if visible => draw_regions(ctx, g, image, size, &[(sx, sy, x, y)])?,
        _ => {}
    }
    Ok(Value::Void)
}

fn set_frame(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let index = a[1].int();
    let s = sprite(ctx, a[0])?;
    if !(0..s.sequence_len()).contains(&index) {
        return ctx.throw(INDEX_OUT_OF_BOUNDS, &format!("frame {index}"));
    }
    s.index = index;
    Ok(Value::Void)
}

fn set_frame_sequence(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    // The sprite keeps a copy of the array, which the heap counts a second
    // time, so a game cannot give many sprites the same big array.
    let sequence = match a[1].obj() {
        Some(array) => {
            let length = ctx.array::<i32>(array)?.len();
            ctx.check_alloc(length.saturating_mul(4))?;
            Some(ctx.array::<i32>(array)?.to_vec())
        }
        None => None,
    };
    let raw = sprite(ctx, a[0])?.raw_frames;
    if let Some(sequence) = &sequence {
        if sequence.is_empty() {
            return illegal(ctx, "an empty frame sequence");
        }
        if sequence.iter().any(|f| !(0..raw).contains(f)) {
            return ctx.throw("java/lang/ArrayIndexOutOfBoundsException", "frame");
        }
    }
    let s = sprite(ctx, a[0])?;
    (s.sequence, s.index) = (sequence, 0);
    ctx.recount(this);
    Ok(Value::Void)
}

/// `setImage(image, frameWidth, frameHeight)`: keeps the frame sequence
/// and the current frame when the new image has at least as many frames,
/// and the collision rectangle when the frame size stays, as MIDP does.
fn set_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let image = ctx.non_null(a[1])?;
    let mut fresh = frames_of(ctx, image, a[2].int(), a[3].int())?;
    let old = sprite(ctx, a[0])?.clone();
    if fresh.raw_frames >= old.raw_frames {
        (fresh.sequence, fresh.index) = (old.sequence, old.index);
    }
    if (fresh.frame_width, fresh.frame_height) == (old.frame_width, old.frame_height) {
        fresh.collision = old.collision;
    }
    let l = layer(ctx, a[0])?;
    (l.width, l.height, l.image) = (fresh.frame_width, fresh.frame_height, Some(image));
    l.kind = Kind::Sprite(fresh);
    Ok(Value::Void)
}

fn copy(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let other = ctx.non_null(a[1])?;
    // The copy has a frame sequence of its own.
    let sequence_bytes = match &ctx.payload::<LayerData>(other)?.kind {
        Kind::Sprite(state) => state.heap_bytes(),
        _ => 0,
    };
    ctx.check_alloc(sequence_bytes)?;
    let from = ctx.payload::<LayerData>(other)?;
    let Kind::Sprite(state) = &from.kind else {
        return ctx.throw("java/lang/IllegalStateException", "not a sprite");
    };
    let copy = LayerData {
        x: from.x,
        y: from.y,
        width: from.width,
        height: from.height,
        visible: from.visible,
        image: from.image,
        kind: Kind::Sprite(state.clone()),
    };
    *layer(ctx, a[0])? = copy;
    ctx.recount(this);
    Ok(Value::Void)
}

/// The class.
pub(super) fn class() -> NativeClass {
    let final_method = access::PUBLIC | access::FINAL;
    NativeClass::new(SPRITE)
        .extends(LAYER)
        .method("<init>", &format!("({IMAGE})V"), |ctx, a| {
            init(ctx, a[0], a[1], None)
        })
        .method("<init>", &format!("({IMAGE}II)V"), |ctx, a| {
            init(ctx, a[0], a[1], Some((a[2].int(), a[3].int())))
        })
        .method("<init>", &format!("(L{SPRITE};)V"), copy)
        .method(
            "collidesWith",
            &format!("(L{SPRITE};Z)Z"),
            collides_with_sprite,
        )
        .method(
            "collidesWith",
            &format!("({IMAGE}IIZ)Z"),
            collides_with_image,
        )
        .method_with("getFrame", "()I", final_method, |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.index))
        })
        .method("getFrameSequenceLength", "()I", |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.sequence_len()))
        })
        .method("getRawFrameCount", "()I", |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.raw_frames))
        })
        .method("nextFrame", "()V", |ctx, a| {
            let s = sprite(ctx, a[0])?;
            s.index = (s.index + 1) % s.sequence_len();
            Ok(Value::Void)
        })
        .method("prevFrame", "()V", |ctx, a| {
            let s = sprite(ctx, a[0])?;
            let len = s.sequence_len();
            s.index = (s.index + len - 1) % len;
            Ok(Value::Void)
        })
        .method_with("paint", &format!("({GRAPHICS})V"), final_method, paint)
        .method("setCollisionRectangle", "(IIII)V", |ctx, a| {
            let rect = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            if rect[2] < 0 || rect[3] < 0 {
                return illegal(ctx, "negative collision rectangle");
            }
            sprite(ctx, a[0])?.collision = rect;
            Ok(Value::Void)
        })
        .method("setFrame", "(I)V", set_frame)
        .method("setFrameSequence", "([I)V", set_frame_sequence)
        .method("setImage", &format!("({IMAGE}II)V"), set_image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(bounds: [i64; 4], opaque: Option<Vec<bool>>) -> Shape {
        Shape { bounds, opaque }
    }

    #[test]
    fn rectangles_overlap_only_when_they_share_area() {
        let a = rect([0, 0, 4, 4], None);
        assert!(overlap(&a, &rect([3, 3, 5, 5], None)));
        assert!(!overlap(&a, &rect([4, 0, 6, 4], None)));
        assert!(!overlap(&a, &rect([0, 0, 0, 4], None)));
    }

    #[test]
    fn pixel_tests_need_an_opaque_pixel_in_both() {
        // A 2 by 2 shape with only its bottom right pixel opaque.
        let corner = rect([0, 0, 2, 2], Some(vec![false, false, false, true]));
        assert!(!overlap(&corner, &rect([0, 0, 1, 2], None)));
        assert!(overlap(&corner, &rect([1, 1, 3, 3], None)));
        let other = rect([1, 1, 3, 3], Some(vec![false, true, true, true]));
        assert!(!overlap(&corner, &other));
    }
}
