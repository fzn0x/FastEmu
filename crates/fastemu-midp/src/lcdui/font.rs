//! `javax.microedition.lcdui.Font` over the bundled fonts of
//! `fastemu-gfx`, sized from the device profile.
//!
//! One `Font` object stands for each face, style and size, so `getFont`
//! returns the same object every time, as phones did.

use std::sync::Arc;

use fastemu_gfx::Font;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Value, classfile::access};

use super::state::state;
use super::throw_gfx;

const CLASS: &str = "javax/microedition/lcdui/Font";

/// What a Java `Font` object holds.
pub struct FontData(pub Font);

impl Payload for FontData {}

/// `Font.FONT_STATIC_TEXT`.
const FONT_STATIC_TEXT: i32 = 0;
/// `Font.FONT_INPUT_TEXT`.
const FONT_INPUT_TEXT: i32 = 1;

/// The Java object for `font`, made once per face, style and size.
pub fn font_object(ctx: &mut Ctx<'_>, font: &Font) -> JResult<ObjRef> {
    let key = (
        font.face().to_midp(),
        font.style().to_midp(),
        font.size().to_midp(),
    );
    if let Some(global) = state(ctx).font_objects.get(&key).cloned() {
        return Ok(global.get(ctx));
    }
    let class = ctx.find_class(CLASS)?;
    let obj = ctx.new_object(class)?;
    ctx.set_payload(obj, FontData(font.clone()));
    let global = Arc::new(ctx.new_global(obj));
    state(ctx).font_objects.insert(key, global);
    Ok(obj)
}

/// The font a Java `Font` object stands for.
pub fn font_of(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Font> {
    Ok(ctx.payload::<FontData>(obj)?.0.clone())
}

/// The instance's default font, `Font.getDefaultFont()`.
pub fn default_font(ctx: &mut Ctx<'_>) -> Font {
    state(ctx).fonts.default_font()
}

fn this_font(ctx: &mut Ctx<'_>, this: Value) -> JResult<Font> {
    let this = ctx.non_null(this)?;
    font_of(ctx, this)
}

/// The text of a `String` argument, throwing `NullPointerException` for
/// `null`.
fn units(ctx: &mut Ctx<'_>, value: Value) -> JResult<Vec<u16>> {
    let s = ctx.non_null(value)?;
    Ok(ctx.string_utf16(s)?.to_vec())
}

fn int_query(f: fn(&Font) -> i32) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| Ok(Value::Int(f(&this_font(ctx, a[0])?)))
}

fn bool_query(f: fn(&Font) -> bool) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> {
    move |ctx, a| Ok(Value::from(f(&this_font(ctx, a[0])?)))
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .final_class()
        .constant("STYLE_PLAIN", "I", int(0))
        .constant("STYLE_BOLD", "I", int(1))
        .constant("STYLE_ITALIC", "I", int(2))
        .constant("STYLE_UNDERLINED", "I", int(4))
        .constant("SIZE_SMALL", "I", int(8))
        .constant("SIZE_MEDIUM", "I", int(0))
        .constant("SIZE_LARGE", "I", int(16))
        .constant("FACE_SYSTEM", "I", int(0))
        .constant("FACE_MONOSPACE", "I", int(32))
        .constant("FACE_PROPORTIONAL", "I", int(64))
        .constant("FONT_STATIC_TEXT", "I", int(FONT_STATIC_TEXT))
        .constant("FONT_INPUT_TEXT", "I", int(FONT_INPUT_TEXT))
        .static_method(
            "getDefaultFont",
            "()Ljavax/microedition/lcdui/Font;",
            |ctx, _| {
                let font = default_font(ctx);
                Ok(font_object(ctx, &font)?.into())
            },
        )
        .static_method("getFont", "(I)Ljavax/microedition/lcdui/Font;", |ctx, a| {
            let font = state(ctx).fonts.get_font_for(a[0].int());
            match font {
                Ok(font) => Ok(font_object(ctx, &font)?.into()),
                Err(e) => Err(throw_gfx(ctx, &e)),
            }
        })
        .static_method(
            "getFont",
            "(III)Ljavax/microedition/lcdui/Font;",
            |ctx, a| {
                let font = state(ctx)
                    .fonts
                    .get_font(a[0].int(), a[1].int(), a[2].int());
                match font {
                    Ok(font) => Ok(font_object(ctx, &font)?.into()),
                    Err(e) => Err(throw_gfx(ctx, &e)),
                }
            },
        )
        .method("getStyle", "()I", int_query(|f| f.style().to_midp()))
        .method("getSize", "()I", int_query(|f| f.size().to_midp()))
        .method("getFace", "()I", int_query(|f| f.face().to_midp()))
        .method("isPlain", "()Z", bool_query(Font::is_plain))
        .method("isBold", "()Z", bool_query(Font::is_bold))
        .method("isItalic", "()Z", bool_query(Font::is_italic))
        .method("isUnderlined", "()Z", bool_query(Font::is_underlined))
        .method("getHeight", "()I", int_query(Font::height))
        .method(
            "getBaselinePosition",
            "()I",
            int_query(Font::baseline_position),
        )
        .method("charWidth", "(C)I", |ctx, a| {
            let font = this_font(ctx, a[0])?;
            Ok(Value::Int(font.char_width(a[1].char())))
        })
        .method("charsWidth", "([CII)I", |ctx, a| {
            let font = this_font(ctx, a[0])?;
            let chars = ctx.non_null(a[1])?;
            let result = font.chars_width(ctx.array::<u16>(chars)?, a[2].int(), a[3].int());
            result.map(Value::Int).map_err(|e| throw_gfx(ctx, &e))
        })
        .method("stringWidth", "(Ljava/lang/String;)I", |ctx, a| {
            let font = this_font(ctx, a[0])?;
            let text = units(ctx, a[1])?;
            Ok(Value::Int(font.string_width(&text)))
        })
        .method_with(
            "substringWidth",
            "(Ljava/lang/String;II)I",
            access::PUBLIC,
            |ctx, a| {
                let font = this_font(ctx, a[0])?;
                let text = units(ctx, a[1])?;
                let result = font.substring_width(&text, a[2].int(), a[3].int());
                result.map(Value::Int).map_err(|e| throw_gfx(ctx, &e))
            },
        )
}
