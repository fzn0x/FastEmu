//! The gpui `Platform` for one GameActivity.
//!
//! Modified for FastEmu: one window, so `open_window` refuses a second one
//! instead of starting an extra-window Activity. The IME bridge, clipboard,
//! credentials, cursor icons, thermal and Battery Saver reports, the splash
//! screen, the Ubuntu runtime's fonts and foreground service are gone; the
//! run loop, vsync, input, surface and lifecycle handling are upstream's.
//! The text system falls back to Android's Roboto instead of Zed's Lilex.

use std::{
    cell::{OnceCell, RefCell},
    ffi::{OsString, c_void},
    path::{Path, PathBuf},
    rc::Rc,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant},
};

use android_activity::{AndroidApp, AndroidAppWaker};
use anyhow::Result;
use futures::channel::{mpsc, oneshot};
use gpui::{
    Action, ActivityGuard, AnyWindowHandle, AppLifecyclePhase, BackgroundExecutor, ClipboardItem,
    CursorStyle, DummyKeyboardMapper, ForegroundExecutor, GestureTuning, Keymap, Menu, MenuItem,
    OperatingSystem, PathPromptOptions, Platform, PlatformDisplay, PlatformGestures, PlatformInfo,
    PlatformInput, PlatformKeyboardLayout, PlatformKeyboardMapper, PlatformTarget,
    PlatformTextSystem, PlatformWindow, PowerMode, PriorityQueueReceiver, RunnableVariant,
    ScrollPhysics, Task, ThermalState, TouchPhase, WindowAppearance, WindowParams, px,
};
use gpui_wgpu::GpuContext;
use ndk::configuration::UiModeNight;

use crate::dispatcher::AndroidDispatcher;
use crate::display::AndroidDisplay;
use crate::keyboard::AndroidKeyboardLayout;
use crate::window::{AndroidWindow, AndroidWindowStatePtr};

/// The family gpui falls back to for text whose font is missing: Android's
/// own UI font, which every device has under /system/fonts.
const FALLBACK_FONT_FAMILY: &str = "Roboto";

// AChoreographer, the NDK's vsync scheduler: Java's would need a JNI hop on
// every vsync. The `ndk` crate doesn't wrap it.
#[link(name = "android")]
unsafe extern "C" {
    fn AChoreographer_getInstance() -> *mut c_void;
    fn AChoreographer_postFrameCallback64(
        choreographer: *mut c_void,
        callback: ChoreographerFrameCallback,
        data: *mut c_void,
    );
}

type ChoreographerFrameCallback = unsafe extern "C" fn(frame_time_nanos: i64, data: *mut c_void);

/// Set by the Choreographer callback (called on the main thread's looper
/// at the vsync after a frame was requested, so no synchronization beyond
/// the atomic). Checked at the end of each event-loop tick to decide
/// whether to call window.refresh().
static FRAME_PENDING: AtomicBool = AtomicBool::new(false);

/// Whether a Choreographer callback is posted and hasn't fired yet.
static FRAME_CALLBACK_POSTED: AtomicBool = AtomicBool::new(false);

/// Live guards from `prevent_idle_sleep`.
static IDLE_SLEEP_GUARDS: AtomicUsize = AtomicUsize::new(0);

/// Wakes the run loop from Java threads. JNI callbacks queue input and
/// window events, or set flags, that the loop only reads on its next tick,
/// and an idle loop otherwise sleeps until something wakes it.
static RUN_LOOP_WAKER: Mutex<Option<AndroidAppWaker>> = Mutex::new(None);

static PATH_PROMPTS: Mutex<Option<mpsc::UnboundedSender<PathPrompt>>> = Mutex::new(None);

/// A path prompt gpui sent to the platform, for the app to show. Dropping
/// `answer` cancels it.
pub struct PathPrompt {
    /// Whether the answer is a path to write to rather than one to open.
    pub creating_path: bool,
    pub suggested_name: Option<String>,
    pub answer: oneshot::Sender<Option<Vec<PathBuf>>>,
}

/// The path prompts gpui sends to the platform. They come through a channel
/// because they arrive while the caller holds the `App`, which showing a
/// picker needs.
pub fn path_prompts() -> mpsc::UnboundedReceiver<PathPrompt> {
    let (sender, receiver) = mpsc::unbounded();
    *PATH_PROMPTS.lock().unwrap_or_else(PoisonError::into_inner) = Some(sender);
    receiver
}

fn request_path_prompt(
    creating_path: bool,
    suggested_name: Option<String>,
) -> oneshot::Receiver<Option<Vec<PathBuf>>> {
    let (answer, paths) = oneshot::channel();
    let prompt = PathPrompt {
        creating_path,
        suggested_name,
        answer,
    };
    let sent = PATH_PROMPTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .is_some_and(|prompts| prompts.unbounded_send(prompt).is_ok());
    if !sent {
        log::error!("no receiver for a path prompt, so it's cancelled");
    }
    paths
}

