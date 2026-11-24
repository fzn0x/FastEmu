/// The keys of a Java ME phone.
///
/// The twelve ITU-T keys (`0` to `9`, `*`, `#`) have the same key code on
/// every phone: their character code. The rest take device-specific codes,
/// which [`crate::KeyCodes`] maps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Key {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Star,
    Pound,
    Up,
    Down,
    Left,
    Right,
    Fire,
    SoftLeft,
    SoftRight,
    Clear,
    Send,
    End,
}

impl Key {
    pub const ALL: [Key; 22] = [
        Key::Num0,
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
        Key::Star,
        Key::Pound,
        Key::Up,
        Key::Down,
        Key::Left,
        Key::Right,
        Key::Fire,
        Key::SoftLeft,
        Key::SoftRight,
        Key::Clear,
        Key::Send,
        Key::End,
    ];

    /// The key code of an ITU-T key, which MIDP fixes as its character code
    /// (`Canvas.KEY_NUM0` is 48). `None` for keys whose codes vary by phone.
    pub fn itu_code(self) -> Option<i32> {
        let c = match self {
            Key::Num0 => '0',
            Key::Num1 => '1',
            Key::Num2 => '2',
            Key::Num3 => '3',
            Key::Num4 => '4',
            Key::Num5 => '5',
            Key::Num6 => '6',
            Key::Num7 => '7',
            Key::Num8 => '8',
            Key::Num9 => '9',
            Key::Star => '*',
            Key::Pound => '#',
            _ => return None,
        };
        Some(c as i32)
    }

    /// A short name for menus and key binding lists.
    pub fn label(self) -> &'static str {
        match self {
            Key::Num0 => "0",
            Key::Num1 => "1",
            Key::Num2 => "2",
            Key::Num3 => "3",
            Key::Num4 => "4",
            Key::Num5 => "5",
            Key::Num6 => "6",
            Key::Num7 => "7",
            Key::Num8 => "8",
            Key::Num9 => "9",
            Key::Star => "*",
            Key::Pound => "#",
            Key::Up => "Up",
            Key::Down => "Down",
            Key::Left => "Left",
            Key::Right => "Right",
            Key::Fire => "Fire",
            Key::SoftLeft => "Left soft key",
            Key::SoftRight => "Right soft key",
            Key::Clear => "Clear",
            Key::Send => "Send",
            Key::End => "End",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum KeyAction {
    Press,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PointerAction {
    Press,
    Drag,
    Release,
}

/// Input an app sends to an instance. Key repeats are the runtime's job:
/// apps send one press and one release per key stroke.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InputEvent {
    Key {
        key: Key,
        action: KeyAction,
    },
    /// A key code with no [`Key`], such as a vendor's extra key.
    RawKey {
        code: i32,
        action: KeyAction,
    },
    /// A touch or mouse event in screen pixels, `(0, 0)` at the top left.
    Pointer {
        action: PointerAction,
        x: i32,
        y: i32,
    },
    /// Text typed on a host keyboard, for the text fields of forms and
    /// text boxes.
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn itu_keys_use_their_character_codes() {
        assert_eq!(Key::Num0.itu_code(), Some(48));
        assert_eq!(Key::Num9.itu_code(), Some(57));
        assert_eq!(Key::Star.itu_code(), Some(42));
        assert_eq!(Key::Pound.itu_code(), Some(35));
        assert_eq!(Key::Fire.itu_code(), None);
    }

    #[test]
    fn all_lists_every_key_once() {
        let mut keys = Key::ALL.to_vec();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), Key::ALL.len());
    }
}
