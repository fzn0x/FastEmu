//! Key codes, game actions, key names and the key state `GameCanvas`
//! reads.
//!
//! Game actions follow `docs/compat/notes/03-input.md`: the profile's
//! arrows and fire key, and also `2`, `4`, `6`, `8` and `5`, give `UP`,
//! `LEFT`, `RIGHT`, `DOWN` and `FIRE`; `1`, `3`, `7` and `9` give `GAME_A`
//! to `GAME_D`; every other code gives 0. The mapping never throws and
//! never changes while a game runs.

use fastemu_core::KeyCodes;
use fastemu_jvm::Ctx;

use super::state::state;

/// `Canvas.UP`.
pub const UP: i32 = 1;
/// `Canvas.DOWN`.
pub const DOWN: i32 = 6;
/// `Canvas.LEFT`.
pub const LEFT: i32 = 2;
/// `Canvas.RIGHT`.
pub const RIGHT: i32 = 5;
/// `Canvas.FIRE`.
pub const FIRE: i32 = 8;
/// `Canvas.GAME_A`.
pub const GAME_A: i32 = 9;
/// `Canvas.GAME_B`.
pub const GAME_B: i32 = 10;
/// `Canvas.GAME_C`.
pub const GAME_C: i32 = 11;
/// `Canvas.GAME_D`.
pub const GAME_D: i32 = 12;

/// The game action of `code` on a phone with `codes`, 0 for none.
pub fn game_action(codes: &KeyCodes, code: i32) -> i32 {
    if code == codes.up {
        return UP;
    }
    if code == codes.down {
        return DOWN;
    }
    if code == codes.left {
        return LEFT;
    }
    if code == codes.right {
        return RIGHT;
    }
    if code == codes.fire {
        return FIRE;
    }
    match u8::try_from(code).map(char::from) {
        Ok('2') => UP,
        Ok('8') => DOWN,
        Ok('4') => LEFT,
        Ok('6') => RIGHT,
        Ok('5') => FIRE,
        Ok('1') => GAME_A,
        Ok('3') => GAME_B,
        Ok('7') => GAME_C,
        Ok('9') => GAME_D,
        _ => 0,
    }
}

/// `Canvas.getKeyCode(action)`: the profile's arrow or fire code, or the
/// digit of `GAME_A` to `GAME_D`; `None` for a value that is no action.
pub fn key_code(codes: &KeyCodes, action: i32) -> Option<i32> {
    Some(match action {
        UP => codes.up,
        DOWN => codes.down,
        LEFT => codes.left,
        RIGHT => codes.right,
        FIRE => codes.fire,
        GAME_A => i32::from(b'1'),
        GAME_B => i32::from(b'3'),
        GAME_C => i32::from(b'7'),
        GAME_D => i32::from(b'9'),
        _ => return None,
    })
}

/// `Canvas.getKeyName(code)`: the names J2ME Loader gives, the character
/// for printable codes, and a made-up name for anything else, since games
/// print the name of every key they are sent and must not get an
/// exception for one.
pub fn key_name(codes: &KeyCodes, code: i32) -> String {
    let named = [
        (codes.up, "UP"),
        (codes.down, "DOWN"),
        (codes.left, "LEFT"),
        (codes.right, "RIGHT"),
        (codes.fire, "SELECT"),
        (codes.soft_left, "SOFT1"),
        (codes.soft_right, "SOFT2"),
        (codes.clear, "CLEAR"),
        (codes.send, "SEND"),
        (codes.end, "END"),
    ];
    if let Some((_, name)) = named.iter().find(|(c, _)| *c == code) {
        return (*name).to_owned();
    }
    match u32::try_from(code).ok().and_then(char::from_u32) {
        Some(c) if !c.is_control() && c != ' ' => c.to_string(),
        _ => format!("KEY {code}"),
    }
}

/// The keys held now and the game-action bits latched since the last read,
/// for `GameCanvas.getKeyStates`.
///
/// A press sets its action's bit in the latch; reading returns the bits of
/// held keys or latched ones and clears the latch, so a tap between two
/// reads is never lost. Showing a canvas resets both, and a key held while
/// the canvas appeared counts only after it is pressed again.
#[derive(Clone, Debug, Default)]
pub struct KeyState {
    held: Vec<(i32, i32)>,
    latched: i32,
}

