//! Which host keyboard keys press which phone keys, and the bookkeeping
//! that turns host key strokes into exactly one press and one release per
//! phone key.
//!
//! Host keys are named the way GPUI names them in a keystroke: `up`,
//! `enter`, `space`, `f1`, `backspace`, single characters such as `5`, `*`
//! or `#`. The number row and the numeric keypad produce the same names, so
//! one binding covers both.

use std::collections::BTreeMap;

use fastemu_core::Key;
use serde::{Deserialize, Serialize};

/// The host keys bound to each phone key. A host key presses at most one
/// phone key; a phone key may have several host keys.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyBindings {
    keys: BTreeMap<Key, Vec<String>>,
}

impl Default for KeyBindings {
    /// Arrows to the D-pad, Enter and Space to fire, F1 and F2 to the soft
    /// keys, digits to the keypad, `*` and `#` (numeric keypad `*` and `/`
    /// too) and Backspace to clear.
    fn default() -> Self {
        let mut keys = BTreeMap::new();
        let mut bind = |key: Key, hosts: &[&str]| {
            keys.insert(key, hosts.iter().map(|&h| h.to_owned()).collect());
        };
        bind(Key::Up, &["up"]);
        bind(Key::Down, &["down"]);
        bind(Key::Left, &["left"]);
        bind(Key::Right, &["right"]);
        bind(Key::Fire, &["enter", "space"]);
        bind(Key::SoftLeft, &["f1"]);
        bind(Key::SoftRight, &["f2"]);
        bind(Key::Clear, &["backspace"]);
        bind(Key::Star, &["*"]);
        bind(Key::Pound, &["#", "/"]);
        for (key, digit) in Key::ALL[..10].iter().zip('0'..='9') {
            bind(*key, &[digit.to_string().as_str()]);
        }
        Self { keys }
    }
}

impl KeyBindings {
    /// The host keys bound to `key`, in the order they were added.
    pub fn hosts(&self, key: Key) -> &[String] {
        self.keys.get(&key).map_or(&[], Vec::as_slice)
    }

    /// The phone key `host` presses, if any.
    pub fn key_for(&self, host: &str) -> Option<Key> {
        let host = normalize(host);
        self.keys
            .iter()
            .find(|(_, hosts)| hosts.contains(&host))
            .map(|(&key, _)| key)
    }

    /// Binds `host` to `key`, taking it away from any other phone key so a
    /// host key never presses two phone keys.
    pub fn bind(&mut self, key: Key, host: &str) {
        let host = normalize(host);
        for hosts in self.keys.values_mut() {
            hosts.retain(|h| *h != host);
        }
        self.keys.entry(key).or_default().push(host);
    }

    /// Removes the binding of `host` to `key`.
    pub fn unbind(&mut self, key: Key, host: &str) {
        let host = normalize(host);
        if let Some(hosts) = self.keys.get_mut(&key) {
            hosts.retain(|h| *h != host);
        }
    }

    /// Picks the phone key for a host keystroke. GPUI gives both the key
    /// (`3`) and the character it typed (`#` with Shift held); the character
    /// wins when it is bound, so Shift+3 can press `#` on layouts that put
    /// `#` there.
    pub fn resolve(&self, key: &str, key_char: Option<&str>) -> Option<Key> {
        key_char
            .filter(|c| !c.is_empty())
            .and_then(|c| self.key_for(c))
            .or_else(|| self.key_for(key))
    }
}

/// Host key names compare without case, except single symbols which have
/// no case anyway.
fn normalize(host: &str) -> String {
    host.trim().to_lowercase()
}

/// A readable name for a host key, for the bindings editor.
pub fn host_label(host: &str) -> String {
    match host {
        "space" => "Space".to_owned(),
        "enter" => "Enter".to_owned(),
        "backspace" => "Backspace".to_owned(),
        "up" => "Up".to_owned(),
        "down" => "Down".to_owned(),
        "left" => "Left".to_owned(),
        "right" => "Right".to_owned(),
        "escape" => "Esc".to_owned(),
        "tab" => "Tab".to_owned(),
        other
            if other.len() > 1
                && other.starts_with('f')
                && other[1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            other.to_uppercase()
        }
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}

/// What holds a phone key down: a host keyboard key, by the name GPUI
/// gives it, or the on-screen keypad.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Source {
    Host(String),
    Keypad,
}

