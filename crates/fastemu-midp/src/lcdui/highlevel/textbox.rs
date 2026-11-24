//! `javax.microedition.lcdui.TextBox`: a full screen of text to edit, as
//! games use for a player's name.
//!
//! A host keyboard types at the caret ([`InputEvent::Text`]); a keypad
//! types by multi-tap, `#` changes the input mode, clear deletes, the
//! left and right arrows move the caret and up and down move it a line.
//! While a text box is shown, `MidpInstance::wants_text` is true, so the
//! apps send typed characters rather than keypad keys.
//!
//! [`InputEvent::Text`]: fastemu_core::InputEvent::Text

use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::draw::{Painter, width as text_width};
use super::text::{self, INSET, KeyUse, TextEditor, editor};
use super::{set_wants_text, ticker_tick};
use crate::env::env;
use crate::lcdui::Rect;
use crate::lcdui::display;
use crate::lcdui::displayable::{self, DisplayableData};
use crate::lcdui::screen::{KeyEvent, KeyPhase, ScreenHandler, ScreenState, ScreenTarget};
use crate::lcdui::state::state;

const CLASS: &str = "javax/microedition/lcdui/TextBox";

/// What a `TextBox` keeps.
pub struct TextBoxState {
    editor: TextEditor,
    /// The first line shown.
    first: usize,
}

impl Default for TextBoxState {
    fn default() -> Self {
        Self {
            editor: TextEditor::new(&[], 1, editor::ANY).expect("a valid empty editor"),
            first: 0,
        }
    }
}

impl ScreenState for TextBoxState {}

fn text_box<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> JResult<&'a TextBoxState> {
    match displayable::data(ctx, obj)?.state::<TextBoxState>() {
        Some(s) => Ok(s),
        None => unreachable!("a TextBox's payload is made by its class"),
    }
}

fn text_box_mut<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut TextBoxState> {
    match displayable::data_mut(ctx, obj)?.state_mut::<TextBoxState>() {
        Some(s) => Ok(s),
        None => unreachable!("a TextBox's payload is made by its class"),
    }
}

fn editor_of<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut TextEditor> {
    Ok(&mut text_box_mut(ctx, obj)?.editor)
}

/// After the API changed the text: repaint, and tell the app whether the
/// box still takes typing (`UNEDITABLE` may have changed).
fn changed(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let editable = text_box(ctx, obj)?.editor.editable();
    set_wants_text(ctx, obj, editable);
    display::repaint(ctx, obj)
}

/// The height of the line above the frame that shows the input mode and
/// how many characters are left.
fn status_height(ctx: &mut Ctx<'_>) -> i32 {
    super::draw::Fonts::new(&state(ctx).fonts).small.height() + 2
}

/// The frame the text is drawn in, for a content area of `size`.
fn frame(status: i32, (width, height): (i32, i32)) -> Rect {
    Rect::new(2, status, width - 4, height - status - 2)
}