impl KeyState {
    /// `code`, with game action `action`, went down.
    pub fn press(&mut self, code: i32, action: i32) {
        if !self.held.iter().any(|&(c, _)| c == code) {
            self.held.push((code, action));
        }
        if action != 0 {
            self.latched |= 1 << action;
        }
    }

    /// `code` came up.
    pub fn release(&mut self, code: i32) {
        self.held.retain(|&(c, _)| c != code);
    }

    /// Whether `code` is down.
    pub fn is_held(&self, code: i32) -> bool {
        self.held.iter().any(|&(c, _)| c == code)
    }

    /// The bits of the held keys' game actions.
    pub fn down_mask(&self) -> i32 {
        self.held
            .iter()
            .filter(|&&(_, action)| action != 0)
            .fold(0, |mask, &(_, action)| mask | 1 << action)
    }

    /// The held and latched bits; clears the latch.
    pub fn take(&mut self) -> i32 {
        let states = self.down_mask() | self.latched;
        self.latched = 0;
        states
    }

    /// Forgets every key, as when a canvas is shown.
    pub fn reset(&mut self) {
        self.held.clear();
        self.latched = 0;
    }
}

/// `GameCanvas.getKeyStates()` for the canvas on the screen: the
/// `*_PRESSED` bits of the keys held now or pressed since the last call.
/// The caller returns 0 itself when its canvas is not shown
/// ([`crate::lcdui::display::is_shown`]).
pub fn take_key_states(ctx: &mut Ctx<'_>) -> i32 {
    state(ctx).keys.take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_and_digits_map_to_actions() {
        let nokia = KeyCodes::NOKIA;
        assert_eq!(game_action(&nokia, -1), UP);
        assert_eq!(game_action(&nokia, -5), FIRE);
        assert_eq!(game_action(&nokia, '2' as i32), UP);
        assert_eq!(game_action(&nokia, '5' as i32), FIRE);
        assert_eq!(game_action(&nokia, '9' as i32), GAME_D);
        assert_eq!(game_action(&nokia, '0' as i32), 0);
        assert_eq!(game_action(&nokia, '*' as i32), 0);
        assert_eq!(game_action(&nokia, -6), 0, "soft keys have no action");
        assert_eq!(game_action(&nokia, 12345), 0);
        let motorola = KeyCodes::MOTOROLA;
        assert_eq!(game_action(&motorola, -6), DOWN);
        assert_eq!(game_action(&motorola, -20), FIRE);
    }

    #[test]
    fn key_codes_invert_actions() {
        let codes = KeyCodes::SIEMENS;
        for action in [UP, DOWN, LEFT, RIGHT, FIRE, GAME_A, GAME_B, GAME_C, GAME_D] {
            let code = key_code(&codes, action).unwrap();
            assert_eq!(game_action(&codes, code), action);
        }
        assert_eq!(key_code(&codes, 0), None);
        assert_eq!(key_code(&codes, 7), None);
    }

    #[test]
    fn every_code_has_a_name() {
        let codes = KeyCodes::NOKIA;
        assert_eq!(key_name(&codes, -5), "SELECT");
        assert_eq!(key_name(&codes, -6), "SOFT1");
        assert_eq!(key_name(&codes, '7' as i32), "7");
        assert_eq!(key_name(&codes, '#' as i32), "#");
        assert_eq!(key_name(&codes, -13), "KEY -13");
    }

    #[test]
    fn key_states_latch_until_read() {
        let mut keys = KeyState::default();
        keys.press(-1, UP);
        keys.release(-1);
        assert_eq!(keys.take(), 1 << UP, "a tap between reads is kept");
        assert_eq!(keys.take(), 0);
        keys.press(-5, FIRE);
        assert_eq!(keys.take(), 1 << FIRE);
        assert_eq!(keys.take(), 1 << FIRE, "a held key stays set");
        keys.press('5' as i32, FIRE);
        keys.release(-5);
        assert_eq!(keys.take(), 1 << FIRE, "the other fire key is still down");
        keys.reset();
        keys.release('5' as i32);
        assert_eq!(keys.take(), 0);
    }
}
