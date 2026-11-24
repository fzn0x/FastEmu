//! Arcs and round rectangles.

use super::{Graphics, StrokeStyle};
use crate::graphics::shapes::draw_rect;
use crate::raster::ellipse::{MAX_OUTLINE, Wedge, ellipse_outline, ellipse_row};
use crate::raster::{Canvas, Clip, dotted_visible};
use crate::surface::Surface;

impl Graphics {
    /// `drawArc(x, y, width, height, startAngle, arcAngle)`: the outline
    /// of the part of the ellipse that covers the `width + 1` by
    /// `height + 1` pixels from `(x, y)`, between the two angles. Nothing
    /// is drawn if either size is negative or `arcAngle` is zero.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_arc(
        &self,
        target: &mut Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        start_angle: i32,
        arc_angle: i32,
    ) {
        if width < 0 || height < 0 {
            return;
        }
        let Some(wedge) = Wedge::new(start_angle, arc_angle) else {
            return;
        };
        let (x0, y0) = self.at(x, y);
        let (w, h) = (i64::from(width), i64::from(height));
        let mut canvas = self.canvas(target);
        if !outline_visible(&canvas.clip, x0, y0, w, h) {
            return;
        }
        // Directions are scaled by the other axis, as `Wedge` expects; a
        // zero size collapses to a line, measured along its own axis.
        let (sx, sy) = (h.max(1) as f64, w.max(1) as f64);
        let (cx2, cy2) = (2 * x0 + w, 2 * y0 + h);
        let clip = canvas.clip;
        let stroke = self.stroke;
        let mut points = Vec::new();
        ellipse_outline(x0, y0, x0 + w, y0 + h, |px, py, step| {
            if !clip.contains(px, py) || (stroke == StrokeStyle::Dotted && !dotted_visible(step)) {
                return;
            }
            let dx = (2 * px - cx2) as f64 * sx;
            let dy = -((2 * py - cy2) as f64) * sy;
            if wedge.contains(dx, dy) {
                points.push((py, px));
            }
        });
        plot_once(&mut canvas, points);
    }

    /// `fillArc(x, y, width, height, startAngle, arcAngle)`: the pie slice
    /// of the ellipse filling the `width` by `height` pixels from
    /// `(x, y)`. A pixel is in it when its centre lies inside the ellipse
    /// and within the angles. Nothing is drawn if either size is zero or
    /// less, or `arcAngle` is zero.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn fill_arc(
        &self,
        target: &mut Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        start_angle: i32,
        arc_angle: i32,
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let Some(wedge) = Wedge::new(start_angle, arc_angle) else {
            return;
        };
        let (x0, y0) = self.at(x, y);
        let (w, h) = (i64::from(width), i64::from(height));
        let mut canvas = self.canvas(target);
        let rows = canvas.clip.y0.max(y0)..canvas.clip.y1.min(y0 + h);
        for py in rows {
            let r = py - y0;
            let Some((c0, c1)) = ellipse_row(w, h, r) else {
                continue;
            };
            if wedge.is_full() {
                canvas.span(py, x0 + c0, x0 + c1 + 1);
                continue;
            }
            let first = (x0 + c0).max(canvas.clip.x0);
            let last = (x0 + c1).min(canvas.clip.x1 - 1);
            let dy = -((2 * r + 1 - h) as f64) * w as f64;
            for px in first..=last {
                let dx = (2 * (px - x0) + 1 - w) as f64 * h as f64;
                if wedge.contains(dx, dy) {
                    canvas.pixel(px, py);
                }
            }
        }
    }

    /// `drawRoundRect(x, y, width, height, arcWidth, arcHeight)`: the
    /// outline of a rectangle `width + 1` by `height + 1` pixels whose
    /// corners are quarters of an `arcWidth` by `arcHeight` ellipse. Arc
    /// sizes are clamped to the rectangle; nothing is drawn if either size
    /// is negative.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_round_rect(
        &self,
        target: &mut Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        arc_width: i32,
        arc_height: i32,
    ) {
        if width < 0 || height < 0 {
            return;
        }
        let (x0, y0) = self.at(x, y);
        let (w, h) = (i64::from(width), i64::from(height));
        let aw = i64::from(arc_width).clamp(0, w);
        let ah = i64::from(arc_height).clamp(0, h);
        let mut canvas = self.canvas(target);
        if aw == 0 || ah == 0 {
            draw_rect(&mut canvas, x0, y0, w, h, self.stroke);
            return;
        }
        if !outline_visible(&canvas.clip, x0, y0, w, h) {
            return;
        }
        let clip = canvas.clip;
        let dotted = self.stroke == StrokeStyle::Dotted;
        let mut points = Vec::new();
        let mut keep = |px: i64, py: i64, k: i64| {
            if clip.contains(px, py) && (!dotted || dotted_visible(k)) {
                points.push((py, px));
            }
        };
        // Each quarter of the corner ellipse goes to its own corner.
        ellipse_outline(0, 0, aw, ah, |px, py, step| {
            for cx in corner_positions(px, aw, x0, w) {
                for cy in corner_positions(py, ah, y0, h) {
                    keep(cx, cy, step);
                }
            }
        });
        // The straight sides join the corners' end points.
        let (left, right) = (x0 + aw / 2, x0 + w - aw + (aw + 1) / 2);
        let (top, bottom) = (y0 + ah / 2, y0 + h - ah + (ah + 1) / 2);
        for px in left.max(clip.x0)..=right.min(clip.x1 - 1) {
            keep(px, y0, px - left);
            keep(px, y0 + h, px - left);
        }
        for py in top.max(clip.y0)..=bottom.min(clip.y1 - 1) {
            keep(x0, py, py - top);
            keep(x0 + w, py, py - top);
        }
        plot_once(&mut canvas, points);
    }

    /// `fillRoundRect(x, y, width, height, arcWidth, arcHeight)`: the
    /// `width` by `height` pixels from `(x, y)` with each corner cut along
    /// a quarter of an `arcWidth` by `arcHeight` ellipse, by pixel centres
    /// as in `fillArc`. Nothing is drawn if either size is zero or less.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn fill_round_rect(
        &self,
        target: &mut Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        arc_width: i32,
        arc_height: i32,
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let (x0, y0) = self.at(x, y);
        let (w, h) = (i64::from(width), i64::from(height));
        let aw = i64::from(arc_width).clamp(0, w);
        let ah = i64::from(arc_height).clamp(0, h);
        let mut canvas = self.canvas(target);
        if aw == 0 || ah == 0 {
            canvas.rect(x0, y0, x0 + w, y0 + h);
            return;
        }
        // The band between the corners is always full height.
        let (band_left, band_right) = (x0 + aw / 2, x0 + w - aw / 2);
        for py in canvas.clip.y0.max(y0)..canvas.clip.y1.min(y0 + h) {
            let r = py - y0;
            let corner_row = if 2 * r + 1 < ah {
                Some(r)
            } else if 2 * (h - 1 - r) + 1 < ah {
                Some(r - (h - ah))
            } else {
                None
            };
            let (left, right) = match corner_row.and_then(|cr| ellipse_row(aw, ah, cr)) {
                Some((c0, c1)) => (
                    (x0 + c0).min(band_left),
                    (x0 + w - aw + c1 + 1).max(band_right),
                ),
                None if corner_row.is_some() => (band_left, band_right),
                None => (x0, x0 + w),
            };
            canvas.span(py, left, right);
        }
    }
}

