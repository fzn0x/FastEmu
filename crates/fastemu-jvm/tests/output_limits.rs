//! What a game makes the machine keep outside its heap stays small:
//! output without a line break, log lines, the diagnostics about names it
//! asks for, and the tasks posted to the machine. Each limit is one of
//! `VmLimits`.
//!
//! The Java cases are in `tests/output_java/`, compiled once with
//! `javac --release 8`; the JDK comes from `JAVA_HOME` or the `PATH`.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, Diagnostic, MemorySource, Throw, Value, Vm, VmConfig, VmLimits};
use support::{ClassBuilder, PUBLIC, STATIC, capture, idx, lines_at};

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

/// Compiles `OutputCases.java` once; `Err` says why the JDK is unusable.
fn compiled() -> &'static Result<MemorySource, String> {
    static COMPILED: OnceLock<Result<MemorySource, String>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        // One folder for each process, so two runs at once do not
        // overwrite each other's class files.
        let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("output-limits-classes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let source_file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("output_java")
            .join("OutputCases.java");
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
            .arg(&out)
            .arg(&source_file)
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let mut source = MemorySource::new();
        for entry in std::fs::read_dir(&out).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|x| x == "class") {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("a class file without a name")?
                    .to_owned();
                source.add_class(name, std::fs::read(&path).map_err(|e| e.to_string())?);
            }
        }
        let _ = std::fs::remove_dir_all(&out);
        Ok(source)
    })
}

