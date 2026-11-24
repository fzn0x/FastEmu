//! The software rasterizer behind MIDP's `Graphics`, `Image` and `Font`.

pub mod color;
pub mod error;
pub mod surface;
pub mod transform;

pub use color::{OPAQUE, blend, display_color, gray_of};
pub use error::{GfxError, Result, non_null};
pub use surface::{MAX_PIXELS, Surface, SurfaceKind};
pub use transform::Transform;
