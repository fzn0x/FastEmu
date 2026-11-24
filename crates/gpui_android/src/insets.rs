//! The insets the main Activity reports through
//! `NativeBridge.nativeOnWindowInsetsChanged`: the system bars and display
//! cutout, which gpui hands the app as the window's safe area.
//!
//! Modified for FastEmu: one window, no on-screen keyboard, and the JNI
//! entry point belongs to `io.github.fzn0x.fastemu.NativeBridge`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use jni::objects::JObject;

/// The window's insets in physical pixels, as its Activity reported them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PhysicalInsets {
    /// `WindowInsets` types `systemBars() | displayCutout()`, zero where
    /// the system bars are hidden.
    pub(crate) safe_area_left: i32,
    pub(crate) safe_area_top: i32,
    pub(crate) safe_area_right: i32,
    pub(crate) safe_area_bottom: i32,
}

/// The latest insets rather than a queue: the Activity reports before Rust
/// has opened its window, and only the latest matters.
static REPORTED: Mutex<PhysicalInsets> = Mutex::new(PhysicalInsets {
    safe_area_left: 0,
    safe_area_top: 0,
    safe_area_right: 0,
    safe_area_bottom: 0,
});

/// Set by each report and cleared by [`take_changed`].
static CHANGED: AtomicBool = AtomicBool::new(false);

/// The insets last reported: zero until the Activity reports.
pub(crate) fn reported() -> PhysicalInsets {
    *REPORTED.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether the Activity reported insets since the last call.
pub(crate) fn take_changed() -> bool {
    CHANGED.swap(false, Ordering::AcqRel)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_io_github_fzn0x_fastemu_NativeBridge_nativeOnWindowInsetsChanged<
    'local,
>(
    _env: jni::JNIEnv<'local>,
    _bridge: JObject<'local>,
    safe_area_left: i32,
    safe_area_top: i32,
    safe_area_right: i32,
    safe_area_bottom: i32,
) {
    let insets = PhysicalInsets {
        safe_area_left,
        safe_area_top,
        safe_area_right,
        safe_area_bottom,
    };
    *REPORTED.lock().unwrap_or_else(PoisonError::into_inner) = insets;
    CHANGED.store(true, Ordering::Release);
    crate::platform::wake_run_loop();
}
