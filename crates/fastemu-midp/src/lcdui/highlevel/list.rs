//! `javax.microedition.lcdui.List`: a full screen of elements, the menu
//! of games that do not draw their own.
//!
//! Up and down move the focus (wrapping round at the ends), left and
//! right move it a screen at a time (stopping at the ends); select
//! chooses the focused element in an `EXCLUSIVE` or `MULTIPLE` list, and
//! in an `IMPLICIT` one fires the select command (`SELECT_COMMAND`
//! unless the game set another), whose `commandAction` sees the focused
//! element as selected. A tap focuses a row and a tap on the focused row
//! selects it.

use std::time::Instant;

use fastemu_core::PointerAction;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value, classfile::access};

use super::choice::{self, ChoiceModel, EXCLUSIVE, IMPLICIT, MULTIPLE, TEXT_WRAP_ON};
use super::draw::{Painter, SCROLLBAR};
use super::rows::{Row, mark_side, paint_row, row_height, rows};
use super::ticker_tick;
use crate::lcdui::Rect;
use crate::lcdui::command;
use crate::lcdui::display;
use crate::lcdui::displayable::{self, DisplayableData};
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase, ScreenHandler, ScreenState, ScreenTarget};
use crate::lcdui::state::state;

const CLASS: &str = "javax/microedition/lcdui/List";
const COMMAND: &str = "Ljavax/microedition/lcdui/Command;";

/// What a `List` keeps.
pub struct ListState {
    pub model: ChoiceModel,
    /// The command select fires in an `IMPLICIT` list.
    select: Option<ObjRef>,
    scroll: i32,
    /// Whether the next paint scrolls the focused row into view.
    reveal: bool,
    /// The row a pointer press landed on, and where.
    pressed: Option<(usize, i32, i32)>,
    dragging: bool,
}

impl Default for ListState {
    fn default() -> Self {
        Self {
            model: ChoiceModel::new(IMPLICIT),
            select: None,
            scroll: 0,
            reveal: true,
            pressed: None,
            dragging: false,
        }
    }
}

impl ScreenState for ListState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.model.trace(tracer);
        tracer.mark_opt(self.select);
    }

    fn heap_bytes(&self) -> usize {
        self.model.heap_bytes()
    }
}

fn list<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> JResult<&'a ListState> {
    match displayable::data(ctx, obj)?.state::<ListState>() {
        Some(s) => Ok(s),
        None => unreachable!("a List's payload is made by its class"),
    }
}

fn list_mut<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut ListState> {
    match displayable::data_mut(ctx, obj)?.state_mut::<ListState>() {
        Some(s) => Ok(s),
        None => unreachable!("a List's payload is made by its class"),
    }
}

fn model_of<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut ChoiceModel> {
    Ok(&mut list_mut(ctx, obj)?.model)
}

fn changed(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    list_mut(ctx, obj)?.reveal = true;
    display::repaint(ctx, obj)
}

/// `List.SELECT_COMMAND`.
fn select_command(ctx: &mut Ctx<'_>) -> JResult<ObjRef> {
    let field = ctx.static_field_id(CLASS, "SELECT_COMMAND", COMMAND)?;
    let value = ctx.get_static(field)?;
    ctx.non_null(value)
}

/// The rows' heights and the rows themselves at `width`.
fn measure(ctx: &mut Ctx<'_>, obj: ObjRef, width: i32) -> JResult<(Vec<Row>, Vec<i32>, i32)> {
    let fonts = super::draw::Fonts::new(&state(ctx).fonts);
    let model = &list(ctx, obj)?.model;
    let rows = rows(ctx, model, &fonts)?;
    let mark = if model.kind == IMPLICIT {
        0
    } else {
        mark_side(&fonts.text)
    };
    let wrap = model.fit_policy == TEXT_WRAP_ON;
    let heights = rows
        .iter()
        .map(|r| row_height(r, width, mark, wrap))
        .collect();
    Ok((rows, heights, mark))
}

/// Keeps the focused row in view of a `view` pixels tall list.
fn reveal(ctx: &mut Ctx<'_>, obj: ObjRef, heights: &[i32], view: i32) -> JResult<()> {
    let l = list_mut(ctx, obj)?;
    let total: i32 = heights.iter().sum();
    if std::mem::take(&mut l.reveal) && !heights.is_empty() {
        let focus = l.model.focus.min(heights.len() - 1);
        let top: i32 = heights[..focus].iter().sum();
        let bottom = top + heights[focus];
        if top < l.scroll {
            l.scroll = top;
        } else if bottom > l.scroll + view {
            l.scroll = bottom - view;
        }
    }
    l.scroll = l.scroll.clamp(0, (total - view).max(0));
    Ok(())
}

/// The width rows take in a content area `width` wide.
fn row_width(width: i32) -> i32 {
    width - SCROLLBAR
}

