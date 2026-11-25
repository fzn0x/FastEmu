//! Reading the size an image file declares from its header, without
//! decoding it. PNG is in `png.rs`, next to the chunk walk decoding uses.

/// A GIF's size: its logical screen, or, where an encoder left that empty,
/// the first frame placed at its offset, as `gif.rs` shows it.
pub(super) fn gif(data: &[u8]) -> Option<(u32, u32)> {
    let word = |at: usize| {
        Some(u32::from(u16::from_le_bytes(
            data.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    let (width, height) = (word(6)?, word(8)?);
    if width > 0 && height > 0 {
        return Some((width, height));
    }
    // Skip the global colour table, then the extensions in front of the
    // first image.
    let flags = *data.get(10)?;
    let mut pos = 13
        + if flags & 0x80 == 0 {
            0
        } else {
            3 << ((flags & 7) + 1)
        };
    loop {
        match *data.get(pos)? {
            0x21 => {
                pos += 2;
                loop {
                    let size = usize::from(*data.get(pos)?);
                    pos += 1 + size;
                    if size == 0 {
                        break;
                    }
                }
            }
            0x2C => {
                let (width, height) = (
                    word(pos + 1)? + word(pos + 5)?,
                    word(pos + 3)? + word(pos + 7)?,
                );
                return (width > 0 && height > 0).then_some((width, height));
            }
            _ => return None,
        }
    }
}

/// A BMP's size, the height without its sign, which only says which end
/// the rows start at.
pub(super) fn bmp(data: &[u8]) -> Option<(u32, u32)> {
    let le = |at: usize, len: usize| data.get(at..at + len);
    let header = u32::from_le_bytes(le(14, 4)?.try_into().ok()?);
    let (width, height) = match header {
        12 => (
            i64::from(u16::from_le_bytes(le(18, 2)?.try_into().ok()?)),
            i64::from(i16::from_le_bytes(le(20, 2)?.try_into().ok()?)),
        ),
        40.. => (
            i64::from(i32::from_le_bytes(le(18, 4)?.try_into().ok()?)),
            i64::from(i32::from_le_bytes(le(22, 4)?.try_into().ok()?)),
        ),
        _ => return None,
    };
    let height = height.abs();
    if width <= 0 || height <= 0 {
        return None;
    }
    Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
}

/// A JPEG's size from its first frame header (SOF0 to SOF15, which are
/// not the tables DHT, DAC or the reserved JPG marker). Segments are
/// skipped by their lengths, so a thumbnail inside an EXIF block does not
/// count; the scan ends at the start of the image data.
pub(super) fn jpeg(data: &[u8]) -> Option<(u32, u32)> {
    let mut pos = 2;
    loop {
        // A marker is 0xFF and its code, after any fill bytes.
        while *data.get(pos)? != 0xFF {
            pos += 1;
        }
        while *data.get(pos)? == 0xFF {
            pos += 1;
        }
        let marker = *data.get(pos)?;
        pos += 1;
        match marker {
            // Stand-alone markers carry no length.
            0x00 | 0x01 | 0xD0..=0xD8 => {}
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                let word = |at: usize| {
                    Some(u32::from(u16::from_be_bytes(
                        data.get(at..at + 2)?.try_into().ok()?,
                    )))
                };
                let (height, width) = (word(pos + 3)?, word(pos + 5)?);
                return (width > 0 && height > 0).then_some((width, height));
            }
            // The end of the image or the start of its data, with no frame
            // header before it.
            0xD9 | 0xDA => return None,
            _ => {
                let len = usize::from(u16::from_be_bytes(data.get(pos..pos + 2)?.try_into().ok()?));
                pos += len.max(2);
            }
        }
    }
}
