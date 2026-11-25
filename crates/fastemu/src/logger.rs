//! A logger that writes to the terminal, so the warnings of the app and
//! its views (a damaged settings file, no audio device) show somewhere.
//! Warnings and errors by default; `FASTEMU_LOG=info` (or `debug`,
//! `trace`, `off`) picks another level.

use log::{Level, LevelFilter, Log, Metadata, Record};

struct Stderr;

impl Log for Stderr {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", line(record.level(), record.args()));
        }
    }

    fn flush(&self) {}
}

/// The line `fastemu: <level>: <message>` for the terminal. Messages hold
/// what games and files say (a game's name, a launch error), and a
/// terminal runs the escape sequences in them, so control and bidirectional
/// override characters are escaped as the game's own log lines are.
fn line(level: Level, message: &std::fmt::Arguments<'_>) -> String {
    let level = level.as_str().to_ascii_lowercase();
    format!(
        "fastemu: {level}: {}",
        fastemu_core::escape_control(message.to_string())
    )
}

/// The level `FASTEMU_LOG` names, or warnings and errors.
fn level(setting: Option<&str>) -> LevelFilter {
    setting
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(LevelFilter::Warn)
}

/// Installs the logger. Call once, before anything logs.
pub fn init() {
    let setting = std::env::var("FASTEMU_LOG").ok();
    if log::set_logger(&Stderr).is_ok() {
        log::set_max_level(level(setting.as_deref()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_level_comes_from_the_setting() {
        assert_eq!(level(None), LevelFilter::Warn);
        assert_eq!(level(Some("info")), LevelFilter::Info);
        assert_eq!(level(Some(" DEBUG ")), LevelFilter::Debug);
        assert_eq!(level(Some("off")), LevelFilter::Off);
        assert_eq!(level(Some("loud")), LevelFilter::Warn);
    }

    #[test]
    fn a_line_holds_no_escape_sequence_a_game_wrote() {
        let name = "Evil\u{1b}]0;owned\u{7}\u{202e}\nfastemu: error: fake";
        let shown = line(Level::Warn, &format_args!("could not start {name}"));
        assert_eq!(
            shown,
            "fastemu: warn: could not start Evil\u{fffd}]0;owned\u{fffd}\u{fffd}\\nfastemu: error: fake"
        );
        assert_eq!(
            line(Level::Error, &format_args!("plain {}", 7)),
            "fastemu: error: plain 7"
        );
    }
}
