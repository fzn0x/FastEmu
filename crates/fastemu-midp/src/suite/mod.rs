//! MIDlet suites: opening a `.jar` or a `.jad`, reading what they say
//! about themselves, and serving their classes and resources.

pub mod attributes;
pub mod source;
pub mod zip;

pub use source::JarSource;
pub use zip::ZipArchive;
