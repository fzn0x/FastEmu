//! The event thread: one Java thread per instance that runs every UI
//! callback in turn.
//!
//! Each pass of [`run`] does, in order: the tasks posted to the VM (the
//! app's input arrives this way, and other modules may post work for the
//! event thread with `Ctx::post` or `Vm::post`), the queued input events,
//! a due key repeat, the runtime's own work (relayouts, going to the
//! background and back), a pending `setCurrent`, the pending paint (when
//! the frame rate allows), and the `callSerially` runnables that were
//! queued before the pass, stopping early when one of them asks for a
//! repaint, so the paint comes first. Then it sleeps until something new
//! arrives or a deadline (a key repeat, a deferred paint, a screen timer)
//! passes.
//!
//! Every callback runs inside [`callback`], which holds the callback lock
//! and logs an exception the game throws instead of ending the thread.

use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, JResult, ObjRef, Value, Wake};

use super::display::{self, notify_size};
use super::displayable::{self, Kind};
use super::input;
use super::screen::{KeyEvent, KeyPhase};
use super::state::{Internal, Rect, current, report, state};

/// How long a key is held before it first repeats.
pub(crate) const FIRST_REPEAT: Duration = Duration::from_millis(400);
/// How often a held key repeats after that.
pub(crate) const REPEAT_PERIOD: Duration = Duration::from_millis(120);

/// The body of the event thread. Runs until the VM stops.
pub fn run(ctx: &mut Ctx<'_>) -> JResult<()> {
    let me = ctx.thread_id();
    state(ctx).event_thread = Some(me);
    loop {
        ctx.local_frame(pass)?;
    }
}

fn pass(ctx: &mut Ctx<'_>) -> JResult<()> {
    ctx.run_posted()?;
    let inputs: Vec<_> = state(ctx).inputs.drain(..).collect();
    for event in inputs {
        input::dispatch(ctx, event)?;
    }
    repeat_key(ctx)?;
    let internal: Vec<_> = state(ctx).internal.drain(..).collect();
    for work in internal {
        match work {
            Internal::Relayout(global) => {
                let obj = global.get(ctx);
                relayout(ctx, obj)?;
            }
        }
    }
    apply_requested(ctx)?;
    tick_screen(ctx)?;
    paint_pending(ctx)?;
    run_serial(ctx)?;
    if ctx.wait_posted(Some(Instant::now()))? || ready(ctx) {
        return Ok(());
    }
    let deadline = next_deadline(ctx);
    match ctx.wait_until(deadline)? {
        Wake::Paused => ctx.yield_now()?,
        Wake::Interrupted => {
            ctx.take_interrupt();
        }
        Wake::Notified | Wake::TimedOut => {}
    }
    Ok(())
}

/// Whether the next pass has work it can do now.
fn ready(ctx: &mut Ctx<'_>) -> bool {
    let now = Instant::now();
    let st = state(ctx);
    let paint_due = st.dirty.is_some() && st.next_frame_at().is_none_or(|at| now >= at);
    !st.inputs.is_empty()
        || !st.internal.is_empty()
        || st.requested.is_some()
        || paint_due
        || (st.dirty.is_none() && !st.serial.is_empty())
        || st.repeat.is_some_and(|(_, at)| now >= at)
        || st.tick_at.is_some_and(|at| now >= at)
}

/// The earliest time the loop must wake without being notified.
fn next_deadline(ctx: &mut Ctx<'_>) -> Option<Instant> {
    let st = state(ctx);
    let paint = st.dirty.and(st.next_frame_at());
    [paint, st.repeat.map(|(_, at)| at), st.tick_at]
        .into_iter()
        .flatten()
        .min()
}

