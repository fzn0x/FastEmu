//! Text from a game or a file name, made safe to print or show.

/// Escapes what a terminal or a reader could take for something other than
/// text. CR and LF become the two characters `\r` and `\n`, so a line stays
/// one line, and every other control character (below U+0020 but the tab,
/// U+007F, and U+0080 to U+009F, which hold the one character forms of
/// escape sequences) becomes U+FFFD. ESC is one of them, so no sequence
/// that clears a screen, sets a title or writes to the clipboard survives.
/// So do the bidirectional embeddings, overrides and isolates (U+202A to
/// U+202E, U+2066 to U+2069), which could show a line's text in another
/// order than it has.
///
/// Every line the runtime logs passes through this, and so does every
/// piece of game or file text the apps print to a terminal.
pub fn escape_control(text: impl Into<String>) -> String {
    let text = text.into();
    if !text.chars().any(is_escaped) {
        return text;
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            c if is_escaped(c) => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out
}

/// Whether [`escape_control`] replaces `c`.
fn is_escaped(c: char) -> bool {
    (c.is_control() && c != '\t') || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_left_alone() {
        assert_eq!(escape_control("Snake 1.0\ttab"), "Snake 1.0\ttab");
        assert_eq!(escape_control(String::new()), "");
        // Letters of right to left scripts and the marks stay as they are.
        let hebrew = "\u{5e9}\u{5dc}\u{5d5}\u{5dd} \u{200f}ok\u{200e}";
        assert_eq!(escape_control(hebrew), hebrew);
    }

    #[test]
    fn line_breaks_and_escape_sequences_are_escaped() {
        assert_eq!(
            escape_control("\u{1b}[2J\u{1b}]0;owned\u{7}ok\r\ninfo: fake"),
            "\u{fffd}[2J\u{fffd}]0;owned\u{fffd}ok\\r\\ninfo: fake"
        );
        assert_eq!(
            escape_control("\u{85} and \u{7f} and \u{9b}31m"),
            "\u{fffd} and \u{fffd} and \u{fffd}31m"
        );
    }

    #[test]
    fn bidirectional_overrides_are_escaped() {
        assert_eq!(
            escape_control("score 100 \u{202e}txt.exe\u{202c} and \u{2067}abc\u{2069}"),
            "score 100 \u{fffd}txt.exe\u{fffd} and \u{fffd}abc\u{fffd}"
        );
    }
}
