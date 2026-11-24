//! The icons FastEmu draws, built into the binary and served to GPUI
//! through an [`AssetSource`]. They are Lucide icons (ISC license, see
//! `assets/icons/LICENSE`).

use std::borrow::Cow;

use gpui::{AssetSource, SharedString};

macro_rules! icons {
    ($($variant:ident => $file:literal,)*) => {
        /// An icon FastEmu draws. Each maps to one SVG under `icons/`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Icon {
            $($variant,)*
        }

        impl Icon {
            /// Every icon, for checks that the files are all present.
            pub const ALL: &[Icon] = &[$(Icon::$variant,)*];

            /// The asset path GPUI's `svg()` element loads.
            pub fn path(self) -> &'static str {
                match self {
                    $(Icon::$variant => concat!("icons/", $file, ".svg"),)*
                }
            }

            fn bytes(self) -> &'static [u8] {
                match self {
                    $(Icon::$variant => include_bytes!(concat!("../assets/icons/", $file, ".svg")),)*
                }
            }
        }
    };
}

icons! {
    Camera => "camera",
    Check => "check",
    ChevronDown => "chevron-down",
    ChevronLeft => "chevron-left",
    ChevronRight => "chevron-right",
    ChevronUp => "chevron-up",
    Close => "x",
    Crash => "circle-x",
    FilePlus => "file-plus",
    Focus => "focus",
    Gamepad => "gamepad-2",
    Globe => "globe",
    Grid => "layout-grid",
    Info => "info",
    Keyboard => "keyboard",
    Log => "scroll-text",
    Moon => "moon",
    Pause => "pause",
    Phone => "smartphone",
    Play => "play",
    Plus => "plus",
    Restart => "rotate-ccw",
    Settings => "settings",
    SidebarClose => "panel-left-close",
    SidebarOpen => "panel-left-open",
    Sliders => "sliders-horizontal",
    Sun => "sun",
    Text => "type",
    Trash => "trash-2",
    Volume => "volume-2",
    VolumeOff => "volume-x",
    Warning => "triangle-alert",
}

/// Serves FastEmu's built-in assets. Pass it to
/// `Application::with_assets` before opening a window.
#[derive(Clone, Copy, Debug, Default)]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        Ok(Icon::ALL
            .iter()
            .find(|icon| icon.path() == path)
            .map(|icon| Cow::Borrowed(icon.bytes())))
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<SharedString>> {
        Ok(Icon::ALL
            .iter()
            .map(|icon| icon.path())
            .filter(|p| p.starts_with(path))
            .map(SharedString::new_static)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_loads_as_svg() {
        for icon in Icon::ALL {
            let bytes = Assets
                .load(icon.path())
                .expect("loads")
                .unwrap_or_else(|| panic!("{icon:?} is missing"));
            let text = std::str::from_utf8(&bytes).expect("utf-8");
            assert!(text.contains("<svg"), "{icon:?}");
        }
        assert!(Assets.load("icons/none.svg").expect("loads").is_none());
        assert_eq!(Assets.list("icons/").expect("lists").len(), Icon::ALL.len());
    }
}