/// Takes the callback lock for the calling thread, waiting while another
/// thread holds it. With a timeout, returns `false` when it passes (or the
/// thread is interrupted) without the lock. The lock is reentrant, so a
/// callback may call `serviceRepaints`.
pub(crate) fn acquire(ctx: &mut Ctx<'_>, timeout: Option<Duration>) -> JResult<bool> {
    let me = ctx.thread_id();
    let deadline = timeout.map(|t| Instant::now() + t);
    loop {
        let lock = &mut state(ctx).callback;
        match lock.holder {
            None => {
                lock.holder = Some(me);
                lock.depth = 1;
                return Ok(true);
            }
            Some(holder) if holder == me => {
                lock.depth += 1;
                return Ok(true);
            }
            Some(_) => {}
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Ok(false);
        }
        lock.waiters += 1;
        let wake = ctx.wait_until(deadline);
        state(ctx).callback.waiters -= 1;
        match wake? {
            Wake::Paused => ctx.yield_now()?,
            Wake::Interrupted if timeout.is_some() => return Ok(false),
            Wake::Interrupted => {
                ctx.take_interrupt();
            }
            Wake::Notified | Wake::TimedOut => {}
        }
    }
}

/// Gives back what [`acquire`] took.
pub(crate) fn release(ctx: &mut Ctx<'_>) {
    let lock = &mut state(ctx).callback;
    lock.depth = lock.depth.saturating_sub(1);
    if lock.depth == 0 {
        lock.holder = None;
        if lock.waiters > 0 {
            ctx.notify_waiters();
        }
    }
}

