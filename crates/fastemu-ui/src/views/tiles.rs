//! The workspace: starting, restarting and closing games, and drawing
//! them as tiles in the grid or focus layout.

use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

use fastemu_core::{Instance, InstanceStatus, LaunchError, LaunchRequest, Launcher};
use gpui::{
    Context, Div, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point, ScrollStrategy, Stateful, Styled, Window, canvas, div, prelude::*, px,
};

use super::library::{SIDEBAR_WIDTH, UNREADABLE};
use super::panels::LOG_HEIGHT;
use super::touch::{self, Finger};
use super::widgets::{icon, icon_button, primary_button, text_button, toggle_icon_button};
use super::{Dialog, FastEmuView, NoticeLevel, SettingsTab, Tile, TileId};
use crate::assets::Icon;
use crate::layout::{
    Extent, GAP, KeypadPlacement, ScreenFit, StripSide, THUMB_TITLE_HEIGHT, TILE_PADDING,
    TITLE_BAR_HEIGHT, TileLayout, fit_screen, focus_layout, grid, tile_layout,
};
use crate::library::EntryId;
use crate::screenshot;
use crate::session::{NotStarted, Session, free_slot, phone_pixel, status_label};
use crate::settings::{LayoutMode, profile_by_id};
use crate::theme::Theme;

/// How faint the link prompt's buttons look while they ignore clicks.
const DIMMED: f32 = 0.5;
/// The toolbar above the workspace.
const TOOLBAR_HEIGHT: f32 = 44.0;
/// Space around the tiles.
const WORKSPACE_PADDING: f32 = 12.0;
/// The tiles' border, inside their padding.
const TILE_BORDER: f32 = 2.0;
/// Below this width, a phone held upright, the library and a game do not
/// fit side by side, so starting a game hides the library.
const NARROW_WIDTH: f32 = 2.5 * SIDEBAR_WIDTH;

/// Starts an instance off the main thread: makes its data folder and asks
/// the launcher. The launcher reads the suite again, and the file may have
/// changed since the library read it, so a reader that panics on it must
/// not take the app down: the game is reported as not started, as one the
/// library could not read is (see `inspect`).
fn start(
    launcher: &dyn Launcher,
    request: LaunchRequest,
) -> Result<Arc<dyn Instance>, LaunchError> {
    std::fs::create_dir_all(&request.data_dir)?;
    let suite = request.suite.clone();
    panic::catch_unwind(AssertUnwindSafe(|| launcher.launch(request))).unwrap_or_else(|_| {
        log::error!("starting {} panicked; it is not started", suite.display());
        Err(LaunchError::InvalidSuite(UNREADABLE.to_owned()))
    })
}

/// Stops an instance that finished starting after its window or view
/// went away: no tile shows it and nothing else would ever stop it.
fn stop_if_orphaned(shown: bool, started: Option<Arc<dyn Instance>>) {
    if !shown && let Some(instance) = started {
        instance.stop();
    }
}

impl FastEmuView {
    fn tile(&self, id: TileId) -> Option<&Tile> {
        self.tiles.iter().find(|t| t.id == id)
    }

    fn tile_mut(&mut self, id: TileId) -> Option<&mut Tile> {
        self.tiles.iter_mut().find(|t| t.id == id)
    }

    pub(super) fn focused_tile_mut(&mut self) -> Option<&mut Tile> {
        let id = self.focused?;
        self.tile_mut(id)
    }

    /// The request that starts `entry` in save slot `slot` with the
    /// entry's current settings.
    fn request_for(&self, entry: EntryId, slot: u32) -> Option<LaunchRequest> {
        let e = self.library.get(entry)?;
        let profile = e
            .settings
            .profile
            .clone()
            .unwrap_or_else(|| self.settings.default_profile.clone());
        let mut request = LaunchRequest::new(e.path.clone(), self.paths.game_dir(entry, slot));
        request.profile = profile_by_id(&profile);
        request.profile.touch |= e.settings.touch;
        request.settings = e.settings.instance_settings();
        request.notify = Some(self.wake.callback());
        Some(request)
    }

