//! PNG, repaired the way phones tolerated broken files before decoding.

use std::io::Cursor;

use ::png::{BitDepth, ColorType, Decoder, Limits, Transformations};

use super::{check_size, failed};
use crate::color::OPAQUE;
use crate::error::{GfxError, Result};
use crate::surface::MAX_PIXELS;

/// The eight bytes every PNG file starts with.
pub(super) const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// A decoded image: width, height and `0xAARRGGBB` pixels.
type Decoded = (u32, u32, Vec<u32>);

/// The most filtered row bytes the repair of a damaged file builds: 64
/// MiB, an 8-bit RGBA image of 16 million pixels, far above any phone's
/// screen. The repair holds the rows twice, so a file may cost twice this
/// on top of the image it makes. Valid files do not go through it.
const MAX_RAW_BYTES: usize = 64 << 20;

pub(super) fn decode(data: &[u8]) -> Result<Decoded> {
    let chunks = Chunks::read(data)?;
    match decode_stream(&chunks.assemble(&chunks.image)?) {
        Err(error @ GfxError::Decode(_)) => refill(&chunks).unwrap_or(Err(error)),
        result => result,
    }
}

/// Decodes a well-formed stream with the `png` crate.
fn decode_stream(stream: &[u8]) -> Result<Decoded> {
    let mut decoder = Decoder::new(Cursor::new(stream));
    // Palettes, grey levels, low and 16-bit depths and tRNS all come out
    // as 8-bit grey, grey and alpha, RGB or RGBA.
    decoder.set_transformations(Transformations::normalize_to_color8());
    decoder.ignore_checksums(true);
    decoder.set_limits(Limits {
        bytes: (MAX_PIXELS * 8) as usize,
    });
    let mut reader = decoder.read_info().map_err(|e| failed("PNG", e))?;
    let (width, height) = (reader.info().width, reader.info().height);
    check_size(width, height)?;
    let (color, depth) = reader.output_color_type();
    if depth != BitDepth::Eight {
        return Err(failed("PNG", "unsupported sample layout"));
    }
    let size = reader.output_buffer_size().ok_or(GfxError::OutOfMemory)?;
    let mut buffer = vec![0; size];
    reader
        .next_frame(&mut buffer)
        .map_err(|e| failed("PNG", e))?;
    Ok((width, height, to_argb(&buffer, color)?))
}

/// When the image data is cut short or its compressed stream is damaged
/// part way, inflates as much as can be inflated, pads it with zeros to
/// the size the header promises and decodes that, so the rows the file
/// does hold still show. Missing rows decode as zero samples: black, or
/// transparent where the image has alpha. `None` when there is nothing
/// to salvage.
fn refill(chunks: &Chunks<'_>) -> Option<Result<Decoded>> {
    let layout = Layout::of(chunks.header)?;
    if let Err(e) = check_size(layout.width, layout.height) {
        return Some(Err(e));
    }
    let expected = layout.raw_len();
    if expected > MAX_RAW_BYTES {
        return Some(Err(GfxError::OutOfMemory));
    }
    let mut raw =
        match miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&chunks.image, expected) {
            Ok(raw) => raw,
            Err(partial) => partial.output,
        };
    if raw.is_empty() {
        return None;
    }
    raw.resize(expected, 0);
    // The rows go into a stored stream and that into a file, each copy
    // freed before the next is made and the last before decoding.
    let stored = zlib_stored(&raw);
    drop(raw);
    let stream = chunks.assemble(&stored);
    drop(stored);
    match stream {
        Ok(stream) => Some(decode_stream(&stream)),
        Err(e) => Some(Err(e)),
    }
}

/// The chunks that decide the picture.
struct Chunks<'a> {
    header: &'a [u8],
    palette: Option<&'a [u8]>,
    transparency: Option<&'a [u8]>,
    /// Every IDAT chunk's data, joined.
    image: Vec<u8>,
}

