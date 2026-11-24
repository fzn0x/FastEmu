//! UTF-8 exactly as the JDK converts it, malformed input included.

use super::REPLACEMENT;

fn is_continuation(b: u8) -> bool {
    b & 0xC0 == 0x80
}

/// The second byte rules out a three-byte sequence (JDK `isMalformed3_2`).
fn malformed3_2(b1: u8, b2: u8) -> bool {
    (b1 == 0xE0 && b2 & 0xE0 == 0x80) || !is_continuation(b2)
}

/// The second byte rules out a four-byte sequence (JDK `isMalformed4_2`).
fn malformed4_2(b1: u8, b2: u8) -> bool {
    (b1 == 0xF0 && !(0x90..=0xBF).contains(&b2))
        || (b1 == 0xF4 && b2 & 0xF0 != 0x80)
        || !is_continuation(b2)
}

/// How many bytes of a bad three-byte sequence one U+FFFD replaces.
fn malformed3_len(b1: u8, b2: u8) -> usize {
    if malformed3_2(b1, b2) { 1 } else { 2 }
}

/// How many bytes of a bad four-byte sequence one U+FFFD replaces.
fn malformed4_len(b1: u8, b2: u8, b3: u8) -> usize {
    if b1 > 0xF4 || malformed4_2(b1, b2) {
        1
    } else if !is_continuation(b3) {
        2
    } else {
        3
    }
}

/// UTF-8 as the JDK decodes it (`String(byte[], UTF_8)` and its
/// `CharsetDecoder`), so each malformed sequence becomes the same number
/// of U+FFFD as in Java: a whole surrogate encoded in three bytes is one,
/// and so is a sequence cut off at the end of the input. Unless `end` is
/// set, such a cut-off sequence is left unread instead. Returns the bytes
/// used.
pub(super) fn decode(bytes: &[u8], end: bool, out: &mut Vec<u16>) -> usize {
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        let b1 = bytes[i];
        if b1 < 0x80 {
            out.push(u16::from(b1));
            i += 1;
        } else if b1 & 0xE0 == 0xC0 && b1 & 0x1E != 0 {
            if i + 1 < n {
                let b2 = bytes[i + 1];
                if is_continuation(b2) {
                    out.push((u16::from(b1 & 0x1F) << 6) | u16::from(b2 & 0x3F));
                    i += 2;
                } else {
                    out.push(REPLACEMENT);
                    i += 1;
                }
                continue;
            }
            if !end {
                return i;
            }
            out.push(REPLACEMENT);
            return n;
        } else if b1 & 0xF0 == 0xE0 {
            if i + 2 < n {
                let (b2, b3) = (bytes[i + 1], bytes[i + 2]);
                if malformed3_2(b1, b2) || !is_continuation(b3) {
                    out.push(REPLACEMENT);
                    i += malformed3_len(b1, b2);
                } else {
                    let c = (u16::from(b1 & 0x0F) << 12)
                        | (u16::from(b2 & 0x3F) << 6)
                        | u16::from(b3 & 0x3F);
                    let surrogate = (0xD800..0xE000).contains(&c);
                    out.push(if surrogate { REPLACEMENT } else { c });
                    i += 3;
                }
                continue;
            }
            if i + 1 < n && malformed3_2(b1, bytes[i + 1]) {
                out.push(REPLACEMENT);
                i += 1;
                continue;
            }
            if !end {
                return i;
            }
            out.push(REPLACEMENT);
            return n;
        } else if b1 & 0xF8 == 0xF0 {
            if i + 3 < n {
                let (b2, b3, b4) = (bytes[i + 1], bytes[i + 2], bytes[i + 3]);
                let uc = (u32::from(b1 & 0x07) << 18)
                    | (u32::from(b2 & 0x3F) << 12)
                    | (u32::from(b3 & 0x3F) << 6)
                    | u32::from(b4 & 0x3F);
                let valid = is_continuation(b2)
                    && is_continuation(b3)
                    && is_continuation(b4)
                    && (0x1_0000..=0x10_FFFF).contains(&uc);
                if valid {
                    let v = uc - 0x1_0000;
                    out.push(0xD800 | (v >> 10) as u16);
                    out.push(0xDC00 | (v & 0x3FF) as u16);
                    i += 4;
                } else {
                    out.push(REPLACEMENT);
                    i += malformed4_len(b1, b2, b3);
                }
                continue;
            }
            if b1 > 0xF4 || (i + 1 < n && malformed4_2(b1, bytes[i + 1])) {
                out.push(REPLACEMENT);
                i += 1;
                continue;
            }
            if i + 2 < n && !is_continuation(bytes[i + 2]) {
                out.push(REPLACEMENT);
                i += 2;
                continue;
            }
            if !end {
                return i;
            }
            out.push(REPLACEMENT);
            return n;
        } else {
            out.push(REPLACEMENT);
            i += 1;
        }
    }
    n
}

