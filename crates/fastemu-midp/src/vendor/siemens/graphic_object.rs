//! Siemens' `com.siemens.mp.game` graphic objects: `GraphicObject`, its
//! `Sprite` and `TiledBackground`, and the `GraphicObjectManager` that
//! paints them, first to last, onto an image.
//!
//! Sprites hold their frames as pictures, transparent where the mask is
//! white (or its bit clear). Backgrounds have 8 by 8 tiles: 0 is
//! transparent, 1 white, 2 black and the game's own tiles start at 3. A
//! background keeps the game's map array, so changing the array changes
//! what is drawn, and the map repeats across the drawing area as the
//! javadoc says. The manager draws objects offset by its `x` and `y`.

use fastemu_gfx::{GfxError, Graphics, Surface, SurfaceKind};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value, classfile::access};

use super::NATIVE_MEM;
use super::bitmap::{apply_mask, copy_of, from_1bpp};
use super::extended_image::{self, ExtendedData};
use crate::lcdui::font::default_font;
use crate::lcdui::highlevel::trim;
use crate::lcdui::image::surface_of;
use crate::lcdui::throw_gfx;

const GRAPHIC_OBJECT: &str = "com/siemens/mp/game/GraphicObject";
const SPRITE: &str = "com/siemens/mp/game/Sprite";
const TILED: &str = "com/siemens/mp/game/TiledBackground";
const MANAGER: &str = "com/siemens/mp/game/GraphicObjectManager";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const IMAGE: &str = "Ljavax/microedition/lcdui/Image;";
const EXTENDED: &str = "Lcom/siemens/mp/game/ExtendedImage;";

/// The side of a background tile.
const TILE: i32 = 8;

/// A sprite's frames, place and collision rectangle.
struct SpriteData {
    frames: Vec<Surface>,
    x: i32,
    y: i32,
    frame: i32,
    /// The collision rectangle relative to the sprite: x, y, width,
    /// height.
    collision: [i32; 4],
}

impl SpriteData {
    /// The collision rectangle on the screen: left, top, right, bottom.
    fn bounds(&self) -> [i64; 4] {
        let [cx, cy, cw, ch] = self.collision.map(i64::from);
        let (x, y) = (i64::from(self.x) + cx, i64::from(self.y) + cy);
        [x, y, x + cw, y + ch]
    }

    fn paint(&self, g: &Graphics, target: &mut Surface, x: i32, y: i32) -> Result<(), GfxError> {
        match usize::try_from(self.frame)
            .ok()
            .and_then(|f| self.frames.get(f))
        {
            Some(frame) => g.draw_image(
                target,
                frame,
                x.wrapping_add(self.x),
                y.wrapping_add(self.y),
                0,
            ),
            None => Ok(()),
        }
    }
}

/// A background's tiles (the three predefined ones first) and its map.
struct Tiles {
    set: Vec<Surface>,
    /// The game's `byte[]` map, one tile index per cell.
    map: ObjRef,
    width: i32,
    height: i32,
    position: (i32, i32),
}

impl Tiles {
    /// Draws the map from its position onto the area of `target` that
    /// starts at `(x, y)`, repeating it.
    fn paint(
        &self,
        g: &Graphics,
        target: &mut Surface,
        map: &[i8],
        x: i32,
        y: i32,
    ) -> Result<(), GfxError> {
        let (right, bottom) = (target.width() as i64, target.height() as i64);
        let (px, py) = (i64::from(self.position.0), i64::from(self.position.1));
        let (w, h) = (i64::from(self.width), i64::from(self.height));
        let tile = i64::from(TILE);
        // Whole tiles above or left of the target are skipped, not
        // stepped over one by one.
        let skip = |start: i64| (-start).max(0) / tile;
        let mut top = i64::from(y) - py.rem_euclid(tile);
        let mut row = py.div_euclid(tile) + skip(top);
        top += skip(top) * tile;
        let first_left = i64::from(x) - px.rem_euclid(tile);
        let first_column = px.div_euclid(tile) + skip(first_left);
        let first_left = first_left + skip(first_left) * tile;
        while top < bottom {
            let mut column = first_column;
            let mut left = first_left;
            while left < right {
                let cell = (row.rem_euclid(h) * w + column.rem_euclid(w)) as usize;
                let index = map.get(cell).map_or(0, |&b| b as u8 as usize);
                if index > 0
                    && let Some(tile) = self.set.get(index)
                {
                    g.draw_image(target, tile, left as i32, top as i32, 0)?;
                }
                left += tile;
                column += 1;
            }
            top += tile;
            row += 1;
        }
        Ok(())
    }
}

