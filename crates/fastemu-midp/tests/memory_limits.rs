//! What a game can make in host memory is counted against its heap:
//! images, tile layers, canvas back buffers and the surfaces of custom
//! items. A request the heap cannot hold is an `OutOfMemoryError` before
//! any pixels are made, and a game that keeps such objects in a list
//! reaches the limit instead of gigabytes.
//!
//! Each test runs one MIDlet of `tests/memory_java` on a phone whose heap
//! is 1 MiB, and reads what it printed. The tests assert the errors the
//! game sees, not the memory of the process: an error means the request
//! never reached the host. Without a JDK the tests print why and pass,
//! unless `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use fastemu_core::default_profile;
use support::{Running, scratch, zip};

/// The heap of the phone the tests run on.
const HEAP: u64 = 1024 * 1024;

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Memory\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: BigImage, , m.BigImage\r\n";

/// Which `MIDlet-<n>` (from zero) runs each program.
mod midlet {
    pub const BIG_IMAGE: usize = 0;
}

/// The MIDlets, compiled once against the shared stubs and this test's
/// own, or `None` (after saying why) when there is no JDK.
fn classes() -> Option<&'static Path> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let compiled = CLASSES.get_or_init(|| {
        let javac = support::tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let stubs = std::env::join_paths([
            tests.join("memory_java").join("stubs"),
            tests.join("java").join("stubs"),
            tests.join("screens_java").join("stubs"),
        ])
        .expect("joining the stub folders");
        let out = scratch("memory-classes");
        let mut sources: Vec<PathBuf> = std::fs::read_dir(tests.join("memory_java/midlets/m"))
            .expect("the MIDlet sources")
            .map(|e| e.expect("an entry").path())
            .collect();
        sources.sort();
        // The stubs are only read: -implicit:none keeps their classes out
        // of the JAR, so the runtime's classes serve their names.
        let output = Command::new(javac)
            .args(["--release", "8", "-g", "-encoding", "UTF-8", "-nowarn"])
            .args(["-implicit:none", "-sourcepath"])
            .arg(stubs)
            .arg("-d")
            .arg(&out)
            .args(&sources)
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(out)
    });
    match compiled {
        Ok(dir) => Some(dir),
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

/// Packs the MIDlets and `resources` into a JAR in a scratch folder of
/// its own.
fn suite(name: &str, resources: &[(&str, Vec<u8>)]) -> Option<PathBuf> {
    let classes = classes()?;
    let mut entries = vec![(
        "META-INF/MANIFEST.MF".to_owned(),
        MANIFEST.as_bytes().to_vec(),
    )];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("m"))
        .expect("the compiled MIDlets")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let name = format!("m/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((name, std::fs::read(&file).expect("a class file")));
    }
    for (path, data) in resources {
        entries.push(((*path).to_owned(), data.clone()));
    }
    let path = scratch(name).join("memory.jar");
    std::fs::write(&path, zip(&entries, true)).expect("writing the JAR");
    Some(path)
}

/// Starts MIDlet `midlet` on the default phone with a heap of `heap`
/// bytes.
fn launch(jar: &Path, midlet: usize, heap: u64) -> Running {
    let profile = fastemu_core::DeviceProfile {
        heap_bytes: heap,
        ..default_profile()
    };
    Running::launch_on(jar, midlet, profile)
}

/// The numbers after `prefix` in the first printed line that starts with
/// it.
fn numbers(line: &str, prefix: &str) -> Vec<i64> {
    line[prefix.len()..]
        .split(|c: char| !c.is_ascii_digit() && c != '-')
        .filter_map(|n| n.parse().ok())
        .collect()
}

#[test]
fn an_image_the_heap_cannot_hold_is_an_out_of_memory_error_before_its_pixels() {
    let Some(jar) = suite("big-image", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::BIG_IMAGE, HEAP);
    run.wait_log("done ");
    let log = run.dump();
    for line in [
        "big image OutOfMemoryError",
        "huge image OutOfMemoryError",
        // A size the array cannot fill is still the exception MIDP names.
        "rgb image ArrayIndexOutOfBoundsException",
        "empty image IllegalArgumentException",
        "small image 64x64",
    ] {
        assert!(run.has(line), "no {line:?}; the log:\n{log}");
    }
    // The refused requests counted only the errors they threw: the 64 MiB
    // of the image would be far more than the heap.
    let changed = run.wait_log("free changed ");
    let used = numbers(&changed, "free changed ")[0];
    assert!((0..8192).contains(&used), "{changed}; {log}");
}

#[test]
fn images_kept_in_a_list_end_in_out_of_memory_at_the_heap() {
    let Some(jar) = suite("kept-images", &[]) else {
        return;
    };
    let mut run = launch(&jar, midlet::BIG_IMAGE, HEAP);
    let kept = run.wait_log("kept ");
    // 400 by 400 pixels is 640000 bytes of a heap of 1048576.
    assert!(kept.ends_with("then OutOfMemoryError"), "{}", run.dump());
    let made = numbers(&kept, "kept ")[0];
    assert!(
        (1..20).contains(&made),
        "kept {made} images; {}",
        run.dump()
    );
    run.wait_log("done ");
}
