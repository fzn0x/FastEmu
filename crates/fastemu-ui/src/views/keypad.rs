//! The on-screen phone keypad: soft keys, a D-pad with fire, and the
//! twelve number keys. Each button sends one press while held down and
//! one release when let go, wherever the pointer is let go.

use fastemu_core::Key;
use gpui::{
    Context, Div, IntoElement, MouseButton, ParentElement, Stateful, Styled, div, prelude::*, px,
};

use super::widgets::icon;
use super::{FastEmuView, Tile, TileId};
use crate::assets::Icon;
use crate::layout::{CLUSTER_GAP, KEY_GAP, KEY_HEIGHT, KEY_WIDTH, KeypadPlacement};
use crate::theme::Theme;

/// The D-pad cluster, row by row; `None` leaves a gap.
const NAV_ROWS: [[Option<Key>; 3]; 3] = [
    [Some(Key::SoftLeft), Some(Key::Up), Some(Key::SoftRight)],
    [Some(Key::Left), Some(Key::Fire), Some(Key::Right)],
    [Some(Key::Clear), Some(Key::Down), None],
];

/// The number pad, row by row.
const NUM_ROWS: [[Key; 3]; 4] = [
    [Key::Num1, Key::Num2, Key::Num3],
    [Key::Num4, Key::Num5, Key::Num6],
    [Key::Num7, Key::Num8, Key::Num9],
    [Key::Star, Key::Num0, Key::Pound],
];

/// The letters printed under a number key, as on phones of the time.
fn letters(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::Num2 => "abc",
        Key::Num3 => "def",
        Key::Num4 => "ghi",
        Key::Num5 => "jkl",
        Key::Num6 => "mno",
        Key::Num7 => "pqrs",
        Key::Num8 => "tuv",
        Key::Num9 => "wxyz",
        _ => return None,
    })
}

impl FastEmuView {
    pub(super) fn keypad_press(&mut self, id: TileId, key: Key, cx: &mut Context<'_, Self>) {
        self.focus_tile(id, cx);
        let Some(tile) = self.tiles.iter_mut().find(|t| t.id == id) else {
            return;
        };
        if !tile.session.is_running() {
            return;
        }
        let (released, press) = tile.session.keys.press_keypad(key);
        for old in released {
            tile.session.release(old);
        }
        if press {
            tile.session.press(key);
        }
        cx.notify();
    }

    pub(super) fn keypad_release(&mut self, id: TileId, cx: &mut Context<'_, Self>) {
        let Some(tile) = self.tiles.iter_mut().find(|t| t.id == id) else {
            return;
        };
        if tile.session.keys.keypad_key().is_some() {
            for key in tile.session.keys.release_keypad() {
                tile.session.release(key);
            }
            cx.notify();
        }
    }

    pub(super) fn render_keypad(
        tile: &Tile,
        placement: KeypadPlacement,
        cx: &mut Context<'_, Self>,
    ) -> Div {
        let held = tile.session.keys.keypad_key();
        let nav = div()
            .flex()
            .flex_col()
            .gap(px(KEY_GAP))
            .children(NAV_ROWS.iter().map(|row| {
                div()
                    .flex()
                    .gap(px(KEY_GAP))
                    .children(row.iter().map(|key| match key {
                        Some(key) => Self::render_key(tile.id, *key, held, cx).into_any_element(),
                        None => div().w(px(KEY_WIDTH)).h(px(KEY_HEIGHT)).into_any_element(),
                    }))
            }));
        let num = div()
            .flex()
            .flex_col()
            .gap(px(KEY_GAP))
            .children(NUM_ROWS.iter().map(|row| {
                div().flex().gap(px(KEY_GAP)).children(
                    row.iter()
                        .map(|key| Self::render_key(tile.id, *key, held, cx)),
                )
            }));
        let pad = div()
            .id("keypad")
            .flex()
            .flex_none()
            .gap(px(CLUSTER_GAP))
            .child(nav)
            .child(num);
        let pad = match placement {
            KeypadPlacement::Below => pad.items_start(),
            KeypadPlacement::Beside => pad.flex_col(),
        };
        div().flex_none().child(pad)
    }

    fn render_key(
        tile: TileId,
        key: Key,
        held: Option<Key>,
        cx: &mut Context<'_, Self>,
    ) -> Stateful<Div> {
        let theme = Theme::get(cx).clone();
        let down = held == Some(key);
        let (bg, fg) = if down {
            (theme.accent, theme.on_accent)
        } else {
            (theme.raised, theme.text)
        };
        let face = match key {
            Key::Up => icon(Icon::ChevronUp, 16.0, fg).into_any_element(),
            Key::Down => icon(Icon::ChevronDown, 16.0, fg).into_any_element(),
            Key::Left => icon(Icon::ChevronLeft, 16.0, fg).into_any_element(),
            Key::Right => icon(Icon::ChevronRight, 16.0, fg).into_any_element(),
            Key::Fire => div().text_xs().child("OK").into_any_element(),
            Key::SoftLeft | Key::SoftRight => div()
                .w(px(16.0))
                .h(px(3.0))
                .rounded(px(2.0))
                .bg(fg)
                .into_any_element(),
            Key::Clear => div().text_xs().child("C").into_any_element(),
            other => div()
                .flex()
                .items_end()
                .gap(px(3.0))
                .child(div().child(other.label()))
                .children(letters(other).map(|l| div().text_xs().text_color(theme.muted).child(l)))
                .into_any_element(),
        };
        let hover = theme.hover;
        div()
            .id(("key", key as u64))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(KEY_WIDTH))
            .h(px(KEY_HEIGHT))
            .rounded(px(6.0))
            .bg(bg)
            .text_color(fg)
            .cursor_pointer()
            .when(!down, |d| d.hover(move |s| s.bg(hover)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.keypad_press(tile, key, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.keypad_release(tile, cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    if this
                        .tiles
                        .iter()
                        .any(|t| t.id == tile && t.session.keys.keypad_key() == Some(key))
                    {
                        this.keypad_release(tile, cx);
                    }
                }),
            )
            .child(face)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keypad_has_every_documented_key_once() {
        let mut keys: Vec<Key> = NAV_ROWS.iter().flatten().flatten().copied().collect();
        keys.extend(NUM_ROWS.iter().flatten().copied());
        let count = keys.len();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), count, "a key appears twice");
        for key in [
            Key::SoftLeft,
            Key::SoftRight,
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::Fire,
            Key::Star,
            Key::Pound,
        ] {
            assert!(keys.contains(&key), "{key:?} is missing");
        }
        assert!(Key::ALL[..10].iter().all(|k| keys.contains(k)));
    }
}
