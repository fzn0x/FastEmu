//! Pixel buffers: the screen, and MIDP's mutable and immutable images.

use fastemu_core::Frame;

use crate::color::OPAQUE;
use crate::error::{GfxError, Result};
use crate::transform::Transform;

/// The most pixels one surface may hold (128 MiB of ARGB). Larger requests
/// fail with [`GfxError::OutOfMemory`], as they would on a phone, instead
/// of exhausting the host.
pub const MAX_PIXELS: u64 = 1 << 25;

/// What a surface is to MIDP, which decides what may draw on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceKind {
    /// The phone's display: `Graphics` may draw on it, `copyArea` may not.
    Screen,
    /// An image from `Image.createImage(width, height)` and the like.
    Mutable,
    /// A decoded, RGB-built or derived image. Nothing draws on it.
    Immutable,
}

/// A rectangle of `0xAARRGGBB` pixels, row-major, the format of
/// [`fastemu_core::Frame`].
///
/// The screen and MIDP's mutable images are opaque. Immutable images and
/// Nokia's `DirectUtils.createImage(w, h, argb)` images may hold
/// transparency. The surface tracks whether every pixel is opaque so
/// drawing it can copy rows instead of blending them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    width: u32,
    height: u32,
    pixels: Vec<u32>,
    kind: SurfaceKind,
    opaque: bool,
}

/// Checks MIDP's image size rules: positive sides
/// (`IllegalArgumentException`) and at most [`MAX_PIXELS`] pixels.
fn checked_size(width: i64, height: i64) -> Result<(u32, u32)> {
    if width <= 0 || height <= 0 {
        return Err(GfxError::IllegalArgument(
            "image width and height must be positive",
        ));
    }
    if (width as u64).saturating_mul(height as u64) > MAX_PIXELS {
        return Err(GfxError::OutOfMemory);
    }
    Ok((width as u32, height as u32))
}

fn all_opaque(pixels: &[u32]) -> bool {
    pixels.iter().all(|&p| p >= OPAQUE)
}

impl Surface {
    /// A `width` by `height` surface of one colour.
    fn filled(width: u32, height: u32, argb: u32, kind: SurfaceKind) -> Self {
        Self {
            width,
            height,
            pixels: vec![argb; width as usize * height as usize],
            kind,
            opaque: argb >= OPAQUE,
        }
    }

    /// The phone's screen, white until the game draws on it.
    pub fn screen(width: u32, height: u32) -> Result<Self> {
        let (w, h) = checked_size(width.into(), height.into())?;
        Ok(Self::filled(w, h, 0xFFFF_FFFF, SurfaceKind::Screen))
    }

    /// `Image.createImage(width, height)`: a white mutable image.
    pub fn new_mutable(width: i32, height: i32) -> Result<Self> {
        Self::new_mutable_filled(width, height, 0xFFFF_FFFF_u32 as i32)
    }

    /// Nokia's `DirectUtils.createImage(width, height, argb)`: a mutable
    /// image of one colour, which may be transparent.
    pub fn new_mutable_filled(width: i32, height: i32, argb: i32) -> Result<Self> {
        let (w, h) = checked_size(width.into(), height.into())?;
        Ok(Self::filled(w, h, argb as u32, SurfaceKind::Mutable))
    }

    /// A surface over existing pixels, which must number `width * height`.
    pub fn from_pixels(
        width: u32,
        height: u32,
        pixels: Vec<u32>,
        kind: SurfaceKind,
    ) -> Result<Self> {
        let (w, h) = checked_size(width.into(), height.into())?;
        if pixels.len() as u64 != u64::from(w) * u64::from(h) {
            return Err(GfxError::IllegalArgument(
                "pixel count does not match the image size",
            ));
        }
        let opaque = all_opaque(&pixels);
        Ok(Self {
            width: w,
            height: h,
            pixels,
            kind,
            opaque,
        })
    }

    /// `Image.createRGBImage(rgb, width, height, processAlpha)`: an
    /// immutable image of the first `width * height` entries of `rgb`. When
    /// `process_alpha` is false every pixel is made opaque.
    pub fn from_rgb(rgb: &[i32], width: i32, height: i32, process_alpha: bool) -> Result<Self> {
        if width <= 0 || height <= 0 {
            return Err(GfxError::IllegalArgument(
                "image width and height must be positive",
            ));
        }
        let count = i64::from(width) * i64::from(height);
        if count > rgb.len() as i64 {
            return Err(GfxError::ArrayIndexOutOfBounds);
        }
        let (w, h) = checked_size(width.into(), height.into())?;
        let source = &rgb[..count as usize];
        let pixels: Vec<u32> = if process_alpha {
            source.iter().map(|&p| p as u32).collect()
        } else {
            source.iter().map(|&p| p as u32 | OPAQUE).collect()
        };
        Self::from_pixels(w, h, pixels, SurfaceKind::Immutable)
    }

