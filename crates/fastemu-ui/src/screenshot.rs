//! Saving a game's screen as a PNG file.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fastemu_core::Frame;

use crate::pixels::encode_png;

/// Writes `frame` as `<dir>/<title>-<date>-<time>.png` and returns the
/// path. The time is UTC; a second shot within the same second gets a
/// `-2`, `-3` suffix instead of replacing the first.
pub fn save(dir: &Path, title: &str, frame: &Frame, now: SystemTime) -> io::Result<PathBuf> {
    let png = encode_png(frame).map_err(io::Error::other)?;
    fs::create_dir_all(dir)?;
    let stem = format!("{}-{}", file_stem(title), timestamp(now));
    for n in 1..=MAX_SAME_SECOND {
        let name = if n == 1 {
            format!("{stem}.png")
        } else {
            format!("{stem}-{n}.png")
        };
        let path = dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(&png)?;
                return Ok(path);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{MAX_SAME_SECOND} screenshots named {stem} exist already"),
    ))
}

/// How many shots of one game one second can hold before saving gives up.
const MAX_SAME_SECOND: u32 = 1000;

/// A file name made from a game title: letters, digits, `-` and `_` kept,
/// runs of anything else turned into one `-`.
pub fn file_stem(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for c in title.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "screenshot".to_owned()
    } else {
        trimmed.chars().take(64).collect()
    }
}

/// `YYYYMMDD-HHMMSS` in UTC.
pub fn timestamp(now: SystemTime) -> String {
    let secs = now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}{m:02}{d:02}-{:02}{:02}{:02}",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// The proleptic Gregorian date of a day count since 1970-01-01, after
/// Howard Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn timestamps_are_utc_dates() {
        assert_eq!(timestamp(UNIX_EPOCH), "19700101-000000");
        let t = UNIX_EPOCH + Duration::from_secs(951_782_400 + 3_723);
        assert_eq!(timestamp(t), "20000229-010203");
        let t = UNIX_EPOCH + Duration::from_secs(1_795_000_000);
        assert_eq!(timestamp(t), "20261118-110640");
    }

    #[test]
    fn titles_become_safe_file_names() {
        assert_eq!(file_stem("Snake 2: Revenge!"), "Snake-2-Revenge");
        assert_eq!(file_stem("../../etc"), "etc");
        assert_eq!(file_stem("???"), "screenshot");
        assert_eq!(file_stem("Ёлки"), "Ёлки");
    }

    #[test]
    fn saving_never_overwrites() {
        let dir = tempfile::tempdir().expect("temp dir");
        let frame = Frame::filled(4, 3, 0xFF12_3456, 1);
        let now = UNIX_EPOCH + Duration::from_secs(60);
        let first = save(dir.path(), "Snake", &frame, now).expect("saves");
        let second = save(dir.path(), "Snake", &frame, now).expect("saves");
        assert_eq!(
            first.file_name().and_then(|n| n.to_str()),
            Some("Snake-19700101-000100.png")
        );
        assert_eq!(
            second.file_name().and_then(|n| n.to_str()),
            Some("Snake-19700101-000100-2.png")
        );
        let bytes = fs::read(&first).expect("reads");
        assert_eq!(&bytes[1..4], b"PNG");
    }
}
