//! GIF: the first frame, placed on the logical screen, with its
//! transparent colour clear.

use std::io::Cursor;
use std::num::NonZeroU64;

use ::gif::{ColorOutput, DecodeOptions, MemoryLimit};

use super::{check_size, failed};
use crate::error::Result;
use crate::surface::MAX_PIXELS;

/// How a file is read, to decode it and to probe its size.
fn options() -> DecodeOptions {
    let mut options = DecodeOptions::new();
    options.set_color_output(ColorOutput::RGBA);
    options.allow_unknown_blocks(true);
    if let Some(limit) = NonZeroU64::new(MAX_PIXELS * 4) {
        options.set_memory_limit(MemoryLimit::Bytes(limit));
    }
    options
}

/// The size the file declares: its logical screen, or, where an encoder
/// left that empty, the first frame at its offset, as `decode` takes it.
/// The `gif` crate does the reading up to the frame header, with the
/// options `decode` uses, so blocks it skips are skipped here too, and
/// no pixels are read.
pub(super) fn probe(data: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = options().read_info(Cursor::new(data)).ok()?;
    let (width, height) = (u32::from(decoder.width()), u32::from(decoder.height()));
    if width > 0 && height > 0 {
        return Some((width, height));
    }
    let frame = decoder.next_frame_info().ok()??;
    let (width, height) = (
        u32::from(frame.left) + u32::from(frame.width),
        u32::from(frame.top) + u32::from(frame.height),
    );
    (width > 0 && height > 0).then_some((width, height))
}

pub(super) fn decode(data: &[u8]) -> Result<(u32, u32, Vec<u32>)> {
    let mut decoder = options()
        .read_info(Cursor::new(data))
        .map_err(|e| failed("GIF", e))?;
    let (screen_w, screen_h) = (u32::from(decoder.width()), u32::from(decoder.height()));
    let frame = decoder
        .read_next_frame()
        .map_err(|e| failed("GIF", e))?
        .ok_or_else(|| failed("GIF", "no frames"))?;
    let (left, top) = (u32::from(frame.left), u32::from(frame.top));
    let (frame_w, frame_h) = (u32::from(frame.width), u32::from(frame.height));
    // Some encoders leave the logical screen empty; use the frame then.
    let (width, height) = if screen_w == 0 || screen_h == 0 {
        (left + frame_w, top + frame_h)
    } else {
        (screen_w, screen_h)
    };
    check_size(width, height)?;
    let mut pixels = vec![0; width as usize * height as usize];
    for (fy, row) in frame
        .buffer
        .chunks_exact(frame_w.max(1) as usize * 4)
        .enumerate()
    {
        let y = top + fy as u32;
        if y >= height {
            break;
        }
        for (fx, &[r, g, b, a]) in row.as_chunks::<4>().0.iter().enumerate() {
            let x = left + fx as u32;
            if x < width {
                pixels[(y * width + x) as usize] =
                    u32::from(a) << 24 | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
            }
        }
    }
    Ok((width, height, pixels))
}

#[cfg(test)]
mod tests {
    use crate::error::GfxError;
    use crate::surface::Surface;
    use ::gif::{Encoder, Frame};

    /// A 3x2 GIF whose first frame covers the right 2x2 with palette
    /// colours 0 (red), 1 (green) and 2, which is transparent.
    fn sample() -> Vec<u8> {
        let palette = [255, 0, 0, 0, 255, 0, 0, 0, 255];
        let mut out = Vec::new();
        {
            let mut encoder = Encoder::new(&mut out, 3, 2, &palette).unwrap();
            let mut frame = Frame::from_indexed_pixels(2, 2, vec![0, 1, 2, 0], Some(2));
            frame.left = 1;
            encoder.write_frame(&frame).unwrap();
            let second = Frame::from_indexed_pixels(3, 2, vec![1; 6], None);
            encoder.write_frame(&second).unwrap();
        }
        out
    }

    #[test]
    fn the_first_frame_lands_on_the_screen() {
        let image = Surface::decode(&sample()).unwrap();
        assert_eq!((image.width(), image.height()), (3, 2));
        assert_eq!(
            image.pixels(),
            &[0, 0xFFFF_0000, 0xFF00_FF00, 0, 0x0000_00FF, 0xFFFF_0000]
        );
        assert!(!image.is_opaque());
    }

    #[test]
    fn broken_gifs_fail() {
        let file = sample();
        assert!(matches!(
            Surface::decode(&file[..14]),
            Err(GfxError::Decode(_))
        ));
    }
}
