//! Natives allocate without collecting, so what they make is counted at
//! once and judged when they return: a game that fills memory through
//! natives alone gets `OutOfMemoryError` within a few times the heap
//! limit, as one that fills it with `new` does.
//!
//! The cases are Java programs in `tests/heap_java/`, compiled once with
//! `javac --release 8`; the JDK comes from `JAVA_HOME` or the `PATH`.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it). Every case runs on a
//! 256 KiB heap (the smallest the heap allows is 64 KiB) and loops a
//! bounded number of times, so even a machine that never throws holds
//! only a few megabytes.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use fastemu_jvm::{HeapStats, MemorySource, Vm, VmConfig};

const HEAP: usize = 256 * 1024;

/// The compiled classes, as a source to build machines from.
struct Compiled {
    source: MemorySource,
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

/// Compiles `HeapCases.java` once; `Err` says why the JDK is unusable.
fn compiled() -> &'static Result<Compiled, String> {
    static COMPILED: OnceLock<Result<Compiled, String>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("heap-limits-classes");
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        let source_file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("heap_java")
            .join("HeapCases.java");
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
        // A resource of 64 KiB.
        source.add_resource("/big.bin", vec![7; 64 * 1024]);
        Ok(Compiled { source })
    })
}

/// Runs `body` with a machine on `heap` bytes, or says why it cannot.
fn with_vm(heap: usize, body: impl FnOnce(&Vm)) {
    let compiled = match compiled() {
        Ok(compiled) => compiled,
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "FASTEMU_REQUIRE_JAVA=1 but {why}"
            );
            eprintln!("skipping: {why}");
            return;
        }
    };
    let config = VmConfig {
        heap_bytes: heap,
        log: Arc::new(|_| {}),
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(compiled.source.clone()), Vec::new());
    body(&vm);
    vm.stop();
    vm.wait_finished(Some(std::time::Duration::from_secs(10)));
}

/// Calls `HeapCases.<method>()I` and returns its value.
fn call(vm: &Vm, method: &str) -> i32 {
    vm.attach("test", |ctx| {
        match ctx.call_static("HeapCases", method, "()I", &[]) {
            Ok(v) => v.int(),
            Err(thrown) => panic!("HeapCases.{method} threw {thrown:?} out of the program"),
        }
    })
}

fn stats(vm: &Vm) -> HeapStats {
    vm.heap_stats()
}

/// A case that fills memory in rounds of about `round_bytes`: it ends in
/// `OutOfMemoryError` after at most three heap limits, the heap count
/// stays under three limits, and a game that lets go of its data
/// allocates normally again.
fn fills_memory(method: &str, round_bytes: usize) {
    fills(method, round_bytes, true);
}

/// As [`fills_memory`]; `recovers` says whether the game can allocate
/// afterwards, which it cannot when what it made is a root for good.
fn fills(method: &str, round_bytes: usize, recovers: bool) {
    with_vm(HEAP, |vm| {
        let round = call(vm, method);
        assert!(
            round >= 0,
            "{method} ran to the end without OutOfMemoryError"
        );
        assert!(
            (round as usize) * round_bytes <= 3 * HEAP,
            "{method} held {round} rounds of {round_bytes} bytes before it threw"
        );
        let held = stats(vm);
        assert!(
            held.used_bytes < 3 * HEAP,
            "{method} left {} bytes counted on a heap of {HEAP}",
            held.used_bytes
        );
        if recovers {
            assert!(
                call(vm, "recover") > 0,
                "{method}: a game that let go of its data could not allocate"
            );
        }
    });
}

#[test]
fn doubling_a_string_with_concat_ends_in_out_of_memory_error() {
    with_vm(HEAP, |vm| {
        let round = call(vm, "concat");
        // 2^17 characters are 256 KiB, the whole heap.
        assert!((12..=17).contains(&round), "threw in round {round}");
        assert!(stats(vm).used_bytes < 3 * HEAP);
        assert!(call(vm, "recover") > 0);
    });
}

#[test]
fn appending_to_one_buffer_ends_in_out_of_memory_error() {
    // 32 characters are 64 bytes.
    fills_memory("append", 64);
}

