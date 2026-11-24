//! MIDlet suites: opening a `.jar` or a `.jad`, reading what they say
//! about themselves, and serving their classes and resources.

pub mod zip;

pub use zip::ZipArchive;
