//! `DirectGraphics.drawPixels` and `getPixels` in Nokia's pixel formats,
//! and `drawImage` with Nokia's flips and rotations.

use super::{
    TYPE_BYTE_1_GRAY, TYPE_BYTE_1_GRAY_VERTICAL, TYPE_BYTE_2_GRAY, TYPE_BYTE_4_GRAY,
    TYPE_BYTE_8_GRAY, TYPE_BYTE_332_RGB, TYPE_INT_888_RGB, TYPE_INT_8888_ARGB, TYPE_USHORT_444_RGB,
    TYPE_USHORT_555_RGB, TYPE_USHORT_565_RGB, TYPE_USHORT_1555_ARGB, TYPE_USHORT_4444_ARGB,
    manipulation_transform,
};
use crate::color::{OPAQUE, gray_of};
use crate::error::{GfxError, Result};
use crate::graphics::Graphics;
use crate::raster::blit::{BlitMode, Source, blit};
use crate::surface::Surface;

/// A Nokia pixel format and the Java array type it travels in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    /// `int` pixels: `TYPE_INT_888_RGB` (`true` means 8888 with alpha).
    Int { alpha: bool },
    /// `short` pixels, by their `TYPE_USHORT_*` value.
    Short(i32),
    /// `byte` pixels packed `bits` to a byte, left pixel in the high bits;
    /// grey levels, or `TYPE_BYTE_332_RGB` when `rgb332`.
    Packed { bits: u32, rgb332: bool },
    /// `TYPE_BYTE_1_GRAY_VERTICAL`: eight pixels of a column per byte, the
    /// top one in the low bit.
    Vertical,
}

impl Format {
    fn int(format: i32) -> Result<Self> {
        match format {
            TYPE_INT_888_RGB => Ok(Self::Int { alpha: false }),
            TYPE_INT_8888_ARGB => Ok(Self::Int { alpha: true }),
            _ => Err(GfxError::IllegalArgument("unsupported pixel format")),
        }
    }

    /// The `TYPE_USHORT_*` value, checked.
    fn short(format: i32) -> Result<i32> {
        match format {
            TYPE_USHORT_444_RGB
            | TYPE_USHORT_4444_ARGB
            | TYPE_USHORT_565_RGB
            | TYPE_USHORT_555_RGB
            | TYPE_USHORT_1555_ARGB => Ok(format),
            _ => Err(GfxError::IllegalArgument("unsupported pixel format")),
        }
    }

    fn byte(format: i32) -> Result<Self> {
        let packed = |bits| {
            Ok(Self::Packed {
                bits,
                rgb332: false,
            })
        };
        match format {
            TYPE_BYTE_1_GRAY => packed(1),
            TYPE_BYTE_2_GRAY => packed(2),
            TYPE_BYTE_4_GRAY => packed(4),
            TYPE_BYTE_8_GRAY => packed(8),
            TYPE_BYTE_332_RGB => Ok(Self::Packed {
                bits: 8,
                rgb332: true,
            }),
            TYPE_BYTE_1_GRAY_VERTICAL => Ok(Self::Vertical),
            _ => Err(GfxError::IllegalArgument("unsupported pixel format")),
        }
    }

    fn bits(self) -> u32 {
        match self {
            Self::Packed { bits, .. } => bits,
            Self::Vertical => 1,
            Self::Int { .. } | Self::Short(_) => 8,
        }
    }
}

/// A `width` by `height` region of a Java pixel array, `offset` and
/// `scanlength` counted in pixels.
#[derive(Clone, Copy)]
struct Region {
    offset: i64,
    scan: i64,
    width: i64,
    height: i64,
}

impl Region {
    /// Checks Nokia's arguments: negative sizes and a scan length shorter
    /// than a row are `IllegalArgumentException`, a negative offset is
    /// `ArrayIndexOutOfBoundsException`. `None` for an empty region.
    fn new(offset: i32, scanlength: i32, width: i32, height: i32) -> Result<Option<Self>> {
        if width < 0 || height < 0 || scanlength < width {
            return Err(GfxError::IllegalArgument("invalid pixel region"));
        }
        if offset < 0 {
            return Err(GfxError::ArrayIndexOutOfBounds);
        }
        Ok((width > 0 && height > 0).then_some(Self {
            offset: offset.into(),
            scan: scanlength.into(),
            width: width.into(),
            height: height.into(),
        }))
    }

