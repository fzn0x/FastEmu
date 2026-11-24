//! The character encodings `String` and the streams support: UTF-8,
//! ISO-8859-1, US-ASCII and windows-1251.

/// A supported encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Charset {
    Utf8,
    Latin1,
    Ascii,
    Cp1251,
}

/// windows-1251 code points for bytes 0x80 to 0xBF; 0xC0 to 0xFF map to
/// U+0410 to U+044F. 0x98 is unassigned and decodes to U+FFFD.
const CP1251_HIGH: [u16; 64] = [
    0x0402, 0x0403, 0x201A, 0x0453, 0x201E, 0x2026, 0x2020, 0x2021, 0x20AC, 0x2030, 0x0409, 0x2039,
    0x040A, 0x040C, 0x040B, 0x040F, 0x0452, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0xFFFD, 0x2122, 0x0459, 0x203A, 0x045A, 0x045C, 0x045B, 0x045F, 0x00A0, 0x040E, 0x045E, 0x0408,
    0x00A4, 0x0490, 0x00A6, 0x00A7, 0x0401, 0x00A9, 0x0404, 0x00AB, 0x00AC, 0x00AD, 0x00AE, 0x0407,
    0x00B0, 0x00B1, 0x0406, 0x0456, 0x0491, 0x00B5, 0x00B6, 0x00B7, 0x0451, 0x2116, 0x0454, 0x00BB,
    0x0458, 0x0405, 0x0455, 0x0457,
];

impl Charset {
    /// The charset `name` names, ignoring case and the common aliases.
    pub(crate) fn lookup(name: &str) -> Option<Charset> {
        let key: String = name
            .chars()
            .filter(|c| !matches!(c, '-' | '_'))
            .flat_map(char::to_lowercase)
            .collect();
        Some(match key.as_str() {
            "utf8" => Charset::Utf8,
            "iso88591" | "latin1" | "iso8859_1" | "l1" | "cp819" | "ibm819" | "iso885911987" => {
                Charset::Latin1
            }
            "usascii" | "ascii" | "ascii7" | "646" | "iso646us" => Charset::Ascii,
            "windows1251" | "cp1251" | "win1251" => Charset::Cp1251,
            _ => return None,
        })
    }

    pub(crate) fn decode(self, bytes: &[u8]) -> Vec<u16> {
        match self {
            Charset::Utf8 => decode_utf8(bytes),
            Charset::Latin1 => bytes.iter().map(|&b| u16::from(b)).collect(),
            Charset::Ascii => bytes
                .iter()
                .map(|&b| if b < 0x80 { u16::from(b) } else { 0xFFFD })
                .collect(),
            Charset::Cp1251 => bytes
                .iter()
                .map(|&b| match b {
                    0..=0x7F => u16::from(b),
                    0x80..=0xBF => CP1251_HIGH[usize::from(b - 0x80)],
                    _ => 0x0410 + u16::from(b - 0xC0),
                })
                .collect(),
        }
    }

    pub(crate) fn encode(self, chars: &[u16]) -> Vec<u8> {
        if self == Charset::Utf8 {
            return encode_utf8(chars);
        }
        // A surrogate pair is one character, so it becomes one `?`.
        char::decode_utf16(chars.iter().copied())
            .map(|c| {
                let Some(unit) = c.ok().and_then(|c| u16::try_from(u32::from(c)).ok()) else {
                    return b'?';
                };
                match self {
                    Charset::Latin1 => u8::try_from(unit).unwrap_or(b'?'),
                    Charset::Ascii if unit < 0x80 => unit as u8,
                    Charset::Cp1251 => encode_cp1251(unit),
                    _ => b'?',
                }
            })
            .collect()
    }
}

fn encode_cp1251(c: u16) -> u8 {
    match c {
        0..=0x7F => c as u8,
        0x0410..=0x044F => (c - 0x0410) as u8 + 0xC0,
        _ => CP1251_HIGH
            .iter()
            .position(|&h| h == c && h != 0xFFFD)
            .map_or(b'?', |i| 0x80 + i as u8),
    }
}

/// UTF-8 as Java decodes it: each malformed sequence becomes one U+FFFD.
fn decode_utf8(bytes: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        out.extend(chunk.valid().encode_utf16());
        if !chunk.invalid().is_empty() {
            out.push(0xFFFD);
        }
    }
    out
}

/// UTF-8 with unpaired surrogates encoded as `?`, as Java does.
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
    fn utf8_round_trips_and_replaces_garbage() {
        let text = utf16("h\u{e9}llo \u{1F600}");
        assert_eq!(Charset::Utf8.decode(&Charset::Utf8.encode(&text)), text);
        assert_eq!(
            Charset::Utf8.decode(&[0x41, 0xFF, 0x42]),
            utf16("A\u{FFFD}B")
        );
        assert_eq!(Charset::Utf8.encode(&[0xD800]), b"?");
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
            if c[0] != 0xFFFD {
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
