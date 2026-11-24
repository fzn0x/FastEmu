//! MIDlet suites: opening a `.jar` or a `.jad`, reading what they say
//! about themselves, and serving their classes and resources.
//!
//! [`Suite::open`] takes either file. For a `.jad` it finds the JAR from
//! `MIDlet-Jar-URL` (as a relative path, then its last segment) or else
//! the JAD's own name with `.jar`; for a `.jar` it also reads a `.jad` of
//! the same name beside it. JAD attributes come before the manifest's, so
//! the first match of a lookup gives the JAD's value, as MIDP says for
//! untrusted suites.

pub mod attributes;
pub mod source;
pub mod zip;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fastemu_core::{LaunchError, MidletInfo, SuiteInfo};

pub use source::JarSource;
pub use zip::ZipArchive;

/// The binary name every MIDlet class extends.
const MIDLET_CLASS: &str = "javax/microedition/midlet/MIDlet";

/// An opened suite: its attributes and its JAR.
#[derive(Clone, Debug)]
pub struct Suite {
    /// What the suite says about itself.
    pub info: SuiteInfo,
    /// The JAR, indexed.
    pub jar: Arc<ZipArchive>,
    /// Where the JAR is.
    pub jar_path: PathBuf,
    /// Where the descriptor is, when there is one.
    pub jad_path: Option<PathBuf>,
    /// Things worth telling the user that did not stop the suite from
    /// opening: a missing `MIDlet-1`, a JAD that disagrees with its JAR.
    pub notes: Vec<String>,
}

fn is_jad(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("jad"))
}

impl Suite {
    /// Opens the suite at `path`, a `.jar` or a `.jad`.
    pub fn open(path: &Path) -> Result<Self, LaunchError> {
        let (jar_path, jad) = if is_jad(path) {
            let jad = std::fs::read(path)?;
            let attributes = attributes::parse(&jad);
            let jar = find_jar_of_jad(path, &attributes).ok_or_else(|| {
                LaunchError::InvalidSuite(format!(
                    "the JAR of {} was not found next to it",
                    path.display()
                ))
            })?;
            (jar, Some((path.to_path_buf(), attributes)))
        } else {
            let beside = path.with_extension("jad");
            let jad = (beside != path && beside.is_file())
                .then(|| std::fs::read(&beside).ok())
                .flatten()
                .map(|bytes| (beside, attributes::parse(&bytes)));
            (path.to_path_buf(), jad)
        };
        let bytes = std::fs::read(&jar_path)?;
        let jar_bytes = bytes.len() as u64;
        let jar = ZipArchive::new(bytes)
            .map_err(|e| LaunchError::InvalidSuite(format!("{}: {e}", jar_path.display())))?;
        let (jad_path, jad_attributes) = match jad {
            Some((p, a)) => (Some(p), a),
            None => (None, Vec::new()),
        };
        let fallback_name = jar_path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let (info, notes) = read_info(&jar, jad_attributes, jar_bytes, &fallback_name);
        Ok(Self {
            info,
            jar: Arc::new(jar),
            jar_path,
            jad_path,
            notes,
        })
    }
}

/// The JAR a JAD names, by the rules in the module comment.
fn find_jar_of_jad(jad: &Path, attributes: &[(String, String)]) -> Option<PathBuf> {
    let dir = jad.parent().unwrap_or(Path::new("."));
    let mut candidates = Vec::new();
    if let Some(url) = attributes::lookup(attributes, "MIDlet-Jar-URL") {
        let local = url
            .strip_prefix("file://")
            .unwrap_or(url)
            .split(['?', '#'])
            .next()
            .unwrap_or_default();
        if !local.contains("://") && !local.is_empty() {
            candidates.push(dir.join(local.trim_start_matches('/')));
        }
        if let Some(last) = local.rsplit(['/', '\\']).next().filter(|s| !s.is_empty()) {
            candidates.push(dir.join(last));
        }
    }
    candidates.push(jad.with_extension("jar"));
    candidates.push(jad.with_extension("JAR"));
    candidates.into_iter().find(|p| p.is_file())
}

