//! `com.siemens.mp.color_game.TiledLayer`: MIDP 2.0's `TiledLayer`. A grid
//! of cells holds tile numbers: 0 is empty, positive numbers are tiles of
//! the image counted from 1 (left to right, then top to bottom), and
//! negative numbers are animated tiles, each standing for a static tile
//! the game changes.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value, classfile::access};

use super::layer::{GRAPHICS, IMAGE, Kind, LAYER, draw_regions, image_size, layer};
use crate::lcdui::graphics::GraphicsData;

const TILED_LAYER: &str = "com/siemens/mp/color_game/TiledLayer";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const INDEX_OUT_OF_BOUNDS: &str = "java/lang/IndexOutOfBoundsException";
/// The most tiles, static or animated, a tiled layer may have.
const MAX_TILES: i32 = 32_767;

/// A tiled layer's grid and tiles.
pub(in crate::vendor::siemens) struct TiledState {
    tile_width: i32,
    tile_height: i32,
    /// Tiles across the image.
    tiles_across: i32,
    tile_count: i32,
    columns: i32,
    rows: i32,
    cells: Vec<i32>,
    /// The static tile each animated tile shows, animated tile `-1` first.
    animated: Vec<i32>,
}

impl TiledState {
    pub(super) fn heap_bytes(&self) -> usize {
        (self.cells.len() + self.animated.len()) * 4
    }

    fn valid_tile(&self, tile: i32) -> bool {
        (-(self.animated.len() as i32)..=self.tile_count).contains(&tile)
    }
}

/// Checks a tile set against an image: tile counts, sizes and limits.
fn tile_set(ctx: &mut Ctx<'_>, image: ObjRef, tw: i32, th: i32) -> JResult<(i32, i32)> {
    let (iw, ih) = image_size(ctx, image)?;
    if tw < 1 || th < 1 {
        return ctx.throw(ILLEGAL_ARGUMENT, "tile size below 1");
    }
    if iw % tw != 0 || ih % th != 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, "the image is not a whole number of tiles");
    }
    let count = i64::from(iw / tw) * i64::from(ih / th);
    if count > i64::from(MAX_TILES) {
        return ctx.throw(ILLEGAL_ARGUMENT, "more than 32767 tiles");
    }
    Ok((iw / tw, count as i32))
}

fn tiled<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut TiledState> {
    if !matches!(layer(ctx, this)?.kind, Kind::Tiled(_)) {
        return ctx.throw("java/lang/IllegalStateException", "not a tiled layer");
    }
    match &mut layer(ctx, this)?.kind {
        Kind::Tiled(t) => Ok(t),
        _ => unreachable!("the kind was checked above"),
    }
}

fn out_of_bounds<T>(ctx: &mut Ctx<'_>, what: &str) -> JResult<T> {
    ctx.throw(INDEX_OUT_OF_BOUNDS, what)
}

fn init(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (columns, rows) = (a[1].int(), a[2].int());
    let image = ctx.non_null(a[3])?;
    let (tw, th) = (a[4].int(), a[5].int());
    if columns < 1 || rows < 1 {
        return ctx.throw(ILLEGAL_ARGUMENT, "fewer than one row or column");
    }
    let (tiles_across, tile_count) = tile_set(ctx, image, tw, th)?;
    let cells = i64::from(columns) * i64::from(rows);
    if cells as u64 > fastemu_gfx::MAX_PIXELS {
        return ctx.throw("java/lang/OutOfMemoryError", "too many cells");
    }
    let cells = cells as usize;
    // The grid is 4 bytes a cell, up to 128 MiB, and the object holds
    // almost nothing until it is there: ask the heap first, tell it after.
    ctx.check_alloc(cells.saturating_mul(4))?;
    let this = ctx.non_null(a[0])?;
    let l = layer(ctx, a[0])?;
    l.width = columns.saturating_mul(tw);
    l.height = rows.saturating_mul(th);
    l.image = Some(image);
    l.kind = Kind::Tiled(TiledState {
        tile_width: tw,
        tile_height: th,
        tiles_across,
        tile_count,
        columns,
        rows,
        cells: vec![0; cells],
        animated: Vec::new(),
    });
    ctx.recount(this);
    Ok(Value::Void)
}