/// Whether an outline over `[x, x + w] x [y, y + h]` is small enough to
/// walk and reaches the clip at all.
fn outline_visible(clip: &Clip, x: i64, y: i64, w: i64, h: i64) -> bool {
    w <= MAX_OUTLINE
        && h <= MAX_OUTLINE
        && x < clip.x1
        && x + w >= clip.x0
        && y < clip.y1
        && y + h >= clip.y0
}

/// Where position `p` of a corner ellipse `arc` wide lands on a side of
/// `size` starting at `origin`: the near corner takes the first half of
/// the ellipse, the far corner the second, and the middle goes to both.
fn corner_positions(p: i64, arc: i64, origin: i64, size: i64) -> impl Iterator<Item = i64> {
    let near = (2 * p <= arc).then_some(origin + p);
    let far = (2 * p >= arc).then_some(origin + size - arc + p);
    near.into_iter().chain(far)
}

/// Draws each of `points` (as `(y, x)`) once, so translucent colours do not
/// stack where an algorithm emits a pixel twice.
fn plot_once(canvas: &mut Canvas<'_>, mut points: Vec<(i64, i64)>) {
    points.sort_unstable();
    points.dedup();
    for (y, x) in points {
        canvas.pixel(x, y);
    }
}

#[cfg(test)]
mod tests {
    use crate::graphics::{DOTTED, Graphics};
    use crate::surface::Surface;