/// What a sprite or background draws.
#[derive(Default)]
enum Shape {
    /// A game's own subclass of `GraphicObject`, which nothing draws.
    #[default]
    None,
    Sprite(SpriteData),
    Tiles(Tiles),
}

/// What every `GraphicObject` holds.
struct GraphicData {
    visible: bool,
    shape: Shape,
}

impl Default for GraphicData {
    fn default() -> Self {
        Self {
            visible: true,
            shape: Shape::None,
        }
    }
}

impl Payload for GraphicData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Shape::Tiles(tiles) = &self.shape {
            tracer.mark(tiles.map);
        }
    }

    fn heap_bytes(&self) -> usize {
        let pictures = match &self.shape {
            Shape::None => return 0,
            Shape::Sprite(s) => &s.frames,
            Shape::Tiles(t) => &t.set,
        };
        pictures
            .iter()
            .map(|p| p.width() as usize * p.height() as usize * 4)
            .sum()
    }
}

fn data<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut GraphicData> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<GraphicData>(this)
}

fn sprite<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut SpriteData> {
    if !matches!(data(ctx, this)?.shape, Shape::Sprite(_)) {
        return ctx.throw("java/lang/IllegalStateException", "not a sprite");
    }
    match &mut data(ctx, this)?.shape {
        Shape::Sprite(s) => Ok(s),
        _ => unreachable!("the shape was checked above"),
    }
}

fn illegal<T>(ctx: &mut Ctx<'_>, why: &str) -> JResult<T> {
    ctx.throw(ILLEGAL_ARGUMENT, why)
}

/// The MIDP image of an `ExtendedImage` argument, `None` for `null`.
fn extended(ctx: &mut Ctx<'_>, value: Value) -> JResult<Option<ObjRef>> {
    match value.obj() {
        Some(e) => {
            ctx.payload::<ExtendedData>(e)?;
            Ok(Some(extended_image::image_of(ctx, e)?))
        }
        None => Ok(None),
    }
}

/// A picture and its optional mask from MIDP images, checked as the
/// constructors' javadoc says: no transparent pixels, the same size, and
/// a width `width_ok` accepts.
fn masked(
    ctx: &mut Ctx<'_>,
    pixels: Option<ObjRef>,
    mask: Option<ObjRef>,
    width_ok: fn(u32) -> bool,
) -> JResult<Surface> {
    let Some(pixels) = copy_of(ctx, pixels)? else {
        return ctx.throw("java/lang/NullPointerException", "no pixels");
    };
    let mask = copy_of(ctx, mask)?;
    let pictures = std::iter::once(&pixels).chain(mask.as_ref());
    for picture in pictures {
        if !width_ok(picture.width()) {
            return illegal(ctx, "wrong image width");
        }
        if !picture.is_opaque() {
            return illegal(ctx, "an image with transparent pixels");
        }
    }
    match mask {
        Some(mask) if (mask.width(), mask.height()) != (pixels.width(), pixels.height()) => {
            illegal(ctx, "the image and the mask differ in size")
        }
        Some(mask) => apply_mask(&pixels, &mask).map_err(|e| throw_gfx(ctx, &e)),
        None => Ok(pixels),
    }
}

