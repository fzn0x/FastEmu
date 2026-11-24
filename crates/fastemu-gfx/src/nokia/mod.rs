//! Nokia's `com.nokia.mid.ui.DirectGraphics`: polygons and triangles in
//! an ARGB colour of their own, raw pixels in Nokia's formats, and images
//! flipped and rotated.
//!
//! `DirectGraphics` shares its state with the `Graphics` it came from, so
//! its calls are methods of [`Graphics`]; `setARGBColor` and
//! `getAlphaComponent` are [`Graphics::set_argb_color`] and
//! [`Graphics::alpha_component`]. Constant values and behaviour follow
//! Nokia's UI API 1.1 documentation and J2ME Loader's implementation
//! (`app/src/main/java/com/nokia/mid/ui/`).

mod pixels;

use crate::error::{GfxError, Result};
use crate::graphics::Graphics;
use crate::raster::polygon::{PolygonPart, polygon};
use crate::surface::{Surface, java_range};
use crate::transform::Transform;

/// `FLIP_HORIZONTAL`: mirror left to right.
pub const FLIP_HORIZONTAL: i32 = 0x2000;
/// `FLIP_VERTICAL`: mirror top to bottom.
pub const FLIP_VERTICAL: i32 = 0x4000;
/// `ROTATE_90`: a quarter turn counter-clockwise.
pub const ROTATE_90: i32 = 90;
/// `ROTATE_180`: a half turn.
pub const ROTATE_180: i32 = 180;
/// `ROTATE_270`: three quarter turns counter-clockwise.
pub const ROTATE_270: i32 = 270;

/// `TYPE_BYTE_1_GRAY`: one bit per pixel, eight to a byte, left pixel in
/// the high bit; a set bit is black.
pub const TYPE_BYTE_1_GRAY: i32 = 1;
/// `TYPE_BYTE_1_GRAY_VERTICAL`: one bit per pixel, eight pixels of a
/// column to a byte, top pixel in the low bit.
pub const TYPE_BYTE_1_GRAY_VERTICAL: i32 = -1;
/// `TYPE_BYTE_2_GRAY`: four grey levels, four pixels to a byte.
pub const TYPE_BYTE_2_GRAY: i32 = 2;
/// `TYPE_BYTE_4_GRAY`: sixteen grey levels, two pixels to a byte.
pub const TYPE_BYTE_4_GRAY: i32 = 4;
/// `TYPE_BYTE_8_GRAY`: 256 grey levels, a byte per pixel.
pub const TYPE_BYTE_8_GRAY: i32 = 8;
/// `TYPE_BYTE_332_RGB`: `RRRGGGBB` in a byte.
pub const TYPE_BYTE_332_RGB: i32 = 332;
/// `TYPE_USHORT_4444_ARGB`: `0xARGB` in a short.
pub const TYPE_USHORT_4444_ARGB: i32 = 4444;
/// `TYPE_USHORT_444_RGB`: `0x0RGB` in a short.
pub const TYPE_USHORT_444_RGB: i32 = 444;
/// `TYPE_USHORT_555_RGB`: `0RRRRRGGGGGBBBBB` in a short.
pub const TYPE_USHORT_555_RGB: i32 = 555;
/// `TYPE_USHORT_1555_ARGB`: a top alpha bit over 5-5-5.
pub const TYPE_USHORT_1555_ARGB: i32 = 1555;
/// `TYPE_USHORT_565_RGB`: `RRRRRGGGGGGBBBBB` in a short.
pub const TYPE_USHORT_565_RGB: i32 = 565;
/// `TYPE_INT_888_RGB`: `0x00RRGGBB`, the top byte ignored.
pub const TYPE_INT_888_RGB: i32 = 888;
/// `TYPE_INT_8888_ARGB`: `0xAARRGGBB`.
pub const TYPE_INT_8888_ARGB: i32 = 8888;

/// What `getNativePixelFormat()` reports. Games pick their `drawPixels`
/// path from it; 5-6-5 is what J2ME Loader reports by default and suits
/// the games written for colour Nokia phones. Every format draws, so the
/// wrapper may report another.
pub const NATIVE_PIXEL_FORMAT: i32 = TYPE_USHORT_565_RGB;

