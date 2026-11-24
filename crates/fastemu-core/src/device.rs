use crate::input::Key;

/// The key codes a phone sends for the keys that differ between vendors.
///
/// MIDP leaves the codes of soft keys, arrows and the fire key to the
/// manufacturer, and games test for the codes of the phones they target.
/// The presets follow the codes those vendors' phones sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeyCodes {
    pub soft_left: i32,
    pub soft_right: i32,
    pub up: i32,
    pub down: i32,
    pub left: i32,
    pub right: i32,
    pub fire: i32,
    pub clear: i32,
    pub send: i32,
    pub end: i32,
}

impl KeyCodes {
    /// Nokia, and the many phones that copied its codes (Sony Ericsson,
    /// Samsung and most generic MIDP 2.0 phones).
    pub const NOKIA: KeyCodes = KeyCodes {
        soft_left: -6,
        soft_right: -7,
        up: -1,
        down: -2,
        left: -3,
        right: -4,
        fire: -5,
        clear: -8,
        send: -10,
        end: -11,
    };

    /// Siemens phones. The soft keys, arrows and fire key follow the
    /// emulators that studied Siemens phones; no source gives Siemens codes
    /// for clear, send and end, so those keep Nokia's, which is what other
    /// emulators' Siemens layouts send as well.
    pub const SIEMENS: KeyCodes = KeyCodes {
        soft_left: -1,
        soft_right: -4,
        up: -59,
        down: -60,
        left: -61,
        right: -62,
        fire: -26,
        clear: -8,
        send: -10,
        end: -11,
    };

    /// Motorola's most common layout (the RAZR V3 and its kin). Clear, send
    /// and end are Nokia's: no source gives Motorola codes for them.
    pub const MOTOROLA: KeyCodes = KeyCodes {
        soft_left: -21,
        soft_right: -22,
        up: -1,
        down: -6,
        left: -2,
        right: -5,
        fire: -20,
        clear: -8,
        send: -10,
        end: -11,
    };

    /// LG phones: Nokia's arrows and fire key, soft keys -202 and -203
    /// and clear -204, as freej2me-plus has them. Send and end are
    /// Nokia's: no source gives LG codes for them.
    pub const LG: KeyCodes = KeyCodes {
        soft_left: -202,
        soft_right: -203,
        up: -1,
        down: -2,
        left: -3,
        right: -4,
        fire: -5,
        clear: -204,
        send: -10,
        end: -11,
    };

    /// The code this phone sends for `key`.
    pub fn code(&self, key: Key) -> i32 {
        if let Some(code) = key.itu_code() {
            return code;
        }
        match key {
            Key::Up => self.up,
            Key::Down => self.down,
            Key::Left => self.left,
            Key::Right => self.right,
            Key::Fire => self.fire,
            Key::SoftLeft => self.soft_left,
            Key::SoftRight => self.soft_right,
            Key::Clear => self.clear,
            Key::Send => self.send,
            Key::End => self.end,
            _ => unreachable!("ITU keys returned above"),
        }
    }

    /// The key that sends `code` on this phone, if any. If two keys ever
    /// share a code, the first in [`Key::ALL`] wins.
    pub fn key(&self, code: i32) -> Option<Key> {
        Key::ALL.into_iter().find(|&k| self.code(k) == code)
    }
}

/// The line heights of MIDP's three font sizes on a phone, in pixels: what
/// `Font.getHeight()` returns. Real phones differ widely and no measured
/// table exists, so these are choices in the range other emulators use.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FontSizes {
    pub small: u32,
    pub medium: u32,
    pub large: u32,
}

/// What a game sees of the phone it runs on.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceProfile {
    /// A stable identifier, such as `nokia-s40-240x320`.
    pub id: String,
    /// A name for menus.
    pub name: String,
    pub screen_width: u32,
    pub screen_height: u32,
    /// The `microedition.platform` system property.
    pub platform: String,
    pub key_codes: KeyCodes,
    /// Whether the screen reports pointer events (`hasPointerEvents`).
    pub touch: bool,
    /// `Display.numColors()`.
    pub colors: u32,
    /// `Display.numAlphaLevels()`.
    pub alpha_levels: u32,
    /// The heap size the phone reports through `Runtime.totalMemory()`.
    pub heap_bytes: u64,
    pub font_sizes: FontSizes,
}

