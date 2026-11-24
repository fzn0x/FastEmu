use std::sync::Arc;

/// One picture of an instance's screen.
///
/// Pixels are row-major `0xAARRGGBB` words. In little-endian memory each
/// word is the bytes B, G, R, A, the BGRA layout GPU texture uploads take,
/// so a renderer can use [`Frame::as_bgra_bytes`] without converting.
/// Screen frames are opaque: alpha is 0xFF everywhere.
#[derive(Clone, Debug)]
pub struct Frame {
    width: u32,
    height: u32,
    pixels: Arc<[u32]>,
    serial: u64,
}

impl Frame {
    /// Wraps `pixels`, which must hold exactly `width * height` words.
    ///
    /// # Panics
    ///
    /// When the pixel count does not match the size.
    pub fn new(width: u32, height: u32, pixels: Arc<[u32]>, serial: u64) -> Self {
        assert_eq!(
            pixels.len(),
            width as usize * height as usize,
            "a {width}x{height} frame needs {} pixels",
            width as usize * height as usize
        );
        Self {
            width,
            height,
            pixels,
            serial,
        }
    }

    /// A frame of one colour, for placeholders and tests.
    pub fn filled(width: u32, height: u32, argb: u32, serial: u64) -> Self {
        let pixels: Arc<[u32]> = vec![argb; width as usize * height as usize].into();
        Self::new(width, height, pixels, serial)
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Row-major `0xAARRGGBB` pixels.
    pub fn pixels(&self) -> &[u32] {
        &self.pixels
    }

    /// The shared pixel buffer, for cheap clones into another thread.
    pub fn shared_pixels(&self) -> Arc<[u32]> {
        Arc::clone(&self.pixels)
    }

    /// Grows each time the instance presents a new picture, so equal
    /// serials from one instance mean the same picture.
    pub fn serial(&self) -> u64 {
        self.serial
    }

    pub fn pixel(&self, x: u32, y: u32) -> Option<u32> {
        if x < self.width && y < self.height {
            Some(self.pixels[y as usize * self.width as usize + x as usize])
        } else {
            None
        }
    }

    /// The pixels as BGRA bytes, four per pixel, without copying.
    #[cfg(target_endian = "little")]
    pub fn as_bgra_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.pixels)
    }

    /// The pixels as RGBA bytes, for image encoders such as PNG.
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 4);
        for &p in self.pixels.iter() {
            out.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, (p >> 24) as u8]);
        }
        out
    }

    /// A copy scaled up `factor` times with nearest-neighbour sampling, for
    /// renderers that would otherwise smooth the pixels when they scale.
    ///
    /// # Panics
    ///
    /// When `factor` is zero.
    #[must_use]
    pub fn scaled(&self, factor: u32) -> Frame {
        assert!(factor > 0, "the scale factor must be at least 1");
        if factor == 1 {
            return self.clone();
        }
        let (w, h, f) = (self.width as usize, self.height as usize, factor as usize);
        let mut out = Vec::with_capacity(w * h * f * f);
        for row in self.pixels.chunks_exact(w.max(1)).take(h) {
            let start = out.len();
            for &p in row {
                out.extend(std::iter::repeat_n(p, f));
            }
            let line = out[start..].to_vec();
            for _ in 1..f {
                out.extend_from_slice(&line);
            }
        }
        Frame::new(
            self.width * factor,
            self.height * factor,
            out.into(),
            self.serial,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_reads_row_major() {
        let frame = Frame::new(2, 2, vec![1, 2, 3, 4].into(), 7);
        assert_eq!(frame.pixel(1, 0), Some(2));
        assert_eq!(frame.pixel(0, 1), Some(3));
        assert_eq!(frame.pixel(2, 0), None);
        assert_eq!(frame.serial(), 7);
    }

    #[test]
    #[cfg(target_endian = "little")]
    fn bgra_bytes_follow_argb_words() {
        let frame = Frame::new(1, 1, vec![0xFF11_2233].into(), 0);
        assert_eq!(frame.as_bgra_bytes(), &[0x33, 0x22, 0x11, 0xFF]);
        assert_eq!(frame.to_rgba8(), vec![0x11, 0x22, 0x33, 0xFF]);
    }

    #[test]
    fn scaled_repeats_each_pixel() {
        let frame = Frame::new(2, 1, vec![1, 2].into(), 0);
        let big = frame.scaled(2);
        assert_eq!((big.width(), big.height()), (4, 2));
        assert_eq!(big.pixels(), &[1, 1, 2, 2, 1, 1, 2, 2]);
    }

    #[test]
    #[should_panic(expected = "needs 4 pixels")]
    fn new_checks_the_pixel_count() {
        let _ = Frame::new(2, 2, vec![0; 3].into(), 0);
    }
}
