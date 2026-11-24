//! The colours the runtime draws its own parts in (the soft key bar, the
//! options menu, screen titles) and that `Display.getColor` reports, so
//! the high-level screens and games that ask match what is drawn.

/// `Display.COLOR_BACKGROUND` and the other colour specifiers, in order.
pub const COLOR_SPECIFIERS: std::ops::RangeInclusive<i32> = 0..=5;

/// The runtime's colours, `0x00RRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub background: i32,
    pub foreground: i32,
    pub highlighted_background: i32,
    pub highlighted_foreground: i32,
    pub border: i32,
    pub highlighted_border: i32,
    /// The soft key bar and screen titles.
    pub bar_background: i32,
    pub bar_foreground: i32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: 0xFF_FFFF,
            foreground: 0x00_0000,
            highlighted_background: 0x1F_5FAF,
            highlighted_foreground: 0xFF_FFFF,
            border: 0x80_8080,
            highlighted_border: 0x1F_5FAF,
            bar_background: 0x20_2020,
            bar_foreground: 0xFF_FFFF,
        }
    }
}

impl Theme {
    /// `Display.getColor(specifier)`, or `None` for a value that is not a
    /// colour specifier.
    pub fn color(&self, specifier: i32) -> Option<i32> {
        Some(match specifier {
            0 => self.background,
            1 => self.foreground,
            2 => self.highlighted_background,
            3 => self.highlighted_foreground,
            4 => self.border,
            5 => self.highlighted_border,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_specifier_has_a_colour() {
        let theme = Theme::default();
        for specifier in COLOR_SPECIFIERS {
            assert!(theme.color(specifier).is_some());
        }
        assert_eq!(theme.color(6), None);
        assert_eq!(theme.color(-1), None);
    }
}
