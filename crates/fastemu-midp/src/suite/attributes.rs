//! The attribute files of a suite: the JAR manifest and the JAD, which
//! share one `Name: value` format, and the `MIDlet-<n>` entries they hold.
//!
//! The parser follows `docs/compat/notes/01-suite-loading.md`: it decodes
//! bytes leniently, strips a UTF-8 byte order mark, splits lines on CR, LF
//! or CR LF, joins a line that starts with a space to the one before it
//! with exactly one space removed, skips blank lines, splits each line at
//! its first colon, trims both sides, and keeps the first value of a key
//! that repeats. No line ever rejects the whole file.

use std::borrow::Cow;
use std::collections::HashSet;

use fastemu_core::MidletInfo;

/// The most bytes of an attribute file the parser reads. Real manifests
/// and descriptors are a few KB; the largest in the test folder is 1.7 KB.
/// The rest of a bigger file is ignored, so the time and memory of a parse
/// are bounded by the file's size, not by how much a JAR can inflate to.
pub const MAX_ATTRIBUTE_BYTES: usize = 1024 * 1024;

/// The most attributes the parser keeps; the 4097th key is ignored. A
/// manifest in the test folder holds 52 at most.
pub const MAX_ATTRIBUTES: usize = 4096;

/// The longest class name a `MIDlet-<n>` entry may give, as the loader
/// accepts at most this many bytes of a class name.
pub const MAX_CLASS_NAME_BYTES: usize = 255;

/// The attributes of one file, in file order, without repeated keys.
///
/// At most [`MAX_ATTRIBUTE_BYTES`] of `bytes` are read and at most
/// [`MAX_ATTRIBUTES`] attributes kept. A line is a slice of `bytes`, and
/// only a line that others continue is copied, once, into the text it
/// joins; a key repeated is found in a set, so the work is linear.
pub fn parse(bytes: &[u8]) -> Vec<(String, String)> {
    let bytes = cut_utf8(bytes, MAX_ATTRIBUTE_BYTES);
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut attributes: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut logical: Option<Cow<'_, [u8]>> = None;
    for line in split_lines(bytes) {
        if let Some(rest) = line.strip_prefix(b" ")
            && let Some(last) = logical.as_mut()
        {
            last.to_mut().extend_from_slice(rest);
            continue;
        }
        if let Some(done) = logical.replace(Cow::Borrowed(line)) {
            keep(&done, &mut attributes, &mut seen);
        }
        if attributes.len() >= MAX_ATTRIBUTES {
            return attributes;
        }
    }
    if let Some(done) = logical {
        keep(&done, &mut attributes, &mut seen);
    }
    attributes
}

/// Adds the attribute that the logical line `line` holds, unless it has
/// no key, repeats one, or the list is full.
fn keep(line: &[u8], attributes: &mut Vec<(String, String)>, seen: &mut HashSet<String>) {
    if attributes.len() >= MAX_ATTRIBUTES {
        return;
    }
    let text = decode(line);
    let Some((key, value)) = text.split_once(':') else {
        return;
    };
    let key = key.trim();
    if key.is_empty() || seen.contains(key) {
        return;
    }
    seen.insert(key.to_owned());
    attributes.push((key.to_owned(), value.trim().to_owned()));
}

/// The first `max` bytes of `bytes`, less the bytes of a UTF-8 character
/// the cut would split.
fn cut_utf8(bytes: &[u8], max: usize) -> &[u8] {
    if bytes.len() <= max {
        return bytes;
    }
    let mut end = max;
    // A continuation byte is `10xxxxxx`; back up to the start of its
    // character, which is at most 3 bytes back in UTF-8.
    while end > max.saturating_sub(3) && bytes[end] & 0xC0 == 0x80 {
        end -= 1;
    }
    &bytes[..end]
}

/// Splits on CR, LF and CR LF, dropping lines that are empty or blank.
fn split_lines(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .filter(|line| line.iter().any(|b| !b.is_ascii_whitespace()))
}

/// UTF-8 when the line is valid UTF-8, else Latin-1, which maps every byte
/// to one character so nothing is lost. A valid line is borrowed.
fn decode(line: &[u8]) -> Cow<'_, str> {
    match std::str::from_utf8(line) {
        Ok(s) => Cow::Borrowed(s),
        Err(_) => Cow::Owned(line.iter().map(|&b| char::from(b)).collect()),
    }
}

/// The value of `key` among `attributes`: an exact match first, then one
/// that differs only in ASCII case.
pub fn lookup<'a>(attributes: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(k, _)| k == key)
        .or_else(|| attributes.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)))
        .map(|(_, v)| v.as_str())
}