    fn render(surface: &Surface) -> Vec<String> {
        surface
            .pixels()
            .chunks(surface.width() as usize)
            .map(|row| {
                row.iter()
                    .map(|&p| if p == 0xFF00_0000 { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = Graphics::new(&surface).unwrap();
        (surface, g)
    }

    #[test]
    fn full_arcs_cover_their_boxes() {
        let (mut s, g) = canvas(6, 6);
        g.draw_arc(&mut s, 0, 0, 4, 4, 0, 360);
        assert_eq!(
            render(&s),
            [".###..", "#...#.", "#...#.", "#...#.", ".###..", "......"]
        );
        let (mut s, g) = canvas(5, 5);
        g.fill_arc(&mut s, 0, 0, 4, 4, 0, 360);
        assert_eq!(render(&s), [".##..", "####.", "####.", ".##..", "....."]);
    }

    #[test]
    fn arcs_follow_the_angles() {
        // A quarter from 3 o'clock to 12 o'clock: the upper right.
        let (mut s, g) = canvas(5, 5);
        g.draw_arc(&mut s, 0, 0, 4, 4, 0, 90);
        assert_eq!(render(&s), ["..##.", "....#", "....#", ".....", "....."]);
        // The same quarter, given as 90 degrees clockwise from 12.
        let (mut t, g) = canvas(5, 5);
        g.draw_arc(&mut t, 0, 0, 4, 4, 90, -90);
        assert_eq!(render(&t), render(&s));
        // A half pie on the left.
        let (mut s, g) = canvas(4, 4);
        g.fill_arc(&mut s, 0, 0, 4, 4, 90, 180);
        assert_eq!(render(&s), [".#..", "##..", "##..", ".#.."]);
    }

    #[test]
    fn empty_arcs_draw_nothing() {
        let (mut s, g) = canvas(4, 4);
        g.draw_arc(&mut s, 0, 0, 3, 3, 10, 0);
        g.fill_arc(&mut s, 0, 0, 3, 3, 10, 0);
        g.fill_arc(&mut s, 0, 0, 0, 3, 0, 360);
        g.draw_arc(&mut s, 0, 0, -1, 3, 0, 360);
        assert!(render(&s).iter().all(|r| !r.contains('#')));
        g.draw_arc(&mut s, 1, 1, 0, 0, 0, 360);
        assert_eq!(render(&s)[1], ".#..");
    }

    #[test]
    fn flat_arcs_are_lines() {
        let (mut s, g) = canvas(5, 3);
        g.draw_arc(&mut s, 0, 1, 4, 0, 0, 360);
        assert_eq!(render(&s), [".....", "#####", "....."]);
        let (mut s, g) = canvas(5, 3);
        g.draw_arc(&mut s, 0, 1, 4, 0, 0, 90);
        assert_eq!(render(&s), [".....", "..###", "....."]);
    }

    #[test]
    fn round_rects_round_their_corners() {
        let (mut s, g) = canvas(8, 6);
        g.draw_round_rect(&mut s, 0, 0, 6, 4, 2, 2);
        assert_eq!(
            render(&s),
            [
                ".#####..", "#.....#.", "#.....#.", "#.....#.", ".#####..", "........"
            ]
        );
        let (mut s, g) = canvas(8, 5);
        g.fill_round_rect(&mut s, 0, 0, 6, 4, 4, 4);
        assert_eq!(
            render(&s),
            [".####...", "######..", "######..", ".####...", "........"]
        );
    }

    #[test]
    fn round_rects_without_arcs_are_rects() {
        let (mut a, g) = canvas(6, 6);
        g.draw_round_rect(&mut a, 1, 1, 3, 3, 0, 5);
        let (mut b, _) = canvas(6, 6);
        g.draw_rect(&mut b, 1, 1, 3, 3);
        assert_eq!(a, b);
        let (mut a, _) = canvas(6, 6);
        g.fill_round_rect(&mut a, 1, 1, 3, 3, -4, 2);
        let (mut b, _) = canvas(6, 6);
        g.fill_rect(&mut b, 1, 1, 3, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn oversized_arcs_are_clamped() {
        let (mut a, g) = canvas(6, 6);
        g.fill_round_rect(&mut a, 0, 0, 5, 5, 50, 50);
        let (mut b, _) = canvas(6, 6);
        g.fill_arc(&mut b, 0, 0, 5, 5, 0, 360);
        assert_eq!(a, b);
    }

    #[test]
    fn huge_arcs_are_fast_and_clipped() {
        let (mut s, g) = canvas(4, 4);
        g.fill_arc(
            &mut s,
            -1_000_000_000,
            -1_000_000_000,
            2_000_000_000,
            2_000_000_000,
            0,
            360,
        );
        assert!(render(&s).iter().all(|r| r == "####"));
        g.draw_arc(&mut s, -1_000_000_000, 0, 2_000_000_000, 10, 0, 360);
    }

    #[test]
    fn dotted_arcs_draw_fewer_pixels() {
        let (mut solid, mut g) = canvas(12, 12);
        g.draw_arc(&mut solid, 0, 0, 10, 10, 0, 360);
        let (mut dotted, _) = canvas(12, 12);
        g.set_stroke_style(DOTTED).unwrap();
        g.draw_arc(&mut dotted, 0, 0, 10, 10, 0, 360);
        let count = |s: &Surface| s.pixels().iter().filter(|&&p| p == 0xFF00_0000).count();
        assert!(count(&dotted) > 0 && count(&dotted) < count(&solid));
        assert!(
            dotted
                .pixels()
                .iter()
                .zip(solid.pixels())
                .all(|(d, s)| *d != 0xFF00_0000 || d == s)
        );
    }
}
