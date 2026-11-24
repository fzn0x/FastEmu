//! The eight transforms of `javax.microedition.lcdui.game.Sprite`, which
//! `Graphics.drawRegion`, `Image.createImage(Image, ...)` and Nokia's
//! `DirectGraphics` share.

use crate::error::{GfxError, Result};

/// A mirror and rotation of an image region, with MIDP's constant values.
///
/// Rotations turn clockwise on screen, and the mirrored forms reflect the
/// region about its vertical centre first, then rotate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transform {
    /// `TRANS_NONE` (0).
    None = 0,
    /// `TRANS_MIRROR_ROT180` (1): the same as a vertical flip.
    MirrorRot180 = 1,
    /// `TRANS_MIRROR` (2): a horizontal flip.
    Mirror = 2,
    /// `TRANS_ROT180` (3).
    Rot180 = 3,
    /// `TRANS_MIRROR_ROT270` (4): the same as swapping x and y.
    MirrorRot270 = 4,
    /// `TRANS_ROT90` (5).
    Rot90 = 5,
    /// `TRANS_ROT270` (6).
    Rot270 = 6,
    /// `TRANS_MIRROR_ROT90` (7).
    MirrorRot90 = 7,
}

/// Maps a destination offset `(u, v)` inside a transformed region back to
/// the source offset: `sx = x.0 * u + x.1 * v + x.2`, `sy` likewise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Inverse {
    pub x: (i64, i64, i64),
    pub y: (i64, i64, i64),
}

impl Transform {
    /// Every transform, in the order of their MIDP values.
    pub const ALL: [Transform; 8] = [
        Transform::None,
        Transform::MirrorRot180,
        Transform::Mirror,
        Transform::Rot180,
        Transform::MirrorRot270,
        Transform::Rot90,
        Transform::Rot270,
        Transform::MirrorRot90,
    ];

    /// The transform for a `Sprite.TRANS_*` value. Any other value is the
    /// `IllegalArgumentException` MIDP throws.
    pub fn from_midp(value: i32) -> Result<Self> {
        usize::try_from(value)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
            .ok_or(GfxError::IllegalArgument("invalid transform"))
    }

    /// The `Sprite.TRANS_*` value.
    pub fn to_midp(self) -> i32 {
        self as i32
    }

    /// Whether the transform turns the region on its side, so a `w` by `h`
    /// region becomes `h` by `w`.
    pub fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::MirrorRot270 | Self::Rot90 | Self::Rot270 | Self::MirrorRot90
        )
    }

    /// The size of a `width` by `height` region after the transform.
    pub fn transformed_size<T>(self, width: T, height: T) -> (T, T) {
        if self.swaps_axes() {
            (height, width)
        } else {
            (width, height)
        }
    }

    /// Where the pixel at `(x, y)` of a `width` by `height` region lands,
    /// as an offset into the transformed region.
    pub fn apply(self, x: i64, y: i64, width: i64, height: i64) -> (i64, i64) {
        let (w, h) = (width, height);
        match self {
            Self::None => (x, y),
            Self::MirrorRot180 => (x, h - 1 - y),
            Self::Mirror => (w - 1 - x, y),
            Self::Rot180 => (w - 1 - x, h - 1 - y),
            Self::MirrorRot270 => (y, x),
            Self::Rot90 => (h - 1 - y, x),
            Self::Rot270 => (y, w - 1 - x),
            Self::MirrorRot90 => (h - 1 - y, w - 1 - x),
        }
    }

    /// The inverse of [`Transform::apply`] as affine coefficients, so a blit
    /// walks the destination and steps through the source.
    pub(crate) fn inverse(self, width: i64, height: i64) -> Inverse {
        let (w, h) = (width, height);
        let (x, y) = match self {
            Self::None => ((1, 0, 0), (0, 1, 0)),
            Self::MirrorRot180 => ((1, 0, 0), (0, -1, h - 1)),
            Self::Mirror => ((-1, 0, w - 1), (0, 1, 0)),
            Self::Rot180 => ((-1, 0, w - 1), (0, -1, h - 1)),
            Self::MirrorRot270 => ((0, 1, 0), (1, 0, 0)),
            Self::Rot90 => ((0, 1, 0), (-1, 0, h - 1)),
            Self::Rot270 => ((0, -1, w - 1), (1, 0, 0)),
            Self::MirrorRot90 => ((0, -1, w - 1), (-1, 0, h - 1)),
        };
        Inverse { x, y }
    }
}

impl Inverse {
    /// The source offset of destination offset `(u, v)`.
    pub fn map(&self, u: i64, v: i64) -> (i64, i64) {
        (
            self.x.0 * u + self.x.1 * v + self.x.2,
            self.y.0 * u + self.y.1 * v + self.y.2,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midp_values_round_trip() {
        for (i, t) in Transform::ALL.into_iter().enumerate() {
            assert_eq!(Transform::from_midp(i as i32), Ok(t));
            assert_eq!(t.to_midp(), i as i32);
        }
        assert!(Transform::from_midp(8).is_err());
        assert!(Transform::from_midp(-1).is_err());
    }

    #[test]
    fn inverse_undoes_apply() {
        let (w, h) = (3, 5);
        for t in Transform::ALL {
            let (tw, th) = t.transformed_size(w, h);
            let inv = t.inverse(w, h);
            for y in 0..h {
                for x in 0..w {
                    let (u, v) = t.apply(x, y, w, h);
                    assert!((0..tw).contains(&u) && (0..th).contains(&v), "{t:?}");
                    assert_eq!(inv.map(u, v), (x, y), "{t:?}");
                }
            }
        }
    }

    #[test]
    fn rotations_turn_clockwise() {
        // The top-left corner of a 4x2 region goes to the top right after a
        // clockwise quarter turn, which makes the region 2x4.
        assert_eq!(Transform::Rot90.apply(0, 0, 4, 2), (1, 0));
        assert_eq!(Transform::Rot90.apply(3, 0, 4, 2), (1, 3));
        assert_eq!(Transform::Rot270.apply(0, 0, 4, 2), (0, 3));
        // Mirroring happens before the rotation.
        assert_eq!(Transform::MirrorRot90.apply(0, 0, 4, 2), (1, 3));
        assert_eq!(Transform::MirrorRot180.apply(1, 0, 4, 2), (1, 1));
    }
}