/// The index of cell `(column, row)`, or `IndexOutOfBoundsException`.
fn cell(ctx: &mut Ctx<'_>, this: Value, column: i32, row: i32) -> JResult<usize> {
    let t = tiled(ctx, this)?;
    if (0..t.columns).contains(&column) && (0..t.rows).contains(&row) {
        Ok((row * t.columns + column) as usize)
    } else {
        out_of_bounds(ctx, "cell outside the grid")
    }
}

fn check_tile(ctx: &mut Ctx<'_>, this: Value, tile: i32) -> JResult<()> {
    if tiled(ctx, this)?.valid_tile(tile) {
        Ok(())
    } else {
        out_of_bounds(ctx, &format!("no tile {tile}"))
    }
}

/// The index of animated tile `index`, or `IndexOutOfBoundsException`.
fn animated(ctx: &mut Ctx<'_>, this: Value, index: i32) -> JResult<usize> {
    let count = tiled(ctx, this)?.animated.len() as i32;
    if index < 0 && -index <= count {
        Ok((-index - 1) as usize)
    } else {
        out_of_bounds(ctx, &format!("no animated tile {index}"))
    }
}

fn fill_cells(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let [column, row, w, h, tile] = [a[1].int(), a[2].int(), a[3].int(), a[4].int(), a[5].int()];
    if w < 0 || h < 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, "negative size");
    }
    check_tile(ctx, a[0], tile)?;
    let t = tiled(ctx, a[0])?;
    let inside = column >= 0
        && row >= 0
        && i64::from(column) + i64::from(w) <= i64::from(t.columns)
        && i64::from(row) + i64::from(h) <= i64::from(t.rows);
    if !inside {
        return out_of_bounds(ctx, "region outside the grid");
    }
    for r in row..row + h {
        let start = (r * t.columns + column) as usize;
        t.cells[start..start + w as usize].fill(tile);
    }
    Ok(Value::Void)
}

fn set_static_tile_set(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let image = ctx.non_null(a[1])?;
    let (tw, th) = (a[2].int(), a[3].int());
    let (tiles_across, tile_count) = tile_set(ctx, image, tw, th)?;
    let l = layer(ctx, a[0])?;
    l.image = Some(image);
    let Kind::Tiled(t) = &mut l.kind else {
        return ctx.throw("java/lang/IllegalStateException", "not a tiled layer");
    };
    if tile_count < t.tile_count {
        t.cells.fill(0);
        t.animated.clear();
    }
    (t.tile_width, t.tile_height, t.tiles_across, t.tile_count) =
        (tw, th, tiles_across, tile_count);
    let (w, h) = (t.columns.saturating_mul(tw), t.rows.saturating_mul(th));
    (l.width, l.height) = (w, h);
    // A smaller tile set deletes the animated tiles.
    let this = ctx.non_null(a[0])?;
    ctx.recount(this);
    Ok(Value::Void)
}

