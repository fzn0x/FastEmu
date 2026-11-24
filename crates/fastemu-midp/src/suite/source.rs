//! The [`ClassSource`] over a suite's JAR: where the virtual machine finds
//! the game's classes and resources.
//!
//! Resource names follow `docs/compat/notes/01-suite-loading.md`. The
//! virtual machine resolves a name relative to the package of the class
//! `getResourceAsStream` is called on; this source then turns `\` into
//! `/`, collapses repeated slashes and `.` segments, strips leading
//! slashes, and tries an exact match, a match that ignores case, and
//! finally the same path with its leading directories dropped one at a
//! time, which rescues games that wrote names relative to the root. Misses
//! are logged once per name.

use std::collections::HashSet;
use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::ClassSource;

use super::zip::ZipArchive;

/// Receives the source's warnings: resources a game asked for that the
/// JAR does not have.
pub type SourceLog = Arc<dyn Fn(LogLine) + Send + Sync>;

/// Classes and resources from a suite's JAR.
pub struct JarSource {
    archive: Arc<ZipArchive>,
    log: Option<SourceLog>,
    missed: Mutex<HashSet<String>>,
}

impl fmt::Debug for JarSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JarSource")
            .field("archive", &self.archive)
            .finish_non_exhaustive()
    }
}

impl JarSource {
    /// A source over `archive` that reports misses to `log`, if given.
    pub fn new(archive: Arc<ZipArchive>, log: Option<SourceLog>) -> Self {
        Self {
            archive,
            log,
            missed: Mutex::new(HashSet::new()),
        }
    }

    /// The archive this source reads.
    pub fn archive(&self) -> &Arc<ZipArchive> {
        &self.archive
    }

    fn read(&self, name: &str) -> Option<Vec<u8>> {
        let entry = self.archive.find_str(name)?;
        self.archive.read(entry).ok()
    }

    fn report_miss(&self, path: &str) {
        let Some(log) = &self.log else { return };
        let first = self
            .missed
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_owned());
        if first {
            log(LogLine {
                level: LogLevel::Warn,
                text: format!("resource not found: /{path}"),
            });
        }
    }
}

impl ClassSource for JarSource {
    fn class_bytes(&self, name: &str) -> Option<Vec<u8>> {
        // A game must not replace the runtime's own classes.
        if name.starts_with("java/") || name.starts_with("javax/") {
            return None;
        }
        self.read(&format!("{name}.class"))
    }

    fn resource(&self, path: &str) -> Option<Vec<u8>> {
        let path = normalize(path);
        if path.is_empty() {
            return None;
        }
        let mut candidate = path.as_str();
        loop {
            if let Some(bytes) = self.read(candidate) {
                return Some(bytes);
            }
            match candidate.split_once('/') {
                Some((_, rest)) if !rest.is_empty() => candidate = rest,
                _ => break,
            }
        }
        self.report_miss(&path);
        None
    }
}

/// A resource path as the JAR stores it: `/` separators, no leading
/// slash, no empty or `.` segments, `..` applied.
pub fn normalize(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::super::zip::testing::{TestEntry, write};
    use super::*;

    fn source(entries: &[TestEntry<'_>]) -> (JarSource, Arc<Mutex<Vec<String>>>) {
        let archive = Arc::new(ZipArchive::new(write(entries, true)).unwrap());
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&lines);
        let log: SourceLog = Arc::new(move |line: LogLine| sink.lock().unwrap().push(line.text));
        (JarSource::new(archive, Some(log)), lines)
    }

    #[test]
    fn normalizes_names() {
        assert_eq!(normalize("//img//a.png"), "img/a.png");
        assert_eq!(normalize("\\img\\a.png"), "img/a.png");
        assert_eq!(normalize("a/./b/../c.png"), "a/c.png");
        assert_eq!(normalize("/"), "");
    }

    #[test]
    fn resources_match_exactly_then_without_case() {
        let (src, _) = source(&[
            TestEntry::stored("img/Title.png", b"T"),
            TestEntry::deflated("data/level1.bin", b"L1 L1 L1"),
        ]);
        assert_eq!(src.resource("img/Title.png").as_deref(), Some(&b"T"[..]));
        assert_eq!(src.resource("IMG/title.PNG").as_deref(), Some(&b"T"[..]));
        assert_eq!(
            src.resource("//data\\level1.bin").as_deref(),
            Some(&b"L1 L1 L1"[..])
        );
    }

    #[test]
    fn paths_fall_back_toward_the_root_and_misses_log_once() {
        let (src, lines) = source(&[TestEntry::stored("img/a.png", b"A")]);
        // A relative name the VM resolved against the class's package.
        assert_eq!(
            src.resource("com/game/img/a.png").as_deref(),
            Some(&b"A"[..])
        );
        assert_eq!(src.resource("com/game/missing.png"), None);
        assert_eq!(src.resource("com/game/missing.png"), None);
        assert_eq!(
            *lines.lock().unwrap(),
            ["resource not found: /com/game/missing.png"]
        );
    }

    #[test]
    fn classes_load_by_name_but_never_replace_the_runtime() {
        let (src, _) = source(&[
            TestEntry::stored("a/Game.class", b"G"),
            TestEntry::stored("B.CLASS", b"B"),
            TestEntry::stored("java/lang/String.class", b"S"),
        ]);
        assert_eq!(src.class_bytes("a/Game").as_deref(), Some(&b"G"[..]));
        assert_eq!(src.class_bytes("B").as_deref(), Some(&b"B"[..]));
        assert_eq!(src.class_bytes("java/lang/String"), None);
        assert_eq!(src.class_bytes("a/Missing"), None);
    }
}
