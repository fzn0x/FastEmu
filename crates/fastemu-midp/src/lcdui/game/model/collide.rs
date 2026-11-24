//! `Sprite.collidesWith`: a sprite against another sprite, a tiled layer
//! or an image, by collision rectangle or pixel by pixel.
//!
//! Rectangles collide as the reference implementation tests them
//! ([`Area::overlaps`]). Pixel-level tests then look only at pixels inside
//! both collision rectangles and inside the sprites' frames, and a pixel
//! counts only when its alpha is `0xFF`, as J2ME Loader and freej2me-plus
//! count it, so transparent and partly transparent pixels never collide.

use fastemu_gfx::{Surface, Transform};

use super::Area;
use super::tiled::TiledLayer;

/// What a collision test needs of a sprite, copied out of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub visible: bool,
    /// The transformed frame on the layer plane.
    pub bounds: Area,
    /// The transformed collision rectangle on the layer plane.
    pub collision: Area,
    /// The untransformed frame in the sprite's image.
    pub frame: Area,
    pub transform: Transform,
}

/// The transform that undoes `t`, applied to the transformed size.
fn inverse(t: Transform) -> Transform {
    match t {
        Transform::Rot90 => Transform::Rot270,
        Transform::Rot270 => Transform::Rot90,
        other => other,
    }
}

/// Whether a pixel is opaque enough to collide.
fn is_opaque(pixel: Option<u32>) -> bool {
    pixel.is_some_and(|p| p >> 24 == 0xFF)
}

/// The pixel of `image` at `(x, y)`, if inside it.
fn pixel_at(image: &Surface, x: i64, y: i64) -> Option<u32> {
    let (x, y) = (u32::try_from(x).ok()?, u32::try_from(y).ok()?);
    image.pixel(x, y)
}

impl Footprint {
    /// The collision rectangle cut to the frame, which is all a
    /// pixel-level test looks at.
    fn pixel_area(&self) -> Area {
        self.collision.intersect(self.bounds)
    }

    /// Whether the sprite shows an opaque pixel at `(x, y)` of the layer
    /// plane, `image` being the sprite's image.
    pub fn is_opaque_at(&self, image: &Surface, x: i64, y: i64) -> bool {
        let (u, v) = (x - self.bounds.left, y - self.bounds.top);
        let (w, h) = (self.bounds.width(), self.bounds.height());
        if u < 0 || v < 0 || u >= w || v >= h {
            return false;
        }
        let (fx, fy) = inverse(self.transform).apply(u, v, w, h);
        is_opaque(pixel_at(image, self.frame.left + fx, self.frame.top + fy))
    }
}

/// Whether any pixel of `area` passes `test`.
fn any_pixel(area: Area, mut test: impl FnMut(i64, i64) -> bool) -> bool {
    (area.top..area.bottom).any(|y| (area.left..area.right).any(|x| test(x, y)))
}

/// `collidesWith(Sprite, pixelLevel)`: both sprites must be visible.
/// `pixels` holds the two sprites' images for a pixel-level test and is
/// `None` for a test of the collision rectangles alone.
pub fn sprites(a: &Footprint, b: &Footprint, pixels: Option<(&Surface, &Surface)>) -> bool {
    if !(a.visible && b.visible && a.collision.overlaps(&b.collision)) {
        return false;
    }
    let Some((a_image, b_image)) = pixels else {
        return true;
    };
    let area = a.pixel_area().intersect(b.pixel_area());
    any_pixel(area, |x, y| {
        a.is_opaque_at(a_image, x, y) && b.is_opaque_at(b_image, x, y)
    })
}

/// `collidesWith(TiledLayer, pixelLevel)`: both must be visible. By
/// rectangle any cell that is not 0 collides, even an animated tile that
/// shows nothing now; pixel by pixel only the opaque pixels of the tiles
/// the cells show. `pixels` holds the sprite's image and the layer's.
pub fn sprite_and_tiles(
    a: &Footprint,
    t: &TiledLayer,
    pixels: Option<(&Surface, &Surface)>,
) -> bool {
    let layer = t.bounds();
    if !(a.visible && t.base.visible && a.collision.overlaps(&layer)) {
        return false;
    }
    let Some((a_image, t_image)) = pixels else {
        let area = a.collision.intersect(layer);
        let (columns, rows) = t.cells_in(area);
        return rows
            .flat_map(|row| columns.clone().map(move |column| (column, row)))
            .any(|(column, row)| t.cell_value(column, row) != 0);
    };
    let area = a.pixel_area().intersect(layer);
    let (cw, ch) = (i64::from(t.cell_width()), i64::from(t.cell_height()));
    any_pixel(area, |x, y| {
        let (u, v) = (x - layer.left, y - layer.top);
        let tile = t.tile_at((u / cw) as i32, (v / ch) as i32);
        if tile == 0 || !a.is_opaque_at(a_image, x, y) {
            return false;
        }
        let source = t.tile_region(tile);
        is_opaque(pixel_at(t_image, source.left + u % cw, source.top + v % ch))
    })
}

