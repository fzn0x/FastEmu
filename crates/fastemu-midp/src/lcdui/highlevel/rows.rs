//! How the elements of a `List` or `ChoiceGroup` are drawn: one row
//! each, with a check box or radio button when the choice has them, the
//! element's image and its text, highlighted when it has the focus.

use fastemu_gfx::Font;
use fastemu_jvm::{Ctx, JResult, ObjRef};

use super::choice::{ChoiceModel, MULTIPLE};
use super::draw::{Fonts, Painter, ellipsize, wrap};
use crate::lcdui::Rect;
use crate::lcdui::font::FontData;
use crate::lcdui::image::surface_of;

/// The tallest an element's image makes its row.
const MAX_IMAGE: i32 = 48;

/// One element as drawn: its text, font and image size.
pub struct Row {
    pub text: Vec<u16>,
    pub font: Font,
    pub image: Option<ObjRef>,
    pub image_size: (i32, i32),
    pub selected: bool,
}

/// The elements of `model` ready to measure and draw.
pub fn rows(ctx: &Ctx<'_>, model: &ChoiceModel, fonts: &Fonts) -> JResult<Vec<Row>> {
    let mut rows = Vec::with_capacity(model.len());
    for e in &model.elements {
        let font = match e.font {
            Some(f) => ctx.payload::<FontData>(f)?.0.clone(),
            None => fonts.text.clone(),
        };
        let image_size = match e.image {
            Some(image) => {
                let s = surface_of(ctx, image)?;
                let s = s.lock();
                (s.width() as i32, (s.height() as i32).min(MAX_IMAGE))
            }
            None => (0, 0),
        };
        rows.push(Row {
            text: ctx.string_utf16(e.text)?.to_vec(),
            font,
            image: e.image,
            image_size,
            selected: e.selected,
        });
    }
    Ok(rows)
}

/// The side of a check box or radio button for `font`.
pub fn mark_side(font: &Font) -> i32 {
    (font.height() - 2).max(8)
}

/// Where a row's text starts after its mark and image.
fn text_left(row: &Row, mark: i32) -> i32 {
    let mut x = 2;
    if mark > 0 {
        x += mark + 4;
    }
    if row.image.is_some() {
        x += row.image_size.0 + 3;
    }
    x
}

/// The height of a row `width` wide.
pub fn row_height(row: &Row, width: i32, mark: i32, wrap_text: bool) -> i32 {
    let text_width = width - text_left(row, mark) - 2;
    let lines = if wrap_text {
        wrap(&row.font, &row.text, text_width).len() as i32
    } else {
        1
    };
    (lines * row.font.height()).max(row.image_size.1).max(mark) + 4
}

/// Draws one row in `r`: highlighted when it has the focus, with its mark
/// (`mark` wide, none when 0; `kind` says which), image and text.
#[allow(clippy::too_many_arguments)] // the parts of one row
pub fn paint_row(
    ctx: &Ctx<'_>,
    p: &mut Painter<'_>,
    row: &Row,
    r: Rect,
    kind: i32,
    mark: i32,
    focused: bool,
    wrap_text: bool,
) -> JResult<()> {
    let theme = *p.theme;
    let ink = if focused {
        p.round(r, theme.highlighted_background);
        theme.highlighted_foreground
    } else {
        theme.foreground
    };
    let mut x = r.x + 2;
    if mark > 0 {
        let y = r.y + (r.height - mark) / 2;
        if kind == MULTIPLE {
            p.check_box(x, y, mark, row.selected, ink);
        } else {
            p.radio(x, y, mark, row.selected, ink);
        }
        x += mark + 4;
    }
    if let Some(image) = row.image {
        let surface = surface_of(ctx, image)?;
        let pixels = surface.lock();
        let y = r.y + (r.height - row.image_size.1) / 2;
        p.clip(Rect::new(x, r.y + 2, row.image_size.0, r.height - 4).intersect(r));
        p.image(&pixels, x, y);
        p.unclip();
        x += row.image_size.0 + 3;
    }
    let text_width = r.x + r.width - x - 2;
    let lines = if wrap_text {
        wrap(&row.font, &row.text, text_width).len() as i32
    } else {
        1
    };
    let y = r.y + (r.height - lines * row.font.height()) / 2;
    if wrap_text {
        p.paragraph(&row.font, &row.text, x, y, text_width, ink);
    } else {
        let cut = ellipsize(&row.font, &row.text, text_width);
        p.text(&row.font, &cut, x, y, ink);
    }
    Ok(())
}
