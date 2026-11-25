//! The text of a `TextBox` or a `TextField`: its characters, caret,
//! maximum size and input constraints, and the editing done on them by
//! the API, by a host keyboard ([`TextEditor::type_text`]) and by a
//! phone's keypad ([`TextEditor::key`], multi-tap).
//!
//! Text is UTF-16, as Java holds it, so positions mean what they mean to
//! the game.

use std::time::{Duration, Instant};

/// `TextField.ANY`.
pub const ANY: i32 = 0;
/// `TextField.EMAILADDR`.
pub const EMAILADDR: i32 = 1;
/// `TextField.NUMERIC`.
pub const NUMERIC: i32 = 2;
/// `TextField.PHONENUMBER`.
pub const PHONENUMBER: i32 = 3;
/// `TextField.URL`.
pub const URL: i32 = 4;
/// `TextField.DECIMAL`.
pub const DECIMAL: i32 = 5;
/// `TextField.PASSWORD`.
pub const PASSWORD: i32 = 0x1_0000;
/// `TextField.UNEDITABLE`.
pub const UNEDITABLE: i32 = 0x2_0000;
/// `TextField.SENSITIVE`.
pub const SENSITIVE: i32 = 0x4_0000;
/// `TextField.NON_PREDICTIVE`.
pub const NON_PREDICTIVE: i32 = 0x8_0000;
/// `TextField.INITIAL_CAPS_WORD`.
pub const INITIAL_CAPS_WORD: i32 = 0x10_0000;
/// `TextField.INITIAL_CAPS_SENTENCE`.
pub const INITIAL_CAPS_SENTENCE: i32 = 0x20_0000;
/// `TextField.CONSTRAINT_MASK`.
pub const CONSTRAINT_MASK: i32 = 0xFFFF;

/// How long a multi-tap letter waits for the same key again before it
/// is final.
pub const MULTI_TAP: Duration = Duration::from_millis(900);

/// Why an API call was refused, named after the exception it throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditError {
    /// `IllegalArgumentException`.
    IllegalArgument(&'static str),
    /// `StringIndexOutOfBoundsException`.
    StringIndex,
}

/// Whether `constraints` is valid: its low 16 bits name one of `ANY` to
/// `DECIMAL`. Unknown modifier bits are let through, as the reference
/// implementation does, since games pass odd values.
pub fn valid_constraints(constraints: i32) -> bool {
    (ANY..=DECIMAL).contains(&(constraints & CONSTRAINT_MASK))
}

const MINUS: u16 = b'-' as u16;
const DOT: u16 = b'.' as u16;

fn is_digit(unit: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&unit)
}

fn is_phone(unit: u16) -> bool {
    is_digit(unit) || b"+*#pw ()-".iter().any(|&c| u16::from(c) == unit)
}

/// Whether `text` is allowed by `constraints`. `NUMERIC` takes an
/// optional leading minus and digits, `DECIMAL` also one decimal point,
/// `PHONENUMBER` digits and `+*#pw ()-`; the others take any text. A lone
/// minus or point passes, since the user may be part way through typing.
pub fn allowed(text: &[u16], constraints: i32) -> bool {
    allowed_units(text.iter().copied(), constraints)
}

/// [`allowed`] for a text given as its units, so that an edit is checked
/// before it is made, on the text it would give, without building that
/// text.
fn allowed_units(units: impl Iterator<Item = u16> + Clone, constraints: i32) -> bool {
    let body = || {
        let minus = units.clone().next() == Some(MINUS);
        units.clone().skip(usize::from(minus))
    };
    match constraints & CONSTRAINT_MASK {
        NUMERIC => body().all(is_digit),
        DECIMAL => {
            body().all(|u| is_digit(u) || u == DOT) && body().filter(|&u| u == DOT).count() <= 1
        }
        PHONENUMBER => units.clone().all(is_phone),
        _ => true,
    }
}

/// The letter case of keypad input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMode {
    /// "Abc": a capital at the start of a sentence.
    Sentence,
    /// "Abc" for each word: a capital at the start of every word.
    Word,
    /// "abc".
    Lower,
    /// "ABC".
    Upper,
    /// "123": the keys type their digits.
    Digits,
}

impl InputMode {
    /// What a phone shows for the mode.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sentence | Self::Word => "Abc",
            Self::Lower => "abc",
            Self::Upper => "ABC",
            Self::Digits => "123",
        }
    }
}

