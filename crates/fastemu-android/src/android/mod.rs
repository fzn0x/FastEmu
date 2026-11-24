//! The app on Android: the entry point GameActivity starts, the Kotlin side's
//! file picker, and the GPUI view.
//!
//! `app` wires the parts together and is what changes when fastemu-ui's views
//! replace [`view::EmulatorView`]; `picker` and the open-file handling stay.

mod app;
mod frame_image;
mod picker;
mod view;

use futures::channel::mpsc;
use gpui_android::android_activity::AndroidApp;
use log::LevelFilter;

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
    log::info!("android_main: starting, files in {}", files_dir.display());

    // Games other apps open arrive before or after the window exists; the app
    // takes them from this queue once it runs.
    let (opened_sender, opened) = mpsc::unbounded();
    let application = gpui_android::application(android_app, ());
    application.on_open_urls(move |urls| {
        for url in urls {
            opened_sender.unbounded_send(url).ok();
        }
    });
    application.run(move |cx| app::start(cx, &files_dir, opened));
}
