//! The keys a bench presses in a real game, the same on both emulators.
//!
//! Scripts are written as `fastemu run --keys` and `cargo xtask compat`
//! take them ([`crate::compat::script`]). FastEmu's runner sends the keys
//! as [`Key`]s; MeBench gets a timeline of key codes, the codes FastEmu's
//! default profile gives each key, so both emulators see the same codes
//! at the same times after the game's first frame.

use std::time::Duration;

use fastemu_core::{Key, KeyCodes};

use crate::compat::script::{self, Step};

/// What a bench presses in each game: compat's default script, with the
/// fire key where compat presses 5. Its own text, so a change of compat's
/// keys never moves the bench's numbers.
///
/// FastEmu gives the digits 2, 4, 5, 6 and 8 the game actions of the
/// arrows and fire, but MicroEmulator gives them none, so a game that asks
/// `getGameAction` or `getKeyStates` would see 5 as fire on one emulator
/// only. The arrows, fire and the soft keys mean the same on both.
pub const BENCH_KEYS: &str = "wait:1500 fire wait:1000 up wait:300 down wait:300 left \
     wait:300 right wait:300 fire wait:1000 fire wait:1000 fire";

/// One press or release, `at_ms` after the game's first frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyStep {
    pub at_ms: u64,
    pub key: Key,
    pub press: bool,
}

/// The presses and releases of `script`, with their times.
pub fn steps(script: &str) -> Result<Vec<KeyStep>, String> {
    let mut at = Duration::ZERO;
    let mut out = Vec::new();
    for step in script::parse(script)? {
        match step {
            Step::Wait(d) => at += d,
            Step::Press(key) | Step::Release(key) => out.push(KeyStep {
                at_ms: at.as_millis() as u64,
                key,
                press: matches!(step, Step::Press(_)),
            }),
        }
    }
    Ok(out)
}

/// `script` as MeBench's `--events` takes it: `T:CODE:p|r`, separated by
/// commas, with the codes of `codes`.
pub fn timeline(script: &str, codes: &KeyCodes) -> Result<String, String> {
    let events: Vec<String> = steps(script)?
        .iter()
        .map(|s| {
            let what = if s.press { 'p' } else { 'r' };
            format!("{}:{}:{what}", s.at_ms, codes.code(s.key))
        })
        .collect();
    Ok(events.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastemu_core::default_profile;

    #[test]
    fn timeline_uses_fastemu_codes() {
        let codes = default_profile().key_codes;
        assert_eq!(
            timeline("wait:1500 5 up:300", &codes).unwrap(),
            "1500:53:p,1600:53:r,1600:-1:p,1900:-1:r"
        );
        // Every key reaches MicroEmulator with the code FastEmu sends.
        for key in Key::ALL {
            let name = match key {
                Key::Star => "star".to_owned(),
                Key::Pound => "pound".to_owned(),
                other => format!("{other:?}")
                    .to_ascii_lowercase()
                    .trim_start_matches("num")
                    .to_owned(),
            };
            let line = timeline(&name, &codes).unwrap();
            let code = KeyCodes::NOKIA.code(key);
            assert_eq!(line, format!("0:{code}:p,100:{code}:r"), "{key:?}");
        }
        assert!(timeline("jump", &codes).is_err());
        assert_eq!(timeline("", &codes).unwrap(), "");
    }

    #[test]
    fn bench_keys_have_the_same_game_actions() {
        let steps = steps(BENCH_KEYS).unwrap();
        assert!(!steps.is_empty());
        for step in steps {
            assert!(
                matches!(
                    step.key,
                    Key::Up
                        | Key::Down
                        | Key::Left
                        | Key::Right
                        | Key::Fire
                        | Key::SoftLeft
                        | Key::SoftRight
                ),
                "{:?} means something else on MicroEmulator",
                step.key
            );
        }
    }
}
