//! A machine says what its threads are doing (`Vm::activity`), so a tool
//! can tell a game that waits on a still screen from one that is stuck: a
//! thread in a sleep, `Object.wait` or `Thread.join` waits, a thread that
//! spins runs, and threads that block each other on monitors or on a
//! class initializer wait for a lock.
//!
//! The cases are Java programs in `tests/activity_java/`, compiled once
//! with `javac --release 8`; the JDK comes from `JAVA_HOME` or the `PATH`.
//! Without a JDK those tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it). No test depends on how
//! fast the machine is: each waits as long as it takes for a state that
//! comes, and checks a state that never changes as often as it likes.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{Activity, MemorySource, Vm, VmConfig};

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

/// Compiles `ActivityCases.java` once; `Err` says why the JDK is unusable.
fn compiled() -> &'static Result<MemorySource, String> {
    static COMPILED: OnceLock<Result<MemorySource, String>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        // One folder for each process, so two runs of this test at once do
        // not overwrite each other's class files; it goes away once the
        // classes are in memory.
        let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("activity-classes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let source_file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("activity_java")
            .join("ActivityCases.java");
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

/// A machine over `source`, with the lines it logs.
fn machine_over(source: MemorySource) -> (Vm, Arc<Mutex<Vec<LogLine>>>) {
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        ..VmConfig::default()
    };
    (Vm::new(config, Arc::new(source), Vec::new()), lines)
}

/// A machine over the compiled cases; `None` after saying why when there
/// is no JDK.
fn machine() -> Option<(Vm, Arc<Mutex<Vec<LogLine>>>)> {
    match compiled() {
        Ok(source) => Some(machine_over(source.clone())),
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

/// Runs `ActivityCases.main(mode)` and waits for it to print `ready`.
fn start(vm: &Vm, lines: &Mutex<Vec<LogLine>>, mode: &str) {
    vm.run_main("ActivityCases", &[mode])
        .expect("starting the main thread");
    let begun = Instant::now();
    while !support::lines_at(lines, LogLevel::Out)
        .iter()
        .any(|l| l == "ready")
    {
        assert!(
            begun.elapsed() < LONG,
            "{mode} did not get ready: {:?}",
            support::lines_at(lines, LogLevel::Err)
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Waits until the machine's activity passes `test`, and returns it.
fn wait_for(vm: &Vm, test: impl Fn(&Activity) -> bool) -> Activity {
    let begun = Instant::now();
    loop {
        if let Some(activity) = vm.activity()
            && test(&activity)
        {
            return activity;
        }
        assert!(
            begun.elapsed() < LONG,
            "the activity stayed {:?}",
            vm.activity()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Looks at the machine `times` times; every look must pass `test` or
/// find the lock taken.
fn always(vm: &Vm, times: usize, test: impl Fn(&Activity) -> bool) {
    for _ in 0..times {
        let activity = vm.activity();
        assert!(activity.is_none_or(|a| test(&a)), "{activity:?}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Stops the machine and waits for every thread to end.
fn stop(vm: &Vm) {
    vm.stop();
    assert!(vm.wait_finished(Some(LONG)), "threads outlived the stop");
}

#[test]
fn threads_that_sleep_wait_or_join_leave_the_machine_idle() {
    let Some((vm, lines)) = machine() else { return };
    start(&vm, &lines, "waits");
    // Every thread waits only once all three have got where they wait.
    let activity = wait_for(&vm, Activity::is_idle);
    // main in join, one thread in Object.wait and one in sleep.
    assert_eq!(
        activity,
        Activity {
            threads: 3,
            waiting: 3,
            blocked: 0,
            posted: 0,
        }
    );
    stop(&vm);
}

#[test]
fn a_thread_that_spins_is_never_idle() {
    let Some((vm, lines)) = machine() else { return };
    start(&vm, &lines, "spins");
    // The loop never waits, so no look finds it waiting, however the
    // threads are scheduled.
    always(&vm, 200, |a| a.running() == 1 && !a.is_idle());
    stop(&vm);
}

#[test]
fn threads_that_block_each_other_on_monitors_wait_for_a_lock() {
    let Some((vm, lines)) = machine() else { return };
    start(&vm, &lines, "crosses");
    // main has ended; each crosser holds one lock and asks for the other.
    let activity = wait_for(&vm, |a| a.threads == 2 && a.blocked == 2);
    assert!(!activity.is_idle(), "{activity:?}");
    // A deadlock does not end, and never looks idle.
    always(&vm, 50, |a| a.blocked == 2 && !a.is_idle());
    stop(&vm);
}

#[test]
fn a_notified_thread_that_cannot_take_its_monitor_back_waits_for_a_lock() {
    let Some((vm, lines)) = machine() else { return };
    start(&vm, &lines, "reenters");
    // The waiter was notified, but main sleeps holding the monitor.
    let expected = Activity {
        threads: 2,
        waiting: 1,
        blocked: 1,
        posted: 0,
    };
    wait_for(&vm, |a| *a == expected);
    // The waiter never gets the monitor back, and the machine never looks
    // idle, though no thread runs.
    always(&vm, 50, |a| a.blocked == 1 && !a.is_idle());
    stop(&vm);
}

#[test]
fn a_thread_that_waits_for_another_threads_class_initializer_waits_for_a_lock() {
    let Some((vm, lines)) = machine() else { return };
    start(&vm, &lines, "initializes");
    // The initializer's thread waits in Object.wait; main waits for it.
    let expected = Activity {
        threads: 2,
        waiting: 1,
        blocked: 1,
        posted: 0,
    };
    wait_for(&vm, |a| *a == expected);
    always(&vm, 50, |a| a.blocked == 1 && !a.is_idle());
    stop(&vm);
}

#[test]
fn a_wait_outside_the_lock_counts_as_waiting_only_when_marked() {
    let (vm, _lines) = machine_over(MemorySource::new());
    let (marked_tx, marked_rx) = mpsc::channel::<()>();
    vm.spawn("marked", move |ctx| {
        let _ = ctx.wait_without_lock(|| marked_rx.recv());
        Ok(())
    })
    .expect("starting a thread");
    let activity = wait_for(&vm, Activity::is_idle);
    assert_eq!((activity.threads, activity.waiting), (1, 1));
    let (plain_tx, plain_rx) = mpsc::channel::<()>();
    vm.spawn("plain", move |ctx| {
        let _ = ctx.without_lock(|| plain_rx.recv());
        Ok(())
    })
    .expect("starting a thread");
    // Work outside the lock, such as decoding, counts as running.
    always(&vm, 100, |a| {
        a.threads == 2 && a.waiting == 1 && a.running() == 1 && !a.is_idle()
    });
    drop((marked_tx, plain_tx));
    stop(&vm);
}

#[test]
fn a_posted_task_keeps_the_machine_busy_until_it_runs() {
    let (vm, _lines) = machine_over(MemorySource::new());
    // A thread that waits for ever and never runs posted tasks.
    vm.spawn("waiter", |ctx| {
        loop {
            ctx.wait_until(None)?;
        }
    })
    .expect("starting a thread");
    wait_for(&vm, Activity::is_idle);
    vm.post(|_| Ok(()));
    let activity = wait_for(&vm, |a| a.posted == 1);
    assert!(!activity.is_idle(), "{activity:?}");
    // Nothing runs the task, so the machine stays busy.
    always(&vm, 50, |a| a.posted == 1 && !a.is_idle());
    stop(&vm);
}