#[test]
fn many_buffers_grown_by_set_length_end_in_out_of_memory_error() {
    // Each buffer is 5000 characters, 10000 bytes.
    fills_memory("buffers", 10_000);
}

#[test]
fn kept_exceptions_end_in_out_of_memory_error() {
    // A trace of 101 frames is 1616 bytes.
    fills_memory("exceptions", 1616);
}

#[test]
fn dropped_exceptions_are_collected_as_they_are_counted() {
    with_vm(HEAP, |vm| {
        assert_eq!(call(vm, "exceptionGarbage"), 2000);
        let s = stats(vm);
        // 2000 traces of 256 frames are 8 MB; none of it may pile up.
        assert!(s.collections >= 10, "{} collections", s.collections);
        assert!(s.used_bytes < HEAP, "{} bytes counted", s.used_bytes);
    });
}

#[test]
fn kept_resource_streams_end_in_out_of_memory_error() {
    fills_memory("resources", 64 * 1024);
}

#[test]
fn kept_char_arrays_end_in_out_of_memory_error() {
    // 8000 characters are 16000 bytes.
    fills_memory("chars", 16_000);
}

#[test]
fn kept_array_clones_end_in_out_of_memory_error() {
    fills_memory("clones", 100_000);
}

#[test]
fn interned_strings_end_in_out_of_memory_error() {
    // An interned string of up to five digits and its table entry are
    // about 80 bytes at the most. The table keeps every string for good,
    // so the game cannot allocate again afterwards, as on a phone.
    fills("intern", 80, false);
}

#[test]
fn thread_names_are_cut_and_counted() {
    with_vm(1024 * 1024, |vm| {
        let longest = call(vm, "threadNames");
        assert_eq!(longest, 255, "the longest name any of the threads reports");
        assert!(stats(vm).used_bytes < 1024 * 1024);
    });
}

#[test]
fn only_the_thread_whose_native_overshot_gets_the_error() {
    with_vm(HEAP, |vm| {
        let outcome = call(vm, "pair");
        assert_eq!(
            outcome, 0,
            "bit 1: the allocating thread missed an error; bit 2: the other thread got one; \
             bit 4: the other thread did not run"
        );
    });
}

#[test]
fn a_native_that_asks_for_more_than_the_heap_fails_before_it_allocates() {
    with_vm(HEAP, |vm| {
        let thrown = vm.attach("test", |ctx| {
            let mut thrown = Vec::new();
            for bytes in [HEAP + 1, 3 * HEAP, usize::MAX] {
                match ctx.check_alloc(bytes) {
                    Err(fastemu_jvm::Throw::Exception(e)) => thrown.push(ctx.describe_exception(e)),
                    other => thrown.push(format!("{bytes}: {other:?}")),
                }
            }
            (thrown, ctx.check_alloc(HEAP).is_ok())
        });
        for t in &thrown.0 {
            assert!(
                t.starts_with("java.lang.OutOfMemoryError: Java heap space"),
                "{t}"
            );
        }
        assert!(thrown.1, "the whole heap may be asked for when it is empty");
    });
}

#[test]
fn a_payload_that_grew_is_counted_when_the_native_says_so() {
    use fastemu_jvm::{NativeClass, Payload};

    #[derive(Default)]
    struct Blob(usize);

    impl Payload for Blob {
        fn heap_bytes(&self) -> usize {
            self.0
        }
    }

    let blob = NativeClass::new("test/Blob").payload::<Blob>();
    let config = VmConfig {
        heap_bytes: HEAP,
        log: Arc::new(|_| {}),
        ..VmConfig::default()
    };
    let vm = Vm::new(config, Arc::new(MemorySource::new()), vec![blob]);
    let (before, quiet, counted) = vm.attach("test", |ctx| {
        let class = ctx.find_class("test/Blob").unwrap();
        let obj = ctx.new_object(class).unwrap();
        let (_, before) = ctx.heap_usage();
        ctx.payload_mut::<Blob>(obj).unwrap().0 = 100_000;
        let (_, quiet) = ctx.heap_usage();
        ctx.recount(obj);
        let (_, counted) = ctx.heap_usage();
        (before, quiet, counted)
    });
    assert_eq!(quiet, before, "nothing is counted until the native says so");
    assert_eq!(counted, before + 100_000);
}
