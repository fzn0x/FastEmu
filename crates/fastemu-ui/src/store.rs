//! Reading and writing the library and settings as JSON files in the app's
//! config folder.
//!
//! A file that cannot be parsed is set aside as `<name>.broken` rather than
//! overwritten, and the app starts with defaults, so a bad edit never loses
//! the user's data silently.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::library::Library;
use crate::settings::AppSettings;

const LIBRARY_FILE: &str = "library.json";
const SETTINGS_FILE: &str = "settings.json";

/// A value read from disk, with what went wrong if it had to fall back to
/// the default.
#[derive(Debug)]
pub struct Loaded<T> {
    pub value: T,
    /// A sentence for the user when the file existed but was not usable.
    pub problem: Option<String>,
}

/// The JSON files of one config folder.
#[derive(Clone, Debug)]
pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn load_library(&self) -> Loaded<Library> {
        let mut loaded: Loaded<Library> = self.load(LIBRARY_FILE, "an empty library");
        loaded.value.dedup();
        loaded.value.cut_strings();
        loaded
    }

    pub fn load_settings(&self) -> Loaded<AppSettings> {
        self.load(SETTINGS_FILE, "default settings")
    }

    pub fn save_library(&self, library: &Library) -> io::Result<()> {
        self.save(LIBRARY_FILE, library)
    }

    pub fn save_settings(&self, settings: &AppSettings) -> io::Result<()> {
        self.save(SETTINGS_FILE, settings)
    }

    fn load<T: DeserializeOwned + Default>(&self, name: &str, fallback: &str) -> Loaded<T> {
        let path = self.dir.join(name);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Loaded {
                    value: T::default(),
                    problem: None,
                };
            }
            Err(err) => {
                return Loaded {
                    value: T::default(),
                    problem: Some(format!("Could not read {name} ({err}). Using {fallback}.")),
                };
            }
        };
        match serde_json::from_str(&text) {
            Ok(value) => Loaded {
                value,
                problem: None,
            },
            Err(err) => {
                let broken = self.dir.join(format!("{name}.broken"));
                let kept = match fs::rename(&path, &broken) {
                    Ok(()) => format!("It was kept as {}.", broken.display()),
                    Err(_) => "It could not be set aside.".to_owned(),
                };
                Loaded {
                    value: T::default(),
                    problem: Some(format!(
                        "{name} is damaged ({err}). {kept} Using {fallback}."
                    )),
                }
            }
        }
    }

    /// Writes through a temporary file and a rename, so a crash mid-write
    /// leaves the previous file intact.
    fn save<T: Serialize>(&self, name: &str, value: &T) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        let path = self.dir.join(name);
        let temp = self.dir.join(format!("{name}.tmp"));
        fs::write(&temp, json)?;
        fs::rename(&temp, &path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::LayoutMode;
    use fastemu_core::SuiteInfo;

    #[test]
    fn missing_files_load_as_defaults_without_a_problem() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::new(dir.path().join("not-yet"));
        let library = store.load_library();
        assert!(library.problem.is_none());
        assert!(library.value.entries().is_empty());
        assert_eq!(store.load_settings().value, AppSettings::default());
    }

    #[test]
    fn saved_values_load_back() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::new(dir.path().join("config"));
        let mut library = Library::default();
        library.add(
            "snake.jar".into(),
            &SuiteInfo {
                name: "Snake".into(),
                ..SuiteInfo::default()
            },
        );
        let settings = AppSettings {
            layout: LayoutMode::Focus,
            ..AppSettings::default()
        };
        store.save_library(&library).expect("saves");
        store.save_settings(&settings).expect("saves");
        assert_eq!(store.load_library().value, library);
        assert_eq!(store.load_settings().value, settings);
        assert!(!dir.path().join("config/library.json.tmp").exists());
    }

    #[test]
    fn a_broken_file_is_kept_aside() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join("settings.json"), "{ not json").expect("writes");
        let store = Store::new(dir.path());
        let loaded = store.load_settings();
        assert_eq!(loaded.value, AppSettings::default());
        let problem = loaded.problem.expect("reports the damage");
        assert!(problem.contains("settings.json is damaged"), "{problem}");
        assert!(dir.path().join("settings.json.broken").exists());
        assert!(!dir.path().join("settings.json").exists());
    }

    #[test]
    fn a_library_file_with_long_text_loads_cut() {
        let dir = tempfile::tempdir().expect("temp dir");
        let long = "n".repeat(100_000);
        let json = format!(
            r#"{{"entries":[{{"id":1,"path":"a.jar","name":"{long}","vendor":"{long}","version":"{long}"}}]}}"#
        );
        fs::write(dir.path().join("library.json"), json).expect("writes");
        let loaded = Store::new(dir.path()).load_library();
        assert!(loaded.problem.is_none());
        let entry = &loaded.value.entries()[0];
        for text in [&entry.name, &entry.vendor, &entry.version] {
            assert_eq!(text.len(), 100);
        }
    }
}
