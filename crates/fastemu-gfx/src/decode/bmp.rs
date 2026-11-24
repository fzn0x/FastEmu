//! Windows BMP: 1, 4, 8, 16, 24 and 32 bits per pixel, uncompressed or
//! with bit-field masks, bottom-up or top-down. Run-length compressed
//! files are refused.

use super::{check_size, failed};
use crate::color::OPAQUE;
use crate::error::Result;

const FILE_HEADER: u64 = 14;
const BI_RGB: u32 = 0;
const BI_BITFIELDS: u32 = 3;
const BI_ALPHABITFIELDS: u32 = 6;

/// Channel masks for 16 and 32-bit pixels.
#[derive(Clone, Copy)]
struct Masks {
    red: u32,
    green: u32,
    blue: u32,
    alpha: u32,
}

impl Masks {
    fn argb(self, v: u32) -> u32 {
        let alpha = if self.alpha == 0 {
            255
        } else {
            channel(v, self.alpha)
        };
        alpha << 24
            | channel(v, self.red) << 16
            | channel(v, self.green) << 8
            | channel(v, self.blue)
    }
}

/// The bits of `v` under `mask`, scaled to 0 to 255.
fn channel(v: u32, mask: u32) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = u64::from(mask >> shift);
    let raw = u64::from((v & mask) >> shift);
    ((raw * 255 + max / 2) / max) as u32
}

pub(super) fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u32>)> {
    // Offsets come from the file and are worked out in u64, so a header
    // near 4 GiB cannot overflow a 32-bit usize. Bytes past the end of
    // the file read as zero.
    let byte = |i: u64| {
        usize::try_from(i)
            .ok()
            .and_then(|i| data.get(i))
            .copied()
            .unwrap_or(0)
    };
    let u16_at = |i: u64| u16::from_le_bytes([byte(i), byte(i + 1)]);
    let u32_at = |i: u64| u32::from_le_bytes([byte(i), byte(i + 1), byte(i + 2), byte(i + 3)]);
    if data.len() < FILE_HEADER as usize + 12 {
        return Err(failed("BMP", "the file is too short"));
    }
    let pixel_offset = u64::from(u32_at(10));
    let header = u64::from(u32_at(FILE_HEADER));
    let (width, height, bits, compression, colours, entry) = match header {
        12 => (
            i64::from(u16_at(18)),
            i64::from(u16_at(20) as i16),
            u16_at(24),
            BI_RGB,
            0,
            3,
        ),
        40.. => (
            i64::from(u32_at(18) as i32),
            i64::from(u32_at(22) as i32),
            u16_at(28),
            u32_at(30),
            u64::from(u32_at(46)),
            4,
        ),
        _ => return Err(failed("BMP", "unknown header")),
    };
    let top_down = height < 0;
    let height = height.abs();
    if width <= 0 || height <= 0 || width > i64::from(u32::MAX) || height > i64::from(u32::MAX) {
        return Err(failed("BMP", "the image is empty"));
    }
    let (w, h) = (width as u32, height as u32);
    check_size(w, h)?;
    if !matches!(bits, 1 | 4 | 8 | 16 | 24 | 32) {
        return Err(failed("BMP", "unsupported bit depth"));
    }
    let masks_at = FILE_HEADER + 40;
    let masks = match (compression, bits) {
        (BI_RGB, 16) => Masks {
            red: 0x7C00,
            green: 0x03E0,
            blue: 0x001F,
            alpha: 0,
        },
        (BI_RGB, _) => Masks {
            red: 0x00FF_0000,
            green: 0x0000_FF00,
            blue: 0x0000_00FF,
            alpha: 0,
        },
        (BI_BITFIELDS | BI_ALPHABITFIELDS, 16 | 32) => Masks {
            red: u32_at(masks_at),
            green: u32_at(masks_at + 4),
            blue: u32_at(masks_at + 8),
            alpha: if header >= 56 || compression == BI_ALPHABITFIELDS {
                u32_at(masks_at + 12)
            } else {
                0
            },
        },
        _ => return Err(failed("BMP", "compressed files are not supported")),
    };
    let palette: Vec<u32> = if bits <= 8 {
        let extra_masks = match (header, compression) {
            (40, BI_BITFIELDS) => 12,
            (40, BI_ALPHABITFIELDS) => 16,
            _ => 0,
        };
        let start = FILE_HEADER + header + extra_masks;
        let count = if colours > 0 {
            colours.min(256)
        } else {
            1 << bits
        };
        (0..count)
            .map(|i: u64| {
                let at = start + i * entry;
                OPAQUE
                    | u32::from(byte(at + 2)) << 16
                    | u32::from(byte(at + 1)) << 8
                    | u32::from(byte(at))
            })
            .collect()
    } else {
        Vec::new()
    };
    let row_bytes = (u64::from(w) * u64::from(bits)).div_ceil(32) * 4;
    let mut pixels = Vec::with_capacity(w as usize * h as usize);
    for y in 0..u64::from(h) {
        let row = if top_down { y } else { u64::from(h) - 1 - y };
        let base = pixel_offset + row * row_bytes;
        for x in 0..u64::from(w) {
            let pixel = match bits {
                1 | 4 | 8 => {
                    let bit = x * u64::from(bits);
                    let shift = 8 - u64::from(bits) - bit % 8;
                    let index = (byte(base + bit / 8) >> shift) & ((1_u16 << bits) - 1) as u8;
                    palette.get(usize::from(index)).copied().unwrap_or(OPAQUE)
                }
                16 => masks.argb(u16_at(base + x * 2).into()),
                24 => {
                    let at = base + x * 3;
                    OPAQUE
                        | u32::from(byte(at + 2)) << 16
                        | u32::from(byte(at + 1)) << 8
                        | u32::from(byte(at))
                }
                32 => masks.argb(u32_at(base + x * 4)),
                _ => return Err(failed("BMP", "unsupported bit depth")),
            };
            pixels.push(pixel);
        }
    }
    // Many tools write 32-bit files with an alpha mask but no alpha at
    // all; showing those as invisible helps nobody.
    if masks.alpha != 0 && pixels.iter().all(|&p| p >> 24 == 0) {
        for p in &mut pixels {
            *p |= OPAQUE;
        }
    }
    Ok((w, h, pixels))
}

