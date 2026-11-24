//! Properties every drawing call must keep, checked on many random calls:
//! the clip restricts drawing exactly, translation only moves it, and
//! font metrics match what text covers.

use fastemu_core::FontSizes;
use fastemu_gfx::anchor::{BASELINE, BOTTOM, HCENTER, LEFT, RIGHT, TOP};
use fastemu_gfx::{Face, Font, FontLibrary, Graphics, Size, Style, Surface, TextRendering};

/// A small deterministic random source (xorshift), so failures repeat.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// A value in `lo..hi`.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % (hi - lo) as u64) as i32
    }
}

const W: i32 = 40;
const H: i32 = 30;

fn fonts() -> FontLibrary {
    let sizes = FontSizes {
        small: 10,
        medium: 13,
        large: 17,
    };
    FontLibrary::new(sizes, TextRendering::Antialiased)
}

/// Draws one random call, described by `seed`, with `g` onto `s`.
fn draw_random(g: &Graphics, s: &mut Surface, seed: u64, sprite: &Surface) {
    let mut r = Rng(seed);
    let mut c = || r.range(-12, 52);
    let (a, b, cc, d, e, f) = (c(), c(), c(), c(), c(), c());
    match seed % 12 {
        0 => g.draw_line(s, a, b, cc, d),
        1 => g.fill_rect(s, a, b, cc - 10, d - 10),
        2 => g.draw_rect(s, a, b, cc - 10, d - 10),
        3 => g.draw_arc(s, a, b, cc, d, e * 7, f * 9),
        4 => g.fill_arc(s, a, b, cc, d, e * 7, f * 9),
        5 => g.draw_round_rect(s, a, b, cc, d, e, f),
        6 => g.fill_round_rect(s, a, b, cc, d, e, f),
        7 => g.fill_triangle(s, a, b, cc, d, e, f),
        8 => g
            .draw_region(s, sprite, 0, 0, 5, 3, (e & 7).abs(), a, b, 0)
            .unwrap(),
        9 => g.draw_str(s, "Ag!Жλ", a, b, TOP | LEFT).unwrap(),
        10 => {
            let rgb: Vec<i32> = (0..12)
                .map(|i| (0x80FF_0000_u32 as i32) ^ (i * 0x1357))
                .collect();
            g.draw_rgb(s, &rgb, 0, 4, a, b, 4, 3, true).unwrap();
        }
        _ => g
            .fill_polygon(
                s,
                &[a, cc, e, b],
                0,
                &[b, d, f, a],
                0,
                4,
                0xC000_FF00_u32 as i32,
            )
            .unwrap(),
    }
}

fn sprite() -> Surface {
    let rgb: Vec<i32> = (0..15)
        .map(|i| (0xFF00_0000_u32 as i32) | (i * 0x0F0F0F))
        .collect();
    let mut rgb = rgb;
    rgb[4] = 0x4000_00FF;
    Surface::from_rgb(&rgb, 5, 3, true).unwrap()
}