/// When the window last presented, if no frame has started since. Only the
/// main thread uses it; upstream's `thread_local!` became a mutex because
/// clippy 1.99 misreports its const initializer.
static LAST_PRESENT_AT: Mutex<Option<Instant>> = Mutex::new(None);

/// A frame that starts within this long of the last present continues a
/// run of frames; after a longer gap it follows an idle pause.
const FRAME_RUN_GAP: Duration = Duration::from_millis(50);

pub(crate) fn wake_run_loop() {
    if let Some(waker) = RUN_LOOP_WAKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        waker.wake();
    }
}

unsafe extern "C" fn frame_callback(_frame_time_nanos: i64, _data: *mut c_void) {
    FRAME_CALLBACK_POSTED.store(false, Ordering::Release);
    FRAME_PENDING.store(true, Ordering::Release);
}

/// Asks for a refresh on the next vsync. Called through the window's frame
/// waker, which gpui invokes while the window is dirty or still animating, so
/// an idle window stops waking at the panel's refresh rate. gpui's web and
/// Wayland backends drive their frame loops the same way.
pub(crate) fn request_frame_callback() {
    // A vsync already arrived and the run loop refreshes before it polls
    // again; gpui asks for another frame afterwards if it still has work.
    if FRAME_PENDING.load(Ordering::Acquire) {
        return;
    }
    if FRAME_CALLBACK_POSTED.swap(true, Ordering::AcqRel) {
        return;
    }
    // SAFETY: called on the main thread, whose looper android-activity
    // attached; the callback takes no data.
    unsafe {
        let choreographer = AChoreographer_getInstance();
        if choreographer.is_null() {
            // No vsync source: refresh on the run loop's next tick instead.
            FRAME_CALLBACK_POSTED.store(false, Ordering::Release);
            FRAME_PENDING.store(true, Ordering::Release);
            wake_run_loop();
            return;
        }
        AChoreographer_postFrameCallback64(choreographer, frame_callback, std::ptr::null_mut());
    }
}

/// Called whenever the window presents. While frames that present run back
/// to back, the run loop asks for the next vsync before it paints.
pub(crate) fn frame_presented() {
    *LAST_PRESENT_AT
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
}

/// Requests the first frame on the calling thread, which must have a Looper
/// attached (android-activity's main thread does).
fn install_choreographer_callback() {
    // SAFETY: a getter with no arguments, on the main thread.
    if unsafe { AChoreographer_getInstance() }.is_null() {
        log::warn!(
            "AChoreographer_getInstance returned null; vsync sync \
             disabled, refreshing on every run loop tick"
        );
    } else {
        log::info!("AndroidPlatform: Choreographer frame callbacks enabled");
    }
    request_frame_callback();
}

#[derive(Default)]
pub(crate) struct PlatformHandlers {
    pub(crate) open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    pub(crate) quit: Option<Box<dyn FnMut() -> bool>>,
    pub(crate) app_lifecycle: Option<Box<dyn FnMut(AppLifecyclePhase)>>,
    pub(crate) memory_warning: Option<Box<dyn FnMut()>>,
}

pub(crate) struct AndroidCommon {
    pub(crate) background_executor: BackgroundExecutor,
    pub(crate) foreground_executor: ForegroundExecutor,
    pub(crate) text_system: Arc<dyn PlatformTextSystem>,
    pub(crate) appearance: WindowAppearance,
    pub(crate) callbacks: PlatformHandlers,
    pub(crate) main_receiver: PriorityQueueReceiver<RunnableVariant>,
    pub(crate) active_window: Option<AnyWindowHandle>,
    pub(crate) gpu_context: GpuContext,
    /// The GameActivity-owned window, once gpui opened it.
    pub(crate) window: Option<AndroidWindowStatePtr>,
    /// Window events that arrived while the window was opening, for the run
    /// loop to apply: gpui can't take callbacks then.
    pub(crate) deferred_primary_resync: bool,
    pub(crate) deferred_primary_focus: Option<bool>,
    pub(crate) deferred_lifecycle: Vec<AppLifecyclePhase>,
    pub(crate) deferred_memory_warning: bool,
    /// Whether the Activity keeps the screen on; see `prevent_idle_sleep`.
    pub(crate) keep_screen_on: bool,
    pub(crate) running: bool,
    /// Set by `quit()` so `run()` exits the process once the event loop ends.
    pub(crate) exit_requested: bool,
}

