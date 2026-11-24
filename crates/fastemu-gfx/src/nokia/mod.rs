//! Nokia's `com.nokia.mid.ui.DirectGraphics`: polygons and triangles in
//! an ARGB colour of their own.
//!
//! `DirectGraphics` shares its state with the `Graphics` it came from, so
//! its calls are methods of [`Graphics`]; `setARGBColor` and
//! `getAlphaComponent` are [`Graphics::set_argb_color`] and
//! [`Graphics::alpha_component`]. Values and behaviour follow Nokia's UI
//! API 1.1 documentation and J2ME Loader's implementation
//! (`app/src/main/java/com/nokia/mid/ui/`).

use crate::error::{GfxError, Result};
use crate::graphics::Graphics;
use crate::raster::polygon::{PolygonPart, polygon};
use crate::surface::{Surface, java_range};

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
    use crate::error::GfxError;
    use crate::graphics::Graphics;
    use crate::surface::Surface;

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = Graphics::new(&surface).unwrap();
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
