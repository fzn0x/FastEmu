//! Text: `drawString`, `drawSubstring`, `drawChar` and `drawChars`.
//!
//! Strings are UTF-16 units, as Java holds them, so offsets and lengths
//! mean what they mean in Java. Each unit draws one glyph at the sum of the
//! advances before it, the same sums [`Font::string_width`] adds up.

use super::Graphics;
use crate::anchor::Anchor;
use crate::color::{blend, with_coverage};
use crate::error::{GfxError, Result};
use crate::font::{Font, Glyph, TextRendering};
use crate::raster::Canvas;
use crate::surface::{Surface, java_range};

/// The slant of synthesized italics: one pixel right for every four rows
/// above the baseline.
const ITALIC_RISE: i64 = 4;

impl Graphics {
    /// `drawString(str, x, y, anchor)` in the current font and colour.
    /// The anchor takes `LEFT`, `HCENTER` or `RIGHT` with `TOP`,
    /// `BASELINE` or `BOTTOM`; `VCENTER` is `IllegalArgumentException`.
    pub fn draw_string(
        &self,
        target: &mut Surface,
        text: &[u16],
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        self.draw_text(target, text, x, y, anchor)
    }

    /// `drawSubstring(str, offset, len, x, y, anchor)`. A range outside
    /// `text` is `StringIndexOutOfBoundsException`.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_substring(
        &self,
        target: &mut Surface,
        text: &[u16],
        offset: i32,
        len: i32,
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        let range = java_range(text.len(), offset, len).ok_or(GfxError::StringIndexOutOfBounds)?;
        self.draw_text(target, &text[range], x, y, anchor)
    }

    /// `drawChar(character, x, y, anchor)`.
    pub fn draw_char(
        &self,
        target: &mut Surface,
        character: u16,
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        self.draw_text(target, &[character], x, y, anchor)
    }

    /// `drawChars(data, offset, length, x, y, anchor)`. A range outside
    /// `data` is `ArrayIndexOutOfBoundsException`.
    #[allow(clippy::too_many_arguments)] // MIDP's signature
    pub fn draw_chars(
        &self,
        target: &mut Surface,
        data: &[u16],
        offset: i32,
        length: i32,
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        let range =
            java_range(data.len(), offset, length).ok_or(GfxError::ArrayIndexOutOfBounds)?;
        self.draw_text(target, &data[range], x, y, anchor)
    }

    /// Draws a Rust string, for callers outside the Java wrapper such as
    /// the high-level MIDP screens.
    pub fn draw_str(
        &self,
        target: &mut Surface,
        text: &str,
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        let units: Vec<u16> = text.encode_utf16().collect();
        self.draw_text(target, &units, x, y, anchor)
    }

    fn draw_text(
        &self,
        target: &mut Surface,
        units: &[u16],
        x: i32,
        y: i32,
        anchor: i32,
    ) -> Result<()> {
        let anchor = Anchor::for_text(anchor, self.anchors)?;
        let font = &self.font;
        let face = font.sized();
        let width = i64::from(font.string_width(units));
        let (ax, ay) = self.at(x, y);
        let (left, top) = anchor.top_left(ax, ay, width, face.height.into(), face.ascent.into());
        let baseline = top + i64::from(face.ascent);
        let mut canvas = self.canvas(target);
        if canvas.clip.is_empty() {
            return Ok(());
        }
        let style = GlyphStyle {
            color: self.color,
            italic: font.is_italic(),
            aliased: font.rendering() == TextRendering::Aliased,
            baseline,
        };
        // Italic glyphs lean right by up to a quarter of the ascent.
        let reach = canvas.clip.x1 + i64::from(face.ascent) / ITALIC_RISE + 1;
        let mut pen = left;
        for &unit in units {
            if pen >= reach {
                break;
            }
            let advance = face.with_glyph(unit, |glyph| {
                draw_glyph(&mut canvas, glyph, pen, &style);
                glyph.advance
            });
            pen += i64::from(advance);
        }
        if font.is_underlined() {
            draw_underline(&mut canvas, font, left, width, baseline);
        }
        Ok(())
    }
}

/// How glyphs of one string are drawn.
struct GlyphStyle {
    color: u32,
    italic: bool,
    aliased: bool,
    baseline: i64,
}

