//! The bundled font files, scaled to a pixel height, with a glyph cache
//! shared by every instance that asks for the same face and height.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, PoisonError, RwLock};

use fontdue::FontSettings;

/// One of the bundled TrueType files (see `fonts/NOTICE`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FaceFile {
    Sans,
    SansBold,
    Mono,
    MonoBold,
}

static SANS: &[u8] = include_bytes!("../../fonts/DejaVuLGCSans.ttf");
static SANS_BOLD: &[u8] = include_bytes!("../../fonts/DejaVuLGCSans-Bold.ttf");
static MONO: &[u8] = include_bytes!("../../fonts/DejaVuLGCSansMono.ttf");
static MONO_BOLD: &[u8] = include_bytes!("../../fonts/DejaVuLGCSansMono-Bold.ttf");

impl FaceFile {
    pub(crate) fn pick(monospace: bool, bold: bool) -> Self {
        match (monospace, bold) {
            (false, false) => Self::Sans,
            (false, true) => Self::SansBold,
            (true, false) => Self::Mono,
            (true, true) => Self::MonoBold,
        }
    }

    /// The parsed font, parsed once per process on first use.
    fn parsed(self) -> &'static fontdue::Font {
        static PARSED: [OnceLock<fontdue::Font>; 4] = [const { OnceLock::new() }; 4];
        let (slot, bytes) = match self {
            Self::Sans => (0, SANS),
            Self::SansBold => (1, SANS_BOLD),
            Self::Mono => (2, MONO),
            Self::MonoBold => (3, MONO_BOLD),
        };
        PARSED[slot].get_or_init(|| {
            let settings = FontSettings {
                load_substitutions: false,
                ..FontSettings::default()
            };
            // The bytes are compiled in and the tests parse every file, so
            // a failure here is a broken build, not something a game does.
            fontdue::Font::from_bytes(bytes, settings).expect("a bundled font parses")
        })
    }
}

/// A rasterized glyph: coverage from 0 (none) to 255 (full), row-major.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Glyph {
    /// How far the pen moves after the glyph, in whole pixels. Measuring
    /// and drawing both add these, so they always agree.
    pub advance: i32,
    /// The bitmap's left column, relative to the pen.
    pub left: i32,
    /// The bitmap's top row, relative to the baseline (negative is above).
    pub top: i32,
    pub width: usize,
    pub height: usize,
    pub coverage: Box<[u8]>,
}

/// A bundled face scaled so its ascent plus descent is a pixel height.
pub(crate) struct SizedFace {
    file: FaceFile,
    /// The size in pixels per em that gives the requested height.
    px: f32,
    /// `Font.getHeight()`.
    pub height: i32,
    /// `Font.getBaselinePosition()`: rows from the top to the baseline.
    pub ascent: i32,
    /// Glyphs for U+0000 to U+00FF, the bulk of what games draw.
    latin1: Box<[OnceLock<Glyph>]>,
    others: RwLock<HashMap<u16, Arc<Glyph>>>,
}

/// The process-wide cache of sized faces. Each holds a glyph cache, so the
/// games running side by side share rasterized glyphs.
pub(crate) fn sized_face(file: FaceFile, height: i32) -> Arc<SizedFace> {
    type Faces = Mutex<HashMap<(FaceFile, i32), Arc<SizedFace>>>;
    static FACES: OnceLock<Faces> = OnceLock::new();
    let faces = FACES.get_or_init(Mutex::default);
    let mut faces = faces.lock().unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        faces
            .entry((file, height))
            .or_insert_with(|| Arc::new(SizedFace::new(file, height))),
    )
}

/// Characters that draw nothing and take no room: the C0 and C1 controls,
/// such as the line breaks games leave in strings.
fn is_control(unit: u16) -> bool {
    unit < 0x20 || (0x7F..=0x9F).contains(&unit)
}

impl SizedFace {
    fn new(file: FaceFile, height: i32) -> Self {
        let font = file.parsed();
        // Line metrics scale linearly, so measure them at a large size for
        // precision. Split the height into whole rows above and below the
        // baseline in the font's proportions, then take the largest size
        // whose ascent and descent fit those rows, so no glyph within the
        // font's line metrics spills out of the line.
        const PROBE: f32 = 1000.0;
        let (ascent, descent) = font
            .horizontal_line_metrics(PROBE)
            .map_or((0.8, 0.2), |m| (m.ascent / PROBE, -m.descent / PROBE));
        let rows_above =
            ((height as f32 * ascent / (ascent + descent)).round() as i32).clamp(1, height);
        let rows_below = height - rows_above;
        let mut px = rows_above as f32 / ascent;
        if rows_below > 0 {
            px = px.min(rows_below as f32 / descent);
        }
        // A hair smaller, so rounding cannot add a row.
        let px = px * 0.999;
        let ascent = rows_above;
        Self {
            file,
            px,
            height,
            ascent,
            latin1: (0..256).map(|_| OnceLock::new()).collect(),
            others: RwLock::default(),
        }
    }

