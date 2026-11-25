//! Decoding the image files games ship: PNG (which MIDP requires), JPEG,
//! GIF and BMP. Every decoded image is an immutable [`Surface`].
//!
//! Games shipped many slightly broken files that phones displayed anyway,
//! so the decoders lean towards showing something: PNG chunks with bad
//! checksums, garbage after the image, a missing end chunk and truncated
//! image data all still decode. Anything that cannot be read fails with
//! [`GfxError::Decode`], which MIDP's `Image.createImage` reports as
//! `IllegalArgumentException`.

mod bmp;
mod gif;
mod jpeg;
mod png;
mod probe;

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::error::{GfxError, Result};
use crate::surface::{MAX_PIXELS, Surface, SurfaceKind, java_range};

/// The largest width or height a decoder accepts.
const MAX_SIDE: u32 = 16384;

/// An image file format FastEmu decodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// PNG, the format MIDP requires.
    Png,
    /// JPEG, baseline or progressive.
    Jpeg,
    /// GIF; only the first frame is used.
    Gif,
    /// Windows BMP.
    Bmp,
}

impl ImageFormat {
    /// The format `data` is in, from its first bytes.
    pub fn sniff(data: &[u8]) -> Option<Self> {
        if data.starts_with(&png::SIGNATURE) {
            Some(Self::Png)
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if data.starts_with(b"BM") {
            Some(Self::Bmp)
        } else {
            None
        }
    }
}

/// The size, in pixels, a PNG, GIF, BMP or JPEG file declares in its
/// header, as `(width, height)`, found without decoding anything: a
/// caller can weigh what the image will need before [`Surface::decode`]
/// allocates it.
///
/// The size is the one the decoder goes by: a PNG's first IHDR chunk, a
/// GIF's logical screen (or its first frame, when the screen is empty), a
/// BMP's header without the sign of its height, and a JPEG's frame
/// header. The decoders' own code finds the PNG, GIF and JPEG sizes, run
/// only as far as the header, so a block or segment the decoder skips is
/// no size here either. `None` when the format is not one of these, the
/// header is cut short or the picture is empty. Decoding such a file
/// fails too, as it reads the same header first, so a caller can go on to
/// [`Surface::decode`] for the error. The size is only what the file
/// claims: decoding stays within its own limits (16 384 pixels a side and
/// [`MAX_PIXELS`] in all) whatever it says.
///
/// ```
/// use fastemu_gfx::probe_size;
///
/// // A BMP with the 12 byte header: 16 by 32 pixels, 24 bits.
/// let mut bmp = b"BM".to_vec();
/// bmp.extend_from_slice(&[0; 8]);
/// bmp.extend_from_slice(&26_u32.to_le_bytes());
/// bmp.extend_from_slice(&12_u32.to_le_bytes());
/// bmp.extend_from_slice(&[16, 0, 32, 0, 1, 0, 24, 0]);
/// assert_eq!(probe_size(&bmp), Some((16, 32)));
/// assert_eq!(probe_size(b"not an image"), None);
/// ```
pub fn probe_size(data: &[u8]) -> Option<(u32, u32)> {
    let format = ImageFormat::sniff(data)?;
    // The gif and zune-jpeg crates do the reading for their formats; a
    // panic in one is no size.
    guarded(|| {
        Ok(match format {
            ImageFormat::Png => png::probe(data),
            ImageFormat::Jpeg => jpeg::probe(data),
            ImageFormat::Gif => gif::probe(data),
            ImageFormat::Bmp => probe::bmp(data),
        })
    })
    .ok()
    .flatten()
}

/// The host bytes [`Surface::decode`] holds for the first frame of a GIF
/// whose frame has more pixels than the image [`probe_size`] gives. The
/// decoder reads that frame whole, at its own size, before it cuts it to
/// the image, so a caller that weighed only the image would let through a
/// file of 34 bytes that holds 160 MiB while it decodes. A caller weighs
/// these bytes too, before it decodes. Only the header is read.
///
/// `None` for every other file: a GIF whose first frame has no more
/// pixels than its image, any other format, and a file cut short before
/// its first frame. Those decoders hold a few times the image's own size
/// at most.
///
/// ```
/// use fastemu_gfx::{probe_frame_bytes, probe_size};
///
/// // 34 bytes: a screen of 1 by 1 pixel and a first frame of 8192 by
/// // 4096 whose data ends at once.
/// let mut gif = b"GIF89a".to_vec();
/// gif.extend_from_slice(&[1, 0, 1, 0, 0x80, 0, 0, 0, 0, 0, 255, 255, 255]);
/// gif.extend_from_slice(&[0x2C, 0, 0, 0, 0, 0, 0x20, 0, 0x10, 0]);
/// gif.extend_from_slice(&[2, 1, 0x2C, 0, 0x3B]);
/// assert_eq!(probe_size(&gif), Some((1, 1)));
/// assert_eq!(probe_frame_bytes(&gif), Some(8192 * 4096 * 5));
/// assert_eq!(probe_frame_bytes(b"not an image"), None);
/// ```
pub fn probe_frame_bytes(data: &[u8]) -> Option<u64> {
    match ImageFormat::sniff(data)? {
        // The gif crate reads the header; a panic in it is no size.
        ImageFormat::Gif => guarded(|| Ok(gif::frame_bytes(data))).ok().flatten(),
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Bmp => None,
    }
}

/// Runs a decode and turns a panic in it into a decode error. The decoders
/// are other people's code reading hostile files; the `png` crate has
/// already panicked on one palette, and a panic on a game's thread would
/// stop its whole virtual machine, or abort the launcher on a background
/// task. Needs `panic = "unwind"`, which every profile of FastEmu sets.
fn guarded<T>(decode: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(decode))
        .unwrap_or_else(|_| Err(GfxError::Decode("the image decoder failed".into())))
}

