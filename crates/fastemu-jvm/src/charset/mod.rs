//! The character encodings of `String`, the readers and writers and
//! `PrintStream`, under the names and aliases Java accepts:
//!
//! - UTF-8, ISO-8859-1 and US-ASCII;
//! - the single-byte code pages windows-1250, windows-1251, windows-1252,
//!   KOI8-R and the ISO-8859 parts Java has (2 to 9, 13, 15 and 16);
//! - UTF-16 with a byte order mark, UTF-16BE, UTF-16LE and
//!   x-UTF-16LE-BOM;
//! - the multi-byte charsets of East Asia: Shift_JIS, EUC-JP,
//!   ISO-2022-JP, GBK and GB2312, GB18030, Big5 and EUC-KR (see
//!   [`multi`]).
//!
//! The single-byte charsets and UTF-8 and UTF-16 convert exactly as the
//! JDK's charset of that name does, malformed and unmappable input
//! included; the multi-byte ones as encoding_rs does, which agrees with
//! Java on the text of their standards.

mod multi;
mod tables;
mod utf16;
mod utf8;

use std::sync::OnceLock;

use encoding_rs::Encoding;

pub(crate) use utf16::Utf16;

/// A supported encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Charset {
    Utf8,
    Latin1,
    Ascii,
    CodePage(CodePage),
    Utf16(Utf16),
    /// A multi-byte charset, converted by encoding_rs.
    Multi(&'static Encoding),
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
    /// ISO-8859-3, Latin-3.
    Latin3,
    /// ISO-8859-4, Latin-4.
    Latin4,
    /// ISO-8859-5, Cyrillic.
    Cyrillic,
    /// ISO-8859-6, Arabic.
    Arabic,
    /// ISO-8859-7, Greek.
    Greek,
    /// ISO-8859-8, Hebrew.
    Hebrew,
    /// ISO-8859-9, Latin-5.
    Latin5,
    /// ISO-8859-13, Latin-7.
    Baltic,
    /// ISO-8859-15, Latin-9.
    Latin9,
    /// ISO-8859-16, Latin-10.
    Latin10,
}

/// U+FFFD, what undecodable bytes become.
const REPLACEMENT: u16 = 0xFFFD;

impl CodePage {
    const ALL: [CodePage; 15] = [
        CodePage::Cp1250,
        CodePage::Cp1251,
        CodePage::Cp1252,
        CodePage::Latin2,
        CodePage::Koi8r,
        CodePage::Latin3,
        CodePage::Latin4,
        CodePage::Cyrillic,
        CodePage::Arabic,
        CodePage::Greek,
        CodePage::Hebrew,
        CodePage::Latin5,
        CodePage::Baltic,
        CodePage::Latin9,
        CodePage::Latin10,
    ];

