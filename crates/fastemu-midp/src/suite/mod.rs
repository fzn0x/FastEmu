//! MIDlet suites: opening a `.jar` or a `.jad`, reading what they say
//! about themselves, and serving their classes and resources.
//!
//! [`Suite::open`] takes either file. For a `.jad` it finds the JAR from
//! `MIDlet-Jar-URL` (as a relative path, then its last segment, always
//! inside the JAD's folder) or else the JAD's own name with `.jar`; for a
//! `.jar` it also reads a `.jad` of the same name beside it. JAD
//! attributes come before the manifest's, so the first match of a lookup
//! gives the JAD's value, as MIDP says for untrusted suites.
//!
//! A JAD is hostile input: its `MIDlet-Jar-URL` never makes the host
//! touch a file outside the JAD's folder or a network share, and a JAR
//! over [`MAX_JAR_BYTES`] or a JAD over [`MAX_JAD_BYTES`] is refused
//! before it is read. So are the JAR and its manifest: the manifest and
//! the JAD are read to [`attributes::MAX_ATTRIBUTE_BYTES`] and
//! [`attributes::MAX_ATTRIBUTES`] attributes, and a suite that lists no
//! MIDlet is searched for one within a budget of classes and inflated
//! bytes (see `find_midlet_classes`).

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

/// The largest JAR [`Suite::open`] reads. The biggest JAR of the test
/// games is 1.4 MB and phones held a few MB. It equals the largest entry
/// the ZIP reader inflates.
pub const MAX_JAR_BYTES: u64 = 64 * 1024 * 1024;

/// The largest JAD [`Suite::open`] reads. Real ones are under 1 KB.
pub const MAX_JAD_BYTES: u64 = 1024 * 1024;

/// The longest file or folder name a JAD may use in its JAR path, as most
/// file systems allow.
const MAX_NAME_BYTES: usize = 255;

/// The most characters of a suite's name, vendor, version and MIDlet names
/// that [`SuiteInfo`] holds for display. The longest of them in the test
/// folder has 40. The attributes in [`SuiteInfo::properties`] stay whole.
pub const MAX_DISPLAY_CHARS: usize = 100;

/// The most characters of a suite's description that [`SuiteInfo`] holds.
/// The longest description in the test folder has 129.
pub const MAX_DESCRIPTION_CHARS: usize = 500;

/// The longest suite icon read, inflated. Icons are a few KB.
const MAX_ICON_BYTES: usize = 1024 * 1024;

/// What a search for MIDlet classes may spend: how many class files it
/// opens, how large one may be, and how many bytes all of them inflate to
/// in all. Entries that share one compressed stream, which a ZIP file
/// allows, are charged for each time they are read, so the search costs
/// the same however the entries overlap.
struct ScanLimits {
    classes: usize,
    class_bytes: usize,
    total_bytes: usize,
    /// The bytes the entries it reads hold in the archive, counted as the
    /// entries say, whether the read gives bytes or not: a stream of empty
    /// blocks inflates to nothing and takes as long to read as any.
    input_bytes: usize,
}