impl Surface {
    /// `Image.createImage(imageData, 0, imageData.length)`: decodes a PNG,
    /// JPEG, GIF (its first frame) or BMP file into an immutable image.
    /// A decoder that panics on a file is a [`GfxError::Decode`], as the
    /// file is not one it can read.
    pub fn decode(data: &[u8]) -> Result<Surface> {
        guarded(|| {
            let format = ImageFormat::sniff(data)
                .ok_or_else(|| GfxError::Decode("not a PNG, JPEG, GIF or BMP image".into()))?;
            let (width, height, pixels) = match format {
                ImageFormat::Png => png::decode(data),
                ImageFormat::Jpeg => jpeg::decode(data),
                ImageFormat::Gif => gif::decode(data),
                ImageFormat::Bmp => bmp::decode(data),
            }?;
            Surface::from_pixels(width, height, pixels, SurfaceKind::Immutable)
        })
    }

    /// `Image.createImage(imageData, imageOffset, imageLength)`. A range
    /// outside `data` is `ArrayIndexOutOfBoundsException`.
    pub fn decode_range(data: &[u8], offset: i32, length: i32) -> Result<Surface> {
        let range =
            java_range(data.len(), offset, length).ok_or(GfxError::ArrayIndexOutOfBounds)?;
        Self::decode(&data[range])
    }
}

/// Fails with `OutOfMemoryError` for images larger than FastEmu
/// allocates, and with a decode error for empty ones, before a decoder
/// allocates their pixels.
fn check_size(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(GfxError::Decode("the image is empty".into()));
    }
    if width > MAX_SIDE || height > MAX_SIDE || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(GfxError::OutOfMemory);
    }
    Ok(())
}

