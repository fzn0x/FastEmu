//! Repository tasks for FastEmu, run as `cargo xtask <task>`.
//!
//! Tasks:
//!
//! * `changelog`: check, print and fold the changelog fragments of
//!   `changelog.d`. See [`changelog`].

mod changelog;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: cargo xtask changelog [--root DIR] <--check | --print | --release VERSION [--date YYYY-MM-DD]>

  --check              fail when a fragment of changelog.d is invalid
  --print              print the Unreleased section as the next release will have it
  --release VERSION    fold the fragments into a new section for VERSION, and
                       delete them
  --date YYYY-MM-DD    the release date (default: today, UTC)
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

/// The workspace root: two levels above this crate's manifest.
fn default_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|tools| tools.parent())
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