/// Cuts `picture` into `count` rows of the same height.
fn rows(picture: &Surface, count: usize) -> Result<Vec<Surface>, GfxError> {
    let (w, h) = (picture.width() as usize, picture.height() as usize);
    let band = h / count.max(1);
    (0..count)
        .map(|i| {
            let start = i * band * w;
            let pixels = picture.pixels()[start..start + band * w].to_vec();
            Surface::from_pixels(w as u32, band as u32, pixels, SurfaceKind::Immutable)
        })
        .collect()
}

/// The bytes the pixels of `picture` take.
fn pixel_bytes(picture: &Surface) -> usize {
    (picture.width() as usize)
        .saturating_mul(picture.height() as usize)
        .saturating_mul(4)
}

/// Counts the pictures `this` was just given against the heap, as the
/// allocation of an object of their size would be: a game that makes
/// sprites in a loop makes no other object of size, and no collection
/// would run.
fn count_frames(ctx: &mut Ctx<'_>, this: Value) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    ctx.recount(this);
    Ok(Value::Void)
}

/// Makes `this` a sprite of `picture` cut into `frames` frames.
fn make_sprite(ctx: &mut Ctx<'_>, this: Value, picture: &Surface, frames: i32) -> JResult<Value> {
    if frames < 1 || !picture.height().is_multiple_of(frames as u32) {
        return illegal(ctx, "wrong number of frames");
    }
    ctx.check_alloc(pixel_bytes(picture))?;
    let frames = rows(picture, frames as usize).map_err(|e| throw_gfx(ctx, &e))?;
    let (w, h) = (frames[0].width() as i32, frames[0].height() as i32);
    data(ctx, this)?.shape = Shape::Sprite(SpriteData {
        frames,
        x: 0,
        y: 0,
        frame: 0,
        collision: [0, 0, w, h],
    });
    count_frames(ctx, this)
}

fn sprite_from_images(
    ctx: &mut Ctx<'_>,
    this: Value,
    pixels: Option<ObjRef>,
    mask: Option<ObjRef>,
    frames: i32,
) -> JResult<Value> {
    let picture = masked(ctx, pixels, mask, |w| w % 8 == 0)?;
    make_sprite(ctx, this, &picture, frames)
}

/// `Sprite(byte[] pixels, int pixel_offset, int width, int height, byte[]
/// mask, int mask_offset, int numFrames)`: one-bit frames of `width` by
/// `height`, one under the other.
fn sprite_from_bits(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let pixels = ctx.non_null(a[1])?;
    let (offset, w, h) = (a[2].int(), a[3].int(), a[4].int());
    let (mask, mask_offset, frames) = (a[5].obj(), a[6].int(), a[7].int());
    if w <= 0 || h <= 0 || w % 8 != 0 || frames < 1 {
        return illegal(ctx, "wrong sprite size or number of frames");
    }
    let need = i64::from(w) * i64::from(h) * i64::from(frames) / 8;
    let slice = |ctx: &Ctx<'_>, array: ObjRef, offset: i32| -> JResult<Option<Vec<i8>>> {
        let all = ctx.array::<i8>(array)?;
        let from = usize::try_from(offset).ok();
        Ok(from
            .filter(|&f| (all.len() as i64 - f as i64) >= need)
            .map(|f| all[f..].to_vec()))
    };
    let Some(bits) = slice(ctx, pixels, offset)? else {
        return illegal(ctx, "the pixel array is too small");
    };
    let mask_bits = match mask {
        Some(mask) => match slice(ctx, mask, mask_offset)? {
            Some(bits) => Some(bits),
            None => return illegal(ctx, "the mask array is too small"),
        },
        None => None,
    };
    let picture =
        from_1bpp(&bits, mask_bits.as_deref(), w, h * frames).map_err(|e| throw_gfx(ctx, &e))?;
    make_sprite(ctx, a[0], &picture, frames)
}

fn is_colliding_with(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let other = ctx.non_null(a[1])?;
    let theirs = sprite(ctx, Value::from(other))?.bounds();
    let ours = sprite(ctx, a[0])?.bounds();
    let overlap =
        ours[0] < theirs[2] && theirs[0] < ours[2] && ours[1] < theirs[3] && theirs[1] < ours[3];
    Ok(Value::from(overlap))
}

