//! MIDP's `Font`: faces, styles and sizes mapped onto the bundled DejaVu
//! LGC fonts, with metrics that match what [`crate::Graphics`] draws.
//!
//! A [`FontLibrary`] belongs to one instance and knows its phone's
//! [`FontSizes`]. The glyphs behind it are cached per face and pixel
//! height for the whole process, so instances share them.

mod face;

use std::fmt;
use std::sync::{Arc, OnceLock};

use fastemu_core::FontSizes;

use crate::error::{GfxError, Result};
use crate::surface::java_range;
use face::{FaceFile, sized_face};
pub(crate) use face::{Glyph, SizedFace};

/// `Font.FACE_SYSTEM`.
pub const FACE_SYSTEM: i32 = 0;
/// `Font.FACE_MONOSPACE`.
pub const FACE_MONOSPACE: i32 = 32;
/// `Font.FACE_PROPORTIONAL`.
pub const FACE_PROPORTIONAL: i32 = 64;
/// `Font.STYLE_PLAIN`.
pub const STYLE_PLAIN: i32 = 0;
/// `Font.STYLE_BOLD`.
pub const STYLE_BOLD: i32 = 1;
/// `Font.STYLE_ITALIC`.
pub const STYLE_ITALIC: i32 = 2;
/// `Font.STYLE_UNDERLINED`.
pub const STYLE_UNDERLINED: i32 = 4;
/// `Font.SIZE_SMALL`.
pub const SIZE_SMALL: i32 = 8;
/// `Font.SIZE_MEDIUM`.
pub const SIZE_MEDIUM: i32 = 0;
/// `Font.SIZE_LARGE`.
pub const SIZE_LARGE: i32 = 16;
/// `Font.FONT_STATIC_TEXT`, a specifier for `Font.getFont(int)`.
pub const FONT_STATIC_TEXT: i32 = 0;
/// `Font.FONT_INPUT_TEXT`, a specifier for `Font.getFont(int)`.
pub const FONT_INPUT_TEXT: i32 = 1;

/// A font face. SYSTEM and PROPORTIONAL both use DejaVu LGC Sans, as
/// most phones drew them alike; MONOSPACE uses DejaVu LGC Sans Mono.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    System,
    Monospace,
    Proportional,
}

impl Face {
    /// The face for a `Font.FACE_*` value.
    pub fn from_midp(value: i32) -> Result<Self> {
        match value {
            FACE_SYSTEM => Ok(Self::System),
            FACE_MONOSPACE => Ok(Self::Monospace),
            FACE_PROPORTIONAL => Ok(Self::Proportional),
            _ => Err(GfxError::IllegalArgument("invalid font face")),
        }
    }

    pub fn to_midp(self) -> i32 {
        match self {
            Self::System => FACE_SYSTEM,
            Self::Monospace => FACE_MONOSPACE,
            Self::Proportional => FACE_PROPORTIONAL,
        }
    }
}

/// A font size, whose pixel height comes from the phone's [`FontSizes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    /// The size for a `Font.SIZE_*` value.
    pub fn from_midp(value: i32) -> Result<Self> {
        match value {
            SIZE_SMALL => Ok(Self::Small),
            SIZE_MEDIUM => Ok(Self::Medium),
            SIZE_LARGE => Ok(Self::Large),
            _ => Err(GfxError::IllegalArgument("invalid font size")),
        }
    }

    pub fn to_midp(self) -> i32 {
        match self {
            Self::Small => SIZE_SMALL,
            Self::Medium => SIZE_MEDIUM,
            Self::Large => SIZE_LARGE,
        }
    }

    /// The height in pixels on a phone with these sizes.
    pub fn pixels(self, sizes: &FontSizes) -> u32 {
        match self {
            Self::Small => sizes.small,
            Self::Medium => sizes.medium,
            Self::Large => sizes.large,
        }
    }
}

/// A combination of `Font.STYLE_*` bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style(u8);

