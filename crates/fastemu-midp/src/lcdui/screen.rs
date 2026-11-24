//! The extension point for the high-level screens (`Form`, `List`,
//! `TextBox`, `Alert`), which are implemented in Rust, and the abstract
//! `Screen` class they extend.
//!
//! A screen class gives its objects a [`DisplayableData`] made with
//! [`DisplayableData::for_screen`], naming a [`ScreenHandler`] (a
//! `static`, so it holds no per-object state) and the object's own
//! [`ScreenState`]. The runtime then:
//!
//! - lays the screen out below its title and ticker and above the soft key
//!   bar, and calls [`ScreenHandler::paint`] with a surface of the content
//!   area's size whenever the screen is repainted;
//! - sends it the keys, pointer events and text the soft keys and the
//!   options menu do not take;
//! - fires its commands through [`ScreenHandler::command`] first, so a
//!   screen can act on its own commands (an `Alert`'s dismiss command);
//! - calls [`ScreenHandler::tick`] while the screen is shown, for timers
//!   (an `Alert`'s timeout, a ticker, an animated gauge).
//!
//! Every hook runs on the event thread, serialized with the other UI
//! callbacks, and gets the screen's Java object. A hook reaches its state
//! with `displayable::data(ctx, screen)?.state::<MyState>()` and asks for
//! a repaint with [`crate::lcdui::display::repaint`].

use std::any::Any;
use std::time::Instant;

use fastemu_core::PointerAction;
use fastemu_gfx::{FontLibrary, Surface};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Tracer, Value};

use super::displayable::DisplayableData;
use super::theme::Theme;

const CLASS: &str = "javax/microedition/lcdui/Screen";

/// The state a high-level screen keeps per object, inside its
/// [`DisplayableData`]. It may hold Java references, which it reports from
/// [`ScreenState::trace`] like any payload.
pub trait ScreenState: Any + Send {
    /// Marks every heap reference the state holds.
    fn trace(&self, tracer: &mut Tracer<'_>) {
        let _ = tracer;
    }

    /// Bytes the state counts against the heap, beyond its object.
    fn heap_bytes(&self) -> usize {
        0
    }
}

/// Whether a key went down, repeats while held or came up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyPhase {
    Pressed,
    Repeated,
    Released,
}

/// A key event for a screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key code the phone sends.
    pub code: i32,
    /// Its game action (`Canvas.UP` and so on), 0 for none.
    pub game_action: i32,
    pub phase: KeyPhase,
}

/// Where a screen paints: a surface the size of its content area, cleared
/// to the theme's background before each paint, and what it needs to draw
/// the way the runtime does.
///
/// `surface` is the screen's buffer, locked for the whole paint: never
/// lock that buffer again from the hook (`canvas::buffer` of the screen
/// gives the same lock, which is not reentrant), and never hold an
/// image's surface lock while calling into Java.
pub struct ScreenTarget<'a> {
    pub surface: &'a mut Surface,
    pub width: i32,
    pub height: i32,
    pub fonts: &'a FontLibrary,
    pub theme: &'a Theme,
}

/// How a high-level screen behaves. Implementations are `static`s; their
/// per-object state is the [`ScreenState`] in the object's payload.
pub trait ScreenHandler: Send + Sync + 'static {
    /// Paints the content area. Only the target is drawn: titles, tickers,
    /// soft keys and menus are the runtime's.
    fn paint(
        &self,
        ctx: &mut Ctx<'_>,
        screen: ObjRef,
        target: &mut ScreenTarget<'_>,
    ) -> JResult<()>;

    /// A key the soft keys and menu did not take. Returns whether the
    /// screen used it.
    fn key(&self, ctx: &mut Ctx<'_>, screen: ObjRef, event: KeyEvent) -> JResult<bool> {
        let _ = (ctx, screen, event);
        Ok(false)
    }

    /// A pointer event inside the content area, in content coordinates.
    fn pointer(
        &self,
        ctx: &mut Ctx<'_>,
        screen: ObjRef,
        action: PointerAction,
        x: i32,
        y: i32,
    ) -> JResult<bool> {
        let _ = (ctx, screen, action, x, y);
        Ok(false)
    }

    /// Text typed on a host keyboard, for text fields.
    fn text(&self, ctx: &mut Ctx<'_>, screen: ObjRef, text: &str) -> JResult<bool> {
        let _ = (ctx, screen, text);
        Ok(false)
    }

    /// The screen became the shown displayable.
    fn shown(&self, ctx: &mut Ctx<'_>, screen: ObjRef) -> JResult<()> {
        let _ = (ctx, screen);
        Ok(())
    }

    /// The screen stopped being shown.
    fn hidden(&self, ctx: &mut Ctx<'_>, screen: ObjRef) -> JResult<()> {
        let _ = (ctx, screen);
        Ok(())
    }

    /// One of the screen's commands fired (from a soft key or the menu).
    /// Returns whether the screen handled it itself; if not, the
    /// listener's `commandAction` runs.
    fn command(&self, ctx: &mut Ctx<'_>, screen: ObjRef, command: ObjRef) -> JResult<bool> {
        let _ = (ctx, screen, command);
        Ok(false)
    }

    /// Called on each pass of the event loop while the screen is shown.
    /// Returns when it wants to be called again at the latest.
    fn tick(&self, ctx: &mut Ctx<'_>, screen: ObjRef, now: Instant) -> JResult<Option<Instant>> {
        let _ = (ctx, screen, now);
        Ok(None)
    }
}

/// The class `Screen`, the base of the high-level screens.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Displayable")
        .abstract_class()
        .payload_with(|| Box::new(DisplayableData::screen(None, None)))
        .method_with("<init>", "()V", 0, |_, _| Ok(Value::Void))
}
