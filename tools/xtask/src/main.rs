//! Repository tasks for FastEmu, run as `cargo xtask <task>`.
//!
//! Tasks:
//!
//! * `changelog`: check, print and fold the changelog fragments of
//!   `changelog.d`. See [`changelog`].
//! * `icons`: render `assets/icon.svg` into the icons of the release builds
//!   and the Android app. See [`icons`].
//! * `release-notes`: write a GitHub release's notes. See
//!   [`release_notes`].
//! * `stubs`: write Java stubs of every class the runtime registers and
//!   compile them into a jar for test MIDlets. See [`api::stubs`].
//! * `coverage`: compare those classes with the API inventory and write
//!   `docs/compat/coverage.md`. See [`coverage`].
//! * `compat`: run every JAR of a folder without a window and report what
//!   each game did. See [`compat`].
//! * `bench`: measure FastEmu against MicroEmulator 2.0.4 on Windows and
//!   write `bench.md` and `bench.json`. See [`bench`].

mod api;
mod bench;
mod changelog;
mod compat;
mod coverage;
mod icons;
mod release_notes;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
Usage: cargo xtask changelog [--root DIR] <--check | --print | --release VERSION [--date YYYY-MM-DD]>
       cargo xtask icons [--root DIR]
       cargo xtask release-notes [--root DIR] --version X.Y.Z --tag TAG --repo OWNER/NAME
                                 [--previous-tag TAG] [--head REF] [--unreleased] [--debug-apk]
                                 [--out FILE]
       cargo xtask stubs [--root DIR] [--out DIR]
       cargo xtask coverage [--root DIR] [--out FILE]
       cargo xtask compat DIR [--seconds S] [--keys SCRIPT] [--profile ID] [--jobs N]
                          [--out DIR]
       cargo run --release -p xtask -- bench [--microemu DIR] [--games DIR] [--runs N]
                          [--out DIR] [--fastemu-only] [--build-only]
                          [--windows --app PATH] [--me-java PATH] [--me-jvm-arg ARG]...
                          [--me-no-headless-awt] [--me-max-fps N]

changelog:
  --check              fail when a fragment of changelog.d is invalid
  --print              print the Unreleased section as the next release will have it
  --release VERSION    fold the fragments into a new section for VERSION, and
                       delete them
  --date YYYY-MM-DD    the release date (default: today, UTC)

icons:
  renders assets/icon.svg into packaging/icons and the Android launcher icons

release-notes:
  --version X.Y.Z      the version whose CHANGELOG.md section the notes start with
  --tag TAG            the tag the downloads are under
  --repo OWNER/NAME    the GitHub repository
  --previous-tag TAG   the release before, for the Full Changelog link
  --head REF           where that link ends (default: the tag)
  --unreleased         take the Unreleased section, with the fragments, instead
  --debug-apk          say that the APK is signed with a debug key
  --out FILE           write the notes there (default: standard output)

stubs:
  writes a Java stub of every class FastEmu registers, with throws clauses
  from docs/compat/api, and compiles them with the JDK's javac into
  fastemu-stubs.jar, for test MIDlets to compile against
  --out DIR            the folder (default: fastemu-stubs in the target folder)

coverage:
  compares the classes FastEmu registers with docs/compat/api and writes
  which classes and members of each tier are there and which are missing
  --out FILE           the report (default: docs/compat/coverage.md)

compat:
  runs every .jar under DIR without a window, each in a process of its own,
  and writes compat.md and compat.json, with a screenshot and the log of
  each game, to the output folder
  --seconds S          how long each game runs (default: 20)
  --keys SCRIPT        the keys to press from the first frame, as fastemu run
                       takes them (default: 5 after a pause, then the
                       arrows and fire)
  --profile ID         the phone profile (default: generic-240x320)
  --jobs N             how many games run at once (default: 1)
  --out DIR            the folder (default: compat in the target folder)

bench:
  measures FastEmu against MicroEmulator 2.0.4 on Windows, with no window,
  and writes bench.md and bench.json to the output folder; only a release
  build measures (docs/benchmark.md has the method)
  --microemu DIR       the unpacked microemulator-2.0.4 folder; needed
                       unless --fastemu-only or --build-only
  --games DIR          the JARs of DIR (not its subfolders) run as real games
  --runs N             how many times each measurement runs (default: 4)
  --out DIR            the folder (default: bench in the target folder); it
                       must be new, empty, or an earlier bench's
  --fastemu-only       measure FastEmu alone; MicroEmulator shows as not run
  --build-only         build the benchmark MIDlet and MeBench, run nothing;
                       works in any build and on any system
  --windows            measure the desktop app and MicroEmulator's windows
                       instead; opens windows, needs --app and --games
  --app PATH           the desktop app's fastemu.exe, for --windows
  --me-java PATH       the java that runs MicroEmulator (default: JAVA_HOME's)
  --me-jvm-arg ARG     a JVM option for every MicroEmulator process; repeat
                       it for more
  --me-no-headless-awt
                       start MicroEmulator without java.awt.headless
  --me-max-fps N       also measure MicroEmulator capped at N frames a second

  --root DIR           the repository's root (default: the workspace root)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("changelog") => run_changelog(&args[1..]),
        Some("icons") => run_icons(&args[1..]),
        Some("release-notes") => run_release_notes(&args[1..]),
        Some("stubs") => run_stubs(&args[1..]),
        Some("coverage") => run_coverage(&args[1..]),
        Some("compat") => run_compat(&args[1..]),
        // One game of `compat`, in the process `compat` starts for it.
        Some("compat-run") => {
            let options = compat::child::Options::parse(&args[1..])?;
            compat::child::run(&options, &mut std::io::stdout().lock())
        }
        Some("bench") => run_bench(&args[1..]),
        // FastEmu's side of one bench measurement, in the process `bench`
        // starts for it.
        Some("bench-run") => {
            let options = bench::child::Options::parse(&args[1..])?;
            bench::child::run(&options, &mut std::io::stdout().lock())
        }
        _ => Err(USAGE.to_string()),
    }
}