/// The sprite transform for a Nokia manipulation: `FLIP_HORIZONTAL`
/// and `FLIP_VERTICAL` combined with at most one of `ROTATE_90`,
/// `ROTATE_180` and `ROTATE_270`. Nokia applies the counter-clockwise
/// rotation first, then the vertical flip, then the horizontal one. Any
/// other value is `IllegalArgumentException`.
pub fn manipulation_transform(manipulation: i32) -> Result<Transform> {
    use Transform as T;
    // Rows: no flip, horizontal, vertical, both. Columns: rotations by 0,
    // 90, 180 and 270 degrees counter-clockwise. The same table as J2ME
    // Loader's DirectGraphicsImp.
    const TABLE: [[Transform; 4]; 4] = [
        [T::None, T::Rot270, T::Rot180, T::Rot90],
        [T::Mirror, T::MirrorRot90, T::MirrorRot180, T::MirrorRot270],
        [T::MirrorRot180, T::MirrorRot270, T::Mirror, T::MirrorRot90],
        [T::Rot180, T::Rot90, T::None, T::Rot270],
    ];
    let flips = (manipulation as u32 >> 13) as usize;
    let rotation = manipulation & 0x1FFF;
    if flips > 3 || rotation % 90 != 0 || rotation > 270 {
        return Err(GfxError::IllegalArgument("unsupported manipulation"));
    }
    Ok(TABLE[flips][(rotation / 90) as usize])
}

