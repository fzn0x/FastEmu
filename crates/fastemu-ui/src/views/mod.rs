//! FastEmu's window: the library sidebar, the workspace of running games,
//! and the dialogs over them, all in one root view.
//!
//! The view owns every running game and redraws as a whole. While any game
//! runs it polls each instance's frame on every animation frame; when all
//! are paused or finished it sleeps until an instance's notify callback
//! wakes it.

mod input;
mod keypad;
mod library;
mod panels;
mod screen;
#[cfg(test)]
mod tests;
mod tiles;
mod touch;
mod widgets;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use fastemu_core::{InputEvent, Key, Launcher, Notify};
use futures::StreamExt;
use futures::channel::{mpsc, oneshot};
use gpui::{
    Action, App, Context, FocusHandle, Focusable, IntoElement, KeyBinding,
    KeyBindingContextPredicate, ParentElement, Render, RenderImage, SharedString, Styled,
    Subscription, Task, UniformListScrollHandle, Window, actions, div, prelude::*, px,
};

use crate::library::{EntryId, Library};
use crate::session::{AppPaths, Session};
use crate::settings::AppSettings;
use crate::store::Store;
use crate::theme::Theme;

pub use screen::{ScreenCache, render_image};

actions!(
    fastemu,
    [
        /// Opens the file picker to add games to the library.
        AddGames,
        /// Opens the app settings.
        OpenSettings,
        /// Switches between the grid and the focus layout.
        ToggleLayout,
        /// Shows or hides the on-screen keypads.
        ToggleKeypad,
        /// Shows or hides the log of the focused game.
        ToggleLog,
        /// Shows or hides the library sidebar.
        ToggleSidebar,
        /// Stops and closes the focused game.
        CloseGame,
        /// Moves keyboard focus to the next running game.
        FocusNextGame,
        /// Moves keyboard focus to the previous running game.
        FocusPreviousGame,
    ]
);

/// The key context the root view's bindings apply in.
const CONTEXT: &str = "FastEmu";

/// Registers FastEmu's global state and app shortcuts. Call once at start,
/// before opening a window.
pub fn init(cx: &mut App) {
    cx.set_global(Theme::dark());
    let context = KeyBindingContextPredicate::parse(CONTEXT).ok().map(Rc::new);
    let shortcuts: [(&str, Box<dyn Action>); 9] = [
        ("secondary-o", Box::new(AddGames)),
        ("secondary-,", Box::new(OpenSettings)),
        ("secondary-g", Box::new(ToggleLayout)),
        ("secondary-k", Box::new(ToggleKeypad)),
        ("secondary-j", Box::new(ToggleLog)),
        ("secondary-b", Box::new(ToggleSidebar)),
        ("secondary-w", Box::new(CloseGame)),
        ("secondary-]", Box::new(FocusNextGame)),
        ("secondary-[", Box::new(FocusPreviousGame)),
    ];
    // `secondary` is Cmd on macOS and Ctrl elsewhere; `load` resolves it
    // for the platform the app runs on.
    let bindings: Vec<KeyBinding> = shortcuts
        .into_iter()
        .filter_map(|(keys, action)| {
            KeyBinding::load(keys, action, context.clone(), false, None, cx)
                .inspect_err(|err| log::error!("bad shortcut {keys}: {err}"))
                .ok()
        })
        .collect();
    cx.bind_keys(bindings);
}

/// Asks the platform for a line of text with its own keyboard, for
/// devices without a hardware one: the Android app shows a dialog over
/// the window. It gets the game's title; the receiver gets the typed
/// text, or `None` when the user cancelled.
pub type TextEntry = Arc<dyn Fn(&str) -> oneshot::Receiver<Option<String>> + Send + Sync>;

/// What the host app hands the view: the runtime that starts games, the
/// folders to keep files in, and on a phone the way to type text. The
/// desktop and Android apps each pick their own.
#[derive(Clone)]
pub struct AppConfig {
    pub launcher: Arc<dyn Launcher>,
    pub paths: AppPaths,
    /// Shown when the focused game starts taking text; `None` where a
    /// keyboard is at hand.
    pub text_entry: Option<TextEntry>,
}

/// A tile's identity for the life of the window, kept across restarts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileId(u64);

