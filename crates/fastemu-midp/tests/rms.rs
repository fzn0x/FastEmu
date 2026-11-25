//! Record stores through the virtual machine: the Java programs in
//! `tests/rms_java/` run on FastEmu with an instance environment in a
//! temporary folder, and print what they see.
//!
//! The programs compile once with `javac --release 8` against stubs of
//! `javax.microedition.rms` (in `tests/rms_java/stubs/`), which stay out
//! of the class folder the virtual machine loads. Without a JDK the tests
//! print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use fastemu_core::{LogLevel, LogLine, SuiteInfo};
use fastemu_jvm::{Ctx, DirSource, Vm, VmConfig};
use fastemu_midp::env::InstanceEnv;
use fastemu_midp::rms::disk;
use fastemu_midp::rms::format::StoreData;
use fastemu_midp::rms::name::StoreName;

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
    for entry in std::fs::read_dir(dir).expect("reading a test source folder") {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            files.extend(java_files(&path));
        } else if path.extension().is_some_and(|x| x == "java") {
            files.push(path);
        }
    }
    files.sort();
    files
}

fn javac(javac: &Path, out: &Path, classpath: Option<&Path>, sources: &[PathBuf]) {
    std::fs::create_dir_all(out).expect("making a class folder");
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

/// The folder of compiled test programs, or `None` without a JDK.
fn programs() -> Option<&'static Path> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let compiled = CLASSES.get_or_init(|| {
        let javac_path = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("rms_java");
        let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rms-java");
        let _ = std::fs::remove_dir_all(&root);
        let stubs = root.join("stubs");
        let classes = root.join("classes");
        javac(&javac_path, &stubs, None, &java_files(&src.join("stubs")));
        let programs: Vec<PathBuf> = java_files(&src)
            .into_iter()
            .filter(|p| !p.starts_with(src.join("stubs")))
            .collect();
        javac(&javac_path, &classes, Some(&stubs), &programs);
        Ok(classes)
    });
    match compiled {
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

/// What one program printed.
struct Output {
    out: String,
    err: String,
    warnings: Vec<String>,
}

/// An environment for suite `name` of `vendor` keeping its data in `dir`.
fn suite_env(dir: &Path, vendor: &str, name: &str) -> InstanceEnv {
    let mut env = InstanceEnv::for_tests(dir);
    env.suite = Arc::new(SuiteInfo {
        name: name.to_owned(),
        vendor: vendor.to_owned(),
        ..SuiteInfo::default()
    });
    env
}

/// Runs `main` of `class` with `args` in a new virtual machine for `env`;
/// `setup` runs first, with the environment installed.
fn run(
    classes: &Path,
    class: &str,
    args: &[&str],
    env: InstanceEnv,
    setup: impl FnOnce(&mut Ctx<'_>),
) -> Output {
    let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let sink = Arc::clone(&lines);
    let config = VmConfig {
        log: Arc::new(move |line| sink.lock().unwrap().push(line)),
        ..VmConfig::default()
    };
    let vm = Vm::new(
        config,
        Arc::new(DirSource::new(classes)),
        fastemu_midp::classes(&fastemu_core::default_profile()),
    );
    vm.attach("setup", |ctx| {
        ctx.set_extension(env);
        setup(ctx);
    });
    vm.run_main(class, args).expect("starting the main thread");
    if !vm.wait_finished(Some(Duration::from_secs(60))) {
        vm.stop();
        vm.wait_finished(Some(Duration::from_secs(5)));
        panic!("{class} did not finish within a minute");
    }
    vm.flush_output();
    let lines = lines.lock().unwrap();
    let pick = |level: LogLevel| {
        lines
            .iter()
            .filter(|l| l.level == level)
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    Output {
        out: pick(LogLevel::Out),
        err: pick(LogLevel::Err),
        warnings: lines
            .iter()
            .filter(|l| l.level == LogLevel::Warn)
            .map(|l| l.text.clone())
            .collect(),
    }
}

/// Checks a program's output line by line against `expected`.
fn check(output: &Output, expected: &str) {
    assert!(output.err.is_empty(), "the program failed:\n{}", output.err);
    let expected: Vec<&str> = expected
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let actual: Vec<&str> = output.out.lines().collect();
    for (i, (e, a)) in expected.iter().zip(&actual).enumerate() {
        assert_eq!(
            a,
            e,
            "line {} differs; the output was:\n{}",
            i + 1,
            output.out
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "the output has a different length:\n{}",
        output.out
    );
}

fn name(text: &str) -> StoreName {
    StoreName::from_text(text).unwrap()
}

/// Reads a store file as FastEmu would.
fn read_store(dir: &Path, store: &str) -> StoreData {
    let path = disk::store_path(dir, &name(store));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let loaded = StoreData::decode(&bytes, &name(store), 0);
    assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
    loaded.data
}

/// Writes a store file as FastEmu would.
fn write_store(dir: &Path, data: &StoreData) {
    disk::write_atomic(&disk::store_path(dir, &data.name), &data.encode()).unwrap();
}

#[test]
fn ids_records_and_exceptions() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        classes,
        "RmsBasics",
        &[],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &output,
        r"
        stores null
        missing RecordStoreNotFoundException
        name save records 0 next 1
        ids 1 2 3
        version grew 3
        record 1 alpha
        record 2 2 2 3
        record 3 null size 0
        after delete next 4 count 2
        id after delete 4
        deleted InvalidRecordIDException
        zero InvalidRecordIDException
        delete twice InvalidRecordIDException
        set missing InvalidRecordIDException
        set longer value size 12
        into 12 longer value
        small buffer ArrayIndexOutOfBoundsException
        late offset ArrayIndexOutOfBoundsException
        bad range ArrayIndexOutOfBoundsException
        null data NullPointerException
        count 3
        size true available true
        modified true
        same object true
        still open 3
        delete open RecordStoreException
        closed RecordStoreNotOpenException
        close twice RecordStoreNotOpenException
        add closed RecordStoreNotOpenException
        new object true records 3 next 5
        empty name IllegalArgumentException
        long name IllegalArgumentException
        case sensitive 0
        stores 12345678901234567890123456789012 SAVE con:/\*?\u5b58 save
        delete missing RecordStoreNotFoundException
        deleted store RecordStoreNotFoundException
        bad mode IllegalArgumentException
        bad setMode IllegalArgumentException
        setMode keeps the version true
        stores 12345678901234567890123456789012 con:/\*?\u5b58 save shared
        ",
    );
    // Every store is one file with a portable name.
    let rms = disk::rms_dir(dir.path());
    let mut files: Vec<String> = std::fs::read_dir(&rms)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(
        files,
        [
            "12345678901234567890123456789012.rms",
            "con_003a_002f_005c_002a_003f_5b58.rms",
            "save.rms",
            "shared.rms",
        ]
    );
    let save = read_store(&rms, "save");
    assert_eq!(save.next_id, 5);
    assert_eq!(save.records.keys().copied().collect::<Vec<_>>(), [1, 3, 4]);
    let shared = read_store(&rms, "shared");
    assert!(!shared.shared && !shared.writable);
}

#[test]
fn enumerations_filter_sort_and_follow_changes() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        classes,
        "RmsEnum",
        &[],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &output,
        "
        all 6: 1,2,3,4,5,6
        sorted 6,2,4,5,3,1
        filtered 5,3,1
        filter only 1,3,5
        descending 1,3,5
        previous first 1
        next 5 has previous false
        next 3 previous 5
        data 40
        last 1 has next false
        end InvalidRecordIDException
        still end InvalidRecordIDException
        back 4
        after reset 5
        stale count 6
        stale record InvalidRecordIDException
        stale next 2
        rebuilt 5: 2,3,4,5,6
        live true 5,3
        added 7 8
        live after add 5,7,3
        live after set 7,3,5,2
        live after delete 7,5,2
        frozen 7,5,2 false
        walk 2 4
        walk on 5 of 6
        walk after delete 6 back 4
        destroyed IllegalStateException
        destroyed twice
        store goes on 6
        closed has next false count 6
        closed RecordStoreNotOpenException
        enumerate closed RecordStoreNotOpenException
        update each 1,2,3
        update sorted 1,2,3
        ",
    );
}