/// A multi-tap letter that may still change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Pending {
    key: u16,
    index: usize,
    at: usize,
    until: Instant,
}

/// The letters a keypad key cycles through in a text mode.
fn letters(key: u16, kind: i32) -> &'static str {
    let mail = kind == EMAILADDR || kind == URL;
    match char::from_u32(u32::from(key)).unwrap_or(' ') {
        '1' if mail => ".@/:-_1",
        '1' => ".,?!'\"-@1",
        '2' => "abc2",
        '3' => "def3",
        '4' => "ghi4",
        '5' => "jkl5",
        '6' => "mno6",
        '7' => "pqrs7",
        '8' => "tuv8",
        '9' => "wxyz9",
        '0' => " 0",
        '*' => "*+-/=%&#$()<>;",
        _ => "",
    }
}

/// The text of a text box or field and how it is edited.
#[derive(Clone, Debug)]
pub struct TextEditor {
    text: Vec<u16>,
    /// How many times the text changed, for telling whether a key did.
    edits: u64,
    caret: usize,
    max_size: usize,
    constraints: i32,
    mode: InputMode,
    pending: Option<Pending>,
}

impl TextEditor {
    /// A new editor, as `TextBox(title, text, maxSize, constraints)` and
    /// `TextField(label, text, maxSize, constraints)` make one.
    pub fn new(text: &[u16], max_size: i32, constraints: i32) -> Result<Self, EditError> {
        if max_size <= 0 {
            return Err(EditError::IllegalArgument("maxSize must be positive"));
        }
        if !valid_constraints(constraints) {
            return Err(EditError::IllegalArgument("invalid constraints"));
        }
        let mut editor = Self {
            text: Vec::new(),
            edits: 0,
            caret: 0,
            max_size: max_size as usize,
            constraints,
            mode: InputMode::Lower,
            pending: None,
        };
        editor.mode = editor.default_mode();
        editor.set_text(text)?;
        Ok(editor)
    }

    fn default_mode(&self) -> InputMode {
        match self.kind() {
            NUMERIC | DECIMAL | PHONENUMBER => InputMode::Digits,
            EMAILADDR | URL => InputMode::Lower,
            _ if self.constraints & INITIAL_CAPS_WORD != 0 => InputMode::Word,
            _ => InputMode::Sentence,
        }
    }

    /// The constraint without modifiers: `ANY` to `DECIMAL`.
    pub fn kind(&self) -> i32 {
        self.constraints & CONSTRAINT_MASK
    }

    /// The text.
    pub fn text(&self) -> &[u16] {
        &self.text
    }

    /// How many times the text changed.
    pub fn edits(&self) -> u64 {
        self.edits
    }

    /// The caret, from 0 (before the first character) to the length.
    pub fn caret(&self) -> usize {
        self.caret
    }

    /// The most characters the text may hold.
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// `getConstraints()`.
    pub fn constraints(&self) -> i32 {
        self.constraints
    }

    /// The keypad's current input mode.
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Whether the user may change the text.
    pub fn editable(&self) -> bool {
        self.constraints & UNEDITABLE == 0
    }

    /// Whether the text is shown as stars.
    pub fn masked(&self) -> bool {
        self.constraints & PASSWORD != 0
    }

    /// The position of the multi-tap letter still open, if any.
    pub fn pending_at(&self) -> Option<usize> {
        self.pending.map(|p| p.at)
    }

    /// When the open multi-tap letter becomes final.
    pub fn pending_until(&self) -> Option<Instant> {
        self.pending.map(|p| p.until)
    }

    /// Makes the open multi-tap letter final. Returns whether there was
    /// one.
    pub fn commit(&mut self) -> bool {
        self.pending.take().is_some()
    }

    /// Ends multi-tap if its time has passed. Returns whether it did.
    pub fn expire(&mut self, now: Instant) -> bool {
        if self.pending.is_some_and(|p| now >= p.until) {
            self.pending = None;
            return true;
        }
        false
    }

    fn check(&self, text: &[u16]) -> Result<(), EditError> {
        if text.len() > self.max_size {
            return Err(EditError::IllegalArgument(
                "the text is longer than maxSize",
            ));
        }
        if !allowed(text, self.constraints) {
            return Err(EditError::IllegalArgument(
                "the text breaks the input constraints",
            ));
        }
        Ok(())
    }