fn draw_glyph(canvas: &mut Canvas<'_>, glyph: &Glyph, pen: i64, style: &GlyphStyle) {
    if glyph.width == 0 {
        return;
    }
    let clip = canvas.clip;
    let top = style.baseline + i64::from(glyph.top);
    let rows = glyph.coverage.chunks_exact(glyph.width);
    for (y, coverage) in (top..).zip(rows) {
        if y < clip.y0 {
            continue;
        }
        if y >= clip.y1 {
            break;
        }
        let slant = if style.italic {
            (style.baseline - 1 - y).div_euclid(ITALIC_RISE)
        } else {
            0
        };
        let x0 = pen + i64::from(glyph.left) + slant;
        let first = (clip.x0 - x0).clamp(0, glyph.width as i64);
        let last = (clip.x1 - x0).clamp(first, glyph.width as i64);
        if first == last {
            continue;
        }
        let run = canvas.row(y, x0 + first, x0 + last);
        for (dst, &c) in run.iter_mut().zip(&coverage[first as usize..last as usize]) {
            let c = if style.aliased {
                if c >= 128 { 255 } else { 0 }
            } else {
                c
            };
            match c {
                0 => {}
                255 => *dst = blend(*dst, style.color),
                _ => *dst = blend(*dst, with_coverage(style.color, c)),
            }
        }
    }
}

/// A one-pixel line under the text, a row below the baseline when the
/// descent has room for the gap.
fn draw_underline(canvas: &mut Canvas<'_>, font: &Font, left: i64, width: i64, baseline: i64) {
    let descent = font.height() - font.baseline_position();
    let y = if descent >= 2 { baseline + 1 } else { baseline };
    canvas.span(y, left, left + width);
}

#[cfg(test)]
mod tests {
    use crate::anchor::{BASELINE, BOTTOM, HCENTER, LEFT, RIGHT, TOP, VCENTER};
    use crate::error::GfxError;
    use crate::font::{Face, FontLibrary, Size, Style, TextRendering};
    use crate::graphics::Graphics;
    use crate::surface::Surface;
    use fastemu_core::FontSizes;

    const SIZES: FontSizes = FontSizes {
        small: 12,
        medium: 14,
        large: 16,
    };

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn canvas(w: i32, h: i32) -> (Surface, Graphics) {
        let surface = Surface::new_mutable(w, h).unwrap();
        let g = crate::testing::graphics(&surface);
        (surface, g)
    }

