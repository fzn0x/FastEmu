//! Renders a few scenes through the public API and compares them with the
//! PNG files in `tests/golden/`. Run with `FASTEMU_UPDATE_GOLDEN=1` to
//! write the files again after an intended change, then look at them.

use std::path::PathBuf;

use fastemu_core::FontSizes;
use fastemu_gfx::anchor::{BASELINE, BOTTOM, HCENTER, LEFT, RIGHT, TOP, VCENTER};
use fastemu_gfx::graphics::DOTTED;
use fastemu_gfx::nokia::{FLIP_HORIZONTAL, ROTATE_90, TYPE_USHORT_4444_ARGB};
use fastemu_gfx::{Face, FontLibrary, Graphics, Size, Style, Surface, TextRendering, Transform};

fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn encode_png(surface: &Surface) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, surface.width(), surface.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().expect("PNG header");
    let frame = surface.to_frame(0);
    writer
        .write_image_data(&frame.to_rgba8())
        .expect("PNG data");
    writer.finish().expect("PNG end");
    out
}

/// Compares `surface` with the golden file `name`, allowing each channel
/// to differ by `tolerance` and `stray` pixels to differ by more (glyph
/// edges may round differently across platforms).
fn check(name: &str, surface: &Surface, tolerance: u32, stray: usize) {
    let path = golden_path(name);
    if std::env::var_os("FASTEMU_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("golden folder");
        std::fs::write(&path, encode_png(surface)).expect("writing a golden file");
        return;
    }
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run with FASTEMU_UPDATE_GOLDEN=1", path.display()));
    let golden = Surface::decode(&bytes).expect("a golden file decodes");
    assert_eq!(
        (golden.width(), golden.height()),
        (surface.width(), surface.height()),
        "{name}"
    );
    let mut far = 0;
    let mut first = None;
    for (i, (&a, &b)) in surface.pixels().iter().zip(golden.pixels()).enumerate() {
        let worst = (0..4)
            .map(|c| ((a >> (c * 8)) & 0xFF).abs_diff((b >> (c * 8)) & 0xFF))
            .max()
            .unwrap_or(0);
        if worst > tolerance {
            far += 1;
            first.get_or_insert((i as u32 % surface.width(), i as u32 / surface.width(), a, b));
        }
    }
    assert!(
        far <= stray,
        "{name}: {far} pixels differ, first at {first:x?} (got, golden); \
         rerun with FASTEMU_UPDATE_GOLDEN=1 if the change is intended"
    );
}

fn fonts(rendering: TextRendering) -> FontLibrary {
    let sizes = FontSizes {
        small: 12,
        medium: 14,
        large: 16,
    };
    FontLibrary::new(sizes, rendering)
}

#[test]
fn primitives() {
    let mut s = Surface::new_mutable(96, 96).unwrap();
    let mut g = Graphics::new(&s, fonts(TextRendering::Aliased).default_font()).unwrap();
    g.set_color(0x20_3040);
    g.fill_rect(&mut s, 0, 0, 96, 96);
    g.set_color(0xFF_FFFF);
    g.draw_line(&mut s, 2, 2, 30, 12);
    g.draw_line(&mut s, 2, 14, 12, 40);
    g.draw_rect(&mut s, 34, 2, 12, 8);
    g.set_color(0xE0_4040);
    g.fill_rect(&mut s, 50, 2, 12, 8);
    g.set_color(0x40_E040);
    g.draw_round_rect(&mut s, 66, 2, 26, 14, 10, 8);
    g.fill_round_rect(&mut s, 66, 20, 26, 14, 10, 8);
    g.set_color(0xF0_D020);
    g.draw_arc(&mut s, 2, 44, 24, 18, 30, 300);
    g.fill_arc(&mut s, 30, 44, 24, 18, 90, -135);
    g.fill_arc(&mut s, 58, 40, 9, 9, 0, 360);
    g.draw_arc(&mut s, 70, 40, 9, 9, 0, 360);
    g.set_color(0x40_A0FF);
    g.fill_triangle(&mut s, 16, 20, 30, 40, 44, 24);
    g.set_stroke_style(DOTTED).unwrap();
    g.set_color(0xFF_FFFF);
    g.draw_rect(&mut s, 50, 14, 12, 12);
    g.draw_line(&mut s, 2, 92, 92, 66);
    g.set_stroke_style(0).unwrap();
    // Translucent Nokia colour over everything drawn so far.
    g.set_argb_color(0x80FF_00FF_u32 as i32);
    g.fill_rect(&mut s, 40, 60, 40, 20);
    // Clipped and translated drawing.
    g.set_color(0xFF_FFFF);
    g.translate(80, 80);
    g.set_clip(-4, -4, 12, 12);
    g.fill_arc(&mut s, -10, -10, 20, 20, 0, 360);
    check("primitives.png", &s, 0, 0);
}