    /// `setString` and `setChars`: replaces the text, the caret at its
    /// end.
    pub fn set_text(&mut self, text: &[u16]) -> Result<(), EditError> {
        self.check(text)?;
        self.text = text.to_vec();
        self.edits += 1;
        self.caret = self.text.len();
        self.pending = None;
        Ok(())
    }

    /// Whether `src` may take the place of `text[start..end]`: the result
    /// must fit `maxSize` and the constraints. The result is not built.
    fn check_edit(&self, start: usize, end: usize, src: &[u16]) -> Result<(), EditError> {
        let len = (self.text.len() - (end - start)).saturating_add(src.len());
        if len > self.max_size {
            return Err(EditError::IllegalArgument(
                "the text is longer than maxSize",
            ));
        }
        let (before, after) = (&self.text[..start], &self.text[end..]);
        let next = before.iter().chain(src).chain(after).copied();
        if !allowed_units(next, self.constraints) {
            return Err(EditError::IllegalArgument(
                "the text breaks the input constraints",
            ));
        }
        Ok(())
    }

    /// `insert(src, position)`: `position` is clamped into the text; the
    /// caret moves along when the text goes in at or before it. The edit
    /// is checked on the text it would give and then made in place, so
    /// an insert costs what it inserts, not the length of the text.
    pub fn insert(&mut self, src: &[u16], position: i32) -> Result<(), EditError> {
        let at = usize::try_from(position).map_or(0, |p| p.min(self.text.len()));
        self.check_edit(at, at, src)?;
        self.text.splice(at..at, src.iter().copied());
        self.edits += 1;
        if at <= self.caret {
            self.caret += src.len();
        }
        self.pending = None;
        Ok(())
    }

    /// `delete(offset, length)`.
    pub fn delete(&mut self, offset: i32, length: i32) -> Result<(), EditError> {
        let len = self.text.len() as i64;
        let (offset, length) = (i64::from(offset), i64::from(length));
        if offset < 0 || length < 0 || offset + length > len {
            return Err(EditError::StringIndex);
        }
        let range = offset as usize..(offset + length) as usize;
        let (before, after) = (&self.text[..range.start], &self.text[range.end..]);
        if !allowed_units(before.iter().chain(after).copied(), self.constraints) {
            return Err(EditError::IllegalArgument(
                "the text would break the input constraints",
            ));
        }
        self.text.drain(range.clone());
        self.edits += 1;
        if self.caret >= range.end {
            self.caret -= range.len();
        } else if self.caret > range.start {
            self.caret = range.start;
        }
        self.pending = None;
        Ok(())
    }

    /// `setMaxSize(maxSize)`: truncates the text if it is longer and
    /// returns the new maximum.
    pub fn set_max_size(&mut self, max_size: i32) -> Result<i32, EditError> {
        if max_size <= 0 {
            return Err(EditError::IllegalArgument("maxSize must be positive"));
        }
        let max = max_size as usize;
        let kept = &self.text[..self.text.len().min(max)];
        if !allowed(kept, self.constraints) {
            return Err(EditError::IllegalArgument(
                "the truncated text would break the input constraints",
            ));
        }
        self.text.truncate(max);
        self.max_size = max;
        self.caret = self.caret.min(self.text.len());
        self.pending = None;
        Ok(max_size)
    }

    /// `setConstraints(constraints)`: text the new constraints do not
    /// allow is cleared.
    pub fn set_constraints(&mut self, constraints: i32) -> Result<(), EditError> {
        if !valid_constraints(constraints) {
            return Err(EditError::IllegalArgument("invalid constraints"));
        }
        let kind_changed = constraints & CONSTRAINT_MASK != self.kind();
        self.constraints = constraints;
        if !allowed(&self.text, constraints) {
            self.text.clear();
            self.edits += 1;
            self.caret = 0;
        }
        if kind_changed {
            self.mode = self.default_mode();
        }
        self.pending = None;
        Ok(())
    }

