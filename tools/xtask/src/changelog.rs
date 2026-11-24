//! The changelog's fragments.
//!
//! A change people will notice adds a file to `changelog.d`, in the folder of
//! its section, instead of lines to `CHANGELOG.md`. Pull requests that are
//! open at the same time then never change the same lines.
//! `changelog.d/fixed/101-stream-retry.md` holds one entry: one Markdown list
//! item or more, written as `CHANGELOG.md` has them.
//!
//! A release folds the fragments into `## [Unreleased]`, after the entries
//! already there, renames that section to the version, adds a fresh empty
//! `## [Unreleased]` above it, and deletes the fragments.

use std::cmp::Ordering;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What the `changelog` task does.
pub enum Mode {
    /// Fail when a fragment is invalid.
    Check,
    /// Print `## [Unreleased]` with the fragments folded in, changing nothing.
    Print,
    /// Fold the fragments into a section for this version, and delete them.
    Release(String),
}

/// The sections of Keep a Changelog, in its order: the folder of
/// `changelog.d` and the heading its fragments fold under.
const SECTIONS: [(&str, &str); 6] = [
    ("added", "Added"),
    ("changed", "Changed"),
    ("deprecated", "Deprecated"),
    ("removed", "Removed"),
    ("fixed", "Fixed"),
    ("security", "Security"),
];

/// The heading of the section the fragments fold into.
const UNRELEASED: &str = "## [Unreleased]";

/// An entry waiting in `changelog.d`.
#[derive(Debug, PartialEq, Eq)]
struct Fragment {
    /// The name messages use, as `changelog.d/fixed/101-stream-retry.md`.
    name: String,
    file: PathBuf,
    heading: &'static str,
    /// The entry without its trailing newline.
    text: String,
}

/// Runs the task for the repository at `root`.
pub fn run(root: &Path, mode: &Mode, date: Option<&str>) -> Result<(), String> {
    let fragments = read_fragments(&root.join("changelog.d"))
        .map_err(|errors| format!("invalid fragments:\n{}", errors.join("\n")))?;
    let file = root.join("CHANGELOG.md");
    let changelog = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let (folded, notes) = fold(&changelog, &fragments)?;
    match mode {
        Mode::Check => println!("ok: {} fragments", fragments.len()),
        Mode::Print => print!("{notes}"),
        Mode::Release(version) => {
            let date = match date {
                Some(date) => date.to_string(),
                None => today(),
            };
            let released = release(&folded, version, &date)?;
            fs::write(&file, released).map_err(|e| format!("{}: {e}", file.display()))?;
            for fragment in &fragments {
                fs::remove_file(&fragment.file)
                    .map_err(|e| format!("{}: {e}", fragment.file.display()))?;
            }
            // A section's folder goes with its last fragment. One that
            // keeps other files stays, which is why errors are ignored.
            for (folder, _) in SECTIONS {
                let _ = fs::remove_dir(root.join("changelog.d").join(folder));
            }
            println!(
                "folded {} fragments into CHANGELOG.md as {version}",
                fragments.len()
            );
        }
    }
    Ok(())
}

/// Reads and checks the fragments of `dir`: section by section, each in the
/// order of the issue number its name starts with. `dir` may also hold a
/// `README.md`, and may be missing.
fn read_fragments(dir: &Path) -> Result<Vec<Fragment>, Vec<String>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(vec![format!("{}: {e}", dir.display())]),
    };
    let folders: Vec<&str> = SECTIONS.iter().map(|(folder, _)| *folder).collect();
    let mut by_folder: Vec<Vec<Fragment>> = SECTIONS.iter().map(|_| Vec::new()).collect();
    let mut errors = Vec::new();
    let mut entries: Vec<_> = entries.filter_map(Result::ok).collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_dir = entry.path().is_dir();
        let Some(index) = SECTIONS.iter().position(|(folder, _)| *folder == name) else {
            if !is_dir && name == "README.md" {
                continue;
            }
            errors.push(format!(
                "changelog.d/{name}: fragments go in the folder of their section: {}",
                folders.join(", ")
            ));
            continue;
        };
        if !is_dir {
            errors.push(format!(
                "changelog.d/{name}: fragments go in the folder of their section: {}",
                folders.join(", ")
            ));
            continue;
        }
        let files = match fs::read_dir(entry.path()) {
            Ok(files) => files,
            Err(e) => {
                errors.push(format!("changelog.d/{name}: {e}"));
                continue;
            }
        };
        for file in files.filter_map(Result::ok) {
            let file_name = file.file_name().to_string_lossy().into_owned();
            let label = format!("changelog.d/{name}/{file_name}");
            let path = file.path();
            if path.is_dir() || path.extension().is_none_or(|ext| ext != "md") {
                errors.push(format!("{label}: a fragment is a .md file"));
                continue;
            }
            match fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|t| check_entry(&t))
            {
                Ok(text) => by_folder[index].push(Fragment {
                    name: label,
                    file: path,
                    heading: SECTIONS[index].1,
                    text,
                }),
                Err(e) => errors.push(format!("{label}: {e}")),
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut fragments = Vec::new();
    for mut group in by_folder {
        group.sort_by(|a, b| compare_names(base_name(&a.name), base_name(&b.name)));
        fragments.extend(group);
    }
    Ok(fragments)
}

