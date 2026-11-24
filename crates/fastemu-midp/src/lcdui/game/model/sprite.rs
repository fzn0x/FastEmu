//! A sprite: frames cut from one image, the frame sequence, the transform,
//! the reference pixel and the collision rectangle.

use fastemu_gfx::anchor::{LEFT, TOP};
use fastemu_gfx::{GfxError, Graphics, Surface, Transform};

use super::collide::Footprint;
use super::{Area, Base, Cells, GameError};

/// The state of one `Sprite`, without its image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    pub base: Base,
    frames: Cells,
    /// A frame sequence set by `setFrameSequence`; `None` is the default,
    /// every raw frame in order, which is never built as a list.
    sequence: Option<Vec<i32>>,
    /// The position in the frame sequence: `getFrame()`.
    index: i32,
    transform: Transform,
    /// The reference pixel in the untransformed frame.
    reference: (i32, i32),
    /// The collision rectangle in the untransformed frame.
    collision: Area,
}

impl Default for Sprite {
    /// One frame of one pixel: what a `Sprite` holds before its
    /// constructor runs.
    fn default() -> Self {
        Self::from_cells(Cells {
            width: 1,
            height: 1,
            columns: 1,
            count: 1,
        })
    }
}

impl Sprite {
    fn from_cells(frames: Cells) -> Self {
        Self {
            base: Base::default(),
            frames,
            sequence: None,
            index: 0,
            transform: Transform::None,
            reference: (0, 0),
            collision: Area::new(0, 0, frames.width.into(), frames.height.into()),
        }
    }

    /// `Sprite(image, frameWidth, frameHeight)` for an image of the given
    /// size; `Sprite(image)` passes the image's own size as the frame.
    pub fn new(
        image_width: u32,
        image_height: u32,
        frame_width: i32,
        frame_height: i32,
    ) -> Result<Self, GameError> {
        let frames = Cells::cut(image_width, image_height, frame_width, frame_height)?;
        Ok(Self::from_cells(frames))
    }

    /// `getWidth()`: the frame width after the transform.
    pub fn width(&self) -> i32 {
        self.transform
            .transformed_size(self.frames.width, self.frames.height)
            .0
    }

    /// `getHeight()`: the frame height after the transform.
    pub fn height(&self) -> i32 {
        self.transform
            .transformed_size(self.frames.width, self.frames.height)
            .1
    }

    /// Where the reference pixel lands inside the frame under `transform`.
    fn reference_offset(&self, transform: Transform) -> (i64, i64) {
        transform.apply(
            self.reference.0.into(),
            self.reference.1.into(),
            self.frames.width.into(),
            self.frames.height.into(),
        )
    }

    /// `defineReferencePixel(x, y)`: in the untransformed frame, and may
    /// lie outside it. The sprite does not move.
    pub fn define_reference_pixel(&mut self, x: i32, y: i32) {
        self.reference = (x, y);
    }

    /// `getRefPixelX()` and `getRefPixelY()`.
    pub fn ref_pixel(&self) -> (i32, i32) {
        let (dx, dy) = self.reference_offset(self.transform);
        (
            (i64::from(self.base.x) + dx) as i32,
            (i64::from(self.base.y) + dy) as i32,
        )
    }

    /// `setRefPixelPosition(x, y)`: moves the sprite so its reference
    /// pixel lands on `(x, y)`.
    pub fn set_ref_pixel_position(&mut self, x: i32, y: i32) {
        let (dx, dy) = self.reference_offset(self.transform);
        self.base.x = (i64::from(x) - dx) as i32;
        self.base.y = (i64::from(y) - dy) as i32;
    }

    /// The transform, `TRANS_NONE` at first.
    pub fn transform(&self) -> Transform {
        self.transform
    }

    /// `setTransform(transform)`: replaces the transform (they do not
    /// add up) and moves the sprite so the reference pixel stays where it
    /// is.
    pub fn set_transform(&mut self, transform: Transform) {
        let (old_x, old_y) = self.reference_offset(self.transform);
        let (new_x, new_y) = self.reference_offset(transform);
        self.base.x = (i64::from(self.base.x) + old_x - new_x) as i32;
        self.base.y = (i64::from(self.base.y) + old_y - new_y) as i32;
        self.transform = transform;
    }