/// `collidesWith(Image, x, y, pixelLevel)`: the image's top left corner
/// at `(x, y)`; only the sprite need be visible. `pixels` holds the
/// sprite's image for a pixel-level test.
pub fn sprite_and_image(
    a: &Footprint,
    image: &Surface,
    x: i32,
    y: i32,
    pixels: Option<&Surface>,
) -> bool {
    let (x, y) = (i64::from(x), i64::from(y));
    let other = Area::new(x, y, image.width().into(), image.height().into());
    if !(a.visible && a.collision.overlaps(&other)) {
        return false;
    }
    let Some(a_image) = pixels else {
        return true;
    };
    let area = a.pixel_area().intersect(other);
    any_pixel(area, |px, py| {
        a.is_opaque_at(a_image, px, py) && is_opaque(pixel_at(image, px - x, py - y))
    })
}

#[cfg(test)]
mod tests {
    use fastemu_gfx::SurfaceKind;

    use super::super::sprite::Sprite;
    use super::*;

    const CLEAR: u32 = 0x0000_0000;
    const SOLID: u32 = 0xFF00_00FF;
    const HALF: u32 = 0x80FF_0000;

    /// An image drawn from rows of `#` (opaque), `+` (half transparent)
    /// and `.` (transparent).
    fn image(rows: &[&str]) -> Surface {
        let pixels: Vec<u32> = rows
            .iter()
            .flat_map(|row| {
                row.chars().map(|c| match c {
                    '#' => SOLID,
                    '+' => HALF,
                    _ => CLEAR,
                })
            })
            .collect();
        let width = rows[0].len() as u32;
        Surface::from_pixels(width, rows.len() as u32, pixels, SurfaceKind::Immutable).unwrap()
    }

    fn sprite_at(img: &Surface, x: i32, y: i32) -> Sprite {
        let mut s = Sprite::new(
            img.width(),
            img.height(),
            img.width() as i32,
            img.height() as i32,
        )
        .unwrap();
        s.base.x = x;
        s.base.y = y;
        s
    }

    #[test]
    fn rectangles_collide_when_their_insides_overlap() {
        let img = image(&["##", "##"]);
        let a = sprite_at(&img, 0, 0);
        let mut b = sprite_at(&img, 1, 1);
        assert!(sprites(&a.footprint(), &b.footprint(), None));
        b.base.x = 2;
        assert!(!sprites(&a.footprint(), &b.footprint(), None));
        b.base.x = 1;
        b.base.visible = false;
        assert!(!sprites(&a.footprint(), &b.footprint(), None));
    }

    #[test]
    fn transparent_pixels_never_collide() {
        // Opaque only on the diagonal; partly transparent elsewhere.
        let img = image(&["#+", "+#"]);
        let a = sprite_at(&img, 0, 0);
        let mut b = sprite_at(&img, 1, 0);
        // b's (0, 0) lands on a's (1, 0), which is only half opaque.
        assert!(sprites(&a.footprint(), &b.footprint(), None));
        assert!(!sprites(&a.footprint(), &b.footprint(), Some((&img, &img))));
        b.base.y = -1;
        // b's (0, 1) lands on a's (1, 0): neither is opaque. b's (1, 1)
        // would be on a's (2, 0), outside a.
        assert!(!sprites(&a.footprint(), &b.footprint(), Some((&img, &img))));
        b.base.x = 0;
        b.base.y = 0;
        assert!(sprites(&a.footprint(), &b.footprint(), Some((&img, &img))));
    }

    #[test]
    fn pixel_tests_use_the_transformed_frame() {
        // An L: opaque down the left side and along the bottom.
        let l = image(&["#..", "#..", "###"]);
        let dot = image(&["#"]);
        let mut a = sprite_at(&l, 0, 0);
        // The middle pixel as reference keeps the frame where it is.
        a.define_reference_pixel(1, 1);
        let hits = |a: &Sprite| -> Vec<(i32, i32)> {
            (0..3)
                .flat_map(|y| (0..3).map(move |x| (x, y)))
                .filter(|&(x, y)| sprite_and_image(&a.footprint(), &dot, x, y, Some(&l)))
                .collect()
        };
        assert_eq!(hits(&a), [(0, 0), (0, 1), (0, 2), (1, 2), (2, 2)]);
        a.set_transform(Transform::Mirror);
        assert_eq!(hits(&a), [(2, 0), (2, 1), (0, 2), (1, 2), (2, 2)]);
        a.set_transform(Transform::Rot90);
        // Turned clockwise: the bottom row goes to the left column, the
        // left column to the top row.
        assert_eq!(hits(&a), [(0, 0), (1, 0), (2, 0), (0, 1), (0, 2)]);
        a.set_transform(Transform::Rot270);
        assert_eq!(hits(&a), [(2, 0), (2, 1), (0, 2), (1, 2), (2, 2)]);
        a.set_transform(Transform::MirrorRot270);
        assert_eq!(hits(&a), [(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)]);
        a.set_transform(Transform::MirrorRot90);
        assert_eq!(
            hits(&a),
            [(0, 0), (0, 1), (0, 2), (1, 2), (2, 2)],
            "its own mirror"
        );
    }

