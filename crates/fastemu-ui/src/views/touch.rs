//! Fingers on the keypad and the game screens.
//!
//! GPUI turns a touch into a tap only when the finger lifts, and then
//! sends the mouse press and release together, so a phone key would
//! never stay held and a game would never see a drag. A keypad key or a
//! running game's screen therefore claims a touch that starts on it, and
//! follows that finger until it lifts.

use fastemu_core::Key;
use gpui::{
    Bounds, Context, DispatchPhase, IntoElement, Pixels, Point, Styled, TouchDragEvent, TouchPhase,
    WeakEntity, canvas,
};

use super::{FastEmuView, TileId};

/// Where a finger is in its touch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Finger {
    Down,
    Moved,
    Up,
}

impl Finger {
    fn of(phase: TouchPhase) -> Self {
        match phase {
            TouchPhase::Started => Finger::Down,
            TouchPhase::Moved => Finger::Moved,
            TouchPhase::Ended | TouchPhase::Cancelled => Finger::Up,
        }
    }
}

/// An invisible layer over an element that hands the touches starting in
/// it to `handle`, with the finger's position. `handle` returns whether
/// it takes a starting touch; a touch it does not take goes on to become
/// a tap or a scroll.
fn touch_layer(
    view: WeakEntity<FastEmuView>,
    handle: impl Fn(&mut FastEmuView, Finger, Point<Pixels>, &mut Context<'_, FastEmuView>) -> bool
    + Clone
    + 'static,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds: Bounds<Pixels>, (), window, _| {
            let view = view.clone();
            let handle = handle.clone();
            window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble || !bounds.contains(&event.start_position) {
                    return;
                }
                let finger = Finger::of(event.phase);
                let taken = view
                    .update(cx, |this, cx| handle(this, finger, event.position, cx))
                    .unwrap_or(false);
                if taken && finger == Finger::Down {
                    window.prevent_default();
                }
            });
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// Holds `key` on `tile`'s keypad while a finger is on its button.
pub(super) fn hold_key(view: WeakEntity<FastEmuView>, tile: TileId, key: Key) -> impl IntoElement {
    touch_layer(view, move |this, finger, _, cx| {
        match finger {
            Finger::Down => this.keypad_press(tile, key, cx),
            Finger::Moved => {}
            Finger::Up => this.keypad_release(tile, cx),
        }
        true
    })
}

/// Sends a finger on `tile`'s screen to the game as pointer events, while
/// the game runs. A finished or paused game's screen leaves touches to
/// the buttons over it.
pub(super) fn touch_screen(view: WeakEntity<FastEmuView>, tile: TileId) -> impl IntoElement {
    touch_layer(view, move |this, finger, position, cx| {
        this.screen_touch(tile, finger, position, cx)
    })
}
