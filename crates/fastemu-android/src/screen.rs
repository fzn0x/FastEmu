//! Drawing a phone's screen crisply: an integer zoom, placed on whole device
//! pixels, with every pixel repeated rather than smoothed.
//!
//! GPUI samples images with a linear filter, which blurs pixel art drawn at
//! any size but its own. So the app scales a frame up by a whole number
//! itself, nearest neighbour, and draws the result one image pixel to one
//! device pixel.

use fastemu_core::Frame;

/// Where a phone screen goes in an area, in device pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    /// How many device pixels wide and tall each phone pixel is. Below 1
    /// only when the area is smaller than the phone's screen, which then
    /// shrinks to fit.
    pub zoom: f32,
    /// The top-left corner, on a whole device pixel.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Placement {
    /// The whole-number factor to scale the frame up by before drawing it:
    /// the zoom when the screen fits, and 1 when it has to shrink.
    pub fn upscale(&self) -> u32 {
        if self.zoom >= 1.0 {
            self.zoom as u32
        } else {
            1
        }
    }

    /// The phone pixel under the device pixel position (`x`, `y`), or `None`
    /// outside the screen: where a touch or a click lands in the game.
    pub fn to_phone(&self, x: f32, y: f32) -> Option<(i32, i32)> {
        let inside =
            x >= self.x && y >= self.y && x < self.x + self.width && y < self.y + self.height;
        if !inside || self.zoom <= 0.0 {
            return None;
        }
        Some((
            ((x - self.x) / self.zoom) as i32,
            ((y - self.y) / self.zoom) as i32,
        ))
    }
}

/// Centres a `screen_width` by `screen_height` phone screen in the area at
/// (`area_x`, `area_y`) of `area_width` by `area_height`, all in device
/// pixels, at the largest whole zoom that fits.
pub fn place(
    (screen_width, screen_height): (u32, u32),
    (area_x, area_y): (f32, f32),
    (area_width, area_height): (f32, f32),
) -> Placement {
    let (screen_width, screen_height) = (screen_width.max(1) as f32, screen_height.max(1) as f32);
    let fit = (area_width / screen_width)
        .min(area_height / screen_height)
        .max(0.0);
    let zoom = if fit >= 1.0 { fit.floor() } else { fit };
    let (width, height) = (screen_width * zoom, screen_height * zoom);
    Placement {
        zoom,
        x: (area_x + (area_width - width) / 2.0).round(),
        y: (area_y + (area_height - height) / 2.0).round(),
        width,
        height,
    }
}

/// `frame` scaled up `factor` times, nearest neighbour, as BGRA bytes: the
/// layout GPUI's images take. One pass, with no intermediate frame.
///
/// # Panics
///
/// When `factor` is zero.
pub fn upscaled_bgra(frame: &Frame, factor: u32) -> Vec<u8> {
    assert!(factor > 0, "the scale factor must be at least 1");
    let (width, factor) = (frame.width() as usize, factor as usize);
    let row_bytes = width * factor * 4;
    let mut out = Vec::with_capacity(row_bytes * frame.height() as usize * factor);
    for row in frame.pixels().chunks_exact(width.max(1)) {
        let start = out.len();
        for &pixel in row {
            // 0xAARRGGBB in little-endian order is B, G, R, A.
            let bgra = pixel.to_le_bytes();
            for _ in 0..factor {
                out.extend_from_slice(&bgra);
            }
        }
        for _ in 1..factor {
            out.extend_from_within(start..start + row_bytes);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zoom_is_the_largest_whole_number_that_fits() {
        // 240x320 in 1080x1200: 4.5 across, 3.75 down.
        let placed = place((240, 320), (0.0, 0.0), (1080.0, 1200.0));
        assert_eq!(placed.zoom, 3.0);
        assert_eq!((placed.width, placed.height), (720.0, 960.0));
        assert_eq!(placed.upscale(), 3);
    }

    #[test]
    fn the_screen_is_centred_on_whole_pixels() {
        // 361 spare pixels across: 180.5 on each side, rounded.
        let placed = place((240, 320), (10.0, 20.0), (1081.0, 1200.0));
        assert_eq!((placed.x, placed.y), (191.0, 140.0));
    }

    #[test]
    fn positions_map_back_to_phone_pixels() {
        let placed = place((240, 320), (0.0, 0.0), (720.0, 960.0));
        assert_eq!(placed.zoom, 3.0);
        assert_eq!(placed.to_phone(0.0, 0.0), Some((0, 0)));
        assert_eq!(placed.to_phone(719.9, 959.9), Some((239, 319)));
        assert_eq!(placed.to_phone(300.0, 31.0), Some((100, 10)));
        assert_eq!(placed.to_phone(720.0, 10.0), None);
        assert_eq!(placed.to_phone(-1.0, 10.0), None);
    }

    #[test]
    fn a_small_area_shrinks_the_screen_to_fit() {
        let placed = place((240, 320), (0.0, 0.0), (120.0, 400.0));
        assert_eq!(placed.zoom, 0.5);
        assert_eq!((placed.width, placed.height), (120.0, 160.0));
        assert_eq!(placed.upscale(), 1);
    }

    #[test]
    fn an_empty_area_draws_nothing() {
        let placed = place((240, 320), (0.0, 0.0), (0.0, 0.0));
        assert_eq!((placed.width, placed.height), (0.0, 0.0));
        let placed = place((0, 0), (0.0, 0.0), (-5.0, 10.0));
        assert_eq!(placed.zoom, 0.0);
    }

    #[test]
    fn upscaling_repeats_each_pixel_like_the_core_frame() {
        let frame = Frame::new(3, 2, vec![0xFF11_2233, 2, 3, 4, 5, 0x80AA_BBCC].into(), 9);
        for factor in [1, 2, 3] {
            let ours = upscaled_bgra(&frame, factor);
            let expected = frame.scaled(factor).as_bgra_bytes().to_vec();
            assert_eq!(ours, expected, "factor {factor}");
        }
        assert_eq!(&upscaled_bgra(&frame, 1)[..4], &[0x33, 0x22, 0x11, 0xFF]);
    }

    #[test]
    #[should_panic(expected = "at least 1")]
    fn upscaling_by_zero_is_a_bug() {
        let _ = upscaled_bgra(&Frame::filled(1, 1, 0, 0), 0);
    }
}
