//! The layout of a form: MIDP 2.0's rows, filled top to bottom.
//!
//! Items go left to right into a row while they fit. A new row starts
//! before an item with `LAYOUT_NEWLINE_BEFORE`, after one with
//! `LAYOUT_NEWLINE_AFTER`, when the item's horizontal alignment differs
//! from the row's, or when it does not fit. `LAYOUT_DEFAULT` keeps the
//! alignment of the item before it (left at the start). A `LAYOUT_SHRINK`
//! item starts at its minimum width and grows back towards its preferred
//! width with the row's spare room; `LAYOUT_EXPAND` items then share what
//! is left. A row's spare room goes to its alignment's side, and items
//! sit at the bottom of their row unless `LAYOUT_TOP` or
//! `LAYOUT_VCENTER` says otherwise; `LAYOUT_VEXPAND` takes the row's
//! height.

use crate::lcdui::Rect;
use crate::lcdui::highlevel::item::{
    LAYOUT_BOTTOM, LAYOUT_CENTER, LAYOUT_EXPAND, LAYOUT_LEFT, LAYOUT_NEWLINE_AFTER,
    LAYOUT_NEWLINE_BEFORE, LAYOUT_RIGHT, LAYOUT_SHRINK, LAYOUT_TOP, LAYOUT_VCENTER, LAYOUT_VEXPAND,
};

/// What one item asks of the layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Want {
    /// Its layout directives.
    pub layout: i32,
    pub min_width: i32,
    /// Its preferred width.
    pub width: i32,
}

/// Places the items in rows `width` wide, `gap` pixels apart;
/// `height_of(i, w)` gives item `i`'s height at width `w`. Returns each
/// item's box and the height of them all.
pub fn arrange<E>(
    wants: &[Want],
    width: i32,
    gap: i32,
    mut height_of: impl FnMut(usize, i32) -> Result<i32, E>,
) -> Result<(Vec<Rect>, i32), E> {
    let mut boxes = vec![Rect::new(0, 0, 0, 0); wants.len()];
    let mut row: Vec<(usize, i32)> = Vec::new();
    let (mut y, mut used) = (0, 0);
    let (mut align, mut row_align) = (LAYOUT_LEFT, LAYOUT_LEFT);
    let mut break_after = false;
    for (i, want) in wants.iter().enumerate() {
        let own = want.layout & 0x3;
        if own != 0 {
            align = own;
        }
        let start = if want.layout & LAYOUT_SHRINK != 0 {
            want.min_width
        } else {
            want.width
        };
        let w = start.clamp(0, width.max(0));
        let new_row = !row.is_empty()
            && (break_after
                || want.layout & LAYOUT_NEWLINE_BEFORE != 0
                || align != row_align
                || used + w > width);
        if new_row {
            y = finish(
                &row,
                wants,
                row_align,
                width,
                y,
                gap,
                &mut boxes,
                &mut height_of,
            )?;
            row.clear();
            used = 0;
        }
        if row.is_empty() {
            row_align = align;
        }
        row.push((i, w));
        used += w;
        break_after = want.layout & LAYOUT_NEWLINE_AFTER != 0;
    }
    if !row.is_empty() {
        y = finish(
            &row,
            wants,
            row_align,
            width,
            y,
            gap,
            &mut boxes,
            &mut height_of,
        )?;
    }
    Ok((boxes, (y - gap).max(0)))
}

