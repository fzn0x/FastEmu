//! The soft keys and the options menu, as `docs/compat/notes/03-input.md`
//! describes after J2ME Loader:
//!
//! 1. Commands sort by type, then priority; the first `BACK` or `EXIT`
//!    moves to the front.
//! 2. One command: its label is on the left, and both soft keys fire it.
//! 3. Two: the front command is on the right soft key, the other on the
//!    left.
//! 4. More: the right soft key fires the front command and the left one
//!    opens a menu of all of them.
//! 5. A soft key press that fires a command or opens the menu is not
//!    passed to the game, and neither is its release.
//! 6. With no commands the soft keys reach the game.
//! 7. In full-screen mode no labels are drawn; with one command the left
//!    soft key opens the menu and the right one goes to the game; with
//!    more, both open the menu. A canvas without a `CommandListener`
//!    passes the soft keys to the game.
//!
//! While the menu is open every key goes to it: up and down (or 2 and 8)
//! move, fire, 5 or the left soft key choose, the right soft key or clear
//! closes it.

use std::sync::Arc;

use fastemu_core::PointerAction;
use fastemu_gfx::Font;
use fastemu_jvm::{Ctx, GlobalRef, JResult, ObjRef};

use super::command::{self, BACK, EXIT};
use super::displayable::{self, Kind, Layout};
use super::frame;
use super::keys;
use super::state::{Rect, current, state};
use crate::env::env;

const OPTIONS: &str = "Options";
const SELECT: &str = "Select";
const CANCEL: &str = "Back";

/// What a soft key does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Fire(ObjRef),
    OpenMenu,
}

/// The soft keys of a displayable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SoftKeys {
    left: Option<Slot>,
    right: Option<Slot>,
    /// The commands in menu order.
    commands: Vec<ObjRef>,
    /// Whether the labels are drawn (not in full-screen mode).
    labelled: bool,
}

/// The open options menu.
#[derive(Debug)]
pub(crate) struct Menu {
    pub owner: Arc<GlobalRef>,
    pub items: Vec<Arc<GlobalRef>>,
    pub selected: usize,
    /// The first entry shown, when not all fit.
    pub first: usize,
}

/// What the frame needs to draw the menu.
pub(crate) struct MenuView {
    pub labels: Vec<Vec<u16>>,
    pub first: usize,
    pub selected: usize,
}

/// The height of one menu entry.
pub(crate) fn menu_line_height(font: &Font) -> i32 {
    font.height() + 4
}