impl<'a> Chunks<'a> {
    /// Collects IHDR, PLTE, tRNS and the image data, ignoring everything
    /// else, so bad checksums, misplaced or malformed ancillary chunks, a
    /// missing IEND, garbage after it and a last chunk cut short no
    /// longer matter.
    fn read(data: &'a [u8]) -> Result<Self> {
        let mut header = None;
        let mut palette = None;
        let mut transparency = None;
        let mut image = Vec::new();
        for (kind, body) in walk(data) {
            match &kind {
                b"IHDR" => {
                    header.get_or_insert(body);
                }
                // The png crate indexes a palette in whole colours and
                // panics on a stray byte or two, so only one of 1 to 256
                // colours is passed on. An indexed picture without one
                // fails to decode, and any other picture has no use for
                // it.
                b"PLTE" if palette.is_none() && is_palette(body) => palette = Some(body),
                b"tRNS" => {
                    transparency.get_or_insert(body);
                }
                b"IDAT" => image.extend_from_slice(body),
                b"IEND" => break,
                _ => {}
            }
        }
        let header = header.ok_or_else(|| failed("PNG", "no IHDR chunk"))?;
        if image.is_empty() {
            return Err(failed("PNG", "no image data"));
        }
        Ok(Self {
            header,
            palette,
            transparency,
            image,
        })
    }

    /// A clean stream: the chunks in the standard order, `image` as one
    /// IDAT, fresh checksums and an end chunk.
    fn assemble(&self, image: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(image.len() + 128);
        out.extend_from_slice(&SIGNATURE);
        write_chunk(&mut out, *b"IHDR", self.header)?;
        for (kind, body) in [(*b"PLTE", self.palette), (*b"tRNS", self.transparency)] {
            if let Some(body) = body {
                write_chunk(&mut out, kind, body)?;
            }
        }
        write_chunk(&mut out, *b"IDAT", image)?;
        write_chunk(&mut out, *b"IEND", &[])?;
        Ok(out)
    }
}

/// Every chunk of a file as its type and body, in file order: a length
/// that runs past the end of the data cuts the body short and ends the
/// walk, and checksums are not looked at.
fn walk(data: &[u8]) -> impl Iterator<Item = ([u8; 4], &[u8])> {
    let mut next = Some(SIGNATURE.len());
    std::iter::from_fn(move || {
        let pos = next?;
        let head = data.get(pos..pos + 8)?;
        let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        let kind = [head[4], head[5], head[6], head[7]];
        let start = pos + 8;
        let end = start.saturating_add(len);
        next = (end < data.len()).then(|| end + 4);
        Some((kind, &data[start..end.min(data.len())]))
    })
}

/// Whether `body` is a palette the `png` crate can use: one to 256 whole
/// colours of three bytes.
fn is_palette(body: &[u8]) -> bool {
    (3..=768).contains(&body.len()) && body.len().is_multiple_of(3)
}

/// The size an image file declares: its first IHDR, found as decoding
/// finds it. `None` when there is none, it is cut short or the picture is
/// empty.
pub(super) fn probe(data: &[u8]) -> Option<(u32, u32)> {
    let (_, header) = walk(data).find(|(kind, _)| kind == b"IHDR")?;
    let field = |at: usize| Some(u32::from_be_bytes(header.get(at..at + 4)?.try_into().ok()?));
    let (width, height) = (field(0)?, field(4)?);
    (width > 0 && height > 0).then_some((width, height))
}