/// Moves the focus by `delta` rows, wrapping round.
fn move_focus(ctx: &mut Ctx<'_>, obj: ObjRef, delta: i32) -> JResult<()> {
    let model = model_of(ctx, obj)?;
    let len = model.len() as i32;
    if len == 0 {
        return Ok(());
    }
    let at = (model.focus as i32 + delta).rem_euclid(len) as usize;
    model.focus_on(at);
    changed(ctx, obj)
}

/// The row a page up (`dir` -1) or down (1) from `focus` lands on, for
/// rows of `heights` in a view `view` pixels tall: as many rows as fit
/// in the view, at least one, and no further than the ends.
fn page_target(heights: &[i32], focus: usize, view: i32, dir: i32) -> usize {
    let mut at = focus;
    let mut used = 0;
    while let Some(next) = at.checked_add_signed(dir as isize) {
        let Some(&h) = heights.get(next) else {
            break;
        };
        used += h;
        if used > view && at != focus {
            break;
        }
        at = next;
    }
    at
}

/// Moves the focus a page up (`dir` -1) or down (1).
fn page(ctx: &mut Ctx<'_>, obj: ObjRef, dir: i32) -> JResult<()> {
    let content = displayable::layout(ctx, obj)?.content;
    let (_, heights, _) = measure(ctx, obj, row_width(content.width))?;
    let model = model_of(ctx, obj)?;
    if model.is_empty() {
        return Ok(());
    }
    let at = page_target(&heights, model.focus, content.height, dir);
    model.focus_on(at);
    changed(ctx, obj)
}

/// What select does on the focused element.
fn choose(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let l = list(ctx, obj)?;
    if l.model.is_empty() {
        return Ok(());
    }
    if l.model.kind == IMPLICIT {
        let Some(select) = l.select else {
            return Ok(());
        };
        let select = ctx.root(select);
        return command::fire(ctx, obj, select);
    }
    model_of(ctx, obj)?.choose_focused();
    changed(ctx, obj)
}

/// The row at `y` pixels down the shown list.
fn row_at(ctx: &mut Ctx<'_>, obj: ObjRef, y: i32) -> JResult<Option<usize>> {
    let width = displayable::layout(ctx, obj)?.content.width;
    let (_, heights, _) = measure(ctx, obj, row_width(width))?;
    let mut top = -list(ctx, obj)?.scroll;
    for (i, h) in heights.into_iter().enumerate() {
        if y >= top && y < top + h {
            return Ok(Some(i));
        }
        top += h;
    }
    Ok(None)
}

struct ListHandler;

static LIST: ListHandler = ListHandler;

impl ScreenHandler for ListHandler {
    fn paint(&self, ctx: &mut Ctx<'_>, obj: ObjRef, target: &mut ScreenTarget<'_>) -> JResult<()> {
        let width = row_width(target.width);
        let (rows, heights, mark) = measure(ctx, obj, width)?;
        reveal(ctx, obj, &heights, target.height)?;
        let l = list(ctx, obj)?;
        let (scroll, focus, kind) = (l.scroll, l.model.focus, l.model.kind);
        let wrap = l.model.fit_policy == TEXT_WRAP_ON;
        let Some(mut p) = Painter::new(
            target.surface,
            target.width,
            target.height,
            target.fonts,
            target.theme,
        ) else {
            return Ok(());
        };
        let mut y = -scroll;
        for (i, (row, &h)) in rows.iter().zip(&heights).enumerate() {
            if y + h > 0 && y < target.height {
                let r = Rect::new(1, y, width - 2, h);
                paint_row(ctx, &mut p, row, r, kind, mark, i == focus, wrap)?;
            }
            y += h;
        }
        p.scrollbar(heights.iter().sum(), target.height, scroll);
        Ok(())
    }

