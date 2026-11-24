//! Input from the app: keys, pointer events and text, delivered to the
//! shown displayable on the event thread.
//!
//! [`queue`] runs in a task the instance posts to the VM; it merges a drag
//! into a drag already waiting, so a slow game does not fall behind.
//! Presses and releases are never merged or dropped. Repeats are made
//! here, not by the app: the first after 400 ms, then every 120 ms, to
//! `keyRepeated` only, and never for a key the soft keys took.

use std::time::Instant;

use fastemu_core::{InputEvent, KeyAction, PointerAction};
use fastemu_jvm::{Ctx, JResult, ObjRef, Value};

use super::displayable::{self, Kind};
use super::event::{FIRST_REPEAT, call_int, callback};
use super::keys;
use super::screen::{KeyEvent, KeyPhase};
use super::softkeys;
use super::state::{current, state};
use crate::env::env;

/// Queues an input event for the event thread. Call it on the event
/// thread, from a posted task.
pub fn queue(ctx: &mut Ctx<'_>, event: InputEvent) {
    let st = state(ctx);
    let is_drag = |e: &InputEvent| {
        matches!(
            e,
            InputEvent::Pointer {
                action: PointerAction::Drag,
                ..
            }
        )
    };
    if is_drag(&event) && st.inputs.back().is_some_and(is_drag) {
        st.inputs.pop_back();
    }
    st.inputs.push_back(event);
    ctx.notify_waiters();
}

/// Delivers one queued event.
pub(crate) fn dispatch(ctx: &mut Ctx<'_>, event: InputEvent) -> JResult<()> {
    if state(ctx).background {
        return Ok(());
    }
    match event {
        InputEvent::Key { key, action } => {
            let code = env(ctx).profile.key_codes.code(key);
            key_event(ctx, code, action)
        }
        InputEvent::RawKey { code, action } => key_event(ctx, code, action),
        InputEvent::Pointer { action, x, y } => pointer_event(ctx, action, x, y),
        InputEvent::Text(text) => text_event(ctx, &text),
    }
}

fn key_event(ctx: &mut Ctx<'_>, code: i32, action: KeyAction) -> JResult<()> {
    let codes = env(ctx).profile.key_codes;
    let game_action = keys::game_action(&codes, code);
    match action {
        KeyAction::Press => {
            if state(ctx).menu.is_some() {
                state(ctx).swallowed.push(code);
                return softkeys::menu_key(ctx, code);
            }
            let Some(obj) = current(ctx) else {
                return Ok(());
            };
            if softkeys::press(ctx, obj, code)? {
                state(ctx).swallowed.push(code);
                return Ok(());
            }
            let st = state(ctx);
            st.keys.press(code, game_action);
            st.repeat = Some((code, Instant::now() + FIRST_REPEAT));
            let event = KeyEvent {
                code,
                game_action,
                phase: KeyPhase::Pressed,
            };
            deliver_key(ctx, obj, event)
        }
        KeyAction::Release => {
            let st = state(ctx);
            if let Some(i) = st.swallowed.iter().position(|&c| c == code) {
                st.swallowed.swap_remove(i);
                return Ok(());
            }
            if !st.keys.is_held(code) {
                return Ok(());
            }
            st.keys.release(code);
            if st.repeat.is_some_and(|(c, _)| c == code) {
                st.repeat = None;
            }
            let Some(obj) = current(ctx) else {
                return Ok(());
            };
            let event = KeyEvent {
                code,
                game_action,
                phase: KeyPhase::Released,
            };
            deliver_key(ctx, obj, event)
        }
    }
}

/// Sends a key event to a canvas's key callbacks or a screen's handler.
/// A canvas that suppresses game keys (`GameCanvas(true)`) gets no
/// callbacks for keys with a game action.
pub(crate) fn deliver_key(ctx: &mut Ctx<'_>, obj: ObjRef, event: KeyEvent) -> JResult<()> {
    let d = displayable::data(ctx, obj)?;
    match d.kind() {
        Kind::Canvas => {
            if d.suppresses_game_keys() && event.game_action != 0 {
                return Ok(());
            }
            let method = match event.phase {
                KeyPhase::Pressed => "keyPressed",
                KeyPhase::Repeated => "keyRepeated",
                KeyPhase::Released => "keyReleased",
            };
            call_int(ctx, obj, method, &[Value::Int(event.code)])
        }
        Kind::Screen => match d.handler() {
            Some(handler) => callback(ctx, "a screen's key handling", |ctx| {
                handler.key(ctx, obj, event).map(drop)
            }),
            None => Ok(()),
        },
    }
}

fn pointer_event(ctx: &mut Ctx<'_>, action: PointerAction, x: i32, y: i32) -> JResult<()> {
    if state(ctx).menu.is_some() {
        return softkeys::menu_pointer(ctx, action, x, y);
    }
    let Some(obj) = current(ctx) else {
        return Ok(());
    };
    let layout = displayable::layout(ctx, obj)?;
    if action == PointerAction::Press
        && let Some(code) = softkeys::bar_key(ctx, &layout, x, y)
    {
        key_event(ctx, code, KeyAction::Press)?;
        return key_event(ctx, code, KeyAction::Release);
    }
    let content = layout.content;
    let (cx, cy) = (x - content.x, y - content.y);
    let d = displayable::data(ctx, obj)?;
    let (kind, handler) = (d.kind(), d.handler());
    match action {
        PointerAction::Press => {
            if !content.contains(x, y) {
                return Ok(());
            }
            if kind == Kind::Canvas && !env(ctx).profile.touch {
                return Ok(());
            }
            let st = state(ctx);
            st.pointer_down = true;
            st.last_pointer = (cx, cy);
        }
        PointerAction::Drag => {
            let st = state(ctx);
            if !st.pointer_down || st.last_pointer == (cx, cy) {
                return Ok(());
            }
            st.last_pointer = (cx, cy);
        }
        PointerAction::Release => {
            let st = state(ctx);
            if !st.pointer_down {
                return Ok(());
            }
            st.pointer_down = false;
        }
    }
    match (kind, handler) {
        (Kind::Canvas, _) => {
            let method = match action {
                PointerAction::Press => "pointerPressed",
                PointerAction::Drag => "pointerDragged",
                PointerAction::Release => "pointerReleased",
            };
            call_int(ctx, obj, method, &[Value::Int(cx), Value::Int(cy)])
        }
        (Kind::Screen, Some(handler)) => callback(ctx, "a screen's pointer handling", |ctx| {
            handler.pointer(ctx, obj, action, cx, cy).map(drop)
        }),
        (Kind::Screen, None) => Ok(()),
    }
}

fn text_event(ctx: &mut Ctx<'_>, text: &str) -> JResult<()> {
    let Some(obj) = current(ctx) else {
        return Ok(());
    };
    match displayable::handler(ctx, obj)? {
        Some(handler) => callback(ctx, "a screen's text input", |ctx| {
            handler.text(ctx, obj, text).map(drop)
        }),
        None => Ok(()),
    }
}
