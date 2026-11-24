//! Lines in MIDP's pixel model: both end points are drawn, and every pixel
//! is the one below and to the right of a point the line passes.

use super::Clip;

/// A line between two pixels, normalized so the pixels it covers depend
/// only on its two end points and not on the order they were given in.
///
/// The line steps one pixel at a time along its major axis (x when it is
/// at least as wide as it is tall). Pixel `i` sits at
/// `minor = round(i * dmin / dmaj)`, rounding halves away from the start,
/// which is Bresenham's choice; the start is the end point with the
/// smaller major coordinate.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Line {
    start_x: i64,
    start_y: i64,
    /// Steps along the major axis; the line has `major + 1` pixels.
    major: i64,
    minor: i64,
    x_major: bool,
    /// +1 or -1: which way the minor coordinate moves.
    minor_dir: i64,
}

impl Line {
    pub fn new(x1: i64, y1: i64, x2: i64, y2: i64) -> Self {
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        let x_major = dx >= dy;
        let swap = if x_major { x2 < x1 } else { y2 < y1 };
        let (sx, sy, ex, ey) = if swap {
            (x2, y2, x1, y1)
        } else {
            (x1, y1, x2, y2)
        };
        let (major, minor, minor_dir) = if x_major {
            (dx, dy, (ey - sy).signum())
        } else {
            (dy, dx, (ex - sx).signum())
        };
        Self {
            start_x: sx,
            start_y: sy,
            major,
            minor,
            x_major,
            minor_dir,
        }
    }

    /// Calls `plot(x, y, i)` for every pixel `i` of the line that lies in
    /// `clip`, in order from the start. Only the part of the line inside
    /// the clip's major extent is walked, so a line of any length costs at
    /// most the clip's width or height.
    pub fn walk(&self, clip: &Clip, mut plot: impl FnMut(i64, i64, i64)) {
        let (major_start, minor_start, lo, hi, minor_lo, minor_hi) = if self.x_major {
            (
                self.start_x,
                self.start_y,
                clip.x0,
                clip.x1,
                clip.y0,
                clip.y1,
            )
        } else {
            (
                self.start_y,
                self.start_x,
                clip.y0,
                clip.y1,
                clip.x0,
                clip.x1,
            )
        };
        let first = (lo - major_start).max(0);
        let last = (hi - 1 - major_start).min(self.major);
        if first > last {
            return;
        }
        // The minor offset of pixel `i` is floor((2 i dmin + dmaj) / 2 dmaj);
        // find it for the first pixel, then step it as Bresenham does.
        let (mut offset, mut rem, two_major) = if self.major == 0 {
            (0, 0, 1)
        } else {
            let two_major = 2 * i128::from(self.major);
            let num = 2 * i128::from(first) * i128::from(self.minor) + i128::from(self.major);
            (
                (num / two_major) as i64,
                (num % two_major) as i64,
                two_major as i64,
            )
        };
        let two_minor = 2 * self.minor;
        for i in first..=last {
            let minor = minor_start + self.minor_dir * offset;
            if (minor_lo..minor_hi).contains(&minor) {
                let major = major_start + i;
                if self.x_major {
                    plot(major, minor, i);
                } else {
                    plot(minor, major, i);
                }
            }
            rem += two_minor;
            if rem >= two_major {
                rem -= two_major;
                offset += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixels(line: Line, clip: Clip) -> Vec<(i64, i64)> {
        let mut out = Vec::new();
        line.walk(&clip, |x, y, _| out.push((x, y)));
        out
    }

    const WIDE: Clip = Clip {
        x0: -100,
        y0: -100,
        x1: 100,
        y1: 100,
    };

    #[test]
    fn both_end_points_are_drawn() {
        assert_eq!(pixels(Line::new(1, 1, 1, 1), WIDE), [(1, 1)]);
        assert_eq!(
            pixels(Line::new(0, 0, 3, 0), WIDE),
            [(0, 0), (1, 0), (2, 0), (3, 0)]
        );
        assert_eq!(
            pixels(Line::new(2, 3, 2, 1), WIDE),
            [(2, 1), (2, 2), (2, 3)]
        );
        assert_eq!(
            pixels(Line::new(0, 0, 4, 2), WIDE),
            [(0, 0), (1, 1), (2, 1), (3, 2), (4, 2)]
        );
    }

    #[test]
    fn direction_does_not_change_the_pixels() {
        for (x1, y1, x2, y2) in [
            (0, 0, 5, 2),
            (0, 0, 2, 5),
            (3, -4, -6, 1),
            (0, 0, 4, 4),
            (0, 0, 6, -3),
        ] {
            let mut a = pixels(Line::new(x1, y1, x2, y2), WIDE);
            let mut b = pixels(Line::new(x2, y2, x1, y1), WIDE);
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b);
            assert_eq!(a.len() as i64, (x2 - x1).abs().max((y2 - y1).abs()) + 1);
        }
    }

    #[test]
    fn clipping_keeps_the_same_pixels() {
        let line = Line::new(-50, -20, 60, 37);
        let clip = Clip {
            x0: -3,
            y0: 0,
            x1: 7,
            y1: 8,
        };
        let all: Vec<_> = pixels(line, WIDE)
            .into_iter()
            .filter(|&(x, y)| clip.contains(x, y))
            .collect();
        assert!(!all.is_empty());
        assert_eq!(pixels(line, clip), all);
    }

    #[test]
    fn huge_lines_walk_only_the_clip() {
        let line = Line::new(-4_000_000_000, 7, 4_000_000_000, 9);
        let clip = Clip {
            x0: 0,
            y0: 0,
            x1: 10,
            y1: 10,
        };
        let got = pixels(line, clip);
        assert_eq!(got.len(), 10);
        assert!(got.iter().all(|&(_, y)| y == 8));
    }

    #[test]
    fn step_indices_count_from_the_start() {
        let mut steps = Vec::new();
        Line::new(5, 0, 0, 0).walk(&WIDE, |x, _, i| steps.push((x, i)));
        assert_eq!(steps[0], (0, 0));
        assert_eq!(steps[5], (5, 5));
    }
}