/// Lays out one row at `y` and returns where the next one starts.
#[allow(clippy::too_many_arguments)] // the row and what it is placed in
fn finish<E>(
    row: &[(usize, i32)],
    wants: &[Want],
    align: i32,
    width: i32,
    y: i32,
    gap: i32,
    boxes: &mut [Rect],
    height_of: &mut impl FnMut(usize, i32) -> Result<i32, E>,
) -> Result<i32, E> {
    let mut widths: Vec<i32> = row.iter().map(|&(_, w)| w).collect();
    let mut spare = width - widths.iter().sum::<i32>();
    // Shrunk items grow back towards their preferred widths first.
    for (k, &(i, _)) in row.iter().enumerate() {
        if wants[i].layout & LAYOUT_SHRINK != 0 && spare > 0 {
            let grow = (wants[i].width.min(width) - widths[k]).clamp(0, spare);
            widths[k] += grow;
            spare -= grow;
        }
    }
    let expanding: Vec<usize> = row
        .iter()
        .enumerate()
        .filter(|(_, (i, _))| wants[*i].layout & LAYOUT_EXPAND != 0)
        .map(|(k, _)| k)
        .collect();
    if !expanding.is_empty() && spare > 0 {
        let share = spare / expanding.len() as i32;
        let extra = spare % expanding.len() as i32;
        for (n, &k) in expanding.iter().enumerate() {
            widths[k] += share + i32::from((n as i32) < extra);
        }
        spare = 0;
    }
    let mut heights = Vec::with_capacity(row.len());
    for (k, &(i, _)) in row.iter().enumerate() {
        heights.push(height_of(i, widths[k])?);
    }
    let row_height = heights.iter().copied().max().unwrap_or(0);
    let mut x = match align {
        LAYOUT_CENTER => spare.max(0) / 2,
        LAYOUT_RIGHT => spare.max(0),
        _ => 0,
    };
    for (k, &(i, _)) in row.iter().enumerate() {
        let layout = wants[i].layout;
        let h = if layout & LAYOUT_VEXPAND != 0 {
            row_height
        } else {
            heights[k]
        };
        let top = match layout & LAYOUT_VCENTER {
            LAYOUT_TOP => y,
            LAYOUT_VCENTER => y + (row_height - h) / 2,
            LAYOUT_BOTTOM | 0 => y + row_height - h,
            _ => unreachable!("two bits give four values"),
        };
        boxes[i] = Rect::new(x, top, widths[k], h);
        x += widths[k];
    }
    Ok(y + row_height + gap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcdui::highlevel::item::LAYOUT_2;

    fn want(layout: i32, min_width: i32, width: i32) -> Want {
        Want {
            layout,
            min_width,
            width,
        }
    }

    fn place(wants: &[Want], width: i32) -> (Vec<Rect>, i32) {
        let result: Result<_, ()> = arrange(wants, width, 2, |_, _| Ok(10));
        result.unwrap()
    }

    #[test]
    fn full_width_items_stack() {
        let (boxes, height) = place(&[want(0, 10, 100), want(0, 10, 100)], 100);
        assert_eq!(boxes[0], Rect::new(0, 0, 100, 10));
        assert_eq!(boxes[1], Rect::new(0, 12, 100, 10));
        assert_eq!(height, 22);
    }

    #[test]
    fn small_items_share_a_row_until_a_newline() {
        let wants = [
            want(LAYOUT_2, 5, 30),
            want(LAYOUT_2, 5, 30),
            want(LAYOUT_2 | LAYOUT_NEWLINE_BEFORE, 5, 30),
        ];
        let (boxes, _) = place(&wants, 100);
        assert_eq!(boxes[0], Rect::new(0, 0, 30, 10));
        assert_eq!(boxes[1], Rect::new(30, 0, 30, 10));
        assert_eq!(boxes[2], Rect::new(0, 12, 30, 10));
        let after = [want(LAYOUT_NEWLINE_AFTER, 5, 30), want(0, 5, 30)];
        assert_eq!(place(&after, 100).0[1].y, 12);
    }

    #[test]
    fn alignment_places_the_spare_room() {
        let (boxes, _) = place(&[want(LAYOUT_CENTER, 5, 40)], 100);
        assert_eq!(boxes[0].x, 30);
        let (boxes, _) = place(&[want(LAYOUT_RIGHT, 5, 40), want(0, 5, 20)], 100);
        assert_eq!((boxes[0].x, boxes[1].x), (40, 80), "DEFAULT keeps RIGHT");
        let (boxes, _) = place(&[want(LAYOUT_LEFT, 5, 40), want(LAYOUT_RIGHT, 5, 20)], 100);
        assert_eq!(
            boxes[1],
            Rect::new(80, 12, 20, 10),
            "a new alignment breaks"
        );
    }

    #[test]
    fn expand_and_shrink_share_the_row() {
        let wants = [want(LAYOUT_EXPAND, 5, 20), want(LAYOUT_EXPAND, 5, 20)];
        let (boxes, _) = place(&wants, 100);
        assert_eq!((boxes[0].width, boxes[1].width), (50, 50));
        let wants = [want(LAYOUT_SHRINK, 20, 80), want(0, 10, 70)];
        let (boxes, _) = place(&wants, 100);
        assert_eq!(
            boxes[0],
            Rect::new(0, 0, 30, 10),
            "grows back into the room"
        );
        assert_eq!(boxes[1].x, 30);
    }

    #[test]
    fn rows_align_items_vertically() {
        let wants = [
            want(LAYOUT_TOP, 5, 20),
            want(LAYOUT_VCENTER, 5, 20),
            want(0, 5, 20),
        ];
        let heights = [10, 20, 30];
        let result: Result<_, ()> = arrange(&wants, 100, 0, |i, _| Ok(heights[i]));
        let (boxes, height) = result.unwrap();
        assert_eq!(height, 30);
        assert_eq!((boxes[0].y, boxes[1].y, boxes[2].y), (0, 5, 0));
        let wants = [want(0, 5, 20), want(LAYOUT_VEXPAND, 5, 20), want(0, 5, 20)];
        let result: Result<_, ()> = arrange(&wants, 100, 0, |i, _| Ok(heights[i]));
        let boxes = result.unwrap().0;
        assert_eq!((boxes[0].y, boxes[1].height), (20, 30), "bottom by default");
    }

    #[test]
    fn too_wide_items_are_cut_to_the_row() {
        let (boxes, _) = place(&[want(0, 5, 500)], 100);
        assert_eq!(boxes[0].width, 100);
        assert_eq!(place(&[], 100), (Vec::new(), 0));
    }
}