fn write_chunk(out: &mut Vec<u8>, kind: [u8; 4], body: &[u8]) -> Result<()> {
    let len = u32::try_from(body.len()).map_err(|_| GfxError::OutOfMemory)?;
    out.extend_from_slice(&len.to_be_bytes());
    let start = out.len();
    out.extend_from_slice(&kind);
    out.extend_from_slice(body);
    let crc = crc32fast::hash(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
    Ok(())
}

/// The shape of the filtered image data an IHDR promises.
struct Layout {
    width: u32,
    height: u32,
    bits_per_pixel: usize,
    interlaced: bool,
}

/// Adam7's passes: first column and row, then column and row steps.
const ADAM7: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

impl Layout {
    /// The layout an IHDR promises, `None` unless it is one PNG allows: a
    /// colour type with one of its bit depths, the standard compression
    /// and filter methods and one of the two interlace methods. The `png`
    /// crate refuses everything else, so there is nothing to repair.
    fn of(header: &[u8]) -> Option<Self> {
        let field = |at: usize| Some(u32::from_be_bytes(header.get(at..at + 4)?.try_into().ok()?));
        let depth = usize::from(*header.get(8)?);
        let channels = match (*header.get(9)?, depth) {
            (0, 1 | 2 | 4 | 8 | 16) | (3, 1 | 2 | 4 | 8) => 1,
            (4, 8 | 16) => 2,
            (2, 8 | 16) => 3,
            (6, 8 | 16) => 4,
            _ => return None,
        };
        let interlaced = match header.get(12)? {
            0 => false,
            1 => true,
            _ => return None,
        };
        if *header.get(10)? != 0 || *header.get(11)? != 0 {
            return None;
        }
        Some(Self {
            width: field(0)?,
            height: field(4)?,
            bits_per_pixel: channels * depth,
            interlaced,
        })
    }

    /// The bytes of one filtered row `width` pixels wide, filter byte
    /// included.
    fn row_len(&self, width: u32) -> usize {
        (width as usize * self.bits_per_pixel).div_ceil(8) + 1
    }

    /// The bytes of all filtered rows, every pass of an interlaced image.
    fn raw_len(&self) -> usize {
        if !self.interlaced {
            return self.height as usize * self.row_len(self.width);
        }
        let count = |size: u32, start: u32, step: u32| size.saturating_sub(start).div_ceil(step);
        ADAM7
            .iter()
            .map(|&(x0, y0, dx, dy)| {
                let (w, h) = (count(self.width, x0, dx), count(self.height, y0, dy));
                if w == 0 {
                    0
                } else {
                    h as usize * self.row_len(w)
                }
            })
            .sum()
    }
}

/// A zlib stream of stored (uncompressed) deflate blocks around `data`.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + data.len() / 65535 * 5 + 11);
    out.extend_from_slice(&[0x78, 0x01]);
    let mut blocks = data.chunks(65535).peekable();
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xFF, 0xFF]);
    }
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    let (mut a, mut b) = (1_u32, 0_u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    out.extend_from_slice(&((b << 16) | a).to_be_bytes());
    out
}