    /// `setInitialInputMode(characterSubset)`: the subsets a phone keypad
    /// can type switch the mode; others are a hint FastEmu does not need.
    pub fn set_initial_mode(&mut self, subset: Option<&str>) {
        if !matches!(self.kind(), ANY | EMAILADDR | URL) {
            return;
        }
        self.mode = match subset {
            Some("IS_LATIN_DIGITS") => InputMode::Digits,
            Some("MIDP_UPPERCASE_LATIN") => InputMode::Upper,
            Some("MIDP_LOWERCASE_LATIN") => InputMode::Lower,
            _ => self.default_mode(),
        };
    }

    /// Moves the caret by `delta` characters, ending multi-tap.
    pub fn move_caret(&mut self, delta: i32) {
        self.pending = None;
        let target = self.caret as i64 + i64::from(delta);
        self.caret = target.clamp(0, self.text.len() as i64) as usize;
    }

    /// Puts the caret at `position`, ending multi-tap.
    pub fn set_caret(&mut self, position: usize) {
        self.pending = None;
        self.caret = position.min(self.text.len());
    }

    /// Whether `unit` may go in at the caret.
    fn fits(&self, unit: u16) -> bool {
        if self.text.len() >= self.max_size || unit < 0x20 && unit != u16::from(b'\n') {
            return false;
        }
        let (before, after) = self.text.split_at(self.caret);
        let next = before
            .iter()
            .copied()
            .chain([unit])
            .chain(after.iter().copied());
        allowed_units(next, self.constraints)
    }

    /// Types `text` from a host keyboard at the caret: characters the
    /// constraints refuse, and those past the maximum size, are dropped.
    /// Returns whether the text changed.
    pub fn type_text(&mut self, text: &str) -> bool {
        if !self.editable() {
            return false;
        }
        self.pending = None;
        let mut changed = false;
        for unit in text.encode_utf16() {
            let unit = if unit == u16::from(b'\r') {
                u16::from(b'\n')
            } else {
                unit
            };
            if self.fits(unit) {
                self.text.insert(self.caret, unit);
                self.edits += 1;
                self.caret += 1;
                changed = true;
            }
        }
        changed
    }

    /// Deletes the character before the caret, as the clear key does.
    /// Returns whether the text changed.
    pub fn backspace(&mut self) -> bool {
        if !self.editable() || self.caret == 0 {
            return false;
        }
        self.pending = None;
        self.caret -= 1;
        self.text.remove(self.caret);
        self.edits += 1;
        true
    }

    /// Whether a capital belongs at the caret in the current mode.
    fn wants_capital(&self) -> bool {
        let before = &self.text[..self.caret];
        let last = before.iter().rev().find(|&&u| u != u16::from(b' '));
        match self.mode {
            InputMode::Upper => true,
            InputMode::Word => before.last().is_none_or(|&u| u == u16::from(b' ')),
            InputMode::Sentence => {
                last.is_none_or(|&u| b".!?".iter().any(|&c| u16::from(c) == u))
                    && before.last().is_none_or(|&u| u == u16::from(b' '))
            }
            InputMode::Lower | InputMode::Digits => false,
        }
    }

    /// A keypad key, by its character (`'0'` to `'9'`, `'*'`, `'#'`), at
    /// `now`. Text modes cycle letters when the same key comes again
    /// within [`MULTI_TAP`]; `#` changes the mode; numeric constraints
    /// type digits, and `*` and `#` give their signs. Returns whether
    /// anything changed.
    pub fn key(&mut self, key: u16, now: Instant) -> bool {
        if !self.editable() {
            return false;
        }
        let star = key == u16::from(b'*');
        let pound = key == u16::from(b'#');
        let digit = is_digit(key);
        if !(digit || star || pound) {
            return false;
        }
        match self.kind() {
            NUMERIC => {
                self.pending = None;
                let unit = if digit { key } else { MINUS };
                self.put(unit)
            }
            DECIMAL => {
                self.pending = None;
                let unit = match () {
                    () if digit => key,
                    () if star => DOT,
                    () => MINUS,
                };
                self.put(unit)
            }
            PHONENUMBER if star => self.cycle(key, "*+pw", now),
            PHONENUMBER => {
                self.pending = None;
                self.put(key)
            }
            _ if pound => {
                self.pending = None;
                self.mode = match self.mode {
                    InputMode::Sentence | InputMode::Word => InputMode::Lower,
                    InputMode::Lower => InputMode::Upper,
                    InputMode::Upper => InputMode::Digits,
                    InputMode::Digits => self.default_mode_for_text(),
                };
                true
            }
            _ if self.mode == InputMode::Digits && digit => {
                self.pending = None;
                self.put(key)
            }
            kind => self.cycle(key, letters(key, kind), now),
        }
    }