/// The tiles of a background: transparent, white and black, then the
/// game's, cut from a column of 8 by 8 tiles.
fn tile_set(tiles: &Surface) -> Result<Vec<Surface>, GfxError> {
    let solid = |colour| Surface::from_pixels(8, 8, vec![colour; 64], SurfaceKind::Immutable);
    let mut all = vec![
        solid(0x00FF_FFFF)?,
        solid(0xFFFF_FFFF)?,
        solid(0xFF00_0000)?,
    ];
    let count = (tiles.height() / TILE as u32) as usize;
    if count > 0 {
        let pixels = tiles.pixels()[..count * 64].to_vec();
        let column = Surface::from_pixels(8, count as u32 * 8, pixels, SurfaceKind::Immutable)?;
        all.extend(rows(&column, count)?);
    }
    Ok(all)
}

/// Makes `this` a background of `tiles` (user tiles, 8 pixels wide)
/// over the game's `map`.
fn make_tiles(ctx: &mut Ctx<'_>, a: &[Value], tiles: &Surface) -> JResult<Value> {
    let map = ctx.non_null(a[3])?;
    let (w, h) = (a[4].int(), a[5].int());
    if w <= 0 || h <= 0 {
        return illegal(ctx, "the map has no cells");
    }
    ctx.check_alloc(pixel_bytes(tiles))?;
    let all = tile_set(tiles).map_err(|e| throw_gfx(ctx, &e))?;
    let cells = ctx.array::<i8>(map)?;
    if (cells.len() as i64) < i64::from(w) * i64::from(h) {
        return illegal(ctx, "the map is smaller than its size");
    }
    if cells.iter().any(|&c| c as u8 as usize >= all.len()) {
        return illegal(ctx, "a wrong tile number in the map");
    }
    data(ctx, a[0])?.shape = Shape::Tiles(Tiles {
        set: all,
        map,
        width: w,
        height: h,
        position: (0, 0),
    });
    count_frames(ctx, a[0])
}

fn tiles_from_images(
    ctx: &mut Ctx<'_>,
    a: &[Value],
    pixels: Option<ObjRef>,
    mask: Option<ObjRef>,
) -> JResult<Value> {
    let tiles = masked(ctx, pixels, mask, |w| w == TILE as u32)?;
    make_tiles(ctx, a, &tiles)
}

fn tiles_from_bits(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let pixels = ctx.non_null(a[1])?;
    let bits = ctx.array::<i8>(pixels)?.to_vec();
    let mask = match a[2].obj() {
        Some(mask) => Some(ctx.array::<i8>(mask)?.to_vec()),
        None => None,
    };
    if bits.is_empty()
        || bits.len() % 8 != 0
        || mask.as_ref().is_some_and(|m| m.len() != bits.len())
    {
        return illegal(ctx, "the tile arrays must be equal multiples of 8 bytes");
    }
    let tiles = from_1bpp(&bits, mask.as_deref(), TILE, bits.len() as i32)
        .map_err(|e| throw_gfx(ctx, &e))?;
    make_tiles(ctx, a, &tiles)
}

/// What a `GraphicObjectManager` holds: its objects, first drawn first.
#[derive(Default)]
struct Manager {
    objects: Vec<ObjRef>,
}

impl Payload for Manager {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &object in &self.objects {
            tracer.mark(object);
        }
    }

    /// The room of the list, which the heap counts against the game: the
    /// objects are Java objects and count as themselves, but a list that
    /// takes the same one again and again grows without making any.
    fn heap_bytes(&self) -> usize {
        self.objects.capacity() * size_of::<ObjRef>()
    }
}

fn manager<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut Manager> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<Manager>(this)
}

/// The object at `index`, `ArrayIndexOutOfBoundsException` when there is
/// none, as `Vector` throws.
fn position(ctx: &mut Ctx<'_>, this: Value, index: i32, inclusive: bool) -> JResult<usize> {
    let len = manager(ctx, this)?.objects.len();
    match usize::try_from(index) {
        Ok(i) if i < len || (inclusive && i == len) => Ok(i),
        _ => ctx.throw(
            "java/lang/ArrayIndexOutOfBoundsException",
            &format!("{index} >= {len}"),
        ),
    }
}

