//! The bench's CPU and memory kernels on the virtual machine alone, with
//! no MIDP.
//!
//! The kernels are the bench MIDlet's own,
//! `tools/xtask/bench/midlet/fastemu/bench/Kernels.java`, and
//! `tests/kernels_java/` holds a driver that runs each with the bench's
//! sizes. Both compile once with `javac --release 8`; the JDK comes from
//! `JAVA_HOME` or the `PATH`. Without a JDK the tests print why and pass,
//! unless `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).
//!
//! At the bench's scale 1, every kernel must give the answer HotSpot
//! gives. The timing run is ignored by default. It times each kernel as
//! the bench does, 5 rounds of at least 500 ms, and prints the median of
//! rounds 2 to 5 and the first repetition. Run it in release:
//!
//! ```text
//! cargo test -p fastemu-jvm --release --test kernels -- --ignored --nocapture
//! ```
//!
//! `FASTEMU_KERNELS=cpu.int,cpu.call` times only the kernels it names.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, DirSource, JResult, Value, Vm, VmConfig};

/// The kernels in the driver's order, with their answers at scale 1, as
/// `tools/xtask/tests/bench.rs` has them from HotSpot.
const KERNELS: [(&str, i64); 12] = [
    ("cpu.int", 12_751_892),
    ("cpu.call", 75_025),
    ("cpu.sieve", 9_592),
    ("cpu.sort", -6_495_784),
    ("cpu.actors", 1_789_839),
    ("cpu.long", 2_070_728_297_839_542_552),
    ("cpu.float", 1_263_480_023),
    ("cpu.string", 60_540),
    ("cpu.collections", 477_080),
    ("mem.small", 49_995_000),
    ("mem.arrays", 2_081_637),
    // One more than the last value of a live set of 2000 links.
    ("mem.gc", 2_000),
];

/// The bench's heap for a game, the generic profile's.
const HEAP_BYTES: usize = 8 * 1024 * 1024;
/// The bench's work for one repetition, its `Bench-Scale`.
const SCALE: i32 = 100;
const ROUNDS: usize = 5;
const ROUND: Duration = Duration::from_millis(500);

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

/// The compiled kernels and driver, once; `Err` says why there are none.
fn classes() -> &'static Result<PathBuf, String> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let kernels = manifest.join("../../tools/xtask/bench/midlet/fastemu/bench/Kernels.java");
        let driver = manifest.join("tests/kernels_java/fastemu/bench/Driver.java");
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("kernel-classes");
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let output = Command::new(&javac)
            .args(["--release", "8", "-encoding", "UTF-8", "-nowarn", "-d"])
            .arg(&out)
            .arg(&kernels)
            .arg(&driver)
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(out)
    })
}

/// The compiled classes, or `None` after saying why there are none.
fn compiled(test: &str) -> Option<&'static Path> {
    match classes() {
        Ok(dir) => Some(dir),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "FASTEMU_REQUIRE_JAVA=1 but {why}"
            );
            eprintln!("skipping {test}: {why}");
            None
        }
    }
}

fn run(ctx: &mut Ctx<'_>, id: usize, scale: i32) -> JResult<i64> {
    let id = Value::Int(i32::try_from(id).expect("twelve kernels"));
    let v = ctx.call_static(
        "fastemu/bench/Driver",
        "run",
        "(II)J",
        &[id, Value::Int(scale)],
    )?;
    Ok(v.long())
}

fn prepare(ctx: &mut Ctx<'_>, id: usize, scale: i32) -> JResult<()> {
    let id = Value::Int(i32::try_from(id).expect("twelve kernels"));
    ctx.call_static(
        "fastemu/bench/Driver",
        "prepare",
        "(II)V",
        &[id, Value::Int(scale)],
    )?;
    Ok(())
}

/// Runs `body` on a Java thread of a new machine with the bench's heap,
/// and returns what it sends, until it is done. Fails when the thread
/// sends nothing for two minutes.
fn on_vm<T: Send + 'static>(
    classes: &Path,
    body: impl FnOnce(&mut Ctx<'_>, &mpsc::Sender<T>) -> JResult<()> + Send + 'static,
) -> Vec<T> {
    let config = VmConfig {
        heap_bytes: HEAP_BYTES,
        log: Arc::new(|line| eprintln!("vm: {line:?}")),
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(DirSource::new(classes)), Vec::new());
    let (tx, rx) = mpsc::channel();
    vm.spawn("kernels", move |ctx| body(ctx, &tx))
        .expect("starting the kernel thread");
    let mut results = Vec::new();
    loop {
        match rx.recv_timeout(Duration::from_secs(120)) {
            Ok(result) => results.push(result),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                vm.stop();
                panic!("a kernel ran for two minutes");
            }
        }
    }
    assert!(
        vm.wait_finished(Some(Duration::from_secs(60))),
        "the kernel thread did not finish"
    );
    results
}

#[test]
fn every_kernel_gives_hotspots_answer_at_scale_1() {
    let Some(classes) = compiled("every_kernel_gives_hotspots_answer_at_scale_1") else {
        return;
    };
    let answers = on_vm(classes, |ctx, tx| {
        for id in 0..KERNELS.len() {
            prepare(ctx, id, 1)?;
            tx.send((id, run(ctx, id, 1)?)).ok();
        }
        Ok(())
    });
    assert_eq!(answers.len(), KERNELS.len(), "every kernel ran");
    for (id, answer) in answers {
        let (name, expected) = KERNELS[id];
        assert_eq!(answer, expected, "{name}");
    }
}

/// The median of `times`, which is not empty.
fn median(times: &mut [f64]) -> f64 {
    times.sort_by(f64::total_cmp);
    let n = times.len();
    if n % 2 == 1 {
        times[n / 2]
    } else {
        f64::midpoint(times[n / 2 - 1], times[n / 2])
    }
}

#[test]
#[ignore = "timing: run in release with --ignored --nocapture"]
fn time_the_kernels() {
    let Some(classes) = compiled("time_the_kernels") else {
        return;
    };
    let only = std::env::var("FASTEMU_KERNELS").ok();
    let picked: Vec<usize> = (0..KERNELS.len())
        .filter(|&id| {
            only.as_deref()
                .is_none_or(|names| names.split(',').any(|n| n.trim() == KERNELS[id].0))
        })
        .collect();
    assert!(!picked.is_empty(), "FASTEMU_KERNELS names no kernel");
    let lines = on_vm(classes, move |ctx, tx| {
        for &id in &picked {
            prepare(ctx, id, SCALE)?;
            let mut per_rep = Vec::with_capacity(ROUNDS);
            let mut first = 0.0;
            let mut check = 0;
            for round in 0..ROUNDS {
                let start = Instant::now();
                check = run(ctx, id, SCALE)?;
                if round == 0 {
                    first = start.elapsed().as_secs_f64() * 1000.0;
                }
                let mut reps = 1u32;
                while start.elapsed() < ROUND {
                    check = run(ctx, id, SCALE)?;
                    reps += 1;
                }
                per_rep.push(start.elapsed().as_secs_f64() * 1000.0 / f64::from(reps));
            }
            let warm = median(&mut per_rep[1..]);
            let line = format!(
                "{:<16} warm {warm:>9.2} ms  first {first:>9.2} ms  check {check}",
                KERNELS[id].0
            );
            tx.send(line).ok();
        }
        Ok(())
    });
    for line in lines {
        println!("{line}");
    }
}
