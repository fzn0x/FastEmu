//! Drawing images, image regions and `int` pixel arrays, and copying
//! within the target.

use super::Graphics;
use crate::anchor::Anchor;
use crate::error::{GfxError, Result};
use crate::raster::blit::{BlitMode, Source, blit};
use crate::surface::{Surface, check_array_rows};
use crate::transform::Transform;

impl Graphics {
    /// `drawImage(img, x, y, anchor)`: the whole image with its anchor
    /// point at `(x, y)`. `VCENTER` is allowed, `BASELINE` is not.
    ///
    /// An image cannot be drawn onto itself through this call (Rust's
    /// borrows forbid it); MIDP leaves the result undefined, so the wrapper
    /// may draw a copy or use [`Graphics::copy_area`].
    pub fn draw_image(
        &self,
        target: &mut Surface,
        image: &Surface,
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        let anchor = Anchor::for_image(anchor, self.anchors)?;
        let (ax, ay) = self.at(x, y);
        let (w, h) = (i64::from(image.width()), i64::from(image.height()));
        let (left, top) = anchor.top_left(ax, ay, w, h, 0);
        let src = Source {
            pixels: image.pixels(),
            origin: 0,
            stride: w,
            width: w,
            height: h,
        };
        let mut canvas = self.canvas(target);
        blit(
            &mut canvas,
            &src,
            Transform::None,
            left,
            top,
            mode_for(image),
        );
        Ok(())
    }

    /// `drawRegion(src, x_src, y_src, width, height, transform, x_dest,
    /// y_dest, anchor)`: a region of `source`, transformed, with the
    /// anchor applied to the transformed region.
    ///
    /// Throws `IllegalArgumentException` for an invalid transform or
    /// anchor, or a region that is negative or not inside the source. MIDP
    /// also forbids drawing a region of the target onto itself; the
    /// wrapper throws that `IllegalArgumentException` when the Java source
    /// and destination are the same image.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_region(
        &self,
        target: &mut Surface,
        source: &Surface,
        x_src: i32,
        y_src: i32,
        width: i32,
        height: i32,
        transform: i32,
        x_dest: i32,
        y_dest: i32,
        anchor: i32,
    ) -> Result<()> {
        let transform = Transform::from_midp(transform)?;
        let anchor = Anchor::for_image(anchor, self.anchors)?;
        let (sx, sy, w, h) = (
            i64::from(x_src),
            i64::from(y_src),
            i64::from(width),
            i64::from(height),
        );
        if !source.contains_region(sx, sy, w, h) {
            return Err(GfxError::IllegalArgument(
                "the region exceeds the bounds of the source image",
            ));
        }
        if w == 0 || h == 0 {
            return Ok(());
        }
        let (tw, th) = transform.transformed_size(w, h);
        let (ax, ay) = self.at(x_dest, y_dest);
        let (left, top) = anchor.top_left(ax, ay, tw, th, 0);
        let stride = i64::from(source.width());
        let src = Source {
            pixels: source.pixels(),
            origin: sy * stride + sx,
            stride,
            width: w,
            height: h,
        };
        let mut canvas = self.canvas(target);
        blit(&mut canvas, &src, transform, left, top, mode_for(source));
        Ok(())
    }

    /// `drawRGB(rgbData, offset, scanlength, x, y, width, height,
    /// processAlpha)`: pixel `(i, j)` of the rectangle is
    /// `rgb[offset + i + j * scanlength]`; `scanlength` may be negative.
    /// Without `process_alpha` every pixel is drawn opaque.
    ///
    /// Nothing is drawn if either size is zero or less. Otherwise any row
    /// reaching outside `rgb` throws `ArrayIndexOutOfBoundsException`,
    /// whether or not it is clipped away.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_rgb(
        &self,
        target: &mut Surface,
        rgb: &[i32],
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        process_alpha: bool,
    ) -> Result<()> {
        if width <= 0 || height <= 0 {
            return Ok(());
        }
        check_array_rows(rgb.len(), offset, scanlength, width, height)?;
        let src = Source {
            pixels: rgb,
            origin: offset.into(),
            stride: scanlength.into(),
            width: width.into(),
            height: height.into(),
        };
        let (left, top) = self.at(x, y);
        let mode = if process_alpha {
            BlitMode::Blend
        } else {
            BlitMode::Opaque
        };
        let mut canvas = self.canvas(target);
        blit(&mut canvas, &src, Transform::None, left, top, mode);
        Ok(())
    }

    /// `copyArea(x_src, y_src, width, height, x_dest, y_dest, anchor)`:
    /// copies a region of the target within itself, overlaps included,
    /// clipped at the destination. Pixels are copied as they are, without
    /// blending.
    ///
    /// Throws `IllegalStateException` on the screen, and
    /// `IllegalArgumentException` for an invalid anchor or a source region
    /// (in translated coordinates) that is negative or not inside the
    /// target. A zero size copies nothing.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn copy_area(
        &self,
        target: &mut Surface,
        x_src: i32,
        y_src: i32,
        width: i32,
        height: i32,
        x_dest: i32,
        y_dest: i32,
        anchor: i32,
    ) -> Result<()> {
        if self.screen {
            return Err(GfxError::IllegalState("copyArea cannot copy on the screen"));
        }
        let anchor = Anchor::for_image(anchor, self.anchors)?;
        let (sx, sy) = self.at(x_src, y_src);
        let (w, h) = (i64::from(width), i64::from(height));
        if !target.contains_region(sx, sy, w, h) {
            return Err(GfxError::IllegalArgument(
                "the region exceeds the bounds of the image",
            ));
        }
        if w == 0 || h == 0 {
            return Ok(());
        }
        // Copy out first so overlapping source and destination work.
        let mut copy = Vec::with_capacity((w * h) as usize);
        for row in sy..sy + h {
            copy.extend_from_slice(target.row_slice(sx as usize, row as usize, w as usize));
        }
        let (ax, ay) = self.at(x_dest, y_dest);
        let (left, top) = anchor.top_left(ax, ay, w, h, 0);
        let src = Source {
            pixels: &copy,
            origin: 0,
            stride: w,
            width: w,
            height: h,
        };
        let mut canvas = self.canvas(target);
        blit(
            &mut canvas,
            &src,
            Transform::None,
            left,
            top,
            BlitMode::Copy,
        );
        Ok(())
    }
}