impl Style {
    pub const PLAIN: Style = Style(0);
    pub const BOLD: Style = Style(STYLE_BOLD as u8);
    pub const ITALIC: Style = Style(STYLE_ITALIC as u8);
    pub const UNDERLINED: Style = Style(STYLE_UNDERLINED as u8);

    /// The style for a `Font.STYLE_*` combination. Other bits are the
    /// `IllegalArgumentException` MIDP throws.
    pub fn from_midp(value: i32) -> Result<Self> {
        if value & !(STYLE_BOLD | STYLE_ITALIC | STYLE_UNDERLINED) != 0 {
            return Err(GfxError::IllegalArgument("invalid font style"));
        }
        Ok(Self(value as u8))
    }

    pub fn to_midp(self) -> i32 {
        self.0.into()
    }

    /// Both styles' bits.
    #[must_use]
    pub fn with(self, other: Style) -> Style {
        Style(self.0 | other.0)
    }

    pub fn is_bold(self) -> bool {
        self.0 & Self::BOLD.0 != 0
    }

    pub fn is_italic(self) -> bool {
        self.0 & Self::ITALIC.0 != 0
    }

    pub fn is_underlined(self) -> bool {
        self.0 & Self::UNDERLINED.0 != 0
    }
}

/// How glyph edges are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextRendering {
    /// Smooth edges blended into the background.
    #[default]
    Antialiased,
    /// Hard edges: a pixel is drawn when the glyph covers at least half of
    /// it, as on phones without font smoothing.
    Aliased,
}

/// The fonts of one instance, sized for its phone.
pub struct FontLibrary {
    sizes: FontSizes,
    rendering: TextRendering,
    /// One slot per face, style and size, so `Font.getFont` hands out the
    /// same font for the same arguments.
    fonts: [OnceLock<Font>; 3 * 8 * 3],
}

impl FontLibrary {
    pub fn new(sizes: FontSizes, rendering: TextRendering) -> Self {
        Self {
            sizes,
            rendering,
            fonts: [const { OnceLock::new() }; 3 * 8 * 3],
        }
    }

    pub fn sizes(&self) -> FontSizes {
        self.sizes
    }

    pub fn rendering(&self) -> TextRendering {
        self.rendering
    }

    /// `Font.getFont(face, style, size)`, which throws
    /// `IllegalArgumentException` for values outside MIDP's constants.
    pub fn get_font(&self, face: i32, style: i32, size: i32) -> Result<Font> {
        Ok(self.font(
            Face::from_midp(face)?,
            Style::from_midp(style)?,
            Size::from_midp(size)?,
        ))
    }

    /// `Font.getFont(fontSpecifier)`: both specifiers name the default font.
    pub fn get_font_for(&self, specifier: i32) -> Result<Font> {
        match specifier {
            FONT_STATIC_TEXT | FONT_INPUT_TEXT => Ok(self.default_font()),
            _ => Err(GfxError::IllegalArgument("invalid font specifier")),
        }
    }

    /// `Font.getDefaultFont()`: plain, medium, system.
    pub fn default_font(&self) -> Font {
        self.font(Face::System, Style::PLAIN, Size::Medium)
    }

    pub fn font(&self, face: Face, style: Style, size: Size) -> Font {
        let slot = (face as usize * 8 + usize::from(style.0)) * 3 + size as usize;
        self.fonts[slot]
            .get_or_init(|| {
                let height = size.pixels(&self.sizes).clamp(1, 512) as i32;
                let file = FaceFile::pick(face == Face::Monospace, style.is_bold());
                Font(Arc::new(FontInner {
                    face,
                    style,
                    size,
                    rendering: self.rendering,
                    sized: sized_face(file, height),
                }))
            })
            .clone()
    }
}

impl fmt::Debug for FontLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontLibrary")
            .field("sizes", &self.sizes)
            .field("rendering", &self.rendering)
            .finish_non_exhaustive()
    }
}

struct FontInner {
    face: Face,
    style: Style,
    size: Size,
    rendering: TextRendering,
    sized: Arc<SizedFace>,
}

