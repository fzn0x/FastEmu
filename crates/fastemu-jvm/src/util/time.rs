//! Calendar arithmetic and the host's time zone, for `Date`, `TimeZone`
//! and `Calendar`.
//!
//! Dates are proleptic Gregorian: the JDK switches to the Julian
//! calendar before October 1582, which no game reaches.

use jiff::Timestamp;
use jiff::tz::{AmbiguousOffset, Offset, TimeZone};

/// Milliseconds in a day.
pub(super) const DAY_MS: i64 = 86_400_000;

/// The days from 1970-01-01 to `year`-`month`-`day` (`month` from 1).
pub(super) fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The year, month (from 1) and day of `days` after 1970-01-01.
pub(super) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Java's `DAY_OF_WEEK` of `days` after 1970-01-01: `SUNDAY` is 1, and
/// 1970-01-01 was a Thursday (5).
pub(super) fn day_of_week(days: i64) -> i64 {
    (days + 4).rem_euclid(7) + 1
}

pub(super) fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// The days in `month` (from 0) of `year`.
pub(super) fn month_length(year: i64, month: i64) -> i64 {
    match month {
        1 => 28 + i64::from(is_leap(year)),
        3 | 5 | 8 | 10 => 30,
        _ => 31,
    }
}

fn millis(offset: Offset) -> i64 {
    i64::from(offset.seconds()) * 1000
}

/// The host's offset from UTC at the instant `utc` (milliseconds since
/// the epoch), daylight saving time included. A host whose zone cannot
/// be read counts as UTC.
pub(super) fn host_offset_at(utc: i64) -> i64 {
    Timestamp::from_millisecond(utc).map_or(0, |t| millis(TimeZone::system().to_offset(t)))
}

/// The host's offset for the wall-clock time `local` (milliseconds since
/// the epoch as if it were UTC): in the hour a clock repeats, and in the
/// hour it skips, the offset from before the change.
pub(super) fn host_offset_for_local(local: i64) -> i64 {
    let Ok(t) = Timestamp::from_millisecond(local) else {
        return 0;
    };
    let wall = TimeZone::UTC.to_datetime(t);
    match TimeZone::system().to_ambiguous_timestamp(wall).offset() {
        AmbiguousOffset::Unambiguous { offset }
        | AmbiguousOffset::Gap { before: offset, .. }
        | AmbiguousOffset::Fold { before: offset, .. } => millis(offset),
    }
}

/// The host's standard offset: the smaller of its offsets in January and
/// July of this year, since daylight saving time adds to it.
pub(super) fn host_raw_offset() -> i64 {
    let (jan, jul) = host_seasons();
    jan.min(jul)
}

/// Whether the host's clocks change for daylight saving time.
pub(super) fn host_uses_daylight_time() -> bool {
    let (jan, jul) = host_seasons();
    jan != jul
}

/// The host's offsets on 1 January and 1 July of this year.
fn host_seasons() -> (i64, i64) {
    let now = crate::lang::system::current_time_millis();
    let (year, _, _) = civil_from_days(now.div_euclid(DAY_MS));
    let jan = days_from_civil(year, 1, 1) * DAY_MS;
    let jul = days_from_civil(year, 7, 1) * DAY_MS;
    (host_offset_at(jan), host_offset_at(jul))
}

/// A `GMT+hh:mm` time zone ID for `offset` milliseconds, as Java writes
/// custom IDs.
pub(super) fn custom_id(offset: i64) -> String {
    let sign = if offset < 0 { '-' } else { '+' };
    let minutes = offset.abs() / 60_000;
    format!("GMT{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The offset a custom ID such as `GMT+7`, `GMT+07:00` or `GMT-0530`
/// names, as `TimeZone.getTimeZone` parses it.
pub(super) fn parse_custom_id(id: &str) -> Option<i64> {
    let rest = id.strip_prefix("GMT")?;
    let negative = match rest.as_bytes().first()? {
        b'+' => false,
        b'-' => true,
        _ => return None,
    };
    let rest = &rest[1..];
    let digits = |s: &str| !s.is_empty() && s.len() <= 2 && s.bytes().all(|b| b.is_ascii_digit());
    let (hours, minutes) = match rest.split_once(':') {
        Some((h, m)) if digits(h) && m.len() == 2 && digits(m) => (h, m),
        Some(_) => return None,
        None if rest.len() <= 2 && digits(rest) => (rest, "0"),
        None if rest.len() == 4 && rest.bytes().all(|b| b.is_ascii_digit()) => {
            (&rest[..2], &rest[2..])
        }
        None => return None,
    };
    let (hours, minutes): (i64, i64) = (hours.parse().ok()?, minutes.parse().ok()?);
    if hours > 23 || minutes > 59 {
        return None;
    }
    let offset = (hours * 60 + minutes) * 60_000;
    Some(if negative { -offset } else { offset })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates_round_trip() {
        for days in [
            -1_000_000, -719_468, -1, 0, 1, 59, 60, 11_016, 20_000, 2_932_896,
        ] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(days_from_civil(2026, 11, 24), 20_781);
        assert_eq!(day_of_week(20_781), 3);
        assert_eq!(day_of_week(0), 5);
    }

    #[test]
    fn lenient_days_count_on() {
        assert_eq!(days_from_civil(2025, 1, 32), days_from_civil(2025, 2, 1));
        assert_eq!(days_from_civil(2025, 3, 0), days_from_civil(2025, 2, 28));
        assert_eq!(month_length(2024, 1), 29);
        assert_eq!(month_length(1900, 1), 28);
        assert_eq!(month_length(2000, 1), 29);
    }

    #[test]
    fn custom_ids_parse_as_java_does() {
        assert_eq!(parse_custom_id("GMT+7"), Some(7 * 3_600_000));
        assert_eq!(parse_custom_id("GMT+07:00"), Some(7 * 3_600_000));
        assert_eq!(parse_custom_id("GMT-0530"), Some(-(5 * 60 + 30) * 60_000));
        assert_eq!(parse_custom_id("GMT+1:5"), None);
        assert_eq!(parse_custom_id("GMT+24"), None);
        assert_eq!(parse_custom_id("GMT"), None);
        assert_eq!(parse_custom_id("UTC+1"), None);
        assert_eq!(custom_id(-(5 * 60 + 30) * 60_000), "GMT-05:30");
        assert_eq!(custom_id(0), "GMT+00:00");
    }

    #[test]
    fn the_host_offset_is_consistent() {
        let now = crate::lang::system::current_time_millis();
        let offset = host_offset_at(now);
        assert_eq!(host_offset_for_local(now + offset), offset);
        assert!(host_raw_offset() <= offset);
    }
}
