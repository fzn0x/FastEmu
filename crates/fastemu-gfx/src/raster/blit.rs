//! Copying pixel rectangles onto a canvas: images, regions with a sprite
//! transform, `int` arrays and copies within a surface.

use super::{Canvas, Clip};
use crate::color::{OPAQUE, blend};
use crate::transform::Transform;

/// A pixel a blit can read: surface pixels are `u32`, Java `int` arrays
/// arrive as `i32`. Both hold `0xAARRGGBB`.
pub(crate) trait SourcePixel: Copy {
    fn argb(self) -> u32;

    /// Copies a run unchanged.
    fn copy_run(dst: &mut [u32], src: &[Self]) {
        for (d, &s) in dst.iter_mut().zip(src) {
            *d = s.argb();
        }
    }
}

impl SourcePixel for u32 {
    #[inline]
    fn argb(self) -> u32 {
        self
    }

    fn copy_run(dst: &mut [u32], src: &[u32]) {
        dst.copy_from_slice(src);
    }
}

impl SourcePixel for i32 {
    #[inline]
    fn argb(self) -> u32 {
        self as u32
    }
}

/// How source pixels land on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlitMode {
    /// Replace the destination, alpha and all: opaque sources and
    /// `copyArea`. Rows of an untransformed source are copied whole.
    Copy,
    /// Replace the destination with the source made opaque, as `drawRGB`
    /// does without `processAlpha`.
    Opaque,
    /// Source-over.
    Blend,
}

/// A `width` by `height` rectangle of pixels in a slice. Row `r` starts at
/// `origin + r * stride`, and `stride` may be negative for rows stored
/// bottom-up. Every pixel of the rectangle must lie in `pixels`.
pub(crate) struct Source<'a, P> {
    pub pixels: &'a [P],
    pub origin: i64,
    pub stride: i64,
    pub width: i64,
    pub height: i64,
}

/// Draws `src` with `transform` applied, the transformed rectangle's top
/// left at `(x, y)`, clipped to the canvas.
pub(crate) fn blit<P: SourcePixel>(
    canvas: &mut Canvas<'_>,
    src: &Source<'_, P>,
    transform: Transform,
    x: i64,
    y: i64,
    mode: BlitMode,
) {
    let (tw, th) = transform.transformed_size(src.width, src.height);
    let area = Clip {
        x0: x,
        y0: y,
        x1: x + tw,
        y1: y + th,
    }
    .intersect(canvas.clip);
    if area.is_empty() {
        return;
    }
    let inverse = transform.inverse(src.width, src.height);
    // Index steps through the source per destination column and row.
    let step = inverse.x.0 + inverse.y.0 * src.stride;
    let u0 = area.x0 - x;
    for py in area.y0..area.y1 {
        let (sx, sy) = inverse.map(u0, py - y);
        let first = src.origin + sy * src.stride + sx;
        let row = canvas.row(py, area.x0, area.x1);
        if step == 1 {
            let run = &src.pixels[first as usize..first as usize + row.len()];
            write_run(row, run, mode);
        } else {
            let mut index = first;
            for d in row {
                *d = write(*d, src.pixels[index as usize].argb(), mode);
                index += step;
            }
        }
    }
}

#[inline]
fn write(dst: u32, src: u32, mode: BlitMode) -> u32 {
    match mode {
        BlitMode::Copy => src,
        BlitMode::Opaque => src | OPAQUE,
        BlitMode::Blend => blend(dst, src),
    }
}

fn write_run<P: SourcePixel>(dst: &mut [u32], src: &[P], mode: BlitMode) {
    match mode {
        BlitMode::Copy => P::copy_run(dst, src),
        BlitMode::Opaque => {
            for (d, &s) in dst.iter_mut().zip(src) {
                *d = s.argb() | OPAQUE;
            }
        }
        BlitMode::Blend => {
            for (d, &s) in dst.iter_mut().zip(src) {
                *d = blend(*d, s.argb());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raster::Clip;
    use crate::surface::Surface;

    #[test]
    fn transformed_blits_match_the_transform() {
        let pixels: Vec<u32> = (0..6).map(|i| OPAQUE | i).collect();
        let src = Source {
            pixels: &pixels,
            origin: 0,
            stride: 3,
            width: 3,
            height: 2,
        };
        for t in Transform::ALL {
            let mut surface = Surface::new_mutable(5, 5).unwrap();
            let mut canvas = Canvas::new(&mut surface, Clip::of_size(5, 5), 0);
            blit(&mut canvas, &src, t, 1, 1, BlitMode::Copy);
            for sy in 0..2 {
                for sx in 0..3 {
                    let (u, v) = t.apply(sx, sy, 3, 2);
                    let got = surface.pixel((u + 1) as u32, (v + 1) as u32);
                    assert_eq!(got, Some(OPAQUE | (sy * 3 + sx) as u32), "{t:?}");
                }
            }
        }
    }

    #[test]
    fn negative_strides_read_rows_upwards() {
        let pixels = [1_i32, 2, 3, 4];
        let src = Source {
            pixels: &pixels,
            origin: 2,
            stride: -2,
            width: 2,
            height: 2,
        };
        let mut surface = Surface::new_mutable(2, 2).unwrap();
        let mut canvas = Canvas::new(&mut surface, Clip::of_size(2, 2), 0);
        blit(&mut canvas, &src, Transform::None, 0, 0, BlitMode::Opaque);
        assert_eq!(
            surface.pixels(),
            &[OPAQUE | 3, OPAQUE | 4, OPAQUE | 1, OPAQUE | 2]
        );
    }

    #[test]
    fn blits_clip_and_blend() {
        let pixels = [0x8000_0000_u32, 0x0000_0000, OPAQUE, 0xFF12_3456];
        let src = Source {
            pixels: &pixels,
            origin: 0,
            stride: 2,
            width: 2,
            height: 2,
        };
        let mut surface = Surface::new_mutable(3, 3).unwrap();
        let clip = Clip {
            x0: 0,
            y0: 0,
            x1: 2,
            y1: 3,
        };
        let mut canvas = Canvas::new(&mut surface, clip, 0);
        blit(&mut canvas, &src, Transform::None, 1, 1, BlitMode::Blend);
        let w = 0xFFFF_FFFF;
        assert_eq!(
            surface.pixels(),
            &[w, w, w, w, 0xFF7F_7F7F, w, w, OPAQUE, w]
        );
    }
}
