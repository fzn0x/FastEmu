//! The games the user added: what `library.json` holds.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use fastemu_core::SuiteInfo;
use serde::{Deserialize, Serialize};

use crate::settings::GameSettings;

/// Identifies a library entry for as long as it stays in the library,
/// across restarts. Saves of a game live in a folder named after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntryId(pub u64);

/// One added game.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub id: EntryId,
    /// The `.jad` or `.jar` the user added.
    pub path: PathBuf,
    /// `MIDlet-Name`, or the file name when the suite does not say.
    pub name: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub settings: GameSettings,
}

impl LibraryEntry {
    /// Refreshes what the suite says about itself, keeping the settings.
    pub fn update_info(&mut self, info: &SuiteInfo) {
        if !info.name.trim().is_empty() {
            info.name.trim().clone_into(&mut self.name);
        }
        info.vendor.trim().clone_into(&mut self.vendor);
        info.version.trim().clone_into(&mut self.version);
    }
}

/// The ordered list of added games.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    entries: Vec<LibraryEntry>,
    /// The id the next added game gets. Ids are never handed out twice,
    /// so a new game never inherits the saves of a removed one.
    next_id: u64,
}

impl Library {
    pub fn entries(&self) -> &[LibraryEntry] {
        &self.entries
    }

    pub fn get(&self, id: EntryId) -> Option<&LibraryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn get_mut(&mut self, id: EntryId) -> Option<&mut LibraryEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// The entry for `path`, if the game is in the library already.
    pub fn find_path(&self, path: &Path) -> Option<EntryId> {
        self.entries
            .iter()
            .find(|e| same_file(&e.path, path))
            .map(|e| e.id)
    }

    /// Adds the suite at `path`, described by `info`. Adding a file that is
    /// already in the library returns the existing entry instead of a
    /// duplicate.
    pub fn add(&mut self, path: PathBuf, info: &SuiteInfo) -> EntryId {
        if let Some(id) = self.find_path(&path) {
            return id;
        }
        let id = self.fresh_id();
        self.next_id = id.0.saturating_add(1);
        let fallback = path.file_stem().map_or_else(
            || "Untitled".to_owned(),
            |s| s.to_string_lossy().into_owned(),
        );
        let mut entry = LibraryEntry {
            id,
            path,
            name: fallback,
            vendor: String::new(),
            version: String::new(),
            settings: GameSettings::default(),
        };
        entry.update_info(info);
        self.entries.push(entry);
        id
    }

    /// An id above every one handed out so far. Only a hand-edited file
    /// can use up the ids; then the smallest id not in use serves.
    fn fresh_id(&self) -> EntryId {
        let highest = self.entries.iter().map(|e| e.id.0).max().unwrap_or(0);
        if let Some(above) = highest.checked_add(1) {
            return EntryId(self.next_id.max(above));
        }
        let mut used: Vec<u64> = self
            .entries
            .iter()
            .map(|e| e.id.0)
            .filter(|&id| id > 0)
            .collect();
        used.sort_unstable();
        used.dedup();
        // Distinct ids from 1 up: the first position that differs is free.
        let free = (1..)
            .zip(&used)
            .find(|&(want, &have)| want != have)
            .map_or(used.len() as u64 + 1, |(want, _)| want);
        EntryId(free)
    }

    pub fn remove(&mut self, id: EntryId) -> Option<LibraryEntry> {
        let index = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(index))
    }

    /// Drops entries with a repeated id or path, which only a hand-edited
    /// file can contain, keeping the first of each.
    pub fn dedup(&mut self) {
        let mut ids = HashSet::new();
        let mut kept: Vec<LibraryEntry> = Vec::with_capacity(self.entries.len());
        for entry in std::mem::take(&mut self.entries) {
            let repeated_path = kept.iter().any(|k| same_file(&k.path, &entry.path));
            if ids.insert(entry.id) && !repeated_path {
                kept.push(entry);
            }
        }
        self.entries = kept;
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Why a picked or dropped file was not added.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejected {
    /// Neither a `.jar` nor a `.jad`.
    NotASuite(PathBuf),
}

impl std::fmt::Display for Rejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotASuite(path) => write!(
                f,
                "{} is not a .jar or .jad file",
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into()
                )
            ),
        }
    }
}