#[test]
fn the_clip_restricts_drawing_exactly() {
    let fonts = fonts();
    let sprite = sprite();
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for seed in 1..600_u64 {
        let mut whole = Surface::new_mutable(W, H).unwrap();
        let mut g = Graphics::new(&whole, fonts.default_font()).unwrap();
        g.set_color(0x10_2030);
        draw_random(&g, &mut whole, seed, &sprite);
        let (cx, cy) = (rng.range(-5, W), rng.range(-5, H));
        let (cw, ch) = (rng.range(0, 30), rng.range(0, 25));
        let mut clipped = Surface::new_mutable(W, H).unwrap();
        g.set_clip(cx, cy, cw, ch);
        draw_random(&g, &mut clipped, seed, &sprite);
        for y in 0..H {
            for x in 0..W {
                let inside = x >= cx && x < cx + cw && y >= cy && y < cy + ch;
                let want = if inside {
                    whole.pixel(x as u32, y as u32)
                } else {
                    Some(0xFFFF_FFFF)
                };
                assert_eq!(
                    clipped.pixel(x as u32, y as u32),
                    want,
                    "call {seed} at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn translation_only_moves_drawing() {
    let fonts = fonts();
    let sprite = sprite();
    for seed in 1..300_u64 {
        let (tx, ty) = ((seed % 7) as i32 - 3, (seed % 5) as i32 - 2);
        // Draw on a large surface twice, once translated, and compare the
        // shifted results.
        let mut plain = Surface::new_mutable(W + 20, H + 20).unwrap();
        let mut g = Graphics::new(&plain, fonts.default_font()).unwrap();
        g.translate(10, 10);
        draw_random(&g, &mut plain, seed, &sprite);
        let mut moved = Surface::new_mutable(W + 20, H + 20).unwrap();
        g.translate(tx, ty);
        draw_random(&g, &mut moved, seed, &sprite);
        for y in 3..H + 17 {
            for x in 3..W + 17 {
                let shifted = moved.pixel((x + tx) as u32, (y + ty) as u32);
                assert_eq!(
                    shifted,
                    plain.pixel(x as u32, y as u32),
                    "call {seed} at ({x}, {y})"
                );
            }
        }
    }
}

fn every_font(fonts: &FontLibrary) -> Vec<Font> {
    let mut all = Vec::new();
    for face in [Face::System, Face::Monospace, Face::Proportional] {
        for style in 0..8 {
            for size in [Size::Small, Size::Medium, Size::Large] {
                all.push(fonts.font(face, Style::from_midp(style).unwrap(), size));
            }
        }
    }
    all
}

#[test]
fn metrics_match_drawing_for_every_font() {
    let fonts = fonts();
    let text: Vec<u16> = "Wy, Ёж! Ωψ 07 \u{4E2D}".encode_utf16().collect();
    for font in every_font(&fonts) {
        let width = font.string_width(&text);
        let sum: i32 = text.iter().map(|&u| font.char_width(u)).sum();
        assert_eq!(width, sum, "{font:?}");
        // Anchoring at the right edge must land exactly where the left
        // anchor at x - width does.
        let mut a = Surface::new_mutable(220, 30).unwrap();
        let mut g = Graphics::new(&a, fonts.default_font()).unwrap();
        g.set_font(Some(font.clone()));
        g.draw_string(&mut a, &text, 210, 2, TOP | RIGHT).unwrap();
        let mut b = Surface::new_mutable(220, 30).unwrap();
        g.draw_string(&mut b, &text, 210 - width, 2, TOP | LEFT)
            .unwrap();
        assert_eq!(a, b, "{font:?}");
        // Baseline and bottom anchors agree with the height and baseline.
        let mut c = Surface::new_mutable(220, 30).unwrap();
        g.draw_string(
            &mut c,
            &text,
            210,
            2 + font.baseline_position(),
            BASELINE | RIGHT,
        )
        .unwrap();
        assert_eq!(a, c, "{font:?}");
        let mut d = Surface::new_mutable(220, 30).unwrap();
        g.draw_string(
            &mut d,
            &text,
            210 - width / 2,
            2 + font.height(),
            BOTTOM | HCENTER,
        )
        .unwrap();
        let mut e = Surface::new_mutable(220, 30).unwrap();
        g.draw_string(&mut e, &text, 210 - width, 2, TOP | LEFT)
            .unwrap();
        if width % 2 == 0 {
            assert_eq!(d, e, "{font:?}");
        }
    }
}

#[test]
fn screens_present_as_frames() {
    let fonts = fonts();
    let mut screen = Surface::screen(4, 2).unwrap();
    let mut g = Graphics::new(&screen, fonts.default_font()).unwrap();
    g.set_color(0x00_FF00);
    g.fill_rect(&mut screen, 1, 0, 2, 2);
    let frame = screen.to_frame(9);
    assert_eq!((frame.width(), frame.height(), frame.serial()), (4, 2, 9));
    assert_eq!(frame.pixel(1, 1), Some(0xFF00_FF00));
    assert!(frame.pixels().iter().all(|&p| p >> 24 == 0xFF));
}
