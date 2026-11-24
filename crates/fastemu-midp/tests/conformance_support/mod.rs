//! Helpers for the conformance tests: the stubs of the API FastEmu
//! offers, test MIDlets compiled against them, and JARs packed from them.
//!
//! [`stubs_jar`] builds the stub jar with the code of `cargo xtask stubs`
//! (included below from `tools/xtask/src/api`), so the MIDlets compile
//! against exactly the classes, members and constants the runtime
//! registers: a test that calls something FastEmu lacks fails to compile.
//! [`compile`] compiles one folder of `tests/conformance/` with it as the
//! boot class path, and [`Jar`] packs suites, stored or deflated, with any
//! manifest. Run them with `support::Running`.
//!
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

#![allow(dead_code, reason = "each test uses a different part")]

#[allow(
    dead_code,
    reason = "the tests use only the stub builder of the xtask's api module"
)]
#[path = "../../../../tools/xtask/src/api/mod.rs"]
mod api;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// The folder of the conformance MIDlets' sources.
fn sources_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/conformance")
}

/// Whether a missing JDK fails the tests instead of skipping them.
fn java_required() -> bool {
    std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() == Ok("1")
}

/// `None` after saying why the tests skip, or a panic when Java is
/// required.
fn skip<T>(why: &str) -> Option<T> {
    assert!(!java_required(), "Java is required but unusable: {why}");
    eprintln!("skipping: {why}");
    None
}

/// The stub jar of every class the runtime registers, built once per test
/// binary (and only compiled again when the stubs changed), or `None`
/// without a JDK.
pub fn stubs_jar() -> Option<&'static Path> {
    static JAR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let built = JAR.get_or_init(|| {
        if api::stubs::jdk_tool("javac").is_none() {
            return Err("no javac in JAVA_HOME or on the PATH".to_owned());
        }
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fastemu-stubs");
        let api_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/compat/api");
        let built = api::stubs::build(&out, &api_dir)
            .unwrap_or_else(|e| panic!("the stubs do not build: {e}"));
        Ok(built.jar)
    });
    match built {
        Ok(jar) => Some(jar),
        Err(why) => skip(why),
    }
}

/// The classes compiled from `tests/conformance/<area>` against the stubs,
/// once per test binary, or `None` without a JDK.
pub fn compile(area: &str) -> Option<PathBuf> {
    static DONE: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
    let jar = stubs_jar()?;
    let done = DONE.get_or_init(Mutex::default);
    let mut done = done
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(dir) = done.get(area) {
        return Some(dir.clone());
    }
    let javac = api::stubs::jdk_tool("javac")?;
    let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance-classes")
        .join(area);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("making the class folder");
    let mut sources = Vec::new();
    api::stubs::java_files(&sources_root().join(area), &mut sources);
    assert!(
        !sources.is_empty(),
        "no Java sources in tests/conformance/{area}"
    );
    let mut command = Command::new(javac);
    command
        .args(api::stubs::javac_options(jar))
        .arg("-d")
        .arg(&out)
        .args(&sources);
    if let Err(e) = api::stubs::run(&mut command, "javac") {
        panic!("tests/conformance/{area} does not compile against the stubs: {e}");
    }
    done.insert(area.to_owned(), out.clone());
    Some(out)
}

/// A scratch folder for one test, inside the target folder.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("making a scratch folder");
    dir
}

/// One entry of a [`Jar`].
struct Entry {
    name: Vec<u8>,
    data: Vec<u8>,
    deflate: bool,
}

/// A JAR (a ZIP file) built entry by entry.
#[derive(Default)]
pub struct Jar {
    entries: Vec<Entry>,
}

impl Jar {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a stored entry; the name is raw bytes, as some JAR tools
    /// wrote them.
    #[must_use]
    pub fn stored(mut self, name: impl AsRef<[u8]>, data: impl Into<Vec<u8>>) -> Self {
        self.entries.push(Entry {
            name: name.as_ref().to_vec(),
            data: data.into(),
            deflate: false,
        });
        self
    }

