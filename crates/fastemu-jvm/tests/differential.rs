//! Differential tests: each Java program in `tests/java/` runs on the
//! installed JDK and on FastEmu, and the two outputs must match.
//!
//! The programs use only CLDC APIs. They are compiled once with
//! `javac --release 8`; the JDK comes from `JAVA_HOME` or the `PATH`.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{DirSource, Vm, VmConfig};

struct Jdk {
    java: PathBuf,
    classes: PathBuf,
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

fn java_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("java")
}

/// Compiles every program once; `Err` says why the JDK is unusable.
fn jdk() -> &'static Result<Jdk, String> {
    static JDK: OnceLock<Result<Jdk, String>> = OnceLock::new();
    JDK.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let java = tool("java").ok_or("no java in JAVA_HOME or on the PATH")?;
        let classes = Path::new(env!("CARGO_TARGET_TMPDIR")).join("differential-classes");
        let _ = std::fs::remove_dir_all(&classes);
        std::fs::create_dir_all(&classes).map_err(|e| e.to_string())?;
        let mut sources: Vec<PathBuf> = std::fs::read_dir(java_dir())
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "java"))
            .collect();
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
        Ok(Jdk { java, classes })
    })
}

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end_matches('\n').to_owned()
}

fn run_reference(jdk: &Jdk, class: &str) -> String {
    let output = Command::new(&jdk.java)
        .args([
            "-Xss8m",
            "-Xmx256m",
            "-Dfile.encoding=UTF-8",
            "-Dsun.stdout.encoding=UTF-8",
            "-Dstdout.encoding=UTF-8",
            "-cp",
        ])
        .arg(&jdk.classes)
        .arg(class)
        .output()
        .expect("running java");
    normalize(&String::from_utf8_lossy(&output.stdout))
}

/// Runs `class` on FastEmu and returns what it printed to `System.out`
/// and to `System.err`.
fn run_fastemu(classes: &Path, class: &str, heap_bytes: usize) -> (String, String) {
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        heap_bytes,
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(DirSource::new(classes)), Vec::new());
    vm.run_main(class, &[]).expect("starting the main thread");
    if !vm.wait_finished(Some(Duration::from_secs(120))) {
        vm.stop();
        vm.wait_finished(Some(Duration::from_secs(5)));
        panic!("{class} did not finish within two minutes");
    }
    let lines = lines.lock().unwrap();
    let pick = |level: LogLevel| {
        lines
            .iter()
            .filter(|l| l.level == level)
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    (pick(LogLevel::Out), pick(LogLevel::Err))
}

fn first_difference(expected: &str, actual: &str) -> String {
    for (i, (e, a)) in expected.lines().zip(actual.lines()).enumerate() {
        if e != a {
            return format!("line {}:\n  java:    {e}\n  fastemu: {a}", i + 1);
        }
    }
    format!(
        "java printed {} lines, fastemu {}",
        expected.lines().count(),
        actual.lines().count()
    )
}

fn check(class: &str, heap_bytes: usize) {
    let jdk = match jdk() {
        Ok(jdk) => jdk,
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "FASTEMU_REQUIRE_JAVA=1 but {why}"
            );
            eprintln!("skipping {class}: {why}");
            return;
        }
    };
    let expected = run_reference(jdk, class);
    let (actual, errors) = run_fastemu(&jdk.classes, class, heap_bytes);
    assert!(
        expected == actual,
        "{class}: output differs, {}\n--- fastemu stderr ---\n{errors}\n--- fastemu stdout ---\n{actual}",
        first_difference(&expected, &actual)
    );
}

macro_rules! programs {
    ($($name:ident: $class:literal, $heap_mib:literal;)*) => {
        $(
            #[test]
            fn $name() {
                check($class, $heap_mib * 1024 * 1024);
            }
        )*
    };
}

programs! {
    arith: "Arith", 8;
    arrays: "Arrays", 8;
    calendar_fields: "CalendarFields", 8;
    charsets: "Charsets", 8;
    data_streams: "DataStreams", 8;
    exceptions: "Exceptions", 8;
    float_math: "FloatMath", 8;
    fused: "Fused", 8;
    gc_stress: "GcStress", 4;
    hashtables: "Hashtables", 8;
    hello: "Hello", 8;
    inheritance: "Inheritance", 8;
    print_streams: "PrintStreams", 8;
    random_sequences: "RandomSequences", 8;
    readers: "Readers", 8;
    recursion: "Recursion", 8;
    static_init: "StaticInit", 8;
    strings: "Strings", 8;
    switches: "Switches", 8;
    threads: "Threads", 8;
    timers: "Timers", 8;
    vectors: "Vectors", 8;
    wrappers: "Wrappers", 8;
}
