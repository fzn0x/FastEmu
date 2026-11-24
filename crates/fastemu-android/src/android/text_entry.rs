//! Typing into a game's text box with the system keyboard.
//!
//! gpui_android has no input method bridge, so the views' [`TextEntry`]
//! asks the Activity to show a dialog with a text field
//! (`MainActivity.enterText`). The typed text, or nothing when the user
//! cancels, comes back through `NativeBridge.nativeTextEntered`.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use fastemu_ui::TextEntry;
use futures::channel::oneshot;
use jni::JNIEnv;
use jni::objects::{JClass, JString, JValue};
use jni::sys::jlong;

type Answer = oneshot::Sender<Option<String>>;

/// Dialogs the Activity is showing, by request number.
static PENDING: Mutex<BTreeMap<i64, Answer>> = Mutex::new(BTreeMap::new());
static NEXT_REQUEST: AtomicI64 = AtomicI64::new(1);

fn pending() -> std::sync::MutexGuard<'static, BTreeMap<i64, Answer>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The text entry the views call when a game takes text.
pub(super) fn entry() -> TextEntry {
    Arc::new(|title: &str| {
        let (answer, receiver) = oneshot::channel();
        let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
        pending().insert(request, answer);
        let shown = gpui_android::android_app().and_then(|android_app| {
            gpui_android::with_activity(android_app, |env, activity| {
                let title = env.new_string(title)?;
                env.call_method(
                    activity,
                    "enterText",
                    "(JLjava/lang/String;)V",
                    &[request.into(), JValue::Object(&title)],
                )?;
                Ok(())
            })
        });
        if let Err(error) = shown {
            log::error!("text entry: could not show the dialog: {error:#}");
            // Dropping the answer tells the view nothing was typed.
            pending().remove(&request);
        }
        receiver
    })
}

/// `NativeBridge.nativeTextEntered(request, text)`: what the user typed, or
/// `null` when the dialog was cancelled. Called on the UI thread.
// JNI finds a native method by its unmangled name.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_fzn0x_fastemu_NativeBridge_nativeTextEntered<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    request: jlong,
    text: JString<'local>,
) {
    let text = if text.is_null() {
        None
    } else {
        match env.get_string(&text) {
            Ok(text) => Some(String::from(text)),
            Err(error) => {
                log::error!("text entry: could not read the text: {error}");
                None
            }
        }
    };
    match pending().remove(&request) {
        // The view may have stopped waiting.
        Some(answer) => drop(answer.send(text)),
        None => log::warn!("text entry: an answer to unknown request {request}"),
    }
}