#[test]
fn listeners_run_on_the_changing_thread() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        classes,
        "RmsListen",
        &[],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &output,
        "
        a added 1 to events, which has 1 records
        b added 1 to events, which has 1 records
        a changed 1
        b changed 1
        b deleted 1
        listener threw boom, the record stays: 1
        b added 2 to events, which has 1 records
        b on the writing thread true
        done with 3 records
        ",
    );
}

#[test]
fn stores_survive_a_restart() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let write = run(
        classes,
        "RmsPersist",
        &["write"],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &write,
        r"
        written version 5 next 5
        stores Progress progress \u5b58\u6863
        ",
    );
    // An instance killed in the middle of a write leaves its temporary file
    // behind; the store keeps what it held before.
    let rms = disk::rms_dir(dir.path());
    std::fs::write(rms.join("progress.tmp"), b"FERS\x01\x00 half a store").unwrap();
    let read = run(
        classes,
        "RmsPersist",
        &["read"],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &read,
        r"
        records 3 version 5 next 5
        1 level 3, 2 score 900, 4 null
        new id 5
        Progress other
        unicode 42
        temp RecordStoreNotFoundException
        stores Progress progress \u5b58\u6863
        ",
    );
    // The next write replaced the leftover.
    assert!(!rms.join("progress.tmp").exists());
    assert_eq!(read_store(&rms, "progress").next_id, 6);
    // A second instance has its own folder and sees none of it.
    let other = tempfile::tempdir().unwrap();
    let list = run(
        classes,
        "RmsPersist",
        &["list"],
        InstanceEnv::for_tests(other.path()),
        |_| {},
    );
    check(&list, "stores null");
}

