//! Answers GPUI's path prompts with Android's document picker.
//!
//! `cx.prompt_for_paths` reaches the backend, which hands the prompt to the
//! app ([`gpui_android::path_prompts`]). The Activity shows the system
//! picker for `.jar` and `.jad` files, copies what the user picks into the
//! app's storage (the picker gives content URIs, not paths), and reports the
//! copy's path through `NativeBridge.nativeSuitePicked`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, PoisonError};

use futures::StreamExt as _;
use futures::channel::oneshot;
use gpui::App;
use gpui_android::PathPrompt;
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jlong;

type Answer = oneshot::Sender<Option<Vec<PathBuf>>>;

/// Prompts the Activity is showing, by request number.
static PENDING: Mutex<BTreeMap<i64, Answer>> = Mutex::new(BTreeMap::new());
static NEXT_REQUEST: AtomicI64 = AtomicI64::new(1);

fn pending() -> std::sync::MutexGuard<'static, BTreeMap<i64, Answer>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Shows the picker for every path prompt from now on.
pub(super) fn serve(cx: &mut App) {
    let mut prompts = gpui_android::path_prompts();
    cx.spawn(async move |_| {
        while let Some(prompt) = prompts.next().await {
            show(prompt);
        }
    })
    .detach();
}

fn show(prompt: PathPrompt) {
    if prompt.creating_path {
        // Dropping the answer cancels the prompt.
        log::warn!("picker: saving files is not supported");
        return;
    }
    let request = NEXT_REQUEST.fetch_add(1, Ordering::Relaxed);
    pending().insert(request, prompt.answer);
    let shown = gpui_android::android_app().and_then(|android_app| {
        gpui_android::with_activity(android_app, |env, activity| {
            env.call_method(activity, "pickSuite", "(J)V", &[request.into()])?;
            Ok(())
        })
    });
    if let Err(error) = shown {
        log::error!("picker: could not show the picker: {error:#}");
        pending().remove(&request);
    }
}

/// `NativeBridge.nativeSuitePicked(request, path)`: the copy of the picked
/// game, or `null` when the user cancelled or the copy failed. Called on the
/// Activity's copying thread.
// JNI finds a native method by its unmangled name.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_fzn0x_fastemu_NativeBridge_nativeSuitePicked<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    request: jlong,
    path: JString<'local>,
) {
    let path = if path.is_null() {
        None
    } else {
        match env.get_string(&path) {
            Ok(path) => Some(PathBuf::from(String::from(path))),
            Err(error) => {
                log::error!("picker: could not read the picked path: {error}");
                None
            }
        }
    };
    let answer = pending().remove(&request);
    match answer {
        Some(answer) => {
            log::info!("picker: request {request} picked {path:?}");
            // The prompt's caller may have stopped waiting.
            answer.send(path.map(|path| vec![path])).ok();
        }
        None => log::warn!("picker: an answer to unknown request {request}"),
    }
}
