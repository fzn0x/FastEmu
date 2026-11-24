//! MIDP's `Graphics`: the drawing state (translation, clip, colour, stroke
//! and font) and the calls that draw with it onto a [`Surface`].
//!
//! A `Graphics` does not own or borrow its target. Each drawing call takes
//! the target surface, so the MIDP wrapper can keep the state in the Java
//! object and look the surface up per call. The target should be the
//! surface the `Graphics` was made for; drawing is clipped to whatever
//! surface is passed, so a mismatch cannot write out of bounds.

mod shapes;

use crate::anchor::AnchorPolicy;
use crate::color::{OPAQUE, display_color, gray_of};
use crate::error::{GfxError, Result};
use crate::raster::{Canvas, Clip};
use crate::surface::{Surface, SurfaceKind};

/// `Graphics.SOLID`.
pub const SOLID: i32 = 0;
/// `Graphics.DOTTED`.
pub const DOTTED: i32 = 1;

/// How outlines are drawn: lines, rectangles, round rectangles and arcs.
/// Fills ignore it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StrokeStyle {
    #[default]
    Solid,
    /// Every other pixel along the outline, starting with the first.
    Dotted,
}

/// The state of a MIDP `Graphics` object.
#[derive(Clone, Debug)]
pub struct Graphics {
    width: i64,
    height: i64,
    screen: bool,
    translate_x: i32,
    translate_y: i32,
    /// The clip in surface pixels, always inside the surface.
    clip: Clip,
    /// The drawing colour, opaque unless Nokia's `setARGBColor` set alpha.
    color: u32,
    /// `getGrayScale()`: the value `setGrayScale` set, or the luma of the
    /// colour `setColor` set.
    gray: u8,
    stroke: StrokeStyle,
    anchors: AnchorPolicy,
    display_colors: u32,
}

impl Graphics {
    /// A `Graphics` for drawing on `target`, in MIDP's initial state: no
    /// translation, the whole surface as clip, black, solid strokes.
    /// Immutable images refuse with `IllegalStateException`, as
    /// `Image.getGraphics()` does.
    pub fn new(target: &Surface) -> Result<Self> {
        if target.kind() == SurfaceKind::Immutable {
            return Err(GfxError::IllegalState("an immutable image has no Graphics"));
        }
        let (width, height) = (i64::from(target.width()), i64::from(target.height()));
        Ok(Self {
            width,
            height,
            screen: target.kind() == SurfaceKind::Screen,
            translate_x: 0,
            translate_y: 0,
            clip: Clip::of_size(width, height),
            color: OPAQUE,
            gray: 0,
            stroke: StrokeStyle::Solid,
            anchors: AnchorPolicy::Strict,
            display_colors: 1 << 24,
        })
    }

    /// Returns to the initial state, as MIDP does before each
    /// `Canvas.paint`. The anchor policy and display depth stay.
    pub fn reset(&mut self) {
        self.translate_x = 0;
        self.translate_y = 0;
        self.clip = Clip::of_size(self.width, self.height);
        self.color = OPAQUE;
        self.gray = 0;
        self.stroke = StrokeStyle::Solid;
    }

    /// Whether the target is the screen rather than an image.
    pub fn is_screen(&self) -> bool {
        self.screen
    }

    /// How anchors are checked; strict by default.
    pub fn anchor_policy(&self) -> AnchorPolicy {
        self.anchors
    }

    pub fn set_anchor_policy(&mut self, policy: AnchorPolicy) {
        self.anchors = policy;
    }

    /// Sets the colour count `getDisplayColor` answers for, normally the
    /// device profile's `colors`.
    pub fn set_display_colors(&mut self, num_colors: u32) {
        self.display_colors = num_colors;
    }

    /// `translate(x, y)`. Like Java, the sum wraps around.
    pub fn translate(&mut self, x: i32, y: i32) {
        self.translate_x = self.translate_x.wrapping_add(x);
        self.translate_y = self.translate_y.wrapping_add(y);
    }

    pub fn translate_x(&self) -> i32 {
        self.translate_x
    }

    pub fn translate_y(&self) -> i32 {
        self.translate_y
    }

    /// The absolute rectangle of a translated one; a negative size gives
    /// an empty rectangle at its corner.
    fn absolute(&self, x: i32, y: i32, width: i32, height: i32) -> Clip {
        let x0 = i64::from(x) + i64::from(self.translate_x);
        let y0 = i64::from(y) + i64::from(self.translate_y);
        Clip {
            x0,
            y0,
            x1: x0 + i64::from(width.max(0)),
            y1: y0 + i64::from(height.max(0)),
        }
    }