    /// What bytes 0x80 to 0xFF decode to.
    fn high(self) -> &'static [u16; 128] {
        let encoding = match self {
            CodePage::Cp1250 => return &tables::CP1250,
            CodePage::Cp1251 => return &tables::CP1251,
            CodePage::Cp1252 => return &tables::CP1252,
            CodePage::Latin2 => return &tables::LATIN2,
            CodePage::Koi8r => return &tables::KOI8R,
            CodePage::Latin5 => return &tables::LATIN5,
            CodePage::Latin3 => encoding_rs::ISO_8859_3,
            CodePage::Latin4 => encoding_rs::ISO_8859_4,
            CodePage::Cyrillic => encoding_rs::ISO_8859_5,
            CodePage::Arabic => encoding_rs::ISO_8859_6,
            CodePage::Greek => encoding_rs::ISO_8859_7,
            CodePage::Hebrew => encoding_rs::ISO_8859_8,
            CodePage::Baltic => encoding_rs::ISO_8859_13,
            CodePage::Latin9 => encoding_rs::ISO_8859_15,
            CodePage::Latin10 => encoding_rs::ISO_8859_16,
        };
        // The ISO-8859 parts of the WHATWG standard map 0x80 to 0x9F to
        // the C1 controls and unassigned bytes to errors, as Java's do.
        static HIGH: [OnceLock<[u16; 128]>; CodePage::ALL.len()] =
            [const { OnceLock::new() }; CodePage::ALL.len()];
        HIGH[self as usize].get_or_init(|| {
            let mut high = [REPLACEMENT; 128];
            let mut decoder = multi::decoder(encoding);
            for (b, c) in (0x80..=0xFF).zip(high.iter_mut()) {
                let mut out = Vec::new();
                multi::decode(&mut decoder, &[b], false, &mut out);
                if let [one] = out[..] {
                    *c = one;
                }
            }
            high
        })
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

/// Where `key`, a name in lowercase without `-`, `_` and `:`, points.
fn by_key(key: &str) -> Option<Charset> {
    use CodePage::{Cp1250, Cp1251, Cp1252, Koi8r};
    let page = match key {
        "utf8" | "unicode11utf8" => return Some(Charset::Utf8),
        "iso88591" | "latin1" | "l1" | "cp819" | "ibm819" | "819" | "885911987"
        | "iso885911987" | "88591" | "iso819" | "csisolatin1" | "isoir100" => {
            return Some(Charset::Latin1);
        }
        "usascii" | "ascii" | "ascii7" | "646" | "iso646us" | "csascii" | "default" | "us"
        | "ansix3.41968" | "ansix3.41986" | "ibm367" | "cp367" | "isoir6" => {
            return Some(Charset::Ascii);
        }
        "utf16" | "unicodebig" | "unicode" => return Some(Charset::Utf16(Utf16::Marked)),
        "utf16be" | "unicodebigunmarked" | "xutf16be" | "iso10646ucs2" => {
            return Some(Charset::Utf16(Utf16::Big));
        }
        "utf16le" | "unicodelittleunmarked" | "xutf16le" => {
            return Some(Charset::Utf16(Utf16::Little));
        }
        "xutf16lebom" | "unicodelittle" => return Some(Charset::Utf16(Utf16::MarkedLittle)),
        "windows1250" | "cp1250" | "cp5346" => Cp1250,
        "windows1251" | "cp1251" | "win1251" | "cp5347" | "ansi1251" => Cp1251,
        "windows1252" | "cp1252" | "cp5348" | "ibm1252" => Cp1252,
        "koi8r" | "koi8" | "cskoi8r" => Koi8r,
        _ => match by_key_multi(key) {
            Some(encoding) => return Some(Charset::Multi(encoding)),
            None => by_key_iso8859(key)?,
        },
    };
    Some(Charset::CodePage(page))
}

/// The multi-byte charset `key` names. Java's strict charsets and their
/// vendor supersets (`Shift_JIS` and `windows-31j`, `GB2312` and `GBK`,
/// `Big5` and `Big5-HKSCS`, `EUC-KR` and `x-windows-949`) share the
/// superset's table.
fn by_key_multi(key: &str) -> Option<&'static Encoding> {
    Some(match key {
        "shiftjis" | "sjis" | "xsjis" | "csshiftjis" | "mskanji" | "windows31j" | "ms932"
        | "windows932" | "cswindows31j" => encoding_rs::SHIFT_JIS,
        "eucjp"
        | "xeucjp"
        | "eucjis"
        | "cseucpkdfmtjapanese"
        | "extendedunixcodepackedformatforjapanese" => encoding_rs::EUC_JP,
        "iso2022jp" | "csiso2022jp" | "jis" | "jisencoding" | "csjisencoding" => {
            encoding_rs::ISO_2022_JP
        }
        "gbk" | "cp936" | "windows936" | "ms936" | "xmswin936" | "gb2312" | "euccn" | "xeuccn"
        | "gb23121980" | "gb231280" => encoding_rs::GBK,
        "gb18030" | "gb180302022" => encoding_rs::GB18030,
        "big5" | "csbig5" | "ms950" | "xwindows950" | "big5hkscs" | "big5hk" => encoding_rs::BIG5,
        "euckr" | "cseuckr" | "5601" | "ksc5601" | "ksc56011987" | "xwindows949" | "windows949"
        | "ms949" => encoding_rs::EUC_KR,
        _ => return None,
    })
}

