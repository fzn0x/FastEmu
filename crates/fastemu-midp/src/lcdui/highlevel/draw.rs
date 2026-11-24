//! What the high-level screens draw with: text wrapped to a width, and
//! the small parts of a phone's look (focus frames, check boxes, radio
//! buttons, scroll bars, arrows) in the runtime's theme.

use std::ops::Range;

use fastemu_gfx::anchor::{LEFT, TOP};
use fastemu_gfx::{Face, Font, FontLibrary, Graphics, Size, Style, Surface};

use crate::lcdui::Rect;
use crate::lcdui::theme::Theme;

/// The space between an item's or a row's edge and its text.
pub const PAD: i32 = 3;
/// The width of a scroll bar, at the right edge of the content.
pub const SCROLLBAR: i32 = 4;

/// Whether `unit` is drawn: control characters such as line feeds and
/// carriage returns only steer the wrapping.
fn shown(unit: u16) -> bool {
    unit >= 0x20 && unit != 0x7F
}

/// The units of `text` that are drawn.
pub fn visible(text: &[u16]) -> Vec<u16> {
    text.iter().copied().filter(|&u| shown(u)).collect()
}

/// The width of `text` in `font`, control characters left out.
pub fn width(font: &Font, text: &[u16]) -> i32 {
    text.iter()
        .filter(|&&u| shown(u))
        .map(|&u| font.char_width(u))
        .sum()
}

/// Breaks `text` into lines no wider than `max` pixels: at line feeds,
/// after the last space that fits, and inside a word only when the word
/// alone is wider than the line. Always gives at least one line; each
/// range indexes `text`.
pub fn wrap(font: &Font, text: &[u16], max: i32) -> Vec<Range<usize>> {
    let max = max.max(1);
    let mut lines = Vec::new();
    let (mut start, mut used, mut space) = (0, 0, None);
    let mut i = 0;
    while i < text.len() {
        let unit = text[i];
        if unit == u16::from(b'\n') {
            lines.push(start..i);
            (start, used, space) = (i + 1, 0, None);
            i += 1;
            continue;
        }
        let w = if shown(unit) {
            font.char_width(unit)
        } else {
            0
        };
        if used + w > max && i > start && unit != u16::from(b' ') {
            let end = space.filter(|&s| s > start).unwrap_or(i);
            lines.push(start..end);
            (start, space) = (end, None);
            used = width(font, &text[start..i]);
            continue;
        }
        used += w;
        if unit == u16::from(b' ') {
            space = Some(i + 1);
        }
        i += 1;
    }
    lines.push(start..text.len());
    lines
}

/// `text` cut to fit `max` pixels, with an ellipsis when cut.
pub fn ellipsize(font: &Font, text: &[u16], max: i32) -> Vec<u16> {
    let text = visible(text);
    if width(font, &text) <= max {
        return text;
    }
    let dots = u16::from(b'.');
    let room = max - 3 * font.char_width(dots);
    let mut used = 0;
    let mut out = Vec::new();
    for unit in text {
        used += font.char_width(unit);
        if used > room {
            break;
        }
        out.push(unit);
    }
    out.extend([dots; 3]);
    out
}

/// The fonts the screens use besides the default one.
pub struct Fonts {
    /// The default font: item text, list elements, text fields.
    pub text: Font,
    /// Item labels.
    pub label: Font,
    /// Small notes, such as a text box's input mode.
    pub small: Font,
}

impl Fonts {
    /// The screens' fonts from the instance's library.
    pub fn new(library: &FontLibrary) -> Self {
        Self {
            text: library.default_font(),
            label: library.font(Face::System, Style::BOLD, Size::Medium),
            small: library.font(Face::System, Style::PLAIN, Size::Small),
        }
    }
}

/// Draws on a screen's content area.
pub struct Painter<'a> {
    pub surface: &'a mut Surface,
    pub g: Graphics,
    pub theme: &'a Theme,
    pub fonts: Fonts,
    area: Rect,
}

impl<'a> Painter<'a> {
    /// A painter for the `width` by `height` content area at the top left
    /// of `surface`, clipped to it.
    pub fn new(
        surface: &'a mut Surface,
        width: i32,
        height: i32,
        library: &FontLibrary,
        theme: &'a Theme,
    ) -> Option<Self> {
        let fonts = Fonts::new(library);
        let mut g = Graphics::new(surface, fonts.text.clone()).ok()?;
        let area = Rect::new(0, 0, width, height);
        g.set_clip(0, 0, width, height);
        Some(Self {
            surface,
            g,
            theme,
            fonts,
            area,
        })
    }

