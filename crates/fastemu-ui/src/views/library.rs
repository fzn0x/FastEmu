//! The library sidebar: adding games by picker or drop, reading their
//! icons, removing and launching them.

use std::path::PathBuf;
use std::sync::Arc;

use fastemu_core::{Launcher, SuiteInfo};
use gpui::{
    Context, Div, IntoElement, ParentElement, PathPromptOptions, Styled, Window, div, img,
    prelude::*, px,
};

use super::widgets::{icon, icon_button, primary_button, text_button};
use super::{Dialog, FastEmuView, NoticeLevel, render_image};
use crate::assets::Icon;
use crate::library::{EntryId, pick_suites};
use crate::pixels::{Bgra, icon_bgra};
use crate::theme::Theme;

/// The sidebar's width.
pub(super) const SIDEBAR_WIDTH: f32 = 264.0;
/// Icons are drawn at 32 logical pixels; decoding them to at least 64
/// keeps them sharp on high-density displays.
const ICON_SIDE: f32 = 32.0;
const ICON_PIXELS: u32 = 64;

/// What reading a suite found, computed off the main thread.
struct Inspected {
    path: PathBuf,
    info: Result<SuiteInfo, String>,
    icon: Option<Bgra>,
}

fn inspect(launcher: &dyn Launcher, path: PathBuf) -> Inspected {
    match launcher.inspect(&path) {
        Ok(info) => {
            let icon = info
                .icon
                .as_deref()
                .and_then(|png| icon_bgra(png, ICON_PIXELS));
            Inspected {
                path,
                info: Ok(info),
                icon,
            }
        }
        Err(err) => Inspected {
            path,
            info: Err(err.to_string()),
            icon: None,
        },
    }
}

impl FastEmuView {
    /// Reads every entry's suite again for its icon and current details,
    /// and notes the ones that can no longer be read.
    pub(super) fn refresh_library(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) {
        let launcher = Arc::clone(&self.launcher);
        let entries: Vec<(EntryId, PathBuf)> = self
            .library
            .entries()
            .iter()
            .map(|e| (e.id, e.path.clone()))
            .collect();
        if entries.is_empty() {
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            let results = cx
                .background_spawn(async move {
                    entries
                        .into_iter()
                        .map(|(id, path)| (id, inspect(&*launcher, path)))
                        .collect::<Vec<_>>()
                })
                .await;
            this.update_in(cx, |this, window, cx| {
                let mut changed = false;
                for (id, found) in results {
                    match found.info {
                        Ok(info) => {
                            this.broken.remove(&id);
                            if let Some(entry) = this.library.get_mut(id) {
                                let before = entry.clone();
                                entry.update_info(&info);
                                changed |= *entry != before;
                            }
                            this.set_icon(id, found.icon, window);
                        }
                        Err(err) => {
                            this.broken.insert(id, err);
                        }
                    }
                }
                if changed {
                    this.save_library(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn set_icon(&mut self, id: EntryId, icon: Option<Bgra>, window: &mut Window) {
        let new = icon.and_then(render_image);
        let old = match new {
            Some(image) => self.icons.insert(id, image),
            None => self.icons.remove(&id),
        };
        if let Some(old) = old {
            window.drop_image(old).ok();
        }
    }

    /// Opens the system file picker for `.jar` and `.jad` files.
    pub(super) fn pick_files(window: &mut Window, cx: &mut Context<'_, Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Add games".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = picked.await;
            this.update_in(cx, |this, window, cx| match result {
                Ok(Ok(Some(paths))) => this.add_paths(paths, false, window, cx),
                Ok(Ok(None)) | Err(_) => {}
                Ok(Err(err)) => this.report(
                    format!("Could not open the file picker: {err}"),
                    NoticeLevel::Error,
                    cx,
                ),
            })
            .ok();
        })
        .detach();
    }

    /// Adds the suites among `paths` to the library, and with `start`
    /// starts each of them too (files opened from the command line or a
    /// file manager). Files that are not suites, or that the runtime cannot
    /// read, are reported and skipped.
    pub fn add_paths(
        &mut self,
        paths: Vec<PathBuf>,
        start: bool,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let (suites, rejected) = pick_suites(&paths);
        for why in rejected {
            self.report(why.to_string(), NoticeLevel::Error, cx);
        }
        if suites.is_empty() {
            return;
        }
        let launcher = Arc::clone(&self.launcher);
        cx.spawn_in(window, async move |this, cx| {
            let results = cx
                .background_spawn(async move {
                    suites
                        .into_iter()
                        .map(|path| inspect(&*launcher, path))
                        .collect::<Vec<_>>()
                })
                .await;
            this.update_in(cx, |this, window, cx| {
                let mut added = 0;
                for found in results {
                    let name = found
                        .path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    match found.info {
                        Ok(info) => {
                            let id = if let Some(id) = this.library.find_path(&found.path) {
                                if !start {
                                    this.report(
                                        format!("{name} is in the library already"),
                                        NoticeLevel::Info,
                                        cx,
                                    );
                                }
                                id
                            } else {
                                let id = this.library.add(found.path, &info);
                                this.broken.remove(&id);
                                this.set_icon(id, found.icon, window);
                                added += 1;
                                id
                            };
                            if start {
                                this.launch(id, window, cx);
                            }
                        }
                        Err(err) => this.report(
                            format!("Could not add {name}: {err}"),
                            NoticeLevel::Error,
                            cx,
                        ),
                    }
                }
                if added > 0 {
                    this.save_library(cx);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Removes an entry, closing its running copies. Its saves stay on
    /// disk.
    pub(super) fn remove_entry(
        &mut self,
        id: EntryId,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        let tiles: Vec<_> = self
            .tiles
            .iter()
            .filter(|t| t.session.entry == id)
            .map(|t| t.id)
            .collect();
        for tile in tiles {
            self.close_tile(tile, window, cx);
        }
        if self.library.remove(id).is_some() {
            self.set_icon(id, None, window);
            self.broken.remove(&id);
            self.save_library(cx);
        }
        if self.dialog == Some(Dialog::Game(id)) {
            self.dialog = None;
        }
        self.confirm_remove = None;
        cx.notify();
    }

    pub(super) fn render_sidebar(&self, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = Theme::get(cx).clone();
        let header = div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .h(px(44.0))
            .px(px(12.0))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(icon(Icon::Phone, 16.0, theme.accent))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("FastEmu"),
                    ),
            )
            .child(
                icon_button("add-games", Icon::Plus, "Add games", &theme)
                    .on_click(cx.listener(|_, _, window, cx| Self::pick_files(window, cx))),
            );

        let list = if self.library.entries().is_empty() {
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.0))
                .px(px(20.0))
                .py(px(32.0))
                .text_center()
                .child(icon(Icon::Gamepad, 32.0, theme.muted))
                .child(
                    div()
                        .text_color(theme.muted)
                        .child("No games yet. Add .jar or .jad files, or drop them on the window."),
                )
                .child(
                    primary_button("add-first", "Add games", &theme)
                        .on_click(cx.listener(|_, _, window, cx| Self::pick_files(window, cx))),
                )
        } else {
            div().flex().flex_col().gap(px(2.0)).p(px(6.0)).children(
                self.library
                    .entries()
                    .iter()
                    .map(|entry| self.render_entry(entry.id, cx)),
            )
        };

        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .child(header)
            .child(
                div()
                    .id("library-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(list),
            )
            .child(
                div()
                    .flex_none()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .text_xs()
                    .text_color(theme.muted)
                    .child("Click a game to start it. Start it again to run another copy."),
            )
    }

    fn render_entry(&self, id: EntryId, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let Some(entry) = self.library.get(id) else {
            return div();
        };
        let running = self.tiles.iter().filter(|t| t.session.entry == id).count();
        let broken = self.broken.get(&id).cloned();
        let group = format!("entry-{}", id.0);

        if self.confirm_remove == Some(id) {
            return div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                .p(px(10.0))
                .rounded(px(8.0))
                .bg(theme.raised)
                .child(div().child(format!("Remove {} from the library?", entry.name)))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted)
                        .child("Its saves stay on disk."),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(8.0))
                        .child(
                            text_button(("remove-yes", id.0), "Remove", &theme)
                                .text_color(theme.danger)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.remove_entry(id, window, cx);
                                })),
                        )
                        .child(text_button(("remove-no", id.0), "Keep", &theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.confirm_remove = None;
                                cx.notify();
                            }),
                        )),
                );
        }

