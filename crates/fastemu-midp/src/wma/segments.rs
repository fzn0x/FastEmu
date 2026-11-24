//! How many SMS segments a message needs, for
//! `MessageConnection.numberOfSegments`.
//!
//! One SMS carries 140 octets. Text goes as GSM 7-bit (160 characters)
//! when every character is in the GSM 03.38 alphabet, and as UCS-2 (70)
//! otherwise; binary payloads go as 8-bit octets. A user data header
//! takes room from every segment: 7 octets for a 16-bit port address
//! (JSR-120 addresses ports that way), 7 for the concatenation of a long
//! message (16-bit reference), 13 for both. JSR-120 asks implementations
//! to send messages of up to three segments; a longer one cannot be sent,
//! which `numberOfSegments` reports as 0.

/// The octets one SMS carries.
const SMS_OCTETS: usize = 140;
/// A user data header with one 16-bit port or concatenation element:
/// its length octet, then the element (identifier, length, 4 octets).
const ONE_ELEMENT: usize = 7;
/// A user data header with both elements.
const TWO_ELEMENTS: usize = 13;
/// The most segments a message may take.
const MAX_SEGMENTS: usize = 3;

/// The GSM 03.38 default alphabet, one septet each (the escape to the
/// extension table left out).
const GSM_BASIC: &str = "@\u{a3}$\u{a5}\u{e8}\u{e9}\u{f9}\u{ec}\u{f2}\u{c7}\n\u{d8}\u{f8}\r\u{c5}\u{e5}\
    \u{394}_\u{3a6}\u{393}\u{39b}\u{3a9}\u{3a0}\u{3a8}\u{3a3}\u{398}\u{39e}\u{c6}\u{e6}\u{df}\u{c9}\
    \u{20}!\"#\u{a4}%&'()*+,-./0123456789:;<=>?\
    \u{a1}ABCDEFGHIJKLMNOPQRSTUVWXYZ\u{c4}\u{d6}\u{d1}\u{dc}\u{a7}\
    \u{bf}abcdefghijklmnopqrstuvwxyz\u{e4}\u{f6}\u{f1}\u{fc}\u{e0}";

/// The GSM 03.38 extension table, two septets each.
const GSM_EXTENSION: &str = "\u{c}^{}\\[~]|\u{20ac}";

/// The septets `unit` takes in the GSM 7-bit alphabet, or `None` when the
/// alphabet does not have it.
fn septets(unit: u16) -> Option<usize> {
    let c = char::from_u32(u32::from(unit))?;
    if GSM_BASIC.contains(c) {
        Some(1)
    } else if GSM_EXTENSION.contains(c) {
        Some(2)
    } else {
        None
    }
}

/// The segments `units` items need when one segment holds `single` alone
/// or `each` as part of a long message; 0 beyond [`MAX_SEGMENTS`].
fn count(units: usize, single: usize, each: usize) -> i32 {
    let segments = if units <= single {
        1
    } else {
        units.div_ceil(each)
    };
    if segments > MAX_SEGMENTS {
        0
    } else {
        segments as i32
    }
}

/// Octets left for the payload of one segment, alone and as part of a
/// long message.
fn room(with_port: bool) -> (usize, usize) {
    if with_port {
        (SMS_OCTETS - ONE_ELEMENT, SMS_OCTETS - TWO_ELEMENTS)
    } else {
        (SMS_OCTETS, SMS_OCTETS - ONE_ELEMENT)
    }
}

/// The segments a text message with `text` (UTF-16) needs.
pub(crate) fn text(text: &[u16], with_port: bool) -> i32 {
    let (single, each) = room(with_port);
    let gsm: Option<usize> = text.iter().map(|&u| septets(u)).sum();
    match gsm {
        Some(septets) => count(septets, single * 8 / 7, each * 8 / 7),
        None => count(text.len(), single / 2, each / 2),
    }
}

/// The segments a binary message of `len` bytes needs.
pub(crate) fn binary(len: usize, with_port: bool) -> i32 {
    let (single, each) = room(with_port);
    count(len, single, each)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn the_basic_alphabet_has_128_characters_less_the_escape() {
        assert_eq!(GSM_BASIC.chars().count(), 127);
        assert_eq!(GSM_EXTENSION.chars().count(), 10);
    }

    #[test]
    fn gsm_text_fills_160_septets_then_152_per_segment() {
        assert_eq!(text(&utf16(""), false), 1);
        assert_eq!(text(&utf16(&"a".repeat(160)), false), 1);
        assert_eq!(text(&utf16(&"a".repeat(161)), false), 2);
        assert_eq!(text(&utf16(&"a".repeat(304)), false), 2);
        assert_eq!(text(&utf16(&"a".repeat(456)), false), 3);
        assert_eq!(text(&utf16(&"a".repeat(457)), false), 0);
        assert_eq!(text(&utf16(&"a".repeat(152)), true), 1);
        assert_eq!(text(&utf16(&"a".repeat(153)), true), 2);
        assert_eq!(text(&utf16(&"a".repeat(290)), true), 2);
    }

    #[test]
    fn extension_characters_take_two_septets() {
        assert_eq!(text(&utf16(&"\u{20ac}".repeat(80)), false), 1);
        assert_eq!(text(&utf16(&"[".repeat(81)), false), 2);
    }

    #[test]
    fn other_text_goes_as_ucs2() {
        assert_eq!(text(&utf16(&"\u{416}".repeat(70)), false), 1);
        assert_eq!(text(&utf16(&"\u{416}".repeat(71)), false), 2);
        assert_eq!(text(&utf16(&"\u{416}".repeat(66)), true), 1);
        assert_eq!(text(&utf16(&"\u{416}".repeat(126)), true), 2);
        assert_eq!(text(&utf16(&"\u{416}".repeat(127)), true), 3);
    }

    #[test]
    fn binary_payloads_count_octets() {
        assert_eq!(binary(140, false), 1);
        assert_eq!(binary(141, false), 2);
        assert_eq!(binary(133, true), 1);
        assert_eq!(binary(134, true), 2);
        assert_eq!(binary(399, false), 3);
        assert_eq!(binary(400, false), 0);
    }
}
