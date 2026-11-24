//! Ellipses for arcs and round rectangles: outlines through pixel corners,
//! areas by pixel centres, and the angular wedge of a partial arc.

/// Outlines wider or taller than this are not drawn: walking one costs
/// time in proportion to its size, and no screen shows enough of it to
/// matter.
pub(crate) const MAX_OUTLINE: i64 = 1 << 16;

/// Calls `emit(x, y, step)` for each pixel of the outline of the ellipse
/// that just fits the pixels `[x0, x1] x [y0, y1]` (inclusive), touching
/// all four sides. `step` counts along each quarter from the left and
/// right ends towards the top and bottom; the four quarters share it.
/// Pixels where quarters meet are emitted more than once.
///
/// This is the integer midpoint algorithm for an ellipse in a rectangle
/// from Alois Zingl's "A Rasterizing Algorithm for Drawing Curves" (2012),
/// with 128-bit error terms so any `i32` size is exact.
pub(crate) fn ellipse_outline(
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    mut emit: impl FnMut(i64, i64, i64),
) {
    let a = (x1 - x0).abs();
    let b = (y1 - y0).abs();
    let b_odd = b & 1;
    let (a2, b2) = (i128::from(a) * i128::from(a), i128::from(b) * i128::from(b));
    let mut dx = 4 * (1 - i128::from(a)) * b2;
    let mut dy = 4 * (i128::from(b_odd) + 1) * a2;
    let mut err = dx + dy + i128::from(b_odd) * a2;
    let mut left = x0.min(x1);
    let mut right = left + a;
    let mut lower = y0.min(y1) + (b + 1) / 2;
    let mut upper = lower - b_odd;
    let mut step = 0;
    loop {
        emit(right, lower, step);
        emit(left, lower, step);
        emit(left, upper, step);
        emit(right, upper, step);
        let e2 = 2 * err;
        if e2 <= dy {
            lower += 1;
            upper -= 1;
            dy += 8 * a2;
            err += dy;
        }
        if e2 >= dx || 2 * err > dy {
            left += 1;
            right -= 1;
            dx += 8 * b2;
            err += dx;
        }
        step += 1;
        if left > right {
            break;
        }
    }
    // Very flat ellipses stop early; finish their tips.
    while lower - upper <= b {
        emit(left - 1, lower, step);
        emit(right + 1, lower, step);
        lower += 1;
        emit(left - 1, upper, step);
        emit(right + 1, upper, step);
        upper -= 1;
        step += 1;
    }
}

/// The first and last pixel columns that row `row` covers in the ellipse
/// filling a `width` by `height` box, counting pixels whose centre lies
/// inside or on the ellipse. `None` when the row covers nothing.
///
/// In doubled coordinates about the centre, pixel `(c, r)` has its centre
/// at `X = 2c + 1 - w`, `Y = 2r + 1 - h`, and is inside when
/// `X^2 h^2 + Y^2 w^2 <= w^2 h^2`; solving for `X` gives the span exactly.
pub(crate) fn ellipse_row(width: i64, height: i64, row: i64) -> Option<(i64, i64)> {
    let (w, h) = (i128::from(width), i128::from(height));
    let y = 2 * i128::from(row) + 1 - h;
    let room = h * h - y * y;
    if w <= 0 || room < 0 {
        return None;
    }
    let x_max = ((w * w * room) / (h * h)).isqrt();
    // X = 2c + 1 - w, so c = (X + w - 1) / 2 over the X of the right parity.
    let first = (w - 1 - x_max + 1).div_euclid(2);
    let last = (w - 1 + x_max).div_euclid(2);
    (first <= last).then_some((first as i64, last as i64))
}

/// The angles an arc covers, tested without trigonometry per pixel.
///
/// Angles follow MIDP: degrees, 0 at three o'clock, counter-clockwise,
/// both ends included. They are measured against the bounding box, so on
/// a flattened ellipse 45 degrees still points at the box's corner: a
/// point offset `(dx, dy)` from the centre (y down) has the direction
/// `(dx * h, -dy * w)`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Wedge {
    /// `None` for a full turn.
    sides: Option<WedgeSides>,
}

#[derive(Clone, Copy, Debug)]
struct WedgeSides {
    start: (f64, f64),
    end: (f64, f64),
    sweep: i64,
}

/// The unit vector at `degrees`, exact on the axes.
fn unit(degrees: i64) -> (f64, f64) {
    match degrees.rem_euclid(360) {
        0 => (1.0, 0.0),
        90 => (0.0, 1.0),
        180 => (-1.0, 0.0),
        270 => (0.0, -1.0),
        d => {
            let r = (d as f64).to_radians();
            (r.cos(), r.sin())
        }
    }
}

fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
    a.0 * b.1 - a.1 * b.0
}

impl Wedge {
    /// The wedge from `start` sweeping `arc` degrees; a negative `arc`
    /// turns clockwise. `None` when `arc` is zero, which draws nothing.
    pub fn new(start: i32, arc: i32) -> Option<Self> {
        if arc == 0 {
            return None;
        }
        if arc >= 360 || arc <= -360 {
            return Some(Self { sides: None });
        }
        let (start, sweep) = if arc < 0 {
            (i64::from(start) + i64::from(arc), -i64::from(arc))
        } else {
            (i64::from(start), i64::from(arc))
        };
        Some(Self {
            sides: Some(WedgeSides {
                start: unit(start),
                end: unit(start + sweep),
                sweep,
            }),
        })
    }

