//! Text entry, shared by `TextBox` and `TextField`: the editor, the Java
//! methods both classes declare, and what the keypad and keyboard do.

pub mod editor;

use std::ops::Range;
use std::time::Instant;

use fastemu_gfx::Font;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Throw, Value};

pub use editor::{EditError, InputMode, TextEditor};

use super::draw::{self, Painter, wrap};
use crate::env::env;
use crate::lcdui::Rect;
use crate::lcdui::screen::{KeyEvent, KeyPhase};

/// How the shared methods reach an object's editor.
#[derive(Clone, Copy)]
pub(crate) struct Access {
    /// The editor of a `TextBox` or `TextField`.
    pub editor: for<'a, 'b> fn(&'a mut Ctx<'b>, ObjRef) -> JResult<&'a mut TextEditor>,
    /// Runs after the API changed the object: repaints it, or lays out
    /// the form that holds it.
    pub changed: fn(&mut Ctx<'_>, ObjRef) -> JResult<()>,
}

/// The exception an [`EditError`] names.
pub(crate) fn throw(ctx: &mut Ctx<'_>, error: EditError) -> Throw {
    match error {
        EditError::IllegalArgument(why) => {
            ctx.new_exception("java/lang/IllegalArgumentException", Some(why))
        }
        EditError::StringIndex => ctx.new_exception(
            "java/lang/StringIndexOutOfBoundsException",
            Some("offset and length do not name a range of the text"),
        ),
    }
}

fn array_index(ctx: &mut Ctx<'_>) -> Throw {
    ctx.new_exception(
        "java/lang/ArrayIndexOutOfBoundsException",
        Some("offset and length do not name a range of the array"),
    )
}

/// Applies an API edit and reports it, or throws what it was refused for.
fn edit<R>(
    ctx: &mut Ctx<'_>,
    this: ObjRef,
    access: Access,
    f: impl FnOnce(&mut TextEditor) -> Result<R, EditError>,
) -> JResult<R> {
    let result = f((access.editor)(ctx, this)?);
    match result {
        Ok(value) => {
            (access.changed)(ctx, this)?;
            Ok(value)
        }
        Err(e) => Err(throw(ctx, e)),
    }
}

/// The units of `chars[offset..offset + length]`, or the
/// `ArrayIndexOutOfBoundsException` a bad range is.
fn char_range(ctx: &mut Ctx<'_>, chars: ObjRef, offset: i32, length: i32) -> JResult<Vec<u16>> {
    let len = ctx.array_len(chars)? as i64;
    let (offset, length) = (i64::from(offset), i64::from(length));
    if offset < 0 || length < 0 || offset + length > len {
        return Err(array_index(ctx));
    }
    let slice = ctx.array::<u16>(chars)?;
    Ok(slice[offset as usize..(offset + length) as usize].to_vec())
}

/// Adds the methods `TextBox` and `TextField` share to `class`.
pub(crate) fn methods(class: NativeClass, access: Access) -> NativeClass {
    class
        .method("getString", "()Ljava/lang/String;", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let text = (access.editor)(ctx, this)?.text().to_vec();
            Ok(ctx.new_string_utf16(text)?.into())
        })
        .method("setString", "(Ljava/lang/String;)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let text = match a[1].obj() {
                Some(s) => ctx.string_utf16(s)?.to_vec(),
                None => Vec::new(),
            };
            edit(ctx, this, access, |e| e.set_text(&text)).map(|()| Value::Void)
        })
        .method("getChars", "([C)I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let data = ctx.non_null(a[1])?;
            let text = (access.editor)(ctx, this)?.text().to_vec();
            if ctx.array_len(data)? < text.len() {
                return Err(array_index(ctx));
            }
            ctx.array_mut::<u16>(data)?[..text.len()].copy_from_slice(&text);
            Ok(Value::Int(text.len() as i32))
        })
        .method("setChars", "([CII)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let text = match a[1].obj() {
                Some(data) => char_range(ctx, data, a[2].int(), a[3].int())?,
                None => Vec::new(),
            };
            edit(ctx, this, access, |e| e.set_text(&text)).map(|()| Value::Void)
        })
        .method("insert", "(Ljava/lang/String;I)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let src = ctx.non_null(a[1])?;
            let src = ctx.string_utf16(src)?.to_vec();
            let position = a[2].int();
            edit(ctx, this, access, |e| e.insert(&src, position)).map(|()| Value::Void)
        })
        .method("insert", "([CIII)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let data = ctx.non_null(a[1])?;
            let src = char_range(ctx, data, a[2].int(), a[3].int())?;
            let position = a[4].int();
            edit(ctx, this, access, |e| e.insert(&src, position)).map(|()| Value::Void)
        })
        .method("delete", "(II)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let (offset, length) = (a[1].int(), a[2].int());
            edit(ctx, this, access, |e| e.delete(offset, length)).map(|()| Value::Void)
        })
        .method("getMaxSize", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.editor)(ctx, this)?.max_size() as i32))
        })
        .method("setMaxSize", "(I)I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let max = a[1].int();
            edit(ctx, this, access, |e| e.set_max_size(max)).map(Value::Int)
        })
        .method("size", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.editor)(ctx, this)?.text().len() as i32))
        })
        .method("getCaretPosition", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.editor)(ctx, this)?.caret() as i32))
        })
        .method("setConstraints", "(I)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let constraints = a[1].int();
            edit(ctx, this, access, |e| e.set_constraints(constraints)).map(|()| Value::Void)
        })
        .method("getConstraints", "()I", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int((access.editor)(ctx, this)?.constraints()))
        })
        .method(
            "setInitialInputMode",
            "(Ljava/lang/String;)V",
            move |ctx, a| {
                let this = ctx.non_null(a[0])?;
                let subset = ctx.opt_string(a[1])?;
                (access.editor)(ctx, this)?.set_initial_mode(subset.as_deref());
                (access.changed)(ctx, this)?;
                Ok(Value::Void)
            },
        )
}

