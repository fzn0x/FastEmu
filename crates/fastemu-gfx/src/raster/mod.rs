//! Low-level rasterization: a clipped canvas over a surface and the scan
//! algorithms for each kind of primitive. Coordinates here are absolute
//! (already translated) and `i64`, so no Java argument can overflow them.

pub(crate) mod blit;
pub(crate) mod ellipse;
pub(crate) mod line;
pub(crate) mod polygon;

use crate::color::{Over, blend};
use crate::surface::Surface;

/// A half-open rectangle `[x0, x1) x [y0, y1)` in surface pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Clip {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Clip {
    /// The whole of a `width` by `height` surface.
    pub fn of_size(width: i64, height: i64) -> Self {
        Self {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.x0 >= self.x1 || self.y0 >= self.y1
    }

    pub fn contains(&self, x: i64, y: i64) -> bool {
        (self.x0..self.x1).contains(&x) && (self.y0..self.y1).contains(&y)
    }

    /// The overlap of two rectangles; empty ones keep a valid shape.
    #[must_use]
    pub fn intersect(self, other: Clip) -> Clip {
        let x0 = self.x0.max(other.x0);
        let y0 = self.y0.max(other.y0);
        Clip {
            x0,
            y0,
            x1: self.x1.min(other.x1).max(x0),
            y1: self.y1.min(other.y1).max(y0),
        }
    }
}

/// Whether pixel `k` of a dotted outline is drawn. MIDP leaves the
/// pattern to the implementation; FastEmu alternates single pixels,
/// counted along the outline from its first pixel.
pub(crate) fn dotted_visible(k: i64) -> bool {
    k & 1 == 0
}

/// A surface's pixels with a clip and a colour to draw in.
pub(crate) struct Canvas<'a> {
    pixels: &'a mut [u32],
    stride: usize,
    /// The clip, already inside the surface.
    pub clip: Clip,
    pub color: u32,
}

impl<'a> Canvas<'a> {
    /// A canvas over `surface`, clipped to `clip` and the surface bounds.
    pub fn new(surface: &'a mut Surface, clip: Clip, color: u32) -> Self {
        let bounds = Clip::of_size(surface.width().into(), surface.height().into());
        let stride = surface.width() as usize;
        Self {
            pixels: surface.pixels_mut(),
            stride,
            clip: clip.intersect(bounds),
            color,
        }
    }

    /// The pixels `[x0, x1)` of row `y`, which must lie inside the clip.
    pub fn row(&mut self, y: i64, x0: i64, x1: i64) -> &mut [u32] {
        let start = y as usize * self.stride;
        &mut self.pixels[start + x0 as usize..start + x1 as usize]
    }

    /// Fills `[x0, x1)` of row `y` with the colour, clipped.
    pub fn span(&mut self, y: i64, x0: i64, x1: i64) {
        if y < self.clip.y0 || y >= self.clip.y1 {
            return;
        }
        let x0 = x0.max(self.clip.x0);
        let x1 = x1.min(self.clip.x1);
        if x0 >= x1 {
            return;
        }
        let color = self.color;
        fill(self.row(y, x0, x1), color);
    }

    /// Fills the rectangle `[x0, x1) x [y0, y1)`, clipped.
    pub fn rect(&mut self, x0: i64, y0: i64, x1: i64, y1: i64) {
        let area = Clip { x0, y0, x1, y1 }.intersect(self.clip);
        if area.is_empty() {
            return;
        }
        let color = self.color;
        for y in area.y0..area.y1 {
            fill(self.row(y, area.x0, area.x1), color);
        }
    }

    /// Draws one pixel in the colour, clipped.
    pub fn pixel(&mut self, x: i64, y: i64) {
        if self.clip.contains(x, y) {
            let i = y as usize * self.stride + x as usize;
            self.pixels[i] = blend(self.pixels[i], self.color);
        }
    }
}

/// Fills a run of pixels with `color`, blending unless it is opaque.
fn fill(run: &mut [u32], color: u32) {
    match color >> 24 {
        0xFF => run.fill(color),
        0 => {}
        _ => {
            let over = Over::new(color);
            for p in run {
                *p = over.apply(*p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_intersect_into_valid_rectangles() {
        let a = Clip::of_size(10, 10);
        let b = Clip {
            x0: 5,
            y0: -3,
            x1: 20,
            y1: 4,
        };
        assert_eq!(
            a.intersect(b),
            Clip {
                x0: 5,
                y0: 0,
                x1: 10,
                y1: 4
            }
        );
        let far = Clip {
            x0: 30,
            y0: 30,
            x1: 40,
            y1: 40,
        };
        assert!(a.intersect(far).is_empty());
    }

    #[test]
    fn canvas_spans_clip_and_blend() {
        let mut surface = Surface::new_mutable(4, 2).unwrap();
        let clip = Clip {
            x0: 1,
            y0: 0,
            x1: 3,
            y1: 2,
        };
        let mut canvas = Canvas::new(&mut surface, clip, 0xFF00_0000);
        canvas.span(0, -5, 50);
        canvas.color = 0x8000_0000;
        canvas.span(1, 0, 2);
        canvas.pixel(3, 1);
        canvas.span(5, 0, 4);
        let w = 0xFFFF_FFFF;
        assert_eq!(
            surface.pixels(),
            &[w, 0xFF00_0000, 0xFF00_0000, w, w, 0xFF7F_7F7F, w, w]
        );
    }
}