    /// The letter mode `#` returns to after digits.
    fn default_mode_for_text(&self) -> InputMode {
        match self.default_mode() {
            InputMode::Digits => InputMode::Sentence,
            mode => mode,
        }
    }

    /// Inserts `unit` at the caret if it fits.
    fn put(&mut self, unit: u16) -> bool {
        if !self.fits(unit) {
            return false;
        }
        self.text.insert(self.caret, unit);
        self.edits += 1;
        self.caret += 1;
        true
    }

    /// One press of a multi-tap key whose letters are `cycle`.
    fn cycle(&mut self, key: u16, cycle: &str, now: Instant) -> bool {
        let units: Vec<u16> = cycle.encode_utf16().collect();
        if units.is_empty() {
            return false;
        }
        let capital = self.wants_capital();
        let shape = |unit: u16| -> u16 {
            if !capital {
                return unit;
            }
            char::from_u32(u32::from(unit)).map_or(unit, |c| c.to_ascii_uppercase() as u16)
        };
        if let Some(p) = self.pending
            && p.key == key
            && now < p.until
            && p.at < self.text.len()
        {
            // The same key again: the next letter the constraints allow.
            for step in 1..=units.len() {
                let index = (p.index + step) % units.len();
                let unit = shape_like(self.text[p.at], units[index]);
                let next =
                    (self.text.iter().enumerate()).map(|(i, &u)| if i == p.at { unit } else { u });
                if allowed_units(next, self.constraints) {
                    if self.text[p.at] != unit {
                        self.text[p.at] = unit;
                        self.edits += 1;
                    }
                    self.pending = Some(Pending {
                        index,
                        until: now + MULTI_TAP,
                        ..p
                    });
                    return true;
                }
            }
            return false;
        }
        self.pending = None;
        for (index, &unit) in units.iter().enumerate() {
            let unit = shape(unit);
            if self.fits(unit) {
                let at = self.caret;
                self.text.insert(at, unit);
                self.edits += 1;
                self.caret += 1;
                self.pending = Some(Pending {
                    key,
                    index,
                    at,
                    until: now + MULTI_TAP,
                });
                return true;
            }
        }
        false
    }
}

