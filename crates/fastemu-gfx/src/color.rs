//! ARGB colours and source-over blending in 8-bit fixed point.
//!
//! Colours are `0xAARRGGBB` words that are not premultiplied, the layout of
//! [`fastemu_core::Frame`] and of the `int` pixels MIDP's `getRGB` and
//! `drawRGB` exchange.

/// The alpha bits of a fully opaque colour.
pub const OPAQUE: u32 = 0xFF00_0000;

/// `x / 255`, rounded to nearest, exact for every `x` up to `255 * 255`.
#[inline]
pub(crate) const fn div255(x: u32) -> u32 {
    let x = x + 128;
    (x + (x >> 8)) >> 8
}

/// Draws `src` over `dst` (Porter-Duff source-over) and returns the result.
///
/// An opaque source replaces the destination, a fully transparent one
/// leaves it, and an opaque destination stays opaque, so the screen and
/// mutable images never gain transparency from drawing.
#[inline]
pub fn blend(dst: u32, src: u32) -> u32 {
    let sa = src >> 24;
    if sa == 0xFF {
        return src;
    }
    if sa == 0 {
        return dst;
    }
    let da = dst >> 24;
    if da == 0xFF {
        blend_onto_opaque(dst, src, sa)
    } else {
        blend_onto_translucent(dst, src, sa, da)
    }
}

/// Source-over onto an opaque destination, red and blue in one word.
#[inline]
fn blend_onto_opaque(dst: u32, src: u32, sa: u32) -> u32 {
    let ia = 255 - sa;
    let rb = (src & 0x00FF_00FF) * sa + (dst & 0x00FF_00FF) * ia + 0x0080_0080;
    let rb = ((rb + ((rb >> 8) & 0x00FF_00FF)) >> 8) & 0x00FF_00FF;
    let g = (src & 0x0000_FF00) * sa + (dst & 0x0000_FF00) * ia + 0x0000_8000;
    let g = ((g + ((g >> 8) & 0x0000_FF00)) >> 8) & 0x0000_FF00;
    OPAQUE | rb | g
}

/// Source-over onto a destination that is itself partly transparent, which
/// only Nokia's transparent mutable images have.
///
/// The weights keep a factor of 255 so faint colours do not lose their hue
/// to rounding.
fn blend_onto_translucent(dst: u32, src: u32, sa: u32, da: u32) -> u32 {
    let ws = sa * 255;
    let wd = da * (255 - sa);
    let total = ws + wd;
    let channel = |shift: u32| {
        let s = (src >> shift) & 0xFF;
        let d = (dst >> shift) & 0xFF;
        ((s * ws + d * wd + total / 2) / total) << shift
    };
    (div255(total) << 24) | channel(16) | channel(8) | channel(0)
}

/// `color` with its alpha scaled by `coverage` (0 to 255), for the
/// anti-aliased edges of glyphs.
#[inline]
pub(crate) fn with_coverage(color: u32, coverage: u8) -> u32 {
    let alpha = div255((color >> 24) * u32::from(coverage));
    (alpha << 24) | (color & 0x00FF_FFFF)
}

/// The grey level (0 to 255) of an RGB colour: Rec. 601 luma, rounded, so
/// white is 255 and black is 0.
pub fn gray_of(rgb: u32) -> u8 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    ((r * 299 + g * 587 + b * 114 + 500) / 1000) as u8
}

/// The colour a display with `num_colors` colours shows for `color`, as
/// `Graphics.getDisplayColor` reports it. The alpha byte is ignored.
///
/// Displays with at least 256 colours keep the top bits of each channel
/// (8-8-8, 6-6-6, 5-6-5, 4-4-4 or 3-3-2) and widen them back by repeating
/// the bits; smaller counts are grey levels, as on monochrome phones.
pub fn display_color(color: i32, num_colors: u32) -> i32 {
    let rgb = color as u32 & 0x00FF_FFFF;
    let bits = match num_colors {
        n if n >= 1 << 24 => return rgb as i32,
        n if n >= 1 << 18 => (6, 6, 6),
        n if n >= 1 << 16 => (5, 6, 5),
        n if n >= 1 << 12 => (4, 4, 4),
        n if n >= 256 => (3, 3, 2),
        n => {
            let levels = n.max(2) - 1;
            let level = (u32::from(gray_of(rgb)) * levels + 127) / 255;
            let gray = level * 255 / levels;
            return (gray * 0x0001_0101) as i32;
        }
    };
    let r = widen((rgb >> 16) & 0xFF, bits.0);
    let g = widen((rgb >> 8) & 0xFF, bits.1);
    let b = widen(rgb & 0xFF, bits.2);
    ((r << 16) | (g << 8) | b) as i32
}

