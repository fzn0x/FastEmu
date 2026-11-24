//! Lines, rectangles and triangles.

use super::{Graphics, StrokeStyle};
use crate::raster::line::Line;
use crate::raster::polygon::{PolygonPart, polygon};
use crate::raster::{Canvas, dotted_visible};
use crate::surface::Surface;

impl Graphics {
    /// `drawLine(x1, y1, x2, y2)`: both end points are drawn.
    pub fn draw_line(&self, target: &mut Surface, x1: i32, y1: i32, x2: i32, y2: i32) {
        let (ax, ay) = self.at(x1, y1);
        let (bx, by) = self.at(x2, y2);
        let mut canvas = self.canvas(target);
        draw_line(&mut canvas, Line::new(ax, ay, bx, by), self.stroke);
    }

    /// `fillRect(x, y, width, height)`: exactly `width` by `height` pixels;
    /// nothing if either is zero or less.
    pub fn fill_rect(&self, target: &mut Surface, x: i32, y: i32, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        let (x0, y0) = self.at(x, y);
        let mut canvas = self.canvas(target);
        canvas.rect(x0, y0, x0 + i64::from(width), y0 + i64::from(height));
    }

    /// `drawRect(x, y, width, height)`: the outline of a rectangle
    /// `width + 1` by `height + 1` pixels; nothing if either is negative.
    pub fn draw_rect(&self, target: &mut Surface, x: i32, y: i32, width: i32, height: i32) {
        if width < 0 || height < 0 {
            return;
        }
        let (x0, y0) = self.at(x, y);
        let mut canvas = self.canvas(target);
        draw_rect(
            &mut canvas,
            x0,
            y0,
            width.into(),
            height.into(),
            self.stroke,
        );
    }

    /// `fillTriangle(x1, y1, x2, y2, x3, y3)`: the pixels whose top-left
    /// corner lies inside or on the triangle, plus the lines between its
    /// corners as `drawLine` draws them.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn fill_triangle(
        &self,
        target: &mut Surface,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
    ) {
        let corners = [self.at(x1, y1), self.at(x2, y2), self.at(x3, y3)];
        polygon(&mut self.canvas(target), &corners, PolygonPart::Filled);
    }
}

/// Draws a line in a stroke style.
pub(crate) fn draw_line(canvas: &mut Canvas<'_>, line: Line, stroke: StrokeStyle) {
    let clip = canvas.clip;
    line.walk(&clip, |x, y, i| {
        if stroke == StrokeStyle::Solid || dotted_visible(i) {
            canvas.pixel(x, y);
        }
    });
}

/// Draws the outline of the rectangle from `(x, y)` to `(x + w, y + h)`
/// inclusive. Every pixel is drawn once, so translucent colours do not
/// darken the corners.
pub(crate) fn draw_rect(
    canvas: &mut Canvas<'_>,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
    stroke: StrokeStyle,
) {
    if w == 0 || h == 0 {
        draw_line(canvas, Line::new(x, y, x + w, y + h), stroke);
        return;
    }
    if stroke == StrokeStyle::Solid {
        canvas.span(y, x, x + w + 1);
        canvas.span(y + h, x, x + w + 1);
        canvas.rect(x, y + 1, x + 1, y + h);
        canvas.rect(x + w, y + 1, x + w + 1, y + h);
        return;
    }
    // Dotted: number the perimeter clockwise from the top-left corner so
    // the pattern runs on around the corners.
    let sides = [
        (x, y, 1, 0, w),
        (x + w, y, 0, 1, h),
        (x + w, y + h, -1, 0, w),
        (x, y + h, 0, -1, h),
    ];
    let mut k0 = 0;
    for (sx, sy, dx, dy, len) in sides {
        let line = Line::new(sx, sy, sx + dx * (len - 1), sy + dy * (len - 1));
        let clip = canvas.clip;
        line.walk(&clip, |px, py, _| {
            // The line may walk either way; number pixels by distance from
            // the side's own start.
            let k = k0 + (px - sx).abs() + (py - sy).abs();
            if dotted_visible(k) {
                canvas.pixel(px, py);
            }
        });
        k0 += len;
    }
}

#[cfg(test)]
mod tests {
    use crate::graphics::{DOTTED, Graphics};
    use crate::surface::Surface;

