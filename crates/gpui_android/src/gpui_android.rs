#![cfg(target_os = "android")]
//! Android backend for the GPUI Platform trait.
//!
//! Modified for FastEmu from `crates/gpui_android` of the Zed Android port
//! (github.com/Dylanmurzello/zed-android-port, revision 746f088). FastEmu
//! shows one window, so the extra-window Activities, the IME bridge, the
//! Ubuntu runtime and the editor's own services are gone; README.md lists
//! every change.

// gpui's platform callbacks are boxed closures, which the window and the
// platform keep as upstream does.
#![allow(clippy::type_complexity)]

mod dispatcher;
mod display;
mod events;
mod insets;
mod jni_env;
mod keyboard;
mod open_files;
mod platform;
mod touch;
mod window;

/// The android-activity this backend is built against, so the app's
/// `android_main(app: AndroidApp)` names the same type.
pub use android_activity;
pub use jni_env::{android_app, with_activity};
pub use platform::{AndroidPlatform, PathPrompt, path_prompts};

use std::rc::Rc;

/// Run a gpui application backed by Android's GameActivity event loop.
///
/// Call this from your `android_main` no_mangle entry point. It constructs
/// the `AndroidPlatform`, hands it to `gpui::Application`, and drives the
/// run loop. Replaces the desktop/web `gpui_platform::application()` path
/// because we need to thread the `AndroidApp` through Platform construction.
///
/// `assets` is the source the SVG renderer uses to resolve asset paths.
pub fn run<A, F>(android_app: android_activity::AndroidApp, assets: A, on_finish_launching: F)
where
    A: gpui::AssetSource,
    F: 'static + FnOnce(&mut gpui::App),
{
    application(android_app, assets).run(on_finish_launching);
}

/// The `gpui::Application` [`run`] drives, for an app that registers
/// callbacks such as `on_open_urls` before running it.
pub fn application<A>(android_app: android_activity::AndroidApp, assets: A) -> gpui::Application
where
    A: gpui::AssetSource,
{
    // For code that runs inside the gpui App with no access to the
    // AndroidApp passed in here.
    jni_env::register_android_app(android_app.clone());
    let platform: Rc<dyn gpui::Platform> = Rc::new(AndroidPlatform::new(android_app));
    gpui::Application::with_platform(platform).with_assets(assets)
}