    /// The content area.
    pub fn area(&self) -> Rect {
        self.area
    }

    /// Limits drawing to `r` within the content area.
    pub fn clip(&mut self, r: Rect) {
        let r = r.intersect(self.area);
        self.g.set_clip(r.x, r.y, r.width, r.height);
    }

    /// Lets drawing reach the whole content area again.
    pub fn unclip(&mut self) {
        let a = self.area;
        self.g.set_clip(a.x, a.y, a.width, a.height);
    }

    /// Fills `r` with `color`.
    pub fn fill(&mut self, r: Rect, color: i32) {
        self.g.set_color(color);
        self.g.fill_rect(self.surface, r.x, r.y, r.width, r.height);
    }

    /// A one-pixel outline just inside `r`.
    pub fn outline(&mut self, r: Rect, color: i32) {
        self.g.set_color(color);
        self.g
            .draw_rect(self.surface, r.x, r.y, r.width - 1, r.height - 1);
    }

    /// A rounded frame of `thickness` pixels just inside `r`.
    pub fn frame(&mut self, r: Rect, color: i32, thickness: i32) {
        self.g.set_color(color);
        for i in 0..thickness {
            self.g.draw_round_rect(
                self.surface,
                r.x + i,
                r.y + i,
                r.width - 1 - 2 * i,
                r.height - 1 - 2 * i,
                6,
                6,
            );
        }
    }

    /// A filled rounded rectangle.
    pub fn round(&mut self, r: Rect, color: i32) {
        self.g.set_color(color);
        self.g
            .fill_round_rect(self.surface, r.x, r.y, r.width, r.height, 6, 6);
    }

    /// One line of `text` with its top left at `(x, y)`.
    pub fn text(&mut self, font: &Font, text: &[u16], x: i32, y: i32, color: i32) {
        self.g.set_color(color);
        self.g.set_font(Some(font.clone()));
        let _ = self
            .g
            .draw_string(self.surface, &visible(text), x, y, LEFT | TOP);
    }

    /// A line under `width` pixels of text drawn at `(x, y)`.
    pub fn underline(&mut self, font: &Font, x: i32, y: i32, width: i32, color: i32) {
        let base = y + font.baseline_position() + 1;
        self.g.set_color(color);
        self.g.draw_line(self.surface, x, base, x + width - 1, base);
    }

    /// `text` wrapped to `width` from `(x, y)`; returns the height used.
    pub fn paragraph(
        &mut self,
        font: &Font,
        text: &[u16],
        x: i32,
        y: i32,
        width: i32,
        color: i32,
    ) -> i32 {
        let lines = wrap(font, text, width);
        for (i, line) in lines.iter().enumerate() {
            let top = y + i as i32 * font.height();
            self.text(font, &text[line.clone()], x, top, color);
        }
        lines.len() as i32 * font.height()
    }

    /// An image with its top left at `(x, y)`.
    pub fn image(&mut self, image: &Surface, x: i32, y: i32) {
        let _ = self.g.draw_image(self.surface, image, x, y, LEFT | TOP);
    }

    /// A check box of side `side` at `(x, y)`, ticked or not.
    pub fn check_box(&mut self, x: i32, y: i32, side: i32, on: bool, color: i32) {
        self.outline(Rect::new(x, y, side, side), color);
        if on {
            self.g.set_color(color);
            let (s, mid) = (side - 1, side / 2);
            for d in 0..2 {
                self.g.draw_line(
                    self.surface,
                    x + 2,
                    y + mid + d - 1,
                    x + mid - 1,
                    y + s - 3 + d,
                );
                self.g.draw_line(
                    self.surface,
                    x + mid - 1,
                    y + s - 3 + d,
                    x + s - 2,
                    y + 2 + d,
                );
            }
        }
    }

    /// A radio button of diameter `side` at `(x, y)`, chosen or not.
    pub fn radio(&mut self, x: i32, y: i32, side: i32, on: bool, color: i32) {
        self.g.set_color(color);
        self.g
            .draw_arc(self.surface, x, y, side - 1, side - 1, 0, 360);
        if on {
            let inset = (side / 4).max(2);
            self.g.fill_arc(
                self.surface,
                x + inset,
                y + inset,
                side - 2 * inset,
                side - 2 * inset,
                0,
                360,
            );
        }
    }