fn base_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

/// Checks a fragment's text and returns it without its trailing newlines:
/// list items as `CHANGELOG.md` has them, under the heading its folder names.
fn check_entry(text: &str) -> Result<String, String> {
    let text = text.replace("\r\n", "\n");
    let text = text.trim_end_matches([' ', '\t', '\n']);
    if text.is_empty() {
        return Err("it is empty".to_string());
    }
    if !text.starts_with("- ") {
        return Err("an entry starts with \"- \", as an item of a list".to_string());
    }
    for (i, line) in text.split('\n').enumerate() {
        if line.starts_with('#') {
            return Err(format!(
                "line {} is a heading: the folder names the section",
                i + 1
            ));
        }
        if line.trim_end_matches([' ', '\t']) != line {
            return Err(format!("line {} ends in spaces", i + 1));
        }
    }
    Ok(text.to_string())
}

/// Orders fragment names by the issue number they start with, so that 94
/// comes before 101, then by name. Names without a number come last.
fn compare_names(a: &str, b: &str) -> Ordering {
    match (number(a), number(b)) {
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (Some(x), Some(y)) if x != y => x.cmp(&y),
        _ => a.cmp(b),
    }
}

/// The number `name` starts with, if it does.
fn number(name: &str) -> Option<u64> {
    let end = name
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(name.len());
    name[..end].parse().ok()
}

/// Returns `changelog` with the fragments added to the end of their sections
/// of `## [Unreleased]`, and that section as it then reads. A section it
/// lacks is added where Keep a Changelog orders it.
fn fold(changelog: &str, fragments: &[Fragment]) -> Result<(String, String), String> {
    let changelog = changelog.replace("\r\n", "\n");
    let marker = format!("\n{UNRELEASED}\n");
    let start = changelog
        .find(&marker)
        .ok_or_else(|| format!("CHANGELOG.md has no {UNRELEASED:?} section"))?
        + 1;
    let body = start + UNRELEASED.len() + 1;
    let end = changelog[body..]
        .find("\n## [")
        .map_or(changelog.len(), |i| body + i + 1);
    let mut lines: Vec<String> = changelog[body..end].split('\n').map(String::from).collect();
    for (_, heading) in SECTIONS {
        let add: Vec<String> = fragments
            .iter()
            .filter(|f| f.heading == heading)
            .flat_map(|f| f.text.split('\n').map(String::from))
            .collect();
        if !add.is_empty() {
            insert(&mut lines, heading, add);
        }
    }
    let notes = format!("{UNRELEASED}\n{}", lines.join("\n"));
    Ok((
        format!("{}{notes}{}", &changelog[..start], &changelog[end..]),
        notes,
    ))
}

