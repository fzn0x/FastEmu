//! HTTP dates, for `getDate`, `getExpiration`, `getLastModified` and
//! `getHeaderFieldDate`.

/// Parses an HTTP date as milliseconds since 1970-01-01 GMT.
///
/// Accepts the three forms RFC 2616 lists, which servers of the phones'
/// era all sent: `Sun, 06 Nov 1994 08:49:37 GMT` (RFC 1123),
/// `Sunday, 06-Nov-94 08:49:37 GMT` (RFC 850, a two-digit year below 70
/// meaning 20xx) and `Sun Nov  6 08:49:37 1994` (asctime). The weekday
/// and the zone name are ignored: HTTP dates are always GMT.
pub(crate) fn parse(text: &str) -> Option<i64> {
    let (mut day, mut month, mut year, mut time) = (None, None, None, None);
    for token in text.split([' ', ',', '-']).filter(|t| !t.is_empty()) {
        if token.contains(':') {
            time = Some(parse_time(token)?);
        } else if let Some(m) = month_number(token) {
            month = Some(m);
        } else if token.bytes().all(|b| b.is_ascii_digit()) {
            let n: i64 = token.parse().ok()?;
            if token.len() <= 2 && day.is_none() {
                day = Some(n);
            } else if token.len() == 2 {
                year = Some(if n < 70 { 2000 + n } else { 1900 + n });
            } else {
                year = Some(n);
            }
        }
    }
    let (day, month, year, (h, m, s)) = (day?, month?, year?, time?);
    if !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(((days * 24 + h) * 60 + m) * 60_000 + s * 1000)
}

/// `hh:mm:ss` as its three numbers.
fn parse_time(token: &str) -> Option<(i64, i64, i64)> {
    let mut parts = token.split(':').map(|p| p.parse::<i64>().ok());
    let (h, m, s) = (parts.next()??, parts.next()??, parts.next()??);
    let valid = parts.next().is_none() && h < 24 && m < 60 && s <= 60;
    (valid && h >= 0 && m >= 0 && s >= 0).then_some((h, m, s))
}

/// The month an English month name (or its first three letters) names,
/// 1 to 12.
fn month_number(token: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let prefix = token.get(..3)?.to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|&m| m == prefix)
        .map(|i| i as i64 + 1)
}

/// Days from 1970-01-01 to the given date in the proleptic Gregorian
/// calendar (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::parse;

    /// 1994-11-06 08:49:37 GMT, the example of RFC 2616.
    const EXAMPLE: i64 = 784_111_777_000;

    #[test]
    fn the_three_http_date_forms_parse() {
        assert_eq!(parse("Sun, 06 Nov 1994 08:49:37 GMT"), Some(EXAMPLE));
        assert_eq!(parse("Sunday, 06-Nov-94 08:49:37 GMT"), Some(EXAMPLE));
        assert_eq!(parse("Sun Nov  6 08:49:37 1994"), Some(EXAMPLE));
    }

    #[test]
    fn edges_of_the_calendar_parse() {
        assert_eq!(parse("Thu, 01 Jan 1970 00:00:00 GMT"), Some(0));
        assert_eq!(
            parse("Tue, 29 Feb 2028 23:59:59 GMT"),
            Some(1_835_481_599_000)
        );
        assert_eq!(
            parse("Monday, 01-Jan-01 00:00:00 GMT"),
            Some(978_307_200_000)
        );
    }

    #[test]
    fn malformed_dates_give_nothing() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("yesterday"), None);
        assert_eq!(parse("Sun, 06 Nov 1994 GMT"), None);
        assert_eq!(parse("Sun, 06 Nov 1994 25:00:00 GMT"), None);
        assert_eq!(parse("Sun, 40 Nov 1994 08:49:37 GMT"), None);
    }
}
