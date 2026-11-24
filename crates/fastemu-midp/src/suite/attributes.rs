//! The attribute files of a suite: the JAR manifest and the JAD, which
//! share one `Name: value` format, and the `MIDlet-<n>` entries they hold.
//!
//! The parser follows `docs/compat/notes/01-suite-loading.md`: it decodes
//! bytes leniently, strips a UTF-8 byte order mark, splits lines on CR, LF
//! or CR LF, joins a line that starts with a space to the one before it
//! with exactly one space removed, skips blank lines, splits each line at
//! its first colon, trims both sides, and keeps the first value of a key
//! that repeats. No line ever rejects the whole file.

use fastemu_core::MidletInfo;

/// The attributes of one file, in file order, without repeated keys.
pub fn parse(bytes: &[u8]) -> Vec<(String, String)> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut logical: Vec<Vec<u8>> = Vec::new();
    for line in split_lines(bytes) {
        if let Some(rest) = line.strip_prefix(b" ")
            && let Some(last) = logical.last_mut()
        {
            last.extend_from_slice(rest);
            continue;
        }
        logical.push(line.to_vec());
    }
    let mut attributes: Vec<(String, String)> = Vec::new();
    for line in logical {
        let text = decode(&line);
        let Some((key, value)) = text.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() || attributes.iter().any(|(k, _)| k == key) {
            continue;
        }
        attributes.push((key.to_owned(), value.trim().to_owned()));
    }
    attributes
}

/// Splits on CR, LF and CR LF, dropping lines that are empty or blank.
fn split_lines(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes
        .split(|&b| b == b'\n' || b == b'\r')
        .filter(|line| line.iter().any(|b| !b.is_ascii_whitespace()))
}

/// UTF-8 when the line is valid UTF-8, else Latin-1, which maps every byte
/// to one character so nothing is lost.
fn decode(line: &[u8]) -> String {
    match std::str::from_utf8(line) {
        Ok(s) => s.to_owned(),
        Err(_) => line.iter().map(|&b| char::from(b)).collect(),
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
/// class alone.
pub fn parse_midlet(value: &str, suite_name: &str) -> Option<MidletInfo> {
    let fields: Vec<&str> = value.split(',').map(str::trim).collect();
    let class = class_name(fields.last()?)?;
    let name = if fields.len() > 1 && !fields[0].is_empty() {
        fields[0].to_owned()
    } else if !suite_name.is_empty() {
        suite_name.to_owned()
    } else {
        class.rsplit('.').next().unwrap_or(&class).to_owned()
    };
    let icon_path = (fields.len() >= 3)
        .then(|| fields[1..fields.len() - 1].join(","))
        .map(|icon| icon.trim().trim_start_matches('/').to_owned())
        .filter(|icon| !icon.is_empty());
    Some(MidletInfo {
        name,
        icon_path,
        class_name: class,
    })
}

/// A class name as manifests write it (`com.a.Game`, `com/a/Game`,
/// `Game.class`) in binary form, `com.a.Game`.
pub fn class_name(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw.strip_suffix(".class").unwrap_or(raw);
    let name = raw.trim_start_matches('/').replace('/', ".");
    (!name.is_empty()).then_some(name)
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
