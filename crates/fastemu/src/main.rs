//! The FastEmu desktop app: parses the command line, picks the launcher
//! and the folders, and opens the window. Everything inside the window
//! lives in `fastemu-ui`, which the Android app shares. `fastemu run`
//! runs a game without a window (see `run`).

mod cli;
mod paths;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use fastemu_core::Launcher;
use fastemu_ui::{AppConfig, Assets, FastEmuView};
use gpui::{
    App, AppContext, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, actions, px,
    size,
};

use crate::cli::Command;

actions!(
    fastemu,
    [
        /// Stops every game and closes the app.
        Quit
    ]
);

fn main() -> ExitCode {
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(Command::Open { files }) => {
            open_window(files);
            ExitCode::SUCCESS
        }
        Ok(Command::Version) => {
            println!("fastemu {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(Command::Help) => {
            print!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        Ok(Command::Run { args }) => run::main(args),
        Err(err) => {
            eprintln!("fastemu: {err}\n\n{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}

/// The runtime that starts games. Until the virtual machine is ready this
/// is the mock launcher, whose games draw a test pattern; the real
/// launcher from fastemu-midp replaces it here.
fn launcher() -> Arc<dyn Launcher> {
    Arc::new(fastemu_core::mock::MockLauncher)
}

fn open_window(files: Vec<PathBuf>) {
    let config = AppConfig {
        launcher: launcher(),
        paths: paths::desktop(),
    };
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            fastemu_ui::init(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            if cfg!(target_os = "macos") {
                cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            }
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();

            let bounds = Bounds::centered(None, size(px(1280.0), px(820.0)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("FastEmu".into()),
                    ..TitlebarOptions::default()
                }),
                window_min_size: Some(size(px(640.0), px(480.0))),
                app_id: Some("io.github.fzn0x.fastemu".into()),
                ..WindowOptions::default()
            };
            let opened = cx.open_window(options, |window, cx| {
                cx.new(|cx| FastEmuView::new(config, window, cx))
            });
            match opened {
                Ok(handle) => {
                    if !files.is_empty() {
                        handle
                            .update(cx, |view, window, cx| {
                                view.add_paths(files, true, window, cx);
                            })
                            .ok();
                    }
                    cx.activate(true);
                }
                Err(err) => {
                    eprintln!("fastemu: could not open a window: {err:#}");
                    cx.quit();
                }
            }
        });
}