impl AndroidCommon {
    pub fn new(android_app: &AndroidApp) -> Self {
        let (dispatcher, main_receiver) = AndroidDispatcher::new(android_app);
        let dispatcher = Arc::new(dispatcher);

        // fontdb's automatic discovery is skipped for /system/fonts alone,
        // a bounded set (about 70 files) that covers Latin, CJK, Cyrillic,
        // Arabic, symbols and emoji.
        let cosmic = gpui_wgpu::CosmicTextSystem::new_without_system_fonts(FALLBACK_FONT_FAMILY);
        cosmic.load_fonts_dir("/system/fonts");
        let text_system: Arc<dyn PlatformTextSystem> = Arc::new(cosmic);

        Self {
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            text_system,
            appearance: appearance_from_config(android_app),
            callbacks: PlatformHandlers::default(),
            main_receiver,
            active_window: None,
            gpu_context: Rc::new(RefCell::new(None)),
            window: None,
            deferred_primary_resync: false,
            deferred_primary_focus: None,
            deferred_lifecycle: Vec::new(),
            deferred_memory_warning: false,
            keep_screen_on: false,
            running: true,
            exit_requested: false,
        }
    }
}

fn appearance_from_config(android_app: &AndroidApp) -> WindowAppearance {
    match android_app.config().ui_mode_night() {
        UiModeNight::Yes => WindowAppearance::Dark,
        _ => WindowAppearance::Light,
    }
}

/// Dispatch `MainActivity.openUrl(url: String)` via JNI. Backs
/// `AndroidPlatform::open_url`. The Kotlin side wraps
/// `startActivity(Intent.ACTION_VIEW, Uri.parse(url))` and swallows
/// ActivityNotFoundException.
fn jni_open_url(android_app: &AndroidApp, url: &str) -> Result<()> {
    use anyhow::Context;

    crate::jni_env::with_activity(android_app, |env, activity| {
        let url_jstring = env
            .new_string(url)
            .context("alloc JString for open_url argument")?;
        env.call_method(
            activity,
            "openUrl",
            "(Ljava/lang/String;)V",
            &[(&url_jstring).into()],
        )
        .context("MainActivity.openUrl")?;
        Ok(())
    })
}

/// Gesture feel for gpui's touch recognizer, from Android's
/// `ViewConfiguration`.
struct AndroidGestures {
    tuning: GestureTuning,
}

impl PlatformGestures for AndroidGestures {
    fn tuning(&self) -> GestureTuning {
        self.tuning
    }
}

impl AndroidGestures {
    fn new(android_app: &AndroidApp) -> Self {
        // AOSP's `ViewConfiguration` defaults, for when JNI fails. The
        // double-tap slop stays gpui's.
        let mut tuning = GestureTuning {
            touch_slop: px(8.0),
            multi_tap_interval: Duration::from_millis(300),
            long_press_duration: Duration::from_millis(400),
            min_fling_velocity: 50.0,
            scroll_physics: ScrollPhysics::android(),
            ..GestureTuning::default()
        };
        match read_gesture_configuration(android_app) {
            Ok([touch_slop, min_fling_velocity, double_tap_ms, long_press_ms]) => {
                tuning.touch_slop = px(touch_slop);
                tuning.min_fling_velocity = min_fling_velocity;
                tuning.multi_tap_interval = Duration::from_millis(double_tap_ms as u64);
                tuning.long_press_duration = Duration::from_millis(long_press_ms as u64);
            }
            Err(error) => log::error!("gestures: reading ViewConfiguration failed: {error:#}"),
        }
        log::info!("gestures: {tuning:?}");
        Self { tuning }
    }
}

/// `MainActivity.gestureConfiguration`'s values.
fn read_gesture_configuration(android_app: &AndroidApp) -> Result<[f32; 4]> {
    crate::jni_env::with_activity(android_app, |env, activity| {
        let values = env
            .call_method(activity, "gestureConfiguration", "()[F", &[])?
            .l()?;
        let mut configuration = [0.0; 4];
        env.get_float_array_region(
            jni::objects::JFloatArray::from(values),
            0,
            &mut configuration,
        )?;
        Ok(configuration)
    })
}

/// Touch moves held back within one input drain, the latest for each
/// touch. The run loop drains input once per tick, so after a slow frame
/// several moves arrive at once, and gpui, which measures fling speed from
/// when touches arrive, would read them as a very fast finger.
#[derive(Default)]
struct PendingTouchMoves {
    moves: Vec<(AndroidWindowStatePtr, gpui::TouchEvent)>,
}

impl PendingTouchMoves {
    /// Dispatches `input` to `window`, but holds back a touch move until
    /// [`Self::flush`] or the next other input.
    fn dispatch(&mut self, window: &AndroidWindowStatePtr, input: PlatformInput) {
        match input {
            PlatformInput::Touch(touch) if touch.phase == TouchPhase::Moved => {
                let pending = self.moves.iter_mut().find(|(pending_window, pending)| {
                    Rc::ptr_eq(&pending_window.state, &window.state) && pending.id == touch.id
                });
                match pending {
                    Some((_, pending)) => *pending = touch,
                    None => self.moves.push((window.clone(), touch)),
                }
            }
            input => {
                self.flush();
                window.handle_input(input);
            }
        }
    }

    fn flush(&mut self) {
        for (window, touch) in self.moves.drain(..) {
            window.handle_input(PlatformInput::Touch(touch));
        }
    }
}