/// The limits [`find_midlet_classes`] runs under. The largest JAR of the
/// test folder, a desktop library, has 1046 classes of 4 MB in all.
const SCAN: ScanLimits = ScanLimits {
    classes: 8192,
    class_bytes: source::MAX_CLASS_BYTES,
    total_bytes: 32 * 1024 * 1024,
    input_bytes: 32 * 1024 * 1024,
};

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
        let mut notes = Vec::new();
        let (jar_path, jad) = if is_jad(path) {
            let jad = read_capped(path, MAX_JAD_BYTES)?;
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
            let jad = if beside != path && beside.is_file() {
                match read_capped(&beside, MAX_JAD_BYTES) {
                    Ok(bytes) => Some((beside, attributes::parse(&bytes))),
                    Err(LaunchError::InvalidSuite(why)) => {
                        notes.push(format!("{why}; its attributes are not used"));
                        None
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            (path.to_path_buf(), jad)
        };
        let bytes = read_capped(&jar_path, MAX_JAR_BYTES)?;
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
        let (info, read_notes) = read_info(&jar, jad_attributes, jar_bytes, &fallback_name);
        notes.extend(read_notes);
        Ok(Self {
            info,
            jar: Arc::new(jar),
            jar_path,
            jad_path,
            notes,
        })
    }
}

/// Reads the whole of the regular file at `path`, which may hold at most
/// `max` bytes. A file that is not a regular file (a folder, a pipe, a
/// device) or is bigger gives [`LaunchError::InvalidSuite`] without being
/// read. The size is checked before the read, and the read stops one byte
/// past `max`, so a file that grows meanwhile cannot fill the memory.
fn read_capped(path: &Path, max: u64) -> Result<Vec<u8>, LaunchError> {
    use std::io::Read;

    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let not_regular = || LaunchError::InvalidSuite(format!("{name} is not a regular file"));
    let too_big = |bytes: u64| {
        LaunchError::InvalidSuite(format!(
            "{name} is {} MiB, more than the {} MiB FastEmu reads",
            mib(bytes),
            mib(max)
        ))
    };
    // Looked at before the open, which waits for a writer on a pipe.
    if !std::fs::metadata(path)?.is_file() {
        return Err(not_regular());
    }
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(not_regular());
    }
    if meta.len() > max {
        return Err(too_big(meta.len()));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.take(max.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max {
        return Err(too_big(bytes.len() as u64));
    }
    Ok(bytes)
}

/// `bytes` in MiB, rounded up to one decimal so that a file over its limit
/// never reads as equal to it in a message.
fn mib(bytes: u64) -> String {
    format!(
        "{:.1}",
        (bytes as f64 / (1024.0 * 1024.0) * 10.0).ceil() / 10.0
    )
}

/// The JAR a JAD names, by the rules in the module comment.
fn find_jar_of_jad(jad: &Path, attributes: &[(String, String)]) -> Option<PathBuf> {
    let dir = jad.parent().unwrap_or(Path::new("."));
    let mut candidates = attributes::lookup(attributes, "MIDlet-Jar-URL")
        .map(|url| jar_candidates(dir, url))
        .unwrap_or_default();
    candidates.push(jad.with_extension("jar"));
    candidates.push(jad.with_extension("JAR"));
    candidates.into_iter().find(|p| p.is_file())
}

/// The places in `dir`, the JAD's folder, that the `MIDlet-Jar-URL` value
/// `url` may name, the likeliest first. The value is only ever a name: each
/// place is the JAD's folder followed by plain file or folder names,
/// whatever `url` says, so it cannot leave the folder, start with a drive
/// or a network share, or reach a stream or a device. This works on the
/// text alone and touches no file: a guess with a part that is not a plain
/// name is dropped, and the caller asks the file system only about what is
/// left. The answer is the same on every system, but for device names,
/// which only Windows refuses.
fn jar_candidates(dir: &Path, url: &str) -> Vec<PathBuf> {
    let url = url.strip_prefix("file://").unwrap_or(url);
    let value = url.split(['?', '#']).next().unwrap_or_default();
    let parts: Vec<&str> = value
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect();
    let mut candidates = Vec::new();
    // A full URL names a download, not a path: only its last part is used.
    if !value.contains("://") && !parts.is_empty() && parts.iter().all(|p| is_plain_name(p)) {
        let mut path = dir.to_path_buf();
        path.extend(&parts);
        candidates.push(path);
    }
    if let Some(last) = parts.last().filter(|p| is_plain_name(p)) {
        let alone = dir.join(last);
        if !candidates.contains(&alone) {
            candidates.push(alone);
        }
    }
    candidates
}

/// Whether `name` is one file or folder name that is safe to put after the
/// JAD's folder on every system: no `..`, no `:` (a drive, or a stream of
/// a file on NTFS), none of the characters `< > " | * ?`, no separator or
/// control character, no dot or space at the end (Windows drops them, so
/// `.. ` would be `..`), at most 255 bytes, and on Windows no device name.
fn is_plain_name(name: &str) -> bool {
    let refused = |c: char| {
        c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '*' | '?' | '/' | '\\')
    };
    // A name that ends with a dot is refused, and `..` is one.
    if name.is_empty() || name.len() > MAX_NAME_BYTES || name.ends_with(['.', ' ']) {
        return false;
    }
    if name.chars().any(refused) {
        return false;
    }
    !(cfg!(windows) && is_device_name(name))
}

/// Whether Windows takes `name` for a device (`NUL`, `COM1`, `con.txt`)
/// and not for a file, with any extension and in any case.
fn is_device_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ');
    let upper = stem.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let mut chars = upper.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    matches!(prefix.as_str(), "COM" | "LPT")
        && matches!(
            chars.next(),
            Some('0'..='9' | '\u{b9}' | '\u{b2}' | '\u{b3}')
        )
        && chars.next().is_none()
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
        .and_then(|entry| jar.read_prefix(entry, attributes::MAX_ATTRIBUTE_BYTES).ok())
        .map(|bytes| attributes::parse(&bytes))
        .unwrap_or_default();
    let mut notes = Vec::new();
    for key in ["MIDlet-Name", "MIDlet-Vendor"] {
        if let (Some(a), Some(b)) = (
            attributes::lookup(&jad, key),
            attributes::lookup(&manifest, key),
        ) && a != b
        {
            let (a, b) = (shown(a, MAX_DISPLAY_CHARS), shown(b, MAX_DISPLAY_CHARS));
            notes.push(format!(
                "the JAD says {key} is {a:?} but the JAR says {b:?}; using the JAD"
            ));
        }
    }
    let mut properties = jad;
    properties.extend(manifest);
    // What the app shows is cut; `properties` keeps every value whole, as
    // `getAppProperty` gives them.
    let get = |key: &str| attributes::lookup(&properties, key);
    let name = get("MIDlet-Name").filter(|n| !n.is_empty()).map_or_else(
        || shown(fallback_name, MAX_DISPLAY_CHARS),
        |n| shown(n, MAX_DISPLAY_CHARS),
    );
    let (mut midlets, midlet_notes) = attributes::midlets(&properties, &name);
    notes.extend(midlet_notes);
    if midlets.is_empty() {
        midlets = find_midlet_classes(jar, &name);
        if !midlets.is_empty() {
            notes
                .push("the suite lists no MIDlet-<n>; using the MIDlet classes found in it".into());
        }
    }
    for midlet in &mut midlets {
        midlet.name = shown(&midlet.name, MAX_DISPLAY_CHARS);
    }
    let icon_path = get("MIDlet-Icon")
        .map(|p| p.trim_start_matches('/').to_owned())
        .filter(|p| !p.is_empty())
        .or_else(|| midlets.first().and_then(|m| m.icon_path.clone()));
    let icon = icon_path.and_then(|path| {
        let entry = jar.find_str(&source::normalize(&path))?;
        jar.read_limited(entry, MAX_ICON_BYTES).ok()
    });
    let vendor = get("MIDlet-Vendor").map_or_else(String::new, |v| shown(v, MAX_DISPLAY_CHARS));
    let version = get("MIDlet-Version").map_or_else(String::new, |v| shown(v, MAX_DISPLAY_CHARS));
    let description = get("MIDlet-Description")
        .filter(|d| !d.is_empty())
        .map(|d| shown(d, MAX_DESCRIPTION_CHARS));
    let info = SuiteInfo {
        name,
        vendor,
        version,
        description,
        icon,
        midlets,
        properties,
        jar_bytes,
    };
    (info, notes)
}

