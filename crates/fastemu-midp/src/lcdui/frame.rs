//! Presenting frames: composing the shown displayable with the parts the
//! runtime draws (title, ticker, soft key bar, options menu) and handing
//! the result to the app, and holding game threads to the frame rate.

use std::time::Instant;

use fastemu_core::Frame;
use fastemu_gfx::anchor::{HCENTER, LEFT, RIGHT, TOP};
use fastemu_gfx::{Font, Graphics, Surface};
use fastemu_jvm::{Ctx, JResult, ObjRef, Wake};

use super::canvas::buffer;
use super::displayable::{self, Layout};
use super::softkeys;
use super::state::{Rect, current, state};
use super::theme::Theme;
use crate::env::env;

/// The runtime's own parts of a frame, read from the VM before drawing.
struct Chrome {
    layout: Layout,
    title: Option<Vec<u16>>,
    ticker: Option<Vec<u16>>,
    left: Vec<u16>,
    right: Vec<u16>,
    menu: Option<softkeys::MenuView>,
}

/// Composes and presents a frame of the shown displayable. Called after
/// every paint, flush and change to the runtime's own parts.
pub(crate) fn present(ctx: &mut Ctx<'_>) -> JResult<()> {
    let host = env(ctx).host.clone();
    let shown = current(ctx);
    let serial = {
        let st = state(ctx);
        st.frame_serial += 1;
        st.last_present = Some(Instant::now());
        st.frame_serial
    };
    let Some(obj) = shown else {
        let st = state(ctx);
        let (w, h, bg) = (st.screen_width, st.screen_height, st.theme.background);
        host.present(Frame::filled(
            w as u32,
            h as u32,
            0xFF00_0000 | bg as u32,
            serial,
        ));
        return Ok(());
    };
    let pixels = buffer(ctx, obj)?;
    let chrome = read_chrome(ctx, obj)?;
    let whole = {
        let st = state(ctx);
        Rect::new(0, 0, st.screen_width, st.screen_height)
    };
    if chrome.layout.content == whole && chrome.layout.bar.is_none() && chrome.menu.is_none() {
        host.present(pixels.lock().to_frame(serial));
        return Ok(());
    }
    let st = state(ctx);
    let font = st.fonts.default_font();
    let theme = st.theme;
    let composed = match &mut st.composed {
        Some(surface) => surface,
        slot => slot.insert(
            Surface::screen(whole.width as u32, whole.height as u32)
                .expect("the screen size was checked when the state was made"),
        ),
    };
    copy_content(composed, &pixels.lock(), chrome.layout.content);
    draw_chrome(composed, &chrome, &font, &theme);
    host.present(composed.to_frame(serial));
    Ok(())
}

