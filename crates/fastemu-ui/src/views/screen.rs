//! Drawing a game's screen: uploading each new frame to GPUI's image atlas
//! once, and freeing the frame before it.
//!
//! Where the renderer magnifies images with the nearest filter, a frame is
//! uploaded at the game's own size and the GPU enlarges it, pixel for
//! pixel. FastEmu's GPUI does that in its wgpu renderer, which Android and
//! Linux use. Elsewhere the frame is enlarged on the processor to the size
//! it is shown, since the GPU would blur it.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use fastemu_core::{Frame, InstanceId};
use gpui::{Bounds, Corners, Pixels, RenderImage, Window};

use crate::pixels::{Bgra, screen_bgra};

/// Whether the platform's renderer magnifies images with the nearest
/// filter, so that a screen can be uploaded at the game's own size.
pub const GPU_ENLARGES_CRISPLY: bool = cfg!(any(
    target_os = "android",
    target_os = "linux",
    target_os = "freebsd"
));

/// How many times a frame is enlarged before upload, for a screen shown
/// `shown` times its size.
pub fn upload_factor(shown: u32) -> u32 {
    if GPU_ENLARGES_CRISPLY { 1 } else { shown }
}

/// Wraps BGRA bytes in the image type GPUI paints.
pub fn render_image(bgra: Bgra) -> Option<Arc<RenderImage>> {
    let buffer = image::RgbaImage::from_raw(bgra.width, bgra.height, bgra.bytes)?;
    Some(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}

/// Which picture an uploaded image shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ImageKey {
    instance: InstanceId,
    serial: u64,
    factor: u32,
}

/// One tile's uploaded screen image and where it was last drawn.
///
/// The image lives in GPUI's atlas until it is dropped there. A new frame
/// replaces it and drops the old one at once, so a tile holds one image
/// however long a game runs.
#[derive(Default)]
pub struct ScreenCache {
    image: Option<(ImageKey, Arc<RenderImage>)>,
    /// Where the screen was drawn in the last frame, for mapping pointer
    /// positions to phone pixels.
    drawn: Option<Bounds<Pixels>>,
    /// How many frames this tile uploaded, for tests to count.
    #[cfg(test)]
    uploads: u64,
}

/// Shared between a tile and the canvas that paints it.
pub type SharedScreen = Rc<RefCell<ScreenCache>>;

impl ScreenCache {
    /// Paints `frame` of `instance` into `bounds`, scaled `factor` times.
    pub fn paint(
        &mut self,
        bounds: Bounds<Pixels>,
        instance: InstanceId,
        frame: &Frame,
        factor: u32,
        window: &mut Window,
    ) {
        self.drawn = Some(bounds);
        let key = ImageKey {
            instance,
            serial: frame.serial(),
            factor,
        };
        if self.image.as_ref().is_none_or(|(k, _)| *k != key) {
            let Some(image) = render_image(screen_bgra(frame, factor)) else {
                return;
            };
            #[cfg(test)]
            {
                self.uploads += 1;
            }
            if let Some((_, old)) = self.image.replace((key, image)) {
                window.drop_image(old).ok();
            }
        }
        if let Some((_, image)) = &self.image {
            window
                .paint_image(
                    bounds,
                    bounds,
                    Corners::default(),
                    Arc::clone(image),
                    0,
                    false,
                )
                .ok();
        }
    }

    /// Frees the uploaded image, when the tile closes or restarts.
    pub fn release(&mut self, window: &mut Window) {
        if let Some((_, image)) = self.image.take() {
            window.drop_image(image).ok();
        }
        self.drawn = None;
    }

    pub fn drawn(&self) -> Option<Bounds<Pixels>> {
        self.drawn
    }

    /// The image currently in the atlas, if any.
    #[cfg(test)]
    pub fn image(&self) -> Option<&Arc<RenderImage>> {
        self.image.as_ref().map(|(_, image)| image)
    }

    #[cfg(test)]
    pub fn uploads(&self) -> u64 {
        self.uploads
    }
}