impl DeviceProfile {
    #[allow(clippy::too_many_arguments)]
    fn preset(
        id: &str,
        name: &str,
        (screen_width, screen_height): (u32, u32),
        platform: &str,
        key_codes: KeyCodes,
        touch: bool,
        heap_mib: u64,
        font_sizes: FontSizes,
    ) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            screen_width,
            screen_height,
            platform: platform.to_owned(),
            key_codes,
            touch,
            colors: 1 << 24,
            alpha_levels: 256,
            heap_bytes: heap_mib * 1024 * 1024,
            font_sizes,
        }
    }
}

const SMALL_SCREEN_FONTS: FontSizes = FontSizes {
    small: 10,
    medium: 12,
    large: 14,
};
const MEDIUM_SCREEN_FONTS: FontSizes = FontSizes {
    small: 12,
    medium: 14,
    large: 16,
};
const LARGE_SCREEN_FONTS: FontSizes = FontSizes {
    small: 14,
    medium: 16,
    large: 20,
};

/// The profile FastEmu starts games with when nothing else is set: a
/// 240x320 MIDP 2.0 phone with Nokia's key codes, the most common target.
pub fn default_profile() -> DeviceProfile {
    builtin_profiles()
        .into_iter()
        .next()
        .expect("the built-in profiles are not empty")
}

/// The phones FastEmu ships profiles for. The first is the default.
pub fn builtin_profiles() -> Vec<DeviceProfile> {
    use DeviceProfile as P;
    vec![
        P::preset(
            "generic-240x320",
            "Generic 240x320",
            (240, 320),
            "Nokia6300/2.0 (05.00) Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            false,
            8,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "generic-320x240",
            "Generic 320x240 (landscape)",
            (320, 240),
            "NokiaE71/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            false,
            8,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "generic-360x640",
            "Generic 360x640 (touch)",
            (360, 640),
            "Nokia5800d-1/2.0 Profile/MIDP-2.1 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            true,
            16,
            LARGE_SCREEN_FONTS,
        ),
        P::preset(
            "nokia-s40-128x160",
            "Nokia Series 40 128x160",
            (128, 160),
            "Nokia6230i/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            false,
            2,
            SMALL_SCREEN_FONTS,
        ),
        P::preset(
            "nokia-s60-176x208",
            "Nokia Series 60 176x208",
            (176, 208),
            "Nokia6600/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.0",
            KeyCodes::NOKIA,
            false,
            4,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "sonyericsson-176x220",
            "Sony Ericsson 176x220",
            (176, 220),
            "SonyEricssonK700i/R2AE Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            false,
            4,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "siemens-132x176",
            "Siemens 132x176",
            (132, 176),
            "SIEMENS-CX65/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::SIEMENS,
            false,
            2,
            SMALL_SCREEN_FONTS,
        ),
        P::preset(
            "motorola-176x220",
            "Motorola 176x220",
            (176, 220),
            "motorazrV3/2.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::MOTOROLA,
            false,
            4,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "samsung-176x220",
            "Samsung 176x220",
            (176, 220),
            "SAMSUNG-SGH-D500/1.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::NOKIA,
            false,
            4,
            MEDIUM_SCREEN_FONTS,
        ),
        P::preset(
            "lg-176x220",
            "LG 176x220",
            (176, 220),
            "LG-U8180/V1.0 Profile/MIDP-2.0 Configuration/CLDC-1.1",
            KeyCodes::LG,
            false,
            4,
            MEDIUM_SCREEN_FONTS,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip_through_keys() {
        for codes in [
            KeyCodes::NOKIA,
            KeyCodes::SIEMENS,
            KeyCodes::MOTOROLA,
            KeyCodes::LG,
        ] {
            for key in Key::ALL {
                assert_eq!(codes.key(codes.code(key)), Some(key), "{key:?}");
            }
        }
    }

    #[test]
    fn itu_keys_keep_their_codes_on_every_phone() {
        assert_eq!(KeyCodes::SIEMENS.code(Key::Num5), '5' as i32);
        assert_eq!(KeyCodes::SIEMENS.key('#' as i32), Some(Key::Pound));
    }

    #[test]
    fn profile_ids_are_unique() {
        let profiles = builtin_profiles();
        let mut ids: Vec<_> = profiles.iter().map(|p| p.id.clone()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), profiles.len());
        assert_eq!(default_profile().screen_width, 240);
    }
}
