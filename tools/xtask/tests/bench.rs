//! `cargo xtask bench` end to end, without MicroEmulator: the benchmark
//! MIDlet builds against the stubs, runs on FastEmu through `xtask
//! bench-run`, and gives the answers a JVM gives; MicroEmulator's wrapper
//! compiles against its stand-ins.
//!
//! The tests need a JDK. Without one they print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it). They assert no time and
//! no rate, only what the MIDlet computed and printed; one expects the
//! scene's first frame within its 5 s run.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The answers of the CPU and memory kernels at Bench-Scale 1, as
/// HotSpot computes them running the same kernels.
const ANSWERS: [(&str, &str); 12] = [
    ("cpu.int", "12751892"),
    ("cpu.call", "75025"),
    ("cpu.sieve", "9592"),
    ("cpu.sort", "-6495784"),
    ("cpu.actors", "1789839"),
    ("cpu.long", "2070728297839542552"),
    ("cpu.float", "1263480023"),
    ("cpu.string", "60540"),
    ("cpu.collections", "477080"),
    ("mem.small", "49995000"),
    ("mem.arrays", "2081637"),
    // One more than the last value of a live set of 2000 links.
    ("mem.gc", "2000"),
];

const DRAWING: [&str; 8] = [
    "draw.fillrect",
    "draw.line",
    "draw.image",
    "draw.alpha",
    "draw.region",
    "draw.text",
    "draw.rgb",
    "draw.scene",
];

/// A scratch folder of its own for each test, so two tests never build
/// into one folder at once.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("bench-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("making a scratch folder");
    dir
}

fn xtask() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
}

/// Builds into `out`; `false` (after saying why) when there is no JDK
/// and none is required.
fn build(out: &Path) -> bool {
    let output = xtask()
        .args(["bench", "--build-only", "--out"])
        .arg(out)
        .output()
        .expect("running xtask");
    let err = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() && err.contains("no javac") {
        assert!(
            std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
            "Java is required but unusable: {err}"
        );
        eprintln!("skipping: {err}");
        return false;
    }
    assert!(output.status.success(), "the build failed:\n{err}");
    true
}

/// Runs `suite` in `bench-run` and returns what it printed.
fn bench_run(suite: &Path, out: &Path) -> String {
    let output = xtask()
        .arg("bench-run")
        .arg(suite)
        .args(["--seconds", "120", "--max-fps", "0", "--data"])
        .arg(out.join("data"))
        .output()
        .expect("running bench-run");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "bench-run failed:\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    text
}

/// The `BENCH name ...` lines of a run, as their words.
fn bench_lines<'a>(text: &'a str, name: &str) -> Vec<Vec<&'a str>> {
    text.lines()
        .filter_map(|l| l.strip_prefix("log 0 out BENCH "))
        .map(|l| l.split(' ').collect::<Vec<_>>())
        .filter(|words| words[0] == name)
        .collect()
}

fn field<'a>(words: &[&'a str], key: &str) -> Option<&'a str> {
    words
        .iter()
        .find_map(|w| w.strip_prefix(key)?.strip_prefix('='))
}

#[test]
fn midlet_runs_on_fastemu_with_jvm_answers() {
    let out = scratch("midlet");
    if !build(&out) {
        return;
    }
    for jar in ["bench.jar", "scene.jar", "check.jar"] {
        assert!(out.join(jar).is_file(), "{jar} was not packed");
    }
    let text = bench_run(&out.join("check.jar"), &out);
    assert!(
        text.contains("log 0 out BENCH screen w=240 h=320"),
        "{text}"
    );
    assert!(
        text.lines().any(|l| l.starts_with("RUN first game=0 ms=")),
        "{text}"
    );
    assert!(
        text.lines()
            .any(|l| l.starts_with("RUN end game=0 status=exited ")),
        "{text}"
    );
    let clock = bench_lines(&text, "clock");
    assert!(
        clock
            .first()
            .and_then(|w| field(w, "step"))
            .is_some_and(|s| s.parse::<u64>().is_ok()),
        "{text}"
    );
    assert!(!bench_lines(&text, "done").is_empty(), "{text}");
    assert!(bench_lines(&text, "error").is_empty(), "{text}");
    let names = ANSWERS.iter().map(|(name, _)| *name).chain(DRAWING);
    for name in names {
        let rounds = bench_lines(&text, name);
        let numbers: Vec<&str> = rounds.iter().filter_map(|w| field(w, "round")).collect();
        assert_eq!(numbers, ["1", "2"], "{name}:\n{text}");
        for words in &rounds {
            for key in ["reps", "ms", "first", "check"] {
                assert!(
                    field(words, key).is_some(),
                    "{name} has no {key}: {words:?}"
                );
            }
        }
    }
    for (name, answer) in ANSWERS {
        for words in bench_lines(&text, name) {
            assert_eq!(field(&words, "check"), Some(answer), "{name}: {words:?}");
        }
    }
    let free = bench_lines(&text, "frames.free");
    assert!(
        free.first()
            .and_then(|w| field(w, "frames"))
            .is_some_and(|n| n.parse::<u64>().is_ok_and(|n| n >= 6)),
        "{text}"
    );
}

#[test]
fn a_failing_kernel_does_not_stop_the_others() {
    let out = scratch("failing");
    if !build(&out) {
        return;
    }
    // A descriptor beside the JAR overrides its manifest: draw.line throws.
    let jad = out.join("fail.jad");
    std::fs::write(
        &jad,
        "MIDlet-Name: FastEmu Bench\nMIDlet-Vendor: FastEmu\nMIDlet-Version: 1.0\n\
         MIDlet-1: FastEmu Bench, , fastemu.bench.Bench\nMIDlet-Jar-URL: check.jar\n\
         Bench-Fail: draw.line\n",
    )
    .expect("writing the descriptor");
    let text = bench_run(&jad, &out);
    let errors = bench_lines(&text, "error");
    assert_eq!(errors.len(), 1, "{text}");
    assert_eq!(field(&errors[0], "name"), Some("draw.line"), "{text}");
    assert!(bench_lines(&text, "draw.line").is_empty(), "{text}");
    for later in ["draw.image", "draw.scene", "frames.free", "done"] {
        assert!(
            !bench_lines(&text, later).is_empty(),
            "{later} did not run:\n{text}"
        );
    }
}

#[test]
fn wrapper_compiles_against_the_api_stubs() {
    let out = scratch("wrapper");
    if !build(&out) {
        return;
    }
    assert!(
        out.join("mebench")
            .join("fastemu")
            .join("bench")
            .join("MeBench.class")
            .is_file()
    );
    // Only the wrapper: the stand-ins are never packed or kept as classes.
    assert!(!out.join("mebench").join("org").exists());
}

#[test]
fn bench_run_ends_a_running_game() {
    let out = scratch("scene");
    if !build(&out) {
        return;
    }
    let output = xtask()
        .arg("bench-run")
        .arg(out.join("scene.jar"))
        .args(["--seconds", "5", "--data"])
        .arg(out.join("data"))
        .output()
        .expect("running bench-run");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{text}");
    let end = text
        .lines()
        .find(|l| l.starts_with("RUN end game=0 "))
        .unwrap_or_else(|| panic!("no end:\n{text}"));
    assert!(end.contains(" status=running "), "{end}");
    let frames: u64 = end
        .split(' ')
        .find_map(|w| w.strip_prefix("frames="))
        .and_then(|n| n.parse().ok())
        .expect("a frame count");
    assert!(frames > 0, "{end}");
}
