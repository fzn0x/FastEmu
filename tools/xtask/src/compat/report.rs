//! What one game did, made from its runner's records, and the reports:
//! a Markdown table with details, and the same as JSON.

use std::fmt::Write as _;

use super::Options;
use super::wire::Record;

/// The result of one game's run.
pub struct Game {
    /// The JAR, relative to the folder that was run.
    pub file: String,
    pub slug: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    /// `runs`, `crashed`, `exited` or `hung`.
    pub status: String,
    pub detail: String,
    pub frames: u64,
    /// When the last new frame came, in seconds since the launch.
    pub last_frame: Option<f64>,
    pub errors: u64,
    /// What the game asked for and FastEmu lacks: `class a/B`,
    /// `method a/B.c()V`, `field a/B.d:I`.
    pub missing: Vec<String>,
    pub first_exception: Option<String>,
    pub screenshot: Option<String>,
    pub notes: Vec<String>,
}

impl Game {
    /// The result the records give; `killed` says why the runner was
    /// stopped, when it was, and `stderr` is what it printed itself.
    pub fn from_records(
        file: String,
        slug: &str,
        records: &[Record],
        killed: Option<String>,
        stderr: &str,
    ) -> Self {
        let mut game = Game {
            file,
            slug: slug.to_owned(),
            name: String::new(),
            vendor: String::new(),
            version: String::new(),
            status: "hung".into(),
            detail: String::new(),
            frames: 0,
            last_frame: None,
            errors: 0,
            missing: Vec::new(),
            first_exception: None,
            screenshot: None,
            notes: Vec::new(),
        };
        let mut ended = false;
        for record in records {
            match record {
                Record::Info {
                    name,
                    vendor,
                    version,
                } => {
                    game.name.clone_from(name);
                    game.vendor.clone_from(vendor);
                    game.version.clone_from(version);
                }
                Record::Log { level, text } => game.log(level, text),
                Record::Frame { serial, secs } => {
                    game.frames = *serial;
                    game.last_frame = Some(*secs);
                }
                Record::Errors(n) => game.errors = *n,
                Record::Screenshot(path) => game.screenshot = Some(path.clone()),
                Record::Note(text) => game.notes.push(text.clone()),
                Record::End { status, detail } => {
                    game.status.clone_from(status);
                    game.detail.clone_from(detail);
                    ended = true;
                }
            }
        }
        if let Some(why) = killed {
            game.status = "hung".into();
            game.detail = match game.last_frame {
                Some(at) => format!("{why}; {} frames, the last at {at:.1} s", game.frames),
                None => format!("{why}; no frame"),
            };
        } else if !ended {
            game.status = "crashed".into();
            game.detail = format!("the runner ended without a result: {stderr}");
        }
        if game.status == "crashed" && game.first_exception.is_none() && !game.detail.is_empty() {
            game.first_exception = Some(game.detail.clone());
        }
        game
    }

    fn log(&mut self, level: &str, text: &str) {
        if level == "warn" {
            for kind in ["class ", "method ", "field "] {
                if let Some(what) = text
                    .strip_prefix("missing ")
                    .filter(|t| t.starts_with(kind))
                    && !self.missing.iter().any(|m| m == what)
                {
                    self.missing.push(what.to_owned());
                }
            }
        }
        let is_exception = level == "err"
            && !text.starts_with(char::is_whitespace)
            && (text.contains("Exception") || text.contains("Error"));
        if is_exception && self.first_exception.is_none() {
            self.first_exception = Some(text.to_owned());
        }
    }

    fn title(&self) -> String {
        if self.name.is_empty() {
            self.file.clone()
        } else {
            format!("{} {}", self.name, self.version).trim().to_owned()
        }
    }
}

/// The game's log as text, one line per log line.
pub fn log_text(records: &[Record]) -> String {
    let mut out = String::new();
    for record in records {
        match record {
            Record::Log { level, text } => {
                let _ = writeln!(out, "[{level}] {text}");
            }
            Record::Note(text) => {
                let _ = writeln!(out, "[runner] {text}");
            }
            Record::End { status, detail } => {
                let _ = writeln!(out, "[runner] {status}: {detail}");
            }
            _ => {}
        }
    }
    out
}

/// Text for one cell of a Markdown table.
fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    format!("{cut}...")
}

