//! `javax.microedition.lcdui.game.Sprite` over the sprite model.

use fastemu_gfx::Transform;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};

use super::layer::{
    LayerData, Shape, image_arg, image_pixels, image_size, layer_arg, layer_pixels, not_a, paint,
    throw, with_both,
};
use super::model::GameError;
use super::model::collide::{self, Footprint};
use super::model::sprite::Sprite;
use crate::lcdui::image::new_image;
use crate::lcdui::throw_gfx;

const CLASS: &str = "javax/microedition/lcdui/game/Sprite";
const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";

/// Runs `f` on the sprite `this` holds.
fn edit<R>(ctx: &mut Ctx<'_>, this: Value, f: impl FnOnce(&mut Sprite) -> R) -> JResult<R> {
    let this = ctx.non_null(this)?;
    let result = match &mut ctx.payload_mut::<LayerData>(this)?.shape {
        Shape::Sprite(sprite) => Some(f(sprite)),
        Shape::Tiled(_) => None,
    };
    result.ok_or_else(|| not_a(ctx, "Sprite"))
}

/// Like [`edit`] for a call that can break a rule of the spec.
fn checked(
    ctx: &mut Ctx<'_>,
    this: Value,
    f: impl FnOnce(&mut Sprite) -> Result<(), GameError>,
) -> JResult<Value> {
    match edit(ctx, this, f)? {
        Ok(()) => Ok(Value::Void),
        Err(e) => Err(throw(ctx, e)),
    }
}

/// A native that reads an `int` from the sprite.
fn query(f: fn(&Sprite) -> i32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| Ok(Value::Int(edit(ctx, a[0], |s| f(s))?))
}

/// A native that changes the sprite and returns nothing.
fn command(f: fn(&mut Sprite)) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| {
        edit(ctx, a[0], f)?;
        Ok(Value::Void)
    }
}

/// Cuts `image` into frames for `this`: `Sprite(image)` when `frame` is
/// `None`, `Sprite(image, frameWidth, frameHeight)` otherwise.
fn construct(
    ctx: &mut Ctx<'_>,
    this: Value,
    image: Value,
    frame: Option<(i32, i32)>,
) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let image = image_arg(ctx, image)?;
    let (width, height) = image_size(&image_pixels(ctx, image)?);
    let (fw, fh) = frame.unwrap_or((width as i32, height as i32));
    let sprite = Sprite::new(width, height, fw, fh).map_err(|e| throw(ctx, e))?;
    let data = ctx.payload_mut::<LayerData>(this)?;
    data.image = Some(image);
    data.shape = Shape::Sprite(sprite);
    Ok(Value::Void)
}

/// `Sprite(Sprite s)`: a copy of every property. A mutable source image
/// is copied as it is now, as `Image.createImage(Image)` would.
fn copy(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let other = layer_arg(ctx, a[1])?;
    let (sprite, image) = {
        let data = ctx.payload::<LayerData>(other)?;
        match &data.shape {
            Shape::Sprite(sprite) => (Some(sprite.clone()), data.image),
            Shape::Tiled(_) => (None, None),
        }
    };
    let Some(sprite) = sprite else {
        return Err(not_a(ctx, "Sprite"));
    };
    let image = match image {
        Some(image) => {
            let image = ctx.root(image);
            let pixels = image_pixels(ctx, image)?;
            let snapshot = {
                let surface = pixels.lock();
                surface.is_mutable().then(|| surface.to_immutable())
            };
            Some(match snapshot {
                Some(surface) => new_image(ctx, surface)?,
                None => image,
            })
        }
        None => None,
    };
    let data = ctx.payload_mut::<LayerData>(this)?;
    data.image = image;
    data.shape = Shape::Sprite(sprite);
    Ok(Value::Void)
}

fn set_frame_sequence(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let sequence = match a[1].obj() {
        Some(array) => Some(ctx.array::<i32>(array)?.to_vec()),
        None => None,
    };
    checked(ctx, a[0], |s| s.set_frame_sequence(sequence.as_deref()))
}

fn set_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let image = image_arg(ctx, a[1])?;
    let (width, height) = image_size(&image_pixels(ctx, image)?);
    let (fw, fh) = (a[2].int(), a[3].int());
    checked(ctx, a[0], |s| s.set_image(width, height, fw, fh))?;
    ctx.payload_mut::<LayerData>(this)?.image = Some(image);
    Ok(Value::Void)
}

fn set_transform(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let transform = Transform::from_midp(a[1].int()).map_err(|e| throw_gfx(ctx, &e))?;
    edit(ctx, a[0], |s| s.set_transform(transform))?;
    Ok(Value::Void)
}

