//! The FastEmu Android app's native library: the GameActivity entry point
//! that runs FastEmu's GPUI views.
//!
//! The modules here hold what the app decides without Android or GPUI, so
//! they build and test on any host: which phone key a hardware key presses,
//! the on-screen keypad, where and how large the phone's screen is drawn,
//! and the game the app runs.

pub mod keymap;
pub mod keypad;
pub mod screen;
pub mod session;

#[cfg(target_os = "android")]
mod android;