/// Opaque images are copied; others blend.
fn mode_for(image: &Surface) -> BlitMode {
    if image.is_opaque() {
        BlitMode::Copy
    } else {
        BlitMode::Blend
    }
}

#[cfg(test)]
mod tests {
    use crate::anchor::{AnchorPolicy, BASELINE, BOTTOM, HCENTER, LEFT, RIGHT, TOP, VCENTER};
    use crate::color::OPAQUE;
    use crate::error::GfxError;
    use crate::graphics::Graphics;
    use crate::surface::{Surface, SurfaceKind};
    use crate::transform::Transform;

    const W: u32 = 0xFFFF_FFFF;

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = crate::testing::graphics(&surface);
        (surface, g)
    }

    /// A 2x3 image with pixels 1 to 6, row by row.
    fn sprite() -> Surface {
        Surface::from_pixels(
            2,
            3,
            (1..=6).map(|i| OPAQUE | i).collect(),
            SurfaceKind::Immutable,
        )
        .unwrap()
    }

    fn at(s: &Surface, x: u32, y: u32) -> u32 {
        s.pixel(x, y).unwrap() & 0xFF_FFFF
    }

    #[test]
    fn draw_image_honours_anchors() {
        let image = sprite();
        for (anchor, x0, y0) in [
            (0, 3, 3),
            (TOP | LEFT, 3, 3),
            (HCENTER | VCENTER, 2, 2),
            (RIGHT | BOTTOM, 1, 0),
            (RIGHT | TOP, 1, 3),
        ] {
            let (mut s, g) = canvas(6, 7);
            g.draw_image(&mut s, &image, 3, 3, anchor).unwrap();
            assert_eq!(at(&s, x0, y0), 1, "anchor {anchor}");
            assert_eq!(at(&s, x0 + 1, y0 + 2), 6, "anchor {anchor}");
        }
        let (mut s, g) = canvas(4, 4);
        assert!(matches!(
            g.draw_image(&mut s, &image, 0, 0, BASELINE | LEFT),
            Err(GfxError::IllegalArgument(_))
        ));
    }

    #[test]
    fn lenient_anchors_draw_where_strict_ones_fail() {
        let image = sprite();
        let (mut s, mut g) = canvas(4, 4);
        assert!(g.draw_image(&mut s, &image, 0, 0, TOP).is_err());
        g.set_anchor_policy(AnchorPolicy::Lenient);
        g.draw_image(&mut s, &image, 0, 0, TOP).unwrap();
        assert_eq!(at(&s, 0, 0), 1);
    }

    #[test]
    fn draw_image_clips_and_translates() {
        let image = sprite();
        let (mut s, mut g) = canvas(3, 3);
        g.translate(-1, 1);
        g.set_clip(1, -1, 2, 2);
        g.draw_image(&mut s, &image, 1, -1, 0).unwrap();
        // The image's top-left lands at surface (0, 0); the clip keeps
        // columns 0 and 1 of rows 0 and 1.
        assert_eq!(
            s.pixels()[0..3]
                .iter()
                .map(|p| p & 0xFF)
                .collect::<Vec<_>>(),
            [1, 2, 0xFF]
        );
        assert_eq!(
            s.pixels()[3..6]
                .iter()
                .map(|p| p & 0xFF)
                .collect::<Vec<_>>(),
            [3, 4, 0xFF]
        );
        assert_eq!(s.pixels()[6..9], [W, W, W]);
    }

    #[test]
    fn regions_apply_the_anchor_after_the_transform() {
        let image = sprite();
        let (mut s, g) = canvas(6, 6);
        // The bottom two rows, turned a quarter clockwise, become 2x2;
        // anchored at the bottom right of (4, 4) they cover (2..4, 2..4).
        let rot90 = Transform::Rot90.to_midp();
        g.draw_region(&mut s, &image, 0, 1, 2, 2, rot90, 4, 4, RIGHT | BOTTOM)
            .unwrap();
        assert_eq!(
            [at(&s, 2, 2), at(&s, 3, 2), at(&s, 2, 3), at(&s, 3, 3)],
            [5, 3, 6, 4]
        );
        // A tall region turned on its side anchors with its new width.
        let (mut s, g) = canvas(6, 6);
        g.draw_region(&mut s, &image, 0, 0, 1, 3, rot90, 3, 0, HCENTER | TOP)
            .unwrap();
        assert_eq!([at(&s, 2, 0), at(&s, 3, 0), at(&s, 4, 0)], [5, 3, 1]);
    }

    #[test]
    fn regions_check_their_arguments() {
        let image = sprite();
        let (mut s, g) = canvas(4, 4);
        let iae = |r| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(g.draw_region(&mut s, &image, 1, 0, 2, 1, 0, 0, 0, 0)));
        assert!(iae(g.draw_region(&mut s, &image, 0, 0, -1, 1, 0, 0, 0, 0)));
        assert!(iae(g.draw_region(&mut s, &image, 0, 0, 1, 1, 9, 0, 0, 0)));
        assert!(iae(
            g.draw_region(&mut s, &image, 0, 0, 1, 1, 0, 0, 0, BASELINE)
        ));
        assert_eq!(
            g.draw_region(&mut s, &image, 2, 3, 0, 0, 0, 0, 0, 0),
            Ok(())
        );
    }

    #[test]
    fn translucent_images_blend() {
        let image = Surface::from_rgb(&[0x8000_0000_u32 as i32, 0], 2, 1, true).unwrap();
        let (mut s, g) = canvas(2, 1);
        g.draw_image(&mut s, &image, 0, 0, 0).unwrap();
        assert_eq!(s.pixels(), &[0xFF7F_7F7F, W]);
        assert!(s.is_opaque());
    }

    #[test]
    fn draw_rgb_uses_offset_scanlength_and_alpha() {
        let rgb = [0, 0x11, 0x22, 0, 0x33, 0x0044_0000];
        let (mut s, g) = canvas(2, 2);
        g.draw_rgb(&mut s, &rgb, 1, 3, 0, 0, 2, 2, false).unwrap();
        assert_eq!(
            s.pixels(),
            &[
                OPAQUE | 0x11,
                OPAQUE | 0x22,
                OPAQUE | 0x33,
                OPAQUE | 0x44_0000
            ]
        );
        let (mut s, g) = canvas(2, 2);
        g.draw_rgb(&mut s, &rgb, 1, 3, 0, 0, 2, 2, true).unwrap();
        assert_eq!(s.pixels(), &[W; 4], "zero alpha draws nothing");
        let (mut s, g) = canvas(1, 2);
        g.draw_rgb(&mut s, &rgb, 4, -3, 0, 0, 1, 2, false).unwrap();
        assert_eq!(s.pixels(), &[OPAQUE | 0x33, OPAQUE | 0x11]);
    }

    #[test]
    fn draw_rgb_checks_the_whole_array_even_when_clipped() {
        let rgb = [0; 4];
        let (mut s, mut g) = canvas(2, 2);
        g.set_clip(0, 0, 0, 0);
        assert_eq!(
            g.draw_rgb(&mut s, &rgb, 0, 2, 0, 0, 2, 3, false),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        assert_eq!(
            g.draw_rgb(&mut s, &rgb, -1, 2, 0, 0, 1, 1, false),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        assert_eq!(g.draw_rgb(&mut s, &rgb, -50, 2, 0, 0, 0, 3, false), Ok(()));
    }

    #[test]
    fn get_rgb_and_draw_rgb_round_trip() {
        let rgb: Vec<i32> = (0..12).map(|i| (OPAQUE | (i * 0x0A0B0C)) as i32).collect();
        let (mut s, g) = canvas(4, 3);
        g.draw_rgb(&mut s, &rgb, 0, 4, 0, 0, 4, 3, true).unwrap();
        let mut back = vec![0; 12];
        s.get_rgb(&mut back, 0, 4, 0, 0, 4, 3).unwrap();
        assert_eq!(back, rgb);
    }

    #[test]
    fn copy_area_handles_overlap_and_refuses_the_screen() {
        let (mut s, g) = canvas(4, 1);
        g.draw_rgb(&mut s, &[1, 2, 3, 4], 0, 4, 0, 0, 4, 1, false)
            .unwrap();
        g.copy_area(&mut s, 0, 0, 3, 1, 1, 0, 0).unwrap();
        assert_eq!(
            s.pixels().iter().map(|p| p & 0xFF).collect::<Vec<_>>(),
            [1, 1, 2, 3]
        );
        assert!(matches!(
            g.copy_area(&mut s, 2, 0, 3, 1, 0, 0, 0),
            Err(GfxError::IllegalArgument(_))
        ));
        assert!(matches!(
            g.copy_area(&mut s, 0, 0, 1, 1, 0, 0, BASELINE),
            Err(GfxError::IllegalArgument(_))
        ));
        let mut screen = Surface::screen(4, 4).unwrap();
        let sg = crate::testing::graphics(&screen);
        assert!(matches!(
            sg.copy_area(&mut screen, 0, 0, 1, 1, 0, 0, 0),
            Err(GfxError::IllegalState(_))
        ));
    }

    #[test]
    fn copy_area_uses_translated_source_coordinates() {
        let (mut s, mut g) = canvas(3, 1);
        g.draw_rgb(&mut s, &[7, 8, 9], 0, 3, 0, 0, 3, 1, false)
            .unwrap();
        g.translate(1, 0);
        g.copy_area(&mut s, 1, 0, 1, 1, -1, 0, 0).unwrap();
        assert_eq!(
            s.pixels().iter().map(|p| p & 0xFF).collect::<Vec<_>>(),
            [9, 8, 9]
        );
    }
}