    /// Where pixel `(col, row)` lives: the array index and, for packed
    /// formats, the shift of its bits within that element.
    fn position(&self, format: Format, col: i64, row: i64) -> (usize, u32) {
        match format {
            Format::Vertical => {
                let band_row = self.offset / self.scan + row;
                let index = (band_row >> 3) * self.scan + self.offset % self.scan + col;
                (index as usize, (band_row & 7) as u32)
            }
            Format::Packed { bits, .. } => {
                let bit = (self.offset + row * self.scan + col) * i64::from(bits);
                ((bit / 8) as usize, 8 - bits - (bit % 8) as u32)
            }
            Format::Int { .. } | Format::Short(_) => {
                ((self.offset + row * self.scan + col) as usize, 0)
            }
        }
    }

    /// Fails with `ArrayIndexOutOfBoundsException` unless every pixel lies
    /// in an array of `len` elements.
    fn check(&self, format: Format, len: usize) -> Result<()> {
        // The last pixel has the largest index. A packed format counts it
        // in bits, which overflows i64 for the largest Java arguments, so
        // it is worked out in 128 bits here; every position `position`
        // computes afterwards is then below `len` bytes in bits.
        let last = match format {
            Format::Packed { bits, .. } => {
                let pixel = i128::from(self.offset)
                    + i128::from(self.height - 1) * i128::from(self.scan)
                    + i128::from(self.width - 1);
                pixel * i128::from(bits) / 8
            }
            _ => self.position(format, self.width - 1, self.height - 1).0 as i128,
        };
        if last < len as i128 {
            Ok(())
        } else {
            Err(GfxError::ArrayIndexOutOfBounds)
        }
    }
}

/// `0xAARRGGBB` from a 16-bit Nokia pixel.
fn from_short(format: i32, v: u16, alpha: bool) -> u32 {
    let v = u32::from(v);
    let nibble = |shift: u32| ((v >> shift) & 0xF) * 0x11;
    let five = |shift: u32| {
        let c = (v >> shift) & 0x1F;
        (c << 3) | (c >> 2)
    };
    let (a, r, g, b) = match format {
        TYPE_USHORT_4444_ARGB => (nibble(12), nibble(8), nibble(4), nibble(0)),
        TYPE_USHORT_565_RGB => {
            let g = (v >> 5) & 0x3F;
            (0xFF, five(11), (g << 2) | (g >> 4), five(0))
        }
        TYPE_USHORT_555_RGB => (0xFF, five(10), five(5), five(0)),
        TYPE_USHORT_1555_ARGB => (
            if v & 0x8000 == 0 { 0 } else { 0xFF },
            five(10),
            five(5),
            five(0),
        ),
        _ => (0xFF, nibble(8), nibble(4), nibble(0)),
    };
    let a = if alpha { a } else { 0xFF };
    a << 24 | r << 16 | g << 8 | b
}

/// A 16-bit Nokia pixel from `0xAARRGGBB`.
fn to_short(format: i32, p: u32) -> u16 {
    let ch = |shift: u32, bits: u32| ((p >> shift) & 0xFF) >> (8 - bits);
    let v = match format {
        TYPE_USHORT_4444_ARGB => ch(24, 4) << 12 | ch(16, 4) << 8 | ch(8, 4) << 4 | ch(0, 4),
        TYPE_USHORT_565_RGB => ch(16, 5) << 11 | ch(8, 6) << 5 | ch(0, 5),
        TYPE_USHORT_555_RGB => ch(16, 5) << 10 | ch(8, 5) << 5 | ch(0, 5),
        TYPE_USHORT_1555_ARGB => ch(24, 1) << 15 | ch(16, 5) << 10 | ch(8, 5) << 5 | ch(0, 5),
        _ => ch(16, 4) << 8 | ch(8, 4) << 4 | ch(0, 4),
    };
    v as u16
}