/// Builds the suite's description from its JAD and manifest attributes.
fn read_info(
    jar: &ZipArchive,
    jad: Vec<(String, String)>,
    jar_bytes: u64,
    fallback_name: &str,
) -> (SuiteInfo, Vec<String>) {
    let manifest = ["META-INF/MANIFEST.MF", "META-INF/MANIFEST.FM"]
        .into_iter()
        .find_map(|name| jar.find_str(name))
        .and_then(|entry| jar.read(entry).ok())
        .map(|bytes| attributes::parse(&bytes))
        .unwrap_or_default();
    let mut notes = Vec::new();
    for key in ["MIDlet-Name", "MIDlet-Vendor"] {
        if let (Some(a), Some(b)) = (
            attributes::lookup(&jad, key),
            attributes::lookup(&manifest, key),
        ) && a != b
        {
            notes.push(format!(
                "the JAD says {key} is {a:?} but the JAR says {b:?}; using the JAD"
            ));
        }
    }
    let mut properties = jad;
    properties.extend(manifest);
    let get = |key: &str| attributes::lookup(&properties, key).map(str::to_owned);
    let name = get("MIDlet-Name")
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| fallback_name.to_owned());
    let (mut midlets, midlet_notes) = attributes::midlets(&properties, &name);
    notes.extend(midlet_notes);
    if midlets.is_empty() {
        midlets = find_midlet_classes(jar, &name);
        if !midlets.is_empty() {
            notes
                .push("the suite lists no MIDlet-<n>; using the MIDlet classes found in it".into());
        }
    }
    let icon_path = get("MIDlet-Icon")
        .map(|p| p.trim_start_matches('/').to_owned())
        .filter(|p| !p.is_empty())
        .or_else(|| midlets.first().and_then(|m| m.icon_path.clone()));
    let icon = icon_path.and_then(|path| {
        let entry = jar.find_str(&source::normalize(&path))?;
        jar.read(entry).ok()
    });
    let info = SuiteInfo {
        name,
        vendor: get("MIDlet-Vendor").unwrap_or_default(),
        version: get("MIDlet-Version").unwrap_or_default(),
        description: get("MIDlet-Description").filter(|d| !d.is_empty()),
        icon,
        midlets,
        properties,
        jar_bytes,
    };
    (info, notes)
}

/// The classes in `jar` that extend `MIDlet`, directly or through other
/// classes of the JAR, for suites whose descriptor names none.
fn find_midlet_classes(jar: &ZipArchive, suite_name: &str) -> Vec<MidletInfo> {
    use fastemu_jvm::classfile::ClassFile;
    use std::collections::HashMap;

    let mut supers: HashMap<String, (String, bool)> = HashMap::new();
    for entry in jar.entries() {
        let name = entry.display_name();
        if !name.to_ascii_lowercase().ends_with(".class") {
            continue;
        }
        let Some(class) = jar.read(entry).ok().and_then(|b| ClassFile::parse(&b).ok()) else {
            continue;
        };
        if let Some(parent) = &class.super_name {
            let is_abstract = class.access & fastemu_jvm::classfile::access::ABSTRACT != 0;
            supers.insert(class.name.to_string(), (parent.to_string(), is_abstract));
        }
    }
    let mut found: Vec<MidletInfo> = supers
        .iter()
        .filter(|(_, (_, is_abstract))| !is_abstract)
        .filter(|(class, _)| {
            let mut current = class.as_str();
            for _ in 0..64 {
                match supers.get(current) {
                    Some((parent, _)) if parent == MIDLET_CLASS => return true,
                    Some((parent, _)) => current = parent,
                    None => return false,
                }
            }
            false
        })
        .map(|(class, _)| MidletInfo {
            name: suite_name.to_owned(),
            icon_path: None,
            class_name: class.replace('/', "."),
        })
        .collect();
    found.sort_by(|a, b| a.class_name.cmp(&b.class_name));
    found
}

#[cfg(test)]
mod tests {
    use super::zip::testing::{TestEntry, write};
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fastemu-suite-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const MANIFEST: &[u8] = b"Manifest-Version: 1.0\r\nMIDlet-Name: Snake\r\nMIDlet-Vendor: Acme\r\nMIDlet-Version: 1.2\r\nMIDlet-1: Snake, /icon.png, com.acme.Snake\r\nGame-Speed: 3\r\n";

    fn jar(manifest_name: &str, manifest: &[u8]) -> Vec<u8> {
        write(
            &[
                TestEntry::deflated(manifest_name, manifest),
                TestEntry::stored("icon.png", b"PNG"),
            ],
            true,
        )
    }