/// UTF-8 with each unpaired surrogate encoded as `?`, as Java does.
pub(super) fn encode(chars: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(chars.len());
    for unit in char::decode_utf16(chars.iter().copied()) {
        match unit {
            Ok(c) => {
                let mut buf = [0; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
            Err(_) => out.push(b'?'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn decode_all(bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::new();
        assert_eq!(decode(bytes, true, &mut out), bytes.len());
        out
    }

    #[test]
    fn utf8_round_trips() {
        let text = utf16("h\u{e9}llo \u{1F600} \u{20ac}");
        assert_eq!(decode_all(&encode(&text)), text);
        assert_eq!(encode(&[0xD800]), b"?");
        assert_eq!(encode(&[0xDC00, 0x41]), b"?A");
    }

    /// Each malformed input decodes as `new String(bytes, "UTF-8")` does
    /// on JDK 17, where every case was checked.
    #[test]
    fn malformed_utf8_is_replaced_as_java_replaces_it() {
        let cases: &[(&[u8], &str)] = &[
            (&[0x41, 0xFF, 0x42], "A\u{FFFD}B"),
            // A surrogate encoded in three bytes is one bad sequence.
            (&[0xED, 0xA0, 0x80, 0x41], "\u{FFFD}A"),
            // So is a cut-off sequence at the end, whatever its length.
            (&[0x41, 0xE2, 0x82], "A\u{FFFD}"),
            (&[0xED, 0xA0], "\u{FFFD}"),
            (&[0xF0, 0x9F, 0x98], "\u{FFFD}"),
            (&[0xE0, 0xA0], "\u{FFFD}"),
            // A cut-off sequence before other bytes.
            (&[0xE2, 0x82, 0x41], "\u{FFFD}A"),
            (&[0xF0, 0x9F, 0x41], "\u{FFFD}A"),
            (&[0xF0, 0x9F, 0x41, 0x42, 0x43], "\u{FFFD}ABC"),
            (&[0xF0, 0x9F, 0x98, 0x41], "\u{FFFD}A"),
            (&[0xC3, 0x41], "\u{FFFD}A"),
            // Overlong forms and values past U+10FFFF: byte by byte.
            (&[0xC0, 0x80], "\u{FFFD}\u{FFFD}"),
            (&[0xE0, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}"),
            (&[0xE0, 0x80], "\u{FFFD}\u{FFFD}"),
            (
                &[0xF0, 0x80, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (
                &[0xF4, 0x90, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (&[0xF5, 0x80], "\u{FFFD}\u{FFFD}"),
            (
                &[0xF8, 0x88, 0x80, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (&[0x80, 0xBF], "\u{FFFD}\u{FFFD}"),
            // Good sequences around the edges.
            (&[0xF0, 0x9F, 0x98, 0x80], "\u{1F600}"),
            (&[0xF0, 0x9F, 0x98, 0x80, 0x80], "\u{1F600}\u{FFFD}"),
            (&[0xEF, 0xBF, 0xBF], "\u{FFFF}"),
        ];
        for (bytes, expected) in cases {
            let decoded = String::from_utf16(&decode_all(bytes)).unwrap();
            assert_eq!(decoded, *expected, "{bytes:02X?}");
        }
    }

    #[test]
    fn utf8_waits_for_the_rest_of_a_sequence_unless_at_the_end() {
        let mut out = Vec::new();
        assert_eq!(decode(&[0x41, 0xE2, 0x82], false, &mut out), 1);
        assert_eq!(out, utf16("A"));
        out.clear();
        assert_eq!(decode(&[0xE2, 0x82, 0xAC], false, &mut out), 3);
        assert_eq!(out, utf16("\u{20ac}"));
        // A second byte that cannot continue the sequence decides now.
        out.clear();
        assert_eq!(decode(&[0xE2, 0x41], false, &mut out), 2);
        assert_eq!(out, utf16("\u{FFFD}A"));
        out.clear();
        assert_eq!(decode(&[0xF0, 0x9F], false, &mut out), 0);
        assert!(out.is_empty());
    }
}
