//! `javax.microedition.lcdui.Form`: items laid out in rows
//! ([`layout`]), scrolled, with one of them holding the focus.
//!
//! Up and down move the focus to the next item that takes it, after the
//! focused item has used them itself (a choice group moves between its
//! elements, a `CustomItem` traverses inside); when the next such item is
//! further than a step away, they scroll instead, so long text can be
//! read. The focused item's commands join the form's on the soft keys and
//! go to its `ItemCommandListener`. A change the user makes reaches the
//! form's `ItemStateListener`.

pub mod layout;

use std::time::Instant;

use fastemu_core::PointerAction;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value};

use super::draw::{Fonts, Painter, SCROLLBAR};
use super::item::{self, Handled, Tag, choice_group, custom, gauge, text_field};
use super::{set_wants_text, ticker_tick, wake_timer};
use crate::lcdui::Rect;
use crate::lcdui::display;
use crate::lcdui::displayable::{self, DisplayableData};
use crate::lcdui::event::callback;
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase, ScreenHandler, ScreenState, ScreenTarget};
use crate::lcdui::state::state;
use layout::{Want, arrange};

const CLASS: &str = "javax/microedition/lcdui/Form";
const ITEM_DESC: &str = "Ljavax/microedition/lcdui/Item;";

/// The space between the content's edges and the items.
const MARGIN: i32 = 2;
/// The space between rows.
const GAP: i32 = 2;
/// A drag shorter than this is still a tap.
const TAP_SLOP: i32 = 6;

/// The width items are laid out in on a screen `screen_width` wide.
pub(crate) fn content_width(screen_width: i32) -> i32 {
    (screen_width - 2 * MARGIN - SCROLLBAR).max(1)
}

/// What a `Form` keeps.
#[derive(Default)]
pub struct FormState {
    pub items: Vec<ObjRef>,
    /// The `ItemStateListener`.
    pub listener: Option<ObjRef>,
    /// The item with the focus.
    pub focus: Option<ObjRef>,
    scroll: i32,
    /// The items as last laid out, with their boxes in form coordinates.
    laid: Vec<(ObjRef, Rect)>,
    height: i32,
    /// The content size the layout was made for.
    laid_for: Option<(i32, i32)>,
    stale: bool,
    /// Whether the next layout scrolls the focused item into view.
    reveal: bool,
    /// The focused item's commands, put on the soft keys.
    added: Vec<ObjRef>,
    /// Where an open popup's list was drawn, in content coordinates.
    popup: Option<Rect>,
    /// The item a pointer press landed on and where the press was.
    pressed: Option<(ObjRef, i32, i32)>,
    /// Whether the pointer moved far enough to scroll.
    dragging: bool,
}

impl ScreenState for FormState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &i in &self.items {
            tracer.mark(i);
        }
        for &(i, _) in &self.laid {
            tracer.mark(i);
        }
        tracer.mark_opt(self.listener);
        tracer.mark_opt(self.focus);
        for &c in &self.added {
            tracer.mark(c);
        }
        tracer.mark_opt(self.pressed.map(|(i, _, _)| i));
    }
}

fn form<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> JResult<&'a FormState> {
    match displayable::data(ctx, obj)?.state::<FormState>() {
        Some(s) => Ok(s),
        None => unreachable!("a Form's payload is made by its class"),
    }
}

fn form_mut<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut FormState> {
    match displayable::data_mut(ctx, obj)?.state_mut::<FormState>() {
        Some(s) => Ok(s),
        None => unreachable!("a Form's payload is made by its class"),
    }
}

/// An item of the form changed: lay it out again and repaint.
pub(crate) fn item_changed(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    form_mut(ctx, obj)?.stale = true;
    wake_timer(ctx, obj);
    display::repaint(ctx, obj)
}

/// An item's commands changed: the soft keys follow if it has the focus.
pub(crate) fn commands_changed(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    form_mut(ctx, obj)?.stale = true;
    sync_commands(ctx, obj)
}

