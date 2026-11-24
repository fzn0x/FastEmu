//! Rough timings of the hot paths. Run them in a release build:
//!
//! ```text
//! cargo test --release -p fastemu-gfx --test bench -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::Instant;

use fastemu_core::FontSizes;
use fastemu_gfx::{FontLibrary, Graphics, Surface, TextRendering, Transform};

/// Runs `op` until about half a second has passed and prints the time per
/// call and, if `pixels` is not zero, the pixels drawn per second.
fn measure(name: &str, pixels: u64, mut op: impl FnMut()) {
    for _ in 0..10 {
        op();
    }
    let start = Instant::now();
    let mut calls = 0_u64;
    while start.elapsed().as_millis() < 500 {
        for _ in 0..100 {
            op();
        }
        calls += 100;
    }
    let per_call = start.elapsed().as_nanos() as f64 / calls as f64;
    if pixels == 0 {
        println!("{name:<40} {per_call:>10.0} ns/call");
    } else {
        let rate = pixels as f64 / per_call * 1e3;
        println!("{name:<40} {per_call:>10.0} ns/call {rate:>8.1} Mpixel/s");
    }
}

fn sprite(alpha: bool) -> Surface {
    let rgb: Vec<i32> = (0..64 * 64)
        .map(|i: u32| {
            let (x, y) = (i % 64, i / 64);
            let a = if !alpha || (x + y) % 3 == 0 {
                0xFF
            } else if (x ^ y) & 4 == 0 {
                0
            } else {
                0x80
            };
            (a << 24 | (x * 4) << 16 | (y * 4) << 8 | 0x40) as i32
        })
        .collect();
    Surface::from_rgb(&rgb, 64, 64, true).unwrap()
}

#[test]
#[ignore = "a benchmark; run it in a release build"]
fn graphics_hot_paths() {
    let sizes = FontSizes {
        small: 12,
        medium: 14,
        large: 16,
    };
    let fonts = FontLibrary::new(sizes, TextRendering::Antialiased);
    let aliased = FontLibrary::new(sizes, TextRendering::Aliased);
    let mut screen = Surface::screen(240, 320).unwrap();
    let mut g = Graphics::new(&screen, fonts.default_font()).unwrap();
    let opaque = sprite(false);
    let translucent = sprite(true);
    let text = "The quick brown fox jumps over";
    let units: Vec<u16> = text.encode_utf16().collect();

    g.set_color(0x12_3456);
    measure("fillRect 240x320 opaque", 240 * 320, || {
        g.fill_rect(black_box(&mut screen), 0, 0, 240, 320);
    });
    measure("fillRect 16x16 opaque", 16 * 16, || {
        g.fill_rect(black_box(&mut screen), 100, 100, 16, 16);
    });
    g.set_argb_color(0x8012_3456_u32 as i32);
    measure("fillRect 240x320 translucent", 240 * 320, || {
        g.fill_rect(black_box(&mut screen), 0, 0, 240, 320);
    });
    g.set_color(0);
    measure("drawImage 64x64 opaque", 64 * 64, || {
        g.draw_image(black_box(&mut screen), &opaque, 50, 50, 0)
            .unwrap();
    });
    measure("drawImage 64x64 with alpha", 64 * 64, || {
        g.draw_image(black_box(&mut screen), &translucent, 50, 50, 0)
            .unwrap();
    });
    let rot90 = Transform::Rot90.to_midp();
    measure("drawRegion 64x64 alpha, TRANS_ROT90", 64 * 64, || {
        g.draw_region(
            black_box(&mut screen),
            &translucent,
            0,
            0,
            64,
            64,
            rot90,
            50,
            50,
            0,
        )
        .unwrap();
    });
    let rgb: Vec<i32> = translucent.pixels().iter().map(|&p| p as i32).collect();
    measure("drawRGB 64x64 processAlpha", 64 * 64, || {
        g.draw_rgb(black_box(&mut screen), &rgb, 0, 64, 50, 50, 64, 64, true)
            .unwrap();
    });
    measure("drawString 30 chars, anti-aliased", 0, || {
        g.draw_string(black_box(&mut screen), &units, 2, 100, 0)
            .unwrap();
    });
    g.set_font(Some(aliased.default_font()));
    measure("drawString 30 chars, aliased", 0, || {
        g.draw_string(black_box(&mut screen), &units, 2, 100, 0)
            .unwrap();
    });
    measure("stringWidth 30 chars", 0, || {
        black_box(g.font().string_width(black_box(&units)));
    });
}