#[test]
fn threads_add_records_without_losing_any() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        classes,
        "RmsPersist",
        &["threads"],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &output,
        "
        busy 100 next 101
        stores busy
        ",
    );
    let busy = read_store(&disk::rms_dir(dir.path()), "busy");
    assert_eq!(busy.records.len(), 100);
    assert_eq!(busy.next_id, 101);
    for t in 0..4u8 {
        let mut seen: Vec<u8> = busy
            .records
            .values()
            .filter(|data| data[0] == t)
            .map(|data| data[1])
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..25).collect::<Vec<u8>>(), "thread {t}");
    }
}

#[test]
fn other_suites_stores_open_by_vendor_and_suite() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let theirs = disk::suite_dir(dir.path(), "Other Vendor", "Other Game");
    let mut scores = StoreData::new(name("scores"), 0);
    scores.add(b"top 100".to_vec(), 0);
    scores.shared = true;
    write_store(&theirs, &scores);
    write_store(&theirs, &StoreData::new(name("private"), 0));
    let mut board = StoreData::new(name("board"), 0);
    board.shared = true;
    board.writable = true;
    write_store(&theirs, &board);

    let output = run(
        classes,
        "RmsShared",
        &[],
        suite_env(dir.path(), "Me", "Mine"),
        |_| {},
    );
    check(
        &output,
        "
        shared top 100 records 1
        read-only add SecurityException
        read-only delete SecurityException
        setMode SecurityException
        same object true
        private SecurityException
        nothing RecordStoreNotFoundException
        unknown suite RecordStoreNotFoundException
        writable board adds 1
        own by name true
        own absent RecordStoreNotFoundException
        null vendor IllegalArgumentException
        stores mine
        ",
    );
    assert_eq!(read_store(&theirs, "scores"), scores);
    assert_eq!(&*read_store(&theirs, "board").records[&1], b"mine");
}

#[test]
fn damaged_files_open_and_the_quota_holds() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let rms = disk::rms_dir(dir.path());
    std::fs::create_dir_all(&rms).unwrap();
    let broken = disk::store_path(&rms, &name("broken"));
    std::fs::write(&broken, b"this is not a record store").unwrap();
    let mut cut = StoreData::new(name("cut"), 0);
    cut.add(b"kept".to_vec(), 0);
    cut.add(vec![7; 100], 0);
    let bytes = cut.encode();
    std::fs::write(
        disk::store_path(&rms, &name("cut")),
        &bytes[..bytes.len() - 50],
    )
    .unwrap();

    let output = run(
        classes,
        "RmsLimits",
        &[],
        InstanceEnv::for_tests(dir.path()),
        |ctx| fastemu_midp::rms::set_quota(ctx, 4096),
    );
    check(
        &output,
        "
        stores broken cut
        broken opens with 0 records, next 1
        broken takes record 1
        cut keeps 1: kept, next 3
        available true
        add RecordStoreFullException, records 1
        set RecordStoreFullException, size 2000
        shrinking frees 1990
        ",
    );
    // The damage was logged, the damaged files were kept, and the first
    // change replaced the broken file with a sound one.
    assert!(
        output.warnings.iter().any(|w| w.contains("broken")),
        "{:?}",
        output.warnings
    );
    assert_eq!(
        std::fs::read(broken.with_extension("bad")).unwrap(),
        b"this is not a record store"
    );
    assert!(rms.join("cut.bad").exists());
    assert_eq!(read_store(&rms, "broken").records.len(), 1);
}

#[test]
fn a_suite_has_at_most_64_stores() {
    let Some(classes) = programs() else { return };
    let dir = tempfile::tempdir().unwrap();
    let output = run(
        classes,
        "RmsStores",
        &[],
        InstanceEnv::for_tests(dir.path()),
        |_| {},
    );
    check(
        &output,
        "
        made 64
        65th RecordStoreFullException, 64 stores
        s0 holds 1 record
        nothing RecordStoreNotFoundException
        deleted, 63 stores
        again made, 64 stores
        one more RecordStoreFullException, 64 stores
        done
        ",
    );
    // The refused stores left no file behind.
    let files = std::fs::read_dir(disk::rms_dir(dir.path()))
        .unwrap()
        .count();
    assert_eq!(files, 64);
}
