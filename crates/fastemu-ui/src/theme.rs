//! FastEmu's colours: a dark and a light palette sharing one amber accent.

use gpui::{App, Global, Hsla, hsla};

use crate::settings::ThemeMode;

/// The colours every view draws with, kept as a GPUI global so a theme
/// switch repaints everything at once.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    /// The window behind everything.
    pub background: Hsla,
    /// Sidebar, toolbar and tiles.
    pub surface: Hsla,
    /// Buttons and inputs on a surface.
    pub raised: Hsla,
    pub hover: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub accent: Hsla,
    /// Text drawn on the accent colour.
    pub on_accent: Hsla,
    pub danger: Hsla,
    pub warning: Hsla,
    /// Behind a game's screen, where the phone's glass would be.
    pub screen: Hsla,
    /// Dims the window behind a dialog.
    pub scrim: Hsla,
}

impl Global for Theme {}

const fn hsl(h: f32, s: f32, l: f32) -> Hsla {
    Hsla {
        h: h / 360.0,
        s: s / 100.0,
        l: l / 100.0,
        a: 1.0,
    }
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            mode: ThemeMode::Dark,
            background: hsl(222.0, 14.0, 8.0),
            surface: hsl(222.0, 13.0, 11.0),
            raised: hsl(222.0, 12.0, 16.0),
            hover: hsl(222.0, 12.0, 21.0),
            border: hsl(222.0, 11.0, 20.0),
            text: hsl(220.0, 14.0, 91.0),
            muted: hsl(220.0, 9.0, 60.0),
            accent: hsl(36.0, 96.0, 56.0),
            on_accent: hsl(30.0, 40.0, 8.0),
            danger: hsl(0.0, 78.0, 63.0),
            warning: hsl(44.0, 92.0, 62.0),
            screen: hsl(0.0, 0.0, 0.0),
            scrim: hsla(222.0 / 360.0, 0.3, 0.02, 0.6),
        }
    }

    pub fn light() -> Self {
        Self {
            mode: ThemeMode::Light,
            background: hsl(220.0, 16.0, 95.0),
            surface: hsl(0.0, 0.0, 100.0),
            raised: hsl(220.0, 16.0, 93.0),
            hover: hsl(220.0, 16.0, 88.0),
            border: hsl(220.0, 14.0, 86.0),
            text: hsl(222.0, 18.0, 10.0),
            muted: hsl(220.0, 9.0, 40.0),
            accent: hsl(30.0, 92.0, 46.0),
            on_accent: hsl(0.0, 0.0, 100.0),
            danger: hsl(0.0, 70.0, 46.0),
            warning: hsl(36.0, 90.0, 38.0),
            screen: hsl(0.0, 0.0, 0.0),
            scrim: hsla(222.0 / 360.0, 0.3, 0.1, 0.35),
        }
    }

    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    /// The current theme.
    pub fn get(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    /// A family for log text: a monospaced font each platform ships.
    pub fn mono_font() -> &'static str {
        if cfg!(target_os = "windows") {
            "Consolas"
        } else if cfg!(any(target_os = "macos", target_os = "ios")) {
            "Menlo"
        } else if cfg!(target_os = "android") {
            "Droid Sans Mono"
        } else {
            "DejaVu Sans Mono"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_differ_but_keep_text_readable() {
        let (dark, light) = (Theme::dark(), Theme::light());
        assert_ne!(dark.background, light.background);
        for theme in [dark, light] {
            let contrast = (theme.text.l - theme.background.l).abs();
            assert!(contrast > 0.6, "{:?} text contrast {contrast}", theme.mode);
            assert_eq!(Theme::for_mode(theme.mode), theme);
        }
    }
}