/// The lines of text the frame shows at once, and how wide they are.
fn text_area(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<(usize, i32)> {
    let content = displayable::layout(ctx, obj)?.content;
    let status = status_height(ctx);
    let r = frame(status, (content.width, content.height));
    let line = state(ctx).fonts.default_font().height().max(1);
    let rows = ((r.height - 2 * INSET) / line).max(1) as usize;
    Ok((rows, r.width - 2 * INSET))
}

/// Keeps the caret's line in view.
fn follow_caret(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    let (rows, width) = text_area(ctx, obj)?;
    let font = state(ctx).fonts.default_font();
    let tb = text_box_mut(ctx, obj)?;
    let (_, line, _) = text::lines(&font, &tb.editor, width);
    if line < tb.first {
        tb.first = line;
    } else if line >= tb.first + rows {
        tb.first = line + 1 - rows;
    }
    Ok(())
}

/// Moves the caret a line up (`-1`) or down, keeping its x position as
/// near as the line allows.
fn caret_line(ctx: &mut Ctx<'_>, obj: ObjRef, delta: i32) -> JResult<bool> {
    let (_, width) = text_area(ctx, obj)?;
    let font = state(ctx).fonts.default_font();
    let tb = text_box_mut(ctx, obj)?;
    let shown = text::display_text(&tb.editor);
    let (lines, line, x) = text::lines(&font, &tb.editor, width);
    let target = line as i64 + i64::from(delta);
    if target < 0 || target >= lines.len() as i64 {
        return Ok(false);
    }
    let range = lines[target as usize].clone();
    let mut at = range.start;
    while at < range.end && text_width(&font, &shown[range.start..=at]) <= x {
        at += 1;
    }
    tb.editor.set_caret(at);
    Ok(true)
}

struct TextBoxHandler;

static TEXT_BOX: TextBoxHandler = TextBoxHandler;

impl ScreenHandler for TextBoxHandler {
    fn paint(&self, ctx: &mut Ctx<'_>, obj: ObjRef, target: &mut ScreenTarget<'_>) -> JResult<()> {
        follow_caret(ctx, obj)?;
        let tb = text_box(ctx, obj)?;
        let Some(mut p) = Painter::new(
            target.surface,
            target.width,
            target.height,
            target.fonts,
            target.theme,
        ) else {
            return Ok(());
        };
        let theme = *p.theme;
        let small = p.fonts.small.clone();
        let status = small.height() + 2;
        let e = &tb.editor;
        if e.editable() {
            let mode = e.mode().label();
            p.text(&small, &units(mode), 3, 1, theme.border);
        }
        let count = units(&format!("{}/{}", e.text().len(), e.max_size()));
        let right = target.width - 3 - text_width(&small, &count);
        p.text(&small, &count, right, 1, theme.border);
        let font = p.fonts.text.clone();
        let r = frame(status, (target.width, target.height));
        text::paint_box(&mut p, &font, e, r, true, tb.first);
        Ok(())
    }

    fn key(&self, ctx: &mut Ctx<'_>, obj: ObjRef, event: KeyEvent) -> JResult<bool> {
        let codes = env(ctx).profile.key_codes;
        let vertical = event.code == codes.up || event.code == codes.down;
        if vertical && event.phase != KeyPhase::Released {
            let delta = if event.code == codes.up { -1 } else { 1 };
            if caret_line(ctx, obj, delta)? {
                display::repaint(ctx, obj)?;
            }
            return Ok(true);
        }
        let mut e = text_box(ctx, obj)?.editor.clone();
        let used = text::key(ctx, &mut e, event);
        editor_of(ctx, obj).map(|slot| *slot = e)?;
        if used != KeyUse::Ignored {
            display::repaint(ctx, obj)?;
            super::wake_timer(ctx, obj);
        }
        Ok(used != KeyUse::Ignored)
    }

    fn text(&self, ctx: &mut Ctx<'_>, obj: ObjRef, typed: &str) -> JResult<bool> {
        let typed_in = editor_of(ctx, obj)?.type_text(typed);
        if typed_in {
            display::repaint(ctx, obj)?;
        }
        Ok(typed_in)
    }

    fn shown(&self, ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
        let editable = text_box(ctx, obj)?.editor.editable();
        set_wants_text(ctx, obj, editable);
        Ok(())
    }

    fn hidden(&self, ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
        editor_of(ctx, obj)?.commit();
        set_wants_text(ctx, obj, false);
        Ok(())
    }

    fn tick(&self, ctx: &mut Ctx<'_>, obj: ObjRef, now: Instant) -> JResult<Option<Instant>> {
        let mut next = ticker_tick(ctx, obj, now)?;
        let e = editor_of(ctx, obj)?;
        if e.expire(now) {
            display::repaint(ctx, obj)?;
        }
        if let Some(at) = text_box(ctx, obj)?.editor.pending_until() {
            next = Some(next.map_or(at, |n| n.min(at)));
        }
        Ok(next)
    }
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// The class.
pub fn class() -> NativeClass {
    let access = text::Access {
        editor: editor_of,
        changed,
    };
    let class = NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Screen")
        .payload_with(|| {
            Box::new(DisplayableData::for_screen(
                &TEXT_BOX,
                TextBoxState::default(),
            ))
        })
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;II)V",
            |ctx, a| {
                let obj = ctx.non_null(a[0])?;
                let initial = match a[2].obj() {
                    Some(s) => ctx.string_utf16(s)?.to_vec(),
                    None => Vec::new(),
                };
                let e = match TextEditor::new(&initial, a[3].int(), a[4].int()) {
                    Ok(e) => e,
                    Err(why) => return Err(text::throw(ctx, why)),
                };
                displayable::data_mut(ctx, obj)?.title = a[1].obj();
                text_box_mut(ctx, obj)?.editor = e;
                Ok(Value::Void)
            },
        );
    text::methods(class, access)
}
