//! The on-screen phone keypad: which keys it shows, and the bookkeeping that
//! sends each key stroke to the game exactly once.

use fastemu_core::{InputEvent, Key, KeyAction};

/// One button of the on-screen keypad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeypadKey {
    pub key: Key,
    /// What the button shows.
    pub label: &'static str,
}

const fn button(key: Key, label: &'static str) -> KeypadKey {
    KeypadKey { key, label }
}

/// The keypad's rows, top to bottom, three buttons each: the soft keys and
/// the D-pad, then the twelve ITU-T keys as a phone lays them out.
pub const ROWS: [[KeypadKey; 3]; 7] = [
    [
        button(Key::SoftLeft, "Soft L"),
        button(Key::Up, "\u{2191}"),
        button(Key::SoftRight, "Soft R"),
    ],
    [
        button(Key::Left, "\u{2190}"),
        button(Key::Fire, "OK"),
        button(Key::Right, "\u{2192}"),
    ],
    [
        button(Key::Send, "Call"),
        button(Key::Down, "\u{2193}"),
        button(Key::Clear, "C"),
    ],
    [
        button(Key::Num1, "1"),
        button(Key::Num2, "2"),
        button(Key::Num3, "3"),
    ],
    [
        button(Key::Num4, "4"),
        button(Key::Num5, "5"),
        button(Key::Num6, "6"),
    ],
    [
        button(Key::Num7, "7"),
        button(Key::Num8, "8"),
        button(Key::Num9, "9"),
    ],
    [
        button(Key::Star, "*"),
        button(Key::Num0, "0"),
        button(Key::Pound, "#"),
    ],
];

/// The phone keys held down on an instance.
///
/// The touch keypad, a mouse and a hardware keyboard can all press keys, and
/// hardware keys repeat while held. MIDP wants one press and one release per
/// stroke (the runtime makes its own repeats), so every press and release
/// goes through here and only changes reach the game.
#[derive(Debug, Default)]
pub struct HeldKeys {
    held: Vec<Key>,
}

impl HeldKeys {
    /// The event that presses `key`, or `None` if it is held already.
    pub fn press(&mut self, key: Key) -> Option<InputEvent> {
        if self.held.contains(&key) {
            return None;
        }
        self.held.push(key);
        Some(InputEvent::Key {
            key,
            action: KeyAction::Press,
        })
    }

    /// The event that releases `key`, or `None` if it isn't held.
    pub fn release(&mut self, key: Key) -> Option<InputEvent> {
        let index = self.held.iter().position(|&held| held == key)?;
        self.held.remove(index);
        Some(InputEvent::Key {
            key,
            action: KeyAction::Release,
        })
    }

    /// Releases every held key, for when the app loses the keys' releases:
    /// it went to the background, or the game was replaced.
    pub fn release_all(&mut self) -> Vec<InputEvent> {
        self.held
            .drain(..)
            .map(|key| InputEvent::Key {
                key,
                action: KeyAction::Release,
            })
            .collect()
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.held.contains(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keypad_shows_every_phone_key_but_end_once() {
        let mut keys: Vec<Key> = ROWS.iter().flatten().map(|button| button.key).collect();
        keys.sort();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before, "a key shows twice");
        let missing: Vec<Key> = Key::ALL
            .into_iter()
            .filter(|key| !keys.contains(key))
            .collect();
        // End (hang up) closes games on real phones; it stays a hardware key.
        assert_eq!(missing, [Key::End]);
    }

    #[test]
    fn the_number_rows_follow_a_phone() {
        let labels: Vec<&str> = ROWS[3..]
            .iter()
            .flatten()
            .map(|button| button.label)
            .collect();
        assert_eq!(
            labels,
            ["1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"]
        );
    }

    #[test]
    fn a_stroke_sends_one_press_and_one_release() {
        let mut held = HeldKeys::default();
        let press = InputEvent::Key {
            key: Key::Fire,
            action: KeyAction::Press,
        };
        assert_eq!(held.press(Key::Fire), Some(press));
        // A hardware key's repeats.
        assert_eq!(held.press(Key::Fire), None);
        assert!(held.is_held(Key::Fire));
        assert_eq!(
            held.release(Key::Fire),
            Some(InputEvent::Key {
                key: Key::Fire,
                action: KeyAction::Release,
            })
        );
        assert_eq!(held.release(Key::Fire), None);
        assert!(!held.is_held(Key::Fire));
    }

    #[test]
    fn release_all_lets_go_of_every_key() {
        let mut held = HeldKeys::default();
        held.press(Key::Up);
        held.press(Key::Num5);
        let released = held.release_all();
        assert_eq!(released.len(), 2);
        assert!(released.iter().all(|event| matches!(
            event,
            InputEvent::Key {
                action: KeyAction::Release,
                ..
            }
        )));
        assert!(held.release_all().is_empty());
        assert_eq!(held.release(Key::Up), None);
    }
}