        let thumb = match self.icons.get(&id) {
            Some(image) => img(Arc::clone(image))
                .size(px(ICON_SIDE))
                .into_any_element(),
            None => icon(Icon::Gamepad, 20.0, theme.muted).into_any_element(),
        };
        let detail = match &broken {
            Some(why) => why.clone(),
            None if entry.vendor.is_empty() => entry.version.clone(),
            None if entry.version.is_empty() => entry.vendor.clone(),
            None => format!("{} {}", entry.vendor, entry.version),
        };

        div().child(
            div()
                .id(("entry", id.0))
                .group(group.clone())
                .flex()
                .items_center()
                .gap(px(10.0))
                .p(px(6.0))
                .rounded(px(8.0))
                .cursor_pointer()
                .hover(|s| s.bg(theme.hover))
                .on_click(cx.listener(move |this, _, window, cx| {
                    if let Some(why) = this.broken.get(&id).cloned() {
                        this.report(why, NoticeLevel::Error, cx);
                    } else {
                        this.launch(id, window, cx);
                    }
                }))
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .size(px(ICON_SIDE + 4.0))
                        .rounded(px(6.0))
                        .bg(theme.raised)
                        .child(thumb),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .truncate()
                                .text_color(if broken.is_some() {
                                    theme.muted
                                } else {
                                    theme.text
                                })
                                .child(entry.name.clone()),
                        )
                        .child(
                            div()
                                .truncate()
                                .text_xs()
                                .text_color(if broken.is_some() {
                                    theme.danger
                                } else {
                                    theme.muted
                                })
                                .child(detail),
                        ),
                )
                .when(running > 0, |d| {
                    d.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .rounded(px(8.0))
                            .bg(theme.accent)
                            .text_xs()
                            .text_color(theme.on_accent)
                            .child(running.to_string()),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_none()
                        .gap(px(2.0))
                        .invisible()
                        .group_hover(group, Styled::visible)
                        .child(
                            icon_button(
                                ("entry-settings", id.0),
                                Icon::Sliders,
                                "Game settings",
                                &theme,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.open_dialog(Dialog::Game(id), cx);
                                },
                            )),
                        )
                        .child(
                            icon_button(("entry-remove", id.0), Icon::Trash, "Remove", &theme)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.confirm_remove = Some(id);
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }
}