/// The ISO-8859 part `key` names, of those Java has.
fn by_key_iso8859(key: &str) -> Option<CodePage> {
    use CodePage::{
        Arabic, Baltic, Cyrillic, Greek, Hebrew, Latin2, Latin3, Latin4, Latin5, Latin9, Latin10,
    };
    Some(match key {
        "iso88592" | "88592" | "912" | "latin2" | "l2" | "isoir101" | "iso885921987" | "ibm912"
        | "cp912" | "csisolatin2" => Latin2,
        "iso88593" | "88593" | "913" | "latin3" | "l3" | "isoir109" | "iso885931988" | "ibm913"
        | "cp913" | "csisolatin3" => Latin3,
        "iso88594" | "88594" | "914" | "latin4" | "l4" | "isoir110" | "iso885941988" | "ibm914"
        | "cp914" | "csisolatin4" => Latin4,
        "iso88595" | "88595" | "915" | "cyrillic" | "isoir144" | "iso885951988" | "ibm915"
        | "cp915" | "csisolatincyrillic" => Cyrillic,
        "iso88596" | "88596" | "1089" | "arabic" | "isoir127" | "iso885961987" | "ibm1089"
        | "cp1089" | "csisolatinarabic" | "asmo708" | "ecma114" => Arabic,
        "iso88597" | "88597" | "813" | "greek" | "greek8" | "isoir126" | "iso885971987"
        | "ibm813" | "cp813" | "csisolatingreek" | "ecma118" | "elot928" | "suneugreek" => Greek,
        "iso88598" | "88598" | "916" | "hebrew" | "isoir138" | "iso885981988" | "ibm916"
        | "cp916" | "csisolatinhebrew" => Hebrew,
        "iso88599" | "88599" | "920" | "latin5" | "l5" | "isoir148" | "iso885991989" | "ibm920"
        | "cp920" | "csisolatin5" => Latin5,
        "iso885913" | "885913" => Baltic,
        "iso885915" | "885915" | "923" | "ibm923" | "cp923" | "latin9" | "l9" | "latin0"
        | "iso885915fdis" | "csiso885915" | "csisolatin0" | "csisolatin9" => Latin9,
        "iso885916" | "iso8859162001" | "isoir226" | "latin10" | "l10" | "csiso885916" => Latin10,
        _ => return None,
    })
}

impl Charset {
    /// The charset `name` names, ignoring case, `-`, `_` and `:`, and
    /// accepting the JDK's aliases and its historical names (`SJIS`,
    /// `EUC_JP`, `UnicodeBig`, `ISO8859_2`, `Cp1251`, ...).
    pub(crate) fn lookup(name: &str) -> Option<Charset> {
        let key: String = name
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ':'))
            .flat_map(char::to_lowercase)
            .collect();
        by_key(&key)
    }

    /// The charset of conversions that name none, from the value of
    /// `microedition.encoding`. When that names no charset this virtual
    /// machine has, ISO-8859-1, which maps every byte to a character and
    /// back, so text a game reads and writes again survives unchanged.
    pub(crate) fn default_for(name: Option<&str>) -> Charset {
        name.and_then(Charset::lookup).unwrap_or(Charset::Latin1)
    }

    /// Decodes all of `bytes`, as `new String(bytes, charset)` does.
    pub(crate) fn decode(self, bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::with_capacity(bytes.len());
        match self {
            Charset::Utf8 => {
                utf8::decode(bytes, true, &mut out);
            }
            Charset::Utf16(kind) => {
                kind.decode(&mut None, bytes, true, &mut out);
            }
            Charset::Multi(encoding) => {
                multi::decode(&mut multi::decoder(encoding), bytes, true, &mut out);
            }
            Charset::Latin1 | Charset::Ascii | Charset::CodePage(_) => {
                out.extend(bytes.iter().map(|&b| self.decode_byte(b)));
            }
        }
        out
    }

    /// Encodes all of `chars`, as `String.getBytes(charset)` does. What
    /// the charset cannot represent becomes `?` (U+FFFD in UTF-16), a
    /// surrogate pair one `?` and an unpaired surrogate one `?`, as Java
    /// writes them.
    pub(crate) fn encode(self, chars: &[u16]) -> Vec<u8> {
        let mut out = Vec::with_capacity(chars.len());
        self.encoder().encode_last(chars, true, &mut out);
        out
    }

    /// A decoder for a stream of bytes in this charset.
    pub(crate) fn decoder(self) -> Decoder {
        let state = match self {
            Charset::Multi(encoding) => DecodeState::Multi {
                decoder: multi::decoder(encoding),
                held: false,
            },
            _ => DecodeState::Bytes {
                pending: Vec::new(),
                order: None,
            },
        };
        Decoder {
            charset: self,
            state,
        }
    }

    /// An encoder for a stream of text in this charset.
    pub(crate) fn encoder(self) -> Encoder {
        Encoder {
            charset: self,
            started: false,
            multi: match self {
                Charset::Multi(encoding) => Some(encoding.new_encoder()),
                _ => None,
            },
        }
    }

    /// The character a single-byte charset decodes `b` to.
    fn decode_byte(self, b: u8) -> u16 {
        match self {
            Charset::Ascii if b >= 0x80 => REPLACEMENT,
            Charset::CodePage(page) => page.decode(b),
            _ => u16::from(b),
        }
    }

    /// The byte a single-byte charset encodes `unit` as.
    fn encode_unit(self, unit: u16) -> Option<u8> {
        match self {
            Charset::Latin1 => u8::try_from(unit).ok(),
            Charset::Ascii => u8::try_from(unit).ok().filter(u8::is_ascii),
            Charset::CodePage(page) => page.encode(unit),
            Charset::Utf8 | Charset::Utf16(_) | Charset::Multi(_) => None,
        }
    }
}