pub struct AndroidPlatform {
    pub(crate) common: RefCell<AndroidCommon>,
    pub(crate) android_app: AndroidApp,
    gestures: OnceCell<Rc<AndroidGestures>>,
}

impl AndroidPlatform {
    pub fn new(android_app: AndroidApp) -> Self {
        *RUN_LOOP_WAKER
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(android_app.create_waker());
        // GameActivity only keeps the axes an app asks for, and these are a mouse
        // wheel's.
        android_app.enable_motion_axis(android_activity::input::Axis::Vscroll);
        android_app.enable_motion_axis(android_activity::input::Axis::Hscroll);
        let common = AndroidCommon::new(&android_app);
        Self {
            common: RefCell::new(common),
            android_app,
            gestures: OnceCell::new(),
        }
    }

    fn android_gestures(&self) -> Rc<AndroidGestures> {
        self.gestures
            .get_or_init(|| Rc::new(AndroidGestures::new(&self.android_app)))
            .clone()
    }

    /// Read the device's display density and convert to a scale factor.
    /// Android reports density in dpi where 160 dpi = 1.0x, so 336 dpi
    /// becomes ~2.1x. Falls back to 1.0 if the density isn't yet
    /// reported (e.g. very early in startup before the first config arrives).
    fn compute_scale_factor(&self) -> f32 {
        self.config_scale_factor().unwrap_or(1.0)
    }

    fn config_scale_factor(&self) -> Option<f32> {
        match self.android_app.config().density() {
            Some(dpi) if dpi > 0 => Some((dpi as f32 / 160.0).max(1.0)),
            _ => None,
        }
    }

    /// Hands the window the insets its Activity last reported.
    fn apply_window_insets(&self) {
        if !crate::insets::take_changed() {
            return;
        }
        let window = self.common.borrow().window.clone();
        if let Some(window) = window {
            window.set_insets(crate::insets::reported());
        }
    }

    /// Mirrors the Activity's input focus onto its gpui window.
    fn window_focus_changed(&self, window_ptr: &AndroidWindowStatePtr, focused: bool) {
        if focused {
            let handle = window_ptr.state.borrow().handle;
            self.common.borrow_mut().active_window = Some(handle);
        }
        window_ptr.set_active(focused);
    }

    /// Pull every queued `InputEvent` off android-activity's iterator and
    /// route translatable ones into the gpui window. Returning
    /// `InputStatus::Handled` for our own events lets android-activity stop
    /// propagating them up the system input stack.
    fn drain_input_events(&self) {
        use android_activity::InputStatus;
        use android_activity::input::InputEvent;

        let Some(window_ptr) = self.common.borrow().window.clone() else {
            return;
        };
        let Ok(mut iter) = self.android_app.input_events_iter() else {
            return;
        };
        let scale_factor = self.compute_scale_factor();
        let mut touch_moves = PendingTouchMoves::default();
        loop {
            let read = iter.next(|event| match event {
                InputEvent::KeyEvent(key) => match crate::events::translate_key_event(key) {
                    Some(input) => {
                        touch_moves.dispatch(&window_ptr, input);
                        InputStatus::Handled
                    }
                    None => InputStatus::Unhandled,
                },
                InputEvent::MotionEvent(motion) => {
                    let translated = {
                        let mut state = window_ptr.state.borrow_mut();
                        crate::events::translate_motion(
                            crate::events::Motion::from_event(motion),
                            scale_factor,
                            &mut state,
                        )
                    };
                    for input in translated.inputs {
                        touch_moves.dispatch(&window_ptr, input);
                    }
                    if translated.handled {
                        InputStatus::Handled
                    } else {
                        InputStatus::Unhandled
                    }
                }
                _ => InputStatus::Unhandled,
            });
            if !read {
                break;
            }
        }
        touch_moves.flush();
    }