/// A decoder's error as a [`GfxError::Decode`] naming the format.
fn failed(format: &str, error: impl std::fmt::Display) -> GfxError {
    GfxError::Decode(format!("{format}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_bytes_fail_to_decode() {
        assert!(matches!(
            Surface::decode(b"hello"),
            Err(GfxError::Decode(_))
        ));
        assert!(matches!(Surface::decode(&[]), Err(GfxError::Decode(_))));
        assert_eq!(
            Surface::decode_range(b"abc", 2, 2),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
    }

    #[test]
    fn sizes_are_bounded() {
        assert!(check_size(16384, 2048).is_ok());
        assert_eq!(check_size(16385, 1), Err(GfxError::OutOfMemory));
        assert_eq!(check_size(16384, 16384), Err(GfxError::OutOfMemory));
        assert!(matches!(check_size(0, 5), Err(GfxError::Decode(_))));
    }

    #[test]
    fn formats_are_sniffed_from_magic_bytes() {
        assert_eq!(ImageFormat::sniff(b"GIF89a..."), Some(ImageFormat::Gif));
        assert_eq!(ImageFormat::sniff(b"BM......"), Some(ImageFormat::Bmp));
        assert_eq!(
            ImageFormat::sniff(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::sniff(b"\x89PNG\r\n\x1a\n"),
            Some(ImageFormat::Png)
        );
        assert_eq!(ImageFormat::sniff(b"\x89PNG"), None);
    }

    #[test]
    fn a_decoder_panic_is_a_decode_error() {
        let failure = guarded(|| -> Result<u8> { panic!("a decoder gave up") });
        assert_eq!(
            failure,
            Err(GfxError::Decode("the image decoder failed".into()))
        );
        // Results that are not panics pass as they are.
        assert_eq!(guarded(|| Ok(7)), Ok(7));
        assert_eq!(
            guarded(|| -> Result<u8> { Err(GfxError::OutOfMemory) }),
            Err(GfxError::OutOfMemory)
        );
    }

    /// A BMP with a 40-byte header and no pixel rows.
    fn bmp_header(width: i32, height: i32) -> Vec<u8> {
        let mut out = b"BM".to_vec();
        out.extend_from_slice(&[0; 8]);
        out.extend_from_slice(&54_u32.to_le_bytes());
        out.extend_from_slice(&40_u32.to_le_bytes());
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&24_u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        out
    }

    /// A GIF whose first frame, 2 by 2 pixels, sits one column in on a
    /// screen of 3 by 2.
    fn gif_file() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = ::gif::Encoder::new(&mut out, 3, 2, &[1, 2, 3, 4, 5, 6]).unwrap();
            let mut frame = ::gif::Frame::from_indexed_pixels(2, 2, vec![0, 1, 1, 0], None);
            frame.left = 1;
            encoder.write_frame(&frame).unwrap();
        }
        out
    }

    fn jpeg_file() -> Vec<u8> {
        let mut out = Vec::new();
        let encoder = jpeg_encoder::Encoder::new(&mut out, 90);
        encoder
            .encode(&[200; 5 * 3 * 3], 5, 3, jpeg_encoder::ColorType::Rgb)
            .unwrap();
        out
    }

    fn png_file() -> Vec<u8> {
        png::tests::encode(
            3,
            4,
            ::png::ColorType::Rgb,
            ::png::BitDepth::Eight,
            &[9; 36],
            None,
            None,
        )
    }

    /// The size `probe_size` reads is the size the decoder makes.
    fn assert_probe_matches_decode(file: &[u8], size: (u32, u32)) {
        let image = Surface::decode(file).unwrap();
        assert_eq!((image.width(), image.height()), size);
        assert_eq!(probe_size(file), Some(size));
    }

    #[test]
    fn probe_size_reads_every_format() {
        assert_probe_matches_decode(&png_file(), (3, 4));
        assert_probe_matches_decode(&gif_file(), (3, 2));
        assert_probe_matches_decode(&jpeg_file(), (5, 3));
        assert_probe_matches_decode(&bmp_header(6, 2), (6, 2));
        // A negative height only turns the rows over.
        assert_probe_matches_decode(&bmp_header(6, -2), (6, 2));
    }

    #[test]
    fn probe_size_follows_the_decoders_own_rules() {
        // A PNG counts its first IHDR, even behind another chunk.
        let png = png_file();
        let mut behind = png[..8].to_vec();
        behind.extend_from_slice(&[0, 0, 0, 2, b't', b'E', b'X', b't', 1, 2, 0, 0, 0, 0]);
        behind.extend_from_slice(&png[8..]);
        assert_eq!(probe_size(&behind), Some((3, 4)));
        // A GIF with an empty screen is as large as its first frame
        // reaches.
        let mut gif = gif_file();
        gif[6..10].fill(0);
        assert_probe_matches_decode(&gif, (3, 2));
        // The old OS/2 BMP header is 12 bytes with 16-bit sizes.
        let mut os2 = b"BM".to_vec();
        os2.extend_from_slice(&[0; 8]);
        os2.extend_from_slice(&26_u32.to_le_bytes());
        os2.extend_from_slice(&12_u32.to_le_bytes());
        os2.extend_from_slice(&[5, 0, 0xF9, 0xFF, 1, 0, 24, 0]);
        assert_probe_matches_decode(&os2, (5, 7));
        // A JPEG's fill bytes and the segments before the frame header
        // do not matter, and the frame header is found by its length
        // fields: a SOF0 inside an APP1 segment is no frame.
        let jpeg = jpeg_file();
        let at = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        let mut padded = jpeg[..at].to_vec();
        padded.extend_from_slice(&[0xFF, 0xFF, 0xFF]);
        padded.extend_from_slice(&jpeg[at..]);
        assert_probe_matches_decode(&padded, (5, 3));
        let mut exif = jpeg[..2].to_vec();
        exif.extend_from_slice(&[0xFF, 0xE1, 0, 10, 0xFF, 0xC0, 0, 0x11, 8, 0xFF, 0xFF, 0xFF]);
        exif.extend_from_slice(&jpeg[2..]);
        assert_eq!(probe_size(&exif), Some((5, 3)));
    }

    /// `file` with `piece` put in at `at`.
    fn with_inserted(file: &[u8], at: usize, piece: &[u8]) -> Vec<u8> {
        let mut out = file[..at].to_vec();
        out.extend_from_slice(piece);
        out.extend_from_slice(&file[at..]);
        out
    }

    #[test]
    fn probe_size_is_not_fooled_by_a_header_the_decoder_skips() {
        // A frame header that is not one: 1 by 1 pixels. If the probe took
        // it for the size, a caller would weigh a 1 by 1 image and decode
        // the real one.
        let fake = [0xFF, 0xC0, 0, 11, 8, 0, 1, 0, 1, 1, 1, 0x11, 0];
        let jpeg = jpeg_file();
        let frame = jpeg.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        // zune-jpeg takes only SOF0, SOF1 and SOF2 for frame headers and
        // skips the other SOF markers by their length.
        for marker in [0xC3, 0xC5, 0xC7, 0xC9, 0xCB, 0xCD, 0xCF] {
            let mut skipped = fake;
            skipped[1] = marker;
            assert_probe_matches_decode(&with_inserted(&jpeg, frame, &skipped), (5, 3));
        }
        // It reads a restart marker and a start of image as the start of a
        // segment with a length too, so a frame header inside one is hidden.
        for marker in [0x01, 0xD0, 0xD7, 0xD8] {
            let mut hidden = vec![0xFF, marker, 0, 15];
            hidden.extend_from_slice(&fake);
            assert_probe_matches_decode(&with_inserted(&jpeg, frame, &hidden), (5, 3));
        }
        // A GIF with an empty screen and a block the decoder skips before
        // its first frame.
        let mut gif = gif_file();
        gif[6..10].fill(0);
        let image = gif.iter().position(|&b| b == 0x2C).unwrap();
        let odd = with_inserted(&gif, image, &[0x05, 2, 0xAA, 0xBB, 0]);
        assert_probe_matches_decode(&odd, (3, 2));
    }

    #[test]
    fn probe_size_gives_up_on_what_it_cannot_read() {
        assert_eq!(probe_size(b""), None);
        assert_eq!(probe_size(b"not an image at all"), None);
        // Cut short, or no size to read.
        let png = png_file();
        assert_eq!(probe_size(&png[..8]), None);
        assert_eq!(probe_size(&png[..20]), None);
        let mut empty = png.clone();
        empty[16..20].fill(0);
        assert_eq!(probe_size(&empty), None);
        assert_eq!(probe_size(&gif_file()[..8]), None);
        assert_eq!(probe_size(&jpeg_file()[..12]), None);
        assert_eq!(probe_size(&[0xFF, 0xD8, 0xFF]), None);
        assert_eq!(probe_size(&bmp_header(0, 4)), None);
        assert_eq!(probe_size(&bmp_header(4, 0)), None);
        assert_eq!(probe_size(&bmp_header(-4, 4)), None);
        assert_eq!(probe_size(&bmp_header(4, 4)[..20]), None);
        let mut odd_header = bmp_header(4, 4);
        odd_header[14..18].copy_from_slice(&20_u32.to_le_bytes());
        assert_eq!(probe_size(&odd_header), None);
        // A JPEG that starts its image data before any frame header.
        let mut no_frame = vec![0xFF, 0xD8, 0xFF, 0xDA, 0, 4, 0, 0];
        assert_eq!(probe_size(&no_frame), None);
        no_frame.truncate(2);
        assert_eq!(probe_size(&no_frame), None);
    }

    #[test]
    fn probe_size_survives_damaged_headers() {
        let files = [png_file(), gif_file(), jpeg_file(), bmp_header(6, 2)];
        for file in &files {
            let head = file.len().min(80);
            for len in 0..=head {
                let _ = probe_size(&file[..len]);
            }
            for at in 0..head {
                for byte in [0x00, 0x01, 0x7F, 0x80, 0xFF] {
                    let mut damaged = file.clone();
                    damaged[at] = byte;
                    let _ = probe_size(&damaged);
                }
            }
        }
    }

    #[test]
    fn probe_size_reports_what_the_file_claims() {
        // Far more than a decoder accepts: the caller sees the claim and
        // the decoder refuses it.
        let mut huge = png_file();
        huge[16..24].copy_from_slice(&[0, 0, 0x40, 0, 0, 0, 0x20, 0]);
        assert_eq!(probe_size(&huge), Some((16384, 8192)));
        let mut wide = bmp_header(0x7FFF_FFFF, i32::MIN + 1);
        assert_eq!(probe_size(&wide), Some((0x7FFF_FFFF, 0x7FFF_FFFF)));
        wide[22..26].copy_from_slice(&i32::MIN.to_le_bytes());
        assert_eq!(probe_size(&wide), Some((0x7FFF_FFFF, 0x8000_0000)));
        assert_eq!(Surface::decode(&wide), Err(GfxError::OutOfMemory));
    }
}
