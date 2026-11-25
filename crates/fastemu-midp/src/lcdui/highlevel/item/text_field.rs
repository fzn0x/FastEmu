//! `javax.microedition.lcdui.TextField`: a text input in a form. Its
//! text, constraints and editing are the `TextBox`'s
//! ([`crate::lcdui::highlevel::text`]).

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::{Handled, ItemData, Kind, Needs, changed, data, data_mut};
use crate::env::env;
use crate::lcdui::Rect;
use crate::lcdui::highlevel::draw::{Fonts, Painter};
use crate::lcdui::highlevel::text::{self, INSET, KeyUse, TextEditor, editor};
use crate::lcdui::screen::KeyEvent;

const CLASS: &str = "javax/microedition/lcdui/TextField";

fn editor_of<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut TextEditor> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Text(e) => Ok(e),
        _ => unreachable!("a TextField's payload is made by its class"),
    }
}

/// The editor of a text field.
pub(crate) fn editor<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a TextEditor> {
    match &data(ctx, item)?.kind {
        Kind::Text(e) => Ok(e),
        _ => unreachable!("a TextField's payload is made by its class"),
    }
}

/// A frame as wide as the field may be, as tall as its wrapped text.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, max: i32) -> JResult<Needs> {
    let e = editor(ctx, item)?;
    let (lines, _, _) = text::lines(&fonts.text, e, max - 2 * INSET);
    let min_width = (fonts.text.char_width(u16::from(b'W')) * 4 + 2 * INSET).min(max);
    Ok(Needs {
        min_width,
        width: max,
        height: lines.len() as i32 * fonts.text.height() + 2 * INSET,
    })
}

/// Paints the frame, the text and, with the focus, the caret.
pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let e = editor(ctx, item)?;
    let font = p.fonts.text.clone();
    text::paint_box(p, &font, e, area, focused, 0);
    Ok(())
}

/// A key while the field has the focus.
pub(super) fn key(ctx: &mut Ctx<'_>, item: ObjRef, event: KeyEvent) -> JResult<Handled> {
    let codes = env(ctx).profile.key_codes;
    let used = text::key(codes, editor_of(ctx, item)?, event);
    if used == KeyUse::Edited {
        ctx.recount(item);
    }
    Ok(match used {
        KeyUse::Ignored => Handled::No,
        KeyUse::Moved => Handled::Yes,
        KeyUse::Edited => Handled::Changed,
    })
}

/// Text from a host keyboard.
pub(super) fn text(ctx: &mut Ctx<'_>, item: ObjRef, typed: &str) -> JResult<Handled> {
    let len = editor(ctx, item)?.text().len();
    if !text::room_for(ctx, len, typed) {
        return Ok(Handled::No);
    }
    Ok(if editor_of(ctx, item)?.type_text(typed) {
        ctx.recount(item);
        Handled::Changed
    } else {
        Handled::No
    })
}

/// Ends a multi-tap letter whose time is up; returns whether one ended.
pub(crate) fn expire(ctx: &mut Ctx<'_>, item: ObjRef, now: std::time::Instant) -> JResult<bool> {
    Ok(editor_of(ctx, item)?.expire(now))
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    let access = text::Access {
        editor: editor_of,
        changed,
    };
    let class = NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| {
            let empty = TextEditor::new(&[], 1, editor::ANY).expect("a valid empty editor");
            Box::new(ItemData::new(Kind::Text(empty)))
        })
        .constant("ANY", "I", int(editor::ANY))
        .constant("EMAILADDR", "I", int(editor::EMAILADDR))
        .constant("NUMERIC", "I", int(editor::NUMERIC))
        .constant("PHONENUMBER", "I", int(editor::PHONENUMBER))
        .constant("URL", "I", int(editor::URL))
        .constant("DECIMAL", "I", int(editor::DECIMAL))
        .constant("PASSWORD", "I", int(editor::PASSWORD))
        .constant("UNEDITABLE", "I", int(editor::UNEDITABLE))
        .constant("SENSITIVE", "I", int(editor::SENSITIVE))
        .constant("NON_PREDICTIVE", "I", int(editor::NON_PREDICTIVE))
        .constant("INITIAL_CAPS_WORD", "I", int(editor::INITIAL_CAPS_WORD))
        .constant(
            "INITIAL_CAPS_SENTENCE",
            "I",
            int(editor::INITIAL_CAPS_SENTENCE),
        )
        .constant("CONSTRAINT_MASK", "I", int(editor::CONSTRAINT_MASK))
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;II)V",
            |ctx, a| {
                let item = ctx.non_null(a[0])?;
                let e = text::new_editor(ctx, a[2].obj(), a[3].int(), a[4].int())?;
                let d = data_mut(ctx, item)?;
                d.label = a[1].obj();
                d.kind = Kind::Text(e);
                ctx.recount(item);
                Ok(Value::Void)
            },
        );
    text::methods(class, access)
}
