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

mod changelog;
mod icons;
mod release_notes;

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage: cargo xtask changelog [--root DIR] <--check | --print | --release VERSION [--date YYYY-MM-DD]>
       cargo xtask icons [--root DIR]
       cargo xtask release-notes [--root DIR] --version X.Y.Z --tag TAG --repo OWNER/NAME
                                 [--previous-tag TAG] [--head REF] [--unreleased] [--debug-apk]
                                 [--out FILE]

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

/// The workspace root: two levels above this crate's manifest.
fn default_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|tools| tools.parent())
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}
