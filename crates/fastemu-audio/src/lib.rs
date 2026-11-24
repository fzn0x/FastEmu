//! Sound for FastEmu instances: a mixer with a group per instance, and the
//! players behind MMAPI and the vendor sound APIs.

pub mod error;
pub mod format;

pub use error::AudioError;
pub use format::{Format, SUPPORTED_CONTENT_TYPES};
