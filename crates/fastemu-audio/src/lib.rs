//! Sound for FastEmu instances: a mixer with a group per instance, and the
//! players behind MMAPI and the vendor sound APIs.

pub mod error;
pub mod format;

// The decoders come before the mixer that plays them; until it lands in a
// later commit, nothing outside the tests calls them.
#[allow(dead_code)]
mod gain;
#[allow(dead_code)]
mod pcm;
#[allow(dead_code)]
mod source;
#[allow(dead_code)]
mod tone;
#[allow(dead_code)]
mod wav;

pub use error::AudioError;
pub use format::{Format, SUPPORTED_CONTENT_TYPES};