/// The compiled cases, or `None` (after saying why) when no JDK is
/// installed and Java is not required.
fn cases() -> Option<&'static MemorySource> {
    match compiled() {
        Ok(source) => Some(source),
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

/// A machine over the compiled cases with `limits`, its log lines and the
/// diagnostics it reported.
struct Rig {
    vm: Vm,
    lines: Arc<Mutex<Vec<fastemu_core::LogLine>>>,
    reported: Arc<Mutex<Vec<Diagnostic>>>,
    delivered: AtomicUsize,
}

fn rig(source: MemorySource, limits: VmLimits) -> Rig {
    let (log, lines) = capture();
    let reported: Arc<Mutex<Vec<Diagnostic>>> = Arc::default();
    let seen = Arc::clone(&reported);
    let config = VmConfig {
        log,
        diagnostics: Some(Arc::new(move |d| seen.lock().unwrap().push(d.clone()))),
        limits,
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(source), Vec::new());
    Rig {
        vm,
        lines,
        reported,
        delivered: AtomicUsize::new(0),
    }
}

impl Rig {
    /// Calls `OutputCases.<method>` with `desc` and `args`, and returns
    /// what it returned.
    fn call(&self, method: &str, desc: &str, args: &[i32]) -> Value {
        let args: Vec<Value> = args.iter().map(|&a| Value::Int(a)).collect();
        self.vm.attach("test", |ctx| {
            let result = ctx.call_static("OutputCases", method, desc, &args);
            // A thread that ends sends what it still holds of its last
            // line, so this is the count the game itself brought out.
            self.delivered
                .store(self.lines.lock().unwrap().len(), Ordering::Relaxed);
            match result {
                Ok(v) => v,
                Err(Throw::Exception(e)) => {
                    panic!("OutputCases.{method} threw {}", ctx.describe_exception(e))
                }
                Err(Throw::Stop) => panic!("OutputCases.{method} was stopped"),
            }
        })
    }

    /// How many lines the sink had when the last call returned.
    fn delivered(&self) -> usize {
        self.delivered.load(Ordering::Relaxed)
    }

    /// The texts of the lines at `level`.
    fn at(&self, level: LogLevel) -> Vec<String> {
        lines_at(&self.lines, level)
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        self.vm.stop();
        self.vm.wait_finished(Some(Duration::from_secs(10)));
    }
}

#[test]
fn output_without_a_line_break_comes_out_as_lines_of_4096_bytes() {
    let Some(source) = cases() else { return };
    let rig = rig(source.clone(), VmLimits::default());
    // 20 MB of text and not one line break.
    rig.call("print", "(II)V", &[200, 100_000]);
    let total = 200 * 100_000;
    let max = VmLimits::default().max_log_line_bytes;
    // The lines are out while the game runs, not when it ends: what is
    // still waiting is less than one line.
    assert_eq!(
        rig.delivered(),
        total / max,
        "full lines when the game returned"
    );
    let lines = rig.at(LogLevel::Out);
    assert_eq!(lines.len(), total.div_ceil(max));
    assert_eq!(lines.iter().map(String::len).sum::<usize>(), total);
    assert!(lines.iter().all(|l| l.len() <= max));
}

#[test]
fn error_output_without_a_line_break_comes_out_as_lines_too() {
    let Some(source) = cases() else { return };
    let limits = VmLimits {
        max_log_line_bytes: 1000,
        ..VmLimits::default()
    };
    let rig = rig(source.clone(), limits);
    rig.call("printErr", "(II)V", &[10, 9_999]);
    assert_eq!(rig.delivered(), 99_990 / 1000);
    let lines = rig.at(LogLevel::Err);
    assert_eq!(lines.len(), 99_990usize.div_ceil(1000));
    assert!(lines.iter().all(|l| l.len() <= 1000));
    assert_eq!(lines.iter().map(String::len).sum::<usize>(), 99_990);
    assert!(rig.at(LogLevel::Out).is_empty());
}

#[test]
fn one_long_line_is_split_and_loses_no_text() {
    let Some(source) = cases() else { return };
    let rig = rig(source.clone(), VmLimits::default());
    rig.call("println", "(I)V", &[500_000]);
    let lines = rig.at(LogLevel::Out);
    assert_eq!(lines.len(), 500_000usize.div_ceil(4096));
    assert!(lines.iter().all(|l| l.len() <= 4096));
    assert_eq!(lines.iter().map(String::len).sum::<usize>(), 500_000);
}

#[test]
fn a_line_the_machine_logs_is_cut_with_a_marker() {
    let (log, lines) = capture();
    let vm = Vm::new(
        VmConfig {
            log,
            ..VmConfig::default()
        },
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    vm.attach("test", |ctx| {
        ctx.log(LogLevel::Warn, "z".repeat(1_000_000));
        ctx.log(LogLevel::Info, "short");
    });
    let lines = lines.lock().unwrap();
    assert_eq!(lines.len(), 2);
    let max = VmLimits::default().max_log_line_bytes;
    assert!(lines[0].text.len() <= max, "{} bytes", lines[0].text.len());
    assert!(lines[0].text.ends_with(" more characters)"));
    assert_eq!(lines[1].text, "short");
}

#[test]
fn a_game_that_asks_for_many_classes_that_are_not_there_is_reported_256_times() {
    let Some(source) = cases() else { return };
    let rig = rig(source.clone(), VmLimits::default());
    let max = VmLimits::default().max_reported;
    // 1000 different names of about 170 characters each.
    let thrown = rig.call("misses", "(II)I", &[1000, 160]).int();
    assert_eq!(thrown, 1000, "the game still gets its exceptions");
    let reported = rig.reported.lock().unwrap().clone();
    assert_eq!(reported.len(), max);
    for d in &reported {
        let Diagnostic::MissingClass(name) = d else {
            panic!("{d}")
        };
        assert!(name.chars().count() <= 255, "{} characters", name.len());
    }
    let warnings = rig.at(LogLevel::Warn);
    assert_eq!(
        warnings
            .iter()
            .filter(|w| w.starts_with("missing class"))
            .count(),
        max
    );
    let stopped = |w: &&String| w.contains("further diagnostics are not reported");
    assert_eq!(warnings.iter().filter(stopped).count(), 1);
    // A name that is reported already is no news, even when the set is
    // full.
    rig.call("misses", "(II)I", &[1, 160]);
    assert_eq!(rig.at(LogLevel::Warn).iter().filter(stopped).count(), 1);
    assert_eq!(rig.reported.lock().unwrap().len(), max);
}

#[test]
fn the_reported_limit_can_be_lowered() {
    let Some(source) = cases() else { return };
    let limits = VmLimits {
        max_reported: 5,
        ..VmLimits::default()
    };
    let rig = rig(source.clone(), limits);
    rig.call("misses", "(II)I", &[50, 10]);
    assert_eq!(rig.reported.lock().unwrap().len(), 5);
}

#[test]
fn the_names_in_a_diagnostic_are_cut_to_255_characters() {
    // A class that calls a method of `Object` whose name has 3000
    // characters.
    let name = "m".repeat(3000);
    let mut class = ClassBuilder::new("Probe", "java/lang/Object", 49, 0);
    let target = class.method_ref("java/lang/Object", &name, "()V");
    let [hi, lo] = idx(target);
    class.method(
        PUBLIC | STATIC,
        "run",
        "()V",
        0,
        0,
        &[0xb8, hi, lo, 0xb1],
        &[],
    );
    let source = support::source(&[("Probe", class.build())]);
    let rig = rig(source, VmLimits::default());
    rig.vm.attach("test", |ctx| {
        let result = ctx.call_static("Probe", "run", "()V", &[]);
        assert!(matches!(result, Err(Throw::Exception(_))), "{result:?}");
    });
    let reported = rig.reported.lock().unwrap().clone();
    let [Diagnostic::MissingMethod { class, name, desc }] = &reported[..] else {
        panic!("{reported:?}");
    };
    assert_eq!(class, "java/lang/Object");
    assert_eq!(name.chars().count(), 255);
    assert_eq!(desc, "()V");
    assert!(rig.at(LogLevel::Warn).iter().all(|w| w.len() <= 4096));
}

/// Posts `count` tasks that each add one to `ran`, from `post`, and then
/// runs whatever the machine kept.
fn posted_tasks_that_ran(
    limits: VmLimits,
    count: usize,
    post: fn(&Vm, &Arc<AtomicUsize>, usize),
) -> (usize, Vec<String>) {
    let (log, lines) = capture();
    let vm = Vm::new(
        VmConfig {
            log,
            limits,
            ..VmConfig::default()
        },
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    let ran = Arc::new(AtomicUsize::new(0));
    post(&vm, &ran, count);
    let drained = vm.attach("loop", |ctx| ctx.run_posted().expect("a drain"));
    assert_eq!(drained, ran.load(Ordering::Relaxed));
    (drained, lines_at(&lines, LogLevel::Warn))
}

fn post_from_host(vm: &Vm, ran: &Arc<AtomicUsize>, count: usize) {
    for _ in 0..count {
        let ran = Arc::clone(ran);
        vm.post(move |_| {
            ran.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
    }
}

fn post_from_a_native(vm: &Vm, ran: &Arc<AtomicUsize>, count: usize) {
    // The thread that would run the tasks is this one, blocked in a
    // native that posts.
    vm.attach("native", |ctx: &mut Ctx<'_>| {
        for _ in 0..count {
            let ran = Arc::clone(ran);
            ctx.post(move |_| {
                ran.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });
        }
    });
}

#[test]
fn tasks_posted_from_a_native_stop_at_the_limit() {
    let max = VmLimits::default().max_posted;
    let (ran, warnings) = posted_tasks_that_ran(VmLimits::default(), 2 * max, post_from_a_native);
    assert_eq!(ran, max);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("tasks wait"), "{warnings:?}");
}

#[test]
fn tasks_posted_from_the_host_stop_at_the_limit() {
    let max = VmLimits::default().max_posted;
    let (ran, warnings) = posted_tasks_that_ran(VmLimits::default(), 2 * max, post_from_host);
    assert_eq!(ran, max);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
}

#[test]
fn the_posted_limit_can_be_lowered_and_few_tasks_do_not_warn() {
    let limits = VmLimits {
        max_posted: 3,
        ..VmLimits::default()
    };
    let (ran, warnings) = posted_tasks_that_ran(limits, 10, post_from_a_native);
    assert_eq!(ran, 3);
    assert_eq!(warnings.len(), 1);
    let (ran, warnings) = posted_tasks_that_ran(limits, 3, post_from_host);
    assert_eq!(ran, 3);
    assert!(warnings.is_empty(), "{warnings:?}");
}
