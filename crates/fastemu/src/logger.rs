//! A logger that writes to the terminal, so the warnings of the app and
//! its views (a damaged settings file, no audio device) show somewhere.
//! Warnings and errors by default; `FASTEMU_LOG=info` (or `debug`,
//! `trace`, `off`) picks another level.

use log::{LevelFilter, Log, Metadata, Record};

struct Stderr;

impl Log for Stderr {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let level = record.level().as_str().to_ascii_lowercase();
            eprintln!("fastemu: {level}: {}", record.args());
        }
    }

    fn flush(&self) {}
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
}
