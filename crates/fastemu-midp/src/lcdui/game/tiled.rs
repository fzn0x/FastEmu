//! `javax.microedition.lcdui.game.TiledLayer` over the tiled layer model.

use fastemu_jvm::{Ctx, JResult, NativeClass, Value, classfile::access};

use super::layer::{LayerData, Shape, image_arg, image_pixels, image_size, not_a, paint, throw};
use super::model::GameError;
use super::model::tiled::TiledLayer;

const CLASS: &str = "javax/microedition/lcdui/game/TiledLayer";
const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";

/// Runs `f` on the tiled layer `this` holds and returns its value, or
/// throws what it names.
fn edit<R: Into<Value>>(
    ctx: &mut Ctx<'_>,
    this: Value,
    f: impl FnOnce(&mut TiledLayer) -> Result<R, GameError>,
) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let result = match &mut ctx.payload_mut::<LayerData>(this)?.shape {
        Shape::Tiled(layer) => Some(f(layer)),
        Shape::Sprite(_) => None,
    };
    match result {
        Some(Ok(value)) => Ok(value.into()),
        Some(Err(e)) => Err(throw(ctx, e)),
        None => Err(not_a(ctx, "TiledLayer")),
    }
}

/// A native that reads an `int` from the layer.
fn query(f: fn(&TiledLayer) -> i32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| edit(ctx, a[0], |t| Ok(f(t)))
}

fn construct(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let (columns, rows) = (a[1].int(), a[2].int());
    let image = image_arg(ctx, a[3])?;
    let (width, height) = image_size(&image_pixels(ctx, image)?);
    let (tw, th) = (a[4].int(), a[5].int());
    let layer = TiledLayer::new(columns, rows, width, height, tw, th).map_err(|e| throw(ctx, e))?;
    let data = ctx.payload_mut::<LayerData>(this)?;
    data.image = Some(image);
    data.shape = Shape::Tiled(layer);
    Ok(Value::Void)
}

fn set_static_tile_set(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let image = image_arg(ctx, a[1])?;
    let (width, height) = image_size(&image_pixels(ctx, image)?);
    let (tw, th) = (a[2].int(), a[3].int());
    edit(ctx, a[0], |t| {
        t.set_static_tile_set(width, height, tw, th)
            .map(|()| Value::Void)
    })?;
    ctx.payload_mut::<LayerData>(this)?.image = Some(image);
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    let final_method = access::PUBLIC | access::FINAL;
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/game/Layer")
        .payload_with(|| Box::new(LayerData::tiled()))
        .method("<init>", &format!("(II{IMAGE}II)V"), construct)
        .method("createAnimatedTile", "(I)I", |ctx, a| {
            let tile = a[1].int();
            edit(ctx, a[0], |t| t.create_animated_tile(tile))
        })
        .method("setAnimatedTile", "(II)V", |ctx, a| {
            let (animated, tile) = (a[1].int(), a[2].int());
            edit(ctx, a[0], |t| {
                t.set_animated_tile(animated, tile).map(|()| Value::Void)
            })
        })
        .method("getAnimatedTile", "(I)I", |ctx, a| {
            let animated = a[1].int();
            edit(ctx, a[0], |t| t.animated_tile(animated))
        })
        .method("setCell", "(III)V", |ctx, a| {
            let (column, row, tile) = (a[1].int(), a[2].int(), a[3].int());
            edit(ctx, a[0], |t| {
                t.set_cell(column, row, tile).map(|()| Value::Void)
            })
        })
        .method("getCell", "(II)I", |ctx, a| {
            let (column, row) = (a[1].int(), a[2].int());
            edit(ctx, a[0], |t| t.cell(column, row))
        })
        .method("fillCells", "(IIIII)V", |ctx, a| {
            let v: Vec<i32> = a[1..6].iter().map(|v| v.int()).collect();
            edit(ctx, a[0], |t| {
                t.fill_cells(v[0], v[1], v[2], v[3], v[4])
                    .map(|()| Value::Void)
            })
        })
        .method_with(
            "getCellWidth",
            "()I",
            final_method,
            query(TiledLayer::cell_width),
        )
        .method_with(
            "getCellHeight",
            "()I",
            final_method,
            query(TiledLayer::cell_height),
        )
        .method_with(
            "getColumns",
            "()I",
            final_method,
            query(TiledLayer::columns),
        )
        .method_with("getRows", "()I", final_method, query(TiledLayer::rows))
        .method(
            "setStaticTileSet",
            &format!("({IMAGE}II)V"),
            set_static_tile_set,
        )
        .method_with(
            "paint",
            "(Ljavax/microedition/lcdui/Graphics;)V",
            final_method,
            paint,
        )
}