    /// Adds a deflated entry.
    #[must_use]
    pub fn deflated(mut self, name: impl AsRef<[u8]>, data: impl Into<Vec<u8>>) -> Self {
        self.entries.push(Entry {
            name: name.as_ref().to_vec(),
            data: data.into(),
            deflate: true,
        });
        self
    }

    /// Adds `META-INF/MANIFEST.MF`.
    #[must_use]
    pub fn manifest(self, text: impl Into<Vec<u8>>) -> Self {
        self.stored("META-INF/MANIFEST.MF", text)
    }

    /// Adds every class file under `dir`, deflated, named by its path.
    #[must_use]
    pub fn classes(mut self, dir: &Path) -> Self {
        let mut files = Vec::new();
        class_files(dir, &mut files);
        files.sort();
        for file in files {
            let name = file
                .strip_prefix(dir)
                .expect("a file under the class folder")
                .to_string_lossy()
                .replace('\\', "/");
            let data = std::fs::read(&file).expect("reading a class file");
            self = self.deflated(name, data);
        }
        self
    }

    /// The ZIP bytes.
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for entry in &self.entries {
            let crc = crc32fast::hash(&entry.data);
            let (method, body) = if entry.deflate {
                (8u16, miniz_oxide::deflate::compress_to_vec(&entry.data, 6))
            } else {
                (0, entry.data.clone())
            };
            let offset = out.len() as u32;
            let mut header = Vec::new();
            header.extend_from_slice(&20u16.to_le_bytes());
            header.extend_from_slice(&0u16.to_le_bytes());
            header.extend_from_slice(&method.to_le_bytes());
            header.extend_from_slice(&[0; 4]);
            header.extend_from_slice(&crc.to_le_bytes());
            header.extend_from_slice(&(body.len() as u32).to_le_bytes());
            header.extend_from_slice(&(entry.data.len() as u32).to_le_bytes());
            header.extend_from_slice(&(entry.name.len() as u16).to_le_bytes());
            header.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&header);
            out.extend_from_slice(&entry.name);
            out.extend_from_slice(&body);
            directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&header);
            // Comment length, disk, internal and external attributes.
            directory.extend_from_slice(&[0; 10]);
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(&entry.name);
        }
        let start = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// Writes the JAR to `path` and returns the path.
    pub fn write(&self, path: &Path) -> PathBuf {
        std::fs::write(path, self.bytes()).expect("writing the JAR");
        path.to_path_buf()
    }
}

fn class_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reading the class folder") {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            class_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "class") {
            out.push(path);
        }
    }
}

/// A manifest with the three required attributes, `MIDlet-1` naming
/// `class`, and `extra` lines after them (each ending in CR LF).
pub fn manifest(class: &str, extra: &str) -> String {
    format!(
        "Manifest-Version: 1.0\r\nMIDlet-Name: Conformance\r\nMIDlet-Vendor: FastEmu\r\n\
         MIDlet-Version: 1.0\r\nMIDlet-1: Test, , {class}\r\n{extra}"
    )
}

/// The lines of `log` that start with `prefix`, without it.
pub fn tagged<'a>(log: &'a [String], prefix: &str) -> Vec<&'a str> {
    log.iter().filter_map(|l| l.strip_prefix(prefix)).collect()
}

/// Fails with both lists side by side when `got` is not `want`.
pub fn assert_lines(got: &[&str], want: &[&str], log: &[String]) {
    if got == want {
        return;
    }
    let mut report = String::new();
    for i in 0..got.len().max(want.len()) {
        let g = got.get(i).copied().unwrap_or("<nothing>");
        let w = want.get(i).copied().unwrap_or("<nothing>");
        let mark = if g == w { "  " } else { "! " };
        let _ = writeln!(report, "{mark}{w:<50} | {g}");
    }
    panic!(
        "the lines differ (expected | got)\n{report}\n--- the whole log ---\n{}",
        log.join("\n")
    );
}