/// Runs a UI callback: holds the callback lock around it and logs an
/// exception it throws as `Exception in <what>`, so the event thread
/// carries on with the next event.
pub fn callback(
    ctx: &mut Ctx<'_>,
    what: &str,
    f: impl FnOnce(&mut Ctx<'_>) -> JResult<()>,
) -> JResult<()> {
    acquire(ctx, None)?;
    let result = f(ctx);
    release(ctx);
    report(ctx, what, result)
}

/// Calls a no-argument callback method of a canvas or the matching hook
/// of a screen.
fn notify(ctx: &mut Ctx<'_>, obj: ObjRef, shown: bool) -> JResult<()> {
    match displayable::data(ctx, obj)?.kind() {
        Kind::Canvas => {
            let method = if shown { "showNotify" } else { "hideNotify" };
            callback(ctx, method, |ctx| {
                ctx.call_virtual(obj, method, "()V", &[]).map(drop)
            })
        }
        Kind::Screen => match displayable::handler(ctx, obj)? {
            Some(handler) if shown => {
                callback(ctx, "showing a screen", |ctx| handler.shown(ctx, obj))
            }
            Some(handler) => callback(ctx, "hiding a screen", |ctx| handler.hidden(ctx, obj)),
            None => Ok(()),
        },
    }
}

/// Forgets held keys, repeats and the menu, as when the screen changes.
fn reset_input(ctx: &mut Ctx<'_>) {
    let st = state(ctx);
    st.keys.reset();
    st.repeat = None;
    st.menu = None;
    st.pointer_down = false;
}

fn mark_all_dirty(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    display::request_paint(ctx, obj, Rect::new(0, 0, i32::MAX, i32::MAX))
}

/// Applies the latest `setCurrent`.
fn apply_requested(ctx: &mut Ctx<'_>) -> JResult<()> {
    let Some(global) = state(ctx).requested.take() else {
        return Ok(());
    };
    let next = global.get(ctx);
    let old = current(ctx);
    if old == Some(next) {
        return Ok(());
    }
    acquire(ctx, None)?;
    let result = notify_size(ctx, next);
    release(ctx);
    result?;
    if let Some(old) = old
        && !state(ctx).background
    {
        notify(ctx, old, false)?;
    }
    reset_input(ctx);
    let st = state(ctx);
    st.current = Some(global);
    st.dirty = None;
    st.tick_at = Some(Instant::now());
    if !state(ctx).background {
        notify(ctx, next, true)?;
        mark_all_dirty(ctx, next)?;
    }
    Ok(())
}

/// The shown displayable's size or soft keys may have changed.
fn relayout(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    if current(ctx) != Some(obj) {
        return Ok(());
    }
    acquire(ctx, None)?;
    let result = notify_size(ctx, obj);
    release(ctx);
    result?;
    mark_all_dirty(ctx, obj)
}

/// Moves the instance to the background (`false`), sending `hideNotify`
/// to the shown displayable and dropping held keys, or back to the
/// foreground with `showNotify` and a full repaint. Runs on the event
/// thread; the instance posts it when the app pauses and resumes.
pub fn set_visible(ctx: &mut Ctx<'_>, visible: bool) -> JResult<()> {
    let was_background = state(ctx).background;
    if was_background != visible {
        return Ok(());
    }
    let shown = current(ctx);
    if visible {
        state(ctx).background = false;
        if let Some(obj) = shown {
            notify(ctx, obj, true)?;
            mark_all_dirty(ctx, obj)?;
        }
    } else {
        if let Some(obj) = shown {
            notify(ctx, obj, false)?;
        }
        reset_input(ctx);
        let st = state(ctx);
        st.background = true;
        st.dirty = None;
    }
    Ok(())
}

/// Sends `keyRepeated` for the held key when its time comes.
fn repeat_key(ctx: &mut Ctx<'_>) -> JResult<()> {
    let now = Instant::now();
    let st = state(ctx);
    let Some((code, at)) = st.repeat else {
        return Ok(());
    };
    if now < at {
        return Ok(());
    }
    st.repeat = Some((code, now + REPEAT_PERIOD));
    let Some(obj) = current(ctx) else {
        return Ok(());
    };
    let codes = crate::env::env(ctx).profile.key_codes;
    let event = KeyEvent {
        code,
        game_action: super::keys::game_action(&codes, code),
        phase: KeyPhase::Repeated,
    };
    input::deliver_key(ctx, obj, event)
}

/// Gives a shown high-level screen its timer tick.
fn tick_screen(ctx: &mut Ctx<'_>) -> JResult<()> {
    let now = Instant::now();
    if state(ctx).tick_at.is_none_or(|at| now < at) {
        return Ok(());
    }
    state(ctx).tick_at = None;
    let Some(obj) = current(ctx) else {
        return Ok(());
    };
    if state(ctx).background {
        return Ok(());
    }
    let Some(handler) = displayable::handler(ctx, obj)? else {
        return Ok(());
    };
    acquire(ctx, None)?;
    let next = handler.tick(ctx, obj, now);
    release(ctx);
    match next {
        Ok(next) => state(ctx).tick_at = next,
        Err(e) => report(ctx, "a screen's timer", Err(e))?,
    }
    Ok(())
}

/// Paints the pending region when the frame rate allows.
fn paint_pending(ctx: &mut Ctx<'_>) -> JResult<()> {
    let st = state(ctx);
    if st.dirty.is_none() || st.next_frame_at().is_some_and(|at| Instant::now() < at) {
        return Ok(());
    }
    let Some(dirty) = st.dirty.take() else {
        return Ok(());
    };
    let Some(obj) = current(ctx) else {
        return Ok(());
    };
    acquire(ctx, None)?;
    let result = display::paint(ctx, obj, dirty);
    release(ctx);
    result
}

/// Runs the `callSerially` runnables queued before this pass.
fn run_serial(ctx: &mut Ctx<'_>) -> JResult<()> {
    let count = state(ctx).serial.len();
    for _ in 0..count {
        if state(ctx).dirty.is_some() {
            break;
        }
        let Some(global) = state(ctx).serial.pop_front() else {
            break;
        };
        let runnable = global.get(ctx);
        callback(ctx, "a callSerially runnable", |ctx| {
            ctx.call_interface(runnable, "run", "()V", &[]).map(drop)
        })?;
    }
    Ok(())
}

/// Calls `name(code)` on a canvas, as a callback.
pub(crate) fn call_int(ctx: &mut Ctx<'_>, obj: ObjRef, name: &str, args: &[Value]) -> JResult<()> {
    let desc = if args.len() == 1 { "(I)V" } else { "(II)V" };
    callback(ctx, name, |ctx| {
        ctx.call_virtual(obj, name, desc, args).map(drop)
    })
}
