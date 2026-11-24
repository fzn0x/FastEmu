//! Files other apps open with FastEmu, such as "Open with" in the Files app.
//! The app's Activity receives the intent, copies the file into app storage
//! and passes its `file://` URL here; the run loop hands the URLs to gpui's
//! `on_open_urls` callback.
//!
//! Modified for FastEmu: the JNI entry point belongs to
//! `io.github.fzn0x.fastemu.NativeBridge`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use jni::JNIEnv;
use jni::objects::{JClass, JString};

static PENDING_URLS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Lets the run loop skip the lock on every iteration while nothing is
/// pending.
static HAS_PENDING_URLS: AtomicBool = AtomicBool::new(false);

pub(crate) fn take_pending_urls() -> Option<Vec<String>> {
    if !HAS_PENDING_URLS.swap(false, Ordering::AcqRel) {
        return None;
    }
    let urls = std::mem::take(&mut *PENDING_URLS.lock().unwrap_or_else(PoisonError::into_inner));
    (!urls.is_empty()).then_some(urls)
}

/// `NativeBridge.nativeOpenUrl`, on any thread. It can arrive before the app
/// has started: the URL waits until the run loop delivers it.
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_fzn0x_fastemu_NativeBridge_nativeOpenUrl<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    url: JString<'local>,
) {
    let url: String = match env.get_string(&url) {
        Ok(url) => url.into(),
        Err(error) => {
            log::warn!("open_files: decode the URL: {error}");
            return;
        }
    };
    log::info!("open_files: opening {url}");
    PENDING_URLS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(url);
    HAS_PENDING_URLS.store(true, Ordering::Release);
    crate::platform::wake_run_loop();
}