    fn handle_main_event(&self, event: android_activity::MainEvent<'_>) {
        use android_activity::MainEvent;
        match event {
            MainEvent::InitWindow { .. } => {
                let window_ptr = self.common.borrow().window.clone();
                let Some(window_ptr) = window_ptr else {
                    log::warn!("MainEvent::InitWindow received before any window registered");
                    return;
                };
                let Some(native_window) = self.android_app.native_window() else {
                    log::warn!("MainEvent::InitWindow but native_window() returned None");
                    return;
                };
                let scale_factor = self.compute_scale_factor();
                if let Err(e) = window_ptr.attach_surface(native_window, scale_factor) {
                    log::error!("attach_surface failed: {e:#}");
                }
            }
            MainEvent::TerminateWindow { .. } => {
                if let Some(window_ptr) = self.common.borrow().window.clone() {
                    window_ptr.detach_surface();
                }
            }
            MainEvent::WindowResized { .. } => {
                let window_ptr = self.common.borrow().window.clone();
                let Some(window_ptr) = window_ptr else { return };
                let Some(native_window) = self.android_app.native_window() else {
                    return;
                };
                window_ptr.resize_surface(
                    native_window.width() as u32,
                    native_window.height() as u32,
                    self.compute_scale_factor(),
                );
            }
            MainEvent::ConfigChanged { .. } => self.resync_primary_window(),
            MainEvent::RedrawNeeded { .. } => {
                if let Some(window_ptr) = self.common.borrow().window.clone() {
                    window_ptr.refresh();
                }
            }
            MainEvent::GainedFocus | MainEvent::LostFocus => {
                let focused = matches!(event, MainEvent::GainedFocus);
                let primary = self.common.borrow().window.clone();
                if let Some(window_ptr) = primary {
                    self.window_focus_changed(&window_ptr, focused);
                }
            }
            MainEvent::Start => self.notify_app_lifecycle(AppLifecyclePhase::Foreground),
            MainEvent::Resume { .. } => self.notify_app_lifecycle(AppLifecyclePhase::Active),
            MainEvent::Pause => self.notify_app_lifecycle(AppLifecyclePhase::Inactive),
            MainEvent::Stop => self.notify_app_lifecycle(AppLifecyclePhase::Background),
            MainEvent::LowMemory => self.notify_memory_warning(),
            MainEvent::Destroy => {
                self.notify_app_lifecycle(AppLifecyclePhase::Disconnected);
                self.common.borrow_mut().running = false;
            }
            _ => {}
        }
    }

    fn notify_app_lifecycle(&self, phase: AppLifecyclePhase) {
        let callback = self.common.borrow_mut().callbacks.app_lifecycle.take();
        if let Some(mut callback) = callback {
            callback(phase);
            self.common.borrow_mut().callbacks.app_lifecycle = Some(callback);
        }
    }

    /// Files other apps opened with FastEmu (`open_files`). They stay queued
    /// until the app has registered `on_open_urls`.
    fn deliver_open_urls(&self) {
        if self.common.borrow().callbacks.open_urls.is_none() {
            return;
        }
        let Some(urls) = crate::open_files::take_pending_urls() else {
            return;
        };
        let callback = self.common.borrow_mut().callbacks.open_urls.take();
        if let Some(mut callback) = callback {
            callback(urls);
            self.common.borrow_mut().callbacks.open_urls = Some(callback);
        }
    }

    fn notify_memory_warning(&self) {
        let callback = self.common.borrow_mut().callbacks.memory_warning.take();
        if let Some(mut callback) = callback {
            callback();
            self.common.borrow_mut().callbacks.memory_warning = Some(callback);
        }
    }

    /// Density may have changed (rotation, dock/scaling). Refresh
    /// scale_factor and re-emit a resize so layout picks it up.
    fn resync_primary_window(&self) {
        let window_ptr = self.common.borrow().window.clone();
        let Some(window_ptr) = window_ptr else { return };
        let new_appearance = appearance_from_config(&self.android_app);
        self.common.borrow_mut().appearance = new_appearance;
        window_ptr.set_appearance(new_appearance);
        let Some(native_window) = self.android_app.native_window() else {
            return;
        };
        window_ptr.resize_surface(
            native_window.width() as u32,
            native_window.height() as u32,
            self.compute_scale_factor(),
        );
    }

    /// `handle_main_event` while `open_window` waits for the surface. gpui
    /// is in the middle of an update then, so events that call back into
    /// gpui wait for the run loop. Surface changes can't wait: Android
    /// releases the old surface as soon as this returns.
    fn handle_main_event_while_opening_window(&self, event: android_activity::MainEvent<'_>) {
        use android_activity::MainEvent;
        match event {
            MainEvent::InitWindow { .. } | MainEvent::TerminateWindow { .. } => {
                self.handle_main_event(event)
            }
            MainEvent::Destroy => {
                self.defer_app_lifecycle(AppLifecyclePhase::Disconnected);
                self.common.borrow_mut().running = false;
            }
            MainEvent::WindowResized { .. }
            | MainEvent::ConfigChanged { .. }
            | MainEvent::RedrawNeeded { .. } => {
                self.common.borrow_mut().deferred_primary_resync = true;
            }
            MainEvent::GainedFocus | MainEvent::LostFocus => {
                self.common.borrow_mut().deferred_primary_focus =
                    Some(matches!(event, MainEvent::GainedFocus));
            }
            MainEvent::Start => self.defer_app_lifecycle(AppLifecyclePhase::Foreground),
            MainEvent::Resume { .. } => self.defer_app_lifecycle(AppLifecyclePhase::Active),
            MainEvent::Pause => self.defer_app_lifecycle(AppLifecyclePhase::Inactive),
            MainEvent::Stop => self.defer_app_lifecycle(AppLifecyclePhase::Background),
            MainEvent::LowMemory => self.common.borrow_mut().deferred_memory_warning = true,
            _ => {}
        }
    }

    fn defer_app_lifecycle(&self, phase: AppLifecyclePhase) {
        self.common.borrow_mut().deferred_lifecycle.push(phase);
    }