/// 8-bit samples to `0xAARRGGBB`.
fn to_argb(samples: &[u8], color: ColorType) -> Result<Vec<u32>> {
    let grey = |v: u8| u32::from(v) * 0x0001_0101;
    let rgb = |r: u8, g: u8, b: u8| u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
    Ok(match color {
        ColorType::Grayscale => samples.iter().map(|&v| OPAQUE | grey(v)).collect(),
        ColorType::GrayscaleAlpha => samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&[v, a]| u32::from(a) << 24 | grey(v))
            .collect(),
        ColorType::Rgb => samples
            .as_chunks::<3>()
            .0
            .iter()
            .map(|&[r, g, b]| OPAQUE | rgb(r, g, b))
            .collect(),
        ColorType::Rgba => samples
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| u32::from(a) << 24 | rgb(r, g, b))
            .collect(),
        ColorType::Indexed => return Err(failed("PNG", "palette left unexpanded")),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::surface::Surface;
    use ::png::{BitDepth, ColorType, Encoder};

    /// Encodes an image with the `png` crate's encoder.
    pub(crate) fn encode(
        w: u32,
        h: u32,
        color: ColorType,
        depth: BitDepth,
        data: &[u8],
        palette: Option<&[u8]>,
        trns: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = Encoder::new(&mut out, w, h);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some(p) = palette {
            encoder.set_palette(p.to_vec());
        }
        if let Some(t) = trns {
            encoder.set_trns(t.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        writer.finish().unwrap();
        out
    }

    fn chunk(kind: [u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        write_chunk(&mut out, kind, body).unwrap();
        out
    }

    fn pixels(data: &[u8]) -> Vec<u32> {
        Surface::decode(data).unwrap().pixels().to_vec()
    }

    #[test]
    fn rgba_and_rgb_images_decode() {
        let rgba = encode(
            2,
            1,
            ColorType::Rgba,
            BitDepth::Eight,
            &[255, 0, 0, 255, 0, 0, 255, 128],
            None,
            None,
        );
        let image = Surface::decode(&rgba).unwrap();
        assert_eq!(image.pixels(), &[0xFFFF_0000, 0x8000_00FF]);
        assert!(!image.is_mutable() && !image.is_opaque());
        let rgb = encode(
            1,
            2,
            ColorType::Rgb,
            BitDepth::Eight,
            &[1, 2, 3, 4, 5, 6],
            None,
            None,
        );
        let image = Surface::decode(&rgb).unwrap();
        assert_eq!(image.pixels(), &[0xFF01_0203, 0xFF04_0506]);
        assert!(image.is_opaque());
    }

    #[test]
    fn palettes_with_transparency_decode() {
        // Four 2-bit pixels: indices 0, 1, 2, 3; index 1 is half clear.
        let palette = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120];
        let png = encode(
            4,
            1,
            ColorType::Indexed,
            BitDepth::Two,
            &[0b00_01_10_11],
            Some(&palette),
            Some(&[255, 128]),
        );
        assert_eq!(
            pixels(&png),
            [0xFF0A_141E, 0x8028_323C, 0xFF46_505A, 0xFF64_6E78]
        );
    }

    #[test]
    fn grey_images_and_grey_transparency_decode() {
        let png = encode(
            8,
            1,
            ColorType::Grayscale,
            BitDepth::One,
            &[0b1010_0000],
            None,
            None,
        );
        assert_eq!(pixels(&png)[..3], [0xFFFF_FFFF, 0xFF00_0000, 0xFFFF_FFFF]);
        // 8-bit grey with grey level 7 marked transparent.
        let png = encode(
            2,
            1,
            ColorType::Grayscale,
            BitDepth::Eight,
            &[7, 9],
            None,
            Some(&[0, 7]),
        );
        assert_eq!(pixels(&png), [0x0007_0707, 0xFF09_0909]);
        let png = encode(
            1,
            1,
            ColorType::GrayscaleAlpha,
            BitDepth::Eight,
            &[200, 100],
            None,
            None,
        );
        assert_eq!(pixels(&png), [0x64C8_C8C8]);
    }

    #[test]
    fn sixteen_bit_images_keep_their_high_bytes() {
        let png = encode(
            1,
            1,
            ColorType::Rgba,
            BitDepth::Sixteen,
            &[0x12, 0x99, 0x34, 0, 0x56, 0, 0xFF, 0xFF],
            None,
            None,
        );
        assert_eq!(pixels(&png), [0xFF12_3456]);
    }

    /// A 3x3 RGB image written with Adam7 interlacing by hand: each pass's
    /// rows, filter byte 0, then the pixels the pass covers.
    #[test]
    fn interlaced_images_decode() {
        let colour = |x: u8, y: u8| [x * 50, y * 50, 7];
        let passes: [(u8, u8, u8, u8); 7] = [
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ];
        let mut raw = Vec::new();
        for (x0, y0, dx, dy) in passes {
            for y in (y0..3).step_by(dy.into()) {
                let xs: Vec<u8> = (x0..3).step_by(dx.into()).collect();
                if xs.is_empty() {
                    continue;
                }
                raw.push(0);
                for x in xs {
                    raw.extend_from_slice(&colour(x, y));
                }
            }
        }
        let mut file = SIGNATURE.to_vec();
        file.extend(chunk(*b"IHDR", &[0, 0, 0, 3, 0, 0, 0, 3, 8, 2, 0, 0, 1]));
        file.extend(chunk(*b"IDAT", &zlib_stored(&raw)));
        file.extend(chunk(*b"IEND", &[]));
        let image = Surface::decode(&file).unwrap();
        for y in 0..3 {
            for x in 0..3 {
                let [r, g, b] = colour(x, y);
                let want = OPAQUE | u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
                assert_eq!(image.pixel(x.into(), y.into()), Some(want));
            }
        }
    }

    #[test]
    fn broken_files_still_decode() {
        let good = encode(2, 2, ColorType::Rgb, BitDepth::Eight, &[9; 12], None, None);
        let want = pixels(&good);
        // Every CRC wrong.
        let mut bad_crc = good.clone();
        let mut pos = 8;
        while pos + 8 <= bad_crc.len() {
            let len = u32::from_be_bytes(bad_crc[pos..pos + 4].try_into().unwrap()) as usize;
            bad_crc[pos + 8 + len] ^= 0xFF;
            pos += 12 + len;
        }
        assert_eq!(pixels(&bad_crc), want);
        // Garbage after IEND.
        let mut trailing = good.clone();
        trailing.extend_from_slice(b"garbage that is not a chunk");
        assert_eq!(pixels(&trailing), want);
        // No IEND at all.
        let missing_end = &good[..good.len() - 12];
        assert_eq!(pixels(missing_end), want);
        // An ancillary chunk with a nonsense length before the data.
        let mut junk = good[..33].to_vec();
        junk.extend(chunk(*b"tEXt", b"\xFF\xFF"));
        junk.extend_from_slice(&good[33..]);
        assert_eq!(pixels(&junk), want);
    }

    #[test]
    fn truncated_image_data_keeps_the_rows_it_has() {
        let rows: Vec<u8> = (0..4).flat_map(|y| [0, y * 60, y * 60, y * 60]).collect();
        let mut file = SIGNATURE.to_vec();
        file.extend(chunk(*b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 4, 8, 2, 0, 0, 0]));
        let stream = zlib_stored(&rows);
        // Keep the zlib header, the block header and two of four rows.
        file.extend(chunk(*b"IDAT", &stream[..2 + 5 + 8]));
        let image = Surface::decode(&file).unwrap();
        assert_eq!(image.pixel(0, 0), Some(0xFF00_0000));
        assert_eq!(image.pixel(0, 1), Some(0xFF3C_3C3C));
        assert_eq!(image.pixel(0, 3), Some(0xFF00_0000));
    }

    #[test]
    fn damaged_streams_are_refilled() {
        // Two stored blocks of two RGBA rows each; the second block's
        // length check is broken, which the png crate rejects and the
        // refill drops.
        let rows: Vec<u8> = (0..4)
            .flat_map(|y| [0, y * 60, y * 60, y * 60, 255])
            .collect();
        let mut stream = vec![0x78, 0x01];
        stream.extend_from_slice(&[0, 10, 0, !10, 0xFF]);
        stream.extend_from_slice(&rows[..10]);
        stream.extend_from_slice(&[1, 10, 0, 0, 0]);
        stream.extend_from_slice(&rows[10..]);
        let mut file = SIGNATURE.to_vec();
        file.extend(chunk(*b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 4, 8, 6, 0, 0, 0]));
        file.extend(chunk(*b"IDAT", &stream));
        let chunks = Chunks::read(&file).unwrap();
        assert!(decode_stream(&chunks.assemble(&chunks.image).unwrap()).is_err());
        let image = Surface::decode(&file).unwrap();
        assert_eq!((image.width(), image.height()), (1, 4));
        assert_eq!(image.pixel(0, 1), Some(0xFF3C_3C3C));
        assert_eq!(image.pixel(0, 3), Some(0));
    }

    #[test]
    fn layouts_count_filtered_bytes() {
        let header = |w: u8, h: u8, depth: u8, color: u8, interlace: u8| {
            Layout::of(&[0, 0, 0, w, 0, 0, 0, h, depth, color, 0, 0, interlace]).unwrap()
        };
        assert_eq!(header(3, 2, 8, 2, 0).raw_len(), 2 * (1 + 9));
        assert_eq!(header(9, 1, 1, 0, 0).raw_len(), 1 + 2);
        // 3x3 RGB interlaced: passes 1, 4, 5, 6 and 7 hold 1, 1, 2, 2
        // and 3 pixels in one row each, except pass 6 with two rows.
        assert_eq!(header(3, 3, 8, 2, 1).raw_len(), 4 + 4 + 7 + 2 * 4 + 10);
        assert!(Layout::of(&[0, 0, 0, 1, 0, 0, 0, 1, 8, 5, 0, 0, 0]).is_none());
        assert!(Layout::of(&[0; 12]).is_none());
    }

    /// A PNG with a hand-made header, a stored image stream of `raw` and
    /// the chunks in `extra` between them.
    fn crafted(header: [u8; 13], extra: &[Vec<u8>], raw: &[u8]) -> Vec<u8> {
        let mut file = SIGNATURE.to_vec();
        file.extend(chunk(*b"IHDR", &header));
        for part in extra {
            file.extend_from_slice(part);
        }
        file.extend(chunk(*b"IDAT", &zlib_stored(raw)));
        file.extend(chunk(*b"IEND", &[]));
        file
    }

    /// An IHDR body.
    fn ihdr(width: u32, height: u32, depth: u8, color: u8, interlace: u8) -> [u8; 13] {
        let mut out = [0; 13];
        out[..4].copy_from_slice(&width.to_be_bytes());
        out[4..8].copy_from_slice(&height.to_be_bytes());
        out[8..].copy_from_slice(&[depth, color, 0, 0, interlace]);
        out
    }

    #[test]
    fn only_the_depths_png_allows_make_a_layout() {
        let allowed: [(u8, &[u8]); 5] = [
            (0, &[1, 2, 4, 8, 16]),
            (2, &[8, 16]),
            (3, &[1, 2, 4, 8]),
            (4, &[8, 16]),
            (6, &[8, 16]),
        ];
        for color in 0..=7_u8 {
            for depth in [0, 1, 2, 3, 4, 5, 8, 12, 16, 17, 32, 255] {
                let legal = allowed
                    .iter()
                    .any(|&(c, depths)| c == color && depths.contains(&depth));
                let layout = Layout::of(&ihdr(4, 4, depth, color, 0));
                assert_eq!(
                    layout.is_some(),
                    legal,
                    "colour type {color}, depth {depth}"
                );
            }
        }
        // Only the two interlace methods PNG has.
        assert!(Layout::of(&ihdr(4, 4, 8, 2, 1)).is_some());
        assert!(Layout::of(&ihdr(4, 4, 8, 2, 2)).is_none());
    }

    #[test]
    fn an_invalid_bit_depth_is_not_repaired() {
        // The png crate refuses depth 255, and the repair used to size
        // three buffers from it: 3 x 127.5 bytes a pixel. Scaled down
        // from the 72 byte file with 16384 x 2048 pixels that wanted
        // 12.8 GB, this one wanted 190 MB.
        let file = crafted(ihdr(1024, 512, 255, 6, 0), &[], &[0, 1, 2, 3]);
        assert!(matches!(Surface::decode(&file), Err(GfxError::Decode(_))));
        let chunks = Chunks::read(&file).unwrap();
        assert!(refill(&chunks).is_none());
    }

    /// The audit's 72 byte file. Before the fix it made the repair ask for
    /// three buffers of 4.28 GB each, and a failed allocation aborts.
    #[test]
    fn the_72_byte_depth_bomb_fails_fast() {
        let file = crafted(ihdr(16384, 2048, 255, 6, 0), &[], &[0, 1, 2, 3]);
        assert_eq!(file.len(), 72);
        assert!(matches!(Surface::decode(&file), Err(GfxError::Decode(_))));
    }

    #[test]
    fn a_repair_larger_than_its_cap_is_refused() {
        // 16-bit RGBA at 4096 x 4096 promises 128 MiB of rows from a
        // 70 byte file. A valid image of that size still decodes; only
        // the repair of a damaged one needs the rows in memory.
        let file = crafted(ihdr(4096, 4096, 16, 6, 0), &[], &[0, 1, 2, 3]);
        assert!(matches!(Surface::decode(&file), Err(GfxError::OutOfMemory)));
        // Just under the cap is repaired: 4 x 8 pixels of 16-bit RGBA.
        let file = crafted(ihdr(4, 8, 16, 6, 0), &[], &[0, 1, 2, 3]);
        assert_eq!(Surface::decode(&file).unwrap().width(), 4);
    }

    /// An indexed 1 x 1 image whose palette chunk is `palette`.
    fn with_palette(palette: &[u8]) -> Vec<u8> {
        crafted(ihdr(1, 1, 8, 3, 0), &[chunk(*b"PLTE", palette)], &[0, 0])
    }

    #[test]
    fn a_palette_that_is_not_whole_colours_is_dropped() {
        // The png crate panics on these (range end index 3 out of range
        // for slice of length 1), which stopped the game's whole VM.
        for len in [1, 2, 4, 5, 767, 769, 771, 1000] {
            let file = with_palette(&vec![9; len]);
            assert!(
                matches!(Surface::decode(&file), Err(GfxError::Decode(_))),
                "{len} byte palette"
            );
        }
        // An empty one is no palette at all.
        assert!(matches!(
            Surface::decode(&with_palette(&[])),
            Err(GfxError::Decode(_))
        ));
        // Whole colours still work, from one to 256.
        for entries in [1, 2, 255, 256] {
            let file = with_palette(&vec![7; entries * 3]);
            assert_eq!(pixels(&file), [0xFF07_0707], "{entries} entries");
        }
        // A bad palette on a picture that needs none does no harm.
        let rgb = crafted(
            ihdr(1, 1, 8, 2, 0),
            &[chunk(*b"PLTE", &[1, 2, 3, 4])],
            &[0, 5, 6, 7],
        );
        assert_eq!(pixels(&rgb), [0xFF05_0607]);
        // A later palette stands in for a bad first one.
        let two = crafted(
            ihdr(1, 1, 8, 3, 0),
            &[chunk(*b"PLTE", &[1, 2, 3, 4]), chunk(*b"PLTE", &[1, 2, 3])],
            &[0, 0],
        );
        assert_eq!(pixels(&two), [0xFF01_0203]);
    }

    #[test]
    fn the_odd_palette_file_from_the_audit_fails_cleanly() {
        let file = encode(
            1,
            1,
            ColorType::Indexed,
            BitDepth::Eight,
            &[0],
            Some(&[1, 2, 3, 4]),
            None,
        );
        assert!(file.len() < 100);
        assert!(matches!(Surface::decode(&file), Err(GfxError::Decode(_))));
    }

    #[test]
    fn a_panic_inside_the_png_crate_is_caught() {
        // The stream the repair would pass on if it kept the palette. The
        // png crate panics on it; the guard around every decode turns that
        // into an error, whatever the crate does about it later.
        let stream = with_palette(&[1, 2, 3, 4]);
        let outcome = super::super::guarded(|| decode_stream(&stream));
        assert!(matches!(outcome, Err(GfxError::Decode(_))));
    }

    #[test]
    fn unreadable_files_fail() {
        let good = encode(
            1,
            1,
            ColorType::Rgb,
            BitDepth::Eight,
            &[1, 2, 3],
            None,
            None,
        );
        assert!(matches!(
            Surface::decode(&good[..20]),
            Err(GfxError::Decode(_))
        ));
        let mut no_data = good[..33].to_vec();
        no_data.extend(chunk(*b"IEND", &[]));
        assert!(matches!(
            Surface::decode(&no_data),
            Err(GfxError::Decode(_))
        ));
        let mut huge = SIGNATURE.to_vec();
        huge.extend(chunk(*b"IHDR", &[0, 1, 0, 0, 0, 1, 0, 0, 8, 2, 0, 0, 0]));
        huge.extend(chunk(*b"IDAT", &zlib_stored(&[0])));
        assert_eq!(Surface::decode(&huge), Err(GfxError::OutOfMemory));
    }
}