    /// `Image.createImage(image, x, y, width, height, transform)`: an
    /// immutable copy of a region of `source`, transformed.
    pub fn from_region(
        source: &Surface,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        transform: i32,
    ) -> Result<Self> {
        let transform = Transform::from_midp(transform)?;
        if width <= 0 || height <= 0 {
            return Err(GfxError::IllegalArgument(
                "region width and height must be positive",
            ));
        }
        if !source.contains_region(x.into(), y.into(), width.into(), height.into()) {
            return Err(GfxError::IllegalArgument(
                "region exceeds the bounds of the source image",
            ));
        }
        let (w, h) = (i64::from(width), i64::from(height));
        let (tw, th) = transform.transformed_size(w, h);
        let inverse = transform.inverse(w, h);
        let mut pixels = Vec::with_capacity((tw * th) as usize);
        for v in 0..th {
            for u in 0..tw {
                let (sx, sy) = inverse.map(u, v);
                pixels.push(source.pixel_at(i64::from(x) + sx, i64::from(y) + sy));
            }
        }
        Self::from_pixels(tw as u32, th as u32, pixels, SurfaceKind::Immutable)
    }

    /// `Image.createImage(image)`: an immutable copy. The wrapper may hand
    /// back an immutable source itself instead, as MIDP allows.
    #[must_use]
    pub fn to_immutable(&self) -> Self {
        Self {
            kind: SurfaceKind::Immutable,
            ..self.clone()
        }
    }

    /// The same pixels as another kind of surface, for Nokia's
    /// `DirectUtils.createImage(byte[], int, int)`, which decodes into a
    /// mutable image.
    #[must_use]
    pub fn into_kind(self, kind: SurfaceKind) -> Self {
        Self { kind, ..self }
    }

    /// `Image.getWidth()`.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// `Image.getHeight()`.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Whether this is the screen, a mutable or an immutable image.
    pub fn kind(&self) -> SurfaceKind {
        self.kind
    }

    /// `Image.isMutable()`: true for mutable images (and the screen, which
    /// is not an `Image` but takes drawing the same way).
    pub fn is_mutable(&self) -> bool {
        self.kind != SurfaceKind::Immutable
    }

    /// Whether every pixel is known to be opaque.
    pub fn is_opaque(&self) -> bool {
        self.opaque
    }

    /// Row-major `0xAARRGGBB` pixels.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// The pixel at `(x, y)`, or `None` outside the surface.
    pub fn pixel(&self, x: u32, y: u32) -> Option<u32> {
        (x < self.width && y < self.height)
            .then(|| self.pixels[y as usize * self.width as usize + x as usize])
    }

    /// Lets code outside the rasterizer (such as a 3D renderer) write
    /// pixels directly, then rechecks whether the surface is opaque.
    pub fn edit_pixels<R>(&mut self, edit: impl FnOnce(&mut [u32]) -> R) -> R {
        let result = edit(&mut self.pixels);
        self.opaque = all_opaque(&self.pixels);
        result
    }

    /// The pixels as a [`Frame`] with the given serial, for presenting the
    /// screen to an app. The pixels are copied once, straight into the
    /// frame's shared buffer.
    pub fn to_frame(&self, serial: u64) -> Frame {
        Frame::new(self.width, self.height, self.pixels[..].into(), serial)
    }

    /// `Image.getRGB(rgbData, offset, scanlength, x, y, width, height)`:
    /// copies a region into `out`, row `r` starting at
    /// `offset + r * scanlength`. A negative `scanlength` stores rows
    /// bottom-up.
    ///
    /// The checks run in this order: `|scanlength| < width`, then a region
    /// outside the image or of negative size (`IllegalArgumentException`),
    /// then any array index outside `out`
    /// (`ArrayIndexOutOfBoundsException`, with `out` left unchanged). An
    /// empty region copies nothing.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn get_rgb(
        &self,
        out: &mut [i32],
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<()> {
        if i64::from(scanlength).abs() < i64::from(width) {
            return Err(GfxError::IllegalArgument(
                "the absolute value of scanlength is less than width",
            ));
        }
        if width < 0
            || height < 0
            || !self.contains_region(x.into(), y.into(), width.into(), height.into())
        {
            return Err(GfxError::IllegalArgument(
                "the region exceeds the bounds of the image",
            ));
        }
        if width == 0 || height == 0 {
            return Ok(());
        }
        check_array_rows(out.len(), offset, scanlength, width, height)?;
        let w = width as usize;
        for r in 0..height {
            let start = (i64::from(offset) + i64::from(r) * i64::from(scanlength)) as usize;
            let src = self.row_slice(x as usize, (y + r) as usize, w);
            for (d, &s) in out[start..start + w].iter_mut().zip(src) {
                *d = s as i32;
            }
        }
        Ok(())
    }

