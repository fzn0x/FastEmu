//! The high-level screens and items of `javax.microedition.lcdui`: Form
//! and its items, List, TextBox, Alert and Ticker, built on the runtime's
//! [`ScreenHandler`](crate::lcdui::screen::ScreenHandler).

pub mod choice;
pub mod draw;
pub mod list;
pub mod rows;
pub mod text;
pub mod ticker;

use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef};

use crate::lcdui::{displayable, frame};

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    vec![choice::interface(), ticker::class(), list::class()]
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