/// Decodes a stream of bytes, as a reader does: what a read cut off in
/// the middle of a character waits for the rest, and UTF-16 keeps the
/// byte order its mark set.
pub(crate) struct Decoder {
    charset: Charset,
    state: DecodeState,
}

enum DecodeState {
    /// The charsets FastEmu converts itself.
    Bytes {
        /// The start of a character, waiting for its other bytes.
        pending: Vec<u8>,
        /// UTF-16's byte order, once set.
        order: Option<utf16::Order>,
    },
    Multi {
        decoder: encoding_rs::Decoder,
        /// Whether bytes went in since the last character came out.
        held: bool,
    },
}

impl Decoder {
    /// Decodes `bytes` onto `out`. Unless `end` is set, a character cut
    /// off at the end waits for the next call; with `end`, it becomes
    /// U+FFFD, as Java decodes the end of its input, and the decoder
    /// starts afresh.
    pub(crate) fn decode(&mut self, bytes: &[u8], end: bool, out: &mut Vec<u16>) {
        let start = out.len();
        match &mut self.state {
            DecodeState::Bytes { pending, order } => {
                pending.extend_from_slice(bytes);
                let used = match self.charset {
                    Charset::Utf8 => utf8::decode(pending, end, out),
                    Charset::Utf16(kind) => kind.decode(order, pending, end, out),
                    charset => {
                        out.extend(pending.iter().map(|&b| charset.decode_byte(b)));
                        pending.len()
                    }
                };
                pending.drain(..used);
            }
            DecodeState::Multi { decoder, held } => {
                multi::decode(decoder, bytes, end, out);
                *held = out.len() == start && (*held || !bytes.is_empty());
            }
        }
        if end {
            self.reset();
        }
    }

    /// Whether bytes wait for the rest of their character.
    pub(crate) fn has_pending(&self) -> bool {
        match &self.state {
            DecodeState::Bytes { pending, .. } => !pending.is_empty(),
            DecodeState::Multi { held, .. } => *held,
        }
    }

    /// Forgets what waits and starts afresh, as after `reset` of the
    /// stream under a reader.
    pub(crate) fn reset(&mut self) {
        *self = self.charset.decoder();
    }
}

/// Encodes a stream of text, as a writer does: UTF-16 writes its byte
/// order mark once, at the start, and ISO-2022-JP keeps its shift state
/// from one write to the next.
pub(crate) struct Encoder {
    charset: Charset,
    /// Whether anything was encoded yet.
    started: bool,
    multi: Option<encoding_rs::Encoder>,
}

impl Encoder {
    /// Encodes `chars` onto `out`; see [`Charset::encode`].
    pub(crate) fn encode(&mut self, chars: &[u16], out: &mut Vec<u8>) {
        self.encode_last(chars, false, out);
    }

    /// Ends the stream: a stateful charset returns to its initial state,
    /// as Java's encoder does when its writer closes.
    pub(crate) fn finish(&mut self, out: &mut Vec<u8>) {
        self.encode_last(&[], true, out);
    }

