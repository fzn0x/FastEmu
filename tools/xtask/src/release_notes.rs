//! The notes of a GitHub release.
//!
//! The release workflow (`.github/workflows/release.yml`) publishes them:
//! the version's section of `CHANGELOG.md`, then the downloads table of
//! `.github/release-notes.md` with its placeholders filled in, then a link
//! to the commits since the previous release. A dry run, which has no
//! released section yet, takes `## [Unreleased]` with the fragments of
//! `changelog.d` folded in.

use std::fs;
use std::path::Path;

use crate::changelog;

/// What the notes are made from.
pub struct Options {
    /// The version, as `0.2.0`.
    pub version: String,
    /// The tag the downloads are under, as `v0.2.0`.
    pub tag: String,
    /// `owner/name` on GitHub.
    pub repo: String,
    /// The tag of the release before, for the compare link. Without one,
    /// the link lists the tag's commits.
    pub previous_tag: Option<String>,
    /// The commit the compare link ends at, when it is not the tag: a dry
    /// run's commit, which has no tag yet.
    pub head: Option<String>,
    /// Take `## [Unreleased]` instead of the version's section.
    pub unreleased: bool,
    /// The APK is signed with a debug key: the notes say so.
    pub debug_apk: bool,
}

/// Said in place of the changes when a dry run's `## [Unreleased]` has none.
const NO_CHANGES: &str = "_No changes are listed under Unreleased in CHANGELOG.md yet._";

/// Said when the APK is signed with the debug key.
const DEBUG_APK: &str = "**This is a test build.** The APK is signed with a debug key, not \
FastEmu's release key: no later release can install over it. Uninstall it before installing a \
release.";

/// Builds the notes for the repository at `root`.
pub fn build(root: &Path, options: &Options) -> Result<String, String> {
    let changes = if options.unreleased {
        let section = changelog::unreleased(root)?;
        let body = section_body(&section);
        if body.trim().is_empty() {
            eprintln!("warning: ## [Unreleased] has no entries");
            NO_CHANGES.to_string()
        } else {
            body
        }
    } else {
        let file = root.join("CHANGELOG.md");
        let log = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let body = version_section(&log, &options.version)?;
        if body.trim().is_empty() {
            return Err(format!(
                "CHANGELOG.md's section for {} has no entries",
                options.version
            ));
        }
        body
    };
    let file = root.join(".github/release-notes.md");
    let template = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(assemble(&changes, &template, options))
}

/// Puts the notes together from the changes and the downloads template.
fn assemble(changes: &str, template: &str, options: &Options) -> String {
    let downloads = strip_comments(&template.replace("\r\n", "\n"))
        .replace("{version}", &options.version)
        .replace("{tag}", &options.tag)
        .replace("{repo}", &options.repo);
    let head = options.head.as_deref().unwrap_or(&options.tag);
    let link = match &options.previous_tag {
        Some(previous) => format!(
            "https://github.com/{}/compare/{previous}...{head}",
            options.repo
        ),
        None => format!("https://github.com/{}/commits/{head}", options.repo),
    };
    let mut notes = String::new();
    notes.push_str(unwrap(changes).trim());
    notes.push_str("\n\n");
    notes.push_str(downloads.trim());
    if options.debug_apk {
        notes.push_str("\n\n");
        notes.push_str(DEBUG_APK);
    }
    notes.push_str("\n\n**Full Changelog**: ");
    notes.push_str(&link);
    notes.push('\n');
    squeeze_blank_lines(&notes)
}

/// The text under `## [version]`, up to the next `## [` heading.
fn version_section(changelog: &str, version: &str) -> Result<String, String> {
    let changelog = changelog.replace("\r\n", "\n");
    let heading = format!("## [{version}]");
    let start = changelog
        .split_inclusive('\n')
        .scan(0, |at, line| {
            let here = *at;
            *at += line.len();
            Some((here, line))
        })
        .find(|(_, line)| line.starts_with(&heading))
        .map(|(at, _)| at)
        .ok_or_else(|| format!("CHANGELOG.md has no section for {version}"))?;
    Ok(section_body(&changelog[start..]))
}

