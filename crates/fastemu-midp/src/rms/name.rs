//! Record store names, and the file and folder names they are saved under.
//!
//! MIDP allows any store name of 1 to 32 Unicode characters, counted as
//! `String.length()` counts them (UTF-16 code units), and names are
//! case-sensitive. File systems disagree about what a file name may hold:
//! Windows forbids `< > : " / \ | ? *`, control characters, trailing dots
//! and spaces, and device names such as `con`; Windows and macOS ignore
//! case by default. So a store name never becomes a file name as it is.
//!
//! The file stem of a name keeps `a` to `z`, `0` to `9` and `-`, and
//! writes every other code unit, capital letters included, as `_` and four
//! lowercase hex digits (`Save` becomes `_0053ave`). A stem that would
//! spell a Windows device name has its first letter escaped too. The stem
//! is plain ASCII, at most 160 characters, the same on every platform, and
//! decodes back to the exact name, unpaired surrogates included.

use std::fmt;
use std::fmt::Write as _;

/// The longest store name MIDP allows, in UTF-16 code units.
pub const MAX_NAME_UNITS: usize = 32;

/// Folder names for vendors and suites longer than this many characters
/// are shortened and end with a hash of the whole name.
const MAX_FOLDER_CHARS: usize = 64;

/// How many characters of a shortened folder name keep the readable
/// prefix, at most.
const FOLDER_PREFIX_CHARS: usize = 40;

/// A record store name exactly as the game gave it, as UTF-16 code units.
/// Names order by code unit, which is how `String.compareTo` orders them.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoreName(Vec<u16>);

impl StoreName {
    /// The name made of `units`, or `None` when MIDP does not allow it
    /// (empty or longer than [`MAX_NAME_UNITS`]).
    pub fn new(units: &[u16]) -> Option<Self> {
        (1..=MAX_NAME_UNITS)
            .contains(&units.len())
            .then(|| Self(units.to_vec()))
    }

    /// The name spelled by `text`, or `None` when MIDP does not allow it.
    pub fn from_text(text: &str) -> Option<Self> {
        Self::new(&text.encode_utf16().collect::<Vec<_>>())
    }

    /// The name's UTF-16 code units.
    pub fn units(&self) -> &[u16] {
        &self.0
    }

    /// The file stem the store is saved under (without an extension).
    pub fn file_stem(&self) -> String {
        encode(&self.0)
    }

    /// The name a file stem stands for, or `None` when the stem is not one
    /// that [`StoreName::file_stem`] writes. Only the canonical stem of a
    /// name is accepted, so names and stems match one to one.
    pub fn from_file_stem(stem: &str) -> Option<Self> {
        let name = Self::new(&decode(stem)?)?;
        (name.file_stem() == stem).then_some(name)
    }
}

impl fmt::Display for StoreName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&String::from_utf16_lossy(&self.0))
    }
}

/// The folder name for a vendor or suite name, as the folders of other
/// suites' stores use. It is encoded as a file stem is; an empty name
/// becomes `_` and a long one is cut and ends with `.` and a 64-bit hash of
/// the whole name, so it stays short enough for every file system. The
/// encoding never writes a `.` otherwise, so the two forms cannot meet.
pub fn folder_name(text: &str) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.is_empty() {
        return "_".to_owned();
    }
    let full = encode(&units);
    if full.len() <= MAX_FOLDER_CHARS {
        return full;
    }
    let mut prefix = String::new();
    for &unit in &units {
        let mut piece = String::new();
        push_unit(&mut piece, unit);
        if prefix.len() + piece.len() > FOLDER_PREFIX_CHARS {
            break;
        }
        prefix.push_str(&piece);
    }
    format!("{prefix}.{:016x}", fnv1a(&units))
}

fn encode(units: &[u16]) -> String {
    let mut out = String::with_capacity(units.len());
    for &unit in units {
        push_unit(&mut out, unit);
    }
    if is_device_name(&out) {
        // Device names start with a plain letter, so escaping the first
        // unit replaces exactly the first character.
        let rest = out.split_off(1);
        out.clear();
        push_escape(&mut out, units[0]);
        out.push_str(&rest);
    }
    out
}

fn decode(stem: &str) -> Option<Vec<u16>> {
    let bytes = stem.as_bytes();
    let mut units = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'_' {
            let hex = stem.get(i + 1..i + 5)?;
            if !hex.bytes().all(|h| matches!(h, b'0'..=b'9' | b'a'..=b'f')) {
                return None;
            }
            units.push(u16::from_str_radix(hex, 16).ok()?);
            i += 5;
        } else if is_plain(u16::from(b)) {
            units.push(u16::from(b));
            i += 1;
        } else {
            return None;
        }
    }
    Some(units)
}

/// Whether a code unit is written as it is.
fn is_plain(unit: u16) -> bool {
    matches!(unit, 0x61..=0x7a | 0x30..=0x39 | 0x2d)
}

