//! The high-level screens and items of `javax.microedition.lcdui`: Form
//! and its items, List, TextBox, Alert and Ticker, built on the runtime's
//! [`ScreenHandler`](crate::lcdui::screen::ScreenHandler).

pub mod choice;
pub mod draw;
pub mod list;
pub mod rows;
pub mod text;
pub mod textbox;
pub mod ticker;

use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef};

use crate::env::env;
use crate::lcdui::{displayable, frame, state};

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    vec![
        choice::interface(),
        ticker::class(),
        list::class(),
        textbox::class(),
    ]
}

/// Tells the app whether `screen` wants typed text rather than keypad
/// keys, if it is the screen on the display.
pub(crate) fn set_wants_text(ctx: &mut Ctx<'_>, screen: ObjRef, wants: bool) {
    if state::current(ctx) == Some(screen) {
        env(ctx).host.set_wants_text(wants);
    }
}

/// Asks for the shown screen's timer at once, after something changed
/// that it times: a multi-tap letter, an alert's timeout, a gauge.
pub(crate) fn wake_timer(ctx: &mut Ctx<'_>, screen: ObjRef) {
    if state::current(ctx) == Some(screen) {
        state::state(ctx).tick_at = Some(Instant::now());
        ctx.notify_waiters();
    }
}

/// The ticker's part of a screen's timer: while the screen has a ticker,
/// presents a frame every [`ticker::STEP`] so the text moves.
pub(crate) fn ticker_tick(
    ctx: &mut Ctx<'_>,
    screen: ObjRef,
    now: Instant,
) -> JResult<Option<Instant>> {
    if displayable::data(ctx, screen)?.ticker().is_none() {
        return Ok(None);
    }
    frame::present(ctx)?;
    Ok(Some(now + ticker::STEP))
}
