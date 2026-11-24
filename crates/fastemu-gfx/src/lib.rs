//! The software rasterizer behind MIDP's `Graphics`, `Image` and `Font`.

pub mod anchor;
pub mod color;
pub mod error;
pub mod font;
pub mod graphics;
pub mod nokia;
mod raster;
pub mod surface;
pub mod transform;

pub use anchor::{Anchor, AnchorPolicy};
pub use color::{OPAQUE, blend, display_color, gray_of};
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
}