/// Adds `entries` at the end of the `### heading` part of `lines`, which
/// holds the lines of `## [Unreleased]`. When `lines` lack that part, it is
/// added where Keep a Changelog orders it.
fn insert(lines: &mut Vec<String>, heading: &str, mut entries: Vec<String>) {
    let title = format!("### {heading}");
    if let Some(at) = lines.iter().position(|l| *l == title) {
        let end = lines[at + 1..]
            .iter()
            .position(|l| l.starts_with("### "))
            .map_or(lines.len(), |i| at + 1 + i);
        let end = last_text(lines, at + 1, end);
        if end == at + 1 {
            entries.insert(0, String::new());
        }
        lines.splice(end..end, entries);
        return;
    }
    let order = SECTIONS.iter().position(|(_, h)| *h == heading);
    let mut part = vec![title, String::new()];
    part.extend(entries);
    let later = lines.iter().position(|line| {
        SECTIONS
            .iter()
            .position(|(_, h)| *line == format!("### {h}"))
            .is_some_and(|i| Some(i) > order)
    });
    if let Some(i) = later {
        part.push(String::new());
        lines.splice(i..i, part);
    } else {
        let end = last_text(lines, 0, lines.len());
        let mut block = vec![String::new()];
        block.extend(part);
        lines.splice(end..end, block);
    }
}

/// The index after the last line of `lines[from..to]` that has text, or
/// `from`.
fn last_text(lines: &[String], from: usize, mut to: usize) -> usize {
    while to > from && lines[to - 1].trim().is_empty() {
        to -= 1;
    }
    to
}

/// Turns the `## [Unreleased]` section of a changelog into the section of
/// `version` dated `date`, with a fresh empty `## [Unreleased]` above it.
fn release(changelog: &str, version: &str, date: &str) -> Result<String, String> {
    check_version(version)?;
    check_date(date)?;
    if changelog.contains(&format!("\n## [{version}]")) {
        return Err(format!("CHANGELOG.md already has a section for {version}"));
    }
    let start = changelog
        .find(&format!("\n{UNRELEASED}\n"))
        .ok_or_else(|| format!("CHANGELOG.md has no {UNRELEASED:?} section"))?
        + 1;
    let body = start + UNRELEASED.len() + 1;
    let end = changelog[body..]
        .find("\n## [")
        .map_or(changelog.len(), |i| body + i + 1);
    if changelog[body..end].trim().is_empty() {
        return Err(format!("{UNRELEASED} has no entries to release"));
    }
    Ok(format!(
        "{}{UNRELEASED}\n\n## [{version}] - {date}\n{}",
        &changelog[..start],
        &changelog[body..]
    ))
}

/// Accepts `X.Y.Z`, with an optional `-prerelease` suffix.
fn check_version(version: &str) -> Result<(), String> {
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (version, None),
    };
    let numbers: Vec<&str> = core.split('.').collect();
    let numeric = |s: &&str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let pre_ok = pre
        .is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.'));
    if numbers.len() == 3 && numbers.iter().all(numeric) && pre_ok {
        Ok(())
    } else {
        Err(format!("{version:?} is not a version such as 0.2.0"))
    }
}

/// Accepts `YYYY-MM-DD`.
fn check_date(date: &str) -> Result<(), String> {
    let parts: Vec<&str> = date.split('-').collect();
    let ok = parts.len() == 3
        && [4, 2, 2]
            .iter()
            .zip(&parts)
            .all(|(len, p)| p.len() == *len && p.bytes().all(|b| b.is_ascii_digit()))
        && (1..=12).contains(&parts[1].parse::<u32>().unwrap_or(0))
        && (1..=31).contains(&parts[2].parse::<u32>().unwrap_or(0));
    if ok {
        Ok(())
    } else {
        Err(format!("{date:?} is not a date such as 2026-11-24"))
    }
}

/// Today's date in UTC as `YYYY-MM-DD`.
fn today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    civil_date((seconds / 86_400) as i64)
}

