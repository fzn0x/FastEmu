//! `com.siemens.mp.color_game.Layer`, the state every layer keeps, and the
//! drawing the layers share.
//!
//! Unlike MIDP's `Layer`, Siemens' has protected constructors, so games
//! may subclass it; such a layer paints itself through its own `paint`.

use std::sync::Arc;

use fastemu_gfx::GfxError;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value, classfile::access};

use super::sprite::SpriteState;
use super::tiled_layer::TiledState;
use crate::lcdui::graphics::GraphicsData;
use crate::lcdui::image::surface_of;
use crate::lcdui::throw_gfx;

pub(super) const LAYER: &str = "com/siemens/mp/color_game/Layer";
pub(super) const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";
pub(super) const GRAPHICS: &str = "Ljavax/microedition/lcdui/Graphics;";
pub(super) const NATIVE_MEM_FIELD: &str = "NativeMemoryTable";
pub(super) const NATIVE_MEM_DESC: &str = "Lcom/siemens/mp/misc/NativeMem;";

/// What a layer is, beyond its place and size.
#[derive(Default)]
pub(super) enum Kind {
    /// A game's own subclass.
    #[default]
    Plain,
    Sprite(SpriteState),
    Tiled(TiledState),
}

/// What every `Layer` holds.
pub(super) struct LayerData {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
    /// The layer's image: a sprite's frames or a tiled layer's tiles.
    pub image: Option<ObjRef>,
    pub kind: Kind,
}

impl Default for LayerData {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            visible: true,
            image: None,
            kind: Kind::Plain,
        }
    }
}

impl Payload for LayerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.image);
    }

    fn heap_bytes(&self) -> usize {
        match &self.kind {
            Kind::Tiled(t) => t.heap_bytes(),
            Kind::Sprite(s) => s.heap_bytes(),
            Kind::Plain => 0,
        }
    }
}

/// The layer state of `this`, `NullPointerException` for `null`.
pub(super) fn layer<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut LayerData> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<LayerData>(this)
}

/// `IllegalArgumentException` unless `size` is zero or more.
fn check_size(ctx: &mut Ctx<'_>, size: i32) -> JResult<()> {
    if size < 0 {
        return ctx.throw("java/lang/IllegalArgumentException", "negative size");
    }
    Ok(())
}

/// The size of an image.
pub(super) fn image_size(ctx: &Ctx<'_>, image: ObjRef) -> JResult<(i32, i32)> {
    let surface = surface_of(ctx, image)?;
    let surface = surface.lock();
    Ok((surface.width() as i32, surface.height() as i32))
}

/// Draws the `width` by `height` region at `(sx, sy)` of `image` with its
/// top left at `(x, y)` on the `Graphics` `g`, for each of `regions`:
/// `(sx, sy, x, y)`. Respects `g`'s translation and clip.
pub(super) fn draw_regions(
    ctx: &mut Ctx<'_>,
    g: ObjRef,
    image: ObjRef,
    (width, height): (i32, i32),
    regions: &[(i32, i32, i32, i32)],
) -> JResult<()> {
    let source = surface_of(ctx, image)?;
    let d = ctx.payload::<GraphicsData>(g)?;
    let draw = |target: &mut fastemu_gfx::Surface, source: &fastemu_gfx::Surface| {
        regions.iter().try_for_each(|&(sx, sy, x, y)| {
            d.state
                .draw_region(target, source, sx, sy, width, height, 0, x, y, 0)
        })
    };
    let result: Result<(), GfxError> = if Arc::ptr_eq(&source, &d.target) {
        let mut target = d.target.lock();
        let copy = target.clone();
        draw(&mut target, &copy)
    } else {
        let source = source.lock();
        draw(&mut d.target.lock(), &source)
    };
    result.map_err(|e| throw_gfx(ctx, &e))
}

/// The class.
pub(super) fn class() -> NativeClass {
    let protected = access::PROTECTED;
    let public_final = access::PUBLIC | access::FINAL;
    let getter = |f: fn(&LayerData) -> i32| {
        move |ctx: &mut Ctx<'_>, a: &[Value]| Ok(Value::Int(f(layer(ctx, a[0])?)))
    };
    NativeClass::new(LAYER)
        .abstract_class()
        .payload::<LayerData>()
        .field_with(NATIVE_MEM_FIELD, NATIVE_MEM_DESC, protected)
        .method_with("<init>", "(II)V", protected, |ctx, a| {
            let (w, h) = (a[1].int(), a[2].int());
            check_size(ctx, w)?;
            check_size(ctx, h)?;
            let l = layer(ctx, a[0])?;
            (l.width, l.height) = (w, h);
            Ok(Value::Void)
        })
        .method_with("<init>", &format!("({IMAGE})V"), protected, |ctx, a| {
            let image = ctx.non_null(a[1])?;
            let (w, h) = image_size(ctx, image)?;
            let l = layer(ctx, a[0])?;
            (l.width, l.height, l.image) = (w, h, Some(image));
            Ok(Value::Void)
        })
        .method_with(
            "getLayerImage",
            &format!("(){IMAGE}"),
            protected,
            |ctx, a| Ok(layer(ctx, a[0])?.image.into()),
        )
        .method_with(
            "setLayerImage",
            &format!("({IMAGE})V"),
            protected,
            |ctx, a| {
                let image = ctx.non_null(a[1])?;
                layer(ctx, a[0])?.image = Some(image);
                Ok(Value::Void)
            },
        )
        .method_with(
            "copyAllLayerVariables",
            "(Lcom/siemens/mp/color_game/Layer;)V",
            protected,
            |ctx, a| {
                let from = layer(ctx, a[0])?;
                let (x, y, w, h, visible) = (from.x, from.y, from.width, from.height, from.visible);
                let to = layer(ctx, a[1])?;
                (to.x, to.y, to.width, to.height, to.visible) = (x, y, w, h, visible);
                Ok(Value::Void)
            },
        )
        .method_with("getHeight", "()I", public_final, getter(|l| l.height))
        .method_with("getWidth", "()I", public_final, getter(|l| l.width))
        .method_with("getX", "()I", public_final, getter(|l| l.x))
        .method_with("getY", "()I", public_final, getter(|l| l.y))
        .method_with("isVisible", "()Z", public_final, |ctx, a| {
            Ok(Value::from(layer(ctx, a[0])?.visible))
        })
        .method("move", "(II)V", |ctx, a| {
            let (dx, dy) = (a[1].int(), a[2].int());
            let l = layer(ctx, a[0])?;
            (l.x, l.y) = (l.x.wrapping_add(dx), l.y.wrapping_add(dy));
            Ok(Value::Void)
        })
        .method("setPosition", "(II)V", |ctx, a| {
            let (x, y) = (a[1].int(), a[2].int());
            let l = layer(ctx, a[0])?;
            (l.x, l.y) = (x, y);
            Ok(Value::Void)
        })
        .method("setVisible", "(Z)V", |ctx, a| {
            let visible = a[1].boolean();
            layer(ctx, a[0])?.visible = visible;
            Ok(Value::Void)
        })
        .method("setWidth", "(I)V", |ctx, a| {
            let w = a[1].int();
            check_size(ctx, w)?;
            layer(ctx, a[0])?.width = w;
            Ok(Value::Void)
        })
        .method("setHeight", "(I)V", |ctx, a| {
            let h = a[1].int();
            check_size(ctx, h)?;
            layer(ctx, a[0])?.height = h;
            Ok(Value::Void)
        })
        .abstract_method("paint", &format!("({GRAPHICS})V"))
}
