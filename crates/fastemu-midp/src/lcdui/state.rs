//! The per-instance state of the display: which `Displayable` is shown,
//! the queues the event thread drains, the pending repaint, the keys, the
//! soft key menu and the fonts. It lives in the virtual machine as an
//! extension, made on first use from the instance's environment.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fastemu_core::InputEvent;
use fastemu_gfx::{FontLibrary, Surface, TextRendering};
use fastemu_jvm::{Ctx, GlobalRef, JResult, ObjRef, ThreadId};

use super::keys::KeyState;
use super::softkeys::Menu;
use super::theme::Theme;
use crate::env::env;

/// The frame rate a game runs at when the settings give no bound, as
/// freej2me-plus does, so games without their own timing do not run at
/// host speed. `InstanceSettings::max_fps` of `Some(0)` removes the bound.
pub const DEFAULT_MAX_FPS: u32 = 60;

/// A rectangle in pixels; empty when either side is zero or less.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    /// A rectangle at `(x, y)` of the given size.
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether it covers no pixel.
    pub fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// The smallest rectangle covering both.
    #[must_use]
    pub fn union(self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = (i64::from(self.x) + i64::from(self.width))
            .max(i64::from(other.x) + i64::from(other.width));
        let bottom = (i64::from(self.y) + i64::from(self.height))
            .max(i64::from(other.y) + i64::from(other.height));
        Rect::new(
            x,
            y,
            (right - i64::from(x)) as i32,
            (bottom - i64::from(y)) as i32,
        )
    }

    /// The part inside `other`, possibly empty.
    #[must_use]
    pub fn intersect(self, other: Rect) -> Rect {
        let x = i64::from(self.x).max(i64::from(other.x));
        let y = i64::from(self.y).max(i64::from(other.y));
        let right = (i64::from(self.x) + i64::from(self.width))
            .min(i64::from(other.x) + i64::from(other.width));
        let bottom = (i64::from(self.y) + i64::from(self.height))
            .min(i64::from(other.y) + i64::from(other.height));
        Rect::new(
            x as i32,
            y as i32,
            (right - x).max(0) as i32,
            (bottom - y).max(0) as i32,
        )
    }

    /// Whether `(px, py)` lies inside.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && py >= self.y && px - self.x < self.width && py - self.y < self.height
    }
}

/// Who may run UI callbacks now. The event thread holds it around every
/// callback; `serviceRepaints` on another thread takes it to paint there,
/// so callbacks stay serialized even when they run on two threads.
#[derive(Debug, Default)]
pub(crate) struct CallbackLock {
    pub holder: Option<ThreadId>,
    pub depth: u32,
    pub waiters: u32,
}

/// Work the runtime itself queues for the event thread.
#[derive(Debug)]
pub(crate) enum Internal {
    /// A displayable's size or soft keys may have changed: commands,
    /// title, ticker or full-screen mode.
    Relayout(Arc<GlobalRef>),
}

/// The display of one instance.
pub(crate) struct LcduiState {
    pub fonts: Arc<FontLibrary>,
    /// `Font` objects by face, style and size, so `getFont` hands out one
    /// object per font as phones did.
    pub font_objects: HashMap<(i32, i32, i32), Arc<GlobalRef>>,
    pub theme: Theme,
    pub screen_width: i32,
    pub screen_height: i32,
    /// The `Display` object, made by the first `getDisplay`.
    pub display: Option<Arc<GlobalRef>>,
    /// The `Displayable` on the screen.
    pub current: Option<Arc<GlobalRef>>,
    /// The latest `setCurrent` the event thread has not applied yet.
    pub requested: Option<Arc<GlobalRef>>,
    /// Whether the app sent the instance to the background.
    pub background: bool,
    pub inputs: VecDeque<InputEvent>,
    pub internal: VecDeque<Internal>,
    /// `callSerially` runnables, in order.
    pub serial: VecDeque<Arc<GlobalRef>>,
    /// The region of the current displayable waiting to be painted.
    pub dirty: Option<Rect>,
    /// Whether a paint is under way, so `serviceRepaints` on another
    /// thread waits for it even when nothing is pending any more.
    pub painting: bool,
    pub event_thread: Option<ThreadId>,
    pub callback: CallbackLock,
    pub keys: KeyState,
    /// The key that repeats while held and when it repeats next.
    pub repeat: Option<(i32, Instant)>,
    /// Keys whose press the runtime took (soft keys that fired a command
    /// or opened the menu), so their release goes nowhere either.
    pub swallowed: Vec<i32>,
    pub menu: Option<Menu>,
    /// Whether the canvas got a pointer press it has not seen released.
    pub pointer_down: bool,
    pub last_pointer: (i32, i32),
    pub frame_serial: u64,
    pub last_present: Option<Instant>,
    /// When a game thread last passed the frame rate check.
    pub last_paced: Option<Instant>,
    /// The shortest time between two presented frames.
    pub frame_period: Option<Duration>,
    /// The screen the frame is composed on when it needs more than the
    /// displayable's own buffer (the soft key bar, a menu, a title).
    pub composed: Option<Surface>,
    /// Warnings already given, so a game that repeats a mistake every
    /// frame logs it once.
    pub warned: Vec<&'static str>,
    /// When the shown high-level screen wants its next timer tick.
    pub tick_at: Option<Instant>,
}

