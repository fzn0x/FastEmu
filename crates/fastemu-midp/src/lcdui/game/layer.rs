//! `javax.microedition.lcdui.game.Layer`, and the payload every sprite
//! and tiled layer keeps: its model and its image.

use std::sync::Arc;

use fastemu_gfx::{GfxError, Surface};
use fastemu_jvm::{
    Ctx, JResult, NativeClass, ObjRef, Payload, Throw, Tracer, Value, classfile::access,
};

use super::model::sprite::Sprite;
use super::model::tiled::TiledLayer;
use super::model::{Base, GameError};
use crate::lcdui::graphics::GraphicsData;
use crate::lcdui::image::{ImageData, SharedSurface};
use crate::lcdui::throw_gfx;

const CLASS: &str = "javax/microedition/lcdui/game/Layer";

/// What a layer is.
pub(crate) enum Shape {
    Sprite(Sprite),
    Tiled(TiledLayer),
}

/// The payload of a `Sprite` or `TiledLayer` object.
pub(crate) struct LayerData {
    /// The image the frames or tiles are cut from; `None` until the
    /// constructor has run.
    pub image: Option<ObjRef>,
    pub shape: Shape,
}

impl Payload for LayerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.image);
    }

    fn heap_bytes(&self) -> usize {
        match &self.shape {
            Shape::Sprite(sprite) => sprite.heap_bytes(),
            Shape::Tiled(tiled) => tiled.heap_bytes(),
        }
    }
}

impl LayerData {
    /// The payload of a new `Sprite`, before its constructor runs.
    pub fn sprite() -> Self {
        Self {
            image: None,
            shape: Shape::Sprite(Sprite::default()),
        }
    }

    /// The payload of a new `TiledLayer`, before its constructor runs.
    pub fn tiled() -> Self {
        Self {
            image: None,
            shape: Shape::Tiled(TiledLayer::default()),
        }
    }

    /// The position and visibility.
    pub fn base(&self) -> &Base {
        match &self.shape {
            Shape::Sprite(s) => &s.base,
            Shape::Tiled(t) => &t.base,
        }
    }

    /// The position and visibility, to change them.
    pub fn base_mut(&mut self) -> &mut Base {
        match &mut self.shape {
            Shape::Sprite(s) => &mut s.base,
            Shape::Tiled(t) => &mut t.base,
        }
    }

    /// `getWidth()` and `getHeight()`.
    pub fn size(&self) -> (i32, i32) {
        match &self.shape {
            Shape::Sprite(s) => (s.width(), s.height()),
            Shape::Tiled(t) => (t.width(), t.height()),
        }
    }
}

/// The exception a rule of the game API names.
pub(crate) fn throw(ctx: &mut Ctx<'_>, e: GameError) -> Throw {
    ctx.new_exception(e.java_class(), e.message())
}

/// The `ClassCastException` for an argument of the wrong class, which
/// only code that skipped the verifier can pass.
pub(crate) fn not_a(ctx: &mut Ctx<'_>, what: &str) -> Throw {
    ctx.new_exception(
        "java/lang/ClassCastException",
        Some(&format!("not a {what}")),
    )
}

/// Checks that an argument is a non-null object with a payload of type
/// `T`, a `ClassCastException` naming `what` otherwise, so the payload can
/// then be read without a check.
fn argument<T: Payload>(ctx: &mut Ctx<'_>, v: Value, what: &str) -> JResult<ObjRef> {
    let obj = ctx.non_null(v)?;
    if ctx.try_payload::<T>(obj).is_none() {
        return Err(not_a(ctx, what));
    }
    Ok(obj)
}

/// A `Layer` argument.
pub(crate) fn layer_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    argument::<LayerData>(ctx, v, "Layer")
}

/// A `Graphics` argument.
pub(crate) fn graphics_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    argument::<GraphicsData>(ctx, v, "Graphics")
}

/// An `Image` argument.
pub(crate) fn image_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    argument::<ImageData>(ctx, v, "Image")
}

