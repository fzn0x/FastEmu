//! The lines both runners of a bench write to their standard output:
//! FastEmu's `xtask bench-run` ([`super::child`]) and MicroEmulator's
//! wrapper, MeBench. One record per line, words of `key=value`:
//!
//! ```text
//! RUN version microemulator=2.0.4
//! RUN first game=0 ms=86
//! RUN frames game=0 n=120 ms=4000 w=240 h=320
//! RUN shot game=0 w=240 h=320
//! RUN end game=0 status=exited frames=600 errors=0 ms=9000
//! log 0 out BENCH cpu.int round=2 reps=3 ms=1404 first=470 check=127
//! ```
//!
//! A `log` line carries a line of the game's log. When its text starts
//! with `BENCH `, the benchmark MIDlet printed it, and it also parses into
//! a [`Line::Bench`] or a [`Line::BenchError`]. A line that fits none of
//! these, or misses a field, parses to `None` and is ignored.

use std::collections::BTreeMap;

/// One record of a runner's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// MeBench's `--version`: MicroEmulator's own version.
    Version { microemulator: String },
    /// Game `game` presented its first frame, `ms` after the runner began.
    First { game: usize, ms: u64 },
    /// Game `game` had presented `n` frames `ms` after the runner began,
    /// on a screen of `size` (width and height), when the runner knows it.
    Frames {
        game: usize,
        n: u64,
        ms: u64,
        size: Option<(i32, i32)>,
    },
    /// The runner saved game `game`'s screen, of this size.
    Shot { game: usize, w: i32, h: i32 },
    /// How game `game` ended: `exited` (it ended itself), `crashed`, or
    /// `running` (the run's time was up).
    End {
        game: usize,
        status: String,
        frames: u64,
        errors: u64,
        ms: u64,
    },
    /// A line of the game's log at `level`.
    Log {
        game: usize,
        level: String,
        text: String,
    },
    /// A result of the benchmark MIDlet: `BENCH name key=value ...`, with
    /// integer values.
    Bench {
        game: usize,
        name: String,
        fields: BTreeMap<String, i64>,
    },
    /// A kernel of the benchmark MIDlet that threw: `BENCH error name=N
    /// what=TEXT`.
    BenchError {
        game: usize,
        name: String,
        what: String,
    },
}

/// The `key=value` words of `words`, or `None` if one is not of that form.
fn pairs<'a>(words: impl Iterator<Item = &'a str>) -> Option<BTreeMap<&'a str, &'a str>> {
    let mut map = BTreeMap::new();
    for word in words {
        let (key, value) = word.split_once('=')?;
        if key.is_empty() {
            return None;
        }
        map.insert(key, value);
    }
    Some(map)
}

fn number<T: std::str::FromStr>(map: &BTreeMap<&str, &str>, key: &str) -> Option<T> {
    map.get(key)?.parse().ok()
}

impl Line {
    /// Reads one line of a runner's output.
    pub fn parse(line: &str) -> Option<Line> {
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(rest) = line.strip_prefix("log ") {
            return Self::log(rest);
        }
        let rest = line.strip_prefix("RUN ")?;
        let mut words = rest.split(' ').filter(|w| !w.is_empty());
        let kind = words.next()?;
        let map = pairs(words)?;
        let game = || number::<usize>(&map, "game");
        Some(match kind {
            "version" => Line::Version {
                microemulator: (*map.get("microemulator")?).to_owned(),
            },
            "first" => Line::First {
                game: game()?,
                ms: number(&map, "ms")?,
            },
            "frames" => Line::Frames {
                game: game()?,
                n: number(&map, "n")?,
                ms: number(&map, "ms")?,
                size: match (map.get("w"), map.get("h")) {
                    (None, None) => None,
                    _ => Some((number(&map, "w")?, number(&map, "h")?)),
                },
            },
            "shot" => Line::Shot {
                game: game()?,
                w: number(&map, "w")?,
                h: number(&map, "h")?,
            },
            "end" => Line::End {
                game: game()?,
                status: (*map.get("status")?).to_owned(),
                frames: number(&map, "frames")?,
                errors: number(&map, "errors")?,
                ms: number(&map, "ms")?,
            },
            _ => return None,
        })
    }

    /// `GAME LEVEL TEXT`, after `log `.
    fn log(rest: &str) -> Option<Line> {
        let mut parts = rest.splitn(3, ' ');
        let game: usize = parts.next()?.parse().ok()?;
        let level = parts.next()?;
        if level.is_empty() {
            return None;
        }
        let text = parts.next().unwrap_or_default();
        let Some(bench) = text.strip_prefix("BENCH ") else {
            return Some(Line::Log {
                game,
                level: level.to_owned(),
                text: text.to_owned(),
            });
        };
        let mut words = bench.split(' ').filter(|w| !w.is_empty());
        let name = words.next()?;
        if name.contains('=') {
            return None;
        }
        let map = pairs(words)?;
        if name == "error" {
            return Some(Line::BenchError {
                game,
                name: (*map.get("name")?).to_owned(),
                what: (*map.get("what")?).to_owned(),
            });
        }
        let mut fields = BTreeMap::new();
        for (key, value) in map {
            fields.insert(key.to_owned(), value.parse().ok()?);
        }
        Some(Line::Bench {
            game,
            name: name.to_owned(),
            fields,
        })
    }