    fn encode_last(&mut self, chars: &[u16], last: bool, out: &mut Vec<u8>) {
        let first = !self.started && !chars.is_empty();
        self.started |= first;
        match (self.charset, &mut self.multi) {
            (Charset::Utf8, _) => out.extend(utf8::encode(chars)),
            (Charset::Utf16(kind), _) => kind.encode(chars, first && kind.writes_mark(), out),
            (_, Some(encoder)) => multi::encode(encoder, chars, last, out),
            (charset, None) => {
                for c in char::decode_utf16(chars.iter().copied()) {
                    let unit = c.ok().and_then(|c| u16::try_from(u32::from(c)).ok());
                    out.push(unit.and_then(|u| charset.encode_unit(u)).unwrap_or(b'?'));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CodePage::{
        Arabic, Baltic, Cp1250, Cp1251, Cp1252, Cyrillic, Greek, Hebrew, Koi8r, Latin2, Latin3,
        Latin4, Latin5, Latin9, Latin10,
    };
    use super::*;

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn names_resolve_with_aliases() {
        let page = |p| Some(Charset::CodePage(p));
        let multi = |e| Some(Charset::Multi(e));
        assert_eq!(Charset::lookup("UTF-8"), Some(Charset::Utf8));
        assert_eq!(Charset::lookup("ISO8859_1"), Some(Charset::Latin1));
        assert_eq!(Charset::lookup("ISO_8859-1:1987"), Some(Charset::Latin1));
        assert_eq!(Charset::lookup("US-ASCII"), Some(Charset::Ascii));
        assert_eq!(Charset::lookup("Cp1251"), page(Cp1251));
        assert_eq!(Charset::lookup("KOI8-R"), page(Koi8r));
        assert_eq!(Charset::lookup("ISO-8859-2"), page(Latin2));
        assert_eq!(Charset::lookup("windows-1250"), page(Cp1250));
        assert_eq!(Charset::lookup("WINDOWS-1252"), page(Cp1252));
        assert_eq!(Charset::lookup("Shift_JIS"), multi(encoding_rs::SHIFT_JIS));
        assert_eq!(Charset::lookup("SJIS"), multi(encoding_rs::SHIFT_JIS));
        assert_eq!(Charset::lookup("MS932"), multi(encoding_rs::SHIFT_JIS));
        assert_eq!(Charset::lookup("EUC_JP"), multi(encoding_rs::EUC_JP));
        assert_eq!(Charset::lookup("JIS"), multi(encoding_rs::ISO_2022_JP));
        assert_eq!(Charset::lookup("GB2312"), multi(encoding_rs::GBK));
        assert_eq!(Charset::lookup("EUC_CN"), multi(encoding_rs::GBK));
        assert_eq!(Charset::lookup("GB18030"), multi(encoding_rs::GB18030));
        assert_eq!(Charset::lookup("Big5"), multi(encoding_rs::BIG5));
        assert_eq!(Charset::lookup("KSC5601"), multi(encoding_rs::EUC_KR));
        assert_eq!(
            Charset::lookup("ks_c_5601-1987"),
            multi(encoding_rs::EUC_KR)
        );
        let kind = |k| Some(Charset::Utf16(k));
        assert_eq!(Charset::lookup("UTF-16"), kind(Utf16::Marked));
        assert_eq!(Charset::lookup("UnicodeBigUnmarked"), kind(Utf16::Big));
        assert_eq!(Charset::lookup("UTF-16LE"), kind(Utf16::Little));
        assert_eq!(Charset::lookup("UnicodeLittle"), kind(Utf16::MarkedLittle));
        assert_eq!(Charset::lookup("ISO8859_15"), page(Latin9));
        assert_eq!(Charset::lookup("ISO_8859-9:1989"), page(Latin5));
        assert_eq!(Charset::lookup("greek"), page(Greek));
        for name in ["ISO-8859-10", "ISO-8859-14", "x-unknown", "gb_2312-80x"] {
            assert_eq!(Charset::lookup(name), None, "Java has no {name}");
        }
    }

    #[test]
    fn an_unknown_default_encoding_falls_back_to_latin_1() {
        assert_eq!(Charset::default_for(Some("x-unknown")), Charset::Latin1);
        assert_eq!(Charset::default_for(None), Charset::Latin1);
        assert_eq!(Charset::default_for(Some("utf-8")), Charset::Utf8);
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

    /// One letter of each ISO-8859 part, and a byte each leaves
    /// unassigned where it has one, as Java decodes them.
    #[test]
    fn iso_8859_parts_decode_as_in_java() {
        let cases = [
            (Latin3, 0xA1, 0x0126),
            (Latin4, 0xA2, 0x0138),
            (Cyrillic, 0xB0, 0x0410),
            (Arabic, 0xC7, 0x0627),
            (Greek, 0xC1, 0x0391),
            (Greek, 0xA4, 0x20AC),
            (Hebrew, 0xE0, 0x05D0),
            (Latin5, 0xF0, 0x011F),
            (Latin5, 0x80, 0x0080),
            (Baltic, 0xFF, 0x2019),
            (Latin9, 0xA4, 0x20AC),
            (Latin10, 0xA1, 0x0104),
            (Latin3, 0xA5, REPLACEMENT),
            (Arabic, 0xA1, REPLACEMENT),
            (Hebrew, 0xA1, REPLACEMENT),
        ];
        for (page, b, c) in cases {
            assert_eq!(Charset::CodePage(page).decode(&[b]), [c], "{page:?} {b:#x}");
            assert_eq!(Charset::CodePage(page).decode(&[0x9A]), [0x9A], "C1");
        }
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

    /// A reader's decoder takes bytes one at a time.
    #[test]
    fn decoders_wait_for_the_rest_of_a_character() {
        let cases: [(Charset, &[u8], &str); 4] = [
            (Charset::Utf8, &[0xE3, 0x81, 0x82, 0x41], "\u{3042}A"),
            (
                Charset::Utf16(Utf16::Marked),
                &[0xFF, 0xFE, 0x42, 0x30, 0x41, 0x00],
                "\u{3042}A",
            ),
            (
                Charset::Multi(encoding_rs::EUC_JP),
                &[0xA4, 0xA2, 0x41],
                "\u{3042}A",
            ),
            (
                Charset::Multi(encoding_rs::ISO_2022_JP),
                &[0x1B, 0x24, 0x42, 0x24, 0x22, 0x1B, 0x28, 0x42, 0x41],
                "\u{3042}A",
            ),
        ];
        for (charset, bytes, text) in cases {
            let mut decoder = charset.decoder();
            let mut out = Vec::new();
            for (i, &b) in bytes.iter().enumerate() {
                let before = out.len();
                decoder.decode(&[b], false, &mut out);
                if i == 0 {
                    assert_eq!(out.len(), before, "{charset:?} waits");
                    assert!(decoder.has_pending(), "{charset:?}");
                }
            }
            assert!(!decoder.has_pending(), "{charset:?}");
            decoder.decode(&[], true, &mut out);
            assert_eq!(out, utf16(text), "{charset:?}");
        }
        let mut cut = Charset::Utf16(Utf16::Big).decoder();
        let mut out = Vec::new();
        cut.decode(&[0x30], false, &mut out);
        cut.decode(&[], true, &mut out);
        assert_eq!(out, [REPLACEMENT], "cut off at the end");
        assert!(!cut.has_pending());
    }

    /// A writer's encoder writes UTF-16's mark once and keeps
    /// ISO-2022-JP's shift state until it finishes.
    #[test]
    fn encoders_keep_their_state_between_writes() {
        let mut out = Vec::new();
        let mut utf16_writer = Charset::Utf16(Utf16::Marked).encoder();
        utf16_writer.encode(&[], &mut out);
        utf16_writer.encode(&utf16("A"), &mut out);
        utf16_writer.encode(&utf16("B"), &mut out);
        utf16_writer.finish(&mut out);
        assert_eq!(out, [0xFE, 0xFF, 0x00, 0x41, 0x00, 0x42]);
        let mut out = Vec::new();
        let mut jis = Charset::Multi(encoding_rs::ISO_2022_JP).encoder();
        jis.encode(&utf16("\u{3042}"), &mut out);
        jis.encode(&utf16("\u{3044}"), &mut out);
        assert_eq!(out, [0x1B, 0x24, 0x42, 0x24, 0x22, 0x24, 0x24]);
        jis.finish(&mut out);
        assert_eq!(out[7..], [0x1B, 0x28, 0x42]);
    }
}
