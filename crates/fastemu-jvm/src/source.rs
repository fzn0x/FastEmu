//! Where a virtual machine finds class files and resources.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Supplies a virtual machine with class files and resources.
///
/// The MIDP crate implements it over a suite's JAR; this crate provides a
/// directory source for development and an in-memory source for tests.
pub trait ClassSource: Send + Sync {
    /// The bytes of the class with internal name `name` (such as
    /// `com/example/Game`, without `.class`), or `None` if the source
    /// has no such class.
    fn class_bytes(&self, name: &str) -> Option<Vec<u8>>;

    /// The bytes of the resource at `path`, relative to the root and
    /// without a leading `/` (such as `images/title.png`).
    fn resource(&self, path: &str) -> Option<Vec<u8>>;
}

/// Classes and resources in a directory tree, as `javac -d` writes them.
#[derive(Clone, Debug)]
pub struct DirSource {
    root: PathBuf,
}

impl DirSource {
    /// A source reading from the directory `root`.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn path(&self, relative: &str) -> Option<PathBuf> {
        let relative = Path::new(relative);
        // Only plain names: no `..`, no absolute paths, no drive letters.
        if !relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        {
            return None;
        }
        Some(self.root.join(relative))
    }
}

impl ClassSource for DirSource {
    fn class_bytes(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(&format!("{name}.class"))?).ok()
    }

    fn resource(&self, path: &str) -> Option<Vec<u8>> {
        let file = self.path(path)?;
        if file.is_file() {
            std::fs::read(file).ok()
        } else {
            None
        }
    }
}

/// Classes and resources held in memory, for tests and for hosts that
/// unpack suites themselves.
#[derive(Clone, Debug, Default)]
pub struct MemorySource {
    classes: HashMap<String, Vec<u8>>,
    resources: HashMap<String, Vec<u8>>,
}

impl MemorySource {
    /// An empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the class file for internal name `name`.
    pub fn add_class(&mut self, name: impl Into<String>, bytes: Vec<u8>) -> &mut Self {
        self.classes.insert(name.into(), bytes);
        self
    }

    /// Adds a resource at `path` (a leading `/` is ignored).
    pub fn add_resource(&mut self, path: &str, bytes: Vec<u8>) -> &mut Self {
        self.resources
            .insert(path.trim_start_matches('/').to_owned(), bytes);
        self
    }
}

impl ClassSource for MemorySource {
    fn class_bytes(&self, name: &str) -> Option<Vec<u8>> {
        self.classes.get(name).cloned()
    }

    fn resource(&self, path: &str) -> Option<Vec<u8>> {
        self.resources.get(path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_sources_find_what_was_added() {
        let mut source = MemorySource::new();
        source
            .add_class("a/B", vec![1, 2])
            .add_resource("/res/x.bin", vec![3]);
        assert_eq!(source.class_bytes("a/B"), Some(vec![1, 2]));
        assert_eq!(source.resource("res/x.bin"), Some(vec![3]));
        assert_eq!(source.class_bytes("a/C"), None);
    }

    #[test]
    fn directory_sources_refuse_to_leave_their_root() {
        let dir = std::env::temp_dir().join(format!("fastemu-dirsource-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("res")).unwrap();
        std::fs::write(dir.join("res/a.txt"), b"hi").unwrap();
        let source = DirSource::new(&dir);
        assert_eq!(source.resource("res/a.txt"), Some(b"hi".to_vec()));
        assert_eq!(source.resource("res"), None);
        assert_eq!(source.resource("../etc/passwd"), None);
        assert_eq!(source.class_bytes("../x"), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