impl LcduiState {
    fn new(ctx: &Ctx<'_>) -> Self {
        let env = env(ctx);
        let fps = env.settings.max_fps.unwrap_or(DEFAULT_MAX_FPS);
        Self {
            fonts: Arc::new(FontLibrary::new(
                env.profile.font_sizes,
                TextRendering::Antialiased,
            )),
            font_objects: HashMap::new(),
            theme: Theme::default(),
            screen_width: env.profile.screen_width.max(1) as i32,
            screen_height: env.profile.screen_height.max(1) as i32,
            display: None,
            current: None,
            requested: None,
            background: false,
            inputs: VecDeque::new(),
            internal: VecDeque::new(),
            serial: VecDeque::new(),
            dirty: None,
            painting: false,
            event_thread: None,
            callback: CallbackLock::default(),
            keys: KeyState::default(),
            repeat: None,
            swallowed: Vec::new(),
            menu: None,
            pointer_down: false,
            last_pointer: (0, 0),
            frame_serial: 0,
            last_present: None,
            last_paced: None,
            frame_period: (fps > 0).then(|| Duration::from_secs(1) / fps),
            composed: None,
            warned: Vec::new(),
            tick_at: None,
        }
    }

    /// When the event thread may paint the next frame, if not now: a
    /// frame period after the last one, less a quarter period of slack, so
    /// a game that paces its own repaints at the frame rate is painted at
    /// once and not a whole slot late for arriving a moment early.
    pub fn next_frame_at(&self) -> Option<Instant> {
        let period = self.frame_period?;
        Some(self.last_present? + period * 3 / 4)
    }
}

/// The display state of the instance `ctx` runs in, made on first use.
pub(crate) fn state<'a>(ctx: &'a mut Ctx<'_>) -> &'a mut LcduiState {
    if ctx.extension::<LcduiState>().is_none() {
        let fresh = LcduiState::new(ctx);
        ctx.set_extension(fresh);
    }
    ctx.extension_mut::<LcduiState>()
        .expect("the display state was installed above")
}

/// The object behind a shared global reference, rooted in `ctx`.
pub(crate) fn get(ctx: &mut Ctx<'_>, global: Option<&Arc<GlobalRef>>) -> Option<ObjRef> {
    global.map(|g| g.get(ctx))
}

/// The `Displayable` on the screen.
pub(crate) fn current(ctx: &mut Ctx<'_>) -> Option<ObjRef> {
    let global = state(ctx).current.clone();
    get(ctx, global.as_ref())
}

/// Whether the current thread is the instance's event thread.
pub(crate) fn on_event_thread(ctx: &mut Ctx<'_>) -> bool {
    let me = ctx.thread_id();
    state(ctx).event_thread == Some(me)
}

/// Logs `text` as a warning the first time `key` comes up.
pub(crate) fn warn_once(ctx: &mut Ctx<'_>, key: &'static str, text: &str) {
    let st = state(ctx);
    if st.warned.contains(&key) {
        return;
    }
    st.warned.push(key);
    ctx.log(fastemu_core::LogLevel::Warn, text);
}

/// Logs `exception` with its stack trace as `Exception in <what>: ...` at
/// `LogLevel::Err`, and counts it as one of the instance's errors.
pub(crate) fn log_exception(ctx: &mut Ctx<'_>, what: &str, exception: ObjRef) {
    let description = ctx.describe_exception(exception);
    ctx.log(
        fastemu_core::LogLevel::Err,
        format!("Exception in {what}: {description}"),
    );
    for line in ctx.stack_trace(exception) {
        ctx.log(fastemu_core::LogLevel::Err, format!("\tat {line}"));
    }
    env(ctx).host.count_error();
}

/// Turns an exception into a log entry; other results pass through.
pub(crate) fn report(ctx: &mut Ctx<'_>, what: &str, result: JResult<()>) -> JResult<()> {
    match result {
        Err(fastemu_jvm::Throw::Exception(e)) => {
            log_exception(ctx, what, e);
            Ok(())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_union_and_intersect() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.union(b), Rect::new(0, 0, 15, 15));
        assert_eq!(a.intersect(b), Rect::new(5, 5, 5, 5));
        assert!(a.intersect(Rect::new(20, 20, 1, 1)).is_empty());
        assert_eq!(Rect::new(0, 0, 0, 0).union(b), b);
        assert!(a.contains(9, 9) && !a.contains(10, 0));
        let huge = Rect::new(i32::MAX - 1, 0, 10, 1);
        assert!(!huge.intersect(a).width.is_negative());
    }
}
