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

/// The most units of an element's text a row keeps to measure and draw.
/// The text is longer than any row shows: a line that does not wrap is
/// cut to the width of the screen, and a paragraph this long fills many
/// screens.
const ROW_UNITS_MAX: usize = 4096;

/// The fewest units of text a row keeps, however many rows there are.
const ROW_UNITS_MIN: usize = 128;

/// The units of text all the rows of one choice keep between them, as far
/// as [`ROW_UNITS_MIN`] allows. Elements may share one long string, which
/// the heap counts once, but each row copies its text: without a limit a
/// choice of many elements that name one large string takes gigabytes
/// each time it is drawn.
const ROW_UNITS_BUDGET: usize = 1 << 20;

/// One element as drawn: its text, font and image size.
pub struct Row {
    pub text: Vec<u16>,
    pub font: Font,
    pub image: Option<ObjRef>,
    pub image_size: (i32, i32),
    pub selected: bool,
}

/// The elements of `model` ready to measure and draw. A row keeps the
/// start of a text that is long, as far as the budget of the choice
/// allows.
pub fn rows(ctx: &Ctx<'_>, model: &ChoiceModel, fonts: &Fonts) -> JResult<Vec<Row>> {
    let keep = (ROW_UNITS_BUDGET / model.len().max(1)).clamp(ROW_UNITS_MIN, ROW_UNITS_MAX);
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
        let text = ctx.string_utf16(e.text)?;
        rows.push(Row {
            text: text[..text.len().min(keep)].to_vec(),
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_core::FontSizes;
    use fastemu_gfx::{FontLibrary, TextRendering};
    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;

    #[test]
    fn elements_that_share_a_long_string_do_not_copy_it_each() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            Vec::new(),
        );
        vm.attach("test", |ctx| {
            let sizes = FontSizes {
                small: 12,
                medium: 14,
                large: 16,
            };
            let fonts = Fonts::new(&FontLibrary::new(sizes, TextRendering::Antialiased));
            let long = ctx.new_string(&"a".repeat(50_000))?;
            let short = ctx.new_string("short")?;

            let mut many = ChoiceModel::new(MULTIPLE);
            for _ in 0..500 {
                let at = many.len();
                many.insert(at, long, None);
            }
            let all = rows(ctx, &many, &fonts)?;
            assert_eq!(all.len(), 500);
            let copied: usize = all.iter().map(|r| r.text.len()).sum();
            assert!(copied <= 2 << 20, "{copied} units copied for 500 rows");

            let mut few = ChoiceModel::new(MULTIPLE);
            few.insert(0, long, None);
            few.insert(1, short, None);
            let all = rows(ctx, &few, &fonts)?;
            assert!(all[0].text.len() >= 1024, "a few rows keep much text");
            assert_eq!(all[1].text, "short".encode_utf16().collect::<Vec<u16>>());
            JResult::Ok(())
        })
        .expect("the test ran");
    }
}