    /// Applies the `prevent_idle_sleep` guards. Here rather than in the guards
    /// themselves, which can be created or dropped on any thread, including
    /// inside android-activity's event callbacks.
    fn apply_keep_screen_on(&self) {
        let wanted = IDLE_SLEEP_GUARDS.load(Ordering::Acquire) > 0;
        if self.common.borrow().keep_screen_on == wanted {
            return;
        }
        self.common.borrow_mut().keep_screen_on = wanted;
        crate::jni_env::call_activity_void(
            &self.android_app,
            "setKeepScreenOn",
            "(Z)V",
            &[wanted.into()],
        );
    }

    fn sync_window_visibility(&self) {
        let window = self.common.borrow().window.clone();
        if let Some(window_ptr) = window {
            window_ptr.sync_visibility();
        }
    }

    fn apply_deferred_main_events(&self) {
        let (resync, focus, lifecycle, memory_warning) = {
            let mut common = self.common.borrow_mut();
            (
                std::mem::take(&mut common.deferred_primary_resync),
                common.deferred_primary_focus.take(),
                std::mem::take(&mut common.deferred_lifecycle),
                std::mem::take(&mut common.deferred_memory_warning),
            )
        };
        for phase in lifecycle {
            self.notify_app_lifecycle(phase);
        }
        if memory_warning {
            self.notify_memory_warning();
        }
        if resync {
            self.resync_primary_window();
        }
        if let Some(focused) = focus {
            let primary = self.common.borrow().window.clone();
            if let Some(window_ptr) = primary {
                self.window_focus_changed(&window_ptr, focused);
            }
        }
    }
}

impl Platform for AndroidPlatform {
    fn platform_info(&self) -> PlatformInfo {
        PlatformInfo {
            os: OperatingSystem::Android,
            target: PlatformTarget::Native,
        }
    }

    fn background_executor(&self) -> BackgroundExecutor {
        self.common.borrow().background_executor.clone()
    }

    fn foreground_executor(&self) -> ForegroundExecutor {
        self.common.borrow().foreground_executor.clone()
    }

    fn gestures(&self) -> Option<Rc<dyn PlatformGestures>> {
        Some(self.android_gestures())
    }

    fn text_system(&self) -> Arc<dyn PlatformTextSystem> {
        self.common.borrow().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        // gpui reads the gesture tuning as the window opens.
        self.android_gestures();
        log::info!("AndroidPlatform::run: invoking on_finish_launching");
        on_finish_launching();
        log::info!("AndroidPlatform::run: entering event loop");

        // Hook AChoreographer for vsync-aligned rendering. The callback
        // is delivered on this thread's looper as part of the same
        // event stream android-activity polls, so vsync arrivals
        // unblock `poll_events` naturally.
        install_choreographer_callback();

        while self.common.borrow().running {
            // Before sleeping, so events deferred while the window opened
            // during the last iteration aren't left waiting.
            self.apply_deferred_main_events();
            // Sleeps until there is work: an input event, a vsync
            // (`frame_callback` runs as a looper task and sets
            // FRAME_PENDING), a runnable from another thread (the dispatcher
            // wakes the looper), a main-thread event from android-activity,
            // or a JNI event (`wake_run_loop`).
            self.android_app.poll_events(None, |event| match event {
                android_activity::PollEvent::Wake => {}
                android_activity::PollEvent::Timeout => {}
                android_activity::PollEvent::Main(main_event) => {
                    log::trace!("MainEvent: {main_event:?}");
                    self.handle_main_event(main_event);
                }
                _ => {}
            });

            // Drain main-thread runnables enqueued from background threads.
            // Only the runnables already queued run in this pass: one that
            // re-enqueues itself from inside `run()` would otherwise keep
            // the iterator going and starve the input and paint steps below.
            let receiver = self.common.borrow().main_receiver.clone();
            let batch: Vec<_> = receiver.try_iter().collect();
            for runnable in batch.into_iter().flatten() {
                runnable.run();
            }

            // Drain input events into the window every tick:
            // android-activity gates input delivery on this iterator being
            // polled, so missing a tick can stall touch.
            self.drain_input_events();

            self.deliver_open_urls();
            self.apply_window_insets();
            self.sync_window_visibility();
            self.apply_keep_screen_on();

            // Refresh on vsync (FRAME_PENDING set by the Choreographer
            // callback). gpui's window.refresh() short-circuits when nothing
            // is dirty.
            if FRAME_PENDING.swap(false, Ordering::AcqRel) {
                let primary = self.common.borrow().window.clone();
                // While frames that present run back to back (scrolling,
                // animations, a running game), ask for the next vsync before
                // painting, as Android's own animations do: a paint that
                // overruns the frame budget then starts the next frame right
                // away instead of waiting for a further vsync.
                if LAST_PRESENT_AT
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take()
                    .is_some_and(|at| at.elapsed() < FRAME_RUN_GAP)
                {
                    request_frame_callback();
                }
                // A hidden window has no surface to present to; gpui's
                // contract is that the platform doesn't request its frames.
                if let Some(window_ptr) = primary
                    && window_ptr.is_visible()
                {
                    window_ptr.refresh();
                }
            }
        }

        log::info!("AndroidPlatform::run: exiting event loop");
        // A Destroy that arrived while the window was opening.
        self.apply_deferred_main_events();
        let quit = self.common.borrow_mut().callbacks.quit.take();
        if let Some(mut fun) = quit
            && !fun()
        {
            log::warn!("AndroidPlatform::run: app was busy, shutdown skipped");
        }
        if self.common.borrow().exit_requested {
            // gpui's quit hook only stops the run loop. On Android the
            // hosting GameActivity stays alive, so the app would stop
            // responding without closing. Exit the process so the OS
            // returns the user to the launcher.
            std::process::exit(0);
        }
    }

