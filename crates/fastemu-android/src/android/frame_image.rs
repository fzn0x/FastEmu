//! The GPUI image of an instance's latest frame.
//!
//! Each new frame becomes a new GPUI image, which GPUI copies into its
//! texture atlas when it is first painted. Atlas entries stay until they are
//! dropped, so this keeps the image for the frame on screen and drops the
//! ones before it.

use std::sync::Arc;

use fastemu_core::{Frame, InstanceId};
use gpui::{RenderImage, Window};

use crate::screen;

struct Shown {
    instance: InstanceId,
    serial: u64,
    upscale: u32,
    image: Arc<RenderImage>,
}

/// The image of the frame on screen, made again only when the frame or its
/// zoom changes.
#[derive(Default)]
pub(super) struct FrameImage {
    shown: Option<Shown>,
    /// The image the last scene drew, dropped one frame late: a scene GPUI
    /// presents again after a failed present still refers to it.
    previous: Option<Arc<RenderImage>>,
}

impl FrameImage {
    /// The image of `frame` from `instance`, scaled up `upscale` times.
    pub(super) fn image(
        &mut self,
        instance: InstanceId,
        frame: &Frame,
        upscale: u32,
        window: &mut Window,
    ) -> Option<Arc<RenderImage>> {
        if let Some(shown) = &self.shown
            && shown.instance == instance
            && shown.serial == frame.serial()
            && shown.upscale == upscale
        {
            return Some(Arc::clone(&shown.image));
        }
        let bytes = screen::upscaled_bgra(frame, upscale);
        let Some(buffer) =
            image::RgbaImage::from_raw(frame.width() * upscale, frame.height() * upscale, bytes)
        else {
            log::error!("frame_image: a frame's pixels don't fill its size");
            return None;
        };
        // GPUI takes BGRA bytes in an RGBA buffer.
        let image = Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]));
        let replaced = self.shown.replace(Shown {
            instance,
            serial: frame.serial(),
            upscale,
            image: Arc::clone(&image),
        });
        self.retire(replaced.map(|shown| shown.image), window);
        Some(image)
    }

    /// Drops every image, for when there is no frame to show.
    pub(super) fn clear(&mut self, window: &mut Window) {
        let shown = self.shown.take().map(|shown| shown.image);
        self.retire(shown, window);
        self.retire(None, window);
    }

    fn retire(&mut self, image: Option<Arc<RenderImage>>, window: &mut Window) {
        if let Some(stale) = std::mem::replace(&mut self.previous, image)
            && let Err(error) = window.drop_image(stale)
        {
            log::warn!("frame_image: dropping an old frame: {error:#}");
        }
    }
}
