//! FastEmu's GPUI views: the game library and the grid of running games,
//! shared by the desktop and Android apps.

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

pub use assets::Assets;
pub use session::AppPaths;