    pub fn is_full(&self) -> bool {
        self.sides.is_none()
    }

    /// Whether the direction `(x, y)` (y up) lies in the wedge. The centre
    /// itself, `(0, 0)`, always does.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        let Some(s) = self.sides else {
            return true;
        };
        if x == 0.0 && y == 0.0 {
            return true;
        }
        // Directions within rounding error of a side count as on it, so
        // points on an end angle (such as a 45 degree diagonal) are drawn
        // whatever the platform's sine and cosine round to.
        let p = (x, y);
        let tolerance = 1e-12 * (x.abs() + y.abs());
        if s.sweep <= 180 {
            cross(s.start, p) >= -tolerance && cross(p, s.end) >= -tolerance
        } else {
            !(cross(s.end, p) > tolerance && cross(p, s.start) > tolerance)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline(w: i64, h: i64) -> Vec<String> {
        let mut grid = vec![vec!['.'; (w + 1) as usize]; (h + 1) as usize];
        ellipse_outline(0, 0, w, h, |x, y, _| grid[y as usize][x as usize] = '#');
        grid.into_iter()
            .map(|row| row.into_iter().collect())
            .collect()
    }

    #[test]
    fn outlines_touch_every_side_of_their_box() {
        for w in 0..12 {
            for h in 0..12 {
                let rows = outline(w, h);
                assert!(
                    rows[0].contains('#') && rows[h as usize].contains('#'),
                    "{w}x{h}"
                );
                let column = |x: usize| rows.iter().any(|r| r.as_bytes()[x] == b'#');
                assert!(column(0) && column(w as usize), "{w}x{h}");
                // Mirror symmetry both ways.
                for (i, row) in rows.iter().enumerate() {
                    let flipped: String = row.chars().rev().collect();
                    assert_eq!(row, &flipped, "{w}x{h}");
                    assert_eq!(row, &rows[h as usize - i], "{w}x{h}");
                }
            }
        }
    }

    #[test]
    fn small_circle_outlines() {
        assert_eq!(outline(0, 0), ["#"]);
        assert_eq!(outline(2, 2), [".#.", "#.#", ".#."]);
        assert_eq!(outline(4, 4), [".###.", "#...#", "#...#", "#...#", ".###."]);
        assert_eq!(outline(3, 0), ["####"]);
        assert_eq!(outline(0, 2), ["#", "#", "#"]);
    }

    fn area(w: i64, h: i64) -> Vec<String> {
        (0..h)
            .map(|r| {
                (0..w)
                    .map(|c| match ellipse_row(w, h, r) {
                        Some((a, b)) if (a..=b).contains(&c) => '#',
                        _ => '.',
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn areas_count_pixel_centres() {
        assert_eq!(area(1, 1), ["#"]);
        assert_eq!(area(2, 2), ["##", "##"]);
        assert_eq!(area(4, 4), [".##.", "####", "####", ".##."]);
        assert_eq!(area(6, 2), ["######", "######"]);
        assert_eq!(area(8, 2), [".######.", ".######."]);
        assert_eq!(area(5, 5), [".###.", "#####", "#####", "#####", ".###."]);
    }

    #[test]
    fn area_rows_match_a_direct_test() {
        for w in 1..20_i64 {
            for h in 1..20_i64 {
                for r in 0..h {
                    let inside = |c: i64| {
                        let x = 2 * c + 1 - w;
                        let y = 2 * r + 1 - h;
                        x * x * h * h + y * y * w * w <= w * w * h * h
                    };
                    let direct: Vec<i64> = (0..w).filter(|&c| inside(c)).collect();
                    let span = ellipse_row(w, h, r).map(|(a, b)| (a..=b).collect::<Vec<_>>());
                    assert_eq!(span.unwrap_or_default(), direct, "{w}x{h} row {r}");
                }
            }
        }
    }

    #[test]
    fn wedges_cover_their_angles() {
        let quarter = Wedge::new(0, 90).unwrap();
        assert!(quarter.contains(1.0, 0.0) && quarter.contains(0.0, 1.0));
        assert!(quarter.contains(1.0, 1.0));
        assert!(!quarter.contains(-1.0, 0.1) && !quarter.contains(1.0, -0.1));
        let clockwise = Wedge::new(0, -90).unwrap();
        assert!(clockwise.contains(1.0, -1.0) && !clockwise.contains(1.0, 1.0));
        let diagonal = Wedge::new(45, 90).unwrap();
        assert!(diagonal.contains(7.0, 7.0) && diagonal.contains(-7.0, 7.0));
        assert!(!diagonal.contains(7.0, 6.9));
        let big = Wedge::new(45, 270).unwrap();
        assert!(big.contains(-1.0, 0.0) && big.contains(0.0, -1.0));
        assert!(!big.contains(1.0, 0.0));
        assert!(big.contains(1.0, 1.0) && big.contains(1.0, -1.0));
        assert!(Wedge::new(10, 0).is_none());
        assert!(Wedge::new(10, -400).unwrap().is_full());
        let wrapped = Wedge::new(-450, 90).unwrap();
        assert!(wrapped.contains(0.0, -1.0) && wrapped.contains(1.0, 0.0));
        assert!(!wrapped.contains(0.0, 1.0));
    }
}