/// `paint(image, x, y)`: every visible object, first to last.
fn paint(ctx: &mut Ctx<'_>, this: Value, image: ObjRef, x: i32, y: i32) -> JResult<Value> {
    let objects = manager(ctx, this)?.objects.clone();
    let target = surface_of(ctx, image)?;
    let font = default_font(ctx);
    let mut surface = target.lock();
    if !surface.is_opaque() {
        drop(surface);
        return illegal(ctx, "an image with transparent pixels");
    }
    let g = match Graphics::new(&surface, font) {
        Ok(g) => g,
        Err(e) => {
            drop(surface);
            return Err(throw_gfx(ctx, &e));
        }
    };
    let mut result = Ok(());
    for object in objects {
        let Some(d) = ctx.try_payload::<GraphicData>(object) else {
            continue;
        };
        if !d.visible {
            continue;
        }
        result = match &d.shape {
            Shape::None => Ok(()),
            Shape::Sprite(s) => s.paint(&g, &mut surface, x, y),
            Shape::Tiles(t) => match ctx.array::<i8>(t.map) {
                Ok(map) => t.paint(&g, &mut surface, map, x, y),
                Err(_) => Ok(()),
            },
        };
        if result.is_err() {
            break;
        }
    }
    drop(surface);
    result.map(|()| Value::Void).map_err(|e| throw_gfx(ctx, &e))
}

/// `GraphicObject`, `Sprite`, `TiledBackground` and
/// `GraphicObjectManager`.
pub fn classes() -> Vec<NativeClass> {
    vec![
        graphic_object(),
        sprite_class(),
        tiled_class(),
        manager_class(),
    ]
}

fn graphic_object() -> NativeClass {
    NativeClass::new(GRAPHIC_OBJECT)
        .extends(NATIVE_MEM)
        .payload::<GraphicData>()
        .method_with("<init>", "()V", access::PROTECTED, |_, _| Ok(Value::Void))
        .method("setVisible", "(Z)V", |ctx, a| {
            let visible = a[1].boolean();
            data(ctx, a[0])?.visible = visible;
            Ok(Value::Void)
        })
        .method("getVisible", "()Z", |ctx, a| {
            Ok(Value::from(data(ctx, a[0])?.visible))
        })
}

fn sprite_class() -> NativeClass {
    NativeClass::new(SPRITE)
        .extends(GRAPHIC_OBJECT)
        .method("<init>", &format!("({IMAGE}{IMAGE}I)V"), |ctx, a| {
            sprite_from_images(ctx, a[0], a[1].obj(), a[2].obj(), a[3].int())
        })
        .method("<init>", &format!("({EXTENDED}{EXTENDED}I)V"), |ctx, a| {
            let pixels = extended(ctx, a[1])?;
            let mask = extended(ctx, a[2])?;
            sprite_from_images(ctx, a[0], pixels, mask, a[3].int())
        })
        .method("<init>", "([BIII[BII)V", sprite_from_bits)
        .method("setPosition", "(II)V", |ctx, a| {
            let (x, y) = (a[1].int(), a[2].int());
            let s = sprite(ctx, a[0])?;
            (s.x, s.y) = (x, y);
            Ok(Value::Void)
        })
        .method("getXPosition", "()I", |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.x))
        })
        .method("getYPosition", "()I", |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.y))
        })
        .method("setCollisionRectangle", "(IIII)V", |ctx, a| {
            let rect = [a[1].int(), a[2].int(), a[3].int(), a[4].int()];
            sprite(ctx, a[0])?.collision = rect;
            Ok(Value::Void)
        })
        .method(
            "isCollidingWith",
            "(Lcom/siemens/mp/game/Sprite;)Z",
            is_colliding_with,
        )
        .method("isCollidingWithPos", "(II)Z", |ctx, a| {
            let (x, y) = (i64::from(a[1].int()), i64::from(a[2].int()));
            let [left, top, right, bottom] = sprite(ctx, a[0])?.bounds();
            Ok(Value::from(
                (left..right).contains(&x) && (top..bottom).contains(&y),
            ))
        })
        .method("setFrame", "(I)V", |ctx, a| {
            let frame = a[1].int();
            sprite(ctx, a[0])?.frame = frame;
            Ok(Value::Void)
        })
        .method("getFrame", "()I", |ctx, a| {
            Ok(Value::Int(sprite(ctx, a[0])?.frame))
        })
}

