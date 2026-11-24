//! The character encodings of `String` and the streams: UTF-8,
//! ISO-8859-1, US-ASCII and windows-1251, each converting exactly as the
//! JDK's charset of that name does, malformed and unmappable input
//! included.

mod tables;

use std::sync::OnceLock;

/// A supported encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Charset {
    Utf8,
    Latin1,
    Ascii,
    Cp1251,
}

/// U+FFFD, what undecodable bytes become.
const REPLACEMENT: u16 = 0xFFFD;

impl Charset {
    /// The charset `name` names, ignoring case, `-` and `_`, and
    /// accepting the JDK's common aliases.
    pub(crate) fn lookup(name: &str) -> Option<Charset> {
        let key: String = name
            .chars()
            .filter(|c| !matches!(c, '-' | '_'))
            .flat_map(char::to_lowercase)
            .collect();
        Some(match key.as_str() {
            "utf8" | "unicode11utf8" => Charset::Utf8,
            "iso88591" | "latin1" | "l1" | "cp819" | "ibm819" | "885911987" | "iso885911987"
            | "88591" | "iso819" | "csisolatin1" | "isoir100" => Charset::Latin1,
            "usascii" | "ascii" | "ascii7" | "646" | "iso646us" | "csascii" | "default" => {
                Charset::Ascii
            }
            "windows1251" | "cp1251" | "win1251" | "cp5347" | "ansi1251" => Charset::Cp1251,
            _ => return None,
        })
    }

    /// The charset of conversions that name none, from the value of
    /// `microedition.encoding`; UTF-8 when it names no charset this
    /// virtual machine has.
    pub(crate) fn default_for(name: Option<&str>) -> Charset {
        name.and_then(Charset::lookup).unwrap_or(Charset::Utf8)
    }

    /// The position of a single-byte code page in [`tables::HIGH`],
    /// `None` for the charsets that are not table driven.
    fn table(self) -> Option<usize> {
        match self {
            Charset::Cp1251 => Some(0),
            Charset::Utf8 | Charset::Latin1 | Charset::Ascii => None,
        }
    }

    /// Decodes all of `bytes`.
    pub(crate) fn decode(self, bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::with_capacity(bytes.len());
        self.decode_into(bytes, true, &mut out);
        out
    }

    /// Decodes `bytes` onto `out` and returns how many bytes it used.
    /// Unless `end` is set, a multi-byte sequence cut off at the end of
    /// `bytes` is left for the next call (a reader's next read); with
    /// `end`, it becomes one U+FFFD, as Java decodes the end of its input.
    pub(crate) fn decode_into(self, bytes: &[u8], end: bool, out: &mut Vec<u16>) -> usize {
        match self {
            Charset::Utf8 => decode_utf8(bytes, end, out),
            Charset::Latin1 => {
                out.extend(bytes.iter().map(|&b| u16::from(b)));
                bytes.len()
            }
            Charset::Ascii => {
                out.extend(
                    bytes
                        .iter()
                        .map(|&b| if b < 0x80 { u16::from(b) } else { REPLACEMENT }),
                );
                bytes.len()
            }
            Charset::Cp1251 => {
                let high = tables::HIGH[self.table().expect("a code page")];
                out.extend(bytes.iter().map(|&b| {
                    if b < 0x80 {
                        u16::from(b)
                    } else {
                        high[usize::from(b - 0x80)]
                    }
                }));
                bytes.len()
            }
        }
    }

    /// Encodes `chars`. What the charset cannot represent becomes `?`, a
    /// surrogate pair one `?` and an unpaired surrogate one `?`, as Java's
    /// `String.getBytes` writes them.
    pub(crate) fn encode(self, chars: &[u16]) -> Vec<u8> {
        if self == Charset::Utf8 {
            return encode_utf8(chars);
        }
        let mut out = Vec::with_capacity(chars.len());
        for c in char::decode_utf16(chars.iter().copied()) {
            let unit = c.ok().and_then(|c| u16::try_from(u32::from(c)).ok());
            out.push(unit.and_then(|u| self.encode_unit(u)).unwrap_or(b'?'));
        }
        out
    }

    /// The byte a single-byte charset encodes `unit` as.
    fn encode_unit(self, unit: u16) -> Option<u8> {
        match self {
            Charset::Latin1 => u8::try_from(unit).ok(),
            Charset::Ascii => u8::try_from(unit).ok().filter(u8::is_ascii),
            Charset::Utf8 => None,
            Charset::Cp1251 if unit < 0x80 => Some(unit as u8),
            Charset::Cp1251 => {
                let reverse = reverse_table(self.table()?);
                reverse
                    .binary_search_by_key(&unit, |&(c, _)| c)
                    .ok()
                    .map(|i| reverse[i].1)
            }
        }
    }
}

