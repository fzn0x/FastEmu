//! Starting the app once GPUI runs: the session, the window, and the tasks
//! that bring it games and lifecycle changes.

use std::path::Path;
use std::sync::Arc;

use fastemu_core::Launcher;
use fastemu_core::mock::MockLauncher;
use futures::StreamExt as _;
use futures::channel::mpsc::UnboundedReceiver;
use gpui::{App, AppContext as _, AppLifecyclePhase, WindowHandle, WindowOptions};

use super::picker;
use super::view::EmulatorView;
use crate::session::{self, Session};

/// What runs before the user opens a game. The mock launcher draws a test
/// pattern for any path; its title comes from the file name.
const DEMO_SUITE: &str = "FastEmu demo.jar";

/// Opens the window and starts the demo game. `opened` yields the `file://`
/// URLs of games other apps open with FastEmu.
pub(super) fn start(cx: &mut App, files_dir: &Path, opened: UnboundedReceiver<String>) {
    // The virtual machine isn't ready yet; MockLauncher stands in for it.
    let launcher: Arc<dyn Launcher> = Arc::new(MockLauncher);
    let (notify, redraws) = session::redraw_signal();
    let mut session = Session::new(launcher, files_dir.join("data"), notify);
    if let Err(error) = session.open(Path::new(DEMO_SUITE)) {
        log::error!("app: could not start the demo: {error}");
    }

    let window = cx.open_window(WindowOptions::default(), |window, cx| {
        cx.new(|cx| EmulatorView::new(session, redraws, window, cx))
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
}

fn open_games_from_other_apps(
    cx: &mut App,
    window: WindowHandle<EmulatorView>,
    mut opened: UnboundedReceiver<String>,
) {
    cx.spawn(async move |cx| {
        while let Some(url) = opened.next().await {
            let Some(path) = session::file_url_path(&url) else {
                log::warn!("app: not a file this app can open: {url}");
                continue;
            };
            let updated = window.update(cx, |view, _, cx| view.open(&path, cx));
            if updated.is_err() {
                break;
            }
        }
    })
    .detach();
}

/// Pauses the game while the app is in the background, as a phone pauses
/// a game when a call comes in.
fn follow_the_lifecycle(cx: &mut App, window: WindowHandle<EmulatorView>) {
    cx.on_app_lifecycle(move |phase, cx| {
        let result = window.update(cx, |view, _, cx| match phase {
            AppLifecyclePhase::Background | AppLifecyclePhase::Disconnected => view.pause(cx),
            AppLifecyclePhase::Foreground | AppLifecyclePhase::Active => view.resume(cx),
            AppLifecyclePhase::Inactive => {}
        });
        if let Err(error) = result {
            log::warn!("app: lifecycle change {phase:?} with no window: {error:#}");
        }
    })
    .detach();
}
