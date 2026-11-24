//! Routing the host keyboard: to the bindings editor while it waits for a
//! key, to the open dialog, or to the focused game.

use fastemu_core::InputEvent;
use gpui::{Context, KeyDownEvent, KeyUpEvent, Keystroke, Window};

use super::FastEmuView;

/// Whether a stroke carries a modifier the app keeps for its own
/// shortcuts. Shift is not one: it types `#` and `*` on many layouts.
fn is_shortcut(keystroke: &Keystroke) -> bool {
    let m = &keystroke.modifiers;
    m.control || m.alt || m.platform || m.function
}

/// The name a binding stores for a stroke: the typed symbol when the key
/// produces one (`#` for Shift+3), else GPUI's key name.
pub(super) fn host_key(keystroke: &Keystroke) -> String {
    match keystroke.key_char.as_deref() {
        Some(c)
            if c.chars().count() == 1 && c.chars().all(|c| !c.is_alphanumeric() && c != ' ') =>
        {
            c.to_owned()
        }
        _ => keystroke.key.clone(),
    }
}

/// The text a stroke types, if any: one or more printable characters.
fn typed_text(keystroke: &Keystroke) -> Option<&str> {
    keystroke
        .key_char
        .as_deref()
        .filter(|t| !t.is_empty() && !t.chars().any(char::is_control))
}

impl FastEmuView {
    pub(super) fn on_key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let keystroke = &event.keystroke;
        if let Some(key) = self.capturing {
            if !event.is_held {
                if keystroke.key == "escape" {
                    self.capturing = None;
                    cx.notify();
                } else if !is_shortcut(keystroke) {
                    let host = host_key(keystroke);
                    self.capturing = None;
                    self.update_settings(cx, |s| s.bindings.bind(key, &host));
                }
            }
            cx.stop_propagation();
            return;
        }
        if self.dialog.is_some() {
            if keystroke.key == "escape" {
                self.close_dialog(cx);
                cx.stop_propagation();
            }
            return;
        }
        if is_shortcut(keystroke) {
            return;
        }
        let bindings = self.settings.bindings.clone();
        let Some(tile) = self.focused_tile_mut() else {
            return;
        };
        let session = &mut tile.session;
        if event.is_held || !session.is_running() {
            cx.stop_propagation();
            return;
        }
        if session.typing_text()
            && let Some(text) = typed_text(keystroke)
        {
            session.instance.send(InputEvent::Text(text.to_owned()));
            cx.stop_propagation();
            return;
        }
        let Some(key) = bindings.resolve(&keystroke.key, keystroke.key_char.as_deref()) else {
            return;
        };
        if session.keys.press(&keystroke.key, key) {
            session.press(key);
        }
        cx.stop_propagation();
    }

    pub(super) fn on_key_up(
        &mut self,
        event: &KeyUpEvent,
        _: &mut Window,
        _: &mut Context<'_, Self>,
    ) {
        if self.capturing.is_some() {
            return;
        }
        let Some(tile) = self.focused_tile_mut() else {
            return;
        };
        for key in tile.session.keys.release(&event.keystroke.key) {
            tile.session.release(key);
        }
    }

    /// Lets go of every key the focused game holds.
    pub(super) fn release_focused_keys(&mut self) {
        if let Some(tile) = self.focused_tile_mut() {
            tile.session.release_everything();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(source: &'static str, key_char: Option<&str>) -> Keystroke {
        let mut keystroke = Keystroke::parse(source).expect("parses");
        keystroke.key_char = key_char.map(str::to_owned);
        keystroke
    }

    #[test]
    fn symbols_bind_by_the_character_they_type() {
        assert_eq!(host_key(&stroke("3", Some("#"))), "#");
        assert_eq!(host_key(&stroke("3", Some("3"))), "3");
        assert_eq!(host_key(&stroke("space", Some(" "))), "space");
        assert_eq!(host_key(&stroke("f1", None)), "f1");
    }

    #[test]
    fn shortcuts_and_control_keys_are_not_text() {
        assert!(is_shortcut(&stroke("ctrl-o", None)));
        assert!(!is_shortcut(&stroke("shift-3", Some("#"))));
        assert_eq!(typed_text(&stroke("a", Some("a"))), Some("a"));
        assert_eq!(typed_text(&stroke("enter", Some("\r"))), None);
        assert_eq!(typed_text(&stroke("up", None)), None);
    }
}