/// What a key did to an editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyUse {
    /// Not a key for the editor.
    Ignored,
    /// Moved the caret or changed the mode; the text is the same.
    Moved,
    /// Changed the text.
    Edited,
}

/// A key on an editor: the keypad's digits, `*` and `#` type by
/// multi-tap, clear deletes, and left and right (the arrow keys only, so
/// `4` and `6` still type) move the caret.
pub(crate) fn key(ctx: &Ctx<'_>, editor: &mut TextEditor, event: KeyEvent) -> KeyUse {
    if event.phase == KeyPhase::Released {
        return KeyUse::Ignored;
    }
    let codes = env(ctx).profile.key_codes;
    let code = event.code;
    if code == codes.clear {
        return if editor.backspace() {
            KeyUse::Edited
        } else {
            KeyUse::Ignored
        };
    }
    if code == codes.left || code == codes.right {
        if !editor.editable() {
            return KeyUse::Ignored;
        }
        let before = editor.caret();
        editor.move_caret(if code == codes.left { -1 } else { 1 });
        return if editor.caret() == before {
            KeyUse::Ignored
        } else {
            KeyUse::Moved
        };
    }
    let Ok(unit) = u16::try_from(code) else {
        return KeyUse::Ignored;
    };
    let typed = matches!(u8::try_from(unit), Ok(b'0'..=b'9' | b'*' | b'#'));
    if !typed || !editor.editable() {
        return KeyUse::Ignored;
    }
    if event.phase == KeyPhase::Repeated {
        // A held key types once; its repeats are not traversal either.
        return KeyUse::Moved;
    }
    let before = editor.text().to_vec();
    if !editor.key(unit, Instant::now()) {
        // A refused key still belongs to the editor.
        return KeyUse::Moved;
    }
    if editor.text() == before.as_slice() {
        KeyUse::Moved
    } else {
        KeyUse::Edited
    }
}

/// The inset of text inside its frame.
pub(crate) const INSET: i32 = 3;

/// The lines `editor`'s text takes at `width` pixels, and the line and
/// x offset of its caret.
pub(crate) fn lines(
    font: &Font,
    editor: &TextEditor,
    width: i32,
) -> (Vec<Range<usize>>, usize, i32) {
    let shown = display_text(editor);
    let lines = wrap(font, &shown, width);
    let caret = editor.caret();
    let line = lines
        .iter()
        .position(|l| caret >= l.start && caret <= l.end)
        .unwrap_or(lines.len() - 1);
    let start = lines[line].start.min(caret);
    let x = draw::width(font, &shown[start..caret]);
    (lines, line, x)
}

/// Draws `editor`'s text in a frame filling `area`, from line `first`,
/// with the caret when `focused`. The letter multi-tap is still choosing
/// is underlined.
pub(crate) fn paint_box(
    p: &mut Painter<'_>,
    font: &Font,
    editor: &TextEditor,
    area: Rect,
    focused: bool,
    first: usize,
) {
    let theme = *p.theme;
    let editable = editor.editable();
    let fill = if editable {
        theme.background
    } else {
        draw::blend(
            theme.background,
            draw::blend(theme.background, theme.border),
        )
    };
    p.fill(area, fill);
    let (border, thickness) = if focused {
        (theme.highlighted_border, 2)
    } else {
        (theme.border, 1)
    };
    p.frame(area, border, thickness);
    let inner = Rect::new(
        area.x + INSET,
        area.y + INSET - 1,
        area.width - 2 * INSET,
        area.height - 2 * INSET + 2,
    );
    p.clip(inner);
    let shown = display_text(editor);
    let (lines, caret_line, caret_x) = lines(font, editor, inner.width);
    let pending = editor.pending_at();
    for (row, line) in lines.iter().enumerate().skip(first) {
        let y = inner.y + (row - first) as i32 * font.height();
        if y >= inner.y + inner.height {
            break;
        }
        p.text(font, &shown[line.clone()], inner.x, y, theme.foreground);
        if let Some(at) = pending.filter(|at| line.contains(at)) {
            let x = inner.x + draw::width(font, &shown[line.start..at]);
            let w = font.char_width(shown[at]);
            p.underline(font, x, y, w, theme.highlighted_border);
        }
    }
    if focused && editable && caret_line >= first {
        let y = inner.y + (caret_line - first) as i32 * font.height();
        let x = inner.x + caret_x;
        p.fill(Rect::new(x, y, 1, font.height()), theme.foreground);
    }
    p.unclip();
}

/// What the text box or field shows: the text, or a star for each
/// character of a password but the one multi-tap is still choosing.
pub(crate) fn display_text(editor: &TextEditor) -> Vec<u16> {
    if !editor.masked() {
        return editor.text().to_vec();
    }
    let open = editor.pending_at();
    editor
        .text()
        .iter()
        .enumerate()
        .map(|(i, &u)| if Some(i) == open { u } else { u16::from(b'*') })
        .collect()
}