    #[test]
    fn opens_a_jar_and_reads_its_manifest() {
        let dir = temp_dir("jar");
        let path = dir.join("snake.jar");
        std::fs::write(&path, jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        let suite = Suite::open(&path).unwrap();
        let info = &suite.info;
        assert_eq!(
            (
                info.name.as_str(),
                info.vendor.as_str(),
                info.version.as_str()
            ),
            ("Snake", "Acme", "1.2")
        );
        assert_eq!(info.midlets[0].class_name, "com.acme.Snake");
        assert_eq!(info.icon.as_deref(), Some(&b"PNG"[..]));
        assert_eq!(info.property("Game-Speed"), Some("3"));
        assert_eq!(info.jar_bytes, std::fs::metadata(&path).unwrap().len());
        assert!(suite.jad_path.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn finds_the_manifest_without_regard_to_case() {
        let dir = temp_dir("case");
        let path = dir.join("game.jar");
        std::fs::write(&path, jar("meta-inf/manifest.mf", MANIFEST)).unwrap();
        assert_eq!(Suite::open(&path).unwrap().info.name, "Snake");
        std::fs::write(&path, jar("META-INF/MANIFEST.FM", MANIFEST)).unwrap();
        assert_eq!(Suite::open(&path).unwrap().info.name, "Snake");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_jad_names_its_jar_and_wins_over_the_manifest() {
        let dir = temp_dir("jad");
        std::fs::create_dir_all(dir.join("files")).unwrap();
        std::fs::write(
            dir.join("files/real.jar"),
            jar("META-INF/MANIFEST.MF", MANIFEST),
        )
        .unwrap();
        let jad = dir.join("game.jad");
        std::fs::write(
            &jad,
            "MIDlet-Name: Snake Deluxe\nMIDlet-Jar-URL: files/real.jar\nMIDlet-Jar-Size: 1\nGame-Speed: 5\n",
        )
        .unwrap();
        let suite = Suite::open(&jad).unwrap();
        assert_eq!(suite.info.name, "Snake Deluxe");
        assert_eq!(suite.info.property("Game-Speed"), Some("5"));
        assert_eq!(suite.info.vendor, "Acme");
        assert_eq!(suite.jar_path, dir.join("files/real.jar"));
        assert_eq!(suite.notes.len(), 1, "the name mismatch is noted");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_jad_with_a_url_falls_back_to_the_last_segment_and_its_own_name() {
        let dir = temp_dir("jadurl");
        std::fs::write(dir.join("real.jar"), jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        let jad = dir.join("game.jad");
        std::fs::write(&jad, "MIDlet-Jar-URL: http://example.com/dl/real.jar?x=1\n").unwrap();
        assert_eq!(Suite::open(&jad).unwrap().jar_path, dir.join("real.jar"));
        std::fs::remove_file(dir.join("real.jar")).unwrap();
        std::fs::write(dir.join("game.jar"), jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        assert_eq!(Suite::open(&jad).unwrap().jar_path, dir.join("game.jar"));
        std::fs::remove_file(dir.join("game.jar")).unwrap();
        assert!(matches!(
            Suite::open(&jad),
            Err(LaunchError::InvalidSuite(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_jar_picks_up_a_jad_beside_it() {
        let dir = temp_dir("beside");
        let path = dir.join("snake.jar");
        std::fs::write(&path, jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        std::fs::write(dir.join("snake.jad"), "Game-Speed: 9\n").unwrap();
        let suite = Suite::open(&path).unwrap();
        assert_eq!(suite.info.property("Game-Speed"), Some("9"));
        assert_eq!(suite.jad_path, Some(dir.join("snake.jad")));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_jar_without_a_manifest_still_opens() {
        let dir = temp_dir("bare");
        let path = dir.join("bare.jar");
        std::fs::write(&path, write(&[TestEntry::stored("a.txt", b"x")], true)).unwrap();
        let suite = Suite::open(&path).unwrap();
        assert_eq!(suite.info.name, "bare");
        assert!(suite.info.midlets.is_empty());
        std::fs::write(&path, b"garbage").unwrap();
        assert!(matches!(
            Suite::open(&path),
            Err(LaunchError::InvalidSuite(_))
        ));
        assert!(matches!(
            Suite::open(&dir.join("none.jar")),
            Err(LaunchError::Io(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