    /// `setClip(x, y, width, height)`: replaces the clip, in translated
    /// coordinates, clamped to the surface. A negative size empties it.
    pub fn set_clip(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let bounds = Clip::of_size(self.width, self.height);
        self.clip = clamp_into(self.absolute(x, y, width, height), bounds);
    }

    /// `clipRect(x, y, width, height)`: narrows the clip to its overlap
    /// with the rectangle.
    pub fn clip_rect(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.clip = clamp_into(self.absolute(x, y, width, height), self.clip);
    }

    /// `getClipX()`, in translated coordinates.
    pub fn clip_x(&self) -> i32 {
        (self.clip.x0 - i64::from(self.translate_x)) as i32
    }

    /// `getClipY()`, in translated coordinates.
    pub fn clip_y(&self) -> i32 {
        (self.clip.y0 - i64::from(self.translate_y)) as i32
    }

    pub fn clip_width(&self) -> i32 {
        (self.clip.x1 - self.clip.x0) as i32
    }

    pub fn clip_height(&self) -> i32 {
        (self.clip.y1 - self.clip.y0) as i32
    }

    /// `setColor(RGB)`: the top byte is ignored and the colour is opaque.
    pub fn set_color(&mut self, rgb: i32) {
        let rgb = rgb as u32 & 0x00FF_FFFF;
        self.color = OPAQUE | rgb;
        self.gray = gray_of(rgb);
    }

    /// `setColor(red, green, blue)`, which throws
    /// `IllegalArgumentException` for a component outside 0 to 255.
    pub fn set_color_rgb(&mut self, red: i32, green: i32, blue: i32) -> Result<()> {
        let component = |c: i32| {
            u8::try_from(c).map_err(|_| GfxError::IllegalArgument("colour component out of range"))
        };
        let (r, g, b) = (component(red)?, component(green)?, component(blue)?);
        self.set_color(i32::from(r) << 16 | i32::from(g) << 8 | i32::from(b));
        Ok(())
    }

    /// `getColor()`: `0x00RRGGBB`.
    pub fn color(&self) -> i32 {
        (self.color & 0x00FF_FFFF) as i32
    }

    /// The colour primitives draw with, alpha included.
    pub fn argb_color(&self) -> u32 {
        self.color
    }

    pub fn red_component(&self) -> i32 {
        ((self.color >> 16) & 0xFF) as i32
    }

    pub fn green_component(&self) -> i32 {
        ((self.color >> 8) & 0xFF) as i32
    }

    pub fn blue_component(&self) -> i32 {
        (self.color & 0xFF) as i32
    }

    /// `setGrayScale(value)`, which throws `IllegalArgumentException`
    /// outside 0 to 255.
    pub fn set_gray_scale(&mut self, value: i32) -> Result<()> {
        let v = u8::try_from(value)
            .map_err(|_| GfxError::IllegalArgument("grey value out of range"))?;
        self.color = OPAQUE | (u32::from(v) * 0x0001_0101);
        self.gray = v;
        Ok(())
    }

    /// `getGrayScale()`.
    pub fn gray_scale(&self) -> i32 {
        self.gray.into()
    }

    /// Nokia's `DirectGraphics.setARGBColor(argb)`: the colour and alpha
    /// of later primitives, which then blend. `setColor` makes them opaque
    /// again.
    pub fn set_argb_color(&mut self, argb: i32) {
        self.color = argb as u32;
        self.gray = gray_of(self.color & 0x00FF_FFFF);
    }

    /// Nokia's `DirectGraphics.getAlphaComponent()`.
    pub fn alpha_component(&self) -> i32 {
        (self.color >> 24) as i32
    }

    /// `getDisplayColor(color)`: the colour the phone's display shows.
    pub fn display_color(&self, color: i32) -> i32 {
        display_color(color, self.display_colors)
    }

    /// `setStrokeStyle(style)`, which throws `IllegalArgumentException` for
    /// anything but `SOLID` and `DOTTED`.
    pub fn set_stroke_style(&mut self, style: i32) -> Result<()> {
        self.stroke = match style {
            SOLID => StrokeStyle::Solid,
            DOTTED => StrokeStyle::Dotted,
            _ => return Err(GfxError::IllegalArgument("invalid stroke style")),
        };
        Ok(())
    }

    /// `getStrokeStyle()`.
    pub fn stroke_style(&self) -> i32 {
        match self.stroke {
            StrokeStyle::Solid => SOLID,
            StrokeStyle::Dotted => DOTTED,
        }
    }

    /// The absolute position of a translated point.
    fn at(&self, x: i32, y: i32) -> (i64, i64) {
        (
            i64::from(x) + i64::from(self.translate_x),
            i64::from(y) + i64::from(self.translate_y),
        )
    }

