//! The FastEmu Android app's native library: the GameActivity entry point
//! that runs FastEmu's GPUI views (`fastemu-ui`) on `gpui_android`, with
//! the real launcher and sound.
//!
//! `files` holds what needs neither Android nor GPUI, so it builds and
//! tests on any host.

pub mod files;

#[cfg(target_os = "android")]
mod android;