    /// Runs `f` with the glyph for UTF-16 unit `unit`, rasterizing it on
    /// first use.
    pub fn with_glyph<R>(&self, unit: u16, f: impl FnOnce(&Glyph) -> R) -> R {
        if let Some(slot) = self.latin1.get(usize::from(unit)) {
            return f(slot.get_or_init(|| self.rasterize(unit)));
        }
        let cached = self
            .others
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&unit)
            .cloned();
        let glyph = cached.unwrap_or_else(|| {
            let glyph = Arc::new(self.rasterize(unit));
            self.others
                .write()
                .unwrap_or_else(PoisonError::into_inner)
                .entry(unit)
                .or_insert(glyph)
                .clone()
        });
        f(&glyph)
    }

    /// The advance of `unit` in pixels.
    pub fn advance(&self, unit: u16) -> i32 {
        self.with_glyph(unit, |g| g.advance)
    }

    fn rasterize(&self, unit: u16) -> Glyph {
        if is_control(unit) {
            return Glyph::default();
        }
        let font = self.file.parsed();
        // Lone surrogates are not characters; they draw as missing.
        let index = char::from_u32(unit.into()).map_or(0, |c| font.lookup_glyph_index(c));
        if index == 0 {
            return self.missing_box();
        }
        let (metrics, coverage) = font.rasterize_indexed(index, self.px);
        Glyph {
            advance: (metrics.advance_width.round() as i32).max(0),
            left: metrics.xmin,
            top: -(metrics.ymin + metrics.height as i32),
            width: metrics.width,
            height: metrics.height,
            coverage: coverage.into(),
        }
    }

    /// The hollow box a character the fonts lack draws as, standing on the
    /// baseline with a pixel of room on each side.
    fn missing_box(&self) -> Glyph {
        let advance = (self.height / 2).max(3);
        let width = (advance - 2) as usize;
        let height = (self.ascent * 7 / 10).max(2) as usize;
        let coverage = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let edge = x == 0 || y == 0 || x + 1 == width || y + 1 == height;
                    if edge { 255 } else { 0 }
                })
            })
            .collect();
        Glyph {
            advance,
            left: 1,
            top: -(height as i32),
            width,
            height,
            coverage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundled_file_parses_and_covers_the_three_scripts() {
        for file in [
            FaceFile::Sans,
            FaceFile::SansBold,
            FaceFile::Mono,
            FaceFile::MonoBold,
        ] {
            let font = file.parsed();
            for c in ['A', 'z', '0', 'é', 'Ж', 'я', 'Ω', 'λ', '€'] {
                assert_ne!(font.lookup_glyph_index(c), 0, "{file:?} lacks {c}");
            }
        }
    }

    #[test]
    fn sizes_hit_the_requested_height() {
        for height in [8, 10, 12, 14, 16, 20, 30] {
            let face = sized_face(FaceFile::Sans, height);
            assert_eq!(face.height, height);
            assert!(face.ascent > height / 2 && face.ascent < height, "{height}");
        }
    }

    #[test]
    fn glyphs_sit_on_the_baseline() {
        let face = sized_face(FaceFile::Sans, 16);
        face.with_glyph(u16::from(b'x'), |g| {
            assert!(g.width > 0 && g.height > 0);
            // An x has no descender: its bottom row is just above the
            // baseline.
            assert_eq!(g.top + g.height as i32, 0);
            assert!(g.advance > 0);
        });
        face.with_glyph(u16::from(b'g'), |g| assert!(g.top + g.height as i32 > 0));
    }

    #[test]
    fn missing_and_control_characters() {
        let face = sized_face(FaceFile::Sans, 14);
        let boxed = face.with_glyph(0x4E2D, Clone::clone);
        assert_eq!(boxed.advance, 7);
        assert!(boxed.coverage.contains(&255));
        assert_eq!(face.with_glyph(0xD800, |g| g.advance), boxed.advance);
        assert_eq!(face.advance(u16::from(b'\n')), 0);
        assert_eq!(face.advance(0x85), 0);
    }

    #[test]
    fn monospace_advances_are_equal() {
        let face = sized_face(FaceFile::Mono, 12);
        let w = face.advance(u16::from(b'i'));
        for unit in "MW.Жλ".encode_utf16() {
            assert_eq!(face.advance(unit), w);
        }
    }

    #[test]
    fn the_cache_is_shared() {
        let a = sized_face(FaceFile::MonoBold, 13);
        let b = sized_face(FaceFile::MonoBold, 13);
        assert!(Arc::ptr_eq(&a, &b));
        let first = a.with_glyph(0x0416, |g| g.coverage.as_ptr());
        let again = b.with_glyph(0x0416, |g| g.coverage.as_ptr());
        assert_eq!(first, again);
    }
}
