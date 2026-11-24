//! `Float.toString`, `Double.toString` and their parsers, as Java prints
//! and reads numbers.
//!
//! Printing uses the shortest digits that read back as the same value
//! (Rust's formatter finds them) laid out by Java's rules: plain notation
//! from 10^-3 up to 10^7, computerized scientific notation (`1.0E10`)
//! outside it, and always a digit after the point.

/// The shortest round-trip digits of a finite, positive value and the
/// decimal exponent of the first digit: 123.4 gives ("1234", 2).
fn shortest_digits(scientific: &str) -> (String, i32) {
    let (mantissa, exp) = scientific
        .split_once('e')
        .expect("LowerExp output has an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exp = exp.parse().expect("LowerExp prints a decimal exponent");
    (digits, exp)
}

fn layout(negative: bool, digits: &str, exp: i32, plain: bool) -> String {
    let mut out = String::with_capacity(digits.len() + 8);
    if negative {
        out.push('-');
    }
    if plain {
        if exp >= 0 {
            let int_len = exp as usize + 1;
            if digits.len() <= int_len {
                out.push_str(digits);
                out.extend(std::iter::repeat_n('0', int_len - digits.len()));
                out.push_str(".0");
            } else {
                out.push_str(&digits[..int_len]);
                out.push('.');
                out.push_str(&digits[int_len..]);
            }
        } else {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-exp - 1) as usize));
            out.push_str(digits);
        }
    } else {
        out.push_str(&digits[..1]);
        out.push('.');
        if digits.len() > 1 {
            out.push_str(&digits[1..]);
        } else {
            out.push('0');
        }
        out.push('E');
        out.push_str(&exp.to_string());
    }
    out
}

/// `Double.toString(double)`.
pub(crate) fn double_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.to_owned();
    }
    let abs = v.abs();
    // Java prints the smallest subnormal with two digits.
    let (digits, exp) = if abs.to_bits() == 1 {
        ("49".to_owned(), -324)
    } else {
        shortest_digits(&format!("{abs:e}"))
    };
    layout(v < 0.0, &digits, exp, (1e-3..1e7).contains(&abs))
}

/// `Float.toString(float)`.
pub(crate) fn float_to_string(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.to_owned();
    }
    let abs = v.abs();
    let (digits, exp) = if abs.to_bits() == 1 {
        ("14".to_owned(), -45)
    } else {
        shortest_digits(&format!("{abs:e}"))
    };
    layout(v < 0.0, &digits, exp, (1e-3..1e7).contains(&abs))
}

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
    fn doubles_print_like_java() {
        let cases = [
            (0.1, "0.1"),
            (1.0, "1.0"),
            (-2.5, "-2.5"),
            (100.0, "100.0"),
            (1.0e7, "1.0E7"),
            (9_999_999.0, "9999999.0"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (1.0 / 3.0, "0.3333333333333333"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123_456_789.0, "1.23456789E8"),
            (1e21, "1.0E21"),
            (f64::MAX, "1.7976931348623157E308"),
            (f64::from_bits(1), "4.9E-324"),
            (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
            (-0.0, "-0.0"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for (v, want) in cases {
            assert_eq!(double_to_string(v), want, "{v:e}");
        }
    }

    #[test]
    fn floats_print_like_java() {
        let cases = [
            (0.1f32, "0.1"),
            (1.0 / 3.0, "0.33333334"),
            (3.0e10, "3.0E10"),
            (1.0e-5, "1.0E-5"),
            (f32::MAX, "3.4028235E38"),
            (f32::from_bits(1), "1.4E-45"),
            (16_777_216.0, "1.6777216E7"),
            (123.456, "123.456"),
        ];
        for (v, want) in cases {
            assert_eq!(float_to_string(v), want, "{v:e}");
        }
    }

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
