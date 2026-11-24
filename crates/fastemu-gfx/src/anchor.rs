//! Anchor points: which point of an image or a line of text the `x` and `y`
//! of a drawing call name.

use crate::error::{GfxError, Result};

/// `Graphics.HCENTER`.
pub const HCENTER: i32 = 1;
/// `Graphics.VCENTER`: images only.
pub const VCENTER: i32 = 2;
/// `Graphics.LEFT`.
pub const LEFT: i32 = 4;
/// `Graphics.RIGHT`.
pub const RIGHT: i32 = 8;
/// `Graphics.TOP`.
pub const TOP: i32 = 16;
/// `Graphics.BOTTOM`.
pub const BOTTOM: i32 = 32;
/// `Graphics.BASELINE`: text only.
pub const BASELINE: i32 = 64;

const HORIZONTAL: i32 = LEFT | HCENTER | RIGHT;
const VERTICAL: i32 = TOP | VCENTER | BASELINE | BOTTOM;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Horizontal {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Vertical {
    Top,
    /// The middle of an image; text in lenient mode.
    Center,
    /// The baseline of text.
    Baseline,
    Bottom,
}

/// How strictly anchors are checked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AnchorPolicy {
    /// MIDP's rule: zero (top left), or exactly one horizontal and one
    /// vertical constant, with `VCENTER` illegal for text and `BASELINE`
    /// illegal for images. Anything else is `IllegalArgumentException`.
    #[default]
    Strict,
    /// Never fails: the first bit set wins, in the order right, centre,
    /// left and bottom, centre, baseline, top, as J2ME Loader reads
    /// anchors. For games that pass sloppy anchors their phones accepted.
    Lenient,
}

/// A parsed anchor point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    pub horizontal: Horizontal,
    pub vertical: Vertical,
}

/// What an anchor positions, which decides the legal vertical constants.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Image,
    Text,
}

impl Anchor {
    /// `TOP | LEFT`, which an anchor of zero also means.
    pub const TOP_LEFT: Anchor = Anchor {
        horizontal: Horizontal::Left,
        vertical: Vertical::Top,
    };

    /// The anchor of `drawImage`, `drawRegion` and `copyArea`.
    pub fn for_image(bits: i32, policy: AnchorPolicy) -> Result<Self> {
        Self::parse(bits, Target::Image, policy)
    }

    /// The anchor of `drawString` and the other text calls.
    pub fn for_text(bits: i32, policy: AnchorPolicy) -> Result<Self> {
        Self::parse(bits, Target::Text, policy)
    }

    fn parse(bits: i32, target: Target, policy: AnchorPolicy) -> Result<Self> {
        if bits == 0 {
            return Ok(Self::TOP_LEFT);
        }
        if policy == AnchorPolicy::Lenient {
            return Ok(Self::lenient(bits, target));
        }
        let illegal = match target {
            Target::Image => BASELINE,
            Target::Text => VCENTER,
        };
        let horizontal = match bits & HORIZONTAL {
            LEFT => Horizontal::Left,
            HCENTER => Horizontal::Center,
            RIGHT => Horizontal::Right,
            _ => return Err(GfxError::IllegalArgument("invalid anchor")),
        };
        let vertical = match bits & VERTICAL {
            v if v == illegal => return Err(GfxError::IllegalArgument("invalid anchor")),
            TOP => Vertical::Top,
            VCENTER => Vertical::Center,
            BASELINE => Vertical::Baseline,
            BOTTOM => Vertical::Bottom,
            _ => return Err(GfxError::IllegalArgument("invalid anchor")),
        };
        if bits & !(HORIZONTAL | VERTICAL) != 0 {
            return Err(GfxError::IllegalArgument("invalid anchor"));
        }
        Ok(Self {
            horizontal,
            vertical,
        })
    }

    fn lenient(bits: i32, target: Target) -> Self {
        let horizontal = if bits & RIGHT != 0 {
            Horizontal::Right
        } else if bits & HCENTER != 0 {
            Horizontal::Center
        } else {
            Horizontal::Left
        };
        let vertical = if bits & BOTTOM != 0 {
            Vertical::Bottom
        } else if bits & VCENTER != 0 {
            Vertical::Center
        } else if bits & BASELINE != 0 && target == Target::Text {
            Vertical::Baseline
        } else {
            Vertical::Top
        };
        Self {
            horizontal,
            vertical,
        }
    }

    /// The top-left corner of a `width` by `height` box anchored at
    /// `(x, y)`; `baseline` is the box's baseline row for text. Centring
    /// rounds toward the top left.
    pub fn top_left(self, x: i64, y: i64, width: i64, height: i64, baseline: i64) -> (i64, i64) {
        let left = match self.horizontal {
            Horizontal::Left => x,
            Horizontal::Center => x - width / 2,
            Horizontal::Right => x - width,
        };
        let top = match self.vertical {
            Vertical::Top => y,
            Vertical::Center => y - height / 2,
            Vertical::Baseline => y - baseline,
            Vertical::Bottom => y - height,
        };
        (left, top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRICT: AnchorPolicy = AnchorPolicy::Strict;

    #[test]
    fn zero_means_top_left() {
        assert_eq!(Anchor::for_image(0, STRICT), Ok(Anchor::TOP_LEFT));
        assert_eq!(Anchor::for_text(0, STRICT), Ok(Anchor::TOP_LEFT));
        assert_eq!(Anchor::for_image(TOP | LEFT, STRICT), Ok(Anchor::TOP_LEFT));
    }

    #[test]
    fn strict_anchors_need_one_of_each() {
        let ok = |bits, h, v| {
            Anchor::for_image(bits, STRICT)
                == Ok(Anchor {
                    horizontal: h,
                    vertical: v,
                })
        };
        assert!(ok(HCENTER | VCENTER, Horizontal::Center, Vertical::Center));
        assert!(ok(RIGHT | BOTTOM, Horizontal::Right, Vertical::Bottom));
        for bad in [
            LEFT,
            TOP,
            LEFT | RIGHT | TOP,
            TOP | BOTTOM | LEFT,
            BASELINE | LEFT,
            128 | TOP | LEFT,
            -1,
        ] {
            assert!(Anchor::for_image(bad, STRICT).is_err(), "{bad}");
        }
        assert!(Anchor::for_text(VCENTER | LEFT, STRICT).is_err());
        assert_eq!(
            Anchor::for_text(BASELINE | HCENTER, STRICT).map(|a| a.vertical),
            Ok(Vertical::Baseline)
        );
    }

    #[test]
    fn lenient_anchors_never_fail() {
        let lenient = AnchorPolicy::Lenient;
        let a = Anchor::for_image(LEFT | RIGHT | BASELINE, lenient).unwrap();
        assert_eq!(
            (a.horizontal, a.vertical),
            (Horizontal::Right, Vertical::Top)
        );
        let t = Anchor::for_text(VCENTER, lenient).unwrap();
        assert_eq!(
            (t.horizontal, t.vertical),
            (Horizontal::Left, Vertical::Center)
        );
        let b = Anchor::for_text(TOP | BASELINE, lenient).unwrap();
        assert_eq!(b.vertical, Vertical::Baseline);
    }

    #[test]
    fn top_left_offsets() {
        let a = Anchor::for_image(HCENTER | BOTTOM, STRICT).unwrap();
        assert_eq!(a.top_left(10, 20, 5, 4, 0), (8, 16));
        let t = Anchor::for_text(RIGHT | BASELINE, STRICT).unwrap();
        assert_eq!(t.top_left(10, 20, 6, 12, 9), (4, 11));
    }
}