/// Paints the cells inside the clip of `g`.
fn paint(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let g = ctx.non_null(a[1])?;
    let clip = {
        let s = &ctx.payload::<GraphicsData>(g)?.state;
        [s.clip_x(), s.clip_y(), s.clip_width(), s.clip_height()].map(i64::from)
    };
    let l = layer(ctx, a[0])?;
    let (x, y, visible, image) = (i64::from(l.x), i64::from(l.y), l.visible, l.image);
    let (Kind::Tiled(t), Some(image), true) = (&l.kind, image, visible) else {
        return Ok(Value::Void);
    };
    let (tw, th) = (i64::from(t.tile_width), i64::from(t.tile_height));
    let first_column = ((clip[0] - x).div_euclid(tw)).max(0);
    let last_column = ((clip[0] + clip[2] - x + tw - 1).div_euclid(tw)).min(t.columns.into());
    let first_row = ((clip[1] - y).div_euclid(th)).max(0);
    let last_row = ((clip[1] + clip[3] - y + th - 1).div_euclid(th)).min(t.rows.into());
    let mut regions = Vec::new();
    for row in first_row..last_row {
        for column in first_column..last_column {
            let mut tile = t.cells[(row * i64::from(t.columns) + column) as usize];
            if tile < 0 {
                tile = t.animated.get((-tile - 1) as usize).copied().unwrap_or(0);
            }
            if tile <= 0 || tile > t.tile_count {
                continue;
            }
            let source = tile - 1;
            regions.push((
                source % t.tiles_across * t.tile_width,
                source / t.tiles_across * t.tile_height,
                (x + column * tw) as i32,
                (y + row * th) as i32,
            ));
        }
    }
    let size = (t.tile_width, t.tile_height);
    draw_regions(ctx, g, image, size, &regions)?;
    Ok(Value::Void)
}

/// The class.
pub(super) fn class() -> NativeClass {
    let final_method = access::PUBLIC | access::FINAL;
    let getter = |f: fn(&TiledState) -> i32| {
        move |ctx: &mut Ctx<'_>, a: &[Value]| Ok(Value::Int(f(tiled(ctx, a[0])?)))
    };
    NativeClass::new(TILED_LAYER)
        .extends(LAYER)
        .method("<init>", &format!("(II{IMAGE}II)V"), init)
        .method("createAnimatedTile", "(I)I", |ctx, a| {
            let tile = a[1].int();
            let t = tiled(ctx, a[0])?;
            if !(0..=t.tile_count).contains(&tile) {
                return out_of_bounds(ctx, &format!("no static tile {tile}"));
            }
            if t.animated.len() >= MAX_TILES as usize {
                return ctx.throw(ILLEGAL_ARGUMENT, "more than 32767 animated tiles");
            }
            t.animated.push(tile);
            let index = -(t.animated.len() as i32);
            // The tile makes no Java object, so only the count tells the
            // heap that a loop of these grows.
            let this = ctx.non_null(a[0])?;
            ctx.recount(this);
            Ok(Value::Int(index))
        })
        .method("getAnimatedTile", "(I)I", |ctx, a| {
            let index = animated(ctx, a[0], a[1].int())?;
            Ok(Value::Int(tiled(ctx, a[0])?.animated[index]))
        })
        .method("setAnimatedTile", "(II)V", |ctx, a| {
            let index = animated(ctx, a[0], a[1].int())?;
            let tile = a[2].int();
            let t = tiled(ctx, a[0])?;
            if !(0..=t.tile_count).contains(&tile) {
                return out_of_bounds(ctx, &format!("no static tile {tile}"));
            }
            t.animated[index] = tile;
            Ok(Value::Void)
        })
        .method("fillCells", "(IIIII)V", fill_cells)
        .method("getCell", "(II)I", |ctx, a| {
            let at = cell(ctx, a[0], a[1].int(), a[2].int())?;
            Ok(Value::Int(tiled(ctx, a[0])?.cells[at]))
        })
        .method("setCell", "(III)V", |ctx, a| {
            let at = cell(ctx, a[0], a[1].int(), a[2].int())?;
            let tile = a[3].int();
            check_tile(ctx, a[0], tile)?;
            tiled(ctx, a[0])?.cells[at] = tile;
            Ok(Value::Void)
        })
        .method_with(
            "getCellHeight",
            "()I",
            final_method,
            getter(|t| t.tile_height),
        )
        .method_with(
            "getCellWidth",
            "()I",
            final_method,
            getter(|t| t.tile_width),
        )
        .method_with("getColumns", "()I", final_method, getter(|t| t.columns))
        .method_with("getRows", "()I", final_method, getter(|t| t.rows))
        .method_with("paint", &format!("({GRAPHICS})V"), final_method, paint)
        .method(
            "setStaticTileSet",
            &format!("({IMAGE}II)V"),
            set_static_tile_set,
        )
}