/// One running game in the workspace.
pub(crate) struct Tile {
    id: TileId,
    session: Session,
    screen: screen::SharedScreen,
    /// A restart is under way; the old instance is stopped.
    restarting: bool,
    /// The app paused the game when it went to the background, and
    /// resumes it when it comes back.
    paused_by_app: bool,
    /// The platform's text entry was shown for the game's current text
    /// box.
    text_asked: bool,
}

impl Tile {
    /// Whether the link prompt covers the game's screen: the game asked
    /// to open a web page and the user has not answered yet.
    fn shows_link(&self) -> bool {
        !self.restarting && self.session.link().is_some()
    }
}

/// Which dialog covers the workspace, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialog {
    Settings(SettingsTab),
    Game(EntryId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsTab {
    General,
    Keys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NoticeLevel {
    Info,
    Error,
}

/// A short message in the corner of the window.
struct Notice {
    id: u64,
    text: SharedString,
    level: NoticeLevel,
}

/// How long a notice stays up.
const NOTICE_TIME: Duration = Duration::from_secs(6);

/// Lets instance threads wake the window without flooding it: a wake is
/// sent only when none is waiting.
struct Wake {
    pending: AtomicBool,
    tx: mpsc::UnboundedSender<()>,
}

impl Wake {
    fn callback(self: &Arc<Self>) -> Notify {
        let wake = Arc::clone(self);
        Arc::new(move |_| {
            if !wake.pending.swap(true, Ordering::AcqRel) {
                wake.tx.unbounded_send(()).ok();
            }
        })
    }
}

/// The root view of a FastEmu window.
pub struct FastEmuView {
    launcher: Arc<dyn Launcher>,
    paths: AppPaths,
    text_entry: Option<TextEntry>,
    store: Store,
    library: Library,
    settings: AppSettings,
    /// Decoded suite icons, by entry.
    icons: HashMap<EntryId, Arc<RenderImage>>,
    /// Entries whose suite could not be read, with the reason.
    broken: HashMap<EntryId, String>,
    tiles: Vec<Tile>,
    focused: Option<TileId>,
    next_tile: u64,
    /// Launches under way, so a second click picks another save slot.
    launching: Vec<(EntryId, u32)>,
    dialog: Option<Dialog>,
    /// The phone key the bindings editor waits to bind a host key to.
    capturing: Option<Key>,
    /// The entry whose removal waits for a second click.
    confirm_remove: Option<EntryId>,
    sidebar_open: bool,
    log_open: bool,
    log_scroll: UniformListScrollHandle,
    notices: Vec<Notice>,
    next_notice: u64,
    focus_handle: FocusHandle,
    wake: Arc<Wake>,
    /// Whether the last render asked for another animation frame.
    animating: bool,
    _wake_task: Task<()>,
    _subscriptions: Vec<Subscription>,
}

impl FastEmuView {
    /// Builds the view, loading the library and settings from
    /// `config.paths.config_dir`.
    pub fn new(config: AppConfig, window: &mut Window, cx: &mut Context<'_, Self>) -> Self {
        let store = Store::new(config.paths.config_dir.clone());
        let library = store.load_library();
        let settings = store.load_settings();
        cx.set_global(Theme::for_mode(settings.value.theme));

        let (tx, mut rx) = mpsc::unbounded();
        let wake = Arc::new(Wake {
            pending: AtomicBool::new(false),
            tx,
        });
        let wake_task = cx.spawn(async move |this, cx| {
            while rx.next().await.is_some() {
                let woke = this.update(cx, |this, cx| {
                    this.wake.pending.store(false, Ordering::Release);
                    if !this.animating {
                        cx.notify();
                    }
                });
                if woke.is_err() {
                    break;
                }
            }
        });

        let subscriptions = vec![
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    for tile in &mut this.tiles {
                        tile.session.release_everything();
                    }
                    cx.notify();
                }
            }),
            cx.on_app_quit(|this, _| {
                this.stop_all();
                async {}
            }),
            cx.on_release(|this, _| this.stop_all()),
        ];

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle, cx);

        let mut view = Self {
            launcher: config.launcher,
            paths: config.paths,
            text_entry: config.text_entry,
            store,
            library: library.value,
            settings: settings.value,
            icons: HashMap::new(),
            broken: HashMap::new(),
            tiles: Vec::new(),
            focused: None,
            next_tile: 1,
            launching: Vec::new(),
            dialog: None,
            capturing: None,
            confirm_remove: None,
            sidebar_open: true,
            log_open: false,
            log_scroll: UniformListScrollHandle::new(),
            notices: Vec::new(),
            next_notice: 1,
            focus_handle,
            wake,
            animating: false,
            _wake_task: wake_task,
            _subscriptions: subscriptions,
        };
        for problem in [library.problem, settings.problem].into_iter().flatten() {
            view.report(problem, NoticeLevel::Error, cx);
        }
        view.refresh_library(window, cx);
        view
    }

    /// Pauses every running game while the app is in the background, as a
    /// phone pauses its game for a call, and resumes those when the app is
    /// back. Games the user paused stay paused.
    pub fn set_background(&mut self, background: bool, cx: &mut Context<'_, Self>) {
        let now = cx.background_executor().now();
        for tile in &mut self.tiles {
            if background && tile.session.is_running() {
                tile.session.release_everything();
                tile.session.instance.pause();
                tile.paused_by_app = true;
            } else if !background && tile.paused_by_app {
                tile.session.instance.resume();
                tile.paused_by_app = false;
            }
            tile.session.poll(now);
        }
        cx.notify();
    }

    /// Shows the platform's text entry when the focused game starts taking
    /// text, once each time it does, and types the answer into the game.
    /// Once the entry is answered or cancelled, the game goes back to
    /// keys, so the Text button opens the entry again with one press.
    fn ask_for_text(&mut self, cx: &mut Context<'_, Self>) {
        let Some(entry) = self.text_entry.clone() else {
            return;
        };
        let focused = self.focused;
        for tile in &mut self.tiles {
            if !(tile.session.typing_text() && tile.session.is_running()) {
                tile.text_asked = false;
                continue;
            }
            if tile.text_asked || Some(tile.id) != focused {
                continue;
            }
            tile.text_asked = true;
            let answer = entry(&tile.session.title);
            let instance = Arc::clone(&tile.session.instance);
            let tile_id = tile.id;
            cx.spawn(async move |this, cx| {
                if let Ok(Some(text)) = answer.await
                    && !text.is_empty()
                {
                    instance.send(InputEvent::Text(text));
                }
                // The view may be gone, or the tile closed or restarted.
                let _ = this.update(cx, |this, cx| {
                    if let Some(tile) = this.tiles.iter_mut().find(|t| t.id == tile_id)
                        && tile.session.instance.id() == instance.id()
                    {
                        tile.session.stop_typing();
                        cx.notify();
                    }
                });
            })
            .detach();
        }
    }

    /// Stops every running game. The app calls this on quit; it is safe to
    /// call more than once.
    pub fn stop_all(&mut self) {
        for tile in &self.tiles {
            tile.session.instance.stop();
        }
    }

    /// Shows a message in the corner for a few seconds.
    fn report(
        &mut self,
        text: impl Into<SharedString>,
        level: NoticeLevel,
        cx: &mut Context<'_, Self>,
    ) {
        let text = text.into();
        if level == NoticeLevel::Error {
            log::warn!("{text}");
        }
        let id = self.next_notice;
        self.next_notice += 1;
        self.notices.push(Notice { id, text, level });
        if self.notices.len() > 4 {
            self.notices.remove(0);
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTICE_TIME).await;
            this.update(cx, |this, cx| {
                this.notices.retain(|n| n.id != id);
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn save_library(&mut self, cx: &mut Context<'_, Self>) {
        if let Err(err) = self.store.save_library(&self.library) {
            self.report(
                format!("Could not save the library: {err}"),
                NoticeLevel::Error,
                cx,
            );
        }
    }

    /// Applies `change` to the settings, saves them and repaints.
    fn update_settings(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut AppSettings),
    ) {
        let theme = self.settings.theme;
        change(&mut self.settings);
        if self.settings.theme != theme {
            cx.set_global(Theme::for_mode(self.settings.theme));
        }
        if let Err(err) = self.store.save_settings(&self.settings) {
            self.report(
                format!("Could not save the settings: {err}"),
                NoticeLevel::Error,
                cx,
            );
        }
        cx.notify();
    }

    fn open_dialog(&mut self, dialog: Dialog, cx: &mut Context<'_, Self>) {
        self.release_focused_keys();
        self.dialog = Some(dialog);
        self.capturing = None;
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<'_, Self>) {
        self.dialog = None;
        self.capturing = None;
        cx.notify();
    }

    fn render_notices(&self, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = Theme::get(cx).clone();
        div()
            .absolute()
            .bottom(px(16.0))
            .right(px(16.0))
            .flex()
            .flex_col()
            .items_end()
            .gap(px(8.0))
            .max_w(px(420.0))
            .children(self.notices.iter().map(|notice| {
                let id = notice.id;
                let (mark, color) = match notice.level {
                    NoticeLevel::Info => (crate::assets::Icon::Info, theme.accent),
                    NoticeLevel::Error => (crate::assets::Icon::Warning, theme.danger),
                };
                div()
                    .id(("notice", id))
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .px(px(12.0))
                    .py(px(10.0))
                    .rounded(px(8.0))
                    .bg(theme.raised)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_md()
                    .text_sm()
                    .text_color(theme.text)
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.notices.retain(|n| n.id != id);
                        cx.notify();
                    }))
                    .child(widgets::icon(mark, 16.0, color))
                    .child(div().min_w_0().child(notice.text.clone()))
            }))
    }

    fn render_drop_hint(cx: &mut Context<'_, Self>) -> Option<impl IntoElement> {
        cx.active_drag_value::<gpui::ExternalPaths>()?;
        let theme = Theme::get(cx);
        Some(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.scrim)
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(10.0))
                        .px(px(40.0))
                        .py(px(28.0))
                        .rounded(px(12.0))
                        .border_2()
                        .border_dashed()
                        .border_color(theme.accent)
                        .bg(theme.surface)
                        .child(widgets::icon(
                            crate::assets::Icon::FilePlus,
                            28.0,
                            theme.accent,
                        ))
                        .child(
                            div()
                                .text_color(theme.text)
                                .child("Drop .jar or .jad files to add them"),
                        ),
                ),
        )
    }
}