    /// A small triangle pointing down, centred in `r`.
    pub fn arrow_down(&mut self, r: Rect, color: i32) {
        let half = (r.width.min(r.height) / 3).max(2);
        let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
        self.g.set_color(color);
        self.g.fill_triangle(
            self.surface,
            cx - half,
            cy - half / 2,
            cx + half,
            cy - half / 2,
            cx,
            cy + half / 2 + 1,
        );
    }

    /// A small triangle pointing left (`left`) or right, centred in `r`.
    pub fn arrow_side(&mut self, r: Rect, left: bool, color: i32) {
        let half = (r.width.min(r.height) / 3).max(2);
        let (cx, cy) = (r.x + r.width / 2, r.y + r.height / 2);
        let (tip, back) = if left {
            (cx - half / 2 - 1, cx + half / 2)
        } else {
            (cx + half / 2 + 1, cx - half / 2)
        };
        self.g.set_color(color);
        self.g
            .fill_triangle(self.surface, back, cy - half, back, cy + half, tip, cy);
    }

    /// A scroll bar along the right edge of the content area for a view
    /// of `view` pixels at `offset` into `total`.
    pub fn scrollbar(&mut self, total: i32, view: i32, offset: i32) {
        if total <= view || view <= 0 {
            return;
        }
        let a = self.area;
        let track = Rect::new(a.x + a.width - SCROLLBAR, a.y, SCROLLBAR, a.height);
        self.fill(track, blend(self.theme.background, self.theme.border));
        let length = (i64::from(a.height) * i64::from(view) / i64::from(total)).max(8) as i32;
        let room = a.height - length;
        let top = (i64::from(room) * i64::from(offset) / i64::from((total - view).max(1))) as i32;
        let thumb = Rect::new(track.x, a.y + top.clamp(0, room.max(0)), SCROLLBAR, length);
        self.fill(thumb, self.theme.border);
    }
}

/// Halfway between two `0xRRGGBB` colours.
pub fn blend(a: i32, b: i32) -> i32 {
    let channel = |shift: i32| i32::midpoint((a >> shift) & 0xFF, (b >> shift) & 0xFF) << shift;
    channel(16) | channel(8) | channel(0)
}

#[cfg(test)]
mod tests {
    use fastemu_core::FontSizes;
    use fastemu_gfx::TextRendering;

    use super::*;

    fn units(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn font() -> Font {
        let sizes = FontSizes {
            small: 12,
            medium: 14,
            large: 16,
        };
        FontLibrary::new(sizes, TextRendering::Antialiased).default_font()
    }

    fn lines(text: &str, max: i32) -> Vec<String> {
        let text = units(text);
        wrap(&font(), &text, max)
            .into_iter()
            .map(|r| String::from_utf16_lossy(&text[r]).trim_end().to_owned())
            .collect()
    }

    #[test]
    fn wrapping_breaks_after_spaces_and_at_line_feeds() {
        let font = font();
        let word = width(&font, &units("hello "));
        assert_eq!(lines("hello world", word + 5), ["hello", "world"]);
        assert_eq!(lines("hello world", 1000), ["hello world"]);
        assert_eq!(lines("a\nb", 1000), ["a", "b"]);
        assert_eq!(lines("", 10), [""]);
        assert_eq!(lines("\n", 10), ["", ""]);
    }

    #[test]
    fn a_long_word_breaks_inside() {
        let font = font();
        let max = width(&font, &units("abc"));
        let broken = lines("abcdefgh", max);
        assert!(broken.len() >= 3, "{broken:?}");
        assert_eq!(broken.concat(), "abcdefgh");
        for line in &broken {
            assert!(width(&font, &units(line)) <= max, "{line}");
        }
    }

    #[test]
    fn every_unit_lands_on_a_line_once() {
        let text = units("one two  three\nfour five six seven eight");
        let ranges = wrap(&font(), &text, 40);
        let mut covered = 0;
        for r in &ranges {
            assert!(r.start >= covered, "{ranges:?}");
            covered = r.end;
        }
        assert_eq!(covered, text.len());
    }

    #[test]
    fn ellipsizing_keeps_short_text() {
        let font = font();
        let short = units("ok");
        assert_eq!(ellipsize(&font, &short, 100), short);
        let cut = ellipsize(&font, &units("a rather long element"), 40);
        assert!(width(&font, &cut) <= 40);
        assert!(cut.ends_with(&units("...")));
    }

    #[test]
    fn blending_halves_each_channel() {
        assert_eq!(blend(0xFF_FFFF, 0x00_0000), 0x7F_7F7F);
        assert_eq!(blend(0x20_4060, 0x20_4060), 0x20_4060);
    }
}
