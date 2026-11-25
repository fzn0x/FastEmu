//! A game's Java threads are OS threads, so the machine counts them and
//! refuses past `VmLimits::max_threads`: `Thread.start` and `new Timer()`
//! throw `OutOfMemoryError`, as they would on a phone with no thread to
//! spare. The host's own threads have 16 more, so a game that has used up
//! its share cannot keep the host from starting its event and lifecycle
//! threads. The thread of a timer the game dropped ends once no task is
//! left, as on Java SE: at the limit, the machine collects once and waits
//! for such threads before it refuses.
//!
//! The cases are Java programs in `tests/thread_java/`, compiled once with
//! `javac --release 8`; the JDK comes from `JAVA_HOME` or the `PATH`.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it). Every machine here is
//! limited to 8 threads, so no test starts more than a few.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{MemorySource, Vm, VmConfig, VmLimits};

/// The threads each machine in this file may have.
const MAX_THREADS: usize = 8;

/// How long a test waits for something that takes milliseconds: far longer
/// than that, so that a busy machine waits instead of failing.
const LONG: Duration = Duration::from_secs(20);

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

/// Compiles `ThreadCases.java` once; `Err` says why the JDK is unusable.
fn compiled() -> &'static Result<MemorySource, String> {
    static COMPILED: OnceLock<Result<MemorySource, String>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        // One folder for each process, so two runs of this test at once do
        // not overwrite each other's class files; it goes away once the
        // classes are in memory.
        let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("thread-limits-classes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let source_file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("thread_java")
            .join("ThreadCases.java");
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

/// A machine that may have [`MAX_THREADS`] threads, over the compiled
/// cases, with the lines it logs; `None` after saying why when there is
/// no JDK.
fn machine() -> Option<(Vm, Arc<Mutex<Vec<LogLine>>>)> {
    let source = match compiled() {
        Ok(source) => source.clone(),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "FASTEMU_REQUIRE_JAVA=1 but {why}"
            );
            eprintln!("skipping: {why}");
            return None;
        }
    };
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        limits: VmLimits {
            max_threads: MAX_THREADS,
            ..VmLimits::default()
        },
        ..VmConfig::default()
    };
    Some((Vm::new(config, Arc::new(source), Vec::new()), lines))
}

/// Everything the game printed and the machine reported, in order.
fn log(lines: &Mutex<Vec<LogLine>>) -> Vec<String> {
    lines
        .lock()
        .unwrap()
        .iter()
        .map(|l| format!("{:?}: {}", l.level, l.text))
        .collect()
}

/// Runs `ThreadCases.main(mode)` and waits for it to print `end`, then
/// returns what it printed to `System.out`. Fails with the whole log when
/// the program does not get that far.
fn run(vm: &Vm, lines: &Mutex<Vec<LogLine>>, mode: &str) -> Vec<String> {
    vm.run_main("ThreadCases", &[mode])
        .expect("starting the main thread");
    let start = std::time::Instant::now();
    loop {
        // The count is read before the log: a program that prints end and
        // ends between the two reads still shows its end in this log.
        let running = vm.thread_count() > 0;
        let out = support::lines_at(lines, LogLevel::Out);
        if out.last().is_some_and(|l| l == "end") {
            return out;
        }
        assert!(
            start.elapsed() < LONG && running,
            "{mode} did not reach its end:\n{}",
            log(lines).join("\n")
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Stops the machine and waits for every thread to end.
fn stop(vm: &Vm) {
    vm.stop();
    assert!(
        vm.wait_finished(Some(LONG)),
        "{} threads outlived the stop",
        vm.thread_count()
    );
    assert_eq!(vm.thread_count(), 0);
}

#[test]
fn thread_start_throws_out_of_memory_error_at_the_limit() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "threads");
    // The main thread counts: it and seven sleepers are the eight.
    assert_eq!(
        out[0],
        format!("threads started {} active {MAX_THREADS}", MAX_THREADS - 1),
        "{out:?}"
    );
    assert_eq!(
        out[1],
        format!("message unable to create a thread: too many threads (limit {MAX_THREADS})")
    );
    // The thread that was refused started later, once there was room.
    assert_eq!(out[2], "restarted true", "{out:?}");
    stop(&vm);
}

#[test]
fn a_new_timer_throws_out_of_memory_error_at_the_limit() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "timers");
    assert_eq!(
        out[0],
        format!("timers started {} active {MAX_THREADS}", MAX_THREADS - 1),
        "{out:?}"
    );
    assert!(out[1].starts_with("message unable to create a thread: too many threads"));
    // The timers that were made worked, and cancelling them gave their
    // threads back: the main thread is the only one left.
    assert_eq!(out[2], "settled 1", "{out:?}");
    assert_eq!(out[3], "again ok", "{out:?}");
    stop(&vm);
}