fn push_unit(out: &mut String, unit: u16) {
    if is_plain(unit) {
        out.push(char::from(unit as u8));
    } else {
        push_escape(out, unit);
    }
}

fn push_escape(out: &mut String, unit: u16) {
    let _ = write!(out, "_{unit:04x}");
}

/// Whether `stem` is a name Windows keeps for a device, whatever the
/// extension after it.
fn is_device_name(stem: &str) -> bool {
    match stem.as_bytes() {
        b"con" | b"prn" | b"aux" | b"nul" => true,
        [b'c', b'o', b'm', d] | [b'l', b'p', b't', d] => d.is_ascii_digit(),
        _ => false,
    }
}

/// FNV-1a over the code units, little-endian: a stable hash that does not
/// change between Rust versions, unlike the standard library's.
fn fnv1a(units: &[u16]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in units.iter().flat_map(|u| u.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> StoreName {
        StoreName::from_text(text).expect("a valid name")
    }

    #[test]
    fn plain_names_stay_readable() {
        assert_eq!(name("save-1").file_stem(), "save-1");
        assert_eq!(name("Save").file_stem(), "_0053ave");
        assert_eq!(name("a b.c").file_stem(), "a_0020b_002ec");
        assert_eq!(name("_").file_stem(), "_005f");
    }

    #[test]
    fn names_differing_in_case_get_different_files() {
        let upper = name("SAVE").file_stem();
        let lower = name("save").file_stem();
        assert_ne!(upper.to_ascii_lowercase(), lower.to_ascii_lowercase());
    }

    #[test]
    fn every_kind_of_name_round_trips() {
        let names = [
            "save",
            "HighScores",
            "con",
            "COM1",
            "lpt9",
            "nul.txt",
            "a/b\\c:d*e?f\"g<h>i|j",
            "   ",
            "...",
            "\u{5b58}\u{6863}",
            "\u{1F3AE} game",
            "\u{0}\u{1f}",
        ];
        for text in names {
            let n = name(text);
            let stem = n.file_stem();
            assert!(
                stem.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            );
            assert_eq!(StoreName::from_file_stem(&stem), Some(n), "{text:?}");
        }
    }

    #[test]
    fn unpaired_surrogates_survive() {
        let n = StoreName::new(&[0xd800, 0x61, 0xdfff]).unwrap();
        assert_eq!(StoreName::from_file_stem(&n.file_stem()), Some(n));
    }

    #[test]
    fn device_names_are_escaped() {
        assert_eq!(name("con").file_stem(), "_0063on");
        assert_eq!(name("com7").file_stem(), "_0063om7");
        assert_eq!(name("lpt0").file_stem(), "_006cpt0");
        assert_eq!(name("console").file_stem(), "console");
        assert_eq!(name("comx").file_stem(), "comx");
    }

    #[test]
    fn stems_are_short_enough_for_every_file_system() {
        let longest = StoreName::new(&[0x4e00; MAX_NAME_UNITS]).unwrap();
        assert_eq!(longest.file_stem().len(), MAX_NAME_UNITS * 5);
    }

    #[test]
    fn only_canonical_stems_decode() {
        assert_eq!(StoreName::from_file_stem("save"), Some(name("save")));
        // "con" is never written as it is.
        assert_eq!(StoreName::from_file_stem("con"), None);
        // Upper-case hex, short escapes and foreign characters are not stems.
        assert_eq!(StoreName::from_file_stem("_0053AVE"), None);
        assert_eq!(StoreName::from_file_stem("_005"), None);
        assert_eq!(StoreName::from_file_stem("Save"), None);
        assert_eq!(StoreName::from_file_stem("a.b"), None);
        assert_eq!(StoreName::from_file_stem(""), None);
        // An escape of a plain character is not canonical.
        assert_eq!(StoreName::from_file_stem("_0061"), None);
    }

    #[test]
    fn names_outside_midp_limits_are_refused() {
        assert_eq!(StoreName::from_text(""), None);
        assert!(StoreName::from_text(&"x".repeat(32)).is_some());
        assert_eq!(StoreName::from_text(&"x".repeat(33)), None);
        // A character outside the BMP counts as two units.
        assert_eq!(StoreName::from_text(&"\u{1F3AE}".repeat(17)), None);
    }

    #[test]
    fn folder_names_are_bounded_and_distinct() {
        assert_eq!(folder_name(""), "_");
        assert_eq!(folder_name("acme"), "acme");
        assert_eq!(folder_name("Acme Games"), "_0041cme_0020_0047ames");
        let long_a = folder_name(&"Very Long Vendor Name ".repeat(4));
        let long_b = folder_name(&"Very Long Vendor Name ".repeat(5));
        assert!(long_a.len() <= FOLDER_PREFIX_CHARS + 17, "{long_a}");
        assert!(long_a.contains('.'));
        assert_ne!(long_a, long_b);
        assert_eq!(long_a, folder_name(&"Very Long Vendor Name ".repeat(4)));
    }
}
