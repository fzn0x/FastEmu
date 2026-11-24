//! The character encodings of `String`, the readers and writers and
//! `PrintStream`: UTF-8, ISO-8859-1, US-ASCII and the code pages
//! windows-1250, windows-1251, windows-1252, ISO-8859-2 and KOI8-R, each
//! converting exactly as the JDK's charset of that name does, malformed
//! and unmappable input included.

mod tables;
mod utf8;

use std::sync::OnceLock;

/// A supported encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Charset {
    Utf8,
    Latin1,
    Ascii,
    CodePage(CodePage),
}

/// A single-byte code page whose upper half comes from a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodePage {
    /// windows-1250, Central European.
    Cp1250,
    /// windows-1251, Cyrillic.
    Cp1251,
    /// windows-1252, Western European.
    Cp1252,
    /// ISO-8859-2, Latin-2.
    Latin2,
    /// KOI8-R, Russian.
    Koi8r,
}

/// U+FFFD, what undecodable bytes become.
const REPLACEMENT: u16 = 0xFFFD;

impl CodePage {
    const ALL: [CodePage; 5] = [
        CodePage::Cp1250,
        CodePage::Cp1251,
        CodePage::Cp1252,
        CodePage::Latin2,
        CodePage::Koi8r,
    ];

    /// What bytes 0x80 to 0xFF decode to.
    fn high(self) -> &'static [u16; 128] {
        match self {
            CodePage::Cp1250 => &tables::CP1250,
            CodePage::Cp1251 => &tables::CP1251,
            CodePage::Cp1252 => &tables::CP1252,
            CodePage::Latin2 => &tables::LATIN2,
            CodePage::Koi8r => &tables::KOI8R,
        }
    }

    fn decode(self, b: u8) -> u16 {
        if b < 0x80 {
            u16::from(b)
        } else {
            self.high()[usize::from(b - 0x80)]
        }
    }

    fn encode(self, unit: u16) -> Option<u8> {
        if unit < 0x80 {
            return Some(unit as u8);
        }
        let reverse = self.reverse();
        reverse
            .binary_search_by_key(&unit, |&(c, _)| c)
            .ok()
            .map(|i| reverse[i].1)
    }

    /// The upper half as (character, byte) pairs sorted by character, for
    /// encoding; made on first use.
    fn reverse(self) -> &'static [(u16, u8)] {
        static REVERSE: [OnceLock<Vec<(u16, u8)>>; CodePage::ALL.len()] =
            [const { OnceLock::new() }; CodePage::ALL.len()];
        REVERSE[self as usize].get_or_init(|| {
            let mut pairs: Vec<(u16, u8)> = self
                .high()
                .iter()
                .enumerate()
                .filter(|&(_, &c)| c != REPLACEMENT)
                .map(|(i, &c)| (c, 0x80 + i as u8))
                .collect();
            pairs.sort_unstable();
            pairs
        })
    }
}

impl Charset {
    /// The charset `name` names, ignoring case, `-` and `_`, and
    /// accepting the JDK's common aliases.
    pub(crate) fn lookup(name: &str) -> Option<Charset> {
        let key: String = name
            .chars()
            .filter(|c| !matches!(c, '-' | '_'))
            .flat_map(char::to_lowercase)
            .collect();
        let page = match key.as_str() {
            "utf8" | "unicode11utf8" => return Some(Charset::Utf8),
            "iso88591" | "latin1" | "l1" | "cp819" | "ibm819" | "885911987" | "iso885911987"
            | "88591" | "iso819" | "csisolatin1" | "isoir100" => return Some(Charset::Latin1),
            "usascii" | "ascii" | "ascii7" | "646" | "iso646us" | "csascii" | "default" => {
                return Some(Charset::Ascii);
            }
            "windows1250" | "cp1250" | "cp5346" => CodePage::Cp1250,
            "windows1251" | "cp1251" | "win1251" | "cp5347" | "ansi1251" => CodePage::Cp1251,
            "windows1252" | "cp1252" | "cp5348" | "ibm1252" => CodePage::Cp1252,
            "iso88592" | "88592" | "latin2" | "l2" | "isoir101" | "iso885921987" | "ibm912"
            | "cp912" | "912" | "csisolatin2" => CodePage::Latin2,
            "koi8r" | "koi8" | "cskoi8r" => CodePage::Koi8r,
            _ => return None,
        };
        Some(Charset::CodePage(page))
    }