#[test]
fn dropped_timers_do_not_use_up_the_games_threads() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "dropped");
    // Four times the limit, each dropped at once.
    assert_eq!(out[0], "dropped 32", "{out:?}");
    stop(&vm);
}

#[test]
fn timers_dropped_after_their_task_ran_do_not_use_up_the_games_threads() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "used");
    assert_eq!(out[0], "used 32", "{out:?}");
    stop(&vm);
}

#[test]
fn thread_start_gets_the_thread_of_a_dropped_timer() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "start");
    assert_eq!(out[0], "started true", "{out:?}");
    stop(&vm);
}

#[test]
fn a_dropped_timers_thread_ends_at_the_next_collection() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "collected");
    assert_eq!(out[0], "ended true", "{out:?}");
    stop(&vm);
}

#[test]
fn a_dropped_timer_runs_its_repeating_task_until_it_is_cancelled() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "repeating");
    assert_eq!(out[0], "still runs true", "{out:?}");
    assert_eq!(out[1], "ended true", "{out:?}");
    stop(&vm);
}

#[test]
fn timers_the_game_keeps_work_across_collections() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "kept");
    assert_eq!(out[0], "kept ok", "{out:?}");
    assert_eq!(out[1], "task ok", "{out:?}");
    let errors = support::lines_at(&lines, LogLevel::Err);
    assert!(errors.is_empty(), "{errors:?}");
    stop(&vm);
}

#[test]
fn a_start_at_the_limit_never_waits_for_a_timer_thread_that_runs_game_code() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "reporting");
    let refused = format!("unable to create a thread: too many threads (limit {MAX_THREADS})");
    assert_eq!(out[0], format!("refused {refused}"), "{out:?}");
    assert_eq!(out[1], format!("refused after gc {refused}"), "{out:?}");
    stop(&vm);
}

#[test]
fn a_thread_named_with_a_nul_character_starts_and_ends() {
    let Some((vm, lines)) = machine() else { return };
    let out = run(&vm, &lines, "nul");
    // The name has 9 characters: the Java side keeps it as it was given.
    assert_eq!(out[0], "ran 9", "{out:?}");
    assert_eq!(out[1], "name true", "{out:?}");
    let errors = support::lines_at(&lines, LogLevel::Err);
    assert!(errors.is_empty(), "{errors:?}");
    stop(&vm);
}

#[test]
fn the_host_has_sixteen_threads_past_a_games_share() {
    let machine = Vm::new(
        VmConfig {
            limits: VmLimits {
                max_threads: 4,
                ..VmLimits::default()
            },
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        },
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    // Host threads, which sleep until the machine stops: the game's share
    // is used up after four, the host's goes to twenty.
    for i in 0..20 {
        machine
            .spawn(&format!("host {i}"), |ctx| ctx.sleep(LONG))
            .unwrap_or_else(|e| panic!("host thread {i} was refused: {e}"));
    }
    assert_eq!(machine.thread_count(), 20);
    let refused = machine
        .spawn("one too many", |_| Ok(()))
        .expect_err("a 21st thread");
    assert!(
        refused.to_string().contains("too many threads (limit 20)"),
        "{refused}"
    );
    assert_eq!(
        machine.thread_count(),
        20,
        "a refused thread leaves no trace"
    );
    stop(&machine);
}

#[test]
fn a_thread_the_library_starts_for_the_game_draws_on_the_hosts_reserve() {
    let machine = Vm::new(
        VmConfig {
            limits: VmLimits {
                max_threads: 4,
                ..VmLimits::default()
            },
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        },
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    for i in 0..4 {
        machine
            .spawn(&format!("host {i}"), |ctx| ctx.sleep(LONG))
            .unwrap();
    }
    let (game, library) = machine.attach("probe", |ctx| {
        let game = ctx.spawn_thread("game", |_| Ok(())).is_ok();
        let library = ctx.spawn_host_thread("library", |_| Ok(())).is_ok();
        (game, library)
    });
    assert!(!game, "the game's share was used up");
    assert!(library, "the host's reserve was free");
    stop(&machine);
}