fn run_changelog(args: &[String]) -> Result<(), String> {
    let mut root = default_root();
    let mut mode = None;
    let mut date = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--check" => mode = Some(changelog::Mode::Check),
            "--print" => mode = Some(changelog::Mode::Print),
            "--release" => mode = Some(changelog::Mode::Release(value("--release")?)),
            "--date" => date = Some(value("--date")?),
            "--root" => root = PathBuf::from(value("--root")?),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let Some(mode) = mode else {
        return Err(USAGE.to_string());
    };
    changelog::run(&root, &mode, date.as_deref())
}

fn run_icons(args: &[String]) -> Result<(), String> {
    let mut root = default_root();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--root" => {
                let dir = iter
                    .next()
                    .ok_or_else(|| format!("--root needs a value\n{USAGE}"))?;
                root = PathBuf::from(dir);
            }
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    icons::run(&root)
}

fn run_release_notes(args: &[String]) -> Result<(), String> {
    let mut root = default_root();
    let (mut version, mut tag, mut repo) = (None, None, None);
    let (mut previous_tag, mut head, mut out) = (None, None, None);
    let (mut unreleased, mut debug_apk) = (false, false);
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        // An empty value, as a workflow passes when there is none, is none.
        let nonempty = |v: String| Some(v).filter(|v| !v.is_empty());
        match arg.as_str() {
            "--version" => version = Some(value("--version")?),
            "--tag" => tag = Some(value("--tag")?),
            "--repo" => repo = Some(value("--repo")?),
            "--previous-tag" => previous_tag = nonempty(value("--previous-tag")?),
            "--head" => head = nonempty(value("--head")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--unreleased" => unreleased = true,
            "--debug-apk" => debug_apk = true,
            "--root" => root = PathBuf::from(value("--root")?),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let (Some(version), Some(tag), Some(repo)) = (version, tag, repo) else {
        return Err(format!(
            "release-notes needs --version, --tag and --repo\n{USAGE}"
        ));
    };
    let options = release_notes::Options {
        version,
        tag,
        repo,
        previous_tag,
        head,
        unreleased,
        debug_apk,
    };
    let notes = release_notes::build(&root, &options)?;
    match out {
        Some(path) => fs::write(&path, notes).map_err(|e| format!("{}: {e}", path.display())),
        None => {
            print!("{notes}");
            Ok(())
        }
    }
}

fn run_stubs(args: &[String]) -> Result<(), String> {
    let mut root = default_root();
    let mut out = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--root" => root = PathBuf::from(value("--root")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let out = out.unwrap_or_else(|| target_dir(&root).join("fastemu-stubs"));
    let built = api::stubs::build(&out, &root.join("docs/compat/api"))?;
    for warning in &built.warnings {
        eprintln!("warning: {warning}");
    }
    let what = if built.compiled {
        "compiled"
    } else {
        "unchanged"
    };
    println!(
        "{} stub classes, {what}: {}",
        built.classes,
        built.jar.display()
    );
    Ok(())
}

fn run_coverage(args: &[String]) -> Result<(), String> {
    let mut root = default_root();
    let mut out = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--root" => root = PathBuf::from(value("--root")?),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    let out = out.unwrap_or_else(|| root.join("docs/compat/coverage.md"));
    coverage::run(&root, &out)
}

fn run_compat(args: &[String]) -> Result<(), String> {
    let mut dir = None;
    let mut out = None;
    let mut seconds = 20.0;
    let mut keys = compat::DEFAULT_KEYS.to_owned();
    let mut profile = "generic-240x320".to_owned();
    let mut jobs = 1;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{name} needs a value\n{USAGE}"))
        };
        match arg.as_str() {
            "--seconds" => {
                let s = value("--seconds")?;
                seconds = s
                    .parse::<f64>()
                    .ok()
                    .filter(|s| s.is_finite() && *s > 0.0)
                    .ok_or_else(|| format!("bad number of seconds {s}"))?;
            }
            "--keys" => keys = value("--keys")?,
            "--profile" => profile = value("--profile")?,
            "--jobs" => {
                let n = value("--jobs")?;
                jobs = n.parse().map_err(|_| format!("bad number of jobs {n}"))?;
            }
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            other if other.starts_with("--") => {
                return Err(format!("unknown argument {other}\n{USAGE}"));
            }
            other if dir.is_none() => dir = Some(PathBuf::from(other)),
            other => return Err(format!("give one folder, not also {other}\n{USAGE}")),
        }
    }
    let dir = dir.ok_or_else(|| format!("compat needs a folder of JARs\n{USAGE}"))?;
    let options = compat::Options {
        out: out.unwrap_or_else(|| target_dir(&default_root()).join("compat")),
        dir,
        seconds,
        keys,
        profile,
        jobs,
    };
    compat::run(&options)
}

fn run_bench(args: &[String]) -> Result<(), String> {
    let root = default_root();
    let out = target_dir(&root).join("bench");
    let options = bench::Options::parse(args, out).map_err(|e| format!("{e}\n{USAGE}"))?;
    bench::run(&options, &root)
}

/// Cargo's target folder: `CARGO_TARGET_DIR`, else the folder this
/// program was built in, else `target` in the workspace.
fn target_dir(root: &Path) -> PathBuf {
    if let Some(dir) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(dir);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.parent()?.to_path_buf()))
        .unwrap_or_else(|| root.join("target"))
}

/// The workspace root: two levels above this crate's manifest.
fn default_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|tools| tools.parent())
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
