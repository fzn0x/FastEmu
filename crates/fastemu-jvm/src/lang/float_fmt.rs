//! `Float.toString` and `Double.toString` (in [`super::dtoa`]) and the
//! parsers `Double.parseDouble` and `Float.parseFloat`, which read numbers
//! as Java does.

pub(crate) use super::dtoa::{double_to_string, float_to_string};

/// Checks `s` against Java's decimal floating-point grammar and returns
/// it without its type suffix, ready for Rust's parser.
fn java_decimal(s: &str) -> Option<&str> {
    let body = s.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(s);
    let b = body.as_bytes();
    let mut i = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - int_start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - frac_start;
    }
    if digits == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exp_start {
            return None;
        }
    }
    (i == b.len()).then_some(body)
}

/// A hexadecimal literal such as `0x1.8p1`, as a double.
fn java_hex(s: &str) -> Option<f64> {
    let body = s.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(s);
    let (negative, body) = match body.as_bytes().first() {
        Some(b'-') => (true, &body[1..]),
        Some(b'+') => (false, &body[1..]),
        _ => (false, body),
    };
    let body = body
        .strip_prefix("0x")
        .or_else(|| body.strip_prefix("0X"))?;
    let (mantissa, exp) = body.split_once(['p', 'P'])?;
    let exp: i32 = exp.parse().ok()?;
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if int_part.is_empty() && frac_part.is_empty() {
        return None;
    }
    let mut value: u64 = 0;
    let mut scale: i32 = 0;
    let mut sticky = false;
    for (i, c) in int_part.chars().chain(frac_part.chars()).enumerate() {
        let d = u64::from(c.to_digit(16)?);
        if value >> 59 == 0 {
            value = value << 4 | d;
            if i >= int_part.len() {
                scale -= 4;
            }
        } else {
            sticky |= d != 0;
            if i < int_part.len() {
                scale += 4;
            }
        }
    }
    let value = (value | u64::from(sticky)) as f64 * 2f64.powi(scale);
    let v = value * 2f64.powi(exp.clamp(-2000, 2000));
    Some(if negative { -v } else { v })
}

/// `Double.parseDouble`: `None` means `NumberFormatException`.
pub(crate) fn parse_double(s: &str) -> Option<f64> {
    let s = s.trim_matches(|c: char| c <= ' ');
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let negative = s.starts_with('-');
    match unsigned {
        "NaN" => return Some(f64::NAN),
        "Infinity" => {
            return Some(if negative {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            });
        }
        _ => {}
    }
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        return java_hex(s);
    }
    java_decimal(s)?.parse().ok()
}

/// `Float.parseFloat`, rounding the decimal straight to `float`.
pub(crate) fn parse_float(s: &str) -> Option<f32> {
    let s = s.trim_matches(|c: char| c <= ' ');
    let unsigned = s.strip_prefix(['+', '-']).unwrap_or(s);
    let negative = s.starts_with('-');
    match unsigned {
        "NaN" => return Some(f32::NAN),
        "Infinity" => {
            return Some(if negative {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            });
        }
        _ => {}
    }
    if unsigned.starts_with("0x") || unsigned.starts_with("0X") {
        return java_hex(s).map(|d| d as f32);
    }
    java_decimal(s)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_follows_java_grammar() {
        assert_eq!(parse_double("  1.5  "), Some(1.5));
        assert_eq!(parse_double("1e3"), Some(1000.0));
        assert_eq!(parse_double(".5"), Some(0.5));
        assert_eq!(parse_double("5."), Some(5.0));
        assert_eq!(parse_double("2.5d"), Some(2.5));
        assert_eq!(parse_double("-Infinity"), Some(f64::NEG_INFINITY));
        assert!(parse_double("NaN").unwrap().is_nan());
        assert_eq!(parse_double("0x1.8p1"), Some(3.0));
        assert_eq!(parse_double("inf"), None);
        assert_eq!(parse_double("nan"), None);
        assert_eq!(parse_double("1e"), None);
        assert_eq!(parse_double("."), None);
        assert_eq!(parse_double(""), None);
        assert_eq!(parse_double("1_000"), None);
        assert_eq!(parse_float("3.4028236e38"), Some(f32::INFINITY));
        assert_eq!(parse_float("0.1f"), Some(0.1));
    }
}
