//! Runs the Java programs in `tests/media_java` on a FastEmu virtual
//! machine with the MIDP classes, an instance environment and, when asked,
//! an offline audio engine (no output device: the program moves its time
//! with the `Pump` class below).
//!
//! The programs compile once per test binary with `javac --release 8`
//! against the stubs in `tests/media_stubs`; only the programs' own classes
//! are loaded. Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set.

#![allow(dead_code)] // Each test file uses its own subset.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use fastemu_audio::{Engine, EngineConfig, Group};
use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{Ctx, MemorySource, NativeClass, Value, Vm, VmConfig};
use fastemu_midp::env::{InstanceEnv, try_env};

/// The rate of the test engine: sound at 8 kHz plays sample for sample.
pub const RATE: u32 = 8000;

/// What a program printed.
#[derive(Debug, Default)]
pub struct Output {
    pub out: Vec<String>,
    pub err: Vec<String>,
    pub warn: Vec<String>,
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
/// JDK to do it.
fn compiled() -> &'static Result<PathBuf, String> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let javac_path = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        // One folder per test binary, so binaries never share one.
        let binary = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "media".to_owned());
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("media-java-{binary}"));
        let stubs = root.join("stubs");
        let classes = root.join("classes");
        javac(
            &javac_path,
            &stubs,
            None,
            &java_files(&tests.join("media_stubs")),
        );
        javac(
            &javac_path,
            &classes,
            Some(&stubs),
            &java_files(&tests.join("media_java")),
        );
        Ok(classes)
    })
}

/// The compiled programs, or `None` (with a note) when there is no JDK.
fn classes() -> Option<&'static Path> {
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

/// A 16-bit mono WAV at [`RATE`] holding a 440 Hz sine of `ms`.
pub fn sine_wav(ms: u32) -> Vec<u8> {
    let frames = RATE * ms / 1000;
    let data: Vec<u8> = (0..frames)
        .map(|i| {
            let phase = f64::from(i) * 440.0 / f64::from(RATE) * std::f64::consts::TAU;
            (phase.sin() * 16000.0) as i16
        })
        .flat_map(i16::to_le_bytes)
        .collect();
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// A MIDI file playing middle C for one beat (500 ms at the default
/// tempo).
pub fn tune_mid() -> Vec<u8> {
    let mut track = vec![
        0x00, 0xC0, 0x00, // piano
        0x00, 0x90, 60, 100, // note on
        0x83, 0x60, 0x80, 60, 0, // 480 ticks later, note off
    ];
    track.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    let mut out = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    out.extend_from_slice(&480u16.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&(track.len() as u32).to_be_bytes());
    out.extend_from_slice(&track);
    out
}

/// A Nokia Smart Messaging ringing tone: one A (880 Hz) for a quarter
/// note at the default 63 bpm, continuous style: 952 ms.
pub fn ring_ott() -> Vec<u8> {
    let mut bits = String::new();
    let mut put = |value: u32, count: usize| {
        for i in (0..count).rev() {
            bits.push(if (value >> i) & 1 == 1 { '1' } else { '0' });
        }
    };
    put(2, 8); // two commands
    put(0b010_0101, 7); // ringing tone programming
    put(0, 1); // filler
    put(0b001_1101, 7); // sound
    put(0b001, 3); // basic song
    put(1, 4); // title of one character
    put(u32::from(b'N'), 8);
    put(1, 8); // one pattern
    put(0, 3);
    put(0, 2);
    put(0, 4);
    put(2, 8); // two instructions
    put(0b011, 3); // style: continuous
    put(1, 2);
    put(0b001, 3); // note A, quarter, no dots
    put(10, 4);
    put(2, 3);
    put(0, 2);
    while !bits.len().is_multiple_of(8) {
        bits.push('0');
    }
    bits.as_bytes()
        .chunks(8)
        .map(|byte| u8::from_str_radix(std::str::from_utf8(byte).unwrap(), 2).unwrap())
        .collect()
}

/// The engine of the instance `ctx` runs in, if it has one.
fn engine(ctx: &Ctx<'_>) -> Option<Engine> {
    try_env(ctx)
        .and_then(|env| env.audio.as_ref())
        .map(|group| group.engine().clone())
}

fn frames(ms: i32) -> usize {
    (ms.max(0) as usize) * RATE as usize / 1000
}

/// `Pump`: moves the offline engine's time from Java.
fn pump() -> NativeClass {
    NativeClass::new("Pump")
        .static_method("advance", "(I)V", |ctx, a| {
            if let Some(engine) = engine(ctx) {
                engine.advance(frames(a[0].int()));
            }
            Ok(Value::Void)
        })
        .static_method("peak", "(I)I", |ctx, a| {
            let Some(engine) = engine(ctx) else {
                return Ok(Value::Int(0));
            };
            let mut buffer = vec![0.0f32; frames(a[0].int()) * 2];
            engine.render(&mut buffer);
            let peak = buffer.iter().fold(0.0f32, |m, v| m.max(v.abs()));
            Ok(Value::Int((peak * 1000.0) as i32))
        })
        .static_method("players", "()I", |ctx, _| {
            Ok(Value::Int(
                engine(ctx).map_or(0, |e| e.player_count()) as i32
            ))
        })
}

/// A test engine and a group for one instance.
pub fn audio() -> (Engine, Group) {
    let engine = Engine::new(EngineConfig { sample_rate: RATE });
    let group = engine.create_group();
    (engine, group)
}

/// Runs `class`'s `main` with `audio` as the instance's sound and returns
/// what it printed, or `None` when there is no JDK. The program ends by
/// printing `done`.
pub fn run(class: &str, audio: Option<Group>) -> Option<Output> {
    let classes = classes()?;
    let mut source = MemorySource::new();
    for entry in std::fs::read_dir(classes).expect("reading the classes") {
        let path = entry.expect("a class entry").path();
        if let Some(name) = path.file_stem().and_then(|s| s.to_str())
            && path.extension().is_some_and(|x| x == "class")
        {
            source.add_class(name, std::fs::read(&path).expect("reading a class"));
        }
    }
    source
        .add_resource("/beep.wav", sine_wav(200))
        .add_resource("/long.wav", sine_wav(1000))
        .add_resource("/tune.mid", tune_mid())
        .add_resource("/ring.ott", ring_ott());

    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        allow_exit: false,
        ..VmConfig::default()
    };
    let mut natives = fastemu_midp::classes();
    natives.push(pump());
    let vm = Vm::new(config, Arc::new(source), natives);
    let data_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("media-data");
    vm.attach("setup", |ctx| {
        ctx.set_extension(InstanceEnv {
            audio,
            ..InstanceEnv::for_tests(data_dir)
        });
    });
    vm.run_main(class, &[]).expect("starting main");

    let deadline = Instant::now() + Duration::from_secs(120);
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
        warn: pick(LogLevel::Warn),
    })
}

/// Checks that `class` printed `expected` (one line per line, `done`
/// last), showing the error output when it did not.
pub fn check(class: &str, audio: Option<Group>, expected: &str) -> Option<Output> {
    let output = run(class, audio)?;
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
            let _ = writeln!(report, "{mark}{w:<55} | {g}");
        }
        panic!(
            "{class}: output differs (expected | got)\n{report}\n--- stderr ---\n{}\n--- warnings ---\n{}",
            output.err.join("\n"),
            output.warn.join("\n")
        );
    }
    Some(output)
}