/// `text` cut to its first `max` characters.
fn shown(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((at, _)) => text[..at].to_owned(),
        None => text.to_owned(),
    }
}

/// The classes in `jar` that extend `MIDlet`, directly or through other
/// classes of the JAR, for suites whose descriptor names none. The search
/// is bounded by [`SCAN`].
fn find_midlet_classes(jar: &ZipArchive, suite_name: &str) -> Vec<MidletInfo> {
    find_midlet_classes_within(jar, suite_name, &SCAN)
}

/// [`find_midlet_classes`] under `limits`. A class file is read up to
/// `limits.class_bytes`, and the search stops after `limits.classes` class
/// entries, when `limits.total_bytes` have been inflated, a read that went
/// over its limit counting the limit, or when the entries it read hold
/// `limits.input_bytes` in the archive. An entry that holds more than is
/// left of that is passed over. The suite may then list MIDlets that the
/// search did not reach, which no suite without a `MIDlet-1` has shown to
/// do.
fn find_midlet_classes_within(
    jar: &ZipArchive,
    suite_name: &str,
    limits: &ScanLimits,
) -> Vec<MidletInfo> {
    use fastemu_jvm::classfile::ClassFile;
    use std::collections::HashMap;
    use zip::ZipError;

    let mut supers: HashMap<String, (String, bool)> = HashMap::new();
    let mut examined = 0;
    let mut left = limits.total_bytes;
    let mut input_left = limits.input_bytes;
    for entry in jar.entries() {
        let name = &entry.name;
        if name.len() < 6 || !name[name.len() - 6..].eq_ignore_ascii_case(b".class") {
            continue;
        }
        if examined >= limits.classes || left == 0 || input_left == 0 {
            break;
        }
        // Entries may share the data they hold, so the sum of their sizes
        // is what the search reads, not the size of the archive.
        let held = entry.compressed_len();
        if held > input_left {
            continue;
        }
        input_left -= held;
        examined += 1;
        let cap = limits.class_bytes.min(left);
        let bytes = match jar.read_limited(entry, cap) {
            Ok(bytes) => {
                left -= bytes.len();
                bytes
            }
            Err(ZipError::TooLarge(_)) => {
                left -= cap;
                continue;
            }
            Err(_) => continue,
        };
        let Ok(class) = ClassFile::parse(&bytes) else {
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
    use super::zip::testing::{
        TestEntry, take_consumed, take_inflated, write, write_overlapping, write_overlapping_stream,
    };
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

    /// A class file `name` whose superclass is `super_name`, with no
    /// members.
    fn class_file(name: &str, super_name: &str) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 49, 0, 5];
        for text in [name, super_name] {
            b.push(1);
            b.extend((text.len() as u16).to_be_bytes());
            b.extend(text.as_bytes());
            b.extend([7, 0, if text == name { 1 } else { 3 }]);
        }
        b.extend([0x00, 0x21, 0, 2, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0]);
        b
    }

    fn open_jar(name: &str, bytes: &[u8]) -> Suite {
        let dir = temp_dir(name);
        let path = dir.join("game.jar");
        std::fs::write(&path, bytes).unwrap();
        let suite = Suite::open(&path).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
        suite
    }

    #[test]
    fn a_manifest_of_40000_keys_opens_with_the_first_4096() {
        use std::fmt::Write;
        let mut keys = String::new();
        for i in 0..40_000 {
            write!(keys, "K{i:07}: v\r\n").unwrap();
        }
        let manifest = format!("MIDlet-Name: Many\r\nMIDlet-1: Many, , com.a.Many\r\n{keys}");
        let suite = open_jar("keys", &jar("META-INF/MANIFEST.MF", manifest.as_bytes()));
        assert_eq!(suite.info.properties.len(), attributes::MAX_ATTRIBUTES);
        assert_eq!(suite.info.name, "Many");
        assert_eq!(suite.info.midlets[0].class_name, "com.a.Many");
    }

    #[test]
    fn a_manifest_that_inflates_to_much_is_read_to_a_mebibyte() {
        // 8 MiB of text from a few KB of JAR. Only the first mebibyte is
        // inflated, and the line that repeats gives one attribute.
        let mut manifest = b"MIDlet-1: A, , com.a.A\n".to_vec();
        manifest.resize(8 << 20, b'x');
        let bytes = write(
            &[
                TestEntry::deflated("META-INF/MANIFEST.MF", &manifest),
                TestEntry::stored("icon.png", b"PNG"),
            ],
            true,
        );
        take_inflated();
        let suite = open_jar("mfbomb", &bytes);
        assert_eq!(take_inflated(), attributes::MAX_ATTRIBUTE_BYTES as u64);
        assert_eq!(suite.info.midlets[0].class_name, "com.a.A");
    }

    #[test]
    fn a_name_vendor_version_and_description_are_cut_for_display_and_whole_as_properties() {
        let long = "N".repeat(200_000);
        let manifest = format!(
            "MIDlet-Name: {long}\nMIDlet-Vendor: {long}\nMIDlet-Version: {long}\n\
             MIDlet-Description: {long}\nMIDlet-1: {long}, , com.a.G\n"
        );
        let suite = open_jar("long", &jar("META-INF/MANIFEST.MF", manifest.as_bytes()));
        let info = &suite.info;
        assert_eq!(info.name.chars().count(), MAX_DISPLAY_CHARS);
        assert_eq!(info.vendor.chars().count(), MAX_DISPLAY_CHARS);
        assert_eq!(info.version.chars().count(), MAX_DISPLAY_CHARS);
        assert_eq!(
            info.description.as_deref().map(|d| d.chars().count()),
            Some(MAX_DESCRIPTION_CHARS)
        );
        assert_eq!(info.midlets[0].name.chars().count(), MAX_DISPLAY_CHARS);
        // What getAppProperty reads is whole.
        for key in ["MIDlet-Name", "MIDlet-Vendor", "MIDlet-Version"] {
            assert_eq!(info.property(key).map(str::len), Some(200_000), "{key}");
        }
    }

    #[test]
    fn a_cut_never_splits_a_character() {
        assert_eq!(shown("abc", 100), "abc");
        assert_eq!(shown("abc", 3), "abc");
        assert_eq!(shown("abcd", 3), "abc");
        assert_eq!(
            shown("\u{e9}\u{4e2d}\u{1f600}x", 3),
            "\u{e9}\u{4e2d}\u{1f600}"
        );
        assert_eq!(shown("", 0), "");
    }

    #[test]
    fn a_midlet_class_with_brackets_is_dropped_and_the_suite_lists_none() {
        let manifest = b"MIDlet-Name: S\nMIDlet-1: A, , [[I\n";
        let suite = open_jar("brackets", &jar("META-INF/MANIFEST.MF", manifest));
        assert!(suite.info.midlets.is_empty());
    }

    #[test]
    fn a_suite_that_lists_no_midlet_is_searched_for_one() {
        let jar_bytes = write(
            &[
                TestEntry::stored("a/Util.class", &class_file("a/Util", "java/lang/Object")),
                TestEntry::stored("a/Base.class", &class_file("a/Base", MIDLET_CLASS)),
                TestEntry::stored("a/Game.class", &class_file("a/Game", "a/Base")),
            ],
            true,
        );
        let suite = open_jar("search", &jar_bytes);
        let classes: Vec<_> = suite
            .info
            .midlets
            .iter()
            .map(|m| &m.class_name[..])
            .collect();
        assert_eq!(classes, ["a.Base", "a.Game"]);
    }

    #[test]
    fn entries_that_share_one_bomb_cost_the_search_for_midlets_one_budget() {
        // 5 MiB of zeros from 5 KB of deflate, listed as 100 class files.
        // Each read stops at the 4 MiB a class may have, and the search
        // stops at 32 MiB in all: 8 reads, not 100.
        let bomb = vec![0u8; 5 << 20];
        let names: Vec<String> = (0..100).map(|i| format!("a{i}.class")).collect();
        let bytes = write_overlapping(&names, &bomb);
        take_inflated();
        let suite = open_jar("overlap", &bytes);
        assert!(suite.info.midlets.is_empty());
        assert_eq!(take_inflated(), SCAN.total_bytes as u64);
    }

    #[test]
    fn the_search_for_midlets_stops_at_its_class_and_byte_limits() {
        let jar_bytes = write(
            &[
                TestEntry::stored("readme.txt", &[b'x'; 100]),
                TestEntry::stored("a/Pad.class", b"not a class"),
                TestEntry::stored("a/Game.class", &class_file("a/Game", MIDLET_CLASS)),
            ],
            true,
        );
        let archive = ZipArchive::new(jar_bytes).unwrap();
        let limits = |classes, class_bytes, total_bytes| ScanLimits {
            classes,
            class_bytes,
            total_bytes,
            input_bytes: usize::MAX,
        };
        let found = |limits: &ScanLimits| find_midlet_classes_within(&archive, "S", limits).len();
        assert_eq!(
            found(&limits(2, 1 << 20, 1 << 30)),
            1,
            "the class is second"
        );
        assert_eq!(
            found(&limits(1, 1 << 20, 1 << 30)),
            0,
            "a class limit of one"
        );
        // The pad takes 11 bytes of the 20, and the game class is longer
        // than the 9 left.
        assert_eq!(found(&limits(9, 1 << 20, 20)), 0, "a byte limit of 20");
        assert_eq!(found(&limits(9, 1 << 20, 1 << 20)), 1);
        // A class longer than its limit is skipped, and charged.
        assert_eq!(
            found(&limits(9, 20, 1 << 20)),
            0,
            "a class limit of 20 bytes"
        );
        assert_eq!(SCAN.classes, 8192);
        assert_eq!(SCAN.class_bytes, 4 << 20);
        assert_eq!(SCAN.total_bytes, 32 << 20);
        assert_eq!(SCAN.input_bytes, 32 << 20);
    }

    #[test]
    fn entries_that_share_a_stream_of_empty_blocks_cost_the_search_one_budget() {
        // 40000 empty stored blocks and an end: 200 KB that inflate to
        // nothing, listed as 100 class files. No byte is inflated, so only
        // what the entries hold bounds the search: 5 reads of the 200 KB
        // for 1 MiB, not 100.
        let mut stream = Vec::new();
        for _ in 0..40_000 {
            stream.extend([0, 0, 0, 0xff, 0xff]);
        }
        stream.extend([1, 0, 0, 0xff, 0xff]);
        let names: Vec<String> = (0..100).map(|i| format!("a{i}.class")).collect();
        let archive = ZipArchive::new(write_overlapping_stream(&names, &stream)).unwrap();
        assert_eq!(archive.read(&archive.entries()[0]).unwrap(), b"");
        take_inflated();
        take_consumed();
        let limits = ScanLimits {
            classes: 8192,
            class_bytes: 4 << 20,
            total_bytes: 32 << 20,
            input_bytes: 1 << 20,
        };
        assert!(find_midlet_classes_within(&archive, "S", &limits).is_empty());
        assert_eq!(take_inflated(), 0);
        assert_eq!(take_consumed(), 5 * stream.len() as u64);
    }

    #[test]
    fn an_icon_that_inflates_to_over_a_mebibyte_is_not_kept() {
        let manifest = b"MIDlet-1: A, big.png, com.a.A\n";
        let big = vec![0u8; MAX_ICON_BYTES + 1];
        let fits = vec![0u8; MAX_ICON_BYTES];
        let with = |icon: &[u8]| {
            write(
                &[
                    TestEntry::deflated("META-INF/MANIFEST.MF", manifest),
                    TestEntry::deflated("big.png", icon),
                ],
                true,
            )
        };
        assert_eq!(open_jar("iconbig", &with(&big)).info.icon, None);
        assert_eq!(open_jar("iconfits", &with(&fits)).info.icon, Some(fits));
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

    /// `dir` followed by the `/` separated names in `parts`.
    fn under(dir: &Path, parts: &str) -> PathBuf {
        parts
            .split('/')
            .fold(dir.to_path_buf(), |p, part| p.join(part))
    }

    #[test]
    fn a_jar_url_gives_only_plain_names_inside_the_jad_folder() {
        use std::path::Component;

        // A folder that does not exist: the rules read the text and touch
        // no file, so this runs the same on every system.
        let dir = Path::new("no-such-folder").join("inbox");
        let cases: &[(&str, &[&str])] = &[
            ("game.jar", &["game.jar"]),
            ("files/real.jar", &["files/real.jar", "real.jar"]),
            (r"a\b\game.jar", &["a/b/game.jar", "game.jar"]),
            ("./a//b/game.jar", &["a/b/game.jar", "game.jar"]),
            ("/files/real.jar", &["files/real.jar", "real.jar"]),
            ("game.jar?x=1#top", &["game.jar"]),
            ("http://example.com/dl/real.jar?x=1", &["real.jar"]),
            // Parent folders: the guess with a `..` is dropped.
            ("../victim.jar", &["victim.jar"]),
            (r"..\..\victim.jar", &["victim.jar"]),
            ("sub/../../victim.jar", &["victim.jar"]),
            ("..", &[]),
            // Absolute paths become paths inside the folder.
            ("/abs/victim.jar", &["abs/victim.jar", "victim.jar"]),
            ("file:///tmp/victim.jar", &["tmp/victim.jar", "victim.jar"]),
            // Windows: drives, network shares, device paths and streams.
            ("file:///C:/x/victim.jar", &["victim.jar"]),
            (r"C:\x\victim.jar", &["victim.jar"]),
            ("C:victim.jar", &[]),
            (
                r"\\127.0.0.1\share\victim.jar",
                &["127.0.0.1/share/victim.jar", "victim.jar"],
            ),
            (r"\\?\C:\x\victim.jar", &[]),
            (r"\\.\pipe\x", &["pipe/x", "x"]),
            ("victim.jar:evil", &[]),
            ("victim.jar::$DATA", &[]),
            ("victim.jar.", &[]),
            ("victim.jar ", &[]),
            // Characters Windows refuses, and control characters.
            ("a|b.jar", &[]),
            ("a*.jar", &[]),
            ("a\"b.jar", &[]),
            ("a<b>.jar", &[]),
            ("a\u{1}b.jar", &[]),
            ("a\nb.jar", &[]),
            // Nothing to name.
            ("", &[]),
            ("/", &[]),
            ("?x", &[]),
            ("file://", &[]),
        ];
        for (url, want) in cases {
            let got = jar_candidates(&dir, url);
            let want: Vec<PathBuf> = want.iter().map(|w| under(&dir, w)).collect();
            assert_eq!(got, want, "for {url:?}");
            for path in &got {
                let rest = path.strip_prefix(&dir).expect("inside the folder");
                assert!(
                    rest.components().all(|c| matches!(c, Component::Normal(_))),
                    "{path:?} for {url:?}"
                );
            }
        }
    }

    #[test]
    fn a_name_is_plain_up_to_255_bytes() {
        let dir = Path::new("inbox");
        let longest = format!("{}.jar", "a".repeat(251));
        assert_eq!(longest.len(), 255);
        assert_eq!(jar_candidates(dir, &longest), [dir.join(&longest)]);
        assert!(jar_candidates(dir, &format!("x{longest}")).is_empty());
        assert!(jar_candidates(dir, &format!("x/{longest}x")).is_empty());
    }

    #[test]
    fn device_names_are_known_by_their_stem() {
        for name in [
            "NUL",
            "nul",
            "Con.jar",
            "aux.tar.gz",
            "PRN",
            "COM1",
            "com9.x",
            "lpt0.txt",
            "COM\u{b9}",
            "NUL .jar",
            "CONIN$",
            "conout$.x",
        ] {
            assert!(is_device_name(name), "{name}");
        }
        for name in [
            "COM",
            "COM10",
            "COMX",
            "LPT",
            "console.jar",
            "game.jar",
            "nul-x",
            "a.nul",
        ] {
            assert!(!is_device_name(name), "{name}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn on_windows_a_device_name_is_not_a_plain_name() {
        let dir = Path::new("inbox");
        for url in ["NUL", "con.jar", "a/COM1.txt", r"..\PRN", "sub\\lpt3"] {
            assert!(jar_candidates(dir, url).is_empty(), "for {url:?}");
        }
        assert_eq!(
            jar_candidates(dir, "console.jar"),
            [dir.join("console.jar")]
        );
    }

    #[test]
    fn a_jad_cannot_open_a_jar_outside_its_folder() {
        let base = temp_dir("escape");
        let inbox = base.join("inbox");
        std::fs::create_dir_all(inbox.join("sub")).unwrap();
        let victim = base.join("victim.jar");
        std::fs::write(&victim, jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        let absolute = victim.to_string_lossy().into_owned();
        let forward = absolute.replace('\\', "/");
        let urls = [
            "../victim.jar".to_owned(),
            r"..\victim.jar".to_owned(),
            r"..\..\victim.jar".to_owned(),
            "sub/../../victim.jar".to_owned(),
            r"sub\..\..\victim.jar".to_owned(),
            absolute.clone(),
            forward.clone(),
            format!("file:///{}", forward.trim_start_matches('/')),
            r"\\127.0.0.1\share\victim.jar".to_owned(),
            r"\\?\C:\x\victim.jar".to_owned(),
            r"C:\x\victim.jar".to_owned(),
            "C:victim.jar".to_owned(),
            "/abs/victim.jar".to_owned(),
            "file:///tmp/victim.jar".to_owned(),
        ];
        let jad = inbox.join("hostile.jad");
        let mut opened = Vec::new();
        for url in &urls {
            std::fs::write(
                &jad,
                format!("MIDlet-Name: Hostile\r\nMIDlet-Jar-URL: {url}\r\n"),
            )
            .unwrap();
            match Suite::open(&jad) {
                Err(LaunchError::InvalidSuite(_)) => {}
                other => opened.push(format!("{url:?} gave {:?}", other.map(|s| s.jar_path))),
            }
        }
        assert!(opened.is_empty(), "opened: {opened:#?}");
        std::fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn a_jad_finds_its_jar_by_a_path_a_windows_path_or_a_url() {
        let dir = temp_dir("forms");
        std::fs::create_dir_all(dir.join("a/b")).unwrap();
        std::fs::write(
            dir.join("a/b/game.jar"),
            jar("META-INF/MANIFEST.MF", MANIFEST),
        )
        .unwrap();
        std::fs::write(dir.join("top.jar"), jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        let jad = dir.join("game.jad");
        for (url, found) in [
            ("a/b/game.jar", "a/b/game.jar"),
            (r"a\b\game.jar", "a/b/game.jar"),
            ("./a//b/game.jar", "a/b/game.jar"),
            ("/a/b/game.jar", "a/b/game.jar"),
            ("file:///a/b/game.jar", "a/b/game.jar"),
            ("http://example.com/dl/top.jar", "top.jar"),
            ("https://example.com/top.jar#x", "top.jar"),
            (r"..\old\top.jar", "top.jar"),
            ("top.jar", "top.jar"),
        ] {
            std::fs::write(&jad, format!("MIDlet-Jar-URL: {url}\n")).unwrap();
            let suite = Suite::open(&jad).unwrap_or_else(|e| panic!("{url:?}: {e}"));
            assert_eq!(suite.jar_path, under(&dir, found), "for {url:?}");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn read_capped_takes_an_exact_fit_and_refuses_the_rest() {
        let dir = temp_dir("capped");
        let file = dir.join("two.bin");
        std::fs::write(&file, vec![7u8; 2048]).unwrap();
        let Err(LaunchError::InvalidSuite(why)) = read_capped(&file, 1024) else {
            panic!("a file over the cap is refused");
        };
        assert!(why.contains("two.bin") && why.contains("MiB"), "{why}");
        assert_eq!(read_capped(&file, 2048).unwrap(), vec![7u8; 2048]);
        assert_eq!(read_capped(&file, 4096).unwrap().len(), 2048);
        std::fs::write(&file, b"").unwrap();
        assert!(read_capped(&file, 0).unwrap().is_empty());
        // A folder is not a file, and a missing file is an I/O error.
        let Err(LaunchError::InvalidSuite(why)) = read_capped(&dir, 1024) else {
            panic!("a folder is refused");
        };
        assert!(why.contains("not a regular file"), "{why}");
        assert!(matches!(
            read_capped(&dir.join("none"), 1024),
            Err(LaunchError::Io(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_size_in_a_message_never_equals_the_limit_it_broke() {
        assert_eq!(mib(MAX_JAR_BYTES), "64.0");
        assert_eq!(mib(MAX_JAR_BYTES + 1), "64.1");
        assert_eq!(mib(MAX_JAD_BYTES), "1.0");
        assert_eq!(mib(MAX_JAD_BYTES + 1), "1.1");
    }

    /// Makes a file of `len` zero bytes without writing them.
    fn zeros(path: &Path, len: u64) {
        std::fs::File::create(path).unwrap().set_len(len).unwrap();
    }

    #[test]
    fn a_jar_over_64_mib_is_refused_without_reading_it() {
        let dir = temp_dir("bigjar");
        let big = dir.join("big.jar");
        zeros(&big, MAX_JAR_BYTES + 1);
        let Err(LaunchError::InvalidSuite(why)) = Suite::open(&big) else {
            panic!("a JAR over the cap is refused");
        };
        assert!(
            why.contains("big.jar") && why.contains("64.1 MiB") && why.contains("64.0 MiB"),
            "{why}"
        );
        // The same through a JAD that names it.
        let jad = dir.join("game.jad");
        std::fs::write(&jad, "MIDlet-Jar-URL: big.jar\n").unwrap();
        let Err(LaunchError::InvalidSuite(why)) = Suite::open(&jad) else {
            panic!("a JAD naming it is refused too");
        };
        assert!(why.contains("big.jar") && why.contains("MiB"), "{why}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_jad_over_1_mib_is_refused_and_one_beside_a_jar_is_skipped() {
        let dir = temp_dir("bigjad");
        let jad = dir.join("game.jad");
        zeros(&jad, MAX_JAD_BYTES + 1);
        let Err(LaunchError::InvalidSuite(why)) = Suite::open(&jad) else {
            panic!("a JAD over the cap is refused");
        };
        assert!(
            why.contains("game.jad") && why.contains("1.1 MiB") && why.contains("1.0 MiB"),
            "{why}"
        );
        // Beside a JAR it is left out, with a note, and the JAR still opens.
        std::fs::write(dir.join("game.jar"), jar("META-INF/MANIFEST.MF", MANIFEST)).unwrap();
        let suite = Suite::open(&dir.join("game.jar")).unwrap();
        assert!(suite.jad_path.is_none());
        assert_eq!(suite.info.name, "Snake");
        assert!(
            suite.notes.iter().any(|n| n.contains("game.jad")),
            "{:?}",
            suite.notes
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_folder_named_like_a_jar_is_refused() {
        let dir = temp_dir("folder");
        std::fs::create_dir_all(dir.join("game.jar")).unwrap();
        let Err(LaunchError::InvalidSuite(why)) = Suite::open(&dir.join("game.jar")) else {
            panic!("a folder is not a JAR");
        };
        assert!(why.contains("not a regular file"), "{why}");
        // A JAD that names it finds no JAR at all.
        std::fs::write(dir.join("game.jad"), "MIDlet-Jar-URL: game.jar\n").unwrap();
        assert!(matches!(
            Suite::open(&dir.join("game.jad")),
            Err(LaunchError::InvalidSuite(_))
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
