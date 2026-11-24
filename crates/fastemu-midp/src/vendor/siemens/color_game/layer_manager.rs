//! `com.siemens.mp.color_game.LayerManager`: MIDP 2.0's `LayerManager`.
//! Layers paint from the last to the first, so the first is on top, seen
//! through a view window placed at `(x, y)` on the `Graphics`.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value, classfile::access};

use super::layer::{GRAPHICS, LayerData, NATIVE_MEM_DESC, NATIVE_MEM_FIELD};
use crate::lcdui::graphics::GraphicsData;

const LAYER_MANAGER: &str = "com/siemens/mp/color_game/LayerManager";
const LAYER: &str = "Lcom/siemens/mp/color_game/Layer;";

/// What a `LayerManager` holds.
struct Manager {
    layers: Vec<ObjRef>,
    /// x, y, width and height of the view window.
    view: [i32; 4],
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            layers: Vec::new(),
            view: [0, 0, i32::MAX, i32::MAX],
        }
    }
}

impl Payload for Manager {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &layer in &self.layers {
            tracer.mark(layer);
        }
    }
}

fn manager<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut Manager> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<Manager>(this)
}

/// Takes `layer` out of the list if it is there.
fn take_out(ctx: &mut Ctx<'_>, this: Value, layer: ObjRef) -> JResult<()> {
    manager(ctx, this)?.layers.retain(|&l| l != layer);
    Ok(())
}

fn insert(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let layer = ctx.non_null(a[1])?;
    let index = a[2].int();
    take_out(ctx, a[0], layer)?;
    let layers = &mut manager(ctx, a[0])?.layers;
    match usize::try_from(index) {
        Ok(i) if i <= layers.len() => {
            layers.insert(i, layer);
            Ok(Value::Void)
        }
        _ => ctx.throw("java/lang/IndexOutOfBoundsException", &format!("{index}")),
    }
}

/// `paint(g, x, y)`: clips `g` to the view window at `(x, y)`, paints
/// every visible layer from the last to the first, and gives `g` its
/// translation and clip back.
fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let g = ctx.non_null(a[1])?;
    let (x, y) = (a[2].int(), a[3].int());
    let (layers, [vx, vy, vw, vh]) = {
        let m = manager(ctx, a[0])?;
        (m.layers.clone(), m.view)
    };
    // A layer's paint may take another layer out of the manager, and the
    // collector may run during it, so every layer is rooted first.
    let layers: Vec<ObjRef> = layers.into_iter().map(|l| ctx.root(l)).collect();
    let saved = {
        let s = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
        let saved = (
            s.translate_x(),
            s.translate_y(),
            [s.clip_x(), s.clip_y(), s.clip_width(), s.clip_height()],
        );
        s.clip_rect(x, y, vw, vh);
        s.translate(x.wrapping_sub(vx), y.wrapping_sub(vy));
        saved
    };
    let mut result = Ok(());
    for &layer in layers.iter().rev() {
        if !ctx
            .try_payload::<LayerData>(layer)
            .is_some_and(|l| l.visible)
        {
            continue;
        }
        result = ctx
            .call_virtual(layer, "paint", &format!("({GRAPHICS})V"), &[g.into()])
            .map(drop);
        if result.is_err() {
            break;
        }
    }
    let s = &mut ctx.payload_mut::<GraphicsData>(g)?.state;
    let (tx, ty, [cx, cy, cw, ch]) = saved;
    s.translate(
        tx.wrapping_sub(s.translate_x()),
        ty.wrapping_sub(s.translate_y()),
    );
    s.set_clip(cx, cy, cw, ch);
    result.map(|()| Value::Void)
}

/// The class.
pub(super) fn class() -> NativeClass {
    NativeClass::new(LAYER_MANAGER)
        .payload::<Manager>()
        .field_with(NATIVE_MEM_FIELD, NATIVE_MEM_DESC, access::PROTECTED)
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method("append", &format!("({LAYER})V"), |ctx, a| {
            let layer = ctx.non_null(a[1])?;
            take_out(ctx, a[0], layer)?;
            manager(ctx, a[0])?.layers.push(layer);
            Ok(Value::Void)
        })
        .method("insert", &format!("({LAYER}I)V"), insert)
        .method("getLayerAt", &format!("(I){LAYER}"), |ctx, a| {
            let index = a[1].int();
            let layers = &manager(ctx, a[0])?.layers;
            match usize::try_from(index).ok().and_then(|i| layers.get(i)) {
                Some(&layer) => Ok(layer.into()),
                None => ctx.throw("java/lang/IndexOutOfBoundsException", &format!("{index}")),
            }
        })
        .method("getSize", "()I", |ctx, a| {
            Ok(Value::Int(manager(ctx, a[0])?.layers.len() as i32))
        })
        .method("remove", &format!("({LAYER})V"), |ctx, a| {
            let layer = ctx.non_null(a[1])?;
            take_out(ctx, a[0], layer)?;
            Ok(Value::Void)
        })
        .method(
            "removeLayer",
            &format!("({NATIVE_MEM_DESC}{LAYER})V"),
            |ctx, a| {
                if let Some(layer) = a[2].obj() {
                    take_out(ctx, a[0], layer)?;
                }
                Ok(Value::Void)
            },
        )
        .method("paint", &format!("({GRAPHICS}II)V"), paint)
        .method("setViewWindow", "(IIII)V", |ctx, a| {
            let view = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            if view[2] < 0 || view[3] < 0 {
                return ctx.throw("java/lang/IllegalArgumentException", "negative size");
            }
            manager(ctx, a[0])?.view = view;
            Ok(Value::Void)
        })
}