#[test]
fn images() {
    // A 6x4 sprite: a gradient body with a clear corner and a translucent
    // stripe, so every transform shows.
    let mut rgb = Vec::new();
    for y in 0..4_u32 {
        for x in 0..6_u32 {
            let p = match (x, y) {
                (0, 0) => 0,
                (_, 3) => 0x80FF_FFFF,
                _ => 0xFF00_0000 | (x * 40) << 16 | (y * 60) << 8 | 0xC0,
            };
            rgb.push(p as i32);
        }
    }
    let sprite = Surface::from_rgb(&rgb, 6, 4, true).unwrap();
    let mut s = Surface::new_mutable(80, 48).unwrap();
    let mut g = Graphics::new(&s, fonts(TextRendering::Aliased).default_font()).unwrap();
    g.set_color(0x30_3030);
    g.fill_rect(&mut s, 0, 0, 80, 48);
    for (i, t) in Transform::ALL.into_iter().enumerate() {
        let x = 2 + (i as i32 % 4) * 10;
        let y = 2 + (i as i32 / 4) * 10;
        g.draw_region(&mut s, &sprite, 0, 0, 6, 4, t.to_midp(), x, y, 0)
            .unwrap();
    }
    for (i, anchor) in [TOP | LEFT, VCENTER | HCENTER, BOTTOM | RIGHT]
        .into_iter()
        .enumerate()
    {
        g.draw_image(&mut s, &sprite, 50 + i as i32 * 10, 10, anchor)
            .unwrap();
    }
    g.draw_rgb(&mut s, &rgb, 0, 6, 2, 24, 6, 4, false).unwrap();
    g.draw_rgb(&mut s, &rgb, 18, -6, 10, 24, 6, 4, true)
        .unwrap();
    let shorts: Vec<i16> = (0..24).map(|i: i32| (0xF000 | (i * 0xA1)) as i16).collect();
    g.draw_pixels_short(
        &mut s,
        &shorts,
        true,
        0,
        6,
        20,
        24,
        6,
        4,
        FLIP_HORIZONTAL | ROTATE_90,
        TYPE_USHORT_4444_ARGB,
    )
    .unwrap();
    g.draw_image_manipulated(&mut s, &sprite, 40, 24, 0, ROTATE_90)
        .unwrap();
    g.copy_area(&mut s, 0, 20, 30, 10, 48, 36, 0).unwrap();
    check("images.png", &s, 0, 0);
}

fn text_scene(rendering: TextRendering) -> Surface {
    let fonts = fonts(rendering);
    let mut s = Surface::new_mutable(160, 120).unwrap();
    let mut g = Graphics::new(&s, fonts.default_font()).unwrap();
    g.set_color(0xFF_FFF0);
    g.fill_rect(&mut s, 0, 0, 160, 120);
    g.set_color(0x10_2040);
    let mut y = 2;
    for (face, style, size, text) in [
        (
            Face::System,
            Style::PLAIN,
            Size::Small,
            "System small: Hello!",
        ),
        (
            Face::Proportional,
            Style::BOLD,
            Size::Medium,
            "Bold Жизнь Ωμέγα",
        ),
        (
            Face::Monospace,
            Style::PLAIN,
            Size::Medium,
            "Mono 0123 {}[]",
        ),
        (
            Face::System,
            Style::ITALIC.with(Style::UNDERLINED),
            Size::Large,
            "Italic line",
        ),
        (
            Face::Monospace,
            Style::BOLD.with(Style::ITALIC),
            Size::Small,
            "mono bold italic",
        ),
    ] {
        g.set_font(Some(fonts.font(face, style, size)));
        g.draw_str(&mut s, text, 2, y, TOP | LEFT).unwrap();
        y += g.font().height() + 2;
    }
    g.set_font(None);
    g.set_color(0xC0_2020);
    g.draw_str(&mut s, "right", 158, 118, BOTTOM | RIGHT)
        .unwrap();
    g.draw_str(&mut s, "centre", 80, 108, BASELINE | HCENTER)
        .unwrap();
    g.draw_str(&mut s, "\u{4E2D}?", 2, 118, BOTTOM | LEFT)
        .unwrap();
    s
}

#[test]
fn text_aliased() {
    check(
        "text-aliased.png",
        &text_scene(TextRendering::Aliased),
        0,
        40,
    );
}

#[test]
fn text_antialiased() {
    check(
        "text-antialiased.png",
        &text_scene(TextRendering::Antialiased),
        24,
        40,
    );
}
