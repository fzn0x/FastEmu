//! Resource lookup (`docs/compat/notes/01-suite-loading.md`): names
//! relative to a class in a package and in the default package, doubled
//! slashes, backslashes, dot segments, the wrong case, leading folders
//! the JAR lacks, and one `read(byte[])` of a large deflated resource.

use crate::conformance_support::{Jar, assert_lines, compile, manifest, scratch};
use crate::support::{Running, test_png};

/// 150 000 bytes that do not compress away: a deflated entry this large
/// inflates in several blocks.
fn big() -> Vec<u8> {
    let mut seed: u32 = 12345;
    (0..150_000)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
            (seed >> 16) as u8
        })
        .collect()
}

/// The hash `Lookup.hash` prints.
fn hash(data: &[u8]) -> i32 {
    data.iter()
        .fold(0i32, |h, &b| h.wrapping_mul(31).wrapping_add(i32::from(b)))
}

#[test]
fn resources_resolve_as_games_expect() {
    let Some(classes) = compile("resources") else {
        return;
    };
    let dir = scratch("resources");
    let data = big();
    let jar = Jar::new()
        .manifest(manifest("r.Lookup", ""))
        .classes(&classes)
        .stored("top.txt", "top")
        .deflated("r/near.txt", "near")
        .deflated("r/sub/deep.txt", "deep")
        .stored("Mixed.TXT", "mixed")
        .deflated("big.bin", data.clone())
        .stored("img/dot.png", test_png())
        .write(&dir.join("resources.jar"));
    let mut run = Running::launch(&jar, 0);
    run.wait_log("lookup done");
    let log = run.lines();
    let got: Vec<&str> = log
        .iter()
        .map(String::as_str)
        .filter(|l| {
            ["res ", "big ", "image ", "lookup "]
                .iter()
                .any(|p| l.starts_with(p))
        })
        .collect();
    let len = data.len();
    let available = format!(
        "big available {len} read {len} then -1 hash {}",
        hash(&data)
    );
    let skipped = format!(
        "big skip 1000 byte {} mark true reset true left {}",
        data[1000],
        len - 1011
    );
    let want = [
        "res near.txt = near",
        "res sub/deep.txt = deep",
        "res /top.txt = top",
        "res top.txt from the default package = top",
        "res //top.txt = top",
        "res /r//sub///deep.txt = deep",
        "res /r\\sub\\deep.txt = deep",
        "res /./r/near.txt = near",
        "res /TOP.TXT = top",
        "res /mixed.txt = mixed",
        "res /res/top.txt = top",
        "res /missing.txt = null",
        "res an object's class = near",
        &available,
        &skipped,
        "image img/dot.png 2x2",
        "image /img/dot.png 2x2",
        "image /img/none.png IOException",
        "lookup done",
    ];
    assert_lines(&got, &want, &log);
    assert!(
        log.iter()
            .any(|l| !l.starts_with("res ") && l.contains("missing.txt")),
        "a missing resource is logged by its name:\n{}",
        log.join("\n")
    );
    assert_eq!(run.instance.error_count(), 0, "{}", log.join("\n"));
}