fn tiled_class() -> NativeClass {
    NativeClass::new(TILED)
        .extends(GRAPHIC_OBJECT)
        .method("<init>", &format!("({IMAGE}{IMAGE}[BII)V"), |ctx, a| {
            tiles_from_images(ctx, a, a[1].obj(), a[2].obj())
        })
        .method(
            "<init>",
            &format!("({EXTENDED}{EXTENDED}[BII)V"),
            |ctx, a| {
                let pixels = extended(ctx, a[1])?;
                let mask = extended(ctx, a[2])?;
                tiles_from_images(ctx, a, pixels, mask)
            },
        )
        .method("<init>", "([B[B[BII)V", tiles_from_bits)
        .method("setPositionInMap", "(II)V", |ctx, a| {
            let position = (a[1].int(), a[2].int());
            if let Shape::Tiles(t) = &mut data(ctx, a[0])?.shape {
                t.position = position;
            }
            Ok(Value::Void)
        })
}

fn manager_class() -> NativeClass {
    let object = "Lcom/siemens/mp/game/GraphicObject;";
    NativeClass::new(MANAGER)
        .extends(NATIVE_MEM)
        .payload::<Manager>()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method("addObject", &format!("({object})V"), |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let object = ctx.non_null(a[1])?;
            let objects = &mut manager(ctx, a[0])?.objects;
            // An object is in the list once, as a layer is in a
            // `LayerManager`'s; adding it again changes nothing.
            if !objects.contains(&object) {
                objects.push(object);
                ctx.recount(this);
            }
            Ok(Value::Void)
        })
        .method("insertObject", &format!("({object}I)V"), |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let object = ctx.non_null(a[1])?;
            let at = position(ctx, a[0], a[2].int(), true)?;
            manager(ctx, a[0])?.objects.insert(at, object);
            ctx.recount(this);
            Ok(Value::Void)
        })
        .method("getObjectPosition", &format!("({object})I"), |ctx, a| {
            let object = a[1].obj();
            let objects = &manager(ctx, a[0])?.objects;
            let found = objects.iter().position(|&o| Some(o) == object);
            Ok(Value::Int(found.map_or(-1, |i| i as i32)))
        })
        .method("getObjectAt", &format!("(I){object}"), |ctx, a| {
            let at = position(ctx, a[0], a[1].int(), false)?;
            Ok(manager(ctx, a[0])?.objects[at].into())
        })
        .method("deleteObject", &format!("({object})V"), |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let object = a[1].obj();
            let objects = &mut manager(ctx, a[0])?.objects;
            if let Some(i) = objects.iter().position(|&o| Some(o) == object) {
                objects.remove(i);
                trim(objects);
                ctx.recount(this);
            }
            Ok(Value::Void)
        })
        .method("deleteObject", "(I)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let at = position(ctx, a[0], a[1].int(), false)?;
            let objects = &mut manager(ctx, a[0])?.objects;
            objects.remove(at);
            trim(objects);
            ctx.recount(this);
            Ok(Value::Void)
        })
        .method("paint", &format!("({IMAGE}II)V"), |ctx, a| {
            let image = ctx.non_null(a[1])?;
            paint(ctx, a[0], image, a[2].int(), a[3].int())
        })
        .method("paint", &format!("({EXTENDED}II)V"), |ctx, a| {
            let extended = ctx.non_null(a[1])?;
            let image = extended_image::image_of(ctx, extended)?;
            paint(ctx, a[0], image, a[2].int(), a[3].int())
        })
}