/// Tracks what holds each phone key down, the keyboard's keys and the
/// on-screen keypad alike, so each phone key gets one press when its first
/// source goes down and one release when its last source lets go, whatever
/// the OS sends in between. A key the keyboard holds stays held when the
/// keypad lets go of it, and the other way round.
#[derive(Debug, Default)]
pub struct KeyTracker {
    held: Vec<(Source, Key)>,
}

impl KeyTracker {
    /// Records `host` going down for `key`. Returns `true` when the phone
    /// key was not held yet, the moment to send its press. A host key that
    /// is already down (an OS key repeat) changes nothing.
    pub fn press(&mut self, host: &str, key: Key) -> bool {
        if self
            .held
            .iter()
            .any(|(s, _)| matches!(s, Source::Host(h) if h == host))
        {
            return false;
        }
        let first = !self.is_held(key);
        self.held.push((Source::Host(host.to_owned()), key));
        first
    }

    /// Records `host` coming up and returns the phone keys to release.
    ///
    /// The OS may name the key differently on the way up: Shift+3 goes down
    /// as `#` and, if Shift is let go first, comes up as `3` (on AZERTY,
    /// down as `1` and up as `&`). A digit or symbol release that matches
    /// no held key therefore also releases the keys digits and symbols
    /// hold, so they cannot stick. Letters and named keys never rename
    /// that way, so their releases leave other keys alone.
    pub fn release(&mut self, host: &str) -> Vec<Key> {
        let matches_held = self
            .held
            .iter()
            .any(|(s, _)| matches!(s, Source::Host(h) if h == host));
        let renamed = !matches_held && is_char_key(host);
        self.release_where(|source| match source {
            Source::Host(h) if matches_held => h == host,
            Source::Host(h) => renamed && is_char_key(h),
            Source::Keypad => false,
        })
    }

    /// Holds `key` from the on-screen keypad, which holds one key at a
    /// time. Returns the phone keys to release (the keypad's last key,
    /// unless the keyboard holds it too) and whether `key` needs a press.
    pub fn press_keypad(&mut self, key: Key) -> (Vec<Key>, bool) {
        let released = self.release_keypad();
        let first = !self.is_held(key);
        self.held.push((Source::Keypad, key));
        (released, first)
    }

    /// Lets go of the keypad's key and returns the phone keys to release:
    /// none when the keyboard still holds it.
    pub fn release_keypad(&mut self) -> Vec<Key> {
        self.release_where(|source| *source == Source::Keypad)
    }

    /// The phone key the on-screen keypad holds.
    pub fn keypad_key(&self) -> Option<Key> {
        self.held
            .iter()
            .find(|(s, _)| *s == Source::Keypad)
            .map(|&(_, key)| key)
    }

    /// Lets go of everything, as when the game loses keyboard focus.
    pub fn release_all(&mut self) -> Vec<Key> {
        self.release_where(|_| true)
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held.iter().any(|&(_, k)| k == key)
    }

    /// Drops the sources `drop` picks and returns the phone keys no
    /// source holds any more, once each.
    fn release_where(&mut self, drop: impl Fn(&Source) -> bool) -> Vec<Key> {
        let mut released = Vec::new();
        self.held.retain(|(source, key)| {
            let dropped = drop(source);
            if dropped {
                released.push(*key);
            }
            !dropped
        });
        released.retain(|&key| !self.is_held(key));
        released.sort();
        released.dedup();
        released
    }
}

