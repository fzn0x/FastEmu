//! Helpers for the Nokia and vendor API tests: the programs in
//! `tests/vendor_java` compile once per test binary with `javac --release
//! 8` against the stubs in `tests/vendor_stubs`, then either run their
//! `main` on a virtual machine with the classes of a chosen device profile
//! ([`run`], [`check`]), or go into a suite JAR whose MIDlets start through
//! the real launcher ([`suite`], with `support::Running`).
//!
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set.

#![allow(dead_code, reason = "each test binary uses a different part")]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use fastemu_core::{DeviceProfile, LogLevel, LogLine, builtin_profiles};
use fastemu_jvm::{DirSource, Vm, VmConfig};
use fastemu_midp::env::InstanceEnv;

/// What a program printed.
#[derive(Debug, Default)]
pub struct Output {
    pub out: Vec<String>,
    pub err: Vec<String>,
    pub info: Vec<String>,
}

/// The built-in profile with id `id`.
pub fn profile(id: &str) -> DeviceProfile {
    builtin_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap_or_else(|| panic!("no built-in profile {id}"))
}

/// The JDK tool `name`, from `JAVA_HOME` first, then the `PATH`.
fn tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
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

fn java_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("reading a test folder") {
            let path = entry.expect("a folder entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|x| x == "java") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn javac(javac: &Path, out: &Path, classpath: Option<&Path>, sources: &[PathBuf]) {
    let _ = std::fs::remove_dir_all(out);
    std::fs::create_dir_all(out).expect("making the class folder");
    let mut command = Command::new(javac);
    command.args([
        "--release",
        "8",
        "-g",
        "-encoding",
        "UTF-8",
        "-nowarn",
        "-d",
    ]);
    command.arg(out);
    if let Some(classpath) = classpath {
        command.arg("-cp").arg(classpath);
    }
    let output = command.args(sources).output().expect("running javac");
    assert!(
        output.status.success(),
        "javac failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Compiles the stubs and the programs once; `Err` says why there is no
/// JDK to do it. The programs' classes land under `v/`, beside the test
/// image `dot.png`.
fn compiled() -> &'static Result<PathBuf, String> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let javac_path = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        // One folder per test binary, so binaries never share one.
        let binary = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "vendor".to_owned());
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("vendor-java-{binary}"));
        let stubs = root.join("stubs");
        let classes = root.join("classes");
        javac(
            &javac_path,
            &stubs,
            None,
            &java_files(&tests.join("vendor_stubs")),
        );
        javac(
            &javac_path,
            &classes,
            Some(&stubs),
            &java_files(&tests.join("vendor_java")),
        );
        std::fs::write(classes.join("dot.png"), test_png()).expect("writing the test image");
        Ok(classes)
    })
}

/// The compiled programs, or `None` (with a note) when there is no JDK.
pub fn classes() -> Option<&'static Path> {
    match compiled() {
        Ok(classes) => Some(classes),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "FASTEMU_REQUIRE_JAVA=1 but {why}"
            );
            eprintln!("skipping: {why}");
            None
        }
    }
}

/// A 2 by 2 PNG: red, green, blue and half-transparent white.
pub fn test_png() -> Vec<u8> {
    let pixels: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 128],
    ];
    let mut raw = Vec::new();
    for row in pixels.chunks(2) {
        raw.push(0);
        for p in row {
            raw.extend_from_slice(p);
        }
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc32fast::hash(&body).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(b"IEND", &[]);
    png
}

/// Runs `v.<class>`'s `main` on a machine with the classes of `profile`
/// and returns what it printed, or `None` when there is no JDK. The
/// program ends by printing `done`.
pub fn run(class: &str, profile: &DeviceProfile) -> Option<Output> {
    let classes = classes()?;
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        allow_exit: false,
        ..VmConfig::default()
    };
    let vm = Vm::new(
        config,
        Arc::new(DirSource::new(classes)),
        fastemu_midp::classes(profile),
    );
    let data_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("vendor-data");
    let profile = profile.clone();
    vm.attach("setup", |ctx| {
        ctx.set_extension(InstanceEnv {
            profile,
            ..InstanceEnv::for_tests(data_dir)
        });
    });
    vm.run_main(&format!("v.{class}"), &[])
        .expect("starting main");

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let finished = {
            let lines = lines.lock().unwrap();
            lines
                .iter()
                .any(|l| l.level == LogLevel::Out && l.text == "done")
                || lines
                    .iter()
                    .any(|l| l.text.starts_with("Exception in thread \"main\""))
        };
        if finished || Instant::now() > deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    vm.stop();
    assert!(
        vm.wait_finished(Some(Duration::from_secs(10))),
        "{class}: threads did not stop"
    );
    vm.flush_output();
    let lines = lines.lock().unwrap();
    let pick = |level: LogLevel| {
        lines
            .iter()
            .filter(|l| l.level == level)
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
    };
    Some(Output {
        out: pick(LogLevel::Out),
        err: pick(LogLevel::Err),
        info: pick(LogLevel::Info),
    })
}

/// Checks that `v.<class>` printed `expected` on `profile` (one line per
/// line, `done` last), showing the error output when it did not.
pub fn check(class: &str, profile: &DeviceProfile, expected: &str) -> Option<Output> {
    let output = run(class, profile)?;
    let mut want: Vec<&str> = expected
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    want.push("done");
    let got: Vec<&str> = output.out.iter().map(String::as_str).collect();
    if got != want {
        let mut report = String::new();
        for i in 0..got.len().max(want.len()) {
            let g = got.get(i).copied().unwrap_or("<nothing>");
            let w = want.get(i).copied().unwrap_or("<nothing>");
            let mark = if g == w { "  " } else { "! " };
            let _ = writeln!(report, "{mark}{w:<50} | {g}");
        }
        panic!(
            "{class} on {}: output differs (expected | got)\n{report}\n--- stderr ---\n{}",
            profile.id,
            output.err.join("\n")
        );
    }
    Some(output)
}

/// Writes a suite JAR holding every compiled class under `v/` and a
/// manifest whose `MIDlet-1`, `MIDlet-2`... start `v.<name>` for each of
/// `midlets`, and returns its path, or `None` when there is no JDK. Each
/// call writes a JAR of its own (and so gets a data folder of its own), as
/// tests run in parallel.
pub fn suite(name: &str, midlets: &[&str]) -> Option<PathBuf> {
    static WRITTEN: AtomicUsize = AtomicUsize::new(0);
    let classes = classes()?;
    let mut manifest = String::from(
        "Manifest-Version: 1.0\r\nMIDlet-Name: Vendor tests\r\n\
         MIDlet-Vendor: FastEmu\r\nMIDlet-Version: 1.0\r\n",
    );
    for (i, midlet) in midlets.iter().enumerate() {
        let _ = write!(manifest, "MIDlet-{}: {midlet}, , v.{midlet}\r\n", i + 1);
    }
    let mut entries = vec![("META-INF/MANIFEST.MF".to_owned(), manifest.into_bytes())];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("v"))
        .expect("the compiled test classes")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let entry = format!("v/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((entry, std::fs::read(&file).expect("a class file")));
    }
    entries.push(("dot.png".to_owned(), test_png()));
    let binary = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("vendor-suites")
        .join(binary);
    std::fs::create_dir_all(&dir).expect("making the suite folder");
    let n = WRITTEN.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!("{name}-{n}.jar"));
    let _ = std::fs::remove_dir_all(path.with_extension("data"));
    std::fs::write(&path, crate::support::zip(&entries, true)).expect("writing the JAR");
    Some(path)
}
