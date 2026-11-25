//! The app on Android: the entry point GameActivity starts, the sound
//! output, and the Kotlin side's file picker and text entry around
//! fastemu-ui's views.

mod app;
mod picker;
mod text_entry;

use fastemu_audio::{AudioOutput, Engine, NullOutput, RenderBudget};
use futures::channel::mpsc;
use gpui_android::android_activity::AndroidApp;
use log::LevelFilter;

/// Where android-activity logs "Spurious ALOOPER_POLL_CALLBACK" for every
/// Choreographer callback; nothing else useful comes from it.
const SPURIOUS_VSYNC_LOG_MODULE: &str = "android_activity::activity_impl";

/// What keeps the sound engine rendering; dropping it stops the sound.
struct Sound {
    _device: Option<AudioOutput>,
    _silent: Option<NullOutput>,
}

/// Opens the phone's audio output through AAudio, or keeps time with a
/// silent output when it does not open, so games waiting for a sound to
/// end still go on.
fn start_sound(engine: &Engine) -> Sound {
    match AudioOutput::open(engine) {
        Ok(output) => {
            let info = output.info();
            log::info!(
                "android_main: sound on {} at {} Hz",
                info.device,
                info.sample_rate
            );
            Sound {
                _device: Some(output),
                _silent: None,
            }
        }
        Err(error) => {
            log::warn!("android_main: no sound: {error}; games play silently");
            Sound {
                _device: None,
                _silent: Some(NullOutput::start(engine)),
            }
        }
    }
}

/// Where android-activity hands over once GameActivity has started, on a
/// thread of its own that becomes GPUI's main thread. GPUI starts once per
/// process: the Activity ends the process when it is destroyed.
// android-activity finds the entry point by its unmangled name.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
extern "Rust" fn android_main(android_app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(LevelFilter::Info)
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter_level(LevelFilter::Info)
                    // android-activity 0.6.1 logs an error each time the
                    // looper runs the backend's vsync callback: every frame.
                    .filter_module(SPURIOUS_VSYNC_LOG_MODULE, LevelFilter::Off)
                    .build(),
            )
            .with_tag("fastemu"),
    );
    // Android drops what a panic prints to stderr.
    std::panic::set_hook(Box::new(|info| {
        log::error!(
            "panic: {info}\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }));

    let Some(files_dir) = android_app.internal_data_path() else {
        log::error!("android_main: the app has no internal storage folder");
        return;
    };
    // Screenshots go where a file manager or a USB cable reaches them.
    let shared_dir = android_app
        .external_data_path()
        .unwrap_or_else(|| files_dir.clone());
    log::info!("android_main: starting, files in {}", files_dir.display());

    // `run` returns when the Activity is destroyed, so the sound plays for
    // as long as the app does.
    let engine = Engine::default();
    // One game's sound may not take the time the others need to play.
    engine.set_render_budget(Some(RenderBudget::DEFAULT));
    let _sound = start_sound(&engine);

    // Games other apps open arrive before or after the window exists; the app
    // takes them from this queue once it runs.
    let (opened_sender, opened) = mpsc::unbounded();
    let application = gpui_android::application(android_app, fastemu_ui::Assets);
    application.on_open_urls(move |urls| {
        for url in urls {
            opened_sender.unbounded_send(url).ok();
        }
    });
    application.run(move |cx| {
        let folders = app::Folders {
            files: files_dir,
            shared: shared_dir,
        };
        app::start(cx, &folders, engine, opened);
    });
}