/// The colour of a packed byte-format value. One-bit pixels are black when
/// set, as on Nokia's monochrome displays; deeper grey levels run from
/// black at zero to white at their maximum.
fn from_packed(format: Format, value: u32) -> u32 {
    match format {
        Format::Packed { rgb332: true, .. } => {
            let r = value >> 5;
            let g = (value >> 2) & 7;
            let b = value & 3;
            OPAQUE | ((r * 0x49) >> 1) << 16 | ((g * 0x49) >> 1) << 8 | (b * 0x55)
        }
        Format::Packed { bits: 1, .. } | Format::Vertical => {
            if value == 0 {
                0xFFFF_FFFF
            } else {
                OPAQUE
            }
        }
        _ => {
            let max = (1 << format.bits()) - 1;
            OPAQUE | ((value * 255 / max) * 0x0001_0101)
        }
    }
}

/// A packed byte-format value for `0xAARRGGBB`.
fn to_packed(format: Format, p: u32) -> u32 {
    match format {
        Format::Packed { rgb332: true, .. } => {
            ((p >> 16) & 0xE0) | ((p >> 11) & 0x1C) | ((p >> 6) & 0x03)
        }
        Format::Packed { bits: 1, .. } | Format::Vertical => u32::from(gray_of(p) < 128),
        _ => u32::from(gray_of(p)) >> (8 - format.bits()),
    }
}