/// Keeps the top `bits` of an 8-bit channel and repeats them to fill eight.
fn widen(channel: u32, bits: u32) -> u32 {
    let top = channel >> (8 - bits);
    let mut out = 0;
    let mut filled = 0;
    while filled < 8 {
        out = (out << bits) | top;
        filled += bits;
    }
    out >> (filled - 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Source-over computed channel by channel in floating point.
    fn reference(dst: u32, src: u32) -> u32 {
        let sa = f64::from(src >> 24) / 255.0;
        let da = f64::from(dst >> 24) / 255.0;
        let oa = sa + da * (1.0 - sa);
        if oa == 0.0 {
            return 0;
        }
        let ch = |shift: u32| {
            let s = f64::from((src >> shift) & 0xFF);
            let d = f64::from((dst >> shift) & 0xFF);
            ((s * sa + d * da * (1.0 - sa)) / oa).round() as u32
        };
        ((oa * 255.0).round() as u32) << 24 | ch(16) << 16 | ch(8) << 8 | ch(0)
    }

    fn close(a: u32, b: u32) -> bool {
        (0..4).all(|i| {
            let x = (a >> (i * 8)) & 0xFF;
            let y = (b >> (i * 8)) & 0xFF;
            x.abs_diff(y) <= 1
        })
    }

    #[test]
    fn div255_rounds_exactly() {
        for x in 0..=255 * 255 {
            assert_eq!(div255(x), (x * 2 + 255) / 510, "{x}");
        }
    }

    #[test]
    fn opaque_and_clear_sources_short_circuit() {
        assert_eq!(blend(0xFF12_3456, 0xFFAB_CDEF), 0xFFAB_CDEF);
        assert_eq!(blend(0xFF12_3456, 0x00AB_CDEF), 0xFF12_3456);
        assert_eq!(blend(0x0000_0000, 0x80FF_0000), 0x80FF_0000);
    }

    #[test]
    fn blending_onto_opaque_matches_the_reference() {
        for sa in (1..255).step_by(7) {
            for s in (0..=255).step_by(15) {
                for d in (0..=255).step_by(17) {
                    let src = sa << 24 | s << 16 | (255 - s) << 8 | (s ^ 0x5A);
                    let dst = OPAQUE | d << 16 | (d / 2) << 8 | (255 - d);
                    let got = blend(dst, src);
                    let want = reference(dst, src);
                    assert!(
                        close(got, want),
                        "{dst:08X} {src:08X}: {got:08X} {want:08X}"
                    );
                    assert_eq!(got >> 24, 0xFF);
                }
            }
        }
    }

    #[test]
    fn half_alpha_lands_halfway() {
        assert_eq!(blend(0xFF00_0000, 0x80FF_FFFF), 0xFF80_8080);
        assert_eq!(blend(0xFFFF_FFFF, 0x8000_0000), 0xFF7F_7F7F);
    }

    #[test]
    fn blending_onto_translucent_matches_the_reference() {
        for sa in (1..255).step_by(11) {
            for da in (0..255).step_by(13) {
                let src = sa << 24 | 0x00C0_4010;
                let dst = da << 24 | 0x0010_80F0;
                let got = blend(dst, src);
                let want = reference(dst, src);
                assert!(
                    close(got, want),
                    "{dst:08X} {src:08X}: {got:08X} {want:08X}"
                );
            }
        }
    }

    #[test]
    fn coverage_scales_alpha_only() {
        assert_eq!(with_coverage(0xFF11_2233, 255), 0xFF11_2233);
        assert_eq!(with_coverage(0xFF11_2233, 0), 0x0011_2233);
        assert_eq!(with_coverage(0x8011_2233, 128) >> 24, 0x40);
    }

    #[test]
    fn gray_levels_span_the_full_range() {
        assert_eq!(gray_of(0xFF_FFFF), 255);
        assert_eq!(gray_of(0), 0);
        assert_eq!(gray_of(0xFF_0000), 76);
        assert_eq!(gray_of(0x80_8080), 128);
    }

    #[test]
    fn display_colors_follow_the_depth() {
        assert_eq!(display_color(0x7F12_3456, 1 << 24), 0x12_3456);
        assert_eq!(display_color(0x12_3456, 1 << 16), 0x10_3452);
        assert_eq!(display_color(0xFF_FFFF, 4096), 0xFF_FFFF);
        assert_eq!(display_color(0x12_3456, 4096), 0x11_3355);
        assert_eq!(display_color(0x80_8080, 2), 0xFF_FFFF);
        assert_eq!(display_color(0x40_4040, 2), 0);
        assert_eq!(display_color(0x80_8080, 4), 0xAA_AAAA);
    }
}