/// The date `days` after 1970-01-01, by Howard Hinnant's civil-from-days
/// algorithm.
fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frag(heading: &'static str, text: &str) -> Fragment {
        Fragment {
            name: String::new(),
            file: PathBuf::new(),
            heading,
            text: text.to_string(),
        }
    }

    const EMPTY: &str =
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- First.\n";

    #[test]
    fn entry_accepts_list_items_and_trims_the_end() {
        assert_eq!(
            check_entry("- One.\n  more.\n\n").unwrap(),
            "- One.\n  more."
        );
        assert_eq!(check_entry("- A.\r\n- B.\r\n").unwrap(), "- A.\n- B.");
    }

    #[test]
    fn entry_rejects_what_changelog_does_not_have() {
        assert!(check_entry("").is_err());
        assert!(check_entry("Fixed a thing.").is_err());
        assert!(check_entry("- Ok.\n## Heading").is_err());
        assert!(check_entry("- Trailing space \n  more.").is_err());
    }

    #[test]
    fn names_order_by_issue_number() {
        assert_eq!(compare_names("94-a.md", "101-a.md"), Ordering::Less);
        assert_eq!(compare_names("101-a.md", "101-b.md"), Ordering::Less);
        assert_eq!(compare_names("x.md", "5-a.md"), Ordering::Greater);
        assert_eq!(compare_names("a.md", "b.md"), Ordering::Less);
    }

    #[test]
    fn fold_adds_sections_in_keep_a_changelog_order() {
        let frags = [frag("Fixed", "- Fix (#2)."), frag("Added", "- Add (#1).")];
        let (folded, notes) = fold(EMPTY, &frags).unwrap();
        assert_eq!(
            notes,
            "## [Unreleased]\n\n### Added\n\n- Add (#1).\n\n### Fixed\n\n- Fix (#2).\n\n"
        );
        assert!(folded.contains(&notes));
        assert!(folded.contains("## [0.1.0] - 2026-01-01"));
    }

    #[test]
    fn fold_appends_to_entries_already_there() {
        let log =
            "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- Old.\n\n### Fixed\n\n- Old fix.\n";
        let frags = [frag("Added", "- New."), frag("Changed", "- Changed.")];
        let (_, notes) = fold(log, &frags).unwrap();
        assert_eq!(
            notes,
            "## [Unreleased]\n\n### Added\n\n- Old.\n- New.\n\n### Changed\n\n- Changed.\n\n### Fixed\n\n- Old fix.\n"
        );
    }

    #[test]
    fn fold_needs_an_unreleased_section() {
        assert!(fold("# Changelog\n", &[]).is_err());
    }

    #[test]
    fn release_renames_the_section_and_adds_a_new_one() {
        let (folded, _) = fold(EMPTY, &[frag("Added", "- Add.")]).unwrap();
        let out = release(&folded, "0.2.0", "2026-12-01").unwrap();
        assert_eq!(
            out,
            "# Changelog\n\n## [Unreleased]\n\n## [0.2.0] - 2026-12-01\n\n### Added\n\n- Add.\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- First.\n"
        );
    }

    #[test]
    fn release_refuses_an_empty_section_a_repeat_and_bad_input() {
        assert!(release(EMPTY, "0.2.0", "2026-12-01").is_err());
        let (folded, _) = fold(EMPTY, &[frag("Added", "- Add.")]).unwrap();
        assert!(release(&folded, "0.1.0", "2026-12-01").is_err());
        assert!(release(&folded, "0.2", "2026-12-01").is_err());
        assert!(release(&folded, "0.2.0", "01/12/2026").is_err());
        assert!(release(&folded, "0.2.0-rc.1", "2026-12-01").is_ok());
    }

    #[test]
    fn civil_date_matches_known_days() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(19_782), "2024-02-29");
        assert_eq!(civil_date(20_447), "2025-12-25");
    }

    #[test]
    fn fragments_are_read_checked_and_folded_on_disk() {
        let root = std::env::temp_dir().join(format!("xtask-changelog-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["added", "fixed"] {
            fs::create_dir_all(root.join("changelog.d").join(dir)).unwrap();
        }
        fs::write(root.join("changelog.d/README.md"), "notes").unwrap();
        fs::write(root.join("changelog.d/added/101-b.md"), "- B.\n").unwrap();
        fs::write(root.join("changelog.d/added/94-a.md"), "- A.\n").unwrap();
        fs::write(root.join("changelog.d/fixed/7-c.md"), "- C.\n").unwrap();
        fs::write(root.join("CHANGELOG.md"), EMPTY).unwrap();

        run(&root, &Mode::Check, None).unwrap();
        run(&root, &Mode::Release("0.2.0".into()), Some("2026-12-01")).unwrap();

        let log = fs::read_to_string(root.join("CHANGELOG.md")).unwrap();
        assert!(
            log.contains(
                "## [0.2.0] - 2026-12-01\n\n### Added\n\n- A.\n- B.\n\n### Fixed\n\n- C.\n"
            )
        );
        assert!(!root.join("changelog.d/added").exists());
        assert!(root.join("changelog.d/README.md").exists());

        fs::create_dir_all(root.join("changelog.d/oops")).unwrap();
        assert!(run(&root, &Mode::Check, None).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