    const W: u32 = 0xFFFF_FFFF;
    const B: u32 = 0xFF00_0000;

    /// The surface as rows of '#' (black) and '.' (anything else).
    fn render(surface: &Surface) -> Vec<String> {
        surface
            .pixels()
            .chunks(surface.width() as usize)
            .map(|row| {
                row.iter()
                    .map(|&p| if p == B { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = crate::testing::graphics(&surface);
        (surface, g)
    }

    #[test]
    fn fill_rect_covers_width_by_height() {
        let (mut s, g) = canvas(5, 4);
        g.fill_rect(&mut s, 1, 1, 3, 2);
        assert_eq!(render(&s), [".....", ".###.", ".###.", "....."]);
        let (mut s, g) = canvas(3, 3);
        g.fill_rect(&mut s, 0, 0, 0, 3);
        g.fill_rect(&mut s, 0, 0, 3, -1);
        assert!(s.pixels().iter().all(|&p| p == W));
    }

    #[test]
    fn draw_rect_covers_one_more_each_way() {
        let (mut s, g) = canvas(6, 5);
        g.draw_rect(&mut s, 1, 1, 3, 2);
        assert_eq!(
            render(&s),
            ["......", ".####.", ".#..#.", ".####.", "......"]
        );
        let (mut s, g) = canvas(3, 3);
        g.draw_rect(&mut s, 1, 1, 0, 0);
        assert_eq!(render(&s), ["...", ".#.", "..."]);
        g.draw_rect(&mut s, 0, 0, -1, 2);
        assert_eq!(render(&s), ["...", ".#.", "..."]);
        g.draw_rect(&mut s, 0, 2, 2, 0);
        assert_eq!(render(&s), ["...", ".#.", "###"]);
    }

    #[test]
    fn draw_line_draws_both_ends_and_clips() {
        let (mut s, mut g) = canvas(5, 3);
        g.draw_line(&mut s, 0, 0, 4, 2);
        assert_eq!(render(&s), ["#....", ".##..", "...##"]);
        let (mut s, _) = canvas(5, 3);
        g.set_clip(1, 0, 2, 3);
        g.draw_line(&mut s, 4, 2, 0, 0);
        assert_eq!(render(&s), [".....", ".##..", "....."]);
    }

    #[test]
    fn translation_moves_primitives() {
        let (mut s, mut g) = canvas(4, 4);
        g.translate(2, 1);
        g.fill_rect(&mut s, 0, 0, 1, 1);
        g.draw_line(&mut s, -2, 2, -1, 2);
        assert_eq!(render(&s), ["....", "..#.", "....", "##.."]);
    }

    #[test]
    fn dotted_lines_and_rects_skip_every_other_pixel() {
        let (mut s, mut g) = canvas(7, 4);
        g.set_stroke_style(DOTTED).unwrap();
        g.draw_line(&mut s, 0, 0, 6, 0);
        g.draw_rect(&mut s, 0, 1, 6, 2);
        assert_eq!(render(&s), ["#.#.#.#", "#.#.#.#", ".......", "#.#.#.#"]);
    }

    #[test]
    fn fill_triangle_includes_the_drawn_sides() {
        let (mut filled, mut g) = canvas(6, 5);
        g.translate(1, 0);
        g.fill_triangle(&mut filled, 3, 0, -1, 2, 3, 4);
        assert_eq!(
            render(&filled),
            ["...##.", ".####.", "#####.", ".####.", "...##."]
        );
        let (mut lines, _) = canvas(6, 5);
        g.draw_line(&mut lines, 3, 0, -1, 2);
        g.draw_line(&mut lines, -1, 2, 3, 4);
        g.draw_line(&mut lines, 3, 4, 3, 0);
        for (f, l) in filled.pixels().iter().zip(lines.pixels()) {
            assert!(*l != B || *f == B, "every line pixel is filled");
        }
    }

    #[test]
    fn translucent_outlines_blend_each_pixel_once() {
        let (mut s, mut g) = canvas(3, 3);
        g.set_argb_color(0x8000_0000_u32 as i32);
        g.draw_rect(&mut s, 0, 0, 2, 2);
        let grey = 0xFF7F_7F7F;
        assert_eq!(
            s.pixels(),
            &[grey, grey, grey, grey, W, grey, grey, grey, grey]
        );
    }
}