/// The commands of `obj`, sorted as rule 1 says.
fn sorted_commands(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<Vec<ObjRef>> {
    let mut commands = Vec::new();
    for &c in displayable::data(ctx, obj)?.commands() {
        let info = command::info(ctx, c)?;
        commands.push((c, info.kind, info.priority));
    }
    commands.sort_by_key(|&(_, kind, priority)| (kind, priority));
    if let Some(back) = commands
        .iter()
        .position(|&(_, kind, _)| kind == BACK || kind == EXIT)
    {
        let front = commands.remove(back);
        commands.insert(0, front);
    }
    Ok(commands.into_iter().map(|(c, _, _)| c).collect())
}

fn soft_keys(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<SoftKeys> {
    let d = displayable::data(ctx, obj)?;
    let canvas = d.kind() == Kind::Canvas;
    if canvas && d.listener().is_none() {
        return Ok(SoftKeys::default());
    }
    let full = canvas && d.is_full_screen();
    let commands = sorted_commands(ctx, obj)?;
    let (left, right) = match (commands.len(), full) {
        (0, _) => (None, None),
        (1, false) => (Some(Slot::Fire(commands[0])), Some(Slot::Fire(commands[0]))),
        (2, false) => (Some(Slot::Fire(commands[1])), Some(Slot::Fire(commands[0]))),
        (_, false) => (Some(Slot::OpenMenu), Some(Slot::Fire(commands[0]))),
        (1, true) => (Some(Slot::OpenMenu), None),
        (_, true) => (Some(Slot::OpenMenu), Some(Slot::OpenMenu)),
    };
    Ok(SoftKeys {
        left,
        right,
        commands,
        labelled: !full,
    })
}

fn label_of(ctx: &Ctx<'_>, slot: Option<Slot>) -> JResult<Vec<u16>> {
    Ok(match slot {
        Some(Slot::Fire(c)) => command::info(ctx, c)?.short_label,
        Some(Slot::OpenMenu) => OPTIONS.encode_utf16().collect(),
        None => Vec::new(),
    })
}

/// The left and right labels of the soft key bar of `obj`.
pub(crate) fn labels(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<(Vec<u16>, Vec<u16>)> {
    if state(ctx).menu.is_some() {
        return Ok((
            SELECT.encode_utf16().collect(),
            CANCEL.encode_utf16().collect(),
        ));
    }
    let keys = soft_keys(ctx, obj)?;
    if !keys.labelled {
        return Ok((Vec::new(), Vec::new()));
    }
    let left = label_of(ctx, keys.left)?;
    // One command shows its label once, on the left.
    let right = if keys.commands.len() == 1 {
        Vec::new()
    } else {
        label_of(ctx, keys.right)?
    };
    Ok((left, right))
}

/// Handles a soft key press on `obj`; returns whether the runtime took it
/// (fired a command or opened the menu).
pub(crate) fn press(ctx: &mut Ctx<'_>, obj: ObjRef, code: i32) -> JResult<bool> {
    let codes = env(ctx).profile.key_codes;
    if code != codes.soft_left && code != codes.soft_right {
        return Ok(false);
    }
    let keys = soft_keys(ctx, obj)?;
    let slot = if code == codes.soft_left {
        keys.left
    } else {
        keys.right
    };
    match slot {
        None => Ok(false),
        Some(Slot::Fire(c)) => {
            let c = ctx.root(c);
            command::fire(ctx, obj, c)?;
            Ok(true)
        }
        Some(Slot::OpenMenu) => {
            open_menu(ctx, obj, &keys.commands)?;
            Ok(true)
        }
    }
}

fn open_menu(ctx: &mut Ctx<'_>, owner: ObjRef, commands: &[ObjRef]) -> JResult<()> {
    let owner = Arc::new(ctx.new_global(owner));
    let items = commands
        .iter()
        .map(|&c| Arc::new(ctx.new_global(c)))
        .collect();
    state(ctx).menu = Some(Menu {
        owner,
        items,
        selected: 0,
        first: 0,
    });
    frame::present(ctx)
}

/// How many menu entries fit above the bar.
fn visible_rows(ctx: &mut Ctx<'_>) -> usize {
    let st = state(ctx);
    let font = st.fonts.default_font();
    let line = menu_line_height(&font);
    ((st.screen_height - (font.height() + 4)) / line).max(1) as usize
}

/// The menu as the frame draws it, if it is open.
pub(crate) fn menu_view(ctx: &mut Ctx<'_>) -> JResult<Option<MenuView>> {
    let rows = visible_rows(ctx);
    let Some(menu) = &mut state(ctx).menu else {
        return Ok(None);
    };
    if menu.selected < menu.first {
        menu.first = menu.selected;
    } else if menu.selected >= menu.first + rows {
        menu.first = menu.selected + 1 - rows;
    }
    let (first, selected) = (menu.first, menu.selected);
    let shown: Vec<Arc<GlobalRef>> = menu.items.iter().skip(first).take(rows).cloned().collect();
    let mut labels = Vec::with_capacity(shown.len());
    for global in shown {
        let c = global.get(ctx);
        labels.push(command::info(ctx, c)?.label);
    }
    Ok(Some(MenuView {
        labels,
        first,
        selected,
    }))
}

/// Closes the menu and fires the chosen command, if any.
fn close_menu(ctx: &mut Ctx<'_>, choose: bool) -> JResult<()> {
    let Some(menu) = state(ctx).menu.take() else {
        return Ok(());
    };
    frame::present(ctx)?;
    if !choose {
        return Ok(());
    }
    let owner = menu.owner.get(ctx);
    if current(ctx) != Some(owner) {
        return Ok(());
    }
    match menu.items.get(menu.selected) {
        Some(item) => {
            let c = item.get(ctx);
            command::fire(ctx, owner, c)
        }
        None => Ok(()),
    }
}

/// A key press while the menu is open.
pub(crate) fn menu_key(ctx: &mut Ctx<'_>, code: i32) -> JResult<()> {
    let codes = env(ctx).profile.key_codes;
    let action = keys::game_action(&codes, code);
    if code == codes.soft_right || code == codes.clear || code == codes.end {
        return close_menu(ctx, false);
    }
    if code == codes.soft_left || action == keys::FIRE {
        return close_menu(ctx, true);
    }
    let Some(menu) = &mut state(ctx).menu else {
        return Ok(());
    };
    let count = menu.items.len().max(1);
    match action {
        keys::UP => menu.selected = (menu.selected + count - 1) % count,
        keys::DOWN => menu.selected = (menu.selected + 1) % count,
        _ => return Ok(()),
    }
    frame::present(ctx)
}

/// A pointer event while the menu is open: a press on an entry chooses
/// it, a press on the bar acts as the soft key under it, and a press
/// anywhere else closes the menu.
pub(crate) fn menu_pointer(
    ctx: &mut Ctx<'_>,
    action: PointerAction,
    x: i32,
    y: i32,
) -> JResult<()> {
    if action != PointerAction::Press {
        return Ok(());
    }
    let (font, w, h) = {
        let st = state(ctx);
        (st.fonts.default_font(), st.screen_width, st.screen_height)
    };
    let bar_h = font.height() + 4;
    let line = menu_line_height(&font);
    if y >= h - bar_h {
        return close_menu(ctx, x < w / 2);
    }
    let Some(view) = menu_view(ctx)? else {
        return Ok(());
    };
    let top = h - bar_h - view.labels.len() as i32 * line;
    if y < top {
        return close_menu(ctx, false);
    }
    let index = view.first + ((y - top) / line) as usize;
    if let Some(menu) = &mut state(ctx).menu {
        menu.selected = index.min(menu.items.len().saturating_sub(1));
    }
    close_menu(ctx, true)
}

/// The soft key under a pointer press on the bar of `layout`, if any.
pub(crate) fn bar_key(ctx: &mut Ctx<'_>, layout: &Layout, x: i32, y: i32) -> Option<i32> {
    let bar: Rect = layout.bar?;
    if !bar.contains(x, y) {
        return None;
    }
    let codes = env(ctx).profile.key_codes;
    Some(if x < bar.x + bar.width / 2 {
        codes.soft_left
    } else {
        codes.soft_right
    })
}
