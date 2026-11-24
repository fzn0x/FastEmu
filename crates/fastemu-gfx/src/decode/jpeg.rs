//! JPEG, baseline and progressive, through zune-jpeg.

use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use super::{MAX_SIDE, check_size, failed};
use crate::color::OPAQUE;
use crate::error::Result;

pub(super) fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u32>)> {
    let options = DecoderOptions::default()
        .jpeg_set_out_colorspace(ColorSpace::RGB)
        .set_max_width(MAX_SIDE as usize)
        .set_max_height(MAX_SIDE as usize)
        .set_strict_mode(false);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(data), options);
    decoder.decode_headers().map_err(|e| failed("JPEG", e))?;
    let (width, height) = decoder
        .dimensions()
        .ok_or_else(|| failed("JPEG", "no frame header"))?;
    let (width, height) = (width as u32, height as u32);
    check_size(width, height)?;
    let rgb = decoder.decode().map_err(|e| failed("JPEG", e))?;
    let pixels = rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|&[r, g, b]| OPAQUE | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b))
        .collect();
    Ok((width, height, pixels))
}

#[cfg(test)]
mod tests {
    use crate::error::GfxError;
    use crate::surface::Surface;
    use jpeg_encoder::{ColorType, Encoder};

    /// A 16x8 image: red on the left half, blue on the right.
    fn halves(progressive: bool) -> Vec<u8> {
        let mut rgb = Vec::new();
        for _ in 0..8 {
            for x in 0..16 {
                rgb.extend_from_slice(if x < 8 { &[255, 0, 0] } else { &[0, 0, 255] });
            }
        }
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out, 100);
        encoder.set_progressive(progressive);
        encoder.encode(&rgb, 16, 8, ColorType::Rgb).unwrap();
        out
    }

    fn near(pixel: u32, want: u32) -> bool {
        (0..3).all(|i| ((pixel >> (i * 8)) & 0xFF).abs_diff((want >> (i * 8)) & 0xFF) <= 24)
    }

    #[test]
    fn baseline_and_progressive_jpegs_decode() {
        for progressive in [false, true] {
            let image = Surface::decode(&halves(progressive)).unwrap();
            assert_eq!((image.width(), image.height()), (16, 8));
            assert!(image.is_opaque());
            assert!(near(image.pixel(2, 4).unwrap(), 0xFF_0000), "{progressive}");
            assert!(
                near(image.pixel(13, 4).unwrap(), 0x00_00FF),
                "{progressive}"
            );
        }
    }

    #[test]
    fn broken_jpegs_fail() {
        let file = halves(false);
        assert!(matches!(
            Surface::decode(&file[..30]),
            Err(GfxError::Decode(_))
        ));
    }
}
