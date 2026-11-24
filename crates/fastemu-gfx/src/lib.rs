//! The software rasterizer behind MIDP's `Graphics`, `Image` and `Font`,
//! and Nokia's `DirectGraphics`.
//!
//! A [`Surface`] is the screen or an image: `0xAARRGGBB` pixels, the
//! format of [`fastemu_core::Frame`]. A [`Graphics`] holds MIDP's drawing
//! state (translation, clip, colour, stroke, font) and draws onto whatever
//! surface each call is given, so a wrapper can keep the state in the Java
//! object and look the target up per call. A [`FontLibrary`] sizes the
//! bundled fonts for one phone. [`Surface::decode`] reads PNG, JPEG, GIF
//! and BMP files. Calls that MIDP lets throw return a [`GfxError`] naming
//! the Java exception.
//!
//! Every primitive follows MIDP's pixel coverage rules; see
//! `docs/architecture/graphics.md` for the full list.
//!
//! ```
//! use fastemu_core::FontSizes;
//! use fastemu_gfx::anchor::{HCENTER, TOP};
//! use fastemu_gfx::{FontLibrary, Graphics, Surface, TextRendering};
//!
//! let sizes = FontSizes { small: 12, medium: 14, large: 16 };
//! let fonts = FontLibrary::new(sizes, TextRendering::Antialiased);
//! let mut screen = Surface::screen(240, 320)?;
//! let mut g = Graphics::new(&screen, fonts.default_font())?;
//! g.set_color(0x00_0080);
//! g.fill_rect(&mut screen, 0, 0, 240, 320);
//! g.set_color(0xFF_FFFF);
//! g.draw_str(&mut screen, "Hello", 120, 10, HCENTER | TOP)?;
//! let frame = screen.to_frame(1);
//! assert_eq!(frame.pixel(0, 0), Some(0xFF00_0080));
//! # Ok::<(), fastemu_gfx::GfxError>(())
//! ```

pub mod anchor;
pub mod color;
pub mod decode;
pub mod error;
pub mod font;
pub mod graphics;
pub mod nokia;
mod raster;
pub mod surface;
pub mod transform;

pub use anchor::{Anchor, AnchorPolicy};
pub use color::{OPAQUE, blend, display_color, gray_of};
pub use decode::ImageFormat;
pub use error::{GfxError, Result, non_null};
pub use font::{Face, Font, FontLibrary, Size, Style, TextRendering};
pub use graphics::{Graphics, StrokeStyle};
pub use surface::{MAX_PIXELS, Surface, SurfaceKind};
pub use transform::Transform;

/// Fonts and graphics for unit tests.
#[cfg(test)]
pub(crate) mod testing {
    use std::sync::OnceLock;

    use fastemu_core::FontSizes;

    use crate::{Font, FontLibrary, Graphics, Surface, TextRendering};

    pub fn library() -> &'static FontLibrary {
        static LIBRARY: OnceLock<FontLibrary> = OnceLock::new();
        LIBRARY.get_or_init(|| {
            let sizes = FontSizes {
                small: 12,
                medium: 14,
                large: 16,
            };
            FontLibrary::new(sizes, TextRendering::Antialiased)
        })
    }

    pub fn font() -> Font {
        library().default_font()
    }

    pub fn graphics(target: &Surface) -> Graphics {
        Graphics::new(target, font()).expect("a drawable surface")
    }

    /// Each game runs on its own threads, so its surfaces, state and fonts
    /// must move between threads, and font libraries are shared.
    #[test]
    fn public_types_cross_threads() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<Surface>();
        send_sync::<Graphics>();
        send_sync::<Font>();
        send_sync::<FontLibrary>();
        send_sync::<crate::GfxError>();
        let font = font();
        let width = std::thread::spawn(move || font.str_width("threads"))
            .join()
            .expect("the thread finishes");
        assert_eq!(width, library().default_font().str_width("threads"));
    }
}