/// The suite's MIDlets from its `MIDlet-<n>` attributes.
///
/// The list runs from `MIDlet-1` to the first missing number. When
/// `MIDlet-1` itself is missing but higher numbers exist, those are used
/// in order and a note says so. Returns the MIDlets and any notes worth
/// logging.
pub fn midlets(
    attributes: &[(String, String)],
    suite_name: &str,
) -> (Vec<MidletInfo>, Vec<String>) {
    let mut notes = Vec::new();
    let mut numbered: Vec<(u32, &str)> = attributes
        .iter()
        .filter_map(|(k, v)| {
            let n = k
                .get(..7)
                .filter(|p| p.eq_ignore_ascii_case("MIDlet-"))
                .and_then(|_| k[7..].parse::<u32>().ok())?;
            Some((n, v.as_str()))
        })
        .collect();
    numbered.sort_by_key(|&(n, _)| n);
    numbered.dedup_by_key(|&mut (n, _)| n);
    let consecutive = numbered
        .iter()
        .enumerate()
        .take_while(|&(i, &(n, _))| n as usize == i + 1)
        .count();
    let chosen: Vec<&str> = if consecutive > 0 {
        if consecutive < numbered.len() {
            notes.push(format!(
                "MIDlet-{} is missing, so the MIDlets after it are ignored",
                consecutive + 1
            ));
        }
        numbered[..consecutive].iter().map(|&(_, v)| v).collect()
    } else {
        if !numbered.is_empty() {
            notes.push("the suite has no MIDlet-1; using its other MIDlet-<n> entries".to_owned());
        }
        numbered.iter().map(|&(_, v)| v).collect()
    };
    let midlets = chosen
        .into_iter()
        .filter_map(|value| parse_midlet(value, suite_name))
        .collect();
    (midlets, notes)
}

/// One `MIDlet-<n>` value: `name, icon, class`. The class is the text
/// after the last comma, the name the text before the first, the icon the
/// middle when there are three or more fields. A value with no comma is a
/// class alone. A class that [`class_name`] refuses leaves no MIDlet.
pub fn parse_midlet(value: &str, suite_name: &str) -> Option<MidletInfo> {
    // The fields are found with the first and the last comma, not split
    // into a list, so a value of a million commas costs no list.
    let (head, class) = match value.rsplit_once(',') {
        Some((head, class)) => (Some(head), class),
        None => (None, value),
    };
    let class = class_name(class.trim())?;
    let (first, middle) = match head.map(|head| head.split_once(',')) {
        Some(Some((first, middle))) => (Some(first.trim()), Some(middle)),
        Some(None) => (head.map(str::trim), None),
        None => (None, None),
    };
    let name = if let Some(first) = first.filter(|first| !first.is_empty()) {
        first.to_owned()
    } else if !suite_name.is_empty() {
        suite_name.to_owned()
    } else {
        class.rsplit('.').next().unwrap_or(&class).to_owned()
    };
    let icon_path = middle
        .map(|middle| {
            let mut icon = String::with_capacity(middle.len());
            for (i, part) in middle.split(',').enumerate() {
                if i > 0 {
                    icon.push(',');
                }
                icon.push_str(part.trim());
            }
            icon.trim().trim_start_matches('/').to_owned()
        })
        .filter(|icon| !icon.is_empty());
    Some(MidletInfo {
        name,
        icon_path,
        class_name: class,
    })
}

