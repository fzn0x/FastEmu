//! The FastEmu desktop app: parses the command line, picks the launcher
//! and the folders, and opens the window. Everything inside the window
//! lives in `fastemu-ui`, which the Android app shares. `fastemu run`
//! runs a game without a window (see `run`).
//!
//! On Windows the program is a GUI program, which opens no console window
//! (see `console`).

// fastemu.exe opens no console window when started from Explorer or a
// shortcut; `console::attach` finds the terminal that starts it.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod audio;
mod cli;
mod console;
mod logger;
mod paths;
mod run;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use fastemu_core::Launcher;
use fastemu_midp::MidpLauncher;
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
    console::attach();
    match cli::parse(std::env::args_os().skip(1)) {
        Ok(Command::Open { files, mock }) => {
            logger::init();
            open_window(files, mock);
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
            // The option is as typed, or as a file name expanded it.
            let err = fastemu_core::escape_control(err.to_string());
            eprintln!("fastemu: {err}\n\n{}", cli::USAGE);
            ExitCode::from(2)
        }
    }
}

/// The runtime that starts games and what plays their sound, which must
/// live as long as the app. With `mock`, the mock launcher's test
/// patterns stand in for games, for working on the window without a
/// game at hand; they make no sound.
fn launcher(mock: bool) -> (Arc<dyn Launcher>, Option<audio::Output>) {
    if mock {
        return (Arc::new(fastemu_core::mock::MockLauncher), None);
    }
    let engine = fastemu_audio::Engine::default();
    // One game's sound may not take the time the others need to play.
    engine.set_render_budget(Some(fastemu_audio::RenderBudget::DEFAULT));
    let output = audio::start(&engine);
    (Arc::new(MidpLauncher::new(Some(engine))), Some(output))
}

fn open_window(files: Vec<PathBuf>, mock: bool) {
    // `run` returns when the app quits (or never, on macOS), so the sound
    // plays for as long as the window is open.
    let (launcher, _output) = launcher(mock);
    let config = AppConfig {
        launcher,
        paths: paths::desktop(),
        text_entry: None,
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