    /// Whether the region lies inside the surface. Sizes must not be
    /// negative.
    pub(crate) fn contains_region(&self, x: i64, y: i64, width: i64, height: i64) -> bool {
        x >= 0
            && y >= 0
            && width >= 0
            && height >= 0
            && x + width <= i64::from(self.width)
            && y + height <= i64::from(self.height)
    }

    /// The pixel at a position known to be inside the surface.
    fn pixel_at(&self, x: i64, y: i64) -> u32 {
        self.pixels[y as usize * self.width as usize + x as usize]
    }

    /// `len` pixels of row `y` from column `x`, all inside the surface.
    pub(crate) fn row_slice(&self, x: usize, y: usize, len: usize) -> &[u32] {
        let start = y * self.width as usize + x;
        &self.pixels[start..start + len]
    }

    /// Mutable pixels for the rasterizer, which keeps the opacity flag
    /// right: drawing onto an opaque surface leaves it opaque, and a
    /// translucent surface stays flagged as one.
    pub(crate) fn pixels_mut(&mut self) -> &mut [u32] {
        &mut self.pixels
    }
}

/// Checks that `height` rows of `width` entries of a Java array of `len`,
/// the first at `offset` and each `scanlength` after the last, all lie in
/// the array, failing with `ArrayIndexOutOfBoundsException` otherwise.
/// `width` and `height` must be positive.
pub(crate) fn check_array_rows(
    len: usize,
    offset: i32,
    scanlength: i32,
    width: i32,
    height: i32,
) -> Result<()> {
    let offset = i64::from(offset);
    let last_row = offset + (i64::from(height) - 1) * i64::from(scanlength);
    let first = offset.min(last_row);
    let last = offset.max(last_row) + i64::from(width) - 1;
    if first < 0 || last >= len as i64 {
        return Err(GfxError::ArrayIndexOutOfBounds);
    }
    Ok(())
}