/// The pixels of an `Image` checked by [`image_arg`].
pub(crate) fn image_pixels(ctx: &Ctx<'_>, image: ObjRef) -> JResult<SharedSurface> {
    Ok(Arc::clone(&ctx.payload::<ImageData>(image)?.surface))
}

/// The width and height of an image's pixels.
pub(crate) fn image_size(pixels: &SharedSurface) -> (u32, u32) {
    let surface = pixels.lock();
    (surface.width(), surface.height())
}

/// The pixels of a layer's image, if it has been constructed.
pub(crate) fn layer_pixels(ctx: &Ctx<'_>, layer: &LayerData) -> Option<SharedSurface> {
    let image = layer.image?;
    let data = ctx.try_payload::<ImageData>(image)?;
    Some(Arc::clone(&data.surface))
}

/// Runs `f` on two surfaces that may be one, locking each once.
pub(crate) fn with_both<R>(
    a: &SharedSurface,
    b: &SharedSurface,
    f: impl FnOnce(&Surface, &Surface) -> R,
) -> R {
    if Arc::ptr_eq(a, b) {
        let one = a.lock();
        f(&one, &one)
    } else {
        f(&a.lock(), &b.lock())
    }
}

/// Draws `layer` with the state and onto the target of `g`. A layer
/// whose image is the target itself is `drawRegion`'s
/// `IllegalArgumentException`.
pub(crate) fn draw(ctx: &Ctx<'_>, layer: ObjRef, g: &GraphicsData) -> Result<(), GfxError> {
    let Some(data) = ctx.try_payload::<LayerData>(layer) else {
        return Ok(());
    };
    if !data.base().visible {
        return Ok(());
    }
    let Some(source) = layer_pixels(ctx, data) else {
        return Ok(());
    };
    if Arc::ptr_eq(&source, &g.target) {
        return Err(GfxError::IllegalArgument(
            "drawRegion of the destination image itself",
        ));
    }
    let (source, mut target) = (source.lock(), g.target.lock());
    match &data.shape {
        Shape::Sprite(s) => s.paint(&g.state, &mut target, &source),
        Shape::Tiled(t) => t.paint(&g.state, &mut target, &source),
    }
}

/// `paint(g)` of a sprite or tiled layer.
pub(crate) fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let g = graphics_arg(ctx, a[1])?;
    let result = draw(ctx, this, ctx.payload::<GraphicsData>(g)?);
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// Reads the payload of `this`.
fn read<R>(f: fn(&LayerData) -> R) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value>
where
    Value: From<R>,
{
    move |ctx, a| {
        let this = ctx.non_null(a[0])?;
        Ok(Value::from(f(ctx.payload::<LayerData>(this)?)))
    }
}

/// The class.
pub fn class() -> NativeClass {
    let final_method = access::PUBLIC | access::FINAL;
    NativeClass::new(CLASS)
        .abstract_class()
        .method("setPosition", "(II)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let base = ctx.payload_mut::<LayerData>(this)?.base_mut();
            (base.x, base.y) = (a[1].int(), a[2].int());
            Ok(Value::Void)
        })
        .method("move", "(II)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.payload_mut::<LayerData>(this)?
                .base_mut()
                .move_by(a[1].int(), a[2].int());
            Ok(Value::Void)
        })
        .method_with("getX", "()I", final_method, read(|d| d.base().x))
        .method_with("getY", "()I", final_method, read(|d| d.base().y))
        .method_with("getWidth", "()I", final_method, read(|d| d.size().0))
        .method_with("getHeight", "()I", final_method, read(|d| d.size().1))
        .method("setVisible", "(Z)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.payload_mut::<LayerData>(this)?.base_mut().visible = a[1].boolean();
            Ok(Value::Void)
        })
        .method_with("isVisible", "()Z", final_method, read(|d| d.base().visible))
        .abstract_method("paint", "(Ljavax/microedition/lcdui/Graphics;)V")
}