/// A class name as manifests write it (`com.a.Game`, `com/a/Game`,
/// `Game.class`) in binary form, `com.a.Game`.
///
/// A name of more than [`MAX_CLASS_NAME_BYTES`] bytes, or with a bracket or
/// a control character, is no class a game has and gives `None`, so the
/// MIDlet is dropped instead of reaching the class loader.
pub fn class_name(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw.strip_suffix(".class").unwrap_or(raw);
    let raw = raw.trim_start_matches('/');
    if raw.is_empty()
        || raw.len() > MAX_CLASS_NAME_BYTES
        || raw
            .chars()
            .any(|c| c.is_control() || matches!(c, '[' | ']'))
    {
        return None;
    }
    Some(raw.replace('/', "."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|&(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    }

    #[test]
    fn parses_lines_and_trims() {
        let text = b"Manifest-Version: 1.0\r\nMIDlet-Name:  Snake \r\n\r\nMIDlet-Vendor:Acme\n";
        assert_eq!(
            parse(text),
            pairs(&[
                ("Manifest-Version", "1.0"),
                ("MIDlet-Name", "Snake"),
                ("MIDlet-Vendor", "Acme")
            ])
        );
    }

    #[test]
    fn strips_a_bom_and_handles_lone_carriage_returns() {
        let text = b"\xEF\xBB\xBFMIDlet-Name: A\rMIDlet-Version: 1.0\r";
        assert_eq!(
            parse(text),
            pairs(&[("MIDlet-Name", "A"), ("MIDlet-Version", "1.0")])
        );
    }

    #[test]
    fn continuation_lines_drop_exactly_one_space() {
        let text =
            b"MIDlet-1: Button, /i.png, com.nokia.button.ButtonM\r\n idlet\r\nX: a\r\n  b: c\r\n";
        assert_eq!(
            parse(text),
            pairs(&[
                ("MIDlet-1", "Button, /i.png, com.nokia.button.ButtonMidlet"),
                ("X", "a b: c")
            ])
        );
    }

    #[test]
    fn keeps_the_first_of_repeated_keys_and_skips_bad_lines() {
        let text = b"A: 1\nno colon here\nA: 2\n: empty key\nB: x:y\n";
        assert_eq!(parse(text), pairs(&[("A", "1"), ("B", "x:y")]));
    }

    #[test]
    fn invalid_utf8_lines_decode_as_latin1() {
        let text = b"MIDlet-Name: \xC8\xE3\xF0\xE0\nMIDlet-Vendor: \xC3\xA9\n";
        let parsed = parse(text);
        assert_eq!(parsed[0].1, "\u{C8}\u{E3}\u{F0}\u{E0}");
        assert_eq!(parsed[1].1, "\u{E9}");
    }

    #[test]
    fn forty_thousand_keys_keep_the_first_4096() {
        use std::fmt::Write;
        let mut text = String::new();
        for i in 0..40_000 {
            write!(text, "K{i:07}: v{i}\r\n").unwrap();
        }
        let parsed = parse(text.as_bytes());
        assert_eq!(parsed.len(), MAX_ATTRIBUTES);
        assert_eq!(parsed[0], ("K0000000".to_owned(), "v0".to_owned()));
        assert_eq!(parsed[4095].0, "K0004095");
    }

    #[test]
    fn a_key_repeated_a_hundred_thousand_times_is_kept_once() {
        let text = "A: b\n".repeat(100_000);
        assert_eq!(parse(text.as_bytes()), pairs(&[("A", "b")]));
    }

    #[test]
    fn text_past_a_mebibyte_is_not_read() {
        // Distinct keys would hit the attribute limit first, so the filler
        // repeats one key and the last line is what the cut removes.
        let mut text = String::from("First: 1\n");
        while text.len() < 2 * MAX_ATTRIBUTE_BYTES {
            text.push_str("Filler: x\n");
        }
        text.push_str("Last: 2\n");
        let parsed = parse(text.as_bytes());
        assert!(parsed.iter().any(|(k, _)| k == "First"));
        assert!(parsed.iter().all(|(k, _)| k != "Last"));

        // One value of two mebibytes keeps its first mebibyte.
        let big = format!("Big: {}\nAfter: 1\n", "x".repeat(2 * MAX_ATTRIBUTE_BYTES));
        let parsed = parse(big.as_bytes());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1.len(), MAX_ATTRIBUTE_BYTES - "Big: ".len());
    }

    #[test]
    fn a_cut_inside_a_character_drops_the_character_and_not_the_encoding() {
        // "K: " and then two byte characters: the cut at 1 MiB falls in the
        // middle of one of them.
        let text = format!("K: {}", "\u{e9}".repeat(MAX_ATTRIBUTE_BYTES));
        let parsed = parse(text.as_bytes());
        let value = &parsed[0].1;
        assert!(value.chars().all(|c| c == '\u{e9}'), "decoded as UTF-8");
        assert_eq!(value.len(), MAX_ATTRIBUTE_BYTES - 4);
    }

    #[test]
    fn a_value_continued_over_a_hundred_thousand_lines_is_joined() {
        let text = format!("Long: a\n{}", " b\n".repeat(100_000));
        let parsed = parse(text.as_bytes());
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1.len(), 1 + 100_000);
    }

    #[test]
    fn a_continuation_of_a_dropped_line_is_dropped_with_it() {
        let text = b"no colon\n here\nA: 1\n";
        assert_eq!(parse(text), pairs(&[("A", "1")]));
        // A first line that starts with a space has nothing to continue.
        assert_eq!(parse(b" B: 2\nA: 1\n"), pairs(&[("B", "2"), ("A", "1")]));
    }

    #[test]
    fn class_names_with_brackets_controls_or_over_255_bytes_are_refused() {
        for bad in [
            "[[I",
            "[Ljava.lang.String;",
            "com.a.Game[]",
            "com.a.Ga\tme",
            "com.a.Game\u{0}",
            "com.a.G\u{85}ame",
            "com/a/\nGame",
        ] {
            assert_eq!(class_name(bad), None, "{bad:?}");
            assert!(
                parse_midlet(&format!("Game, , {bad}"), "S").is_none(),
                "{bad:?}"
            );
        }
        let long = "a".repeat(300);
        assert_eq!(class_name(&long), None);
        assert_eq!(class_name(&format!("{long}.class")), None);
        let longest = format!("{}.Game", "a".repeat(250));
        assert_eq!(longest.len(), MAX_CLASS_NAME_BYTES);
        assert_eq!(class_name(&longest).as_deref(), Some(longest.as_str()));
        assert_eq!(class_name(&format!("x{longest}")), None);
        // The names that games use still pass.
        assert_eq!(
            class_name("com/a/Game.class").as_deref(),
            Some("com.a.Game")
        );
        assert_eq!(class_name("/Game").as_deref(), Some("Game"));
        assert_eq!(class_name("a.B$C").as_deref(), Some("a.B$C"));
    }

    #[test]
    fn a_midlet_with_a_refused_class_leaves_the_others() {
        let attrs = pairs(&[("MIDlet-1", "One, , [[I"), ("MIDlet-2", "Two, , com.a.Two")]);
        let (list, _) = midlets(&attrs, "S");
        let classes: Vec<_> = list.iter().map(|m| m.class_name.as_str()).collect();
        assert_eq!(classes, ["com.a.Two"]);
    }

    #[test]
    fn a_value_of_a_hundred_thousand_commas_is_read_without_a_list_of_fields() {
        let value = format!("N{}, com.a.Game", ",".repeat(100_000));
        let midlet = parse_midlet(&value, "S").unwrap();
        assert_eq!(midlet.name, "N");
        assert_eq!(midlet.class_name, "com.a.Game");
        assert_eq!(midlet.icon_path.map(|icon| icon.len()), Some(99_999));
    }

    #[test]
    fn lookup_prefers_exact_case() {
        let attrs = pairs(&[("midlet-name", "lower"), ("MIDlet-Name", "exact")]);
        assert_eq!(lookup(&attrs, "MIDlet-Name"), Some("exact"));
        assert_eq!(lookup(&attrs, "MIDLET-NAME"), Some("lower"));
        assert_eq!(lookup(&attrs, "MIDlet-Vendor"), None);
    }

    #[test]
    fn midlet_entries_split_on_commas() {
        let m = parse_midlet(" Snake , /icons/s.png , com.acme.Snake ", "Suite").unwrap();
        assert_eq!(m.name, "Snake");
        assert_eq!(m.icon_path.as_deref(), Some("icons/s.png"));
        assert_eq!(m.class_name, "com.acme.Snake");

        let two = parse_midlet("Snake, Snake", "Suite").unwrap();
        assert_eq!(
            (two.name.as_str(), two.icon_path, two.class_name.as_str()),
            ("Snake", None, "Snake")
        );

        let bare = parse_midlet("com/acme/Main.class", "Suite").unwrap();
        assert_eq!(
            (bare.name.as_str(), bare.class_name.as_str()),
            ("Suite", "com.acme.Main")
        );

        let empty_icon = parse_midlet("A,,B", "").unwrap();
        assert_eq!(empty_icon.icon_path, None);
        assert!(parse_midlet("Name,", "").is_none());
    }

    #[test]
    fn the_list_stops_at_the_first_gap() {
        let attrs = pairs(&[
            ("MIDlet-2", "Two, , B"),
            ("MIDlet-1", "One, , A"),
            ("MIDlet-4", "Four, , D"),
        ]);
        let (list, notes) = midlets(&attrs, "S");
        let classes: Vec<_> = list.iter().map(|m| m.class_name.as_str()).collect();
        assert_eq!(classes, ["A", "B"]);
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn higher_numbers_stand_in_for_a_missing_first() {
        let attrs = pairs(&[("MIDlet-3", "Three, , C"), ("MIDlet-2", "Two, , B")]);
        let (list, notes) = midlets(&attrs, "S");
        let classes: Vec<_> = list.iter().map(|m| m.class_name.as_str()).collect();
        assert_eq!(classes, ["B", "C"]);
        assert_eq!(notes.len(), 1);
        assert!(midlets(&[], "S").0.is_empty());
    }
}
