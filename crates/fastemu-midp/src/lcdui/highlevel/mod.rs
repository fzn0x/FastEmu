//! The high-level screens and items of `javax.microedition.lcdui`: `Form`
//! and its items, `List`, `TextBox`, `Alert` and `Ticker`, built on the
//! runtime's [`ScreenHandler`](crate::lcdui::screen::ScreenHandler).
//!
//! Each screen draws its content area in the runtime's theme and fonts;
//! the runtime draws the title, the ticker line and the soft keys around
//! it. `docs/architecture/midp.md` describes how they behave.

pub mod alert;
pub mod choice;
pub mod draw;
pub mod form;
pub mod item;
pub mod list;
pub mod rows;
pub mod text;
pub mod textbox;
pub mod ticker;

use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef};

use crate::env::env;
use crate::lcdui::{displayable, frame, image, state};

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = vec![
        choice::interface(),
        ticker::class(),
        alert::class(),
        alert::type_class(),
        list::class(),
        textbox::class(),
        form::class(),
    ];
    classes.extend(item::classes());
    classes
}

/// Gives back the room of a vector that took in many items and lost them,
/// so that what the heap counts for it, its room, falls with its length.
/// A vector within twice its length (and a few items) keeps its room, so
/// deleting one item at a time does not reallocate each time.
pub(crate) fn trim<T>(items: &mut Vec<T>) {
    const SLACK: usize = 16;
    if items.capacity() > items.len().saturating_mul(2).saturating_add(SLACK) {
        items.shrink_to(items.len());
    }
}

/// The image a screen or item keeps for `image`: MIDP has it take a
/// snapshot of a mutable image, so later drawing on the image does not
/// show; an immutable image is kept itself.
pub(crate) fn snapshot(ctx: &mut Ctx<'_>, image: Option<ObjRef>) -> JResult<Option<ObjRef>> {
    let Some(image) = image else {
        return Ok(None);
    };
    let surface = image::surface_of(ctx, image)?;
    let Some(copy) = image::snapshot_of(ctx, &surface)? else {
        return Ok(Some(image));
    };
    Ok(Some(image::new_image(ctx, copy)?))
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