/// The footprint of the sprite `this` holds.
fn footprint(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<Footprint> {
    edit(ctx, this.into(), |s| s.footprint())
}

fn collides_with_sprite(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let other = layer_arg(ctx, a[1])?;
    let (mine, theirs) = (footprint(ctx, this)?, footprint(ctx, other)?);
    if !a[2].boolean() {
        return Ok(collide::sprites(&mine, &theirs, None).into());
    }
    let images = (
        layer_pixels(ctx, ctx.payload::<LayerData>(this)?),
        layer_pixels(ctx, ctx.payload::<LayerData>(other)?),
    );
    let hit = match images {
        (Some(a), Some(b)) => with_both(&a, &b, |a, b| {
            collide::sprites(&mine, &theirs, Some((a, b)))
        }),
        _ => false,
    };
    Ok(hit.into())
}

fn collides_with_tiles(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let tiles = layer_arg(ctx, a[1])?;
    let mine = footprint(ctx, this)?;
    let pixel_level = a[2].boolean();
    let mine_pixels = layer_pixels(ctx, ctx.payload::<LayerData>(this)?);
    let data = ctx.payload::<LayerData>(tiles)?;
    let Shape::Tiled(layer) = &data.shape else {
        return Err(not_a(ctx, "TiledLayer"));
    };
    if !pixel_level {
        return Ok(collide::sprite_and_tiles(&mine, layer, None).into());
    }
    let hit = match (mine_pixels, layer_pixels(ctx, data)) {
        (Some(a), Some(b)) => with_both(&a, &b, |a, b| {
            collide::sprite_and_tiles(&mine, layer, Some((a, b)))
        }),
        _ => false,
    };
    Ok(hit.into())
}

fn collides_with_image(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let image = image_arg(ctx, a[1])?;
    let (x, y, pixel_level) = (a[2].int(), a[3].int(), a[4].boolean());
    let mine = footprint(ctx, this)?;
    let other = image_pixels(ctx, image)?;
    if !pixel_level {
        return Ok(collide::sprite_and_image(&mine, &other.lock(), x, y, None).into());
    }
    let hit = match layer_pixels(ctx, ctx.payload::<LayerData>(this)?) {
        Some(mine_pixels) => with_both(&mine_pixels, &other, |a, b| {
            collide::sprite_and_image(&mine, b, x, y, Some(a))
        }),
        None => false,
    };
    Ok(hit.into())
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    let final_method = access::PUBLIC | access::FINAL;
    let collides = |other: &str| format!("({other}Z)Z");
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/game/Layer")
        .payload_with(|| Box::new(LayerData::sprite()))
        .constant("TRANS_NONE", "I", int(0))
        .constant("TRANS_ROT90", "I", int(5))
        .constant("TRANS_ROT180", "I", int(3))
        .constant("TRANS_ROT270", "I", int(6))
        .constant("TRANS_MIRROR", "I", int(2))
        .constant("TRANS_MIRROR_ROT90", "I", int(7))
        .constant("TRANS_MIRROR_ROT180", "I", int(1))
        .constant("TRANS_MIRROR_ROT270", "I", int(4))
        .method("<init>", &format!("({IMAGE})V"), |ctx, a| {
            construct(ctx, a[0], a[1], None)
        })
        .method("<init>", &format!("({IMAGE}II)V"), |ctx, a| {
            construct(ctx, a[0], a[1], Some((a[2].int(), a[3].int())))
        })
        .method("<init>", &format!("(L{CLASS};)V"), copy)
        .method("defineReferencePixel", "(II)V", |ctx, a| {
            let (x, y) = (a[1].int(), a[2].int());
            edit(ctx, a[0], |s| s.define_reference_pixel(x, y))?;
            Ok(Value::Void)
        })
        .method("setRefPixelPosition", "(II)V", |ctx, a| {
            let (x, y) = (a[1].int(), a[2].int());
            edit(ctx, a[0], |s| s.set_ref_pixel_position(x, y))?;
            Ok(Value::Void)
        })
        .method("getRefPixelX", "()I", query(|s| s.ref_pixel().0))
        .method("getRefPixelY", "()I", query(|s| s.ref_pixel().1))
        .method("setFrame", "(I)V", |ctx, a| {
            let index = a[1].int();
            checked(ctx, a[0], |s| s.set_frame(index))
        })
        .method_with("getFrame", "()I", final_method, query(Sprite::frame))
        .method("getRawFrameCount", "()I", query(Sprite::raw_frame_count))
        .method("getFrameSequenceLength", "()I", query(Sprite::sequence_len))
        .method("nextFrame", "()V", command(Sprite::next_frame))
        .method("prevFrame", "()V", command(Sprite::prev_frame))
        .method_with(
            "paint",
            "(Ljavax/microedition/lcdui/Graphics;)V",
            final_method,
            paint,
        )
        .method("setFrameSequence", "([I)V", set_frame_sequence)
        .method("setImage", &format!("({IMAGE}II)V"), set_image)
        .method("defineCollisionRectangle", "(IIII)V", |ctx, a| {
            let [x, y, w, h] = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            checked(ctx, a[0], |s| s.define_collision_rectangle(x, y, w, h))
        })
        .method("setTransform", "(I)V", set_transform)
        .method_with(
            "collidesWith",
            &collides(&format!("L{CLASS};")),
            final_method,
            collides_with_sprite,
        )
        .method_with(
            "collidesWith",
            &collides("Ljavax/microedition/lcdui/game/TiledLayer;"),
            final_method,
            collides_with_tiles,
        )
        .method_with(
            "collidesWith",
            &format!("({IMAGE}IIZ)Z"),
            final_method,
            collides_with_image,
        )
}
