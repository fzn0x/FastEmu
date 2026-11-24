//! Which phone key a hardware key stands for.
//!
//! The names are gpui's `Keystroke::key`: the character a key types for
//! printable keys, a word such as `up` or `enter` for named ones, and for
//! Android keys gpui has no name for, the lowercased `Keycode` (`dpadcenter`,
//! `softleft`, `call`). Android's system keys (Back, Home, volume) never get
//! here: the backend leaves them to Android.

use fastemu_core::Key;

/// The phone key that `host_key` presses, if any.
pub fn phone_key(host_key: &str) -> Option<Key> {
    let key = match host_key {
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "*" => Key::Star,
        "#" => Key::Pound,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        // A keyboard's Enter and Space, a D-pad's centre and a gamepad's A
        // button are where a thumb looks for "select".
        "enter" | "space" | "dpadcenter" | "buttona" => Key::Fire,
        "f1" | "softleft" | "menu" => Key::SoftLeft,
        "f2" | "softright" => Key::SoftRight,
        "backspace" | "escape" | "clear" => Key::Clear,
        "f3" | "call" => Key::Send,
        "f4" | "endcall" => Key::End,
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_and_symbols_press_the_itu_keys() {
        for (name, key) in [("0", Key::Num0), ("5", Key::Num5), ("9", Key::Num9)] {
            assert_eq!(phone_key(name), Some(key));
        }
        assert_eq!(phone_key("*"), Some(Key::Star));
        assert_eq!(phone_key("#"), Some(Key::Pound));
    }

    #[test]
    fn arrows_and_select_keys_drive_the_d_pad() {
        assert_eq!(phone_key("up"), Some(Key::Up));
        assert_eq!(phone_key("left"), Some(Key::Left));
        for name in ["enter", "space", "dpadcenter", "buttona"] {
            assert_eq!(phone_key(name), Some(Key::Fire), "{name}");
        }
    }

    #[test]
    fn soft_and_call_keys_have_function_key_stand_ins() {
        assert_eq!(phone_key("f1"), Some(Key::SoftLeft));
        assert_eq!(phone_key("softright"), Some(Key::SoftRight));
        assert_eq!(phone_key("backspace"), Some(Key::Clear));
        assert_eq!(phone_key("call"), Some(Key::Send));
        assert_eq!(phone_key("f4"), Some(Key::End));
    }

    #[test]
    fn letters_and_unknown_keys_press_nothing() {
        for name in ["a", "tab", "f12", "back", ""] {
            assert_eq!(phone_key(name), None, "{name}");
        }
    }

    #[test]
    fn every_phone_key_has_a_host_key() {
        let names = [
            "0",
            "1",
            "2",
            "3",
            "4",
            "5",
            "6",
            "7",
            "8",
            "9",
            "*",
            "#",
            "up",
            "down",
            "left",
            "right",
            "enter",
            "f1",
            "f2",
            "backspace",
            "f3",
            "f4",
        ];
        let mut keys: Vec<Key> = names.iter().filter_map(|name| phone_key(name)).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), Key::ALL.len());
    }
}
