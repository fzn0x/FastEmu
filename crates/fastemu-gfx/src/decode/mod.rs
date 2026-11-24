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

impl Surface {
    /// `Image.createImage(imageData, 0, imageData.length)`: decodes a PNG,
    /// JPEG, GIF (its first frame) or BMP file into an immutable image.
    pub fn decode(data: &[u8]) -> Result<Surface> {
        let format = ImageFormat::sniff(data)
            .ok_or_else(|| GfxError::Decode("not a PNG, JPEG, GIF or BMP image".into()))?;
        let (width, height, pixels) = match format {
            ImageFormat::Png => png::decode(data),
            ImageFormat::Jpeg => jpeg::decode(data),
            ImageFormat::Gif => gif::decode(data),
            ImageFormat::Bmp => bmp::decode(data),
        }?;
        Surface::from_pixels(width, height, pixels, SurfaceKind::Immutable)
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
}