/// A MIDP font. Cheap to clone; clones are the same font.
///
/// Widths add up glyph by glyph in whole pixels, and text drawing places
/// each glyph at the sum of the widths before it, so `stringWidth` is
/// exactly how far a string reaches.
#[derive(Clone)]
pub struct Font(Arc<FontInner>);

impl Font {
    pub fn face(&self) -> Face {
        self.0.face
    }

    pub fn style(&self) -> Style {
        self.0.style
    }

    pub fn size(&self) -> Size {
        self.0.size
    }

    /// `Font.isPlain()`.
    pub fn is_plain(&self) -> bool {
        self.0.style == Style::PLAIN
    }

    /// `Font.isBold()`.
    pub fn is_bold(&self) -> bool {
        self.0.style.is_bold()
    }

    /// `Font.isItalic()`.
    pub fn is_italic(&self) -> bool {
        self.0.style.is_italic()
    }

    /// `Font.isUnderlined()`.
    pub fn is_underlined(&self) -> bool {
        self.0.style.is_underlined()
    }

    pub fn rendering(&self) -> TextRendering {
        self.0.rendering
    }

    /// `Font.getHeight()`: the line height, from the phone's font sizes.
    pub fn height(&self) -> i32 {
        self.0.sized.height
    }

    /// `Font.getBaselinePosition()`: rows from the top of a line to its
    /// baseline.
    pub fn baseline_position(&self) -> i32 {
        self.0.sized.ascent
    }

    /// `Font.charWidth(ch)`.
    pub fn char_width(&self, ch: u16) -> i32 {
        self.0.sized.advance(ch)
    }

    /// `Font.charsWidth(ch, offset, length)`, which throws
    /// `ArrayIndexOutOfBoundsException` for a range outside `chars`.
    pub fn chars_width(&self, chars: &[u16], offset: i32, length: i32) -> Result<i32> {
        let range =
            java_range(chars.len(), offset, length).ok_or(GfxError::ArrayIndexOutOfBounds)?;
        Ok(self.string_width(&chars[range]))
    }

    /// `Font.stringWidth(str)`, with the string as UTF-16 units.
    pub fn string_width(&self, text: &[u16]) -> i32 {
        self.width_of(text.iter().copied())
    }

    /// `Font.substringWidth(str, offset, len)`, which throws
    /// `StringIndexOutOfBoundsException` for a range outside `text`.
    pub fn substring_width(&self, text: &[u16], offset: i32, length: i32) -> Result<i32> {
        let range =
            java_range(text.len(), offset, length).ok_or(GfxError::StringIndexOutOfBounds)?;
        Ok(self.string_width(&text[range]))
    }

    /// The width of a Rust string, measured as its UTF-16 units.
    pub fn str_width(&self, text: &str) -> i32 {
        self.width_of(text.encode_utf16())
    }

    fn width_of(&self, units: impl Iterator<Item = u16>) -> i32 {
        let sized = &self.0.sized;
        let total: i64 = units.map(|u| i64::from(sized.advance(u))).sum();
        total.min(i64::from(i32::MAX)) as i32
    }

    pub(crate) fn sized(&self) -> &SizedFace {
        &self.0.sized
    }
}

impl PartialEq for Font {
    /// Fonts are equal when they draw alike.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.face == other.0.face
                && self.0.style == other.0.style
                && self.0.rendering == other.0.rendering
                && Arc::ptr_eq(&self.0.sized, &other.0.sized))
    }
}

