//! App-wide and per-game settings, as saved in `settings.json` and in each
//! library entry.
//!
//! Every field has a default and unknown fields are ignored, so a settings
//! file from an older or newer FastEmu still loads.

use fastemu_core::{DeviceProfile, InstanceSettings, NetworkPolicy, builtin_profiles};
use serde::{Deserialize, Serialize};

use crate::bindings::KeyBindings;

/// How the workspace arranges running games.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutMode {
    /// Every game the same size, in the grid that fits the window best.
    #[default]
    Grid,
    /// The focused game large, the others as small live thumbnails.
    Focus,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

/// Settings that apply to the whole app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// The id of the device profile games start with unless their own
    /// settings pick another.
    pub default_profile: String,
    pub layout: LayoutMode,
    /// Whether tiles show the on-screen phone keypad.
    pub keypad: bool,
    pub theme: ThemeMode,
    pub bindings: KeyBindings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_profile: fastemu_core::default_profile().id,
            layout: LayoutMode::default(),
            keypad: true,
            theme: ThemeMode::default(),
            bindings: KeyBindings::default(),
        }
    }
}

/// Settings of one game in the library, used each time it starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// A device profile id, or `None` to follow the app's default.
    pub profile: Option<String>,
    /// 0.0 (silent) to 1.0.
    pub volume: f32,
    /// Whether the game may open network connections.
    pub network: bool,
    /// The `microedition.encoding` the game sees.
    pub encoding: String,
}

impl Default for GameSettings {
    fn default() -> Self {
        let base = InstanceSettings::default();
        Self {
            profile: None,
            volume: base.volume,
            network: base.network == NetworkPolicy::Allow,
            encoding: base.encoding,
        }
    }
}

impl GameSettings {
    /// The instance settings a launch request carries.
    pub fn instance_settings(&self) -> InstanceSettings {
        InstanceSettings {
            encoding: self.encoding.clone(),
            volume: self.volume.clamp(0.0, 1.0),
            network: if self.network {
                NetworkPolicy::Allow
            } else {
                NetworkPolicy::Deny
            },
            ..InstanceSettings::default()
        }
    }

    /// Moves the volume by `delta`, kept between 0 and 1 in steps of a
    /// tenth so repeated clicks land on round values.
    pub fn step_volume(&mut self, delta: f32) {
        self.volume = ((self.volume + delta) * 10.0).round().clamp(0.0, 10.0) / 10.0;
    }
}

/// The character encodings offered for `microedition.encoding`: the ones
/// phones of the era shipped with.
pub const ENCODINGS: &[&str] = &[
    "ISO-8859-1",
    "UTF-8",
    "ISO-8859-2",
    "windows-1250",
    "windows-1251",
    "windows-1252",
    "KOI8-R",
    "Shift_JIS",
    "EUC-JP",
    "GBK",
    "Big5",
    "EUC-KR",
];

/// The profile with `id` among the built-in ones, or the default profile
/// when the id is unknown (a profile removed in a newer version).
pub fn profile_by_id(id: &str) -> DeviceProfile {
    builtin_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap_or_else(fastemu_core::default_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_defaults() {
        let s: AppSettings = serde_json::from_str(r#"{"layout":"focus"}"#).expect("parses");
        assert_eq!(s.layout, LayoutMode::Focus);
        assert_eq!(s.theme, ThemeMode::Dark);
        assert!(s.keypad);
        assert_eq!(s.bindings, KeyBindings::default());
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let s: GameSettings =
            serde_json::from_str(r#"{"volume":0.5,"future":true}"#).expect("parses");
        assert!((s.volume - 0.5).abs() < f32::EPSILON);
        assert_eq!(s.encoding, "ISO-8859-1");
    }

    #[test]
    fn game_settings_become_instance_settings() {
        let game = GameSettings {
            profile: None,
            volume: 1.5,
            network: true,
            encoding: "windows-1251".into(),
        };
        let s = game.instance_settings();
        assert_eq!(s.network, NetworkPolicy::Allow);
        assert_eq!(s.encoding, "windows-1251");
        assert!((s.volume - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn volume_steps_stay_in_range() {
        let mut game = GameSettings::default();
        game.step_volume(0.1);
        assert!((game.volume - 1.0).abs() < f32::EPSILON);
        for _ in 0..3 {
            game.step_volume(-0.1);
        }
        assert!((game.volume - 0.7).abs() < 1e-6);
        for _ in 0..20 {
            game.step_volume(-0.1);
        }
        assert!(game.volume.abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_profiles_fall_back_to_the_default() {
        assert_eq!(profile_by_id("nokia-s40-128x160").screen_width, 128);
        assert_eq!(profile_by_id("gone").id, fastemu_core::default_profile().id);
    }
}