    #[test]
    fn pixel_tests_stay_inside_the_collision_rectangle_and_frame() {
        let img = image(&["###", "###", "###"]);
        let dot = image(&["#"]);
        let mut a = sprite_at(&img, 0, 0);
        a.define_collision_rectangle(1, 1, 5, 5).unwrap();
        let fp = a.footprint();
        assert!(
            !sprite_and_image(&fp, &dot, 0, 0, Some(&img)),
            "outside the rectangle"
        );
        assert!(sprite_and_image(&fp, &dot, 2, 2, Some(&img)));
        assert!(
            sprite_and_image(&fp, &dot, 4, 4, None),
            "the rectangle reaches it"
        );
        assert!(
            !sprite_and_image(&fp, &dot, 4, 4, Some(&img)),
            "outside the frame"
        );
        a.define_collision_rectangle(1, 1, 0, 0).unwrap();
        let big = image(&["###", "###", "###"]);
        let fp = a.footprint();
        assert!(
            sprite_and_image(&fp, &big, 0, 0, None),
            "an empty rectangle inside"
        );
        assert!(!sprite_and_image(&fp, &big, 0, 0, Some(&img)));
    }

    #[test]
    fn tiled_layers_collide_by_cell_and_by_tile_pixel() {
        // Two 2 by 2 tiles: tile 1 opaque only at its top left, tile 2
        // opaque everywhere.
        let tiles = image(&["#.##", "..##"]);
        let mut t = TiledLayer::new(3, 2, 4, 2, 2, 2).unwrap();
        t.base.x = 10;
        t.base.y = 20;
        t.set_cell(0, 0, 1).unwrap();
        t.set_cell(2, 1, 2).unwrap();
        let animated = t.create_animated_tile(0).unwrap();
        t.set_cell(1, 0, animated).unwrap();
        let dot = image(&["#"]);
        let hits = |t: &TiledLayer, x: i32, y: i32, pixel: bool| {
            let s = sprite_at(&dot, x, y);
            sprite_and_tiles(&s.footprint(), t, pixel.then_some((&dot, &tiles)))
        };
        assert!(hits(&t, 10, 20, true), "tile 1's opaque pixel");
        assert!(hits(&t, 11, 21, false), "inside a set cell");
        assert!(!hits(&t, 11, 21, true), "tile 1's clear pixel");
        assert!(!hits(&t, 10, 22, false), "an empty cell");
        assert!(
            hits(&t, 12, 20, false),
            "an animated cell counts by rectangle"
        );
        assert!(!hits(&t, 12, 20, true), "but shows nothing");
        t.set_animated_tile(animated, 2).unwrap();
        assert!(hits(&t, 13, 21, true), "it shows tile 2 now");
        assert!(hits(&t, 15, 23, true));
        assert!(!hits(&t, 16, 23, false), "right of the layer");
        t.base.visible = false;
        assert!(!hits(&t, 15, 23, false));
    }

    #[test]
    fn sprites_on_tiles_use_both_masks() {
        let tiles = image(&["##", "##"]);
        let mut t = TiledLayer::new(2, 2, 2, 2, 2, 2).unwrap();
        t.fill_cells(0, 0, 2, 2, 1).unwrap();
        let ring = image(&["###", "#.#", "###"]);
        let mut s = sprite_at(&ring, -1, -1);
        s.define_collision_rectangle(1, 1, 1, 1).unwrap();
        assert!(sprite_and_tiles(&s.footprint(), &t, None));
        assert!(
            !sprite_and_tiles(&s.footprint(), &t, Some((&ring, &tiles))),
            "the hole"
        );
        s.define_collision_rectangle(0, 0, 3, 3).unwrap();
        assert!(sprite_and_tiles(&s.footprint(), &t, Some((&ring, &tiles))));
    }

    #[test]
    fn inverse_undoes_every_transform() {
        for t in Transform::ALL {
            let (w, h) = (3, 5);
            let (tw, th) = t.transformed_size(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (u, v) = t.apply(x, y, w, h);
                    assert_eq!(inverse(t).apply(u, v, tw, th), (x, y), "{t:?}");
                }
            }
        }
    }
}
