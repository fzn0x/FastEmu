//! Sprites, tiled layers and their collisions as plain Rust, with no
//! virtual machine in them, so every rule of JSR-118 can be tested pixel
//! by pixel. The Java classes one module up keep one of these in each
//! layer's payload.
//!
//! The arithmetic follows the MIDP 2.0 reference implementation, which
//! J2ME Loader copies: positions wrap like Java's `int`, the reference
//! pixel is a pixel (so a mirror maps `x` to `width - 1 - x`), and two
//! rectangles collide when their insides overlap.

pub mod collide;
pub mod sprite;
pub mod tiled;

/// What a layer call that breaks a rule of the spec throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameError {
    /// `IllegalArgumentException`.
    IllegalArgument(&'static str),
    /// `IndexOutOfBoundsException`.
    IndexOutOfBounds(&'static str),
    /// `ArrayIndexOutOfBoundsException`.
    ArrayIndexOutOfBounds(&'static str),
    /// `OutOfMemoryError`: a tile grid larger than a phone could hold.
    OutOfMemory,
}

impl GameError {
    /// The internal name of the exception class to throw.
    pub fn java_class(self) -> &'static str {
        match self {
            Self::IllegalArgument(_) => "java/lang/IllegalArgumentException",
            Self::IndexOutOfBounds(_) => "java/lang/IndexOutOfBoundsException",
            Self::ArrayIndexOutOfBounds(_) => "java/lang/ArrayIndexOutOfBoundsException",
            Self::OutOfMemory => "java/lang/OutOfMemoryError",
        }
    }

    /// The detail message.
    pub fn message(self) -> Option<&'static str> {
        match self {
            Self::IllegalArgument(why)
            | Self::IndexOutOfBounds(why)
            | Self::ArrayIndexOutOfBounds(why) => Some(why),
            Self::OutOfMemory => None,
        }
    }
}

/// What every layer has: where it is and whether it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Base {
    pub x: i32,
    pub y: i32,
    pub visible: bool,
}

impl Default for Base {
    /// At the origin and visible, as a new layer is.
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            visible: true,
        }
    }
}

impl Base {
    /// `move(dx, dy)`, wrapping like Java's `int`.
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x = self.x.wrapping_add(dx);
        self.y = self.y.wrapping_add(dy);
    }
}

/// An image cut into equal cells, read left to right and top to bottom:
/// a sprite's frames or a tiled layer's tiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cells {
    /// The size of one cell.
    pub width: i32,
    pub height: i32,
    /// Cells in one row of the image.
    pub columns: i32,
    /// Cells in the whole image.
    pub count: i32,
}

impl Cells {
    /// Cuts an `image_width` by `image_height` image into `width` by
    /// `height` cells. A size below 1, or one that does not divide the
    /// image exactly, is the spec's `IllegalArgumentException`.
    pub fn cut(
        image_width: u32,
        image_height: u32,
        width: i32,
        height: i32,
    ) -> Result<Self, GameError> {
        if width < 1 || height < 1 {
            return Err(GameError::IllegalArgument(
                "the frame or tile size is less than 1",
            ));
        }
        let (iw, ih) = (i64::from(image_width), i64::from(image_height));
        let (w, h) = (i64::from(width), i64::from(height));
        if iw % w != 0 || ih % h != 0 {
            return Err(GameError::IllegalArgument(
                "the image size is not a multiple of the frame or tile size",
            ));
        }
        // An image holds at most 2^25 pixels, so the count fits.
        Ok(Self {
            width,
            height,
            columns: (iw / w) as i32,
            count: ((iw / w) * (ih / h)) as i32,
        })
    }

    /// The part of the image cell `index` (from 0) covers.
    pub fn region(&self, index: i32) -> Area {
        let columns = self.columns.max(1);
        let x = i64::from(index % columns) * i64::from(self.width);
        let y = i64::from(index / columns) * i64::from(self.height);
        Area::new(x, y, self.width.into(), self.height.into())
    }
}

/// A rectangle on the layer plane, `[left, right)` by `[top, bottom)`, in
/// 64-bit coordinates so that positions near the ends of Java's `int`
/// range cannot overflow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub left: i64,
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
}

impl Area {
    /// The rectangle at `(x, y)` of the given size.
    pub fn new(x: i64, y: i64, width: i64, height: i64) -> Self {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// The width, never negative.
    pub fn width(&self) -> i64 {
        (self.right - self.left).max(0)
    }

    /// The height, never negative.
    pub fn height(&self) -> i64 {
        (self.bottom - self.top).max(0)
    }

    /// Whether it covers no pixel.
    pub fn is_empty(&self) -> bool {
        self.right <= self.left || self.bottom <= self.top
    }

    /// The part inside `other`, possibly empty.
    #[must_use]
    pub fn intersect(self, other: Area) -> Area {
        Area {
            left: self.left.max(other.left),
            top: self.top.max(other.top),
            right: self.right.min(other.right),
            bottom: self.bottom.min(other.bottom),
        }
    }

    /// The reference implementation's rectangle test: each one's left
    /// edge lies before the other's right edge, and the same for top and
    /// bottom. A rectangle of no width inside another still overlaps it.
    pub fn overlaps(&self, other: &Area) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

/// Images and screens for the tests of the layers.
#[cfg(test)]
pub(crate) mod testing {
    use fastemu_core::FontSizes;
    use fastemu_gfx::{FontLibrary, Graphics, Surface, SurfaceKind, TextRendering};

    /// A `width` by `height` image whose pixel `(x, y)` is
    /// `0xFF000000 | x << 8 | (y + 1)`, so every pixel names its place.
    pub fn numbered(width: u32, height: u32) -> Surface {
        let pixels = (0..height)
            .flat_map(|y| (0..width).map(move |x| 0xFF00_0000 | x << 8 | (y + 1)))
            .collect();
        Surface::from_pixels(width, height, pixels, SurfaceKind::Immutable).unwrap()
    }

    /// A white mutable image and a `Graphics` on it.
    pub fn screen(width: u32, height: u32) -> (Graphics, Surface) {
        let fonts = FontLibrary::new(
            FontSizes {
                small: 12,
                medium: 14,
                large: 16,
            },
            TextRendering::Antialiased,
        );
        let surface = Surface::new_mutable(width as i32, height as i32).unwrap();
        let g = Graphics::new(&surface, fonts.default_font()).unwrap();
        (g, surface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_cut_into_whole_cells() {
        let cells = Cells::cut(30, 20, 10, 10).unwrap();
        assert_eq!((cells.columns, cells.count), (3, 6));
        assert_eq!(cells.region(4), Area::new(10, 10, 10, 10));
        assert!(matches!(
            Cells::cut(30, 20, 0, 10),
            Err(GameError::IllegalArgument(_))
        ));
        assert!(matches!(
            Cells::cut(30, 20, 7, 10),
            Err(GameError::IllegalArgument(_))
        ));
        assert!(matches!(
            Cells::cut(30, 20, 10, 15),
            Err(GameError::IllegalArgument(_))
        ));
    }

    #[test]
    fn overlap_follows_the_reference_test() {
        let a = Area::new(0, 0, 10, 10);
        assert!(a.overlaps(&Area::new(9, 9, 5, 5)));
        assert!(!a.overlaps(&Area::new(10, 0, 5, 5)), "edges touch only");
        assert!(a.overlaps(&Area::new(5, 5, 0, 0)), "a point inside");
        assert!(!Area::new(5, 5, 0, 0).overlaps(&Area::new(5, 5, 0, 0)));
        assert!(a.intersect(Area::new(5, 5, 0, 0)).is_empty());
    }
}