/// The range `offset .. offset + length` if it lies within `len`: Java's
/// rule for array and string ranges.
pub(crate) fn java_range(len: usize, offset: i32, length: i32) -> Option<std::ops::Range<usize>> {
    let start = usize::try_from(offset).ok()?;
    let count = usize::try_from(length).ok()?;
    let end = start.checked_add(count)?;
    (end <= len).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbered(width: u32, height: u32) -> Surface {
        let pixels = (0..width * height).map(|i| OPAQUE | i).collect();
        Surface::from_pixels(width, height, pixels, SurfaceKind::Immutable).unwrap()
    }

    #[test]
    fn mutable_images_start_white_and_opaque() {
        let image = Surface::new_mutable(3, 2).unwrap();
        assert!(image.is_mutable() && image.is_opaque());
        assert!(image.pixels().iter().all(|&p| p == 0xFFFF_FFFF));
        assert_eq!(
            Surface::new_mutable(0, 2),
            Err(GfxError::IllegalArgument(
                "image width and height must be positive"
            ))
        );
        assert!(Surface::new_mutable(-1, 2).is_err());
        assert_eq!(
            Surface::new_mutable(1 << 20, 1 << 20),
            Err(GfxError::OutOfMemory)
        );
    }

    #[test]
    fn nokia_images_may_start_transparent() {
        let image = Surface::new_mutable_filled(2, 2, 0x0012_3456).unwrap();
        assert!(image.is_mutable());
        assert!(!image.is_opaque());
        assert_eq!(image.pixel(1, 1), Some(0x0012_3456));
    }

    #[test]
    fn rgb_images_keep_or_drop_alpha() {
        let rgb = [0x1122_3344, 0x00FF_FFFF, -1];
        let kept = Surface::from_rgb(&rgb, 2, 1, true).unwrap();
        assert_eq!(kept.pixels(), &[0x1122_3344, 0x00FF_FFFF]);
        assert!(!kept.is_mutable() && !kept.is_opaque());
        let dropped = Surface::from_rgb(&rgb, 3, 1, false).unwrap();
        assert_eq!(dropped.pixels(), &[0xFF22_3344, 0xFFFF_FFFF, 0xFFFF_FFFF]);
        assert!(dropped.is_opaque());
        assert_eq!(
            Surface::from_rgb(&rgb, 2, 2, true),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        assert!(matches!(
            Surface::from_rgb(&rgb, 0, 1, true),
            Err(GfxError::IllegalArgument(_))
        ));
    }

    #[test]
    fn get_rgb_copies_rows_with_offset_and_scanlength() {
        let image = numbered(3, 3);
        let mut out = [0; 8];
        image.get_rgb(&mut out, 1, 4, 1, 1, 2, 2).unwrap();
        let p = |i: u32| (OPAQUE | i) as i32;
        assert_eq!(out, [0, p(4), p(5), 0, 0, p(7), p(8), 0]);
    }

    #[test]
    fn get_rgb_stores_rows_bottom_up_with_negative_scanlength() {
        let image = numbered(2, 2);
        let mut out = [0; 4];
        image.get_rgb(&mut out, 2, -2, 0, 0, 2, 2).unwrap();
        let p = |i: u32| (OPAQUE | i) as i32;
        assert_eq!(out, [p(2), p(3), p(0), p(1)]);
    }

    #[test]
    fn get_rgb_checks_its_arguments() {
        let image = numbered(4, 4);
        let mut out = [7; 16];
        let iae = |r: Result<()>| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(image.get_rgb(&mut out, 0, 3, 0, 0, 4, 1)));
        assert!(iae(image.get_rgb(&mut out, 0, -3, 0, 0, 4, 1)));
        assert!(iae(image.get_rgb(&mut out, 0, 4, 1, 0, 4, 1)));
        assert!(iae(image.get_rgb(&mut out, 0, 4, -1, 0, 1, 1)));
        assert!(iae(image.get_rgb(&mut out, 0, 4, 0, 3, 1, 2)));
        assert!(iae(image.get_rgb(&mut out, 0, 4, 0, 0, 1, -1)));
        let oob = Err(GfxError::ArrayIndexOutOfBounds);
        assert_eq!(image.get_rgb(&mut out, 1, 4, 0, 0, 4, 4), oob);
        assert_eq!(image.get_rgb(&mut out, -1, 4, 0, 0, 1, 1), oob);
        assert_eq!(image.get_rgb(&mut out, 0, -4, 0, 0, 4, 2), oob);
        assert!(out.iter().all(|&v| v == 7), "failed calls leave the array");
        assert_eq!(image.get_rgb(&mut out, 99, 4, 0, 0, 0, 0), Ok(()));
        assert_eq!(image.get_rgb(&mut out, 0, 4, 0, 0, 4, 4), Ok(()));
    }

    #[test]
    fn regions_are_cut_and_transformed() {
        let image = numbered(3, 2);
        let cut = Surface::from_region(&image, 1, 0, 2, 2, 0).unwrap();
        assert_eq!(
            cut.pixels(),
            &[OPAQUE | 1, OPAQUE | 2, OPAQUE | 4, OPAQUE | 5]
        );
        assert!(!cut.is_mutable());
        let turned = Surface::from_region(&image, 0, 0, 3, 2, Transform::Rot90.to_midp()).unwrap();
        assert_eq!((turned.width(), turned.height()), (2, 3));
        // The left column of the source becomes the top row, read upwards.
        assert_eq!(turned.pixel(0, 0), Some(OPAQUE | 3));
        assert_eq!(turned.pixel(1, 0), Some(OPAQUE));
        assert_eq!(turned.pixel(0, 2), Some(OPAQUE | 5));
        let iae = |r: Result<Surface>| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(Surface::from_region(&image, 2, 0, 2, 1, 0)));
        assert!(iae(Surface::from_region(&image, 0, 0, 0, 1, 0)));
        assert!(iae(Surface::from_region(&image, 0, 0, 1, 1, 8)));
    }

    #[test]
    fn edits_recheck_opacity() {
        let mut image = Surface::new_mutable(2, 1).unwrap();
        image.edit_pixels(|p| p[0] = 0x8000_0000);
        assert!(!image.is_opaque());
        let frame = image.to_frame(5);
        assert_eq!(frame.pixels(), &[0x8000_0000, 0xFFFF_FFFF]);
        assert_eq!(frame.serial(), 5);
    }
}