impl Graphics {
    /// `drawPixels(int[] pixels, boolean transparency, offset, scanlength,
    /// x, y, width, height, manipulation, format)` in `TYPE_INT_888_RGB`
    /// or `TYPE_INT_8888_ARGB`. Alpha counts only for 8888 with
    /// `transparency`; the flips and rotations of `manipulation` apply to
    /// the block, whose top left lands at `(x, y)`.
    ///
    /// Negative sizes, a `scanlength` below `width`, an unknown
    /// manipulation or a format of another array type are
    /// `IllegalArgumentException`; pixels outside `pixels` are
    /// `ArrayIndexOutOfBoundsException`.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_pixels_int(
        &self,
        target: &mut Surface,
        pixels: &[i32],
        transparency: bool,
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        manipulation: i32,
        format: i32,
    ) -> Result<()> {
        let format = Format::int(format)?;
        let alpha = transparency && format == Format::Int { alpha: true };
        self.draw_pixels(
            target,
            pixels.len(),
            format,
            alpha,
            (offset, scanlength),
            (x, y, width, height),
            manipulation,
            |i, _| pixels[i] as u32,
        )
    }

    /// `drawPixels(short[] pixels, boolean transparency, ...)` in
    /// `TYPE_USHORT_444_RGB`, `4444_ARGB`, `565_RGB`, `555_RGB` or
    /// `1555_ARGB`. Checks as [`Graphics::draw_pixels_int`].
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_pixels_short(
        &self,
        target: &mut Surface,
        pixels: &[i16],
        transparency: bool,
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        manipulation: i32,
        format: i32,
    ) -> Result<()> {
        let code = Format::short(format)?;
        let alpha = transparency && matches!(code, TYPE_USHORT_4444_ARGB | TYPE_USHORT_1555_ARGB);
        self.draw_pixels(
            target,
            pixels.len(),
            Format::Short(code),
            alpha,
            (offset, scanlength),
            (x, y, width, height),
            manipulation,
            |i, _| from_short(code, pixels[i] as u16, alpha),
        )
    }

    /// `drawPixels(byte[] pixels, byte[] transparencyMask, ...)` in the
    /// grey formats, `TYPE_BYTE_332_RGB` or `TYPE_BYTE_1_GRAY_VERTICAL`.
    /// `offset` and `scanlength` count pixels, not bytes. The mask packs
    /// like the pixels: zero is transparent, anything else opaque; without
    /// a mask every pixel is opaque. Checks as
    /// [`Graphics::draw_pixels_int`], with the mask checked like the
    /// pixels.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_pixels_byte(
        &self,
        target: &mut Surface,
        pixels: &[i8],
        mask: Option<&[i8]>,
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        manipulation: i32,
        format: i32,
    ) -> Result<()> {
        let format = Format::byte(format)?;
        let len = mask.map_or(pixels.len(), |m| m.len().min(pixels.len()));
        let field = |array: &[i8], i: usize, shift: u32| {
            (u32::from(array[i] as u8) >> shift) & ((1 << format.bits()) - 1)
        };
        self.draw_pixels(
            target,
            len,
            format,
            mask.is_some(),
            (offset, scanlength),
            (x, y, width, height),
            manipulation,
            |i, shift| {
                let colour = from_packed(format, field(pixels, i, shift));
                match mask {
                    Some(mask) if field(mask, i, shift) == 0 => 0,
                    _ => colour,
                }
            },
        )
    }

    /// Converts a region of a Nokia pixel array with `read(index, shift)`
    /// and draws it.
    #[allow(clippy::too_many_arguments)]
    fn draw_pixels(
        &self,
        target: &mut Surface,
        len: usize,
        format: Format,
        alpha: bool,
        (offset, scanlength): (i32, i32),
        (x, y, width, height): (i32, i32, i32, i32),
        manipulation: i32,
        read: impl Fn(usize, u32) -> u32,
    ) -> Result<()> {
        let Some(region) = Region::new(offset, scanlength, width, height)? else {
            return Ok(());
        };
        let transform = manipulation_transform(manipulation)?;
        region.check(format, len)?;
        let mut argb = Vec::with_capacity((region.width * region.height) as usize);
        for row in 0..region.height {
            for col in 0..region.width {
                let (i, shift) = region.position(format, col, row);
                argb.push(read(i, shift));
            }
        }
        let src = Source {
            pixels: &argb,
            origin: 0,
            stride: region.width,
            width: region.width,
            height: region.height,
        };
        let (left, top) = self.at(x, y);
        let mode = if alpha {
            BlitMode::Blend
        } else {
            BlitMode::Opaque
        };
        blit(&mut self.canvas(target), &src, transform, left, top, mode);
        Ok(())
    }

    /// `getPixels(int[] pixels, offset, scanlength, x, y, width, height,
    /// format)`: reads the target's pixels at translated `(x, y)`,
    /// ignoring the clip. `TYPE_INT_888_RGB` clears the alpha byte.
    /// Pixels outside the surface leave their array entries alone.
    ///
    /// Negative position or size, a `scanlength` below `width` or another
    /// format are `IllegalArgumentException`; an array too small is
    /// `ArrayIndexOutOfBoundsException`, with the array unchanged.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn get_pixels_int(
        &self,
        source: &Surface,
        pixels: &mut [i32],
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> Result<()> {
        let format = Format::int(format)?;
        let keep_alpha = format == Format::Int { alpha: true };
        let len = pixels.len();
        self.get_pixels(
            source,
            len,
            format,
            (offset, scanlength),
            (x, y, width, height),
            |i, _, p| {
                pixels[i] = (if keep_alpha { p } else { p & 0x00FF_FFFF }) as i32;
            },
        )
    }

    /// `getPixels(short[] pixels, ...)` in a `TYPE_USHORT_*` format.
    /// Checks as [`Graphics::get_pixels_int`].
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn get_pixels_short(
        &self,
        source: &Surface,
        pixels: &mut [i16],
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> Result<()> {
        let code = Format::short(format)?;
        let len = pixels.len();
        self.get_pixels(
            source,
            len,
            Format::Short(code),
            (offset, scanlength),
            (x, y, width, height),
            |i, _, p| {
                pixels[i] = to_short(code, p) as i16;
            },
        )
    }

    /// `getPixels(byte[] pixels, byte[] transparencyMask, ...)` in a byte
    /// format. Only the bits of the pixels read change. Pixels less than
    /// half opaque read as zero; with a mask, each pixel's mask field is
    /// set to all ones where the pixel is at least half opaque and cleared
    /// elsewhere. Checks as [`Graphics::get_pixels_int`], the mask like
    /// the pixels.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn get_pixels_byte(
        &self,
        source: &Surface,
        pixels: &mut [i8],
        mut mask: Option<&mut [i8]>,
        offset: i32,
        scanlength: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: i32,
    ) -> Result<()> {
        let format = Format::byte(format)?;
        let len = mask
            .as_ref()
            .map_or(pixels.len(), |m| m.len().min(pixels.len()));
        let field = (1_u32 << format.bits()) - 1;
        let store = |array: &mut [i8], i: usize, shift: u32, value: u32| {
            let byte = (u32::from(array[i] as u8) & !(field << shift)) | ((value & field) << shift);
            array[i] = byte as u8 as i8;
        };
        self.get_pixels(
            source,
            len,
            format,
            (offset, scanlength),
            (x, y, width, height),
            |i, shift, p| {
                let opaque = p >> 31 == 1;
                let value = if opaque { to_packed(format, p) } else { 0 };
                store(pixels, i, shift, value);
                if let Some(mask) = mask.as_deref_mut() {
                    store(mask, i, shift, if opaque { field } else { 0 });
                }
            },
        )
    }

    /// Checks a `getPixels` call and hands each pixel inside the surface to
    /// `write(index, shift, argb)`.
    fn get_pixels(
        &self,
        source: &Surface,
        len: usize,
        format: Format,
        (offset, scanlength): (i32, i32),
        (x, y, width, height): (i32, i32, i32, i32),
        mut write: impl FnMut(usize, u32, u32),
    ) -> Result<()> {
        if x < 0 || y < 0 {
            return Err(GfxError::IllegalArgument("negative position"));
        }
        let Some(region) = Region::new(offset, scanlength, width, height)? else {
            return Ok(());
        };
        region.check(format, len)?;
        let (left, top) = self.at(x, y);
        for row in 0..region.height {
            for col in 0..region.width {
                let (px, py) = (left + col, top + row);
                let Some(p) = u32::try_from(px)
                    .ok()
                    .zip(u32::try_from(py).ok())
                    .and_then(|(px, py)| source.pixel(px, py))
                else {
                    continue;
                };
                let (i, shift) = region.position(format, col, row);
                write(i, shift, p);
            }
        }
        Ok(())
    }

    /// `DirectGraphics.drawImage(img, x, y, anchor, manipulation)`: the
    /// whole image flipped and rotated as Nokia's manipulation says, then
    /// anchored by its new size. Anchors follow `drawImage`'s rules.
    #[allow(clippy::too_many_arguments)] // Nokia's signature
    pub fn draw_image_manipulated(
        &self,
        target: &mut Surface,
        image: &Surface,
        x: i32,
        y: i32,
        anchor: i32,
        manipulation: i32,
    ) -> Result<()> {
        let transform = manipulation_transform(manipulation)?;
        // Images never exceed MAX_PIXELS, so their sides fit an i32.
        let (w, h) = (image.width() as i32, image.height() as i32);
        self.draw_region(target, image, 0, 0, w, h, transform.to_midp(), x, y, anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nokia::{FLIP_HORIZONTAL, FLIP_VERTICAL, ROTATE_90};

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = crate::testing::graphics(&surface);
        (surface, g)
    }

    #[test]
    fn short_formats_round_trip_through_argb() {
        for (format, value) in [
            (TYPE_USHORT_444_RGB, 0x0A5F_u16),
            (TYPE_USHORT_4444_ARGB, 0x8A5F),
            (TYPE_USHORT_565_RGB, 0xF81F),
            (TYPE_USHORT_555_RGB, 0x7C1F),
            (TYPE_USHORT_1555_ARGB, 0x83E0),
        ] {
            assert_eq!(
                to_short(format, from_short(format, value, true)),
                value,
                "{format}"
            );
        }
        assert_eq!(from_short(TYPE_USHORT_565_RGB, 0xF800, true), 0xFFFF_0000);
        assert_eq!(from_short(TYPE_USHORT_4444_ARGB, 0x0F00, true), 0x00FF_0000);
        assert_eq!(
            from_short(TYPE_USHORT_4444_ARGB, 0x0F00, false),
            0xFFFF_0000
        );
        assert_eq!(from_short(TYPE_USHORT_444_RGB, 0x0FFF, true), 0xFFFF_FFFF);
    }

    #[test]
    fn packed_values_map_to_colours() {
        let one = Format::byte(TYPE_BYTE_1_GRAY).unwrap();
        assert_eq!(from_packed(one, 1), OPAQUE);
        assert_eq!(from_packed(one, 0), 0xFFFF_FFFF);
        let two = Format::byte(TYPE_BYTE_2_GRAY).unwrap();
        assert_eq!(from_packed(two, 3), 0xFFFF_FFFF);
        assert_eq!(from_packed(two, 1), 0xFF55_5555);
        let rgb = Format::byte(TYPE_BYTE_332_RGB).unwrap();
        assert_eq!(from_packed(rgb, 0xFF), 0xFFFF_FFFF);
        assert_eq!(from_packed(rgb, 0xE0), 0xFFFF_0000);
        assert_eq!(to_packed(rgb, 0xFFFF_0000), 0xE0);
        assert_eq!(to_packed(one, 0xFF00_0000), 1);
        assert_eq!(
            to_packed(Format::byte(TYPE_BYTE_4_GRAY).unwrap(), 0xFFFF_FFFF),
            15
        );
    }

    #[test]
    fn int_pixels_draw_with_offset_scanlength_and_alpha() {
        let pixels = [0, 0x11, 0x22, 0, 0x8000_0000_u32 as i32, 0x44];
        let (mut s, g) = canvas(2, 2);
        g.draw_pixels_int(
            &mut s,
            &pixels,
            true,
            1,
            3,
            0,
            0,
            2,
            2,
            0,
            TYPE_INT_8888_ARGB,
        )
        .unwrap();
        // 0x11 and 0x22 have zero alpha: nothing; 0x80000000 blends.
        assert_eq!(
            s.pixels(),
            &[0xFFFF_FFFF, 0xFFFF_FFFF, 0xFF7F_7F7F, 0xFFFF_FFFF]
        );
        let (mut s, g) = canvas(2, 2);
        g.draw_pixels_int(&mut s, &pixels, true, 1, 3, 0, 0, 2, 2, 0, TYPE_INT_888_RGB)
            .unwrap();
        assert_eq!(
            s.pixels(),
            &[0xFF00_0011, 0xFF00_0022, 0xFF00_0000, 0xFF00_0044]
        );
    }

    #[test]
    fn draw_pixels_checks_its_arguments() {
        let (mut s, g) = canvas(2, 2);
        let px = [0; 4];
        let iae = |r| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(g.draw_pixels_int(
            &mut s,
            &px,
            false,
            0,
            2,
            0,
            0,
            2,
            2,
            0,
            TYPE_USHORT_565_RGB
        )));
        assert!(iae(g.draw_pixels_int(
            &mut s,
            &px,
            false,
            0,
            1,
            0,
            0,
            2,
            2,
            0,
            TYPE_INT_888_RGB
        )));
        assert!(iae(g.draw_pixels_int(
            &mut s,
            &px,
            false,
            0,
            2,
            0,
            0,
            -1,
            2,
            0,
            TYPE_INT_888_RGB
        )));
        assert!(iae(g.draw_pixels_int(
            &mut s,
            &px,
            false,
            0,
            2,
            0,
            0,
            2,
            2,
            45,
            TYPE_INT_888_RGB
        )));
        assert!(iae(g.draw_pixels_short(
            &mut s,
            &[0; 4],
            false,
            0,
            2,
            0,
            0,
            2,
            2,
            0,
            TYPE_INT_888_RGB
        )));
        assert!(iae(g.draw_pixels_byte(
            &mut s,
            &[0; 4],
            None,
            0,
            2,
            0,
            0,
            2,
            2,
            0,
            TYPE_USHORT_444_RGB
        )));
        let oob = Err(GfxError::ArrayIndexOutOfBounds);
        assert_eq!(
            g.draw_pixels_int(&mut s, &px, false, -1, 2, 0, 0, 2, 2, 0, TYPE_INT_888_RGB),
            oob
        );
        assert_eq!(
            g.draw_pixels_int(&mut s, &px, false, 1, 2, 0, 0, 2, 2, 0, TYPE_INT_888_RGB),
            oob
        );
        assert_eq!(
            g.draw_pixels_byte(
                &mut s,
                &[0],
                Some(&[]),
                0,
                8,
                0,
                0,
                8,
                1,
                0,
                TYPE_BYTE_1_GRAY
            ),
            oob
        );
        assert_eq!(
            g.draw_pixels_int(&mut s, &px, false, 9, 2, 0, 0, 0, 2, 0, TYPE_INT_888_RGB),
            Ok(())
        );
    }

    #[test]
    fn one_bit_pixels_use_masks() {
        // Row of 8: pixels 1100_0000, mask 1010_0000.
        let (mut s, g) = canvas(8, 1);
        g.draw_pixels_byte(
            &mut s,
            &[0b1100_0000_u8 as i8],
            Some(&[0b1010_0000_u8 as i8]),
            0,
            8,
            0,
            0,
            8,
            1,
            0,
            TYPE_BYTE_1_GRAY,
        )
        .unwrap();
        assert_eq!(&s.pixels()[..3], &[OPAQUE, 0xFFFF_FFFF, 0xFFFF_FFFF]);
        let (mut s, g) = canvas(8, 1);
        g.draw_pixels_byte(
            &mut s,
            &[0b0100_0000],
            None,
            0,
            8,
            0,
            0,
            8,
            1,
            0,
            TYPE_BYTE_1_GRAY,
        )
        .unwrap();
        assert_eq!(&s.pixels()[..2], &[0xFFFF_FFFF, OPAQUE]);
    }

    #[test]
    fn vertical_bytes_hold_columns() {
        // Two columns of 8 pixels: column 0 has its top pixel set (low
        // bit), column 1 its bottom pixel.
        let (mut s, g) = canvas(2, 8);
        g.draw_pixels_byte(
            &mut s,
            &[0b0000_0001, 0b1000_0000_u8 as i8],
            None,
            0,
            2,
            0,
            0,
            2,
            8,
            0,
            TYPE_BYTE_1_GRAY_VERTICAL,
        )
        .unwrap();
        assert_eq!(s.pixel(0, 0), Some(OPAQUE));
        assert_eq!(s.pixel(1, 7), Some(OPAQUE));
        assert_eq!(s.pixel(0, 7), Some(0xFFFF_FFFF));
        let mut back = [0_i8; 2];
        g.get_pixels_byte(
            &s,
            &mut back,
            None,
            0,
            2,
            0,
            0,
            2,
            8,
            TYPE_BYTE_1_GRAY_VERTICAL,
        )
        .unwrap();
        assert_eq!(back, [0b0000_0001, 0b1000_0000_u8 as i8]);
    }

    #[test]
    fn huge_packed_regions_are_out_of_bounds_not_overflows() {
        // The last pixel's bit index, (2^31 - 2) * (2^31 - 1) * 8, does
        // not fit an i64.
        let (mut s, g) = canvas(2, 2);
        for format in [TYPE_BYTE_8_GRAY, TYPE_BYTE_4_GRAY, TYPE_BYTE_332_RGB] {
            let oob = Err(GfxError::ArrayIndexOutOfBounds);
            let drawn = g.draw_pixels_byte(
                &mut s,
                &[0; 16],
                None,
                0,
                i32::MAX,
                0,
                0,
                1,
                i32::MAX,
                0,
                format,
            );
            assert_eq!(drawn, oob, "{format}");
            let mut out = [0_i8; 16];
            let read =
                g.get_pixels_byte(&s, &mut out, None, 0, i32::MAX, 0, 0, 1, i32::MAX, format);
            assert_eq!(read, oob, "{format}");
        }
    }

    #[test]
    fn get_pixels_reads_translated_and_round_trips() {
        let (mut s, mut g) = canvas(3, 2);
        let shorts = [0x0F00, 0x00F0, 0x000F, 0x0FFF, 0x0000, 0x0888];
        g.draw_pixels_short(
            &mut s,
            &shorts,
            false,
            0,
            3,
            0,
            0,
            3,
            2,
            0,
            TYPE_USHORT_444_RGB,
        )
        .unwrap();
        let mut back = [0_i16; 6];
        g.get_pixels_short(&s, &mut back, 0, 3, 0, 0, 3, 2, TYPE_USHORT_444_RGB)
            .unwrap();
        assert_eq!(back, shorts);
        g.translate(1, 1);
        let mut one = [7_i32; 2];
        g.get_pixels_int(&s, &mut one, 0, 2, 0, 0, 2, 1, TYPE_INT_888_RGB)
            .unwrap();
        assert_eq!(one, [0, 0x88_8888]);
        // Off the surface: entries stay.
        let mut off = [7_i32; 2];
        g.get_pixels_int(&s, &mut off, 0, 2, 2, 0, 2, 1, TYPE_INT_8888_ARGB)
            .unwrap();
        assert_eq!(off, [7, 7]);
        let iae = |r| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(g.get_pixels_int(
            &s,
            &mut one,
            0,
            2,
            -1,
            0,
            1,
            1,
            TYPE_INT_888_RGB
        )));
        assert_eq!(
            g.get_pixels_int(&s, &mut one, 1, 2, 0, 0, 2, 1, TYPE_INT_888_RGB),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
    }

    #[test]
    fn get_pixels_packs_bits_and_masks() {
        let mut s = Surface::new_mutable_filled(4, 1, 0).unwrap();
        let g = crate::testing::graphics(&s);
        g.draw_pixels_int(
            &mut s,
            &[-1, 0xFF00_0000_u32 as i32],
            false,
            0,
            2,
            1,
            0,
            2,
            1,
            0,
            TYPE_INT_888_RGB,
        )
        .unwrap();
        // Pixels: clear, white, black, clear. Other bits of the bytes stay.
        let mut px = [0b0000_1111];
        let mut mask = [0b0000_1111];
        g.get_pixels_byte(
            &s,
            &mut px,
            Some(&mut mask),
            0,
            4,
            0,
            0,
            4,
            1,
            TYPE_BYTE_1_GRAY,
        )
        .unwrap();
        assert_eq!(px, [0b0010_1111]);
        assert_eq!(mask, [0b0110_1111]);
        let mut grey = [0_i8; 1];
        g.get_pixels_byte(&s, &mut grey, None, 0, 4, 0, 0, 4, 1, TYPE_BYTE_2_GRAY)
            .unwrap();
        assert_eq!(grey[0] as u8, 0b0011_0000);
    }

    #[test]
    fn manipulations_flip_and_rotate_the_block() {
        // A 2x1 block: red, green.
        let pixels = [0xFF_0000, 0x00_FF00];
        let draw = |manipulation| {
            let (mut s, g) = canvas(2, 2);
            g.draw_pixels_int(
                &mut s,
                &pixels,
                false,
                0,
                2,
                0,
                0,
                2,
                1,
                manipulation,
                TYPE_INT_888_RGB,
            )
            .unwrap();
            s
        };
        let flipped = draw(FLIP_HORIZONTAL);
        assert_eq!(&flipped.pixels()[..2], &[0xFF00_FF00, 0xFFFF_0000]);
        // A quarter turn counter-clockwise stands the block up with green
        // on top.
        let turned = draw(ROTATE_90);
        assert_eq!(
            (turned.pixel(0, 0), turned.pixel(0, 1)),
            (Some(0xFF00_FF00), Some(0xFFFF_0000))
        );
        let both = draw(ROTATE_90 | FLIP_VERTICAL);
        assert_eq!(
            (both.pixel(0, 0), both.pixel(0, 1)),
            (Some(0xFFFF_0000), Some(0xFF00_FF00))
        );
    }

    #[test]
    fn draw_image_manipulated_anchors_the_turned_image() {
        let image = Surface::from_rgb(&[1, 2], 2, 1, false).unwrap();
        let (mut s, g) = canvas(3, 3);
        g.draw_image_manipulated(
            &mut s,
            &image,
            3,
            3,
            crate::anchor::RIGHT | crate::anchor::BOTTOM,
            ROTATE_90,
        )
        .unwrap();
        assert_eq!(s.pixel(2, 1), Some(OPAQUE | 2));
        assert_eq!(s.pixel(2, 2), Some(OPAQUE | 1));
        assert!(
            g.draw_image_manipulated(&mut s, &image, 0, 0, 0, 7)
                .is_err()
        );
    }
}
