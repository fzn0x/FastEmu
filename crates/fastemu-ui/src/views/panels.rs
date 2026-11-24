//! The dialogs over the workspace (app settings, key bindings, a game's
//! settings) and the log drawer under it.

use std::ops::Range;

use fastemu_core::{Key, LogLevel, builtin_profiles};
use gpui::{
    AnyElement, Context, Div, IntoElement, MouseButton, ParentElement, SharedString, Styled,
    Window, div, prelude::*, px, uniform_list,
};

use super::widgets::{chip, heading, icon, icon_button, setting_row, switch, text_button};
use super::{Dialog, FastEmuView, SettingsTab};
use crate::assets::Icon;
use crate::bindings::{KeyBindings, host_label};
use crate::library::EntryId;
use crate::settings::{ENCODINGS, GameSettings, LayoutMode, ThemeMode, profile_by_id};
use crate::theme::Theme;

/// The log drawer's height.
pub(super) const LOG_HEIGHT: f32 = 200.0;
const LOG_ROW_HEIGHT: f32 = 18.0;

impl FastEmuView {
    pub(super) fn render_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Option<impl IntoElement> {
        let dialog = self.dialog?;
        let theme = Theme::get(cx).clone();
        let (title, tabs, body): (SharedString, Option<Div>, AnyElement) = match dialog {
            Dialog::Settings(tab) => (
                "Settings".into(),
                Some(Self::render_tabs(tab, cx)),
                match tab {
                    SettingsTab::General => self.render_general(cx).into_any_element(),
                    SettingsTab::Keys => self.render_keys(cx).into_any_element(),
                },
            ),
            Dialog::Game(id) => {
                let name = self.library.get(id)?.name.clone();
                (
                    name.into(),
                    None,
                    self.render_game_settings(id, cx)?.into_any_element(),
                )
            }
        };
        let max_height = f32::from(window.viewport_size().height) * 0.85;
        Some(
            div()
                .id("dialog-scrim")
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.scrim)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.close_dialog(cx)),
                )
                .child(
                    div()
                        .id("dialog")
                        .flex()
                        .flex_col()
                        .w(px(560.0))
                        .max_h(px(max_height))
                        .rounded(px(12.0))
                        .bg(theme.surface)
                        .border_1()
                        .border_color(theme.border)
                        .shadow_lg()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .flex()
                                .flex_none()
                                .items_center()
                                .gap(px(12.0))
                                .px(px(16.0))
                                .h(px(48.0))
                                .border_b_1()
                                .border_color(theme.border)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child(title),
                                )
                                .children(tabs)
                                .child(
                                    icon_button("close-dialog", Icon::Close, "Close", &theme)
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.close_dialog(cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .id("dialog-body")
                                .flex_1()
                                .min_h_0()
                                .overflow_y_scroll()
                                .px(px(16.0))
                                .pb(px(16.0))
                                .child(body),
                        ),
                ),
        )
    }

    fn render_tabs(current: SettingsTab, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        div()
            .flex()
            .gap(px(6.0))
            .child(
                chip(
                    "tab-general",
                    "General",
                    current == SettingsTab::General,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.open_dialog(Dialog::Settings(SettingsTab::General), cx);
                })),
            )
            .child(
                chip("tab-keys", "Keys", current == SettingsTab::Keys, &theme).on_click(
                    cx.listener(|this, _, _, cx| {
                        this.open_dialog(Dialog::Settings(SettingsTab::Keys), cx);
                    }),
                ),
            )
    }

    fn render_general(&self, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let s = &self.settings;
        let theme_choice = div()
            .flex()
            .gap(px(6.0))
            .child(
                chip("theme-dark", "Dark", s.theme == ThemeMode::Dark, &theme)
                    .child(icon(Icon::Moon, 13.0, theme.muted))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.update_settings(cx, |s| s.theme = ThemeMode::Dark);
                    })),
            )
            .child(
                chip("theme-light", "Light", s.theme == ThemeMode::Light, &theme)
                    .child(icon(Icon::Sun, 13.0, theme.muted))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.update_settings(cx, |s| s.theme = ThemeMode::Light);
                    })),
            );
        let layout_choice = div()
            .flex()
            .gap(px(6.0))
            .child(
                chip(
                    "layout-grid-choice",
                    "Grid",
                    s.layout == LayoutMode::Grid,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_settings(cx, |s| s.layout = LayoutMode::Grid);
                })),
            )
            .child(
                chip(
                    "layout-focus-choice",
                    "Focus",
                    s.layout == LayoutMode::Focus,
                    &theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.update_settings(cx, |s| s.layout = LayoutMode::Focus);
                })),
            );
        let default_profile = s.default_profile.clone();
        div()
            .flex()
            .flex_col()
            .child(heading("Appearance", &theme))
            .child(setting_row("Theme", None, theme_choice, &theme))
            .child(heading("Workspace", &theme))
            .child(setting_row(
                "Layout",
                Some("Grid sizes all games alike; focus enlarges one".into()),
                layout_choice,
                &theme,
            ))
            .child(setting_row(
                "On-screen keypad",
                Some("Soft keys, D-pad and number keys beside each game".into()),
                switch("keypad-switch", s.keypad, &theme).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.update_settings(cx, |s| s.keypad = !s.keypad);
                    },
                )),
                &theme,
            ))
            .child(heading("Default phone", &theme))
            .child(
                div()
                    .text_xs()
                    .text_color(theme.muted)
                    .pb(px(6.0))
                    .child("Games start on this phone unless their own settings pick another."),
            )
            .child(Self::render_profiles(
                "default-profile",
                Some(default_profile.as_str()),
                None,
                cx,
                |this, id, cx| this.update_settings(cx, |s| s.default_profile = id),
            ))
    }

    /// A list of the built-in phones with the selected one marked. With
    /// `default_label`, the list starts with a "use the app default" row,
    /// selected when `selected` is `None`.
    fn render_profiles(
        list: &'static str,
        selected: Option<&str>,
        default_label: Option<String>,
        cx: &mut Context<'_, Self>,
        pick: impl Fn(&mut Self, String, &mut Context<'_, Self>) + Clone + 'static,
    ) -> Div {
        let theme = Theme::get(cx).clone();
        let row = |index: usize, label: String, detail: String, on: bool| {
            div()
                .id((list, index))
                .flex()
                .items_center()
                .gap(px(10.0))
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .cursor_pointer()
                .when(on, |d| d.bg(theme.raised))
                .hover(|s| s.bg(theme.hover))
                .child(
                    div()
                        .size(px(16.0))
                        .flex_none()
                        .children(on.then(|| icon(Icon::Check, 16.0, theme.accent))),
                )
                .child(div().flex_1().min_w_0().truncate().child(label))
                .child(div().text_xs().text_color(theme.muted).child(detail))
        };
        let mut rows = Vec::new();
        if let Some(label) = default_label {
            let pick = pick.clone();
            rows.push(
                row(0, label, String::new(), selected.is_none())
                    .on_click(cx.listener(move |this, _, _, cx| pick(this, String::new(), cx))),
            );
        }
        for (i, profile) in builtin_profiles().into_iter().enumerate() {
            let detail = format!(
                "{}x{}{}",
                profile.screen_width,
                profile.screen_height,
                if profile.touch { ", touch" } else { "" }
            );
            let on = selected == Some(profile.id.as_str());
            let pick = pick.clone();
            let id = profile.id.clone();
            rows.push(
                row(i + 1, profile.name, detail, on)
                    .on_click(cx.listener(move |this, _, _, cx| pick(this, id.clone(), cx))),
            );
        }
        div().flex().flex_col().gap(px(2.0)).children(rows)
    }

    fn render_keys(&self, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let bindings = &self.settings.bindings;
        let rows = Key::ALL.iter().map(|&key| {
            let capturing = self.capturing == Some(key);
            let hosts = bindings.hosts(key).to_vec();
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .py(px(5.0))
                .border_b_1()
                .border_color(theme.border)
                .child(div().w(px(120.0)).flex_none().child(key.label()))
                .child(div().flex().flex_1().flex_wrap().gap(px(6.0)).children(
                    hosts.into_iter().enumerate().map(|(i, host)| {
                        let label = host_label(&host);
                        div()
                            .id(SharedString::from(format!("bound-{key:?}-{i}")))
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .h(px(24.0))
                            .pl(px(8.0))
                            .pr(px(4.0))
                            .rounded(px(5.0))
                            .bg(theme.raised)
                            .child(label)
                            .child(
                                div()
                                    .id("unbind")
                                    .rounded(px(4.0))
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.hover))
                                    .child(icon(Icon::Close, 12.0, theme.muted))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let host = host.clone();
                                        this.update_settings(cx, |s| {
                                            s.bindings.unbind(key, &host);
                                        });
                                    })),
                            )
                    }),
                ))
                .child(
                    text_button(
                        SharedString::from(format!("bind-{key:?}")),
                        if capturing { "Press a key" } else { "Add" },
                        &theme,
                    )
                    .when(capturing, |d| d.border_1().border_color(theme.accent))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.capturing = if this.capturing == Some(key) {
                            None
                        } else {
                            Some(key)
                        };
                        cx.notify();
                    })),
                )
        });
        div()
            .flex()
            .flex_col()
            .child(div().pt(px(12.0)).pb(px(8.0)).text_xs().text_color(theme.muted).child(
                "Click Add, then press a key on your keyboard (Esc cancels). Each keyboard key \
                 presses one phone key; the number row and the numeric keypad share names.",
            ))
            .children(rows)
            .child(
                div().pt(px(12.0)).child(
                    text_button("reset-keys", "Reset to defaults", &theme).on_click(cx.listener(
                        |this, _, _, cx| {
                            this.capturing = None;
                            this.update_settings(cx, |s| s.bindings = KeyBindings::default());
                        },
                    )),
                ),
            )
    }

    /// Applies `change` to a game's settings, saves the library, and sets
    /// the new volume on its running copies at once.
    pub(super) fn update_game_settings(
        &mut self,
        id: EntryId,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut GameSettings),
    ) {
        let Some(entry) = self.library.get_mut(id) else {
            return;
        };
        change(&mut entry.settings);
        let volume = entry.settings.volume;
        for tile in self.tiles.iter().filter(|t| t.session.entry == id) {
            if !tile.session.muted {
                tile.session.instance.set_volume(volume);
            }
        }
        self.save_library(cx);
        cx.notify();
    }

    fn render_game_settings(&self, id: EntryId, cx: &mut Context<'_, Self>) -> Option<Div> {
        let theme = Theme::get(cx).clone();
        let entry = self.library.get(id)?;
        let settings = entry.settings.clone();
        let default_name = profile_by_id(&self.settings.default_profile).name;
        let percent = (settings.volume * 100.0).round() as u32;
        let volume = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                icon_button("volume-down", Icon::VolumeOff, "Quieter", &theme).on_click(
                    cx.listener(move |this, _, _, cx| {
                        this.update_game_settings(id, cx, |s| s.step_volume(-0.1));
                    }),
                ),
            )
            .child(div().flex().gap(px(2.0)).children((1..=10).map(|step| {
                div()
                    .w(px(8.0))
                    .h(px(14.0))
                    .rounded(px(2.0))
                    .bg(if step * 10 <= percent {
                        theme.accent
                    } else {
                        theme.raised
                    })
            })))
            .child(
                icon_button("volume-up", Icon::Volume, "Louder", &theme).on_click(cx.listener(
                    move |this, _, _, cx| {
                        this.update_game_settings(id, cx, |s| s.step_volume(0.1));
                    },
                )),
            )
            .child(
                div()
                    .w(px(40.0))
                    .text_xs()
                    .text_color(theme.muted)
                    .child(format!("{percent}%")),
            );
        let encodings =
            div()
                .flex()
                .flex_wrap()
                .gap(px(6.0))
                .children(ENCODINGS.iter().enumerate().map(|(i, &name)| {
                    chip(("encoding", i), name, settings.encoding == name, &theme).on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.update_game_settings(id, cx, |s| name.clone_into(&mut s.encoding));
                        }),
                    )
                }));
        Some(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .pt(px(10.0))
                        .text_xs()
                        .text_color(theme.muted)
                        .truncate()
                        .child(entry.path.display().to_string()),
                )
                .child(heading("Sound", &theme))
                .child(setting_row("Volume", None, volume, &theme))
                .child(heading("Network", &theme))
                .child(setting_row(
                    "Allow network access",
                    Some("Off, connections fail as if the phone had no signal".into()),
                    switch("network-switch", settings.network, &theme).on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.update_game_settings(id, cx, |s| s.network = !s.network);
                        },
                    )),
                    &theme,
                ))
                .child(heading("Text encoding", &theme))
                .child(
                    div()
                        .pb(px(4.0))
                        .text_xs()
                        .text_color(theme.muted)
                        .child("Games from some regions expect their phones' encoding."),
                )
                .child(encodings)
                .child(heading("Phone", &theme))
                .child(Self::render_profiles(
                    "game-profile",
                    settings.profile.as_deref(),
                    Some(format!("App default ({default_name})")),
                    cx,
                    move |this, profile, cx| {
                        this.update_game_settings(id, cx, |s| {
                            s.profile = (!profile.is_empty()).then_some(profile);
                        });
                    },
                ))
                .child(div().pt(px(12.0)).text_xs().text_color(theme.muted).child(
                    "Phone, network and encoding apply when the game starts again; \
                             volume applies at once.",
                )),
        )
    }

    pub(super) fn render_log(&mut self, cx: &mut Context<'_, Self>) -> Div {
        let theme = Theme::get(cx).clone();
        let focused = self.focused;
        let tile = focused.and_then(|id| self.tiles.iter().find(|t| t.id == id));
        let (title, count) = tile.map_or_else(
            || ("No game selected".to_owned(), 0),
            |t| (format!("Log of {}", t.session.title), t.session.log.len()),
        );
        let list = uniform_list(
            "log-lines",
            count,
            cx.processor(move |this, range: Range<usize>, _, cx| {
                let theme = Theme::get(cx).clone();
                let Some(tile) = focused.and_then(|id| this.tiles.iter().find(|t| t.id == id))
                else {
                    return Vec::new();
                };
                range
                    .filter_map(|i| tile.session.log.get(i))
                    .map(|line| {
                        let color = match line.level {
                            LogLevel::Out => theme.text,
                            LogLevel::Err => theme.danger,
                            LogLevel::Warn => theme.warning,
                            LogLevel::Info => theme.muted,
                        };
                        div()
                            .h(px(LOG_ROW_HEIGHT))
                            .px(px(12.0))
                            .whitespace_nowrap()
                            .text_color(color)
                            .child(line.text.clone())
                    })
                    .collect()
            }),
        )
        .track_scroll(&self.log_scroll)
        .flex_1()
        .font_family(Theme::mono_font())
        .text_xs();
        div()
            .flex()
            .flex_col()
            .flex_none()
            .h(px(LOG_HEIGHT))
            .bg(theme.surface)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(4.0))
                    .h(px(32.0))
                    .px(px(8.0))
                    .child(icon(Icon::Log, 14.0, theme.muted))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(theme.muted)
                            .child(title),
                    )
                    .child(
                        icon_button("clear-log", Icon::Trash, "Clear the log", &theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                if let Some(tile) = this.focused_tile_mut() {
                                    tile.session.log.clear();
                                }
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        icon_button("close-log", Icon::Close, "Hide the log", &theme)
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_log(cx))),
                    ),
            )
            .child(list)
    }
}
