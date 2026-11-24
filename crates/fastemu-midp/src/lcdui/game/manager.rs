//! `javax.microedition.lcdui.game.LayerManager`: layers in drawing order
//! and the view window that shows part of them.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::layer::{draw, graphics_arg, layer_arg};
use crate::lcdui::Rect;
use crate::lcdui::graphics::GraphicsData;
use crate::lcdui::throw_gfx;

const CLASS: &str = "javax/microedition/lcdui/game/LayerManager";
const LAYER: &str = "Ljavax/microedition/lcdui/game/Layer;";

/// What a `LayerManager` holds.
struct ManagerData {
    /// Index 0 is drawn last, on top.
    layers: Vec<ObjRef>,
    /// The part of the layer plane `paint` shows.
    view: Rect,
}

impl Default for ManagerData {
    /// No layers, and a view window from the origin as large as an `int`
    /// allows, as the reference implementation starts.
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            view: Rect::new(0, 0, i32::MAX, i32::MAX),
        }
    }
}

impl Payload for ManagerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &layer in &self.layers {
            tracer.mark(layer);
        }
    }

    fn heap_bytes(&self) -> usize {
        self.layers.len() * 4
    }
}

fn data<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut ManagerData> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<ManagerData>(this)
}

/// `insert(l, index)`: a layer already present is taken out first, so
/// the index may then reach only the other layers' count.
fn insert(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let layer = layer_arg(ctx, a[1])?;
    let index = a[2].int();
    let d = data(ctx, a[0])?;
    let present = d.layers.contains(&layer);
    let limit = d.layers.len() - usize::from(present);
    let Some(index) = usize::try_from(index).ok().filter(|&i| i <= limit) else {
        return ctx.throw(
            "java/lang/IndexOutOfBoundsException",
            &format!("index {index} with {limit} other layers"),
        );
    };
    d.layers.retain(|&l| l != layer);
    d.layers.insert(index, layer);
    Ok(Value::Void)
}

fn get_layer_at(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let index = a[1].int();
    let d = data(ctx, a[0])?;
    let layer = usize::try_from(index)
        .ok()
        .and_then(|i| d.layers.get(i).copied());
    match layer {
        Some(layer) => Ok(ctx.root(layer).into()),
        None => ctx.throw(
            "java/lang/IndexOutOfBoundsException",
            &format!("no layer at index {index}"),
        ),
    }
}

/// `paint(g, x, y)`: the view window's top left corner at `(x, y)` of
/// `g`, clipped to the window, the last layer first. The translation and
/// clip of `g` are given back afterwards.
fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let g = graphics_arg(ctx, a[1])?;
    let (x, y) = (a[2].int(), a[3].int());
    let view = ctx.payload::<ManagerData>(this)?.view;
    let (dx, dy) = (x.wrapping_sub(view.x), y.wrapping_sub(view.y));
    let clip = {
        let state = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
        let clip = Rect::new(
            state.clip_x(),
            state.clip_y(),
            state.clip_width(),
            state.clip_height(),
        );
        state.translate(dx, dy);
        state.clip_rect(view.x, view.y, view.width, view.height);
        clip
    };
    let result = {
        let layers = &ctx.payload::<ManagerData>(this)?.layers;
        let graphics = ctx.payload::<GraphicsData>(g)?;
        layers
            .iter()
            .rev()
            .try_for_each(|&layer| draw(ctx, layer, graphics))
    };
    let state = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
    state.translate(dx.wrapping_neg(), dy.wrapping_neg());
    state.set_clip(clip.x, clip.y, clip.width, clip.height);
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .payload::<ManagerData>()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method("append", &format!("({LAYER})V"), |ctx, a| {
            let layer = layer_arg(ctx, a[1])?;
            let d = data(ctx, a[0])?;
            d.layers.retain(|&l| l != layer);
            d.layers.push(layer);
            Ok(Value::Void)
        })
        .method("insert", &format!("({LAYER}I)V"), insert)
        .method("getLayerAt", &format!("(I){LAYER}"), get_layer_at)
        .method("getSize", "()I", |ctx, a| {
            Ok(Value::Int(data(ctx, a[0])?.layers.len() as i32))
        })
        .method("remove", &format!("({LAYER})V"), |ctx, a| {
            let layer = ctx.non_null(a[1])?;
            data(ctx, a[0])?.layers.retain(|&l| l != layer);
            Ok(Value::Void)
        })
        .method("paint", "(Ljavax/microedition/lcdui/Graphics;II)V", paint)
        .method("setViewWindow", "(IIII)V", |ctx, a| {
            let [x, y, w, h] = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            if w < 0 || h < 0 {
                return ctx.throw(
                    "java/lang/IllegalArgumentException",
                    "the view window's width or height is negative",
                );
            }
            data(ctx, a[0])?.view = Rect::new(x, y, w, h);
            Ok(Value::Void)
        })
}