    /// Starts another copy of a library entry in the workspace.
    pub(super) fn launch(
        &mut self,
        entry: EntryId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let taken = self
            .tiles
            .iter()
            .filter(|t| t.session.entry == entry)
            .map(|t| t.session.slot)
            .chain(
                self.launching
                    .iter()
                    .filter(|(e, _)| *e == entry)
                    .map(|&(_, slot)| slot),
            );
        let slot = free_slot(taken);
        let Some(request) = self.request_for(entry, slot) else {
            return;
        };
        if f32::from(window.viewport_size().width) < NARROW_WIDTH {
            self.sidebar_open = false;
        }
        let name = self
            .library
            .get(entry)
            .map(|e| e.name.clone())
            .unwrap_or_default();
        self.launching.push((entry, slot));
        let launcher = Arc::clone(&self.launcher);
        cx.spawn_in(window, async move |this, cx| {
            let sent = request.clone();
            let result = cx
                .background_spawn(async move { start(&*launcher, sent) })
                .await;
            let started = result.as_ref().ok().map(Arc::clone);
            let shown = this.update_in(cx, |this, _, cx| {
                this.launching.retain(|l| *l != (entry, slot));
                let instance = result.unwrap_or_else(|err| {
                    log::warn!("could not start {name}: {err}");
                    Arc::new(NotStarted::new(name, &err))
                });
                this.add_tile(entry, slot, request, instance, cx);
            });
            stop_if_orphaned(shown.is_ok(), started);
        })
        .detach();
    }

    fn add_tile(
        &mut self,
        entry: EntryId,
        slot: u32,
        request: LaunchRequest,
        instance: Arc<dyn Instance>,
        cx: &mut Context<'_, Self>,
    ) {
        let id = TileId(self.next_tile);
        self.next_tile += 1;
        self.tiles.push(Tile {
            id,
            session: Session::new(entry, slot, request, instance),
            screen: Rc::default(),
            restarting: false,
            paused_by_app: false,
            text_asked: false,
        });
        self.focus_tile(id, cx);
    }

    /// Gives `id` the keyboard, releasing whatever the previous game held.
    pub(super) fn focus_tile(&mut self, id: TileId, cx: &mut Context<'_, Self>) {
        if self.focused != Some(id) {
            self.release_focused_keys();
            self.focused = Some(id);
            let log_open = self.log_open;
            if let Some(tile) = self.tile_mut(id)
                && log_open
            {
                tile.session.log.mark_seen();
                self.log_scroll.scroll_to_bottom();
            }
            cx.notify();
        }
    }

    pub(super) fn cycle_focus(&mut self, step: isize, cx: &mut Context<'_, Self>) {
        if self.tiles.is_empty() {
            return;
        }
        let len = self.tiles.len() as isize;
        let current = self
            .focused
            .and_then(|id| self.tiles.iter().position(|t| t.id == id))
            .map_or(-1, |i| i as isize);
        let next = (current + step).rem_euclid(len) as usize;
        let id = self.tiles[next].id;
        self.focus_tile(id, cx);
    }

    /// Stops a game and takes its tile away.
    pub(super) fn close_tile(
        &mut self,
        id: TileId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(index) = self.tiles.iter().position(|t| t.id == id) else {
            return;
        };
        let mut tile = self.tiles.remove(index);
        tile.session.release_everything();
        tile.session.instance.stop();
        tile.screen.borrow_mut().release(window);
        if self.focused == Some(id) {
            self.focused = self
                .tiles
                .get(index.min(self.tiles.len().saturating_sub(1)))
                .map(|t| t.id);
        }
        cx.notify();
    }

    /// Stops a game and starts it again in the same save slot, with the
    /// entry's current settings.
    pub(super) fn restart_tile(
        &mut self,
        id: TileId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let Some((entry, slot)) = self
            .tile(id)
            .filter(|t| !t.restarting)
            .map(|t| (t.session.entry, t.session.slot))
        else {
            return;
        };
        let Some(request) = self.request_for(entry, slot) else {
            return;
        };
        if let Some(tile) = self.tile_mut(id) {
            tile.session.release_everything();
            tile.session.instance.stop();
            tile.restarting = true;
        }
        let launcher = Arc::clone(&self.launcher);
        cx.spawn_in(window, async move |this, cx| {
            let sent = request.clone();
            let result = cx
                .background_spawn(async move { start(&*launcher, sent) })
                .await;
            let started = result.as_ref().ok().map(Arc::clone);
            let shown = this.update_in(cx, |this, window, cx| {
                let Some(tile) = this.tile_mut(id) else {
                    if let Ok(instance) = result {
                        instance.stop();
                    }
                    return;
                };
                tile.restarting = false;
                let instance = result.unwrap_or_else(|err| {
                    let title = tile.session.title.clone();
                    log::warn!("could not restart {title}: {err}");
                    Arc::new(NotStarted::new(title, &err))
                });
                tile.screen.borrow_mut().release(window);
                tile.session.request = request;
                tile.session.replace_instance(instance);
                cx.notify();
            });
            stop_if_orphaned(shown.is_ok(), started);
        })
        .detach();
        cx.notify();
    }