    /// A canvas over `target` with the clip and a colour.
    fn canvas_with<'a>(&self, target: &'a mut Surface, color: u32) -> Canvas<'a> {
        Canvas::new(target, self.clip, color)
    }

    /// A canvas over `target` with the clip and the current colour.
    fn canvas<'a>(&self, target: &'a mut Surface) -> Canvas<'a> {
        self.canvas_with(target, self.color)
    }
}

/// `rect` inside `bounds`, keeping an empty result at a position inside.
fn clamp_into(rect: Clip, bounds: Clip) -> Clip {
    let x0 = rect.x0.clamp(bounds.x0, bounds.x1);
    let y0 = rect.y0.clamp(bounds.y0, bounds.y1);
    Clip {
        x0,
        y0,
        x1: rect.x1.clamp(x0, bounds.x1),
        y1: rect.y1.clamp(y0, bounds.y1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graphics(w: i32, h: i32) -> Graphics {
        Graphics::new(&Surface::new_mutable(w, h).unwrap()).unwrap()
    }

    #[test]
    fn immutable_images_have_no_graphics() {
        let image = Surface::from_rgb(&[0], 1, 1, false).unwrap();
        assert!(matches!(
            Graphics::new(&image),
            Err(GfxError::IllegalState(_))
        ));
        let screen = Surface::screen(4, 4).unwrap();
        assert!(Graphics::new(&screen).unwrap().is_screen());
    }

    #[test]
    fn clip_is_translated_and_clamped() {
        let mut g = graphics(20, 10);
        assert_eq!(
            (g.clip_x(), g.clip_y(), g.clip_width(), g.clip_height()),
            (0, 0, 20, 10)
        );
        g.translate(5, 2);
        g.set_clip(-10, 0, 30, 4);
        assert_eq!(
            (g.clip_x(), g.clip_y(), g.clip_width(), g.clip_height()),
            (-5, 0, 20, 4)
        );
        g.clip_rect(0, 1, 100, 100);
        assert_eq!(
            (g.clip_x(), g.clip_y(), g.clip_width(), g.clip_height()),
            (0, 1, 15, 3)
        );
        g.translate(-5, -2);
        assert_eq!((g.clip_x(), g.clip_y()), (5, 3));
        g.clip_rect(100, 100, 5, 5);
        assert_eq!((g.clip_width(), g.clip_height()), (0, 0));
    }

    #[test]
    fn negative_clip_sizes_empty_the_clip() {
        let mut g = graphics(8, 8);
        g.set_clip(2, 2, -1, 4);
        assert_eq!((g.clip_width(), g.clip_height()), (0, 4));
        g.set_clip(2, 2, 4, 4);
        g.clip_rect(0, 0, 3, -2);
        assert_eq!(g.clip_height(), 0);
    }

    #[test]
    fn translation_wraps_like_java() {
        let mut g = graphics(1, 1);
        g.translate(i32::MAX, 0);
        g.translate(1, 0);
        assert_eq!(g.translate_x(), i32::MIN);
    }

    #[test]
    fn colours_and_greys() {
        let mut g = graphics(1, 1);
        assert_eq!(g.color(), 0);
        g.set_color(0x7F12_3456);
        assert_eq!(g.color(), 0x12_3456);
        assert_eq!(
            (g.red_component(), g.green_component(), g.blue_component()),
            (0x12, 0x34, 0x56)
        );
        assert_eq!(g.argb_color(), 0xFF12_3456);
        g.set_color_rgb(255, 255, 255).unwrap();
        assert_eq!(g.gray_scale(), 255);
        assert!(g.set_color_rgb(256, 0, 0).is_err());
        assert!(g.set_color_rgb(0, -1, 0).is_err());
        g.set_gray_scale(77).unwrap();
        assert_eq!((g.color(), g.gray_scale()), (0x4D_4D4D, 77));
        assert!(g.set_gray_scale(300).is_err());
        g.set_argb_color(0x80FF_0000_u32 as i32);
        assert_eq!((g.color(), g.alpha_component()), (0xFF_0000, 0x80));
        g.set_color(0xFF);
        assert_eq!(g.alpha_component(), 0xFF);
        assert_eq!(g.display_color(-1), 0xFF_FFFF);
    }

    #[test]
    fn stroke_styles() {
        let mut g = graphics(1, 1);
        assert_eq!(g.stroke_style(), SOLID);
        g.set_stroke_style(DOTTED).unwrap();
        assert_eq!(g.stroke_style(), DOTTED);
        assert!(g.set_stroke_style(2).is_err());
        g.reset();
        assert_eq!(g.stroke_style(), SOLID);
    }
}