/// A key named by the one digit or symbol it types, the kind Shift
/// renames.
fn is_char_key(host: &str) -> bool {
    let mut chars = host.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_the_documented_keys() {
        let b = KeyBindings::default();
        assert_eq!(b.key_for("up"), Some(Key::Up));
        assert_eq!(b.key_for("enter"), Some(Key::Fire));
        assert_eq!(b.key_for("space"), Some(Key::Fire));
        assert_eq!(b.key_for("f1"), Some(Key::SoftLeft));
        assert_eq!(b.key_for("f2"), Some(Key::SoftRight));
        assert_eq!(b.key_for("0"), Some(Key::Num0));
        assert_eq!(b.key_for("7"), Some(Key::Num7));
        assert_eq!(b.key_for("*"), Some(Key::Star));
        assert_eq!(b.key_for("#"), Some(Key::Pound));
        assert_eq!(b.key_for("/"), Some(Key::Pound));
        assert_eq!(b.key_for("backspace"), Some(Key::Clear));
        assert_eq!(b.key_for("q"), None);
    }

    #[test]
    fn host_names_ignore_case() {
        assert_eq!(KeyBindings::default().key_for("F1"), Some(Key::SoftLeft));
    }

    #[test]
    fn the_typed_character_wins_when_bound() {
        let b = KeyBindings::default();
        assert_eq!(b.resolve("3", Some("#")), Some(Key::Pound));
        assert_eq!(b.resolve("8", Some("*")), Some(Key::Star));
        assert_eq!(b.resolve("space", Some(" ")), Some(Key::Fire));
        assert_eq!(b.resolve("3", Some("3")), Some(Key::Num3));
        assert_eq!(b.resolve("f9", None), None);
    }

    #[test]
    fn binding_moves_a_host_key() {
        let mut b = KeyBindings::default();
        b.bind(Key::Fire, "f1");
        assert_eq!(b.key_for("f1"), Some(Key::Fire));
        assert!(b.hosts(Key::SoftLeft).is_empty());
        b.unbind(Key::Fire, "F1");
        assert_eq!(b.key_for("f1"), None);
    }

    #[test]
    fn bindings_round_trip_through_json() {
        let mut b = KeyBindings::default();
        b.bind(Key::Send, "f3");
        let json = serde_json::to_string(&b).expect("serializes");
        assert!(json.contains("\"SoftLeft\":[\"f1\"]"), "{json}");
        let back: KeyBindings = serde_json::from_str(&json).expect("parses");
        assert_eq!(back, b);
    }

    #[test]
    fn repeats_and_shared_keys_press_once() {
        let mut t = KeyTracker::default();
        assert!(t.press("enter", Key::Fire));
        assert!(!t.press("enter", Key::Fire), "an OS repeat");
        assert!(!t.press("space", Key::Fire), "already held by enter");
        assert!(t.release("enter").is_empty(), "space still holds it");
        assert_eq!(t.release("space"), vec![Key::Fire]);
        assert!(t.release("space").is_empty());
    }

    #[test]
    fn a_renamed_release_frees_symbol_keys() {
        let mut t = KeyTracker::default();
        assert!(t.press("#", Key::Pound));
        assert!(t.press("up", Key::Up));
        assert_eq!(t.release("3"), vec![Key::Pound]);
        assert!(t.is_held(Key::Up));
        assert_eq!(t.release_all(), vec![Key::Up]);
    }

    #[test]
    fn other_keys_coming_up_leave_symbols_held() {
        let mut t = KeyTracker::default();
        assert!(t.press("#", Key::Pound));
        assert!(t.release("q").is_empty(), "an unbound letter");
        assert!(t.release("escape").is_empty());
        assert!(t.is_held(Key::Pound));
    }

    #[test]
    fn a_renamed_release_frees_shifted_digits() {
        // On AZERTY layouts Shift types the digits: down as `1`, up as
        // `&` once Shift is let go.
        let mut t = KeyTracker::default();
        assert!(t.press("1", Key::Num1));
        assert_eq!(t.release("&"), vec![Key::Num1]);
    }

    #[test]
    fn the_keypad_and_the_keyboard_share_holds() {
        let mut t = KeyTracker::default();
        assert!(t.press("up", Key::Up));
        assert_eq!(t.press_keypad(Key::Up), (vec![], false), "held already");
        assert!(t.release_keypad().is_empty(), "the keyboard still holds Up");
        assert!(t.is_held(Key::Up));
        assert_eq!(t.press_keypad(Key::Fire), (vec![], true));
        assert!(!t.press("enter", Key::Fire), "the keypad holds Fire");
        assert!(t.release("enter").is_empty(), "the keypad still holds Fire");
        assert_eq!(t.keypad_key(), Some(Key::Fire));
        assert_eq!(t.release("up"), vec![Key::Up]);
        assert_eq!(t.press_keypad(Key::Num5), (vec![Key::Fire], true));
        assert_eq!(t.release_all(), vec![Key::Num5]);
        assert_eq!(t.keypad_key(), None);
    }

    #[test]
    fn a_renamed_release_leaves_the_keypad_alone() {
        let mut t = KeyTracker::default();
        assert_eq!(t.press_keypad(Key::Pound), (vec![], true));
        assert!(t.press("#", Key::Star));
        assert_eq!(t.release("3"), vec![Key::Star]);
        assert_eq!(t.keypad_key(), Some(Key::Pound));
    }

    #[test]
    fn labels_read_well() {
        assert_eq!(host_label("f1"), "F1");
        assert_eq!(host_label("space"), "Space");
        assert_eq!(host_label("#"), "#");
        assert_eq!(host_label("w"), "W");
    }
}
