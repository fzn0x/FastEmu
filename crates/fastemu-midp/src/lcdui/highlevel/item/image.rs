//! `javax.microedition.lcdui.ImageItem`: a picture in a form, with the
//! text shown when it has none.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value};

use super::{BUTTON, ItemData, Kind, Needs, PLAIN, changed, data, data_mut, units, valid_layout};
use crate::lcdui::Rect;
use crate::lcdui::highlevel::draw::{Fonts, Painter, width, wrap};
use crate::lcdui::image::surface_of;

const CLASS: &str = "javax/microedition/lcdui/ImageItem";

/// What an `ImageItem` keeps.
#[derive(Debug, Default)]
pub struct ImageState {
    pub image: Option<ObjRef>,
    pub alt: Option<ObjRef>,
    pub appearance: i32,
}

impl ImageState {
    pub(super) fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.image);
        tracer.mark_opt(self.alt);
    }
}

fn state<'a>(ctx: &'a Ctx<'_>, item: ObjRef) -> JResult<&'a ImageState> {
    match &data(ctx, item)?.kind {
        Kind::Image(s) => Ok(s),
        _ => unreachable!("an ImageItem's payload is made by its class"),
    }
}

fn state_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut ImageState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Image(s) => Ok(s),
        _ => unreachable!("an ImageItem's payload is made by its class"),
    }
}

/// The image's size, or what the alternative text needs.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, max: i32) -> JResult<Needs> {
    let s = state(ctx, item)?;
    let frame = if s.appearance == BUTTON { 2 } else { 0 };
    if let Some(image) = s.image {
        let surface = surface_of(ctx, image)?;
        let (w, h) = {
            let s = surface.lock();
            (s.width() as i32, s.height() as i32)
        };
        return Ok(Needs {
            min_width: (w + 2 * frame).min(max),
            width: w + 2 * frame,
            height: h + 2 * frame,
        });
    }
    let alt = units(ctx, s.alt)?;
    if alt.is_empty() {
        return Ok(Needs {
            min_width: 0,
            width: 0,
            height: 0,
        });
    }
    let lines = wrap(&fonts.small, &alt, max);
    Ok(Needs {
        min_width: 0,
        width: width(&fonts.small, &alt).min(max),
        height: lines.len() as i32 * fonts.small.height(),
    })
}

/// Paints the image at the top left of `area`, or the alternative text.
pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let s = state(ctx, item)?;
    let button = s.appearance == BUTTON;
    let frame = if button { 2 } else { 0 };
    match s.image {
        Some(image) => {
            let surface = surface_of(ctx, image)?;
            let pixels = surface.lock();
            if button {
                let r = Rect::new(
                    area.x,
                    area.y,
                    pixels.width() as i32 + 4,
                    pixels.height() as i32 + 4,
                );
                let theme = *p.theme;
                let color = if focused {
                    theme.highlighted_border
                } else {
                    theme.border
                };
                p.frame(r, color, 1);
            }
            p.image(&pixels, area.x + frame, area.y + frame);
        }
        None => {
            let alt = units(ctx, s.alt)?;
            let font = p.fonts.small.clone();
            let color = p.theme.border;
            p.paragraph(&font, &alt, area.x, area.y, area.width, color);
        }
    }
    Ok(())
}

fn init(ctx: &mut Ctx<'_>, a: &[Value], appearance: i32) -> JResult<Value> {
    let layout = a[3].int();
    if !valid_layout(layout) {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "invalid layout directives",
        );
    }
    if !(PLAIN..=BUTTON).contains(&appearance) {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "invalid appearance mode",
        );
    }
    let item = ctx.non_null(a[0])?;
    let d = data_mut(ctx, item)?;
    d.label = a[1].obj();
    d.layout = layout;
    d.kind = Kind::Image(ImageState {
        image: a[2].obj(),
        alt: a[4].obj(),
        appearance,
    });
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    let image = "Ljavax/microedition/lcdui/Image;";
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| Box::new(ItemData::new(Kind::Image(ImageState::default()))))
        .constant("LAYOUT_DEFAULT", "I", int(0))
        .constant("LAYOUT_LEFT", "I", int(super::LAYOUT_LEFT))
        .constant("LAYOUT_RIGHT", "I", int(super::LAYOUT_RIGHT))
        .constant("LAYOUT_CENTER", "I", int(super::LAYOUT_CENTER))
        .constant(
            "LAYOUT_NEWLINE_BEFORE",
            "I",
            int(super::LAYOUT_NEWLINE_BEFORE),
        )
        .constant(
            "LAYOUT_NEWLINE_AFTER",
            "I",
            int(super::LAYOUT_NEWLINE_AFTER),
        )
        .method(
            "<init>",
            &format!("(Ljava/lang/String;{image}ILjava/lang/String;)V"),
            |ctx, a| init(ctx, a, PLAIN),
        )
        .method(
            "<init>",
            &format!("(Ljava/lang/String;{image}ILjava/lang/String;I)V"),
            |ctx, a| init(ctx, a, a[5].int()),
        )
        .method("getImage", &format!("(){image}"), |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(state(ctx, item)?.image.into())
        })
        .method("setImage", &format!("({image})V"), |ctx, a| {
            let item = ctx.non_null(a[0])?;
            state_mut(ctx, item)?.image = a[1].obj();
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getAltText", "()Ljava/lang/String;", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(state(ctx, item)?.alt.into())
        })
        .method("setAltText", "(Ljava/lang/String;)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            state_mut(ctx, item)?.alt = a[1].obj();
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getLayout", "()I", super::get_layout)
        .method("setLayout", "(I)V", super::set_layout)
        .method("getAppearanceMode", "()I", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::Int(state(ctx, item)?.appearance))
        })
}
