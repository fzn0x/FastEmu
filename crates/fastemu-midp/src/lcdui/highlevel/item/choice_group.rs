//! `javax.microedition.lcdui.ChoiceGroup`: a `Choice` inside a form, as
//! radio buttons (`EXCLUSIVE`), check boxes (`MULTIPLE`) or a closed
//! line that opens a list (`POPUP`).
//!
//! Up and down move between the elements before they leave the group;
//! select chooses the element with the focus. An open popup takes every
//! key until select or another key closes it; the form draws it over the
//! items below.

use fastemu_core::PointerAction;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::{Handled, ItemData, Kind, Needs, changed, data, data_mut};
use crate::lcdui::Rect;
use crate::lcdui::highlevel::choice::{
    self, ChoiceModel, EXCLUSIVE, MULTIPLE, POPUP, TEXT_WRAP_ON,
};
use crate::lcdui::highlevel::draw::{Fonts, Painter, ellipsize};
use crate::lcdui::highlevel::rows::{mark_side, paint_row, row_height, rows};
use crate::lcdui::highlevel::text::INSET;
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase};

const CLASS: &str = "javax/microedition/lcdui/ChoiceGroup";

/// What a `ChoiceGroup` keeps.
#[derive(Debug)]
pub struct GroupState {
    pub model: ChoiceModel,
    /// Whether a popup is open.
    pub open: bool,
}

fn group<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a GroupState> {
    match &data(ctx, item)?.kind {
        Kind::Choice(g) => Ok(g),
        _ => unreachable!("a ChoiceGroup's payload is made by its class"),
    }
}

fn group_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut GroupState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Choice(g) => Ok(g),
        _ => unreachable!("a ChoiceGroup's payload is made by its class"),
    }
}

fn model_of<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut ChoiceModel> {
    Ok(&mut group_mut(ctx, item)?.model)
}

/// The rows, or the closed popup's line.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, max: i32) -> JResult<Needs> {
    let g = group(ctx, item)?;
    let min_width = (fonts.text.height() * 4).min(max);
    if g.model.kind == POPUP {
        return Ok(Needs {
            min_width,
            width: max,
            height: fonts.text.height() + 2 * INSET,
        });
    }
    let mark = mark_side(&fonts.text);
    let wrap_text = g.model.fit_policy == TEXT_WRAP_ON;
    let height = rows(ctx, &g.model, fonts)?
        .iter()
        .map(|r| row_height(r, max, mark, wrap_text))
        .sum();
    Ok(Needs {
        min_width,
        width: max,
        height,
    })
}

pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let g = group(ctx, item)?;
    let rows = rows(ctx, &g.model, &p.fonts)?;
    let theme = *p.theme;
    if g.model.kind == POPUP {
        p.fill(area, theme.background);
        let border = if focused {
            theme.highlighted_border
        } else {
            theme.border
        };
        p.frame(area, border, if focused { 2 } else { 1 });
        let arrow = area.height;
        p.arrow_down(
            Rect::new(area.x + area.width - arrow, area.y, arrow, area.height),
            border,
        );
        if let Some(row) = rows.iter().find(|r| r.selected) {
            let font = row.font.clone();
            let room = area.width - arrow - 2 * INSET;
            let cut = ellipsize(&font, &row.text, room);
            p.text(
                &font,
                &cut,
                area.x + INSET,
                area.y + INSET,
                theme.foreground,
            );
        }
        return Ok(());
    }
    let mark = mark_side(&p.fonts.text);
    let wrap_text = g.model.fit_policy == TEXT_WRAP_ON;
    let mut y = area.y;
    for (i, row) in rows.iter().enumerate() {
        let h = row_height(row, area.width, mark, wrap_text);
        let r = Rect::new(area.x, y, area.width, h);
        let lit = focused && i == g.model.focus;
        paint_row(ctx, p, row, r, g.model.kind, mark, lit, wrap_text)?;
        y += h;
    }
    Ok(())
}

