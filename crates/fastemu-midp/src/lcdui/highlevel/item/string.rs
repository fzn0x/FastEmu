//! `javax.microedition.lcdui.StringItem`: text in a form, plain, as a
//! hyperlink or as a button.

use fastemu_gfx::Font;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value};

use super::{BUTTON, HYPERLINK, ItemData, Kind, Needs, PLAIN, changed, data, data_mut, units};
use crate::lcdui::Rect;
use crate::lcdui::font::{default_font, font_object, font_of};
use crate::lcdui::highlevel::draw::{Fonts, Painter, width, wrap};

const CLASS: &str = "javax/microedition/lcdui/StringItem";

/// The space between a button's frame and its text.
const BUTTON_PAD: i32 = 3;

/// What a `StringItem` keeps.
#[derive(Debug, Default)]
pub struct StringState {
    pub text: Option<ObjRef>,
    pub font: Option<ObjRef>,
    pub appearance: i32,
}

impl StringState {
    pub(super) fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.text);
        tracer.mark_opt(self.font);
    }
}

fn state<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a StringState> {
    match &data(ctx, item)?.kind {
        Kind::String(s) => Ok(s),
        _ => unreachable!("a StringItem's payload is made by its class"),
    }
}

fn state_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut StringState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::String(s) => Ok(s),
        _ => unreachable!("a StringItem's payload is made by its class"),
    }
}

/// The text and font to draw, without one line feed at the end, which
/// only ends the line.
fn text_and_font(ctx: &Ctx<'_>, item: ObjRef, fonts: &Fonts) -> JResult<(Vec<u16>, Font)> {
    let s = state(ctx, item)?;
    let mut text = units(ctx, s.text)?;
    if text.last() == Some(&u16::from(b'\n')) {
        text.pop();
    }
    let font = match s.font {
        Some(f) => ctx.payload::<crate::lcdui::font::FontData>(f)?.0.clone(),
        None => fonts.text.clone(),
    };
    Ok((text, font))
}

/// The text's lines and size at most `max` wide.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, max: i32) -> JResult<Needs> {
    let (text, font) = text_and_font(ctx, item, fonts)?;
    let button = state(ctx, item)?.appearance == BUTTON;
    let pad = if button { BUTTON_PAD + 1 } else { 0 };
    let min_width = (font.char_width(u16::from(b'W')) * 3 + 2 * pad).min(max);
    if text.is_empty() {
        return Ok(Needs {
            min_width: 0,
            width: if button { 2 * pad } else { 0 },
            height: if button { font.height() + 2 * pad } else { 0 },
        });
    }
    let lines = wrap(&font, &text, max - 2 * pad);
    let widest = lines
        .iter()
        .map(|l| width(&font, &text[l.clone()]))
        .max()
        .unwrap_or(0);
    Ok(Needs {
        min_width,
        width: widest + 2 * pad,
        height: lines.len() as i32 * font.height() + 2 * pad,
    })
}

/// Paints the text: a hyperlink underlined in the highlight colour, a
/// button in a rounded frame.
pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let (text, font) = text_and_font(ctx, item, &p.fonts)?;
    let theme = *p.theme;
    match state(ctx, item)?.appearance {
        BUTTON => {
            let (fill, ink) = if focused {
                (theme.highlighted_background, theme.highlighted_foreground)
            } else {
                (
                    super::super::draw::blend(theme.background, theme.border),
                    theme.foreground,
                )
            };
            let size = needs_box(&font, &text, area.width);
            let r = Rect::new(
                area.x,
                area.y,
                size.0.min(area.width),
                size.1.min(area.height),
            );
            p.round(r, fill);
            p.frame(r, theme.border, 1);
            let pad = BUTTON_PAD + 1;
            p.paragraph(&font, &text, r.x + pad, r.y + pad, r.width - 2 * pad, ink);
        }
        HYPERLINK => {
            let color = theme.highlighted_border;
            for (i, line) in wrap(&font, &text, area.width).into_iter().enumerate() {
                let y = area.y + i as i32 * font.height();
                let units = &text[line];
                p.text(&font, units, area.x, y, color);
                p.underline(&font, area.x, y, width(&font, units), color);
            }
        }
        _ => {
            p.paragraph(&font, &text, area.x, area.y, area.width, theme.foreground);
        }
    }
    Ok(())
}

/// The outer size of a button holding `text` at most `max` wide.
fn needs_box(font: &Font, text: &[u16], max: i32) -> (i32, i32) {
    let pad = BUTTON_PAD + 1;
    let lines = wrap(font, text, max - 2 * pad);
    let widest = lines
        .iter()
        .map(|l| width(font, &text[l.clone()]))
        .max()
        .unwrap_or(0);
    (
        widest + 2 * pad,
        lines.len() as i32 * font.height() + 2 * pad,
    )
}

fn init(ctx: &mut Ctx<'_>, a: &[Value], appearance: i32) -> JResult<Value> {
    if !(PLAIN..=BUTTON).contains(&appearance) {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "invalid appearance mode",
        );
    }
    let item = ctx.non_null(a[0])?;
    let d = data_mut(ctx, item)?;
    d.label = a[1].obj();
    d.kind = Kind::String(StringState {
        text: a[2].obj(),
        font: None,
        appearance,
    });
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| Box::new(ItemData::new(Kind::String(StringState::default()))))
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            |ctx, a| init(ctx, a, PLAIN),
        )
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;I)V",
            |ctx, a| init(ctx, a, a[3].int()),
        )
        .method("getText", "()Ljava/lang/String;", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(state(ctx, item)?.text.into())
        })
        .method("setText", "(Ljava/lang/String;)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            state_mut(ctx, item)?.text = a[1].obj();
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getAppearanceMode", "()I", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::Int(state(ctx, item)?.appearance))
        })
        .method("setFont", "(Ljavax/microedition/lcdui/Font;)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            if let Some(font) = a[1].obj() {
                // Check it is a Font before keeping it.
                font_of(ctx, font)?;
            }
            state_mut(ctx, item)?.font = a[1].obj();
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getFont", "()Ljavax/microedition/lcdui/Font;", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            match state(ctx, item)?.font {
                Some(font) => Ok(font.into()),
                None => {
                    let font = default_font(ctx);
                    Ok(font_object(ctx, &font)?.into())
                }
            }
        })
        .method("setPreferredSize", "(II)V", super::set_preferred_size)
}