    /// The charset of conversions that name none, from the value of
    /// `microedition.encoding`; UTF-8 when it names no charset this
    /// virtual machine has.
    pub(crate) fn default_for(name: Option<&str>) -> Charset {
        name.and_then(Charset::lookup).unwrap_or(Charset::Utf8)
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
            Charset::Utf8 => return utf8::decode(bytes, end, out),
            Charset::Latin1 => out.extend(bytes.iter().map(|&b| u16::from(b))),
            Charset::Ascii => out.extend(
                bytes
                    .iter()
                    .map(|&b| if b < 0x80 { u16::from(b) } else { REPLACEMENT }),
            ),
            Charset::CodePage(page) => out.extend(bytes.iter().map(|&b| page.decode(b))),
        }
        bytes.len()
    }

    /// Encodes `chars`. What the charset cannot represent becomes `?`, a
    /// surrogate pair one `?` and an unpaired surrogate one `?`, as Java's
    /// `String.getBytes` writes them.
    pub(crate) fn encode(self, chars: &[u16]) -> Vec<u8> {
        if self == Charset::Utf8 {
            return utf8::encode(chars);
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
            Charset::CodePage(page) => page.encode(unit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodePage::{Cp1250, Cp1251, Cp1252, Koi8r, Latin2};
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn names_resolve_with_aliases() {
        let page = |p| Some(Charset::CodePage(p));
        assert_eq!(Charset::lookup("UTF-8"), Some(Charset::Utf8));
        assert_eq!(Charset::lookup("ISO8859_1"), Some(Charset::Latin1));
        assert_eq!(Charset::lookup("US-ASCII"), Some(Charset::Ascii));
        assert_eq!(Charset::lookup("Cp1251"), page(Cp1251));
        assert_eq!(Charset::lookup("KOI8-R"), page(Koi8r));
        assert_eq!(Charset::lookup("ISO-8859-2"), page(Latin2));
        assert_eq!(Charset::lookup("windows-1250"), page(Cp1250));
        assert_eq!(Charset::lookup("WINDOWS-1252"), page(Cp1252));
        assert_eq!(Charset::lookup("Shift_JIS"), None);
    }

    #[test]
    fn code_pages_round_trip_every_assigned_byte() {
        for page in CodePage::ALL {
            let cs = Charset::CodePage(page);
            for b in 0..=0xFFu8 {
                let c = cs.decode(&[b]);
                if c[0] != REPLACEMENT {
                    assert_eq!(cs.encode(&c), [b], "{page:?} byte {b:#x}");
                }
            }
        }
    }

    #[test]
    fn code_pages_map_their_scripts() {
        let russian = utf16("\u{41f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{401}\u{451}");
        let cp1251 = Charset::CodePage(Cp1251);
        let bytes = cp1251.encode(&russian);
        assert_eq!(
            bytes,
            [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2, 0x20, 0xA8, 0xB8]
        );
        assert_eq!(cp1251.decode(&bytes), russian);
        let koi = Charset::CodePage(Koi8r).encode(&russian);
        assert_eq!(koi, [0xF0, 0xD2, 0xC9, 0xD7, 0xC5, 0xD4, 0x20, 0xB3, 0xA3]);
        let polish = utf16("\u{17c}\u{f3}\u{142}w");
        for page in [Latin2, Cp1250] {
            let bytes = Charset::CodePage(page).encode(&polish);
            assert_eq!(bytes, [0xBF, 0xF3, 0xB3, b'w'], "{page:?}");
        }
        assert_eq!(
            Charset::CodePage(Cp1252).decode(&[0x80, 0x81, 0xE9]),
            utf16("\u{20ac}\u{FFFD}\u{e9}")
        );
    }

    #[test]
    fn single_byte_charsets_substitute() {
        assert_eq!(
            Charset::Latin1.encode(&utf16("\u{e9}\u{20ac}")),
            [0xE9, b'?']
        );
        assert_eq!(Charset::Ascii.encode(&utf16("a\u{1F600}b")), b"a?b");
        assert_eq!(Charset::CodePage(Cp1251).encode(&[0xD800, 0x41]), b"?A");
        assert_eq!(Charset::CodePage(Koi8r).encode(&utf16("\u{e9}")), b"?");
        assert_eq!(Charset::Ascii.decode(&[0x41, 0xE9]), utf16("A\u{FFFD}"));
    }
}