    /// The line as a runner writes it.
    pub fn text(&self) -> String {
        match self {
            Line::Version { microemulator } => format!("RUN version microemulator={microemulator}"),
            Line::First { game, ms } => format!("RUN first game={game} ms={ms}"),
            Line::Frames { game, n, ms, size } => {
                let size = size.map_or_else(String::new, |(w, h)| format!(" w={w} h={h}"));
                format!("RUN frames game={game} n={n} ms={ms}{size}")
            }
            Line::Shot { game, w, h } => format!("RUN shot game={game} w={w} h={h}"),
            Line::End {
                game,
                status,
                frames,
                errors,
                ms,
            } => format!(
                "RUN end game={game} status={status} frames={frames} errors={errors} ms={ms}"
            ),
            Line::Log { game, level, text } => format!("log {game} {level} {text}"),
            Line::Bench { game, name, fields } => {
                let fields: Vec<String> = fields.iter().map(|(k, v)| format!("{k}={v}")).collect();
                format!("log {game} out BENCH {name} {}", fields.join(" "))
            }
            Line::BenchError { game, name, what } => {
                format!("log {game} out BENCH error name={name} what={what}")
            }
        }
    }

    /// The game a record is about, if any.
    pub fn game(&self) -> Option<usize> {
        match self {
            Line::Version { .. } => None,
            Line::First { game, .. }
            | Line::Frames { game, .. }
            | Line::Shot { game, .. }
            | Line::End { game, .. }
            | Line::Log { game, .. }
            | Line::Bench { game, .. }
            | Line::BenchError { game, .. } => Some(*game),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fields(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
        pairs.iter().map(|&(k, v)| (k.to_owned(), v)).collect()
    }

    #[test]
    fn parses_run_and_bench_lines() {
        assert_eq!(
            Line::parse("RUN first game=0 ms=86"),
            Some(Line::First { game: 0, ms: 86 })
        );
        assert_eq!(
            Line::parse("RUN frames game=3 n=120 ms=4000"),
            Some(Line::Frames {
                game: 3,
                n: 120,
                ms: 4000,
                size: None
            })
        );
        assert_eq!(
            Line::parse("RUN frames game=0 n=7 ms=1000 w=176 h=176"),
            Some(Line::Frames {
                game: 0,
                n: 7,
                ms: 1000,
                size: Some((176, 176))
            })
        );
        assert_eq!(
            Line::parse("RUN end game=0 status=exited frames=600 errors=0 ms=9000\r"),
            Some(Line::End {
                game: 0,
                status: "exited".into(),
                frames: 600,
                errors: 0,
                ms: 9000
            })
        );
        assert_eq!(
            Line::parse("RUN version microemulator=2.0.4"),
            Some(Line::Version {
                microemulator: "2.0.4".into()
            })
        );
        let kernel = Line::Bench {
            game: 0,
            name: "cpu.int".into(),
            fields: fields(&[
                ("round", 2),
                ("reps", 3),
                ("ms", 1404),
                ("first", 470),
                ("check", 127),
            ]),
        };
        // FastEmu's runner gives System.out the level `out`; MicroEmulator
        // logs it as `info`. Both parse the same.
        assert_eq!(
            Line::parse("log 0 out BENCH cpu.int round=2 reps=3 ms=1404 first=470 check=127"),
            Some(kernel.clone())
        );
        assert_eq!(
            Line::parse("log 0 info BENCH cpu.int round=2 reps=3 ms=1404 first=470 check=127"),
            Some(kernel.clone())
        );
        assert_eq!(Line::parse(&kernel.text()), Some(kernel));
        assert_eq!(
            Line::parse(
                "log 2 info BENCH error name=draw.line what=java.lang.IllegalStateException:_Bench-Fail"
            ),
            Some(Line::BenchError {
                game: 2,
                name: "draw.line".into(),
                what: "java.lang.IllegalStateException:_Bench-Fail".into()
            })
        );
        assert_eq!(
            Line::parse("log 1 err Exception in thread main"),
            Some(Line::Log {
                game: 1,
                level: "err".into(),
                text: "Exception in thread main".into()
            })
        );
        // Garbage, a missing field, a value that is no number, and the
        // call-location line MicroEmulator's own appender adds.
        for bad in [
            "",
            "noise",
            "RUN",
            "RUN first game=0",
            "RUN first ms=86",
            "RUN frames game=0 n=7 ms=1000 w=176",
            "RUN end game=0 status=exited frames=600 errors=0",
            "RUN first game=zero ms=86",
            "RUN nothing game=0",
            "log x out text",
            "log 0 out BENCH cpu.int round=2 reps=many",
            "log 0 out BENCH cpu.int round",
            "log 0 out BENCH error name=cpu.int",
            "\tat fastemu.bench.Bench.line(Bench.java:31)",
        ] {
            assert_eq!(Line::parse(bad), None, "{bad:?}");
        }
    }
}
