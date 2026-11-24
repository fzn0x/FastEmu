//! `fastemu run` end to end: the binary runs small MIDlets compiled from
//! `tests/java` with the installed JDK.
//!
//! The probe classes compile against the stubs in `tests/java/stubs` with
//! `javac --release 8`; only the probe classes go into the JAR, so the
//! runtime's own classes serve the stubs' names. Without a JDK the tests
//! print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// A scratch folder for one test, inside the target folder.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("runner-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("making a scratch folder");
    dir
}

/// The JDK tool `name`, from `JAVA_HOME` first, then the `PATH`.
fn tool(name: &str) -> Option<PathBuf> {
    let exe = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let candidate = Path::new(&home).join("bin").join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let works = Command::new(name)
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success());
    works.then(|| PathBuf::from(name))
}

fn java_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reading the Java sources") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            java_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

/// A ZIP of stored entries. FastEmu ignores CRCs, so they stay zero.
fn zip(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in entries {
        let offset = u32::try_from(out.len()).expect("a small JAR");
        let size = u32::try_from(data.len()).expect("a small entry");
        let name_len = u16::try_from(name.len()).expect("a short name");
        let mut header = vec![20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&size.to_le_bytes());
        header.extend_from_slice(&name_len.to_le_bytes());
        header.extend_from_slice(&[0, 0]);
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&header);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        directory.extend_from_slice(b"PK\x01\x02\x14\x00");
        directory.extend_from_slice(&header);
        directory.extend_from_slice(&[0; 10]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }
    let start = u32::try_from(out.len()).expect("a small JAR");
    let count = u16::try_from(entries.len()).expect("few entries");
    let directory_len = u32::try_from(directory.len()).expect("a small directory");
    out.extend_from_slice(&directory);
    out.extend_from_slice(b"PK\x05\x06\x00\x00\x00\x00");
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&directory_len.to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&[0, 0]);
    out
}

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Probe\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: Probe, , probe.Probe\r\n";

/// Compiles the probe and packs its JAR once.
fn build() -> Result<PathBuf, String> {
    let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
    let dir = scratch("build");
    let classes = dir.join("classes");
    let java = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/java");
    let mut sources = Vec::new();
    java_sources(&java, &mut sources);
    sources.sort();
    let output = Command::new(&javac)
        .args([
            "--release",
            "8",
            "-g",
            "-encoding",
            "UTF-8",
            "-nowarn",
            "-d",
        ])
        .arg(&classes)
        .args(&sources)
        .output()
        .map_err(|e| format!("running javac: {e}"))?;
    assert!(
        output.status.success(),
        "javac failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let mut entries = vec![("META-INF/MANIFEST.MF".to_owned(), MANIFEST.into())];
    let probe = classes.join("probe");
    let mut names: Vec<_> = std::fs::read_dir(&probe)
        .expect("compiled classes")
        .map(|e| e.expect("an entry").file_name())
        .collect();
    names.sort();
    for name in names {
        let data = std::fs::read(probe.join(&name)).expect("a class file");
        entries.push((format!("probe/{}", name.to_string_lossy()), data));
    }
    let jar = dir.join("probe.jar");
    std::fs::write(&jar, zip(&entries)).expect("writing the JAR");
    Ok(jar)
}

/// The probe's JAR, or `None` (after saying why) when no JDK is installed
/// and Java is not required.
fn probe_jar() -> Option<&'static Path> {
    static JAR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    match JAR.get_or_init(build) {
        Ok(jar) => Some(jar),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "Java is required but unusable: {why}"
            );
            eprintln!("skipping: {why}");
            None
        }
    }
}

/// What a run printed and how it ended.
struct Output {
    code: Option<i32>,
    out: String,
    err: String,
}

/// Runs `fastemu run` with `args` after the probe's JAR.
fn run(jar: &Path, args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_fastemu"))
        .arg("run")
        .arg(jar)
        .args(args)
        .output()
        .expect("running fastemu");
    Output {
        code: output.status.code(),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn path_arg(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

#[test]
fn record_stores_survive_between_runs_in_a_data_folder() {
    let Some(jar) = probe_jar() else {
        return;
    };
    let data = scratch("data");
    for runs in 0..2 {
        let run = run(jar, &["--frames", "5", "--data", path_arg(&data)]);
        assert_eq!(run.code, Some(0), "{}{}", run.out, run.err);
        assert!(
            run.out.contains(&format!("runs before {runs}")),
            "{}",
            run.out
        );
    }
    assert!(data.join("rms").is_dir(), "the stores stay in the folder");
    let fresh = run(jar, &["--frames", "5"]);
    assert!(fresh.out.contains("runs before 0"), "{}", fresh.out);
}