/// A section's lines after its heading, up to the next `## ` heading,
/// without link reference definitions.
fn section_body(section: &str) -> String {
    let section = section.replace("\r\n", "\n");
    let mut lines = section.split('\n');
    lines.next();
    lines
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| !is_link_reference(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A link reference definition, as `[0.1.0]: https://...`.
fn is_link_reference(line: &str) -> bool {
    line.starts_with('[') && line.contains("]: ")
}

/// Joins the wrapped lines of each paragraph and list item: release pages
/// break lines where the text does, unlike CHANGELOG.md's renderer.
fn unwrap(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut code = false;
    for line in text.split('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            out.extend(current.take());
            out.push(line.to_string());
            code = !code;
            continue;
        }
        let starts_block = code
            || trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('|')
            || trimmed.starts_with('>');
        if starts_block {
            out.extend(current.take());
            out.push(line.to_string());
        } else if is_list_item(trimmed) || current.is_none() {
            out.extend(current.take());
            current = Some(line.trim_end().to_string());
        } else if let Some(buffer) = current.as_mut() {
            buffer.push(' ');
            buffer.push_str(trimmed.trim_end());
        }
    }
    out.extend(current);
    out.join("\n")
}

fn is_list_item(line: &str) -> bool {
    if line.starts_with("- ") || line.starts_with("* ") {
        return true;
    }
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    digits > 0 && line[digits..].starts_with(". ")
}

/// The text without its HTML comments, and without the lines they leave
/// empty.
fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => {
                rest = &rest[start + end + 3..];
                // The comment's own line goes with it.
                if out.ends_with('\n') || out.is_empty() {
                    rest = rest.strip_prefix('\n').unwrap_or(rest);
                }
            }
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// At most one empty line in a row.
fn squeeze_blank_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "# Changelog\n\n## [Unreleased]\n\n## [0.2.0] - 2026-12-01\n\n### Added\n\n\
- A long entry that\n  wraps onto a second line (#3).\n- Short (#4).\n\n### Fixed\n\n\
- Fixed (#5).\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- First.\n\n\
[0.2.0]: https://example.com\n";

    const TEMPLATE: &str = "### Get it\n\n<!-- The release workflow replaces {version}. -->\n\n\
| System | Download |\n| --- | --- |\n| **Windows** | [`fastemu-{version}-windows-x64.zip`](https://github.com/{repo}/releases/download/{tag}/fastemu-{version}-windows-x64.zip) |\n\n\
Verify downloads with `SHA256SUMS`.\n";

    fn options() -> Options {
        Options {
            version: "0.2.0".into(),
            tag: "v0.2.0".into(),
            repo: "fzn0x/FastEmu".into(),
            previous_tag: Some("v0.1.0".into()),
            head: None,
            unreleased: false,
            debug_apk: false,
        }
    }

    #[test]
    fn the_section_ends_at_the_next_version() {
        let body = version_section(LOG, "0.2.0").unwrap();
        assert!(body.contains("### Fixed"));
        assert!(!body.contains("First."));
        assert!(version_section(LOG, "0.3.0").is_err());
        let last = version_section(LOG, "0.1.0").unwrap();
        assert!(last.contains("- First."));
        assert!(!last.contains("example.com"));
    }

    #[test]
    fn wrapped_items_become_one_line() {
        let text = "### Added\n\n- A long entry that\n  wraps here.\n- Next.\n\nA paragraph\nthat wraps.\n\n```\nkeep\n  this\n```";
        assert_eq!(
            unwrap(text),
            "### Added\n\n- A long entry that wraps here.\n- Next.\n\nA paragraph that wraps.\n\n```\nkeep\n  this\n```"
        );
        assert!(is_list_item("12. Twelve"));
        assert!(!is_list_item("12 Twelve"));
    }

    #[test]
    fn the_notes_have_the_changes_the_downloads_and_the_link() {
        let body = version_section(LOG, "0.2.0").unwrap();
        let notes = assemble(&body, TEMPLATE, &options());
        assert!(
            notes.starts_with("### Added\n\n- A long entry that wraps onto a second line (#3).\n")
        );
        assert!(notes.contains("[`fastemu-0.2.0-windows-x64.zip`](https://github.com/fzn0x/FastEmu/releases/download/v0.2.0/fastemu-0.2.0-windows-x64.zip)"));
        assert!(!notes.contains("<!--"));
        assert!(!notes.contains('{'));
        assert!(!notes.contains("\n\n\n"));
        assert!(notes.contains("Verify downloads with `SHA256SUMS`."));
        assert!(notes.ends_with(
            "\n\n**Full Changelog**: https://github.com/fzn0x/FastEmu/compare/v0.1.0...v0.2.0\n"
        ));
        assert!(!notes.contains("test build"));
    }

    #[test]
    fn a_first_release_links_its_commits_and_a_dry_run_says_what_it_is() {
        let mut options = options();
        options.previous_tag = None;
        options.head = Some("abc123".into());
        options.debug_apk = true;
        let notes = assemble("- One.", TEMPLATE, &options);
        assert!(notes.contains("**This is a test build.**"));
        assert!(
            notes
                .ends_with("**Full Changelog**: https://github.com/fzn0x/FastEmu/commits/abc123\n")
        );
    }

    #[test]
    fn comments_go_with_their_lines() {
        assert_eq!(strip_comments("a\n<!-- x -->\nb\n"), "a\nb\n");
        assert_eq!(strip_comments("a <!-- x --> b"), "a  b");
        assert_eq!(strip_comments("<!-- open"), "");
    }

    #[test]
    fn notes_are_built_from_the_files_on_disk() {
        let root = std::env::temp_dir().join(format!("xtask-notes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github")).unwrap();
        fs::create_dir_all(root.join("changelog.d/fixed")).unwrap();
        fs::write(root.join("CHANGELOG.md"), LOG).unwrap();
        fs::write(root.join(".github/release-notes.md"), TEMPLATE).unwrap();

        let notes = build(&root, &options()).unwrap();
        assert!(notes.contains("- Fixed (#5)."));

        let mut dry = options();
        dry.unreleased = true;
        let notes = build(&root, &dry).unwrap();
        assert!(notes.starts_with(NO_CHANGES));
        fs::write(root.join("changelog.d/fixed/9-x.md"), "- Fragment (#9).\n").unwrap();
        let notes = build(&root, &dry).unwrap();
        assert!(notes.starts_with("### Fixed\n\n- Fragment (#9)."));

        let mut empty = options();
        empty.version = "0.3.0".into();
        assert!(build(&root, &empty).is_err());
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [Unreleased]\n\n## [0.3.0] - 2026-12-02\n\n## [0.2.0] - 2026-12-01\n",
        )
        .unwrap();
        assert!(build(&root, &empty).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
