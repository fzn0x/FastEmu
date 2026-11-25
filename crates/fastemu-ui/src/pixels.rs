//! Turning phone pictures into the BGRA bytes GPUI uploads: game screens,
//! suite icons and screenshots.

use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use fastemu_core::Frame;

/// A picture as GPUI's image atlas takes it: four bytes per pixel in B, G,
/// R, A order, rows top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bgra {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

/// `frame` scaled up `factor` times with nearest-neighbour sampling, as
/// BGRA bytes. The result matches `frame.scaled(factor)` byte for byte but
/// skips the intermediate copy, which matters at sixty frames a second.
pub fn screen_bgra(frame: &Frame, factor: u32) -> Bgra {
    let factor = factor.max(1);
    let (w, h, f) = (
        frame.width() as usize,
        frame.height() as usize,
        factor as usize,
    );
    let row_bytes = w * f * 4;
    let mut bytes = Vec::with_capacity(row_bytes * h * f);
    for row in frame.pixels().chunks_exact(w.max(1)).take(h) {
        let start = bytes.len();
        for &p in row {
            let px = [p as u8, (p >> 8) as u8, (p >> 16) as u8, (p >> 24) as u8];
            for _ in 0..f {
                bytes.extend_from_slice(&px);
            }
        }
        for _ in 1..f {
            bytes.extend_from_within(start..start + row_bytes);
        }
    }
    Bgra {
        width: frame.width() * factor,
        height: frame.height() * factor,
        bytes,
    }
}

/// The largest icon FastEmu decodes; MIDlet icons are tiny, so anything
/// bigger is not worth the memory.
const MAX_ICON_SIDE: u32 = 512;

/// Decodes a suite's PNG icon and scales it up by whole pixels until its
/// longer side reaches `min_side`, so a 16x16 icon stays crisp when the
/// sidebar draws it larger. `None` for anything that is not a usable PNG.
///
/// The icon is a game's own file, read for every library entry on a
/// background task, and the `png` crate panics on some damaged ones (a
/// palette that is not a whole number of colours). A panic leaving a
/// background task aborts the whole app, and every game running in it, so
/// a decoder that panics gives no icon.
pub fn icon_bgra(png: &[u8], min_side: u32) -> Option<Bgra> {
    catch_unwind(AssertUnwindSafe(|| decode_icon(png, min_side))).unwrap_or_else(|_| {
        log::warn!("a suite icon could not be decoded: the PNG decoder failed");
        None
    })
}