    pub(super) fn toggle_mute(&mut self, id: TileId, cx: &mut Context<'_, Self>) {
        let volume = self
            .tile(id)
            .and_then(|t| self.library.get(t.session.entry))
            .map_or(1.0, |e| e.settings.volume);
        if let Some(tile) = self.tile_mut(id) {
            let muted = !tile.session.muted;
            tile.session.set_muted(muted, volume);
        }
        cx.notify();
    }

    fn screenshot(&mut self, id: TileId, cx: &mut Context<'_, Self>) {
        let Some(tile) = self.tile(id) else {
            return;
        };
        let Some(frame) = tile.session.frame.clone() else {
            self.report("Nothing on screen yet", NoticeLevel::Info, cx);
            return;
        };
        let dir = self.paths.screenshot_dir.clone();
        let title = tile.session.title.clone();
        cx.spawn(async move |this, cx| {
            let saved = cx
                .background_spawn(async move {
                    screenshot::save(&dir, &title, &frame, SystemTime::now())
                })
                .await;
            this.update(cx, |this, cx| match saved {
                Ok(path) => this.report(format!("Saved {}", path.display()), NoticeLevel::Info, cx),
                Err(err) => this.report(
                    format!("Could not save the screenshot: {err}"),
                    NoticeLevel::Error,
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn toggle_layout(&mut self, cx: &mut Context<'_, Self>) {
        self.update_settings(cx, |s| {
            s.layout = match s.layout {
                LayoutMode::Grid => LayoutMode::Focus,
                LayoutMode::Focus => LayoutMode::Grid,
            };
        });
    }

    pub(super) fn toggle_log(&mut self, cx: &mut Context<'_, Self>) {
        self.log_open = !self.log_open;
        if self.log_open
            && let Some(tile) = self.focused_tile_mut()
        {
            tile.session.log.mark_seen();
            self.log_scroll.scroll_to_bottom();
        }
        cx.notify();
    }

    /// Reads every game's latest frame, status and log lines, at `now`. The
    /// open log follows its game's new lines while it shows the last line;
    /// scrolled up to read older ones, it stays where it is.
    pub(super) fn poll_tiles(&mut self, now: Instant) {
        let focused = self.focused;
        let log_open = self.log_open;
        let following = self.log_scroll.is_scrolled_to_end().unwrap_or(true);
        let mut scroll_to = None;
        for tile in &mut self.tiles {
            let before = tile.session.log.len() + tile.session.log.dropped() as usize;
            tile.session.poll(now);
            let grew = tile.session.log.len() + tile.session.log.dropped() as usize != before;
            if log_open && Some(tile.id) == focused {
                tile.session.log.mark_seen();
                if grew {
                    scroll_to = Some(tile.session.log.len().saturating_sub(1));
                }
            }
        }
        if let Some(last) = scroll_to
            && following
        {
            self.log_scroll.scroll_to_item(last, ScrollStrategy::Bottom);
        }
    }

    /// Opens `link`, which `id`'s prompt showed, in the system's browser,
    /// and answers the prompt. Only the user's click gets here, and only
    /// one that comes after the prompt's wait (see `answer_link`).
    fn open_link(&mut self, id: TileId, link: &str, cx: &mut Context<'_, Self>) {
        if self.answer_link(id, link, cx) {
            cx.open_url(link);
        }
    }

    /// Answers `id`'s prompt for `link` without opening it, with the same
    /// wait.
    fn dismiss_link(&mut self, id: TileId, link: &str, cx: &mut Context<'_, Self>) {
        self.answer_link(id, link, cx);
    }

    /// Takes a click on a button of `id`'s prompt for `link`. A click in the
    /// first half second of the prompt, or on a prompt whose link changed
    /// since, does nothing: a user who taps fast has not read it. Returns
    /// whether the prompt was answered.
    fn answer_link(&mut self, id: TileId, link: &str, cx: &mut Context<'_, Self>) -> bool {
        let now = cx.background_executor().now();
        let answered = self
            .tile_mut(id)
            .is_some_and(|tile| tile.session.answer_link(link, now));
        if answered {
            cx.notify();
        }
        answered
    }

    fn on_screen_down(&mut self, id: TileId, event: &MouseDownEvent, cx: &mut Context<'_, Self>) {
        self.focus_tile(id, cx);
        let Some(tile) = self.tile_mut(id) else {
            return;
        };
        if !tile.session.is_running() || tile.shows_link() {
            return;
        }
        let Some(((x, y), inside)) = map_pointer(tile, event.position) else {
            return;
        };
        if inside {
            tile.session.pointer_press((x, y));
        }
    }

    /// A finger on `id`'s screen. A touch starting on a running game's
    /// screen is the game's, as a press, drags and a release; any other
    /// is left to become a tap, for the buttons over a stopped game or of
    /// a link prompt.
    pub(super) fn screen_touch(
        &mut self,
        id: TileId,
        finger: Finger,
        position: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if finger == Finger::Down {
            let running = self
                .tile(id)
                .is_some_and(|t| t.session.is_running() && !t.restarting && !t.shows_link());
            if !running {
                return false;
            }
            self.focus_tile(id, cx);
        }
        let Some(tile) = self.tile_mut(id) else {
            return false;
        };
        let at = map_pointer(tile, position);
        match finger {
            Finger::Down => match at {
                Some((pixel, true)) => tile.session.pointer_press(pixel),
                _ => return false,
            },
            Finger::Moved => {
                if let Some((pixel, _)) = at {
                    tile.session.pointer_drag(pixel);
                }
            }
            Finger::Up => tile.session.pointer_release(at.map(|(pixel, _)| pixel)),
        }
        true
    }

    fn on_screen_move(&mut self, id: TileId, event: &MouseMoveEvent) {
        let Some(tile) = self.tile_mut(id) else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            return;
        }
        if let Some((pixel, _)) = map_pointer(tile, event.position) {
            tile.session.pointer_drag(pixel);
        }
    }

    fn on_screen_up(&mut self, id: TileId, event: &MouseUpEvent) {
        let Some(tile) = self.tile_mut(id) else {
            return;
        };
        let pixel = map_pointer(tile, event.position).map(|(p, _)| p);
        tile.session.pointer_release(pixel);
    }

    /// The toolbar, the tiles and the log drawer.
    pub(super) fn render_main(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let viewport = window.viewport_size();
        let safe = window.insets().safe_area;
        let sidebar = if self.sidebar_open {
            SIDEBAR_WIDTH
        } else {
            0.0
        };
        let log = if self.log_open { LOG_HEIGHT } else { 0.0 };
        let area = Extent::new(
            f32::from(viewport.width - safe.left - safe.right) - sidebar - 2.0 * WORKSPACE_PADDING,
            f32::from(viewport.height - safe.top - safe.bottom)
                - TOOLBAR_HEIGHT
                - log
                - 2.0 * WORKSPACE_PADDING,
        );
        let workspace = if self.tiles.is_empty() {
            Self::render_empty_workspace(cx)
        } else {
            match self.settings.layout {
                LayoutMode::Grid => self.render_grid(area, window, cx),
                LayoutMode::Focus => self.render_focus(area, window, cx),
            }
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .p(px(WORKSPACE_PADDING))
                    .bg(theme.background)
                    .child(workspace),
            )
            .when(self.log_open, |d| d.child(self.render_log(cx)))
    }

    fn render_toolbar(&self, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let count = self.tiles.len();
        let summary = match count {
            0 => "No games running".to_owned(),
            1 => "1 game running".to_owned(),
            n => format!("{n} games running"),
        };
        let layout = self.settings.layout;
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.0))
            .h(px(TOOLBAR_HEIGHT))
            .px(px(8.0))
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .child(
                icon_button(
                    "toggle-sidebar",
                    if self.sidebar_open {
                        Icon::SidebarClose
                    } else {
                        Icon::SidebarOpen
                    },
                    "Show or hide the library",
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.sidebar_open = !this.sidebar_open;
                    cx.notify();
                })),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .px(px(6.0))
                    .truncate()
                    .text_xs()
                    .text_color(theme.muted)
                    .child(summary),
            )
            .child(
                toggle_icon_button(
                    "layout-grid",
                    Icon::Grid,
                    "Grid layout",
                    layout == LayoutMode::Grid,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_settings(cx, |s| s.layout = LayoutMode::Grid);
                })),
            )
            .child(
                toggle_icon_button(
                    "layout-focus",
                    Icon::Focus,
                    "Focus layout",
                    layout == LayoutMode::Focus,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_settings(cx, |s| s.layout = LayoutMode::Focus);
                })),
            )
            .child(div().w(px(1.0)).h(px(18.0)).mx(px(4.0)).bg(theme.border))
            .child(
                toggle_icon_button(
                    "toggle-keypad",
                    Icon::Keyboard,
                    "On-screen keypad",
                    self.settings.keypad,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_settings(cx, |s| s.keypad = !s.keypad);
                })),
            )
            .child(
                toggle_icon_button("toggle-log", Icon::Log, "Game log", self.log_open, &theme)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_log(cx))),
            )
            .child(
                icon_button("open-settings", Icon::Settings, "Settings", &theme).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.open_dialog(Dialog::Settings(SettingsTab::General), cx);
                    }),
                ),
            )
    }

    fn render_empty_workspace(cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        div()
            .flex()
            .flex_col()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(10.0))
            .text_center()
            .child(icon(Icon::Gamepad, 40.0, theme.muted))
            .child(div().text_color(theme.text).child("No games running"))
            .child(div().max_w(px(360.0)).text_color(theme.muted).child(
                "Start a game from the library. Games run side by side; \
                 the keyboard plays the highlighted one.",
            ))
    }

    /// The largest screen among the running games, for sizing the grid.
    fn largest_screen(&self) -> (u32, u32) {
        self.tiles
            .iter()
            .map(|t| t.session.screen_size())
            .fold((1, 1), |a, b| (a.0.max(b.0), a.1.max(b.1)))
    }

    fn render_grid(&self, area: Extent, window: &mut Window, cx: &mut Context<'_, Self>) -> Div {
        let scale = window.scale_factor();
        let keypad = self.settings.keypad;
        let g = grid(self.tiles.len(), area, self.largest_screen(), keypad, scale);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .justify_center()
            .gap(px(GAP))
            .children(self.tiles.chunks(g.columns).map(|row| {
                div()
                    .flex()
                    .justify_center()
                    .gap(px(GAP))
                    .children(row.iter().map(|tile| {
                        let layout = tile_layout(g.cell, tile.session.screen_size(), keypad, scale);
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .w(px(g.cell.width))
                            .h(px(g.cell.height))
                            .child(self.render_tile(tile, &layout, cx))
                    }))
            }))
    }

    fn render_focus(&self, area: Extent, window: &mut Window, cx: &mut Context<'_, Self>) -> Div {
        let scale = window.scale_factor();
        let f = focus_layout(self.tiles.len(), area);
        let main_id = self.focused.unwrap_or(self.tiles[0].id);
        let main = self.tile(main_id).unwrap_or(&self.tiles[0]);
        let layout = tile_layout(
            f.main,
            main.session.screen_size(),
            self.settings.keypad,
            scale,
        );
        let main_cell = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .w(px(f.main.width))
            .h(px(f.main.height))
            .child(self.render_tile(main, &layout, cx));
        let Some((side, strip)) = f.strip else {
            return div().flex().flex_1().justify_center().child(main_cell);
        };
        let thumbs = self
            .tiles
            .iter()
            .filter(|t| t.id != main.id)
            .map(|t| Self::render_thumb(t, f.thumb, scale, cx));
        let strip = div()
            .id("thumbnails")
            .flex()
            .flex_none()
            .gap(px(GAP))
            .w(px(strip.width))
            .h(px(strip.height))
            .children(thumbs);
        let strip = match side {
            StripSide::Right => strip.flex_col().overflow_y_scroll(),
            StripSide::Bottom => strip.flex_row().overflow_x_scroll(),
        };
        div()
            .flex()
            .flex_1()
            .gap(px(GAP))
            .when(side == StripSide::Bottom, Styled::flex_col)
            .child(main_cell)
            .child(strip)
    }

    fn render_tile(
        &self,
        tile: &Tile,
        layout: &TileLayout,
        cx: &mut Context<'_, Self>,
    ) -> Stateful<Div> {
        let theme = Theme::get(cx).clone();
        let id = tile.id;
        let focused = self.focused == Some(id);
        let screen = Self::render_screen(tile, layout.screen, true, cx);
        let body = match layout.keypad {
            None => div().flex().justify_center().child(screen),
            Some(KeypadPlacement::Below) => div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(GAP))
                .child(screen)
                .child(Self::render_keypad(tile, KeypadPlacement::Below, cx)),
            Some(KeypadPlacement::Beside) => div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(GAP))
                .child(screen)
                .child(Self::render_keypad(tile, KeypadPlacement::Beside, cx)),
        };
        div()
            .id(("tile", id.0))
            .flex()
            .flex_col()
            .flex_none()
            .w(px(layout.frame.width))
            .h(px(layout.frame.height))
            .p(px(TILE_PADDING - TILE_BORDER))
            .rounded(px(10.0))
            .bg(theme.surface)
            .border(px(TILE_BORDER))
            .border_color(if focused { theme.accent } else { theme.border })
            .overflow_hidden()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    this.focus_tile(id, cx);
                }),
            )
            .child(Self::render_title_bar(tile, cx))
            .child(body.flex_1().min_h_0())
    }

    fn render_title_bar(tile: &Tile, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let id = tile.id;
        let s = &tile.session;
        let paused = s.status == InstanceStatus::Paused;
        let finished = s.status.is_finished();
        let dot = match (&s.status, tile.restarting) {
            (_, true) | (InstanceStatus::Starting | InstanceStatus::Paused, _) => theme.muted,
            (InstanceStatus::Running, _) => theme.accent,
            (InstanceStatus::Exited, _) => theme.border,
            (InstanceStatus::Crashed { .. }, _) => theme.danger,
        };
        let status = (s.status != InstanceStatus::Running || tile.restarting).then(|| {
            if tile.restarting {
                "Restarting".to_owned()
            } else {
                s.status_label()
            }
        });
        let problems = s.log.unseen_problems();
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.0))
            .h(px(TITLE_BAR_HEIGHT))
            .pb(px(4.0))
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .size(px(7.0))
                    .mr(px(6.0))
                    .rounded(px(4.0))
                    .bg(dot),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child(s.title.clone()),
            )
            .children(status.map(|label| {
                div()
                    .flex_none()
                    .mr(px(4.0))
                    .text_xs()
                    .text_color(theme.muted)
                    .child(label)
            }))
            .child(
                toggle_icon_button(
                    "text-mode",
                    Icon::Text,
                    "Type text into the game",
                    s.typing_text(),
                    &theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(tile) = this.tile_mut(id) {
                        tile.session.toggle_typing();
                    }
                    cx.notify();
                })),
            )
            .when(!finished, |d| {
                d.child(
                    icon_button(
                        "pause",
                        if paused { Icon::Play } else { Icon::Pause },
                        if paused { "Resume" } else { "Pause" },
                        &theme,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(tile) = this.tile_mut(id) {
                            tile.session.toggle_pause();
                        }
                        cx.notify();
                    })),
                )
            })
            .child(
                icon_button("restart", Icon::Restart, "Restart", &theme).on_click(
                    cx.listener(move |this, _, window, cx| this.restart_tile(id, window, cx)),
                ),
            )
            .child(
                toggle_icon_button(
                    "mute",
                    if s.muted {
                        Icon::VolumeOff
                    } else {
                        Icon::Volume
                    },
                    if s.muted { "Unmute" } else { "Mute" },
                    s.muted,
                    &theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.toggle_mute(id, cx))),
            )
            .child(
                icon_button("screenshot", Icon::Camera, "Save a screenshot", &theme)
                    .on_click(cx.listener(move |this, _, _, cx| this.screenshot(id, cx))),
            )
            .child(
                div()
                    .relative()
                    .child(
                        icon_button("log", Icon::Log, "Show the log", &theme).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.focus_tile(id, cx);
                                if !this.log_open {
                                    this.toggle_log(cx);
                                }
                            }),
                        ),
                    )
                    .when(problems > 0, |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(2.0))
                                .right(px(2.0))
                                .size(px(7.0))
                                .rounded(px(4.0))
                                .bg(theme.danger),
                        )
                    }),
            )
            .child(
                icon_button("close", Icon::Close, "Close", &theme).on_click(
                    cx.listener(move |this, _, window, cx| this.close_tile(id, window, cx)),
                ),
            )
    }

    /// A game's screen, drawn by a canvas from the tile's uploaded image.
    fn render_screen(
        tile: &Tile,
        fit: ScreenFit,
        interactive: bool,
        cx: &mut Context<'_, Self>,
    ) -> Stateful<Div> {
        let theme = Theme::get(cx).clone();
        let id = tile.id;
        let cache = tile.screen.clone();
        let frame = tile.session.frame.clone();
        let instance = tile.session.instance.id();
        let factor = fit.texture_factor;
        let screen = div()
            .id("screen")
            .relative()
            .flex_none()
            .w(px(fit.size.width))
            .h(px(fit.size.height))
            .bg(theme.screen)
            .overflow_hidden()
            .child(
                canvas(
                    |_, _, _| {},
                    move |bounds, (), window, _| {
                        if let Some(frame) = &frame {
                            cache
                                .borrow_mut()
                                .paint(bounds, instance, frame, factor, window);
                        }
                    },
                )
                .size_full(),
            )
            .children(Self::render_screen_overlay(tile, interactive, cx));
        if !interactive {
            return screen;
        }
        screen
            .children(Self::render_link_prompt(tile, cx))
            .debug_selector(|| format!("screen-{}", id.0))
            .child(touch::touch_screen(cx.entity().downgrade(), id))
            .cursor_crosshair()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.on_screen_down(id, event, cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, _| {
                this.on_screen_move(id, event);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseUpEvent, _, _| this.on_screen_up(id, event)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseUpEvent, _, _| this.on_screen_up(id, event)),
            )
    }

    /// What covers the screen while a game is not simply running.
    fn render_screen_overlay(
        tile: &Tile,
        interactive: bool,
        cx: &mut Context<'_, Self>,
    ) -> Option<Div> {
        let theme = Theme::get(cx).clone();
        let s = &tile.session;
        let id = tile.id;
        let (mark, title, detail) = if tile.restarting {
            (Icon::Restart, "Restarting".to_owned(), None)
        } else {
            match &s.status {
                InstanceStatus::Paused => (Icon::Pause, "Paused".to_owned(), None),
                InstanceStatus::Exited => (Icon::Info, "The game ended".to_owned(), None),
                InstanceStatus::Crashed { message } => (
                    Icon::Crash,
                    if s.frame.is_some() {
                        "The game crashed".to_owned()
                    } else {
                        "The game could not start".to_owned()
                    },
                    Some(message.clone()),
                ),
                InstanceStatus::Starting if s.frame.is_none() => {
                    (Icon::Gamepad, status_label(&s.status), None)
                }
                _ => return None,
            }
        };
        let finished = s.status.is_finished() && !tile.restarting;
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .p(px(12.0))
                .bg(theme.scrim)
                .text_color(theme.text)
                .text_center()
                .child(icon(mark, 24.0, theme.text))
                .child(div().child(title))
                .children(detail.map(|d| {
                    div()
                        .max_w_full()
                        .max_h(px(120.0))
                        .overflow_hidden()
                        .text_xs()
                        .text_color(theme.muted)
                        .child(d)
                }))
                .when(finished && interactive, |d| {
                    d.child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(text_button("overlay-restart", "Restart", &theme).on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.restart_tile(id, window, cx);
                                }),
                            ))
                            .child(text_button("overlay-close", "Close", &theme).on_click(
                                cx.listener(move |this, _, window, cx| {
                                    this.close_tile(id, window, cx);
                                }),
                            )),
                    )
                }),
        )
    }

    /// The question over a game's screen while the game asks to open a web
    /// page: the game's name, the whole link, **Open** and **Dismiss**.
    /// Nothing opens until the user chooses **Open**, and the buttons act
    /// on the link they showed, even if the game asked for another since.
    ///
    /// On a small screen the question scrolls as one, so the link is never
    /// squeezed out and the buttons, below it, are always in reach. A new
    /// link gets a new scroll position, at the top, so **Open** is not left
    /// under the pointer for a link the user has not read.
    ///
    /// The buttons ignore clicks for half a second after the question
    /// appears or its link changes (see `LINK_ARMING`), and show dimmed
    /// until then, so a fast tap meant for the game answers nothing.
    fn render_link_prompt(tile: &Tile, cx: &mut Context<'_, Self>) -> Option<Div> {
        if !tile.shows_link() {
            return None;
        }
        let link = tile.session.link()?.to_owned();
        let armed = tile.session.link_armed(cx.background_executor().now());
        let theme = Theme::get(cx).clone();
        let id = tile.id;
        let (to_open, to_dismiss) = (link.clone(), link.clone());
        let question = format!("{} wants to open a web page", tile.session.title);
        let card = div()
            .id(("link-prompt", link_key(&link)))
            .debug_selector(|| format!("link-prompt-{}", id.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .max_w_full()
            .max_h_full()
            .overflow_y_scroll()
            .p(px(8.0))
            .rounded(px(8.0))
            .bg(theme.raised)
            .border_1()
            .border_color(theme.border)
            .text_color(theme.text)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(Icon::Globe, 16.0, theme.accent))
                    .child(div().min_w_0().text_sm().child(question)),
            )
            .child(
                div()
                    .debug_selector(|| format!("link-url-{}", id.0))
                    .flex_none()
                    .text_xs()
                    .child(link),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .flex_wrap()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        text_button("link-dismiss", "Dismiss", &theme)
                            .flex_shrink(1.0)
                            .min_w_0()
                            .overflow_hidden()
                            .debug_selector(|| format!("link-dismiss-{}", id.0))
                            .when(!armed, |button| button.opacity(DIMMED))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.dismiss_link(id, &to_dismiss, cx);
                            })),
                    )
                    .child(
                        primary_button("link-open", "Open", &theme)
                            .flex_shrink(1.0)
                            .min_w_0()
                            .overflow_hidden()
                            .debug_selector(|| format!("link-open-{}", id.0))
                            .when(!armed, |button| button.opacity(DIMMED))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_link(id, &to_open, cx);
                            })),
                    ),
            );
        // The question covers the game's screen and, over an ended game,
        // its Restart and Close: a click on it must not reach them.
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .p(px(4.0))
                .bg(theme.scrim)
                .block_mouse_except_scroll()
                .child(card),
        )
    }

    /// A small live picture of a game in the focus layout's strip.
    fn render_thumb(
        tile: &Tile,
        size: Extent,
        scale: f32,
        cx: &mut Context<'_, Self>,
    ) -> Stateful<Div> {
        let theme = Theme::get(cx).clone();
        let id = tile.id;
        let inset = 2.0 * (4.0 + TILE_BORDER);
        let fit = fit_screen(
            tile.session.screen_size(),
            Extent::new(size.width - inset, size.height - inset - THUMB_TITLE_HEIGHT),
            scale,
        );
        let hover = theme.muted;
        div()
            .id(("thumb", id.0))
            .flex()
            .flex_col()
            .flex_none()
            .items_center()
            .w(px(size.width))
            .h(px(size.height))
            .p(px(4.0))
            .rounded(px(8.0))
            .bg(theme.surface)
            .border(px(TILE_BORDER))
            .border_color(theme.border)
            .cursor_pointer()
            .hover(move |s| s.border_color(hover))
            .on_click(cx.listener(move |this, _, _, cx| this.focus_tile(id, cx)))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(4.0))
                    .w_full()
                    .h(px(THUMB_TITLE_HEIGHT))
                    .text_xs()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(tile.session.title.clone()),
                    )
                    // The game waits for an answer to its link, which
                    // shows once it is brought forward.
                    .when(tile.shows_link(), |d| {
                        d.child(icon(Icon::Globe, 12.0, theme.accent))
                    }),
            )
            .child(Self::render_screen(tile, fit, false, cx))
    }
}

/// Maps a window position onto the phone pixels of the tile's screen, as
/// last drawn.
fn map_pointer(tile: &Tile, position: Point<Pixels>) -> Option<((i32, i32), bool)> {
    let bounds = tile.screen.borrow().drawn()?;
    Some(phone_pixel(
        (f32::from(position.x), f32::from(position.y)),
        (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
        (f32::from(bounds.size.width), f32::from(bounds.size.height)),
        tile.session.screen_size(),
    ))
}

/// A number for `link`, so each link's prompt keeps a scroll position of
/// its own.
fn link_key(link: &str) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    link.hash(&mut hasher);
    hasher.finish()
}