/// Picks the MIDlet suites among `paths`: `.jar` and `.jad` files, without
/// case. When a `.jad` and the `.jar` of the same name come together, only
/// the `.jad` is kept: it carries the descriptor and points at the JAR.
pub fn pick_suites(paths: &[PathBuf]) -> (Vec<PathBuf>, Vec<Rejected>) {
    let extension = |p: &Path| {
        p.extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
    };
    let jads: Vec<PathBuf> = paths
        .iter()
        .filter(|p| extension(p).as_deref() == Some("jad"))
        .map(|p| p.with_extension(""))
        .collect();
    let mut suites = Vec::new();
    let mut rejected = Vec::new();
    for path in paths {
        match extension(path).as_deref() {
            Some("jad") => suites.push(path.clone()),
            Some("jar") => {
                if !jads.contains(&path.with_extension("")) {
                    suites.push(path.clone());
                }
            }
            _ => rejected.push(Rejected::NotASuite(path.clone())),
        }
    }
    (suites, rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(name: &str) -> SuiteInfo {
        SuiteInfo {
            name: name.into(),
            vendor: " Acme ".into(),
            version: "1.2".into(),
            ..SuiteInfo::default()
        }
    }

    #[test]
    fn add_assigns_new_ids_and_reads_the_suite() {
        let mut lib = Library::default();
        let a = lib.add("a.jar".into(), &info("Alpha"));
        let b = lib.add("b.jar".into(), &info(""));
        assert_ne!(a, b);
        let alpha = lib.get(a).expect("added");
        assert_eq!(alpha.name, "Alpha");
        assert_eq!(alpha.vendor, "Acme");
        assert_eq!(
            lib.get(b).expect("added").name,
            "b",
            "falls back to the file name"
        );
    }

    #[test]
    fn adding_twice_keeps_one_entry() {
        let mut lib = Library::default();
        let a = lib.add("games/a.jar".into(), &info("A"));
        assert_eq!(lib.add("games/a.jar".into(), &info("A")), a);
        assert_eq!(lib.entries().len(), 1);
    }

    #[test]
    fn ids_are_never_reused() {
        let mut lib = Library::default();
        let a = lib.add("a.jar".into(), &info("A"));
        let b = lib.add("b.jar".into(), &info("B"));
        lib.remove(b).expect("present");
        let c = lib.add("c.jar".into(), &info("C"));
        assert!(c != a && c != b, "{c:?} reuses a removed id");
        assert!(lib.remove(b).is_none());
    }

    #[test]
    fn the_largest_id_in_a_file_does_not_overflow() {
        let mut lib: Library = serde_json::from_str(&format!(
            r#"{{"entries":[{{"id":{max},"path":"a.jar","name":"A"}}],"next_id":{max}}}"#,
            max = u64::MAX
        ))
        .expect("parses");
        let b = lib.add("b.jar".into(), &info("B"));
        let c = lib.add("c.jar".into(), &info("C"));
        let mut ids: Vec<u64> = lib.entries().iter().map(|e| e.id.0).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 3, "{b:?} and {c:?} must be new ids");
    }

    #[test]
    fn dedup_drops_repeated_ids_and_paths() {
        let entry = |id, path: &str| LibraryEntry {
            id: EntryId(id),
            path: path.into(),
            name: path.into(),
            vendor: String::new(),
            version: String::new(),
            settings: GameSettings::default(),
        };
        let mut lib = Library {
            entries: vec![
                entry(1, "a.jar"),
                entry(1, "b.jar"),
                entry(2, "a.jar"),
                entry(3, "c.jar"),
            ],
            next_id: 0,
        };
        lib.dedup();
        let ids: Vec<_> = lib.entries().iter().map(|e| e.id.0).collect();
        assert_eq!(ids, vec![1, 3]);
    }

    #[test]
    fn pick_suites_prefers_the_descriptor() {
        let paths: Vec<PathBuf> = [
            "x/Game.JAR",
            "x/Game.jad",
            "y/Other.jar",
            "notes.txt",
            "dir",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        let (suites, rejected) = pick_suites(&paths);
        assert_eq!(
            suites,
            vec![PathBuf::from("x/Game.jad"), PathBuf::from("y/Other.jar")]
        );
        assert_eq!(rejected.len(), 2);
        assert_eq!(
            rejected[0].to_string(),
            "notes.txt is not a .jar or .jad file"
        );
    }

    #[test]
    fn the_library_round_trips_through_json() {
        let mut lib = Library::default();
        let id = lib.add("a.jar".into(), &info("A"));
        lib.get_mut(id).expect("added").settings.network = true;
        let json = serde_json::to_string(&lib).expect("serializes");
        let back: Library = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, lib);
    }
}