/// `unit` in the case of `like`, so cycling keeps a capital a capital.
fn shape_like(like: u16, unit: u16) -> u16 {
    let upper = char::from_u32(u32::from(like)).is_some_and(|c| c.is_ascii_uppercase());
    match char::from_u32(u32::from(unit)) {
        Some(c) if upper => c.to_ascii_uppercase() as u16,
        _ => unit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn text(editor: &TextEditor) -> String {
        String::from_utf16_lossy(editor.text())
    }

    fn editor(text: &str, max: i32, constraints: i32) -> TextEditor {
        TextEditor::new(&units(text), max, constraints).expect("a valid editor")
    }

    fn keys(editor: &mut TextEditor, keys: &str, now: Instant) {
        for key in keys.encode_utf16() {
            editor.key(key, now);
        }
    }

    #[test]
    fn constraints_filter_whole_texts() {
        assert!(allowed(&units("-12"), NUMERIC));
        assert!(allowed(&units("-"), NUMERIC));
        assert!(!allowed(&units("1-2"), NUMERIC));
        assert!(!allowed(&units("1.5"), NUMERIC));
        assert!(allowed(&units("-1.5"), DECIMAL));
        assert!(!allowed(&units("1.5.2"), DECIMAL));
        assert!(allowed(&units("+44 (20) 123-45p6"), PHONENUMBER));
        assert!(!allowed(&units("call me"), PHONENUMBER));
        assert!(allowed(&units("anything at all"), ANY | PASSWORD));
        assert!(valid_constraints(DECIMAL | PASSWORD | 0x4000_0000));
        assert!(!valid_constraints(6));
    }

    #[test]
    fn construction_checks_size_and_constraints() {
        assert_eq!(
            TextEditor::new(&units("abc"), 0, ANY).err(),
            Some(EditError::IllegalArgument("maxSize must be positive"))
        );
        assert!(TextEditor::new(&units("abcd"), 3, ANY).is_err());
        assert!(TextEditor::new(&units("12a"), 10, NUMERIC).is_err());
        assert!(TextEditor::new(&units(""), 10, 9).is_err());
        let e = editor("hello", 10, ANY);
        assert_eq!(e.caret(), 5);
    }

    #[test]
    fn api_edits_move_the_caret_and_check() {
        let mut e = editor("hello", 10, ANY);
        e.insert(&units("X"), 0).unwrap();
        assert_eq!((text(&e).as_str(), e.caret()), ("Xhello", 6));
        e.insert(&units("!"), 99).unwrap();
        assert_eq!(text(&e), "Xhello!");
        assert_eq!(
            e.insert(&units("toolong"), 0),
            Err(EditError::IllegalArgument(
                "the text is longer than maxSize"
            ))
        );
        e.delete(0, 1).unwrap();
        assert_eq!((text(&e).as_str(), e.caret()), ("hello!", 6));
        assert_eq!(e.delete(4, 5), Err(EditError::StringIndex));
        assert_eq!(e.set_max_size(3), Ok(3));
        assert_eq!((text(&e).as_str(), e.caret()), ("hel", 3));
        let mut n = editor("-5", 4, NUMERIC);
        assert!(n.delete(1, 1).is_ok());
        assert!(n.insert(&units("x"), 0).is_err());
    }

    /// Every text of up to `len` units over a few that the constraints
    /// treat differently.
    fn small_texts(len: usize) -> Vec<Vec<u16>> {
        let mut all = vec![Vec::new()];
        let mut last = vec![Vec::new()];
        for _ in 0..len {
            last = last
                .iter()
                .flat_map(|t: &Vec<u16>| {
                    "1-.x+".encode_utf16().map(move |u| {
                        let mut longer = t.clone();
                        longer.push(u);
                        longer
                    })
                })
                .collect();
            all.extend(last.iter().cloned());
        }
        all
    }

    #[test]
    fn edits_made_in_place_are_allowed_when_the_text_they_give_is() {
        for constraints in [ANY, NUMERIC, DECIMAL, PHONENUMBER, NUMERIC | PASSWORD] {
            for before in small_texts(3).iter().filter(|t| allowed(t, constraints)) {
                for src in small_texts(2) {
                    for at in 0..=before.len() {
                        let mut want = before.clone();
                        want.splice(at..at, src.iter().copied());
                        let mut e = TextEditor::new(before, 5, constraints).unwrap();
                        let done = e.insert(&src, at as i32);
                        let fits = want.len() <= 5 && allowed(&want, constraints);
                        assert_eq!(done.is_ok(), fits, "{before:?} + {src:?} at {at}");
                        let text = if fits { &want } else { before };
                        assert_eq!(e.text(), text.as_slice(), "{before:?} + {src:?} at {at}");
                    }
                }
                for offset in 0..=before.len() {
                    for length in 0..=before.len() - offset {
                        let mut want = before.clone();
                        want.drain(offset..offset + length);
                        let mut e = TextEditor::new(before, 5, constraints).unwrap();
                        let done = e.delete(offset as i32, length as i32);
                        let fits = allowed(&want, constraints);
                        assert_eq!(done.is_ok(), fits, "{before:?} - {offset}+{length}");
                        let text = if fits { &want } else { before };
                        assert_eq!(e.text(), text.as_slice(), "{before:?} - {offset}+{length}");
                    }
                }
            }
        }
    }

    #[test]
    fn a_refused_edit_changes_nothing() {
        let mut e = editor("12", 4, NUMERIC);
        let edits = e.edits();
        assert!(e.insert(&units("x"), 1).is_err());
        assert!(e.insert(&units("345"), 1).is_err());
        assert!(e.delete(0, 3).is_err());
        assert_eq!((text(&e).as_str(), e.caret(), e.edits()), ("12", 2, edits));
        e.insert(&units("3"), 1).unwrap();
        assert_eq!((text(&e).as_str(), e.caret()), ("132", 3));
        assert_eq!(e.edits(), edits + 1);
    }

    #[test]
    fn new_constraints_clear_what_they_refuse() {
        let mut e = editor("abc", 10, ANY);
        e.set_constraints(NUMERIC).unwrap();
        assert_eq!(text(&e), "");
        assert_eq!(e.mode(), InputMode::Digits);
        let mut d = editor("12", 10, ANY);
        d.set_constraints(NUMERIC | PASSWORD).unwrap();
        assert_eq!(text(&d), "12");
        assert!(d.masked());
    }

    #[test]
    fn typed_text_goes_in_at_the_caret_and_is_filtered() {
        let mut e = editor("ad", 5, ANY);
        e.move_caret(-1);
        assert!(e.type_text("bc"));
        assert_eq!(text(&e), "abcd");
        assert!(e.type_text("xyz"));
        assert_eq!(text(&e), "abcxd", "the maximum size stops it");
        let mut n = editor("", 8, NUMERIC);
        n.type_text("a-1b2.3");
        assert_eq!(text(&n), "-123");
        let mut locked = editor("x", 8, ANY | UNEDITABLE);
        assert!(!locked.type_text("y"));
        assert!(!locked.backspace());
    }

    #[test]
    fn backspace_deletes_before_the_caret() {
        let mut e = editor("abc", 5, ANY);
        assert!(e.backspace());
        assert_eq!((text(&e).as_str(), e.caret()), ("ab", 2));
        e.set_caret(0);
        assert!(!e.backspace());
    }

    #[test]
    fn multi_tap_cycles_and_times_out() {
        let t = Instant::now();
        let mut e = editor("", 20, ANY);
        assert_eq!(e.mode(), InputMode::Sentence);
        keys(&mut e, "44", t);
        assert_eq!(text(&e), "H", "a capital starts the sentence");
        e.key(u16::from(b'4'), t + MULTI_TAP * 2);
        assert_eq!(text(&e), "Hg", "a late press starts a new letter");
        e.commit();
        keys(&mut e, "444", t + MULTI_TAP * 2);
        assert_eq!(text(&e), "Hgi");
        assert!(e.expire(t + MULTI_TAP * 4));
        keys(&mut e, "0", t + MULTI_TAP * 4);
        assert_eq!(text(&e), "Hgi ");
        e.commit();
        keys(&mut e, "2", t + MULTI_TAP * 5);
        assert_eq!(text(&e), "Hgi a", "no capital inside a sentence");
    }

    #[test]
    fn pound_changes_the_mode() {
        let t = Instant::now();
        let mut e = editor("", 20, ANY);
        keys(&mut e, "#", t);
        assert_eq!(e.mode(), InputMode::Lower);
        keys(&mut e, "#", t);
        assert_eq!(e.mode(), InputMode::Upper);
        keys(&mut e, "2", t);
        e.commit();
        keys(&mut e, "#", t);
        assert_eq!(e.mode(), InputMode::Digits);
        keys(&mut e, "42", t);
        assert_eq!(text(&e), "A42");
        keys(&mut e, "#", t);
        assert_eq!(e.mode(), InputMode::Sentence);
    }

    #[test]
    fn numeric_keys_type_digits_and_signs() {
        let t = Instant::now();
        let mut n = editor("", 10, NUMERIC);
        keys(&mut n, "*12*", t);
        assert_eq!(text(&n), "-12");
        let mut d = editor("", 10, DECIMAL);
        keys(&mut d, "#3*14*", t);
        assert_eq!(text(&d), "-3.14");
        let mut p = editor("", 10, PHONENUMBER);
        keys(&mut p, "**", t);
        keys(&mut p, "1#", t);
        assert_eq!(text(&p), "+1#");
    }

    #[test]
    fn mail_keys_offer_the_at_sign() {
        let t = Instant::now();
        let mut e = editor("", 20, EMAILADDR);
        assert_eq!(e.mode(), InputMode::Lower);
        keys(&mut e, "11", t);
        assert_eq!(text(&e), "@");
    }

    #[test]
    fn initial_modes() {
        let mut e = editor("", 5, ANY);
        e.set_initial_mode(Some("IS_LATIN_DIGITS"));
        assert_eq!(e.mode(), InputMode::Digits);
        e.set_initial_mode(Some("UCB_BASIC_LATIN"));
        assert_eq!(e.mode(), InputMode::Sentence);
        let mut w = editor("", 20, ANY | INITIAL_CAPS_WORD);
        let t = Instant::now();
        keys(&mut w, "2", t);
        w.commit();
        keys(&mut w, "0", t);
        w.commit();
        keys(&mut w, "2", t);
        assert_eq!(text(&w), "A A");
    }
}