    fn key(&self, ctx: &mut Ctx<'_>, obj: ObjRef, event: KeyEvent) -> JResult<bool> {
        if event.phase == KeyPhase::Released {
            return Ok(false);
        }
        match event.game_action {
            keys::UP => move_focus(ctx, obj, -1)?,
            keys::DOWN => move_focus(ctx, obj, 1)?,
            keys::LEFT => page(ctx, obj, -1)?,
            keys::RIGHT => page(ctx, obj, 1)?,
            keys::FIRE if event.phase == KeyPhase::Pressed => choose(ctx, obj)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn pointer(
        &self,
        ctx: &mut Ctx<'_>,
        obj: ObjRef,
        action: PointerAction,
        _: i32,
        y: i32,
    ) -> JResult<bool> {
        match action {
            PointerAction::Press => {
                let row = row_at(ctx, obj, y)?;
                let l = list_mut(ctx, obj)?;
                l.pressed = row.map(|r| (r, y, l.scroll));
                l.dragging = false;
                if let Some(row) = row {
                    let was = l.model.focus;
                    if was != row {
                        l.model.focus_on(row);
                        display::repaint(ctx, obj)?;
                    }
                }
            }
            PointerAction::Drag => {
                let l = list_mut(ctx, obj)?;
                let Some((_, y0, scroll0)) = l.pressed else {
                    return Ok(false);
                };
                if (y - y0).abs() > 6 {
                    l.dragging = true;
                }
                if l.dragging {
                    l.scroll = scroll0 - (y - y0);
                    display::repaint(ctx, obj)?;
                }
            }
            PointerAction::Release => {
                let pressed = list_mut(ctx, obj)?.pressed.take();
                let dragging = list(ctx, obj)?.dragging;
                if let Some((row, _, _)) = pressed
                    && !dragging
                    && row_at(ctx, obj, y)? == Some(row)
                {
                    choose(ctx, obj)?;
                }
            }
        }
        Ok(true)
    }

    fn tick(&self, ctx: &mut Ctx<'_>, obj: ObjRef, now: Instant) -> JResult<Option<Instant>> {
        ticker_tick(ctx, obj, now)
    }
}

fn init(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<ObjRef> {
    let kind = a[2].int();
    choice::check_type(ctx, kind, &[IMPLICIT, EXCLUSIVE, MULTIPLE])?;
    let obj = ctx.non_null(a[0])?;
    let select = if kind == IMPLICIT {
        Some(select_command(ctx)?)
    } else {
        None
    };
    displayable::data_mut(ctx, obj)?.title = a[1].obj();
    let l = list_mut(ctx, obj)?;
    l.model = ChoiceModel::new(kind);
    l.select = select;
    Ok(obj)
}

/// The class.
pub fn class() -> NativeClass {
    let access = choice::Access {
        model: model_of,
        changed,
    };
    let class = NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Screen")
        .implements(choice::CHOICE)
        .payload_with(|| Box::new(DisplayableData::for_screen(&LIST, ListState::default())))
        .field_with(
            "SELECT_COMMAND",
            COMMAND,
            access::PUBLIC | access::STATIC | access::FINAL,
        )
        .static_init(|ctx, _| {
            let label = ctx.new_string("")?;
            let select = ctx.construct(
                "javax/microedition/lcdui/Command",
                "(Ljava/lang/String;II)V",
                &[label.into(), Value::Int(command::SCREEN), Value::Int(0)],
            )?;
            let field = ctx.static_field_id(CLASS, "SELECT_COMMAND", COMMAND)?;
            ctx.set_static(field, select.into())?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;I)V", |ctx, a| {
            init(ctx, a)?;
            Ok(Value::Void)
        })
        .method(
            "<init>",
            "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
            move |ctx, a| {
                let obj = init(ctx, a)?;
                choice::fill(ctx, obj, access, a[3], a[4])?;
                Ok(Value::Void)
            },
        )
        .method("removeCommand", &format!("({COMMAND})V"), |ctx, a| {
            let obj = ctx.non_null(a[0])?;
            let Some(command) = a[1].obj() else {
                return Ok(Value::Void);
            };
            let l = list_mut(ctx, obj)?;
            if l.select == Some(command) {
                l.select = None;
            }
            displayable::data_mut(ctx, obj)?.remove_command(command);
            display::relayout(ctx, obj);
            Ok(Value::Void)
        })
        .method("setSelectCommand", &format!("({COMMAND})V"), |ctx, a| {
            let obj = ctx.non_null(a[0])?;
            if list(ctx, obj)?.model.kind != IMPLICIT {
                return Ok(Value::Void);
            }
            let default = select_command(ctx)?;
            let command = a[1].obj();
            list_mut(ctx, obj)?.select = command;
            if let Some(command) = command.filter(|&c| c != default) {
                displayable::data_mut(ctx, obj)?.add_command(command);
                display::relayout(ctx, obj);
            }
            Ok(Value::Void)
        });
    choice::methods(class, access)
}

#[cfg(test)]
mod tests {
    use super::page_target;

    #[test]
    fn a_page_moves_as_many_rows_as_fit() {
        let rows = [20; 10];
        assert_eq!(page_target(&rows, 0, 60, 1), 3);
        assert_eq!(page_target(&rows, 3, 60, -1), 0);
        assert_eq!(page_target(&rows, 8, 60, 1), 9, "it stops at the end");
        assert_eq!(page_target(&rows, 1, 60, -1), 0, "and at the start");
        assert_eq!(page_target(&rows, 9, 60, 1), 9);
    }

    #[test]
    fn a_page_moves_at_least_one_row() {
        assert_eq!(page_target(&[100, 100, 100], 0, 60, 1), 1);
        assert_eq!(page_target(&[], 0, 60, 1), 0);
    }
}
