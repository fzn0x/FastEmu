//! Starting the app once GPUI runs: fastemu-ui's window with the real
//! launcher, and the tasks that bring it games, lifecycle changes and a
//! screen that stays on.

use std::path::PathBuf;
use std::sync::Arc;

use fastemu_audio::Engine;
use fastemu_midp::MidpLauncher;
use fastemu_ui::{AppConfig, AppPaths, FastEmuView};
use futures::StreamExt as _;
use futures::channel::mpsc::UnboundedReceiver;
use gpui::{App, AppContext as _, AppLifecyclePhase, WindowHandle, WindowOptions};

use super::{picker, text_entry};
use crate::files;

/// The app's folders.
pub(super) struct Folders {
    /// Private storage: settings, the library, saves and game copies.
    pub files: PathBuf,
    /// Storage a file manager reaches, for screenshots.
    pub shared: PathBuf,
}

impl Folders {
    fn app_paths(&self) -> AppPaths {
        AppPaths {
            config_dir: self.files.join("config"),
            data_dir: self.files.join("data"),
            screenshot_dir: self.shared.join("screenshots"),
        }
    }
}

/// Opens the window. Games play through `engine`; `opened` yields the
/// `file://` URLs of games other apps open with FastEmu.
pub(super) fn start(
    cx: &mut App,
    folders: &Folders,
    engine: Engine,
    opened: UnboundedReceiver<String>,
) {
    fastemu_ui::init(cx);
    let config = AppConfig {
        launcher: Arc::new(MidpLauncher::new(Some(engine))),
        paths: folders.app_paths(),
        text_entry: Some(text_entry::entry()),
    };
    let window = cx.open_window(WindowOptions::default(), |window, cx| {
        cx.new(|cx| FastEmuView::new(config, window, cx))
    });
    let window = match window {
        Ok(window) => window,
        Err(error) => {
            log::error!("app: could not open the window: {error:#}");
            return;
        }
    };

    picker::serve(cx);
    open_games_from_other_apps(cx, window, opened);
    follow_the_lifecycle(cx, window);
    keep_the_screen_on(cx);
}

/// Adds and starts each game another app opens with FastEmu.
fn open_games_from_other_apps(
    cx: &mut App,
    window: WindowHandle<FastEmuView>,
    mut opened: UnboundedReceiver<String>,
) {
    cx.spawn(async move |cx| {
        while let Some(url) = opened.next().await {
            let Some(path) = files::file_url_path(&url) else {
                log::warn!("app: not a file this app can open: {url}");
                continue;
            };
            log::info!("app: opening {}", path.display());
            let updated = window.update(cx, |view, window, cx| {
                view.add_paths(vec![path], true, window, cx);
            });
            if updated.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// Pauses the games while the app is in the background, as a phone pauses
/// a game when a call comes in, and resumes them when it comes back.
fn follow_the_lifecycle(cx: &mut App, window: WindowHandle<FastEmuView>) {
    cx.on_app_lifecycle(move |phase, cx| {
        let result = window.update(cx, |view, _, cx| match phase {
            AppLifecyclePhase::Background | AppLifecyclePhase::Disconnected => {
                view.set_background(true, cx);
            }
            AppLifecyclePhase::Foreground | AppLifecyclePhase::Active => {
                view.set_background(false, cx);
            }
            AppLifecyclePhase::Inactive => {}
        });
        if let Err(error) = result {
            log::warn!("app: lifecycle change {phase:?} with no window: {error:#}");
        }
    })
    .detach();
}

/// Keeps the screen from dimming while the app is in front: games are
/// played without touching the screen for minutes.
fn keep_the_screen_on(cx: &mut App) {
    let awake = cx.prevent_idle_sleep("games are on screen");
    cx.spawn(async move |_| match awake.await {
        Ok(guard) => {
            // Never finishes: the guard lasts as long as the app.
            futures::future::pending::<()>().await;
            drop(guard);
        }
        Err(error) => log::warn!("app: the screen may turn off: {error:#}"),
    })
    .detach();
}