#[cfg(test)]
mod tests {
    use crate::color::OPAQUE;
    use crate::error::GfxError;
    use crate::surface::Surface;

    /// A BMP with a 40-byte header, `extra` bytes (masks or palette) after
    /// it and then the pixel rows as given.
    fn bmp(
        width: i32,
        height: i32,
        bits: u16,
        compression: u32,
        extra: &[u8],
        rows: &[u8],
    ) -> Vec<u8> {
        let offset = 14 + 40 + extra.len() as u32;
        let mut out = b"BM".to_vec();
        out.extend_from_slice(&(offset + rows.len() as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&40_u32.to_le_bytes());
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(&compression.to_le_bytes());
        out.extend_from_slice(&[0; 20]);
        out.extend_from_slice(extra);
        out.extend_from_slice(rows);
        out
    }

    #[test]
    fn rgb_bmps_are_bottom_up_unless_height_is_negative() {
        // Two rows of one pixel, padded to four bytes, stored bottom row
        // first: blue, then red.
        let rows = [255, 0, 0, 0, 0, 0, 255, 0];
        let image = Surface::decode(&bmp(1, 2, 24, 0, &[], &rows)).unwrap();
        assert_eq!(image.pixels(), &[0xFFFF_0000, 0xFF00_00FF]);
        let image = Surface::decode(&bmp(1, -2, 24, 0, &[], &rows)).unwrap();
        assert_eq!(image.pixels(), &[0xFF00_00FF, 0xFFFF_0000]);
    }

    #[test]
    fn palette_bmps_decode_at_each_depth() {
        let palette = [0, 0, 0, 0, 255, 255, 255, 0, 0, 0, 255, 0];
        let one =
            Surface::decode(&bmp(3, 1, 1, 0, &palette[..8], &[0b0100_0000, 0, 0, 0])).unwrap();
        assert_eq!(one.pixels(), &[0xFF00_0000, 0xFFFF_FFFF, 0xFF00_0000]);
        let four = Surface::decode(&bmp(3, 1, 4, 0, &palette, &[0x21, 0x00, 0, 0])).unwrap();
        assert_eq!(four.pixels(), &[0xFFFF_0000, 0xFFFF_FFFF, 0xFF00_0000]);
        let eight = Surface::decode(&bmp(2, 1, 8, 0, &palette, &[2, 1, 0, 0])).unwrap();
        assert_eq!(eight.pixels(), &[0xFFFF_0000, 0xFFFF_FFFF]);
    }

    #[test]
    fn sixteen_and_thirty_two_bit_bmps_use_masks() {
        // 5-5-5 by default: pure red is 0x7C00.
        let image = Surface::decode(&bmp(1, 1, 16, 0, &[], &[0x00, 0x7C, 0, 0])).unwrap();
        assert_eq!(image.pixels(), &[0xFFFF_0000]);
        // 5-6-5 through masks: pure green is 0x07E0.
        let masks = [0x00, 0xF8, 0, 0, 0xE0, 0x07, 0, 0, 0x1F, 0, 0, 0];
        let image = Surface::decode(&bmp(1, 1, 16, 3, &masks, &[0xE0, 0x07, 0, 0])).unwrap();
        assert_eq!(image.pixels(), &[0xFF00_FF00]);
        // 32-bit BGRX without masks is opaque.
        let image = Surface::decode(&bmp(1, 1, 32, 0, &[], &[1, 2, 3, 0])).unwrap();
        assert_eq!(image.pixels(), &[0xFF03_0201]);
        // 32-bit with an alpha mask keeps alpha.
        let masks = [0, 0, 0xFF, 0, 0, 0xFF, 0, 0, 0xFF, 0, 0, 0, 0, 0, 0, 0xFF];
        let image = Surface::decode(&bmp(1, 1, 32, 6, &masks, &[1, 2, 3, 0x80])).unwrap();
        assert_eq!(image.pixels(), &[0x8003_0201]);
    }

    #[test]
    fn unsupported_and_broken_bmps_fail() {
        let rle = bmp(1, 1, 8, 1, &[0; 4], &[0; 4]);
        assert!(matches!(Surface::decode(&rle), Err(GfxError::Decode(_))));
        assert!(matches!(
            Surface::decode(b"BM\0\0"),
            Err(GfxError::Decode(_))
        ));
        assert!(matches!(
            Surface::decode(&bmp(0, 1, 24, 0, &[], &[])),
            Err(GfxError::Decode(_))
        ));
        // Missing pixel bytes read as zero rather than failing.
        let short = Surface::decode(&bmp(2, 2, 24, 0, &[], &[9, 9, 9])).unwrap();
        assert_eq!(short.pixel(0, 1), Some(0xFF09_0909));
        assert_eq!(short.pixel(1, 0), Some(0xFF00_0000));
    }

    #[test]
    fn offsets_near_four_gib_read_as_missing_bytes() {
        // A pixel offset and a header size of nearly 4 GiB overflow a
        // 32-bit usize when added to; the pixels and palette they point
        // at are simply past the end of the file.
        let mut far = bmp(1, 1, 24, 0, &[], &[1, 2, 3, 0]);
        far[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(Surface::decode(&far).unwrap().pixel(0, 0), Some(OPAQUE));
        let mut huge_header = bmp(1, 1, 8, 0, &[], &[0; 4]);
        huge_header[14..18].copy_from_slice(&u32::MAX.to_le_bytes());
        let image = Surface::decode(&huge_header).unwrap();
        assert_eq!(image.pixel(0, 0), Some(OPAQUE));
        let odd_depth = bmp(1, 1, 2, 0, &[], &[0; 4]);
        assert!(matches!(
            Surface::decode(&odd_depth),
            Err(GfxError::Decode(_))
        ));
    }
}
