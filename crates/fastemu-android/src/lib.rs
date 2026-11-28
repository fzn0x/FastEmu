//! The FastEmu Android app's native library: the GameActivity entry point
//! that runs FastEmu's GPUI views (`fastemu-ui`) on `gpui_android`, with
//! the real launcher and sound.
//!
//! `files` and `bench_log` hold what needs neither Android nor GPUI, so
//! they build and test on any host.

pub mod bench_log;
pub mod files;

#[cfg(target_os = "android")]
mod android;
