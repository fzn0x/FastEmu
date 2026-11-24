//! Turning phone pictures into the BGRA bytes GPUI uploads: game screens,
//! suite icons and screenshots.

use std::io::Cursor;

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
pub fn icon_bgra(png: &[u8], min_side: u32) -> Option<Bgra> {
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(png),
        png::Limits {
            bytes: 4 * 1024 * 1024,
        },
    );
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    if w == 0 || h == 0 || w > MAX_ICON_SIDE || h > MAX_ICON_SIDE {
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

    #[test]
    fn broken_icons_are_ignored() {
        assert!(icon_bgra(b"not a png", 32).is_none());
        assert!(icon_bgra(&[], 32).is_none());
    }
}
