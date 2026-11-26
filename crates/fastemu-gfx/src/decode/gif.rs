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

/// The bytes decoding holds for the first frame when it has more pixels
/// than the image `probe` gives. `decode` reads the frame whole, at its
/// own size, before it cuts it to the image: 4 bytes a pixel, and 1 for
/// the index the `gif` crate reads it into first. `None` when the frame
/// has no more pixels than the image, or the file cannot be read that
/// far, which `decode` then refuses too. Only the frame header is read.
pub(super) fn frame_bytes(data: &[u8]) -> Option<u64> {
    let mut decoder = options().read_info(Cursor::new(data)).ok()?;
    let screen = u64::from(decoder.width()) * u64::from(decoder.height());
    // An empty screen makes the frame at its offset the image, which
    // holds every pixel of the frame.
    if screen == 0 {
        return None;
    }
    let frame = decoder.next_frame_info().ok()??;
    let pixels = u64::from(frame.width) * u64::from(frame.height);
    (pixels > screen).then_some(pixels * 5)
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

    /// A GIF whose logical screen is `screen` (width, height) and whose
    /// first frame, of size `frame` at the place `at`, is all palette
    /// colour 1 (green).
    fn framed(screen: (u16, u16), frame: (u16, u16), at: (u16, u16), interlaced: bool) -> Vec<u8> {
        let palette = [255, 0, 0, 0, 255, 0];
        let mut out = Vec::new();
        {
            let mut encoder = Encoder::new(&mut out, screen.0, screen.1, &palette).unwrap();
            let indices = vec![1; usize::from(frame.0) * usize::from(frame.1)];
            let mut first = Frame::from_indexed_pixels(frame.0, frame.1, indices, None);
            (first.left, first.top) = at;
            first.interlaced = interlaced;
            encoder.write_frame(&first).unwrap();
        }
        out
    }

    /// The pixels of a `width` by `height` image that are green inside
    /// `x` and `y` and clear outside.
    fn green_inside(
        (width, height): (u32, u32),
        x: std::ops::Range<u32>,
        y: std::ops::Range<u32>,
    ) -> Vec<u32> {
        (0..height)
            .flat_map(|row| (0..width).map(move |column| (column, row)))
            .map(|(column, row)| {
                if x.contains(&column) && y.contains(&row) {
                    0xFF00_FF00
                } else {
                    0
                }
            })
            .collect()
    }

    #[test]
    fn a_first_frame_with_more_pixels_than_the_image_is_probed_and_still_cut() {
        // The caller asks the heap for the 2 by 2 image the file declares;
        // the frame of 64 by 64 is decoded whole before it is cut, its
        // pixels and their index: 5 bytes a pixel.
        let big = framed((2, 2), (64, 64), (0, 0), false);
        assert_eq!(crate::probe_size(&big), Some((2, 2)));
        assert_eq!(crate::probe_frame_bytes(&big), Some(64 * 64 * 5));
        let image = Surface::decode(&big).unwrap();
        assert_eq!(image.pixels(), &[0xFF00_FF00; 4]);
        // A frame of as many pixels in another shape, or of fewer, holds
        // no more than the image: no frame to ask the heap for.
        let tall = framed((4, 4), (2, 8), (0, 0), false);
        assert_eq!(crate::probe_frame_bytes(&tall), None);
        let image = Surface::decode(&tall).unwrap();
        assert_eq!(image.pixels(), green_inside((4, 4), 0..2, 0..4));
        assert_eq!(crate::probe_frame_bytes(&sample()), None);
        // An empty screen takes the frame at its offset as the image.
        let empty = framed((0, 0), (7, 5), (2, 3), false);
        assert_eq!(crate::probe_size(&empty), Some((9, 8)));
        assert_eq!(crate::probe_frame_bytes(&empty), None);
        // Not a GIF, or a GIF cut before its frame: nothing to say.
        assert_eq!(crate::probe_frame_bytes(b"\x89PNG\r\n\x1a\n"), None);
        assert_eq!(crate::probe_frame_bytes(&big[..20]), None);
    }

    #[test]
    fn a_frame_partly_outside_the_screen_is_cut_to_it_interlaced_or_not() {
        for interlaced in [false, true] {
            let file = framed((10, 10), (20, 20), (5, 5), interlaced);
            // 400 pixels of frame on a screen of 100: asked of the heap.
            assert_eq!(crate::probe_frame_bytes(&file), Some(400 * 5));
            let image = Surface::decode(&file).unwrap();
            assert_eq!((image.width(), image.height()), (10, 10));
            assert_eq!(
                image.pixels(),
                green_inside((10, 10), 5..10, 5..10),
                "interlaced {interlaced}"
            );
            // A frame inside the screen lands where it says.
            let file = framed((10, 10), (4, 3), (6, 2), interlaced);
            assert_eq!(crate::probe_frame_bytes(&file), None);
            let image = Surface::decode(&file).unwrap();
            assert_eq!(image.pixels(), green_inside((10, 10), 6..10, 2..5));
        }
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