    /// `defineCollisionRectangle(x, y, width, height)`, in the
    /// untransformed frame; it may reach outside the frame.
    pub fn define_collision_rectangle(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> Result<(), GameError> {
        if width < 0 || height < 0 {
            return Err(GameError::IllegalArgument(
                "the collision rectangle's width or height is negative",
            ));
        }
        self.collision = Area::new(x.into(), y.into(), width.into(), height.into());
        Ok(())
    }

    /// The collision rectangle after the transform, relative to the
    /// sprite's top left corner.
    pub fn collision_rect(&self) -> Area {
        let (fw, fh) = (i64::from(self.frames.width), i64::from(self.frames.height));
        let c = self.collision;
        let (x, y, w, h) = (c.left, c.top, c.width(), c.height());
        let (from_right, from_bottom) = (fw - x - w, fh - y - h);
        let (x, y, w, h) = match self.transform {
            Transform::None => (x, y, w, h),
            Transform::Mirror => (from_right, y, w, h),
            Transform::MirrorRot180 => (x, from_bottom, w, h),
            Transform::Rot180 => (from_right, from_bottom, w, h),
            Transform::Rot90 => (from_bottom, x, h, w),
            Transform::Rot270 => (y, from_right, h, w),
            Transform::MirrorRot90 => (from_bottom, from_right, h, w),
            Transform::MirrorRot270 => (y, x, h, w),
        };
        Area::new(x, y, w, h)
    }

    /// `getRawFrameCount()`.
    pub fn raw_frame_count(&self) -> i32 {
        self.frames.count
    }

    /// `getFrameSequenceLength()`.
    pub fn sequence_len(&self) -> i32 {
        match &self.sequence {
            Some(sequence) => sequence.len() as i32,
            None => self.frames.count,
        }
    }

    /// `getFrame()`: the position in the frame sequence.
    pub fn frame(&self) -> i32 {
        self.index
    }

    /// `setFrame(sequenceIndex)`.
    pub fn set_frame(&mut self, index: i32) -> Result<(), GameError> {
        if index < 0 || index >= self.sequence_len() {
            return Err(GameError::IndexOutOfBounds(
                "the frame index is outside the frame sequence",
            ));
        }
        self.index = index;
        Ok(())
    }

    /// `nextFrame()`: wraps to the start of the sequence.
    pub fn next_frame(&mut self) {
        self.index = (self.index + 1) % self.sequence_len().max(1);
    }

    /// `prevFrame()`: wraps to the end of the sequence.
    pub fn prev_frame(&mut self) {
        self.index = if self.index == 0 {
            self.sequence_len() - 1
        } else {
            self.index - 1
        };
    }

    /// `setFrameSequence(sequence)`: `None` restores the default. Either
    /// way the sequence starts again at its first entry.
    pub fn set_frame_sequence(&mut self, sequence: Option<&[i32]>) -> Result<(), GameError> {
        if let Some(sequence) = sequence {
            if sequence.is_empty() {
                return Err(GameError::IllegalArgument("the frame sequence is empty"));
            }
            if sequence.iter().any(|&f| f < 0 || f >= self.frames.count) {
                return Err(GameError::ArrayIndexOutOfBounds(
                    "the frame sequence names a frame that does not exist",
                ));
            }
        }
        self.sequence = sequence.map(<[i32]>::to_vec);
        self.index = 0;
        Ok(())
    }

    /// The raw frame shown now.
    pub fn raw_frame(&self) -> i32 {
        match &self.sequence {
            Some(sequence) => sequence.get(self.index as usize).copied().unwrap_or(0),
            None => self.index,
        }
    }

    /// The part of the image the current frame covers.
    pub fn frame_region(&self) -> Area {
        self.frames.region(self.raw_frame())
    }

    /// `setImage(image, frameWidth, frameHeight)` for an image of the
    /// given size. With at least as many frames as before, the current
    /// frame and a custom sequence stay; with fewer, both go back to the
    /// start. A new frame size resets the collision rectangle to the whole
    /// frame and moves the sprite so the reference pixel stays put.
    pub fn set_image(
        &mut self,
        image_width: u32,
        image_height: u32,
        frame_width: i32,
        frame_height: i32,
    ) -> Result<(), GameError> {
        let frames = Cells::cut(image_width, image_height, frame_width, frame_height)?;
        if frames.count < self.frames.count {
            self.index = 0;
            self.sequence = None;
        }
        if (frames.width, frames.height) == (self.frames.width, self.frames.height) {
            self.frames = frames;
            return Ok(());
        }
        let (x, y) = self.ref_pixel();
        self.frames = frames;
        self.collision = Area::new(0, 0, frame_width.into(), frame_height.into());
        self.set_ref_pixel_position(x, y);
        Ok(())
    }

    /// What a collision test needs of this sprite.
    pub fn footprint(&self) -> Footprint {
        let (x, y) = (i64::from(self.base.x), i64::from(self.base.y));
        let c = self.collision_rect();
        Footprint {
            visible: self.base.visible,
            bounds: Area::new(x, y, self.width().into(), self.height().into()),
            collision: Area::new(x + c.left, y + c.top, c.width(), c.height()),
            frame: self.frame_region(),
            transform: self.transform,
        }
    }

    /// `paint(g)`: the current frame, transformed, with its top left
    /// corner at the sprite's position, through `drawRegion`. An invisible
    /// sprite draws nothing.
    pub fn paint(
        &self,
        g: &Graphics,
        target: &mut Surface,
        image: &Surface,
    ) -> Result<(), GfxError> {
        if !self.base.visible {
            return Ok(());
        }
        let frame = self.frame_region();
        g.draw_region(
            target,
            image,
            frame.left as i32,
            frame.top as i32,
            self.frames.width,
            self.frames.height,
            self.transform.to_midp(),
            self.base.x,
            self.base.y,
            TOP | LEFT,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{numbered, screen};
    use super::*;

    #[test]
    fn frames_follow_the_spec_rules() {
        assert!(matches!(
            Sprite::new(30, 20, 0, 10),
            Err(GameError::IllegalArgument(_))
        ));
        assert!(matches!(
            Sprite::new(30, 20, 7, 10),
            Err(GameError::IllegalArgument(_))
        ));
        let mut s = Sprite::new(30, 20, 10, 10).unwrap();
        assert_eq!(
            (s.raw_frame_count(), s.sequence_len(), s.frame()),
            (6, 6, 0)
        );
        assert_eq!((s.width(), s.height()), (10, 10));
        s.next_frame();
        assert_eq!(s.frame_region(), Area::new(10, 0, 10, 10));
        s.prev_frame();
        s.prev_frame();
        assert_eq!(s.frame(), 5, "prevFrame wraps to the end");
        assert_eq!(s.frame_region(), Area::new(20, 10, 10, 10));
        s.next_frame();
        assert_eq!(s.frame(), 0, "nextFrame wraps to the start");
        assert!(matches!(
            s.set_frame(6),
            Err(GameError::IndexOutOfBounds(_))
        ));
        assert!(matches!(
            s.set_frame(-1),
            Err(GameError::IndexOutOfBounds(_))
        ));
    }

    #[test]
    fn frame_sequences_are_checked_copied_and_reset() {
        let mut s = Sprite::new(20, 10, 10, 10).unwrap();
        assert!(matches!(
            s.set_frame_sequence(Some(&[])),
            Err(GameError::IllegalArgument(_))
        ));
        assert!(matches!(
            s.set_frame_sequence(Some(&[0, 2])),
            Err(GameError::ArrayIndexOutOfBounds(_))
        ));
        s.set_frame(1).unwrap();
        s.set_frame_sequence(Some(&[1, 1, 0])).unwrap();
        assert_eq!((s.frame(), s.sequence_len(), s.raw_frame()), (0, 3, 1));
        s.set_frame(2).unwrap();
        assert_eq!(s.raw_frame(), 0);
        s.set_frame_sequence(None).unwrap();
        assert_eq!((s.frame(), s.sequence_len()), (0, 2));
    }

    #[test]
    fn set_image_keeps_or_resets_the_frames() {
        let mut s = Sprite::new(20, 10, 10, 10).unwrap();
        s.set_frame_sequence(Some(&[1, 0, 1])).unwrap();
        s.set_frame(2).unwrap();
        s.set_image(30, 10, 10, 10).unwrap();
        assert_eq!(
            (s.frame(), s.sequence_len()),
            (2, 3),
            "more frames keep both"
        );
        s.set_frame_sequence(None).unwrap();
        s.set_frame(2).unwrap();
        s.set_image(40, 10, 10, 10).unwrap();
        assert_eq!(s.sequence_len(), 4, "the default sequence grows");
        assert_eq!(s.frame(), 2);
        s.set_image(10, 10, 10, 10).unwrap();
        assert_eq!((s.frame(), s.sequence_len()), (0, 1), "fewer frames reset");
        assert!(matches!(
            s.set_image(10, 10, 3, 10),
            Err(GameError::IllegalArgument(_))
        ));
    }

    #[test]
    fn set_image_with_a_new_frame_size_keeps_the_reference_pixel() {
        let mut s = Sprite::new(4, 2, 4, 2).unwrap();
        s.define_reference_pixel(1, 0);
        s.define_collision_rectangle(1, 1, 1, 1).unwrap();
        s.set_transform(Transform::Rot90);
        s.set_ref_pixel_position(50, 60);
        s.set_image(6, 6, 3, 3).unwrap();
        assert_eq!(s.ref_pixel(), (50, 60));
        // Under ROT90 the reference (1, 0) of a 3 by 3 frame sits at (2, 1).
        assert_eq!((s.base.x, s.base.y), (48, 59));
        assert_eq!(
            s.collision_rect(),
            Area::new(0, 0, 3, 3),
            "reset to the frame"
        );
        let before = s.clone();
        s.set_image(6, 6, 3, 3).unwrap();
        assert_eq!(s, before, "the same frame size moves nothing");
    }

    /// Where `getX` and `getY` end up for each transform after the
    /// reference pixel (0, 0) of a 3 by 2 frame is put at (10, 10).
    const POSITIONS: [(Transform, (i32, i32)); 8] = [
        (Transform::None, (10, 10)),
        (Transform::MirrorRot180, (10, 9)),
        (Transform::Mirror, (8, 10)),
        (Transform::Rot180, (8, 9)),
        (Transform::MirrorRot270, (10, 10)),
        (Transform::Rot90, (9, 10)),
        (Transform::Rot270, (10, 8)),
        (Transform::MirrorRot90, (9, 8)),
    ];

    #[test]
    fn transforms_keep_the_reference_pixel_in_place() {
        for (t, position) in POSITIONS {
            let mut s = Sprite::new(3, 2, 3, 2).unwrap();
            s.set_ref_pixel_position(10, 10);
            s.set_transform(t);
            assert_eq!((s.base.x, s.base.y), position, "{t:?}");
            assert_eq!(s.ref_pixel(), (10, 10), "{t:?}");
            let (w, h) = t.transformed_size(3, 2);
            assert_eq!((s.width(), s.height()), (w, h), "{t:?}");
        }
    }

    #[test]
    fn the_reference_pixel_is_drawn_at_its_position_under_every_transform() {
        let image = numbered(4, 3);
        for t in Transform::ALL {
            for reference in [(0, 0), (3, 2), (1, 2), (2, 0)] {
                let mut s = Sprite::new(4, 3, 4, 3).unwrap();
                s.define_reference_pixel(reference.0, reference.1);
                s.set_transform(Transform::Rot90);
                s.set_ref_pixel_position(20, 20);
                s.set_transform(t);
                assert_eq!(s.ref_pixel(), (20, 20), "{t:?} {reference:?}");
                let (g, mut target) = screen(40, 40);
                s.paint(&g, &mut target, &image).unwrap();
                let expected = image.pixel(reference.0 as u32, reference.1 as u32);
                assert_eq!(target.pixel(20, 20), expected, "{t:?} {reference:?}");
            }
        }
    }

    #[test]
    fn paint_draws_every_frame_pixel_where_its_transform_puts_it() {
        // Two 3 by 2 frames side by side; the second is shown.
        let image = numbered(6, 2);
        for t in Transform::ALL {
            let mut s = Sprite::new(6, 2, 3, 2).unwrap();
            s.set_frame(1).unwrap();
            s.set_transform(t);
            s.base.x = 5;
            s.base.y = 7;
            let (g, mut target) = screen(20, 20);
            s.paint(&g, &mut target, &image).unwrap();
            for y in 0..2 {
                for x in 0..3 {
                    let (u, v) = t.apply(x, y, 3, 2);
                    let shown = target.pixel((5 + u) as u32, (7 + v) as u32);
                    assert_eq!(shown, image.pixel(3 + x as u32, y as u32), "{t:?} {x},{y}");
                }
            }
            let (w, h) = t.transformed_size(3, 2);
            assert_eq!(target.pixel(5 + w as u32, 7), Some(0xFFFF_FFFF), "{t:?}");
            assert_eq!(target.pixel(5, 7 + h as u32), Some(0xFFFF_FFFF), "{t:?}");
            assert_eq!(target.pixel(4, 7), Some(0xFFFF_FFFF), "{t:?}");
        }
    }

    #[test]
    fn invisible_sprites_draw_nothing() {
        let image = numbered(2, 2);
        let mut s = Sprite::new(2, 2, 2, 2).unwrap();
        s.base.visible = false;
        let (g, mut target) = screen(4, 4);
        s.paint(&g, &mut target, &image).unwrap();
        assert!(target.pixels().iter().all(|&p| p == 0xFFFF_FFFF));
    }

    #[test]
    fn collision_rectangles_follow_the_transform() {
        // A 1 by 2 rectangle at (1, 0) of a 4 by 3 frame.
        let cases = [
            (Transform::None, Area::new(1, 0, 1, 2)),
            (Transform::Mirror, Area::new(2, 0, 1, 2)),
            (Transform::MirrorRot180, Area::new(1, 1, 1, 2)),
            (Transform::Rot180, Area::new(2, 1, 1, 2)),
            (Transform::Rot90, Area::new(1, 1, 2, 1)),
            (Transform::Rot270, Area::new(0, 2, 2, 1)),
            (Transform::MirrorRot90, Area::new(1, 2, 2, 1)),
            (Transform::MirrorRot270, Area::new(0, 1, 2, 1)),
        ];
        let image = numbered(4, 3);
        for (t, expected) in cases {
            let mut s = Sprite::new(4, 3, 4, 3).unwrap();
            s.define_collision_rectangle(1, 0, 1, 2).unwrap();
            s.set_transform(t);
            let (x, y) = (s.base.x, s.base.y);
            assert_eq!(s.collision_rect(), expected, "{t:?}");
            // The rectangle covers exactly the pixels its source pixels
            // are drawn on.
            let (g, mut target) = screen(20, 20);
            s.base.x = 5;
            s.base.y = 5;
            s.paint(&g, &mut target, &image).unwrap();
            let covered: Vec<u32> = (expected.top..expected.bottom)
                .flat_map(|v| (expected.left..expected.right).map(move |u| (u, v)))
                .map(|(u, v)| target.pixel((5 + u) as u32, (5 + v) as u32).unwrap())
                .collect();
            let mut want = vec![image.pixel(1, 0).unwrap(), image.pixel(1, 1).unwrap()];
            let mut got = covered;
            want.sort_unstable();
            got.sort_unstable();
            assert_eq!(got, want, "{t:?} from {x},{y}");
        }
        let mut s = Sprite::new(4, 3, 4, 3).unwrap();
        assert!(matches!(
            s.define_collision_rectangle(0, 0, -1, 1),
            Err(GameError::IllegalArgument(_))
        ));
    }

    #[test]
    fn positions_wrap_like_java_ints() {
        let mut s = Sprite::new(4, 3, 4, 3).unwrap();
        s.define_reference_pixel(2, 0);
        s.set_ref_pixel_position(i32::MIN, 0);
        assert_eq!(s.base.x, i32::MAX - 1);
        assert_eq!(s.ref_pixel(), (i32::MIN, 0));
        s.base.move_by(2, 0);
        assert_eq!(s.base.x, i32::MIN);
    }
}