impl Graphics {
    /// `fillPolygon(xPoints, xOffset, yPoints, yOffset, nPoints,
    /// argbColor)`: the polygon closed from its last point to its first,
    /// filled by the even-odd rule with its sides included, in
    /// `argb_color`. The `Graphics` colour does not change.
    ///
    /// Throws `ArrayIndexOutOfBoundsException` when the points reach
    /// outside either array; draws nothing for `n_points` of zero or less.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn fill_polygon(
        &self,
        target: &mut Surface,
        x_points: &[i32],
        x_offset: i32,
        y_points: &[i32],
        y_offset: i32,
        n_points: i32,
        argb_color: i32,
    ) -> Result<()> {
        let points = self.polygon_points(x_points, x_offset, y_points, y_offset, n_points)?;
        let mut canvas = self.canvas_with(target, argb_color as u32);
        polygon(&mut canvas, &points, PolygonPart::Filled);
        Ok(())
    }

    /// `drawPolygon(xPoints, xOffset, yPoints, yOffset, nPoints,
    /// argbColor)`: the closed outline, each pixel drawn once. Checks as
    /// [`Graphics::fill_polygon`].
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_polygon(
        &self,
        target: &mut Surface,
        x_points: &[i32],
        x_offset: i32,
        y_points: &[i32],
        y_offset: i32,
        n_points: i32,
        argb_color: i32,
    ) -> Result<()> {
        let points = self.polygon_points(x_points, x_offset, y_points, y_offset, n_points)?;
        let mut canvas = self.canvas_with(target, argb_color as u32);
        polygon(&mut canvas, &points, PolygonPart::Outline);
        Ok(())
    }

    /// `DirectGraphics.fillTriangle(x1, y1, x2, y2, x3, y3, argbColor)`:
    /// MIDP's `fillTriangle` in `argb_color`.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn fill_triangle_argb(
        &self,
        target: &mut Surface,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        argb_color: i32,
    ) {
        let corners = [self.at(x1, y1), self.at(x2, y2), self.at(x3, y3)];
        let mut canvas = self.canvas_with(target, argb_color as u32);
        polygon(&mut canvas, &corners, PolygonPart::Filled);
    }

    /// `DirectGraphics.drawTriangle(x1, y1, x2, y2, x3, y3, argbColor)`:
    /// the three sides in `argb_color`, each pixel drawn once.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_triangle_argb(
        &self,
        target: &mut Surface,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        argb_color: i32,
    ) {
        let corners = [self.at(x1, y1), self.at(x2, y2), self.at(x3, y3)];
        let mut canvas = self.canvas_with(target, argb_color as u32);
        polygon(&mut canvas, &corners, PolygonPart::Outline);
    }

    /// The translated points of a Nokia polygon, checked against the
    /// arrays.
    fn polygon_points(
        &self,
        x_points: &[i32],
        x_offset: i32,
        y_points: &[i32],
        y_offset: i32,
        n_points: i32,
    ) -> Result<Vec<(i64, i64)>> {
        if n_points <= 0 {
            return Ok(Vec::new());
        }
        let xs = java_range(x_points.len(), x_offset, n_points)
            .ok_or(GfxError::ArrayIndexOutOfBounds)?;
        let ys = java_range(y_points.len(), y_offset, n_points)
            .ok_or(GfxError::ArrayIndexOutOfBounds)?;
        Ok(x_points[xs]
            .iter()
            .zip(&y_points[ys])
            .map(|(&x, &y)| self.at(x, y))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = crate::testing::graphics(&surface);
        (surface, g)
    }

    fn render(surface: &Surface) -> Vec<String> {
        surface
            .pixels()
            .chunks(surface.width() as usize)
            .map(|row| {
                row.iter()
                    .map(|&p| if p == 0xFFFF_FFFF { '.' } else { '#' })
                    .collect()
            })
            .collect()
    }

    /// Nokia's order, step by step: rotate counter-clockwise, flip
    /// vertically, flip horizontally.
    fn by_hand(manipulation: i32, x: i64, y: i64, w: i64, h: i64) -> (i64, i64) {
        let (mut x, mut y, mut w, mut h) = (x, y, w, h);
        for _ in 0..(manipulation & 0x1FFF) / 90 {
            (x, y, w, h) = (y, w - 1 - x, h, w);
        }
        if manipulation & FLIP_VERTICAL != 0 {
            y = h - 1 - y;
        }
        if manipulation & FLIP_HORIZONTAL != 0 {
            x = w - 1 - x;
        }
        (x, y)
    }

    #[test]
    fn manipulations_match_nokias_order() {
        for flips in [
            0,
            FLIP_HORIZONTAL,
            FLIP_VERTICAL,
            FLIP_HORIZONTAL | FLIP_VERTICAL,
        ] {
            for rotation in [0, ROTATE_90, ROTATE_180, ROTATE_270] {
                let m = flips | rotation;
                let t = manipulation_transform(m).unwrap();
                for y in 0..2 {
                    for x in 0..3 {
                        assert_eq!(t.apply(x, y, 3, 2), by_hand(m, x, y, 3, 2), "{m:#x}");
                    }
                }
            }
        }
        for bad in [45, 360, 0x8000, -1, FLIP_VERTICAL + 91] {
            assert!(manipulation_transform(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn nokia_constants_match_the_documentation() {
        assert_eq!((FLIP_HORIZONTAL, FLIP_VERTICAL), (8192, 16384));
        assert_eq!((ROTATE_90, ROTATE_180, ROTATE_270), (90, 180, 270));
        assert_eq!(
            [
                TYPE_BYTE_1_GRAY,
                TYPE_BYTE_1_GRAY_VERTICAL,
                TYPE_BYTE_2_GRAY,
                TYPE_BYTE_4_GRAY,
                TYPE_BYTE_8_GRAY
            ],
            [1, -1, 2, 4, 8]
        );
        assert_eq!(
            [
                TYPE_BYTE_332_RGB,
                TYPE_USHORT_4444_ARGB,
                TYPE_USHORT_444_RGB,
                TYPE_USHORT_555_RGB
            ],
            [332, 4444, 444, 555]
        );
        assert_eq!(
            [
                TYPE_USHORT_1555_ARGB,
                TYPE_USHORT_565_RGB,
                TYPE_INT_888_RGB,
                TYPE_INT_8888_ARGB
            ],
            [1555, 565, 888, 8888]
        );
    }

    #[test]
    fn polygons_read_points_from_offsets() {
        let (mut s, g) = canvas(5, 4);
        let xs = [9, 0, 4, 4, 0];
        let ys = [0, 0, 0, 3, 3, 9];
        g.fill_polygon(&mut s, &xs, 1, &ys, 1, 4, 0xFF00_0000_u32 as i32)
            .unwrap();
        assert_eq!(render(&s), ["#####", "#####", "#####", "#####"]);
        let (mut s, g) = canvas(5, 4);
        g.draw_polygon(&mut s, &xs, 1, &ys, 1, 4, 0xFF00_0000_u32 as i32)
            .unwrap();
        assert_eq!(render(&s), ["#####", "#...#", "#...#", "#####"]);
    }

    #[test]
    fn polygon_points_are_checked() {
        let (mut s, g) = canvas(2, 2);
        let oob = Err(GfxError::ArrayIndexOutOfBounds);
        assert_eq!(g.fill_polygon(&mut s, &[0, 1], 1, &[0, 1], 0, 2, -1), oob);
        assert_eq!(g.draw_polygon(&mut s, &[0, 1], 0, &[0, 1], -1, 2, -1), oob);
        assert_eq!(g.fill_polygon(&mut s, &[], 5, &[], 5, 0, -1), Ok(()));
    }

    #[test]
    fn argb_colours_blend_once_and_leave_the_graphics_colour() {
        let (mut s, mut g) = canvas(3, 3);
        g.set_color(0x12_3456);
        g.fill_triangle_argb(&mut s, 0, 0, 2, 0, 0, 2, 0x8000_0000_u32 as i32);
        assert_eq!(s.pixel(0, 0), Some(0xFF7F_7F7F));
        assert_eq!(s.pixel(1, 1), Some(0xFF7F_7F7F));
        assert_eq!(s.pixel(2, 2), Some(0xFFFF_FFFF));
        g.draw_triangle_argb(&mut s, 0, 2, 2, 2, 2, 0, 0x0000_0000);
        assert_eq!(s.pixel(2, 2), Some(0xFFFF_FFFF));
        assert_eq!(g.color(), 0x12_3456);
        g.translate(1, 1);
        g.draw_triangle_argb(&mut s, 1, 1, 1, 1, 1, 1, 0xFF00_00FF_u32 as i32);
        assert_eq!(s.pixel(2, 2), Some(0xFF00_00FF));
    }
}
