//! Reading the size a BMP file declares from its header, without decoding
//! it. PNG, GIF and JPEG are read next to their decoders, in `png.rs`,
//! `gif.rs` and `jpeg.rs`, with the code decoding uses to find the header.

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
