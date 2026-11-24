//! JNI calls into the app's Java code.
//!
//! Modified for FastEmu: one Activity, so the calls that fanned out to extra
//! windows' Activities are gone, and errors are logged without Zed's `util`.
//!
//! GameActivity attaches the gpui thread to the JVM for the life of the app,
//! so what a JNI call leaves behind isn't cleaned up by a detach: local
//! references pile up in the thread's reference table, and a pending Java
//! exception makes every later JNI call on the thread fail (CheckJNI aborts
//! the process). Every call goes through [`with_env`] or [`with_activity`],
//! which free the call's local references and clear its exceptions.
//!
//! What takes more than one call lives in a small Kotlin method, which this
//! then calls: the Android APIs read as they do in Android's docs, and Kotlin's
//! compiler and lint check them.

use std::sync::OnceLock;

use android_activity::AndroidApp;
use anyhow::{Context as _, Result, anyhow};
use jni::{
    JNIEnv, JavaVM,
    objects::{JObject, JString, JValue},
};

static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

/// Keeps the `AndroidApp` for code that runs inside the gpui `App`, with no
/// access to the one `android_main` received.
pub(crate) fn register_android_app(android_app: AndroidApp) {
    ANDROID_APP.set(android_app).ok();
}

/// The `AndroidApp` that [`crate::application`] started with.
pub fn android_app() -> Result<&'static AndroidApp> {
    ANDROID_APP.get().context("no AndroidApp registered yet")
}

/// The JVM grows a local frame past this as needed; it only sizes the
/// initial allocation.
const LOCAL_FRAME_CAPACITY: i32 = 16;

/// Runs `f` with the current thread's JNI environment, attaching the thread
/// if it isn't yet. Local references made in `f` are freed when it returns,
/// and a Java exception it leaves pending is cleared and returned as the
/// error.
pub(crate) fn with_env<T>(
    android_app: &AndroidApp,
    f: impl FnOnce(&mut JNIEnv<'_>) -> Result<T>,
) -> Result<T> {
    // SAFETY: android-activity's pointer is the process's JavaVM, valid for
    // the life of the process.
    let vm =
        unsafe { JavaVM::from_raw(android_app.vm_as_ptr().cast()) }.context("get the JavaVM")?;
    let mut env = vm
        .attach_current_thread()
        .context("attach the current thread to the JVM")?;
    if let Some(exception) = take_pending_exception(&mut env) {
        log::warn!("jni_env: cleared a Java exception left by an earlier call: {exception}");
    }
    let result = env.with_local_frame(LOCAL_FRAME_CAPACITY, |env| f(env));
    match take_pending_exception(&mut env) {
        Some(exception) => Err(match result {
            Err(error) => error.context(exception),
            Ok(_) => anyhow!(exception),
        }),
        None => result,
    }
}

/// [`with_env`] with the main Activity, for the app's own JNI calls.
pub fn with_activity<T>(
    android_app: &AndroidApp,
    f: impl FnOnce(&mut JNIEnv<'_>, &JObject<'_>) -> Result<T>,
) -> Result<T> {
    with_env(android_app, |env| {
        // SAFETY: android-activity holds a global reference to the Activity
        // for as long as the `AndroidApp` exists.
        let activity = unsafe { JObject::from_raw(android_app.activity_as_ptr().cast()) };
        f(env, &activity)
    })
}

/// Calls a `void` method on the main Activity, logging a failure.
pub(crate) fn call_activity_void(
    android_app: &AndroidApp,
    method: &str,
    signature: &str,
    args: &[JValue<'_, '_>],
) {
    let result = with_activity(android_app, |env, activity| {
        env.call_method(activity, method, signature, args)
            .with_context(|| format!("call MainActivity.{method}"))?;
        Ok(())
    });
    if let Err(error) = result {
        log::warn!("{error:#}");
    }
}

/// Clears a pending Java exception and returns its description.
fn take_pending_exception(env: &mut JNIEnv<'_>) -> Option<String> {
    if !env.exception_check().unwrap_or(false) {
        return None;
    }
    let exception = env
        .exception_occurred()
        .ok()
        .map(|exception| env.auto_local(exception));
    log_jni_error(env.exception_describe());
    log_jni_error(env.exception_clear());
    let description = exception
        .and_then(|exception| {
            env.call_method(&exception, "toString", "()Ljava/lang/String;", &[])
                .ok()
        })
        .and_then(|description| description.l().ok())
        .and_then(|description| {
            let description = env.auto_local(JString::from(description));
            env.get_string(&description).ok().map(String::from)
        });
    // `toString` itself can throw.
    log_jni_error(env.exception_clear());
    Some(description.unwrap_or_else(|| "Java exception during a JNI call".to_string()))
}

fn log_jni_error(result: jni::errors::Result<()>) {
    if let Err(error) = result {
        log::error!("jni_env: {error}");
    }
}
