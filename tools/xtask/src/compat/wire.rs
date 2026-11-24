//! The lines a game's runner process writes to its standard output, one
//! record per line, so the parent knows how far a game got even when it
//! has to kill the runner.

/// One record of a runner's output.
#[derive(Clone, Debug, PartialEq)]
pub enum Record {
    /// The suite's `MIDlet-Name`, `MIDlet-Vendor` and `MIDlet-Version`.
    Info {
        name: String,
        vendor: String,
        version: String,
    },
    /// A line of the game's log; `level` is `out`, `err`, `warn` or `info`.
    Log { level: String, text: String },
    /// A new frame, and the seconds since the launch it came at.
    Frame { serial: u64, secs: f64 },
    /// How many exceptions the game's callbacks and threads threw.
    Errors(u64),
    /// Where the last frame was saved.
    Screenshot(String),
    /// Something about the run itself, such as a stop that did not end.
    Note(String),
    /// How the run ended: `runs`, `crashed`, `exited` or `hung`, and why.
    End { status: String, detail: String },
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

impl Record {
    /// The record as one line, without its line end.
    pub fn line(&self) -> String {
        let fields: Vec<String> = match self {
            Record::Info {
                name,
                vendor,
                version,
            } => vec!["info".into(), escape(name), escape(vendor), escape(version)],
            Record::Log { level, text } => vec!["log".into(), escape(level), escape(text)],
            Record::Frame { serial, secs } => {
                vec!["frame".into(), serial.to_string(), format!("{secs:.3}")]
            }
            Record::Errors(n) => vec!["errors".into(), n.to_string()],
            Record::Screenshot(path) => vec!["screenshot".into(), escape(path)],
            Record::Note(text) => vec!["note".into(), escape(text)],
            Record::End { status, detail } => {
                vec!["end".into(), escape(status), escape(detail)]
            }
        };
        fields.join("\t")
    }

    /// Reads a line [`Record::line`] wrote.
    pub fn parse(line: &str) -> Option<Record> {
        let fields: Vec<String> = line.split('\t').map(unescape).collect();
        let field = |i: usize| fields.get(i).cloned();
        Some(match fields.first()?.as_str() {
            "info" => Record::Info {
                name: field(1)?,
                vendor: field(2)?,
                version: field(3)?,
            },
            "log" => Record::Log {
                level: field(1)?,
                text: field(2)?,
            },
            "frame" => Record::Frame {
                serial: field(1)?.parse().ok()?,
                secs: field(2)?.parse().ok()?,
            },
            "errors" => Record::Errors(field(1)?.parse().ok()?),
            "screenshot" => Record::Screenshot(field(1)?),
            "note" => Record::Note(field(1)?),
            "end" => Record::End {
                status: field(1)?,
                detail: field(2)?,
            },
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_survive_the_trip() {
        let records = [
            Record::Info {
                name: "Snake\t2".into(),
                vendor: "A\\B".into(),
                version: "1.0".into(),
            },
            Record::Log {
                level: "err".into(),
                text: "Exception in paint\n\tat x".into(),
            },
            Record::Frame {
                serial: 42,
                secs: 1.5,
            },
            Record::Errors(3),
            Record::Screenshot("D:\\out\\a.png".into()),
            Record::Note("stop".into()),
            Record::End {
                status: "runs".into(),
                detail: String::new(),
            },
        ];
        for record in records {
            assert_eq!(Record::parse(&record.line()), Some(record.clone()));
        }
        assert_eq!(Record::parse("noise"), None);
    }
}