impl Eq for Font {}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("face", &self.0.face)
            .field("style", &self.0.style)
            .field("size", &self.0.size)
            .field("height", &self.height())
            .field("rendering", &self.0.rendering)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZES: FontSizes = FontSizes {
        small: 12,
        medium: 14,
        large: 16,
    };

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn get_font_checks_midp_constants() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Antialiased);
        let font = fonts
            .get_font(FACE_MONOSPACE, STYLE_BOLD | STYLE_UNDERLINED, SIZE_LARGE)
            .unwrap();
        assert_eq!(font.face(), Face::Monospace);
        assert!(font.is_bold() && font.is_underlined() && !font.is_italic() && !font.is_plain());
        assert_eq!(font.style().to_midp(), 5);
        assert_eq!(font.size().to_midp(), SIZE_LARGE);
        assert_eq!(font.height(), 16);
        let iae = |r: Result<Font>| matches!(r, Err(GfxError::IllegalArgument(_)));
        assert!(iae(fonts.get_font(1, 0, 0)));
        assert!(iae(fonts.get_font(0, 8, 0)));
        assert!(iae(fonts.get_font(0, 0, 4)));
        assert!(iae(fonts.get_font_for(2)));
        assert_eq!(
            fonts.get_font_for(FONT_INPUT_TEXT).unwrap(),
            fonts.default_font()
        );
    }

    #[test]
    fn heights_follow_the_phone() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Aliased);
        assert_eq!(
            fonts.font(Face::System, Style::PLAIN, Size::Small).height(),
            12
        );
        assert_eq!(fonts.default_font().height(), 14);
        let large = fonts.font(Face::Proportional, Style::ITALIC, Size::Large);
        assert_eq!(large.height(), 16);
        assert!(large.baseline_position() > 8 && large.baseline_position() < 16);
        assert_eq!(large.rendering(), TextRendering::Aliased);
    }

    #[test]
    fn widths_add_up_per_character() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Antialiased);
        let font = fonts.default_font();
        let text = utf16("Hello, Мир! Ωμέγα");
        let sum: i32 = text.iter().map(|&u| font.char_width(u)).sum();
        assert_eq!(font.string_width(&text), sum);
        assert_eq!(font.str_width("Hello, Мир! Ωμέγα"), sum);
        assert_eq!(font.chars_width(&text, 0, 5), Ok(font.str_width("Hello")));
        assert_eq!(font.substring_width(&text, 7, 3), Ok(font.str_width("Мир")));
        assert!(font.char_width(u16::from(b'W')) > font.char_width(u16::from(b'i')));
    }

    #[test]
    fn bold_is_wider_and_styles_share_metrics() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Antialiased);
        let plain = fonts.default_font();
        let bold = fonts.font(Face::System, Style::BOLD, Size::Medium);
        let italic = fonts.font(
            Face::System,
            Style::ITALIC.with(Style::UNDERLINED),
            Size::Medium,
        );
        assert!(bold.str_width("Width") > plain.str_width("Width"));
        assert_eq!(italic.str_width("Width"), plain.str_width("Width"));
        assert_eq!(italic.baseline_position(), plain.baseline_position());
    }

    #[test]
    fn ranges_throw_the_right_exceptions() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Antialiased);
        let font = fonts.default_font();
        let text = utf16("abc");
        assert_eq!(
            font.chars_width(&text, 1, 3),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        assert_eq!(
            font.chars_width(&text, -1, 1),
            Err(GfxError::ArrayIndexOutOfBounds)
        );
        assert_eq!(font.chars_width(&text, 3, 0), Ok(0));
        assert_eq!(
            font.substring_width(&text, 0, -1),
            Err(GfxError::StringIndexOutOfBounds)
        );
        assert_eq!(
            font.substring_width(&text, 2, 2),
            Err(GfxError::StringIndexOutOfBounds)
        );
    }

    #[test]
    fn getfont_returns_the_same_font_twice() {
        let fonts = FontLibrary::new(SIZES, TextRendering::Antialiased);
        let a = fonts
            .get_font(FACE_PROPORTIONAL, STYLE_ITALIC, SIZE_SMALL)
            .unwrap();
        let b = fonts
            .get_font(FACE_PROPORTIONAL, STYLE_ITALIC, SIZE_SMALL)
            .unwrap();
        assert!(Arc::ptr_eq(&a.0, &b.0));
        assert_ne!(a, fonts.default_font());
    }
}
