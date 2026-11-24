//! The workspace: starting, restarting and closing games, and drawing
//! them as tiles in the grid or focus layout.

use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTime;

use fastemu_core::{Instance, InstanceStatus, LaunchError, LaunchRequest, Launcher};
use gpui::{
    Context, Div, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    Point, ScrollStrategy, Stateful, Styled, Window, canvas, div, prelude::*, px,
};

use super::library::SIDEBAR_WIDTH;
use super::panels::LOG_HEIGHT;
use super::widgets::{icon, icon_button, text_button, toggle_icon_button};
use super::{Dialog, FastEmuView, NoticeLevel, SettingsTab, Tile, TileId};
use crate::assets::Icon;
use crate::layout::{
    Extent, GAP, KeypadPlacement, ScreenFit, StripSide, THUMB_TITLE_HEIGHT, TILE_PADDING,
    TITLE_BAR_HEIGHT, TileLayout, fit_screen, focus_layout, grid, tile_layout,
};
use crate::library::EntryId;
use crate::screenshot;
use crate::session::{Session, free_slot, phone_pixel, status_label};
use crate::settings::{LayoutMode, profile_by_id};
use crate::theme::Theme;

/// The toolbar above the workspace.
const TOOLBAR_HEIGHT: f32 = 44.0;
/// Space around the tiles.
const WORKSPACE_PADDING: f32 = 12.0;
/// The tiles' border, inside their padding.
const TILE_BORDER: f32 = 2.0;

/// Starts an instance off the main thread: makes its data folder and asks
/// the launcher.
fn start(
    launcher: &dyn Launcher,
    request: LaunchRequest,
) -> Result<Arc<dyn Instance>, LaunchError> {
    std::fs::create_dir_all(&request.data_dir)?;
    launcher.launch(request)
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
                match result {
                    Ok(instance) => this.add_tile(entry, slot, request, instance, cx),
                    Err(err) => this.report(
                        format!("Could not start {name}: {err}"),
                        NoticeLevel::Error,
                        cx,
                    ),
                }
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
                match result {
                    Ok(instance) => {
                        tile.screen.borrow_mut().release(window);
                        tile.session.request = request;
                        tile.session.replace_instance(instance);
                    }
                    Err(err) => {
                        let title = tile.session.title.clone();
                        this.report(
                            format!("Could not restart {title}: {err}"),
                            NoticeLevel::Error,
                            cx,
                        );
                    }
                }
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
        }
        cx.notify();
    }

    /// Reads every game's latest frame, status and log lines.
    pub(super) fn poll_tiles(&mut self) {
        let focused = self.focused;
        let log_open = self.log_open;
        let mut scroll_to = None;
        for tile in &mut self.tiles {
            let before = tile.session.log.len() + tile.session.log.dropped() as usize;
            tile.session.poll();
            let grew = tile.session.log.len() + tile.session.log.dropped() as usize != before;
            if log_open && Some(tile.id) == focused {
                tile.session.log.mark_seen();
                if grew {
                    scroll_to = Some(tile.session.log.len().saturating_sub(1));
                }
            }
        }
        if let Some(last) = scroll_to {
            self.log_scroll.scroll_to_item(last, ScrollStrategy::Bottom);
        }
    }

    fn on_screen_down(&mut self, id: TileId, event: &MouseDownEvent, cx: &mut Context<'_, Self>) {
        self.focus_tile(id, cx);
        let Some(tile) = self.tile_mut(id) else {
            return;
        };
        if !tile.session.is_running() {
            return;
        }
        let Some(((x, y), inside)) = map_pointer(tile, event.position) else {
            return;
        };
        if inside {
            tile.session.pointer_press((x, y));
        }
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
        let sidebar = if self.sidebar_open {
            SIDEBAR_WIDTH
        } else {
            0.0
        };
        let log = if self.log_open { LOG_HEIGHT } else { 0.0 };
        let area = Extent::new(
            f32::from(viewport.width) - sidebar - 2.0 * WORKSPACE_PADDING,
            f32::from(viewport.height) - TOOLBAR_HEIGHT - log - 2.0 * WORKSPACE_PADDING,
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
                    s.text_mode,
                    &theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(tile) = this.tile_mut(id) {
                        tile.session.release_everything();
                        tile.session.text_mode = !tile.session.text_mode;
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
                    "The game crashed".to_owned(),
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
                .children(detail.map(|d| div().text_xs().text_color(theme.muted).child(d)))
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
                    .w_full()
                    .h(px(THUMB_TITLE_HEIGHT))
                    .text_xs()
                    .truncate()
                    .child(tile.session.title.clone()),
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