/// Tells the form's `ItemStateListener` that the user changed `item`.
pub(crate) fn notify_state(ctx: &mut Ctx<'_>, obj: ObjRef, item: ObjRef) -> JResult<()> {
    let Some(listener) = form(ctx, obj)?.listener else {
        return Ok(());
    };
    let listener = ctx.root(listener);
    callback(ctx, "itemStateChanged", |ctx| {
        ctx.call_interface(
            listener,
            "itemStateChanged",
            "(Ljavax/microedition/lcdui/Item;)V",
            &[item.into()],
        )
        .map(drop)
    })
}

/// The content area's size.
fn view(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<(i32, i32)> {
    let content = displayable::layout(ctx, obj)?.content;
    Ok((content.width, content.height))
}

/// The fonts items are drawn and measured in.
fn fonts(ctx: &mut Ctx<'_>) -> Fonts {
    Fonts::new(&state(ctx).fonts)
}

/// Lays the items out if anything changed since the last time, and
/// scrolls the focused item into view when asked to.
fn ensure_layout(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let size = view(ctx, obj)?;
    let f = form(ctx, obj)?;
    if f.stale || f.laid_for != Some(size) {
        let items: Vec<ObjRef> = f.items.clone();
        let items: Vec<ObjRef> = items.into_iter().map(|i| ctx.root(i)).collect();
        let fonts = fonts(ctx);
        let width = (size.0 - 2 * MARGIN - SCROLLBAR).max(1);
        let mut wants = Vec::with_capacity(items.len());
        for &i in &items {
            let needs = item::needs(ctx, i, &fonts, width)?;
            wants.push(Want {
                layout: item::effective_layout(ctx, i)?,
                min_width: needs.min_width,
                width: needs.width,
            });
        }
        let (boxes, height) = arrange(&wants, width, GAP, |k, w| {
            item::height_at(ctx, items[k], &fonts, w)
        })?;
        let f = form_mut(ctx, obj)?;
        f.laid = items
            .into_iter()
            .zip(boxes)
            .map(|(i, r)| (i, Rect::new(r.x + MARGIN, r.y + MARGIN, r.width, r.height)))
            .collect();
        f.height = height + 2 * MARGIN;
        f.laid_for = Some(size);
        f.stale = false;
    }
    let f = form_mut(ctx, obj)?;
    if std::mem::take(&mut f.reveal)
        && let Some(r) = f.focus.and_then(|i| boxed(f, i))
    {
        if r.y < f.scroll || r.height > size.1 {
            f.scroll = r.y - MARGIN;
        } else if r.y + r.height > f.scroll + size.1 {
            f.scroll = r.y + r.height + MARGIN - size.1;
        }
    }
    f.scroll = f.scroll.clamp(0, (f.height - size.1).max(0));
    Ok(())
}

/// Where `item` was laid out.
fn boxed(f: &FormState, item: ObjRef) -> Option<Rect> {
    f.laid.iter().find(|(i, _)| *i == item).map(|&(_, r)| r)
}

/// How far one press of up or down scrolls.
fn step(ctx: &mut Ctx<'_>) -> i32 {
    fonts(ctx).text.height() * 2
}

/// Moves the focus to `next`, telling the items it leaves and enters.
fn move_focus(ctx: &mut Ctx<'_>, obj: ObjRef, next: Option<ObjRef>, dir: i32) -> JResult<()> {
    let old = form(ctx, obj)?.focus;
    if old == next {
        return Ok(());
    }
    if let Some(old) = old {
        let old = ctx.root(old);
        match item::tag(ctx, old)? {
            Tag::Custom => custom::traverse_out(ctx, old)?,
            Tag::Choice => choice_group::close(ctx, old)?,
            _ => {}
        }
    }
    let f = form_mut(ctx, obj)?;
    f.focus = next;
    f.reveal = true;
    f.popup = None;
    if let Some(next) = next {
        match item::tag(ctx, next)? {
            Tag::Choice => choice_group::enter(ctx, next, dir == keys::UP)?,
            Tag::Custom => enter_custom(ctx, obj, next, dir)?,
            _ => {}
        }
    }
    sync_commands(ctx, obj)?;
    update_text(ctx, obj)?;
    display::repaint(ctx, obj)
}

/// Gives a `CustomItem` the traversal that brought the focus to it.
fn enter_custom(ctx: &mut Ctx<'_>, obj: ObjRef, item: ObjRef, dir: i32) -> JResult<()> {
    ensure_layout(ctx, obj)?;
    let size = view(ctx, obj)?;
    let visible = visible_part(ctx, obj, item)?;
    custom::traverse(ctx, item, dir, size, visible)?;
    Ok(())
}

/// The part of `item`'s content in view, in content coordinates.
fn visible_part(ctx: &mut Ctx<'_>, obj: ObjRef, item: ObjRef) -> JResult<Rect> {
    let size = view(ctx, obj)?;
    let fonts = fonts(ctx);
    let f = form(ctx, obj)?;
    let scroll = f.scroll;
    let Some(r) = boxed(f, item) else {
        return Ok(Rect::new(0, 0, 0, 0));
    };
    let content = item::content_area(ctx, item, &fonts, r)?;
    let shown = content.intersect(Rect::new(0, scroll, size.0, size.1));
    Ok(Rect::new(
        shown.x - content.x,
        shown.y - content.y,
        shown.width,
        shown.height,
    ))
}

/// Puts the focused item's commands on the soft keys, in place of the
/// ones the item before it put there.
fn sync_commands(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let wanted = match form(ctx, obj)?.focus {
        Some(item) => item::commands(ctx, item)?,
        None => Vec::new(),
    };
    let old = std::mem::take(&mut form_mut(ctx, obj)?.added);
    let d = displayable::data_mut(ctx, obj)?;
    for &c in &old {
        d.remove_command(c);
    }
    let mut added = Vec::new();
    for &c in &wanted {
        if !d.commands().contains(&c) {
            d.add_command(c);
            added.push(c);
        }
    }
    let changed = old != added;
    form_mut(ctx, obj)?.added = added;
    if changed {
        display::relayout(ctx, obj);
    }
    Ok(())
}

/// Tells the app whether the focused item takes typed text.
fn update_text(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let wants = match form(ctx, obj)?.focus {
        Some(item) => item::takes_text(ctx, item)?,
        None => false,
    };
    set_wants_text(ctx, obj, wants);
    Ok(())
}

/// Up (`keys::UP`) or down from the focused item.
fn traverse(ctx: &mut Ctx<'_>, obj: ObjRef, dir: i32) -> JResult<bool> {
    ensure_layout(ctx, obj)?;
    let (_, view_h) = view(ctx, obj)?;
    let step = step(ctx);
    let down = dir == keys::DOWN;
    let f = form(ctx, obj)?;
    let (focus, scroll, height) = (f.focus, f.scroll, f.height);
    let laid = f.laid.clone();
    if let Some(item) = focus
        && item::tag(ctx, item)? == Tag::Custom
        && custom::internal(ctx, item)?
    {
        let size = view(ctx, obj)?;
        let visible = visible_part(ctx, obj, item)?;
        if custom::traverse(ctx, item, dir, size, visible)? {
            display::repaint(ctx, obj)?;
            return Ok(true);
        }
    }
    let at = focus.and_then(|i| laid.iter().position(|(x, _)| *x == i));
    // A focused item taller than the view scrolls through first.
    if let Some(r) = at.map(|k| laid[k].1) {
        let hidden = if down {
            r.y + r.height > scroll + view_h
        } else {
            r.y < scroll
        };
        if hidden {
            return scroll_by(ctx, obj, if down { step } else { -step });
        }
    }
    let order: Vec<usize> = match (at, down) {
        (Some(k), true) => (k + 1..laid.len()).collect(),
        (Some(k), false) => (0..k).rev().collect(),
        (None, true) => (0..laid.len()).collect(),
        (None, false) => (0..laid.len()).rev().collect(),
    };
    for k in order {
        let (item, r) = laid[k];
        if !item::focusable(ctx, item)? {
            continue;
        }
        let near = if down {
            r.y <= scroll + view_h + step
        } else {
            r.y + r.height >= scroll - step
        };
        if !near {
            break;
        }
        move_focus(ctx, obj, Some(item), dir)?;
        return Ok(true);
    }
    let can_scroll = if down {
        scroll + view_h < height
    } else {
        scroll > 0
    };
    if can_scroll {
        return scroll_by(ctx, obj, if down { step } else { -step });
    }
    Ok(false)
}

fn scroll_by(ctx: &mut Ctx<'_>, obj: ObjRef, delta: i32) -> JResult<bool> {
    let (_, view_h) = view(ctx, obj)?;
    let f = form_mut(ctx, obj)?;
    let before = f.scroll;
    f.scroll = (f.scroll + delta).clamp(0, (f.height - view_h).max(0));
    let moved = f.scroll != before;
    if moved {
        display::repaint(ctx, obj)?;
    }
    Ok(moved)
}

/// After the focused item used an event: lay out again (a text field may
/// have grown), keep it in view, and tell the listener of a change.
fn after(ctx: &mut Ctx<'_>, obj: ObjRef, item: ObjRef, handled: Handled) -> JResult<bool> {
    if handled == Handled::No {
        return Ok(false);
    }
    let f = form_mut(ctx, obj)?;
    f.stale = true;
    f.reveal = true;
    wake_timer(ctx, obj);
    display::repaint(ctx, obj)?;
    if handled == Handled::Changed {
        notify_state(ctx, obj, item)?;
    }
    Ok(true)
}

/// The focus the form starts with: the first item that takes it, if it
/// is on the first screen.
fn first_focus(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Option<ObjRef>> {
    ensure_layout(ctx, obj)?;
    let (_, view_h) = view(ctx, obj)?;
    let laid = form(ctx, obj)?.laid.clone();
    for (item, r) in laid {
        if item::focusable(ctx, item)? {
            return Ok((r.y < view_h).then_some(item));
        }
    }
    Ok(None)
}

/// `Display.setCurrentItem(item)`: shows the item's form with the focus
/// on it. The focus moves on the event thread, since moving it calls
/// the items' `traverseOut` and `traverse`, which MIDP serializes with
/// every other callback; the game may call this from any thread.
pub(crate) fn set_current_item(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    let d = item::data(ctx, item)?;
    let (owner, in_alert) = (d.owner, d.in_alert);
    let Some(owner) = owner else {
        return ctx.throw(
            "java/lang/IllegalStateException",
            "the item is not in a Form",
        );
    };
    if in_alert {
        return ctx.throw(
            "java/lang/IllegalStateException",
            "the item belongs to an Alert",
        );
    }
    if item::focusable(ctx, item)? {
        let (form, item) = (ctx.new_global(owner), ctx.new_global(item));
        ctx.post(move |ctx| {
            let (form, item) = (form.get(ctx), item.get(ctx));
            // The game may have moved the item since.
            if item::data(ctx, item)?.owner != Some(form) {
                return Ok(());
            }
            callback(ctx, "setCurrentItem", |ctx| {
                move_focus(ctx, form, Some(item), 0)
            })
        });
    } else {
        let f = form_mut(ctx, owner)?;
        f.reveal = true;
        if let Some(r) = boxed(f, item) {
            f.scroll = r.y - MARGIN;
        }
    }
    display::set_current(ctx, owner);
    Ok(())
}

/// The form's screen behaviour.
struct FormHandler;

static FORM: FormHandler = FormHandler;

impl ScreenHandler for FormHandler {
    fn prepare(&self, ctx: &mut Ctx<'_>, obj: ObjRef, _: i32, height: i32) -> JResult<()> {
        ensure_layout(ctx, obj)?;
        let f = form(ctx, obj)?;
        let (laid, scroll) = (f.laid.clone(), f.scroll);
        let fonts = fonts(ctx);
        for (item, r) in laid {
            // Root it: the game's code below may take it out of the form.
            let item = ctx.root(item);
            if item::tag(ctx, item)? != Tag::Custom {
                continue;
            }
            if r.y + r.height < scroll || r.y > scroll + height {
                continue;
            }
            let content = item::content_area(ctx, item, &fonts, r)?;
            custom::render(ctx, item, content.width, content.height)?;
        }
        Ok(())
    }

    fn paint(&self, ctx: &mut Ctx<'_>, obj: ObjRef, target: &mut ScreenTarget<'_>) -> JResult<()> {
        let f = form(ctx, obj)?;
        let (laid, scroll, height, focus) = (f.laid.clone(), f.scroll, f.height, f.focus);
        let Some(mut p) = Painter::new(
            target.surface,
            target.width,
            target.height,
            target.fonts,
            target.theme,
        ) else {
            return Ok(());
        };
        let view = Rect::new(0, 0, target.width, target.height);
        let mut popup = None;
        for &(item, r) in &laid {
            let on_screen = Rect::new(r.x, r.y - scroll, r.width, r.height);
            if on_screen.intersect(view).is_empty() {
                continue;
            }
            item::paint(ctx, item, &mut p, on_screen, focus == Some(item))?;
            if focus == Some(item) && choice_group::is_open(ctx, item)? {
                popup = Some((item, on_screen));
            }
        }
        if let Some((item, r)) = popup {
            let anchor = item::content_area(ctx, item, &p.fonts, r)?;
            let drawn = choice_group::paint_popup(ctx, item, &mut p, anchor, view)?;
            form_mut(ctx, obj)?.popup = Some(drawn);
        } else {
            form_mut(ctx, obj)?.popup = None;
        }
        p.scrollbar(height, target.height, scroll);
        Ok(())
    }

    fn key(&self, ctx: &mut Ctx<'_>, obj: ObjRef, event: KeyEvent) -> JResult<bool> {
        ensure_layout(ctx, obj)?;
        if let Some(item) = form(ctx, obj)?.focus {
            let item = ctx.root(item);
            let handled = item::key(ctx, obj, item, event)?;
            if after(ctx, obj, item, handled)? {
                return Ok(true);
            }
        }
        if event.phase == KeyPhase::Released {
            return Ok(false);
        }
        match event.game_action {
            keys::UP | keys::DOWN => traverse(ctx, obj, event.game_action),
            _ => Ok(false),
        }
    }

    fn pointer(
        &self,
        ctx: &mut Ctx<'_>,
        obj: ObjRef,
        action: PointerAction,
        x: i32,
        y: i32,
    ) -> JResult<bool> {
        ensure_layout(ctx, obj)?;
        let f = form(ctx, obj)?;
        let (scroll, popup, focus) = (f.scroll, f.popup, f.focus);
        if let (Some(r), Some(item)) = (popup, focus) {
            if action == PointerAction::Release {
                let handled = if r.contains(x, y) {
                    choice_group::popup_tap(ctx, item, r, y)?
                } else {
                    choice_group::close(ctx, item)?;
                    Handled::Yes
                };
                return after(ctx, obj, item, handled);
            }
            return Ok(true);
        }
        let (fx, fy) = (x, y + scroll);
        match action {
            PointerAction::Press => {
                let hit = f
                    .laid
                    .iter()
                    .find(|(_, r)| r.contains(fx, fy))
                    .map(|&(i, _)| i);
                let f = form_mut(ctx, obj)?;
                f.pressed = hit.map(|i| (i, y, scroll));
                f.dragging = false;
                if let Some(item) = hit
                    && item::focusable(ctx, item)?
                {
                    move_focus(ctx, obj, Some(item), 0)?;
                }
            }
            PointerAction::Drag => {
                let Some((_, y0, scroll0)) = f.pressed else {
                    return Ok(false);
                };
                if (y - y0).abs() > TAP_SLOP {
                    form_mut(ctx, obj)?.dragging = true;
                }
                if form(ctx, obj)?.dragging {
                    let target = scroll0 - (y - y0);
                    let current = form(ctx, obj)?.scroll;
                    scroll_by(ctx, obj, target - current)?;
                    return Ok(true);
                }
            }
            PointerAction::Release => {}
        }
        let f = form(ctx, obj)?;
        let (pressed, dragging) = (f.pressed, f.dragging);
        let Some((item, _, _)) = pressed else {
            return Ok(false);
        };
        if action == PointerAction::Release {
            form_mut(ctx, obj)?.pressed = None;
            if dragging {
                return Ok(true);
            }
        }
        let item = ctx.root(item);
        let Some(r) = boxed(form(ctx, obj)?, item) else {
            return Ok(false);
        };
        let fonts = fonts(ctx);
        let content = item::content_area(ctx, item, &fonts, r)?;
        let at = (fx - content.x, fy - content.y);
        let size = (content.width, content.height);
        let handled = item::pointer(ctx, obj, item, action, at, size)?;
        after(ctx, obj, item, handled)
    }

    fn text(&self, ctx: &mut Ctx<'_>, obj: ObjRef, typed: &str) -> JResult<bool> {
        let Some(item) = form(ctx, obj)?.focus else {
            return Ok(false);
        };
        let item = ctx.root(item);
        let handled = item::text(ctx, item, typed)?;
        after(ctx, obj, item, handled)
    }

    fn shown(&self, ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
        let items = form(ctx, obj)?.items.clone();
        for item in items {
            let item = ctx.root(item);
            if item::tag(ctx, item)? == Tag::Custom {
                custom::notify_shown(ctx, item, true)?;
            }
        }
        if form(ctx, obj)?.focus.is_none() {
            let first = first_focus(ctx, obj)?;
            move_focus(ctx, obj, first, 0)?;
        }
        sync_commands(ctx, obj)?;
        update_text(ctx, obj)
    }

    fn hidden(&self, ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
        set_wants_text(ctx, obj, false);
        let items = form(ctx, obj)?.items.clone();
        for item in items {
            let item = ctx.root(item);
            match item::tag(ctx, item)? {
                Tag::Custom => custom::notify_shown(ctx, item, false)?,
                Tag::Choice => choice_group::close(ctx, item)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn command(&self, ctx: &mut Ctx<'_>, obj: ObjRef, command: ObjRef) -> JResult<bool> {
        let f = form(ctx, obj)?;
        let Some(item) = f.focus.filter(|_| f.added.contains(&command)) else {
            return Ok(false);
        };
        let item = ctx.root(item);
        item::fire_command(ctx, item, command)?;
        Ok(true)
    }

    fn tick(&self, ctx: &mut Ctx<'_>, obj: ObjRef, now: Instant) -> JResult<Option<Instant>> {
        let mut next = ticker_tick(ctx, obj, now)?;
        let mut sooner = |at: Instant| next = Some(next.map_or(at, |n| n.min(at)));
        if let Some(item) = form(ctx, obj)?.focus
            && item::tag(ctx, item)? == Tag::Text
        {
            if text_field::expire(ctx, item, now)? {
                display::repaint(ctx, obj)?;
            }
            if let Some(at) = text_field::editor(ctx, item)?.pending_until() {
                sooner(at);
            }
        }
        let laid = form(ctx, obj)?.laid.clone();
        for (item, _) in laid {
            if item::tag(ctx, item)? == Tag::Gauge && gauge::running(ctx, item)? {
                display::repaint(ctx, obj)?;
                sooner(now + gauge::FRAME);
                break;
            }
        }
        Ok(next)
    }
}

fn this(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    ctx.non_null(v)
}

/// Checks that `item` may join the form.
fn check_free(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    if item::data(ctx, item)?.owner.is_some() {
        return ctx.throw(
            "java/lang/IllegalStateException",
            "the item is already in a container",
        );
    }
    Ok(())
}

/// Puts `item` at `at` in the form.
fn insert(ctx: &mut Ctx<'_>, obj: ObjRef, at: usize, item: ObjRef) -> JResult<()> {
    check_free(ctx, item)?;
    let d = item::data_mut(ctx, item)?;
    d.owner = Some(obj);
    d.in_alert = false;
    form_mut(ctx, obj)?.items.insert(at, item);
    item_changed(ctx, obj)
}

/// Takes the item at `at` out of the form.
fn remove(ctx: &mut Ctx<'_>, obj: ObjRef, at: usize) -> JResult<ObjRef> {
    let f = form_mut(ctx, obj)?;
    let item = f.items.remove(at);
    f.laid.retain(|(i, _)| *i != item);
    item::data_mut(ctx, item)?.owner = None;
    if form(ctx, obj)?.focus == Some(item) {
        form_mut(ctx, obj)?.focus = None;
        sync_commands(ctx, obj)?;
        update_text(ctx, obj)?;
    }
    item_changed(ctx, obj)?;
    Ok(item)
}

/// `itemNum` as an index of the form, up to `size` with `end`.
fn index(ctx: &mut Ctx<'_>, obj: ObjRef, at: i32, end: bool) -> JResult<usize> {
    let len = form(ctx, obj)?.items.len() as i64;
    let limit = if end { len + 1 } else { len };
    if !(0..limit).contains(&i64::from(at)) {
        return ctx.throw("java/lang/IndexOutOfBoundsException", &format!("item {at}"));
    }
    Ok(at as usize)
}

fn append(ctx: &mut Ctx<'_>, obj: ObjRef, item: ObjRef) -> JResult<Value> {
    let at = form(ctx, obj)?.items.len();
    insert(ctx, obj, at, item)?;
    Ok(Value::Int(at as i32))
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Screen")
        .payload_with(|| Box::new(DisplayableData::for_screen(&FORM, FormState::default())))
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            displayable::data_mut(ctx, obj)?.title = a[1].obj();
            Ok(Value::Void)
        })
        .method(
            "<init>",
            &format!("(Ljava/lang/String;[{ITEM_DESC})V"),
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                displayable::data_mut(ctx, obj)?.title = a[1].obj();
                let Some(array) = a[2].obj() else {
                    return Ok(Value::Void);
                };
                let items = ctx.array::<Option<ObjRef>>(array)?.to_vec();
                for item in &items {
                    match item {
                        None => {
                            return ctx.throw("java/lang/NullPointerException", "a null item");
                        }
                        Some(item) => check_free(ctx, *item)?,
                    }
                }
                for item in items.into_iter().flatten() {
                    append(ctx, obj, item)?;
                }
                Ok(Value::Void)
            },
        )
        .method("append", &format!("({ITEM_DESC})I"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let item = ctx.non_null(a[1])?;
            append(ctx, obj, item)
        })
        .method("append", "(Ljava/lang/String;)I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            let text = ctx.non_null(a[1])?;
            let item = ctx.construct(
                "javax/microedition/lcdui/StringItem",
                "(Ljava/lang/String;Ljava/lang/String;)V",
                &[Value::NULL, text.into()],
            )?;
            append(ctx, obj, item)
        })
        .method("append", "(Ljavax/microedition/lcdui/Image;)I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            let image = ctx.non_null(a[1])?;
            let item = ctx.construct(
                "javax/microedition/lcdui/ImageItem",
                "(Ljava/lang/String;Ljavax/microedition/lcdui/Image;ILjava/lang/String;)V",
                &[Value::NULL, image.into(), Value::Int(0), Value::NULL],
            )?;
            append(ctx, obj, item)
        })
        .method("insert", &format!("(I{ITEM_DESC})V"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let at = index(ctx, obj, a[1].int(), true)?;
            let item = ctx.non_null(a[2])?;
            insert(ctx, obj, at, item)?;
            Ok(Value::Void)
        })
        .method("delete", "(I)V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            let at = index(ctx, obj, a[1].int(), false)?;
            remove(ctx, obj, at)?;
            Ok(Value::Void)
        })
        .method("deleteAll", "()V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            while !form(ctx, obj)?.items.is_empty() {
                let last = form(ctx, obj)?.items.len() - 1;
                remove(ctx, obj, last)?;
            }
            form_mut(ctx, obj)?.scroll = 0;
            Ok(Value::Void)
        })
        .method("set", &format!("(I{ITEM_DESC})V"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let at = index(ctx, obj, a[1].int(), false)?;
            let item = ctx.non_null(a[2])?;
            if form(ctx, obj)?.items[at] == item {
                return Ok(Value::Void);
            }
            check_free(ctx, item)?;
            remove(ctx, obj, at)?;
            insert(ctx, obj, at, item)?;
            Ok(Value::Void)
        })
        .method("get", &format!("(I){ITEM_DESC}"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let at = index(ctx, obj, a[1].int(), false)?;
            Ok(form(ctx, obj)?.items[at].into())
        })
        .method(
            "setItemStateListener",
            "(Ljavax/microedition/lcdui/ItemStateListener;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                form_mut(ctx, obj)?.listener = a[1].obj();
                Ok(Value::Void)
            },
        )
        .method("size", "()I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(Value::Int(form(ctx, obj)?.items.len() as i32))
        })
}