fn read_chrome(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Chrome> {
    let layout = displayable::layout(ctx, obj)?;
    let d = displayable::data(ctx, obj)?;
    let (title, ticker) = (d.title(), d.ticker());
    let title = match (layout.title, title) {
        (Some(_), Some(t)) => Some(ctx.string_utf16(t)?.to_vec()),
        _ => None,
    };
    let ticker = match (layout.ticker, ticker) {
        (Some(_), Some(t)) => {
            let t = ctx.root(t);
            let text = ctx.call_virtual(t, "getString", "()Ljava/lang/String;", &[]);
            match text.ok().and_then(fastemu_jvm::Value::obj) {
                Some(s) => Some(ctx.string_utf16(s)?.to_vec()),
                None => Some(Vec::new()),
            }
        }
        _ => None,
    };
    let menu = softkeys::menu_view(ctx)?;
    let (left, right) = if menu.is_some() || layout.bar.is_some() {
        softkeys::labels(ctx, obj)?
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(Chrome {
        layout,
        title,
        ticker,
        left,
        right,
        menu,
    })
}

/// Copies the displayable's pixels (kept at the top left of its buffer)
/// into their place on the screen.
fn copy_content(composed: &mut Surface, from: &Surface, content: Rect) {
    let (cw, fw) = (composed.width() as usize, from.width() as usize);
    let width = (content.width.max(0) as usize).min(cw).min(fw);
    let source = from.pixels();
    composed.edit_pixels(|to| {
        for row in 0..content.height.max(0) as usize {
            let dst_row = content.y as usize + row;
            if dst_row * cw >= to.len() || row * fw >= source.len() {
                break;
            }
            to[dst_row * cw..dst_row * cw + width]
                .copy_from_slice(&source[row * fw..row * fw + width]);
        }
    });
}

fn bar(g: &mut Graphics, s: &mut Surface, area: Rect, theme: &Theme) {
    g.set_color(theme.bar_background);
    g.fill_rect(s, area.x, area.y, area.width, area.height);
    g.set_color(theme.bar_foreground);
}

fn draw_chrome(composed: &mut Surface, chrome: &Chrome, font: &Font, theme: &Theme) {
    let Ok(mut g) = Graphics::new(composed, font.clone()) else {
        return;
    };
    let text_y = |area: Rect| area.y + (area.height - font.height()) / 2;
    if let (Some(area), Some(title)) = (chrome.layout.title, &chrome.title) {
        bar(&mut g, composed, area, theme);
        let _ = g.draw_string(composed, title, area.width / 2, text_y(area), HCENTER | TOP);
    }
    if let (Some(area), Some(ticker)) = (chrome.layout.ticker, &chrome.ticker) {
        g.set_color(theme.highlighted_background);
        g.fill_rect(composed, area.x, area.y, area.width, area.height);
        g.set_color(theme.highlighted_foreground);
        let _ = g.draw_string(composed, ticker, 2, text_y(area), LEFT | TOP);
    }
    let bar_height = font.height() + 4;
    let bar_area = chrome.layout.bar.unwrap_or(Rect::new(
        0,
        composed.height() as i32 - bar_height,
        composed.width() as i32,
        bar_height,
    ));
    if let Some(menu) = &chrome.menu {
        draw_menu(&mut g, composed, menu, bar_area.y, font, theme);
    }
    if chrome.layout.bar.is_some() || chrome.menu.is_some() {
        bar(&mut g, composed, bar_area, theme);
        let y = text_y(bar_area);
        let _ = g.draw_string(composed, &chrome.left, 3, y, LEFT | TOP);
        let right_x = bar_area.width - 3;
        let _ = g.draw_string(composed, &chrome.right, right_x, y, RIGHT | TOP);
    }
}

/// The options menu: a list above the soft key bar, the selected entry
/// highlighted.
fn draw_menu(
    g: &mut Graphics,
    s: &mut Surface,
    menu: &softkeys::MenuView,
    bottom: i32,
    font: &Font,
    theme: &Theme,
) {
    let line = softkeys::menu_line_height(font);
    let rows = menu.labels.len() as i32;
    let top = bottom - rows * line;
    let width = s.width() as i32;
    g.set_color(theme.border);
    g.fill_rect(s, 0, top - 1, width, rows * line + 1);
    for (i, label) in menu.labels.iter().enumerate() {
        let y = top + i as i32 * line;
        let selected = menu.first + i == menu.selected;
        let (bg, fg) = if selected {
            (theme.highlighted_background, theme.highlighted_foreground)
        } else {
            (theme.background, theme.foreground)
        };
        g.set_color(bg);
        g.fill_rect(s, 0, y, width, line);
        g.set_color(fg);
        let _ = g.draw_string(s, label, 4, y + (line - font.height()) / 2, LEFT | TOP);
    }
}

/// Waits until the next frame may be presented, so games that loop on
/// `repaint`, `serviceRepaints` or `flushGraphics` run at the instance's
/// frame rate. Only game threads wait; the event thread defers its paints
/// instead. An interrupt ends the wait early and stays set for the game.
pub(crate) fn wait_for_slot(ctx: &mut Ctx<'_>) -> JResult<()> {
    loop {
        let Some(at) = state(ctx).next_frame_at() else {
            return Ok(());
        };
        if Instant::now() >= at {
            return Ok(());
        }
        match ctx.wait_until(Some(at))? {
            Wake::Paused => ctx.yield_now()?,
            Wake::Interrupted => return Ok(()),
            Wake::Notified | Wake::TimedOut => {}
        }
    }
}