/// The Markdown report.
pub fn markdown(options: &Options, games: &[Game]) -> String {
    let mut out = String::from("# Compatibility run\n\n");
    let _ = writeln!(
        out,
        "`cargo xtask compat` ran each JAR under `{}` for {} s on the `{}` profile, \
         without a window or sound device, pressing `{}` from the first frame.\n",
        options.dir.display(),
        options.seconds,
        options.profile,
        options.keys
    );
    out.push_str(
        "A game **runs** when it still presented new frames in the last 10 s of its run, \
         or when its screen stood still that long but it waits, as on an About page: its \
         event thread answered, every thread waited, it threw no exception and it did not \
         pause itself. It \
         **crashed** when the runtime ended it, **exited** when it ended itself, and is \
         **hung** when it presented no frame, stood still without showing that it waits \
         (the detail says why), or its runner had to be killed. Missing lists every class \
         and member the game asked for and FastEmu lacks; games probe for optional APIs, \
         so not every one is a fault.\n\n",
    );
    let count = |status: &str| games.iter().filter(|g| g.status == status).count();
    let _ = writeln!(
        out,
        "{} games: {} run, {} crashed, {} exited, {} hung.\n",
        games.len(),
        count("runs"),
        count("crashed"),
        count("exited"),
        count("hung")
    );
    out.push_str("| Game | File | Status | Frames | Errors | Missing | First exception |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for g in games {
        let missing = match g.missing.len() {
            0 => String::new(),
            n => format!("{n}: {}", g.missing.first().map_or("", String::as_str)),
        };
        let _ = writeln!(
            out,
            "| {} | `{}` | {} | {} | {} | {} | {} |",
            cell(&g.title()),
            cell(&g.file),
            g.status,
            g.frames,
            g.errors,
            cell(&missing),
            cell(&shorten(g.first_exception.as_deref().unwrap_or(""), 90))
        );
    }
    out.push_str("\n## Details\n");
    for g in games {
        let _ = writeln!(out, "\n### {}\n", g.title());
        let _ = writeln!(out, "- File: `{}`", g.file);
        let _ = writeln!(out, "- Status: {} ({})", g.status, g.detail);
        let _ = writeln!(out, "- Frames: {}, errors: {}", g.frames, g.errors);
        if let Some(e) = &g.first_exception {
            let _ = writeln!(out, "- First exception: `{}`", e.replace('`', "'"));
        }
        for m in &g.missing {
            let _ = writeln!(out, "- Missing {m}");
        }
        for n in &g.notes {
            let _ = writeln!(out, "- Note: {n}");
        }
        if g.screenshot.is_some() {
            let _ = writeln!(out, "- Screenshot: `screens/{}.png`", g.slug);
        }
        let _ = writeln!(out, "- Log: `logs/{}.log`", g.slug);
    }
    out
}

/// A JSON string literal.
fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn optional(text: Option<&str>) -> String {
    text.map_or_else(|| "null".to_owned(), string)
}

/// The JSON report: the run's settings and one object per game.
pub fn json(options: &Options, games: &[Game]) -> String {
    let mut out = String::from("{\n");
    let _ = writeln!(
        out,
        "  \"dir\": {},",
        string(&options.dir.display().to_string())
    );
    let _ = writeln!(out, "  \"seconds\": {},", options.seconds);
    let _ = writeln!(out, "  \"profile\": {},", string(&options.profile));
    let _ = writeln!(out, "  \"keys\": {},", string(&options.keys));
    out.push_str("  \"games\": [\n");
    for (i, g) in games.iter().enumerate() {
        let missing: Vec<String> = g.missing.iter().map(|m| string(m)).collect();
        let notes: Vec<String> = g.notes.iter().map(|n| string(n)).collect();
        let fields = [
            ("file", string(&g.file)),
            ("name", string(&g.name)),
            ("vendor", string(&g.vendor)),
            ("version", string(&g.version)),
            ("status", string(&g.status)),
            ("detail", string(&g.detail)),
            ("frames", g.frames.to_string()),
            (
                "last_frame_seconds",
                g.last_frame
                    .map_or_else(|| "null".to_owned(), |s| format!("{s:.3}")),
            ),
            ("errors", g.errors.to_string()),
            ("missing", format!("[{}]", missing.join(", "))),
            ("first_exception", optional(g.first_exception.as_deref())),
            (
                "screenshot",
                optional(
                    g.screenshot
                        .as_ref()
                        .map(|_| format!("screens/{}.png", g.slug))
                        .as_deref(),
                ),
            ),
            ("log", string(&format!("logs/{}.log", g.slug))),
            ("notes", format!("[{}]", notes.join(", "))),
        ];
        let body: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("      \"{k}\": {v}"))
            .collect();
        let comma = if i + 1 < games.len() { "," } else { "" };
        let _ = writeln!(out, "    {{\n{}\n    }}{comma}", body.join(",\n"));
    }
    out.push_str("  ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(level: &str, text: &str) -> Record {
        Record::Log {
            level: level.into(),
            text: text.into(),
        }
    }

    #[test]
    fn records_make_a_result() {
        let records = [
            Record::Info {
                name: "Bomb".into(),
                vendor: "SSG".into(),
                version: "1.1".into(),
            },
            log("warn", "missing class com/nokia/mid/ui/FullCanvas"),
            log("warn", "missing class com/nokia/mid/ui/FullCanvas"),
            log("warn", "missing method a/B.c()V"),
            log(
                "err",
                "Exception in thread \"MIDlet\" java.lang.NoClassDefFoundError: x",
            ),
            log("err", "\tat a.B.c(B.java)"),
            Record::Frame {
                serial: 3,
                secs: 0.5,
            },
            Record::Errors(1),
            Record::End {
                status: "crashed".into(),
                detail: "NoClassDefFoundError".into(),
            },
        ];
        let game = Game::from_records("b.jar".into(), "b", &records, None, "");
        assert_eq!(game.status, "crashed");
        assert_eq!(
            game.missing,
            ["class com/nokia/mid/ui/FullCanvas", "method a/B.c()V"]
        );
        assert!(
            game.first_exception
                .unwrap()
                .contains("NoClassDefFoundError")
        );
        assert_eq!((game.frames, game.errors), (3, 1));
    }

    #[test]
    fn a_killed_runner_is_a_hang() {
        let records = [Record::Frame {
            serial: 10,
            secs: 2.0,
        }];
        let game = Game::from_records("t.jar".into(), "t", &records, Some("killed".into()), "");
        assert_eq!(game.status, "hung");
        assert!(game.detail.contains("10 frames"), "{}", game.detail);
        let silent = Game::from_records("t.jar".into(), "t", &[], None, "boom");
        assert_eq!(silent.status, "crashed");
        assert!(silent.detail.contains("boom"));
    }

    #[test]
    fn json_escapes_text() {
        assert_eq!(string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }
}