/// Whether the item is a popup that is open now.
pub(crate) fn is_open(ctx: &Ctx<'_>, item: ObjRef) -> JResult<bool> {
    Ok(matches!(&data(ctx, item)?.kind, Kind::Choice(g) if g.open))
}

/// Closes an open popup.
pub(crate) fn close(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<()> {
    if let Kind::Choice(g) = &mut data_mut(ctx, item)?.kind {
        g.open = false;
    }
    Ok(())
}

/// Puts the focus on the first element, or the last when it comes from
/// below.
pub(crate) fn enter(ctx: &mut Ctx<'_>, item: ObjRef, from_below: bool) -> JResult<()> {
    let model = model_of(ctx, item)?;
    if model.kind == POPUP {
        return Ok(());
    }
    let at = if from_below {
        model.len().saturating_sub(1)
    } else {
        0
    };
    model.focus = at;
    Ok(())
}

/// Selects the element with the focus and closes a popup.
fn choose(ctx: &mut Ctx<'_>, item: ObjRef) -> JResult<Handled> {
    let g = group_mut(ctx, item)?;
    g.open = false;
    Ok(if g.model.choose_focused() {
        Handled::Changed
    } else {
        Handled::Yes
    })
}

pub(super) fn key(
    ctx: &mut Ctx<'_>,
    _form: ObjRef,
    item: ObjRef,
    event: KeyEvent,
) -> JResult<Handled> {
    if event.phase == KeyPhase::Released {
        return Ok(Handled::No);
    }
    let action = event.game_action;
    let g = group_mut(ctx, item)?;
    let len = g.model.len();
    if g.model.kind == POPUP {
        if !g.open {
            if action == keys::FIRE && event.phase == KeyPhase::Pressed && len > 0 {
                g.open = true;
                g.model.focus = g.model.selected_index().max(0) as usize;
                return Ok(Handled::Yes);
            }
            return Ok(Handled::No);
        }
        match action {
            keys::UP => g.model.focus = (g.model.focus + len - 1) % len.max(1),
            keys::DOWN => g.model.focus = (g.model.focus + 1) % len.max(1),
            keys::FIRE => return choose(ctx, item),
            _ => g.open = false,
        }
        return Ok(Handled::Yes);
    }
    match action {
        keys::UP if g.model.focus > 0 => g.model.focus -= 1,
        keys::DOWN if g.model.focus + 1 < len => g.model.focus += 1,
        keys::FIRE if event.phase == KeyPhase::Pressed && len > 0 => return choose(ctx, item),
        _ => return Ok(Handled::No),
    }
    Ok(Handled::Yes)
}

/// A tap on a row chooses it; a tap on a closed popup opens it.
pub(super) fn pointer(
    ctx: &mut Ctx<'_>,
    _form: ObjRef,
    item: ObjRef,
    action: PointerAction,
    y: i32,
    width: i32,
) -> JResult<Handled> {
    if action != PointerAction::Release {
        return Ok(Handled::No);
    }
    let fonts = super::measure_fonts(ctx).0;
    let g = group(ctx, item)?;
    if g.model.kind == POPUP {
        let g = group_mut(ctx, item)?;
        g.open = !g.open && !g.model.is_empty();
        g.model.focus = g.model.selected_index().max(0) as usize;
        return Ok(Handled::Yes);
    }
    let mark = mark_side(&fonts.text);
    let wrap_text = g.model.fit_policy == TEXT_WRAP_ON;
    let mut top = 0;
    let mut hit = None;
    for (i, row) in rows(ctx, &g.model, &fonts)?.iter().enumerate() {
        let h = row_height(row, width, mark, wrap_text);
        if y >= top && y < top + h {
            hit = Some(i);
            break;
        }
        top += h;
    }
    let Some(at) = hit else {
        return Ok(Handled::No);
    };
    model_of(ctx, item)?.focus = at;
    choose(ctx, item)
}

/// Draws an open popup's list below `anchor` (or above it when there is
/// more room there), inside `view`; returns where it went.
pub(crate) fn paint_popup(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    anchor: Rect,
    view: Rect,
) -> JResult<Rect> {
    let r = popup_rect(ctx, item, &p.fonts, anchor, view)?;
    let g = group(ctx, item)?;
    let rows = rows(ctx, &g.model, &p.fonts)?;
    let theme = *p.theme;
    p.fill(r, theme.background);
    p.outline(r, theme.highlighted_border);
    let line = popup_line(&p.fonts);
    let visible = ((r.height - 2) / line).max(1) as usize;
    let first = g.model.focus.saturating_sub(visible - 1);
    p.clip(Rect::new(r.x + 1, r.y + 1, r.width - 2, r.height - 2));
    for (i, row) in rows.iter().enumerate().skip(first).take(visible) {
        let y = r.y + 1 + (i - first) as i32 * line;
        let cell = Rect::new(r.x + 1, y, r.width - 2, line);
        paint_row(ctx, p, row, cell, POPUP, 0, i == g.model.focus, false)?;
    }
    p.unclip();
    Ok(r)
}

fn popup_line(fonts: &Fonts) -> i32 {
    fonts.text.height() + 4
}

/// Where the open popup's list goes.
pub(crate) fn popup_rect(
    ctx: &Ctx<'_>,
    item: ObjRef,
    fonts: &Fonts,
    anchor: Rect,
    view: Rect,
) -> JResult<Rect> {
    let count = group(ctx, item)?.model.len().max(1) as i32;
    let line = popup_line(fonts);
    let below = view.y + view.height - (anchor.y + anchor.height);
    let above = anchor.y - view.y;
    let room = below.max(above);
    let height = (count * line + 2).min(room.max(line + 2));
    let y = if below >= height || below >= above {
        anchor.y + anchor.height
    } else {
        anchor.y - height
    };
    Ok(Rect::new(anchor.x, y, anchor.width, height))
}

/// A tap at `y` inside an open popup's list at `r`: chooses the row.
pub(crate) fn popup_tap(ctx: &mut Ctx<'_>, item: ObjRef, r: Rect, y: i32) -> JResult<Handled> {
    let fonts = super::measure_fonts(ctx).0;
    let line = popup_line(&fonts);
    let g = group(ctx, item)?;
    let visible = ((r.height - 2) / line).max(1) as usize;
    let first = g.model.focus.saturating_sub(visible - 1);
    let at = first + ((y - r.y - 1).max(0) / line) as usize;
    if at >= g.model.len() {
        close(ctx, item)?;
        return Ok(Handled::Yes);
    }
    model_of(ctx, item)?.focus = at;
    choose(ctx, item)
}

fn label_and_type(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<ObjRef> {
    let kind = a[2].int();
    choice::check_type(ctx, kind, &[EXCLUSIVE, MULTIPLE, POPUP])?;
    let item = ctx.non_null(a[0])?;
    let d = data_mut(ctx, item)?;
    d.label = a[1].obj();
    d.kind = Kind::Choice(GroupState {
        model: ChoiceModel::new(kind),
        open: false,
    });
    Ok(item)
}

/// The class.
pub fn class() -> NativeClass {
    let access = choice::Access {
        model: model_of,
        changed,
    };
    let class = NativeClass::new(CLASS)
        .extends(super::ITEM)
        .implements(choice::CHOICE)
        .payload_with(|| {
            Box::new(ItemData::new(Kind::Choice(GroupState {
                model: ChoiceModel::new(EXCLUSIVE),
                open: false,
            })))
        })
        .method("<init>", "(Ljava/lang/String;I)V", |ctx, a| {
            label_and_type(ctx, a)?;
            Ok(Value::Void)
        })
        .method(
            "<init>",
            "(Ljava/lang/String;I[Ljava/lang/String;[Ljavax/microedition/lcdui/Image;)V",
            move |ctx, a| {
                let item = label_and_type(ctx, a)?;
                choice::fill(ctx, item, access, a[3], a[4])?;
                Ok(Value::Void)
            },
        );
    choice::methods(class, access)
}