    /// The columns and rows any non-white pixel occupies, inclusive.
    fn ink(surface: &Surface) -> Option<(u32, u32, u32, u32)> {
        let w = surface.width();
        let mut bounds: Option<(u32, u32, u32, u32)> = None;
        for (i, &p) in surface.pixels().iter().enumerate() {
            if p != 0xFFFF_FFFF {
                let (x, y) = (i as u32 % w, i as u32 / w);
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
                });
            }
        }
        bounds
    }

    #[test]
    fn strings_draw_glyph_by_glyph_at_their_widths() {
        let (mut whole, g) = canvas(120, 20);
        let text = utf16("Hi Жλ!");
        g.draw_string(&mut whole, &text, 2, 2, 0).unwrap();
        let (mut pieces, _) = canvas(120, 20);
        let mut x = 2;
        for &unit in &text {
            g.draw_char(&mut pieces, unit, x, 2, TOP | LEFT).unwrap();
            x += g.font().char_width(unit);
        }
        assert_eq!(whole, pieces);
        assert_eq!(x - 2, g.font().string_width(&text));
    }

    #[test]
    fn anchors_place_text_by_width_height_and_baseline() {
        let text = utf16("Mag");
        let (mut reference, g) = canvas(100, 40);
        let font = g.font().clone();
        let (w, h, b) = (
            font.string_width(&text),
            font.height(),
            font.baseline_position(),
        );
        g.draw_string(&mut reference, &text, 10, 5, TOP | LEFT)
            .unwrap();
        for (anchor, x, y) in [
            (TOP | RIGHT, 10 + w, 5),
            (TOP | HCENTER, 10 + w / 2, 5),
            (BASELINE | LEFT, 10, 5 + b),
            (BOTTOM | LEFT, 10, 5 + h),
        ] {
            let (mut s, _) = canvas(100, 40);
            g.draw_string(&mut s, &text, x, y, anchor).unwrap();
            assert_eq!(s, reference, "anchor {anchor}");
        }
        let (mut s, _) = canvas(100, 40);
        assert!(matches!(
            g.draw_string(&mut s, &text, 0, 0, VCENTER | LEFT),
            Err(GfxError::IllegalArgument(_))
        ));
    }

    #[test]
    fn text_stays_inside_its_line() {
        let (mut s, g) = canvas(60, 30);
        g.draw_str(&mut s, "Ágy", 5, 5, 0).unwrap();
        let (x0, y0, _, y1) = ink(&s).unwrap();
        assert!(x0 >= 5);
        assert!(y0 >= 5 && y1 < 5 + g.font().height() as u32);
    }

    #[test]
    fn substrings_and_char_ranges_are_checked() {
        let (mut a, g) = canvas(80, 20);
        let text = utf16("abcdef");
        g.draw_substring(&mut a, &text, 2, 3, 0, 0, 0).unwrap();
        let (mut b, _) = canvas(80, 20);
        g.draw_str(&mut b, "cde", 0, 0, 0).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            g.draw_substring(&mut a, &text, 4, 3, 0, 0, 0),
            Err(GfxError::StringIndexOutOfBounds)
        );
        assert_eq!(
            g.draw_chars(&mut a, &text, -1, 2, 0, 0, 0),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        let (mut c, _) = canvas(80, 20);
        g.draw_chars(&mut c, &text, 2, 3, 0, 0, 0).unwrap();
        assert_eq!(c, b);
    }

    #[test]
    fn missing_characters_draw_boxes_as_wide_as_measured() {
        let (mut s, g) = canvas(40, 20);
        let unit = 0x4E2D;
        g.draw_char(&mut s, unit, 0, 0, 0).unwrap();
        let (x0, _, x1, _) = ink(&s).unwrap();
        assert!(x0 >= 1 && (x1 as i32) < g.font().char_width(unit));
    }

    #[test]
    fn underline_spans_the_width() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Aliased);
        let (mut s, mut g) = canvas(60, 20);
        g.set_font(Some(fonts.font(
            Face::System,
            Style::UNDERLINED,
            Size::Medium,
        )));
        g.draw_str(&mut s, "  ", 3, 0, 0).unwrap();
        let w = g.font().str_width("  ") as u32;
        let (x0, y0, x1, y1) = ink(&s).unwrap();
        assert_eq!((x0, x1, y0, y1), (3, 3 + w - 1, y0, y0));
        assert!(y0 > g.font().baseline_position() as u32 - 1);
    }

    #[test]
    fn italic_leans_right() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Aliased);
        let (mut upright, mut g) = canvas(40, 20);
        g.set_font(Some(fonts.font(Face::Monospace, Style::PLAIN, Size::Large)));
        g.draw_str(&mut upright, "l", 5, 0, 0).unwrap();
        let (mut slanted, _) = canvas(40, 20);
        g.set_font(Some(fonts.font(
            Face::Monospace,
            Style::ITALIC,
            Size::Large,
        )));
        g.draw_str(&mut slanted, "l", 5, 0, 0).unwrap();
        let (_, top_u, _, bottom_u) = ink(&upright).unwrap();
        let (_, top_s, _, bottom_s) = ink(&slanted).unwrap();
        assert_eq!((top_u, bottom_u), (top_s, bottom_s));
        // The top row moves right; the row on the baseline stays.
        let first_ink = |s: &Surface, y: u32| (0..40).find(|&x| s.pixel(x, y) != Some(0xFFFF_FFFF));
        assert!(first_ink(&slanted, top_s) > first_ink(&upright, top_u));
        let base = g.font().baseline_position() as u32 - 1;
        assert_eq!(first_ink(&slanted, base), first_ink(&upright, base));
    }

    #[test]
    fn aliased_text_uses_only_the_colour() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Aliased);
        let (mut s, mut g) = canvas(60, 20);
        g.set_font(Some(fonts.default_font()));
        g.set_color(0x00_FF00);
        g.draw_str(&mut s, "Sharp", 0, 0, 0).unwrap();
        assert!(
            s.pixels()
                .iter()
                .all(|&p| p == 0xFFFF_FFFF || p == 0xFF00_FF00)
        );
        let (mut t, mut g) = canvas(60, 20);
        g.set_color(0x00_FF00);
        g.draw_str(&mut t, "Smooth", 0, 0, 0).unwrap();
        assert!(
            t.pixels()
                .iter()
                .any(|&p| p != 0xFFFF_FFFF && p != 0xFF00_FF00)
        );
    }

    #[test]
    fn clipped_and_far_text_is_cheap_and_safe() {
        let (mut s, mut g) = canvas(10, 10);
        let long = vec![u16::from(b'W'); 100_000];
        g.draw_string(&mut s, &long, -5, 0, 0).unwrap();
        g.set_clip(0, 0, 0, 0);
        g.draw_string(&mut s, &long, 0, 0, RIGHT | BOTTOM).unwrap();
        g.set_font(None);
        assert_eq!(g.font(), &crate::testing::font());
    }
}
