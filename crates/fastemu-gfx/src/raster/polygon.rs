//! Filled and outlined polygons: MIDP's `fillTriangle` and Nokia's
//! polygons and triangles.

use super::line::Line;
use super::{Canvas, Clip};

/// What [`polygon`] draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PolygonPart {
    /// The outline: the lines between consecutive points and from the
    /// last back to the first.
    Outline,
    /// The interior and the outline.
    Filled,
}

/// Draws a closed polygon through `points`, each pixel once so
/// translucent colours do not stack.
///
/// A pixel is inside when its top-left corner, the grid point MIDP names
/// it by, lies inside or on the polygon under the even-odd rule. The
/// outline adds exactly the pixels `drawLine` draws for each side, so a
/// fill includes its edges as `fillTriangle` must.
pub(crate) fn polygon(canvas: &mut Canvas<'_>, points: &[(i64, i64)], part: PolygonPart) {
    let Some(&(x, y)) = points.first() else {
        return;
    };
    let (mut x0, mut y0, mut x1, mut y1) = (x, y, x, y);
    for &(px, py) in points {
        x0 = x0.min(px);
        y0 = y0.min(py);
        x1 = x1.max(px);
        y1 = y1.max(py);
    }
    let area = Clip {
        x0,
        y0,
        x1: x1 + 1,
        y1: y1 + 1,
    }
    .intersect(canvas.clip);
    if area.is_empty() {
        return;
    }
    let mut mask = Mask::new(area);
    let edges = || points.iter().zip(points.iter().cycle().skip(1));
    if part == PolygonPart::Filled && points.len() >= 3 {
        let mut crossings = Vec::new();
        for row in area.y0..area.y1 {
            crossings.clear();
            for (&a, &b) in edges() {
                if let Some(crossing) = crossing(a, b, row) {
                    crossings.push(crossing);
                }
            }
            crossings.sort_unstable();
            for pair in crossings.as_chunks::<2>().0 {
                mask.span(row, pair[0].1, pair[1].0);
            }
        }
    }
    for (&(ax, ay), &(bx, by)) in edges() {
        Line::new(ax, ay, bx, by).walk(&area, |px, py, _| mask.set(px, py));
    }
    mask.draw(canvas);
}

/// Where the edge from `a` to `b` crosses row `y`, as the floor and the
/// ceiling of the exact x. Rows are half-open per edge (the top end
/// counts, the bottom end does not), so a vertex on a row is counted once
/// and horizontal edges never.
fn crossing(a: (i64, i64), b: (i64, i64), y: i64) -> Option<(i64, i64)> {
    let (top, bottom) = if a.1 < b.1 { (a, b) } else { (b, a) };
    if top.1 == bottom.1 || y < top.1 || y >= bottom.1 {
        return None;
    }
    let dy = i128::from(bottom.1 - top.1);
    let num = i128::from(top.0) * dy + i128::from(y - top.1) * i128::from(bottom.0 - top.0);
    let floor = num.div_euclid(dy);
    let ceil = -(-num).div_euclid(dy);
    Some((floor as i64, ceil as i64))
}

/// Which pixels of a rectangle a polygon covers.
struct Mask {
    area: Clip,
    width: usize,
    cells: Vec<bool>,
}

impl Mask {
    fn new(area: Clip) -> Self {
        let width = (area.x1 - area.x0) as usize;
        Self {
            area,
            width,
            cells: vec![false; width * (area.y1 - area.y0) as usize],
        }
    }

    fn row(&mut self, y: i64) -> &mut [bool] {
        let start = (y - self.area.y0) as usize * self.width;
        &mut self.cells[start..start + self.width]
    }

    /// Marks `[first, last]` of row `y`, clipped to the area.
    fn span(&mut self, y: i64, first: i64, last: i64) {
        let first = first.max(self.area.x0);
        let last = last.min(self.area.x1 - 1);
        if first <= last {
            let x0 = self.area.x0;
            self.row(y)[(first - x0) as usize..=(last - x0) as usize].fill(true);
        }
    }

    /// Marks one pixel inside the area.
    fn set(&mut self, x: i64, y: i64) {
        let x0 = self.area.x0;
        self.row(y)[(x - x0) as usize] = true;
    }

    /// Fills each marked run with the canvas colour.
    fn draw(mut self, canvas: &mut Canvas<'_>) {
        let x0 = self.area.x0;
        for y in self.area.y0..self.area.y1 {
            let row = self.row(y);
            let mut x = 0;
            while x < row.len() {
                if !row[x] {
                    x += 1;
                    continue;
                }
                let start = x;
                while x < row.len() && row[x] {
                    x += 1;
                }
                canvas.span(y, x0 + start as i64, x0 + x as i64);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::Surface;

    fn draw(w: i32, h: i32, points: &[(i64, i64)], part: PolygonPart) -> Vec<String> {
        let mut surface = Surface::new_mutable(w, h).unwrap();
        let clip = Clip::of_size(w.into(), h.into());
        let mut canvas = Canvas::new(&mut surface, clip, 0xFF00_0000);
        polygon(&mut canvas, points, part);
        surface
            .pixels()
            .chunks(w as usize)
            .map(|row| {
                row.iter()
                    .map(|&p| if p == 0xFF00_0000 { '#' } else { '.' })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn triangles_include_their_edges() {
        let filled = draw(6, 6, &[(0, 0), (4, 0), (0, 4)], PolygonPart::Filled);
        assert_eq!(
            filled,
            ["#####.", "####..", "###...", "##....", "#.....", "......"]
        );
        let outline = draw(6, 6, &[(0, 0), (4, 0), (0, 4)], PolygonPart::Outline);
        assert_eq!(
            outline,
            ["#####.", "#..#..", "#.#...", "##....", "#.....", "......"]
        );
    }

    #[test]
    fn degenerate_polygons_are_lines_and_points() {
        assert_eq!(
            draw(3, 2, &[(1, 1), (1, 1), (1, 1)], PolygonPart::Filled),
            ["...", ".#."]
        );
        assert_eq!(
            draw(4, 1, &[(0, 0), (3, 0), (1, 0)], PolygonPart::Filled),
            ["####"]
        );
        assert_eq!(draw(2, 1, &[], PolygonPart::Filled), [".."]);
    }

    #[test]
    fn concave_polygons_use_even_odd() {
        // A "U": the notch between the arms stays empty.
        let u = [
            (0, 0),
            (2, 0),
            (2, 3),
            (4, 3),
            (4, 0),
            (6, 0),
            (6, 5),
            (0, 5),
        ];
        let rows = draw(7, 6, &u, PolygonPart::Filled);
        assert_eq!(rows[1], "###.###");
        assert_eq!(rows[4], "#######");
    }

    #[test]
    fn shapes_far_outside_cost_nothing_and_clip() {
        let rows = draw(
            3,
            3,
            &[
                (-1_000_000_000, -1_000_000_000),
                (1_000_000_000, -1_000_000_000),
                (0, 1_000_000_000),
            ],
            PolygonPart::Filled,
        );
        assert_eq!(rows, ["###", "###", "###"]);
    }

    #[test]
    fn crossings_are_exact() {
        assert_eq!(crossing((0, 0), (3, 2), 1), Some((1, 2)));
        assert_eq!(crossing((0, 0), (4, 2), 1), Some((2, 2)));
        assert_eq!(crossing((0, 0), (4, 2), 2), None);
        assert_eq!(crossing((0, 0), (-3, 2), 1), Some((-2, -1)));
        assert_eq!(crossing((5, 1), (9, 1), 1), None);
    }
}