fn decode_icon(png: &[u8], min_side: u32) -> Option<Bgra> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(png),
        png::Limits {
            bytes: 4 * 1024 * 1024,
        },
    );
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    // The decoder's limits do not cover the frame buffer, which is ours:
    // check the size the header claims before allocating for it, or a
    // crafted icon could ask for gigabytes.
    let header = reader.info();
    if !usable_icon_size(header.width, header.height) {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    if !usable_icon_size(w, h) {
        return None;
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let pixels: Vec<u32> = buf[..info.line_size * h as usize]
        .chunks_exact(info.line_size)
        .flat_map(|line| line.chunks_exact(channels).take(w as usize))
        .map(|px| {
            let (r, g, b, a) = match *px {
                [l] => (l, l, l, 0xFF),
                [l, a] => (l, l, l, a),
                [r, g, b] => (r, g, b, 0xFF),
                [r, g, b, a] => (r, g, b, a),
                _ => unreachable!("chunks hold one pixel"),
            };
            u32::from_be_bytes([a, r, g, b])
        })
        .collect();
    let factor = min_side.div_ceil(w.max(h)).max(1);
    Some(screen_bgra(&Frame::new(w, h, pixels.into(), 0), factor))
}

fn usable_icon_size(width: u32, height: u32) -> bool {
    (1..=MAX_ICON_SIDE).contains(&width) && (1..=MAX_ICON_SIDE).contains(&height)
}

/// Encodes `frame` as an RGBA PNG at its own size.
pub fn encode_png(frame: &Frame) -> Result<Vec<u8>, png::EncodingError> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, frame.width(), frame.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&frame.to_rgba8())?;
    writer.finish()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_endian = "little")]
    fn screen_bytes_match_the_core_scaler() {
        let frame = Frame::new(3, 2, vec![0xFF01_0203, 2, 3, 4, 5, 0x8899_AABB].into(), 9);
        for factor in 1..4 {
            let bgra = screen_bgra(&frame, factor);
            let reference = frame.scaled(factor);
            assert_eq!(bgra.width, reference.width());
            assert_eq!(bgra.height, reference.height());
            assert_eq!(bgra.bytes, reference.as_bgra_bytes());
        }
    }

    #[test]
    fn a_zero_factor_means_one() {
        let frame = Frame::filled(2, 2, 0xFF00_00FF, 0);
        assert_eq!(screen_bgra(&frame, 0).width, 2);
        assert_eq!(&screen_bgra(&frame, 0).bytes[..4], &[0xFF, 0, 0, 0xFF]);
    }

    #[test]
    fn png_round_trips_and_icons_scale_up() {
        let frame = Frame::new(2, 1, vec![0xFFFF_0000, 0x8000_FF00].into(), 0);
        let png = encode_png(&frame).expect("encodes");
        let icon = icon_bgra(&png, 8).expect("decodes");
        assert_eq!((icon.width, icon.height), (8, 4));
        // Red, opaque, as B G R A.
        assert_eq!(&icon.bytes[..4], &[0, 0, 0xFF, 0xFF]);
        // The right half is half-transparent green.
        assert_eq!(&icon.bytes[16..20], &[0, 0xFF, 0, 0x80]);
    }

    /// The CRC-32 a PNG chunk ends with.
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &b in bytes {
            crc ^= u32::from(b);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// A valid one-pixel PNG whose header claims `width` by `height`.
    fn png_claiming(width: u32, height: u32) -> Vec<u8> {
        let mut png = encode_png(&Frame::filled(1, 1, 0xFF00_0000, 0)).expect("encodes");
        // Signature (8), IHDR length (4), then "IHDR" and its 13 bytes.
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        png
    }

    #[test]
    fn oversized_icons_are_refused_before_decoding() {
        assert!(
            icon_bgra(&png_claiming(1, 1), 8).is_some(),
            "the patch is valid"
        );
        // 100000 x 100000 RGBA would need a 40 GB buffer.
        assert!(icon_bgra(&png_claiming(100_000, 100_000), 8).is_none());
        assert!(icon_bgra(&png_claiming(MAX_ICON_SIDE + 1, 1), 8).is_none());
    }

    /// An indexed 1 x 1 PNG whose palette chunk holds `palette`, with valid
    /// checksums, as a game's icon file would have them.
    fn png_with_palette(palette: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 1, 1);
        encoder.set_color(png::ColorType::Indexed);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_palette(palette.to_vec());
        let mut writer = encoder.write_header().expect("writes");
        writer.write_image_data(&[0]).expect("writes");
        writer.finish().expect("writes");
        out
    }

    #[test]
    fn an_icon_the_png_crate_panics_on_gives_no_icon() {
        // The audit's file: a palette of 4 bytes. The decoder panics with
        // "range end index 3 out of range" on it, and on a background task
        // that aborts the app.
        let odd = png_with_palette(&[1, 2, 3, 4]);
        assert!(odd.len() < 100);
        assert!(icon_bgra(&odd, 32).is_none());
        for len in [1, 2, 5, 769] {
            assert!(icon_bgra(&png_with_palette(&vec![9; len]), 32).is_none());
        }
    }

    #[test]
    fn broken_icons_are_ignored() {
        assert!(icon_bgra(b"not a png", 32).is_none());
        assert!(icon_bgra(&[], 32).is_none());
    }
}
