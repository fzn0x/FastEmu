//! The software rasterizer behind MIDP's `Graphics`, `Image` and `Font`.

pub mod anchor;
pub mod color;
pub mod error;
pub mod graphics;
pub mod nokia;
mod raster;
pub mod surface;
pub mod transform;

pub use anchor::{Anchor, AnchorPolicy};
pub use color::{OPAQUE, blend, display_color, gray_of};
pub use error::{GfxError, Result, non_null};
pub use graphics::{Graphics, StrokeStyle};
pub use surface::{MAX_PIXELS, Surface, SurfaceKind};
pub use transform::Transform;