/// The upper half of code page `table` as (character, byte) pairs sorted
/// by character, for encoding.
fn reverse_table(table: usize) -> &'static [(u16, u8)] {
    static REVERSE: [OnceLock<Vec<(u16, u8)>>; tables::HIGH.len()] =
        [const { OnceLock::new() }; tables::HIGH.len()];
    REVERSE[table].get_or_init(|| {
        let mut pairs: Vec<(u16, u8)> = tables::HIGH[table]
            .iter()
            .enumerate()
            .filter(|&(_, &c)| c != REPLACEMENT)
            .map(|(i, &c)| (c, 0x80 + i as u8))
            .collect();
        pairs.sort_unstable();
        pairs
    })
}

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
/// and so is a sequence cut off at the end of the input.
fn decode_utf8(bytes: &[u8], end: bool, out: &mut Vec<u16>) -> usize {
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
fn encode_utf8(chars: &[u16]) -> Vec<u8> {
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

    #[test]
    fn names_resolve_with_aliases() {
        assert_eq!(Charset::lookup("UTF-8"), Some(Charset::Utf8));
        assert_eq!(Charset::lookup("ISO8859_1"), Some(Charset::Latin1));
        assert_eq!(Charset::lookup("Cp1251"), Some(Charset::Cp1251));
        assert_eq!(Charset::lookup("US-ASCII"), Some(Charset::Ascii));
        assert_eq!(Charset::lookup("KOI8-R"), None);
    }

    #[test]
    fn utf8_round_trips() {
        let text = utf16("h\u{e9}llo \u{1F600} \u{20ac}");
        assert_eq!(Charset::Utf8.decode(&Charset::Utf8.encode(&text)), text);
        assert_eq!(Charset::Utf8.encode(&[0xD800]), b"?");
        assert_eq!(Charset::Utf8.encode(&[0xDC00, 0x41]), b"?A");
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
            // A cut-off sequence before other bytes.
            (&[0xE2, 0x82, 0x41], "\u{FFFD}A"),
            (&[0xF0, 0x9F, 0x41], "\u{FFFD}A"),
            (&[0xF0, 0x9F, 0x41, 0x42, 0x43], "\u{FFFD}ABC"),
            // Overlong forms and values past U+10FFFF: byte by byte.
            (&[0xC0, 0x80], "\u{FFFD}\u{FFFD}"),
            (&[0xE0, 0x80, 0x80], "\u{FFFD}\u{FFFD}\u{FFFD}"),
            (
                &[0xF4, 0x90, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (
                &[0xF8, 0x88, 0x80, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (&[0xE0, 0x80], "\u{FFFD}\u{FFFD}"),
            (&[0x80, 0xBF], "\u{FFFD}\u{FFFD}"),
            (&[0xF0, 0x9F, 0x98, 0x80], "\u{1F600}"),
            (&[0xF0, 0x9F, 0x98, 0x41], "\u{FFFD}A"),
            (&[0xF0, 0x9F, 0x98, 0x80, 0x80], "\u{1F600}\u{FFFD}"),
            (&[0xE0, 0xA0], "\u{FFFD}"),
            (&[0xF5, 0x80], "\u{FFFD}\u{FFFD}"),
            (
                &[0xF0, 0x80, 0x80, 0x80],
                "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}",
            ),
            (&[0xC3, 0x41], "\u{FFFD}A"),
            (&[0xEF, 0xBF, 0xBF], "\u{FFFF}"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(
                String::from_utf16(&Charset::Utf8.decode(bytes)).unwrap(),
                *expected,
                "{bytes:02X?}"
            );
        }
    }

    #[test]
    fn utf8_waits_for_the_rest_of_a_sequence_unless_at_the_end() {
        let mut out = Vec::new();
        assert_eq!(
            Charset::Utf8.decode_into(&[0x41, 0xE2, 0x82], false, &mut out),
            1
        );
        assert_eq!(out, utf16("A"));
        out.clear();
        assert_eq!(
            Charset::Utf8.decode_into(&[0xE2, 0x82, 0xAC], false, &mut out),
            3
        );
        assert_eq!(out, utf16("\u{20ac}"));
        // A second byte that cannot continue the sequence decides now.
        out.clear();
        assert_eq!(Charset::Utf8.decode_into(&[0xE2, 0x41], false, &mut out), 2);
        assert_eq!(out, utf16("\u{FFFD}A"));
        out.clear();
        assert_eq!(Charset::Utf8.decode_into(&[0xF0, 0x9F], false, &mut out), 0);
    }

    #[test]
    fn cp1251_maps_cyrillic() {
        let text = utf16("\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{401}\u{451}");
        let bytes = Charset::Cp1251.encode(&text);
        assert_eq!(
            bytes,
            [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2, 0x20, 0xA8, 0xB8]
        );
        assert_eq!(Charset::Cp1251.decode(&bytes), text);
        for b in 0x80..=0xFFu8 {
            let c = Charset::Cp1251.decode(&[b]);
            if c[0] != REPLACEMENT {
                assert_eq!(Charset::Cp1251.encode(&c), [b], "byte {b:#x}");
            }
        }
    }

    #[test]
    fn single_byte_charsets_substitute() {
        assert_eq!(
            Charset::Latin1.encode(&utf16("\u{e9}\u{20ac}")),
            [0xE9, b'?']
        );
        assert_eq!(Charset::Ascii.encode(&utf16("a\u{1F600}b")), b"a?b");
        assert_eq!(Charset::Cp1251.encode(&[0xD800, 0x41]), b"?A");
        assert_eq!(Charset::Ascii.decode(&[0x41, 0xE9]), utf16("A\u{FFFD}"));
    }
}