    fn quit(&self) {
        // App::quit runs inside an app update, where the quit callback can't
        // borrow the app and skips App::shutdown. Stop the loop instead;
        // run() invokes the callback once the update has unwound, as the
        // Linux and Windows backends do.
        let mut common = self.common.borrow_mut();
        common.running = false;
        common.exit_requested = true;
    }

    fn restart(&self, _binary_path: Option<PathBuf>, _arguments: Vec<OsString>) {}
    fn activate(&self, _ignoring_other_apps: bool) {}
    fn hide(&self) {}
    fn hide_other_apps(&self) {}
    fn unhide_other_apps(&self) {}

    fn on_displays_changed(&self, _callback: Box<dyn FnMut()>) {}

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        vec![Rc::new(AndroidDisplay::new())]
    }

    fn primary_display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(Rc::new(AndroidDisplay::new()))
    }

    fn active_window(&self) -> Option<AnyWindowHandle> {
        self.common.borrow().active_window
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowParams,
    ) -> Result<Box<dyn PlatformWindow>> {
        if self.common.borrow().window.is_some() {
            anyhow::bail!("FastEmu shows one window on Android, and it is open");
        }

        // gpui's `Window::new` calls `platform_window.sprite_atlas()` immediately,
        // so the renderer (and therefore atlas) must already be live by the time
        // we return. On Android we have no surface until `MainEvent::InitWindow`
        // fires, so block here pumping the Android event loop until a native
        // window is available, then attach inline.
        while self.android_app.native_window().is_none() {
            if !self.common.borrow().running {
                return Err(anyhow::anyhow!(
                    "open_window: app destroyed before surface attached"
                ));
            }
            self.android_app
                .poll_events(Some(std::time::Duration::from_millis(100)), |event| {
                    if let android_activity::PollEvent::Main(main_event) = event {
                        log::trace!("MainEvent during open_window block: {main_event:?}");
                        self.handle_main_event_while_opening_window(main_event);
                    }
                });

            // Do not drain main_receiver here: open_window runs inside
            // gpui's `cx.update` borrow guard, and a runnable that calls
            // `cx.update(...)` would re-enter the borrow and panic. The
            // outer event loop drains the queue on the next tick.
        }

        let appearance = self.common.borrow().appearance;
        let gpu_context = self.common.borrow().gpu_context.clone();
        let window = AndroidWindow::new(handle, options, gpu_context, appearance);

        let native_window = self.android_app.native_window().ok_or_else(|| {
            anyhow::anyhow!("open_window: native_window vanished between poll and attach")
        })?;
        let scale_factor = self.compute_scale_factor();
        window.ptr().attach_surface(native_window, scale_factor)?;
        // The Activity reports its insets long before gpui opens its window.
        window.ptr().set_insets(crate::insets::reported());

        self.common.borrow_mut().window = Some(window.ptr());
        self.common.borrow_mut().active_window = Some(handle);
        Ok(Box::new(window))
    }

    fn window_appearance(&self) -> WindowAppearance {
        self.common.borrow().appearance
    }

    fn open_url(&self, url: &str) {
        if let Err(err) = jni_open_url(&self.android_app, url) {
            log::warn!("AndroidPlatform::open_url({url:?}) failed: {err:#}");
        }
    }
    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.common.borrow_mut().callbacks.open_urls = Some(callback);
    }
    fn register_url_scheme(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    /// Android's picker hands back content URIs, not paths, so the app
    /// answers the prompt: see [`path_prompts`]. `options` doesn't apply.
    fn prompt_for_paths(
        &self,
        _options: PathPromptOptions,
    ) -> oneshot::Receiver<Result<Option<Vec<PathBuf>>>> {
        let paths = request_path_prompt(false, None);
        let (result, receiver) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                let paths = paths.await.ok().flatten();
                result.send(Ok(paths)).ok();
            })
            .detach();
        receiver
    }

    /// As [`Self::prompt_for_paths`]; `directory` doesn't apply.
    fn prompt_for_new_path(
        &self,
        _directory: &Path,
        suggested_name: Option<&str>,
    ) -> oneshot::Receiver<Result<Option<PathBuf>>> {
        let paths = request_path_prompt(true, suggested_name.map(str::to_owned));
        let (result, receiver) = oneshot::channel();
        self.foreground_executor()
            .spawn(async move {
                let path = paths
                    .await
                    .ok()
                    .flatten()
                    .and_then(|paths| paths.into_iter().next());
                result.send(Ok(path)).ok();
            })
            .detach();
        receiver
    }

    fn can_select_mixed_files_and_dirs(&self) -> bool {
        false
    }
    fn reveal_path(&self, path: &Path) {
        log::info!("reveal_path({path:?}): no file manager to reveal it in");
    }
    fn open_with_system(&self, path: &Path) {
        log::info!("open_with_system({path:?}): not supported on Android");
    }

    fn on_quit(&self, callback: Box<dyn FnMut() -> bool>) {
        self.common.borrow_mut().callbacks.quit = Some(callback);
    }
    fn on_app_lifecycle(&self, callback: Box<dyn FnMut(AppLifecyclePhase)>) {
        self.common.borrow_mut().callbacks.app_lifecycle = Some(callback);
    }
    fn on_memory_warning(&self, callback: Box<dyn FnMut()>) {
        self.common.borrow_mut().callbacks.memory_warning = Some(callback);
    }
    fn on_reopen(&self, _callback: Box<dyn FnMut()>) {}
    /// Android exposes no system sleep or wake signal to apps; the app
    /// lifecycle (`onPause` and the like) flows through `on_app_lifecycle`.
    fn on_system_sleep(&self, _callback: Box<dyn FnMut()>) {}
    fn on_system_wake(&self, _callback: Box<dyn FnMut()>) {}
    fn set_menus(&self, _menus: Vec<Menu>, _keymap: &Keymap) {}
    fn set_dock_menu(&self, _menu: Vec<MenuItem>, _keymap: &Keymap) {}
    fn on_app_menu_action(&self, _callback: Box<dyn FnMut(&dyn Action)>) {}
    fn on_will_open_app_menu(&self, _callback: Box<dyn FnMut()>) {}
    fn on_validate_app_menu_command(&self, _callback: Box<dyn FnMut(&dyn Action) -> bool>) {}

    fn thermal_state(&self) -> ThermalState {
        ThermalState::Nominal
    }
    fn on_thermal_state_change(&self, _callback: Box<dyn FnMut()>) {}
    fn power_mode(&self) -> PowerMode {
        PowerMode::Unknown
    }
    fn on_power_mode_change(&self, _callback: Box<dyn FnMut()>) {}
    /// Keeps the screen on while the app is in front. Counted, so
    /// overlapping requests don't release it early; the run loop applies
    /// the result.
    fn prevent_idle_sleep(&self, reason: &str) -> Task<Result<ActivityGuard>> {
        log::debug!("keeping the screen on: {reason}");
        IDLE_SLEEP_GUARDS.fetch_add(1, Ordering::AcqRel);
        wake_run_loop();
        Task::ready(Ok(ActivityGuard::new(|| {
            IDLE_SLEEP_GUARDS.fetch_sub(1, Ordering::AcqRel);
            wake_run_loop();
        })))
    }

    fn compositor_name(&self) -> &'static str {
        "Android"
    }
    fn app_path(&self) -> Result<PathBuf> {
        Err(anyhow::anyhow!("app_path is not available on Android"))
    }
    fn path_for_auxiliary_executable(&self, _name: &str) -> Result<PathBuf> {
        Err(anyhow::anyhow!(
            "auxiliary executables are not available on Android"
        ))
    }

    /// A pointer's icon stays Android's arrow.
    fn set_cursor_style(&self, _style: CursorStyle) {}
    fn hide_cursor_until_mouse_moves(&self) {}
    fn is_cursor_visible(&self) -> bool {
        true
    }
    fn should_auto_hide_scrollbars(&self) -> bool {
        true
    }

    /// FastEmu doesn't use the clipboard yet.
    fn read_from_clipboard(&self) -> Option<ClipboardItem> {
        None
    }
    fn write_to_clipboard(&self, _item: ClipboardItem) {}

    fn write_credentials(&self, _url: &str, _username: &str, _password: &[u8]) -> Task<Result<()>> {
        Task::ready(Err(anyhow::anyhow!(
            "FastEmu keeps no credentials on Android"
        )))
    }
    fn read_credentials(&self, _url: &str) -> Task<Result<Option<(String, Vec<u8>)>>> {
        Task::ready(Ok(None))
    }
    fn delete_credentials(&self, _url: &str) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn keyboard_layout(&self) -> Box<dyn PlatformKeyboardLayout> {
        Box::new(AndroidKeyboardLayout)
    }
    fn keyboard_mapper(&self) -> Rc<dyn PlatformKeyboardMapper> {
        Rc::new(DummyKeyboardMapper)
    }
    fn on_keyboard_layout_change(&self, _callback: Box<dyn FnMut()>) {}
}
