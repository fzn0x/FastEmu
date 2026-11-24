//! FastEmu's GPUI views: the game library and the grid of running games,
//! shared by the desktop and Android apps.
//!
//! The crate uses only GPUI's platform-neutral API. A host app builds the
//! GPUI application for its platform, registers [`Assets`], calls
//! [`init`], and opens a window whose root is a [`FastEmuView`] made from an
//! [`AppConfig`]: the launcher that starts games and the folders to keep
//! files in.

pub mod assets;
pub mod bindings;
pub mod layout;
pub mod library;
pub mod logs;
pub mod pixels;
pub mod screenshot;
pub mod session;
pub mod settings;
pub mod store;
pub mod theme;
pub mod views;

pub use assets::Assets;
pub use session::AppPaths;
pub use views::{AppConfig, FastEmuView, init};
