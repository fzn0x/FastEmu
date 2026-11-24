//! Record stores (`docs/compat/notes/06-rms.md`): ids that start at 1 and
//! are never reused, a record with no data, enumeration in ascending id
//! order and with a filter and a comparator, the open count, reopening,
//! and a second launch of the suite that finds everything as it was left.

use fastemu_core::InstanceStatus;

use crate::conformance_support::{Jar, assert_lines, compile, manifest, scratch, tagged};
use crate::support::Running;

/// Runs the MIDlet until it exits and returns its `rms` lines and log.
fn run_once(jar: &std::path::Path, last: &str) -> (Vec<String>, Vec<String>) {
    let mut run = Running::launch(jar, 0);
    run.wait_log(last);
    run.wait_status(InstanceStatus::is_finished);
    let log = run.lines();
    assert_eq!(run.instance.error_count(), 0, "{}", log.join("\n"));
    let lines = tagged(&log, "rms ")
        .iter()
        .map(|l| (*l).to_owned())
        .collect();
    (lines, log)
}

#[test]
fn records_keep_their_ids_across_closes_and_launches() {
    let Some(classes) = compile("records") else {
        return;
    };
    let dir = scratch("records");
    let jar = Jar::new()
        .manifest(manifest("s.Records", ""))
        .classes(&classes)
        .write(&dir.join("records.jar"));

    let (lines, log) = run_once(&jar, "rms phase 1 done");
    let got: Vec<&str> = lines.iter().map(String::as_str).collect();
    let want = [
        "phase 1",
        "missing store: RecordStoreNotFoundException",
        "new store: records 0 next 1",
        "added 1 2 3",
        "after deleting 3: next 4",
        "re-added 4",
        "deleted 2: InvalidRecordIDException",
        "empty record 5 reads null",
        "enumerated 1 4 5",
        "sorted 5 1 4",
        "filtered 4",
        "opened twice: same object true",
        "closed once: records 3",
        "closed twice: RecordStoreNotOpenException",
        "reopened: new object true records 3 next 6 record 4 four",
        "deleting it while open: RecordStoreException",
        "stores save",
        "phase 1 done",
    ];
    assert_lines(&got, &want, &log);
    let data = jar.with_extension("data");
    assert!(
        data.join("rms/save.rms").is_file(),
        "the store is a file under the instance's data folder"
    );
    assert!(
        !data.join("rms/scratch.rms").exists(),
        "a deleted store leaves no file"
    );

    let (lines, log) = run_once(&jar, "rms phase 2 done");
    let got: Vec<&str> = lines.iter().map(String::as_str).collect();
    let want = [
        "phase 2",
        "stores save",
        "kept: records 3 next 6 enumerated 1 4 5",
        "record 1 one record 4 four",
        "added 6",
        "phase 2 done",
    ];
    assert_lines(&got, &want, &log);
}