impl Focusable for FastEmuView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FastEmuView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let now = cx.background_executor().now();
        self.poll_tiles(now);
        self.ask_for_text(cx);
        // A running game asks for no frames of its own: its notification
        // wakes the window when it has a new frame, status, log lines or
        // link. A link prompt that does not take clicks yet is dimmed, and
        // gets its frames until it does, even over a game that has ended.
        self.animating = self
            .tiles
            .iter()
            .any(|t| t.shows_link() && !t.session.link_armed(now));
        if self.animating {
            window.request_animation_frame();
        }

        let theme = Theme::get(cx).clone();
        let workspace = self.render_main(window, cx);
        // A phone's status bar, navigation bar and camera cutout cover the
        // window's edges; nothing goes under them.
        let safe = window.insets().safe_area;
        div()
            .id("fastemu")
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::on_key_down))
            .on_key_up(cx.listener(Self::on_key_up))
            .on_action(cx.listener(|_, _: &AddGames, window, cx| Self::pick_files(window, cx)))
            .on_action(cx.listener(|this, _: &OpenSettings, _, cx| {
                this.open_dialog(Dialog::Settings(SettingsTab::General), cx);
            }))
            .on_action(cx.listener(|this, _: &ToggleLayout, _, cx| this.toggle_layout(cx)))
            .on_action(cx.listener(|this, _: &ToggleKeypad, _, cx| {
                this.update_settings(cx, |s| s.keypad = !s.keypad);
            }))
            .on_action(cx.listener(|this, _: &ToggleLog, _, cx| this.toggle_log(cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| {
                this.sidebar_open = !this.sidebar_open;
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &CloseGame, window, cx| {
                if let Some(id) = this.focused {
                    this.close_tile(id, window, cx);
                }
            }))
            .on_action(cx.listener(|this, _: &FocusNextGame, _, cx| this.cycle_focus(1, cx)))
            .on_action(cx.listener(|this, _: &FocusPreviousGame, _, cx| {
                this.cycle_focus(-1, cx);
            }))
            .on_drop(
                cx.listener(|this, paths: &gpui::ExternalPaths, window, cx| {
                    this.add_paths(paths.paths().to_vec(), false, window, cx);
                }),
            )
            .relative()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_row()
            .bg(theme.background)
            .text_color(theme.text)
            .text_sm()
            .when(self.sidebar_open, |d| d.child(self.render_sidebar(cx)))
            .child(workspace)
            .children(self.render_dialog(window, cx))
            .child(self.render_notices(cx))
            .children(Self::render_drop_hint(cx))
    }
}
