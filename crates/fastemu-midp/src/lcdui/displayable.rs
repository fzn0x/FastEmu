//! `javax.microedition.lcdui.Displayable`: what every canvas and screen
//! shares (title, ticker, commands, listener, size), and the layout of a
//! displayable on the phone's screen.
//!
//! Every `Displayable` object carries one [`DisplayableData`] payload; the
//! payload type is the same for canvases and screens, since an object has
//! one payload and subclasses inherit their parent's. Canvases and the
//! high-level screens differ in its kind and in the [`ScreenHandler`] a
//! screen names.

use std::any::Any;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value, classfile::access};

use super::display;
use super::image::SharedSurface;
use super::screen::{ScreenHandler, ScreenState};
use super::state::{Rect, state};

const CLASS: &str = "javax/microedition/lcdui/Displayable";

/// Whether a displayable is a `Canvas` (the game draws it) or a `Screen`
/// (the runtime lays it out).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Canvas,
    Screen,
}

/// What every `Displayable` object holds.
pub struct DisplayableData {
    kind: Kind,
    pub(crate) title: Option<ObjRef>,
    pub(crate) ticker: Option<ObjRef>,
    pub(crate) commands: Vec<ObjRef>,
    pub(crate) listener: Option<ObjRef>,
    after_alert: Option<ObjRef>,
    pub(crate) full_screen: bool,
    /// The size last reported through `sizeChanged`.
    pub(crate) last_size: Option<(i32, i32)>,
    /// The pixels of the displayable: a canvas's back buffer, kept between
    /// paints, or a screen's content. Made on first paint, screen-sized.
    pub(crate) buffer: Option<SharedSurface>,
    /// The bytes of `buffer`, which the heap counts with the object: 4 for
    /// each pixel. A plain number, so a collection that measures the
    /// payload never waits for the buffer's lock.
    pub(crate) buffer_bytes: usize,
    /// The `Graphics` passed to `paint`, reset before each paint.
    pub(crate) graphics: Option<ObjRef>,
    suppress_game_keys: bool,
    handler: Option<&'static dyn ScreenHandler>,
    state: Option<Box<dyn ScreenState>>,
}

impl Payload for DisplayableData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.title);
        tracer.mark_opt(self.ticker);
        tracer.mark_opt(self.listener);
        tracer.mark_opt(self.after_alert);
        tracer.mark_opt(self.graphics);
        for &command in &self.commands {
            tracer.mark(command);
        }
        if let Some(state) = &self.state {
            state.trace(tracer);
        }
    }

    fn heap_bytes(&self) -> usize {
        self.buffer_bytes + self.state.as_ref().map_or(0, |s| s.heap_bytes())
    }
}

impl DisplayableData {
    fn new(kind: Kind) -> Self {
        Self {
            kind,
            title: None,
            ticker: None,
            commands: Vec::new(),
            listener: None,
            after_alert: None,
            full_screen: false,
            last_size: None,
            buffer: None,
            buffer_bytes: 0,
            graphics: None,
            suppress_game_keys: false,
            handler: None,
            state: None,
        }
    }

    /// The payload of a new `Canvas`.
    pub fn canvas() -> Self {
        Self::new(Kind::Canvas)
    }

    /// The payload of a new screen with `handler` and `state`.
    pub fn screen(
        handler: Option<&'static dyn ScreenHandler>,
        state: Option<Box<dyn ScreenState>>,
    ) -> Self {
        Self {
            handler,
            state,
            ..Self::new(Kind::Screen)
        }
    }

    /// The payload of a new high-level screen, for
    /// `NativeClass::payload_with(|| Box::new(DisplayableData::for_screen(&FORM, FormState::default())))`.
    pub fn for_screen(handler: &'static dyn ScreenHandler, state: impl ScreenState) -> Self {
        Self::screen(Some(handler), Some(Box::new(state)))
    }

    /// Whether this is a canvas or a screen.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The title, a Java `String`.
    pub fn title(&self) -> Option<ObjRef> {
        self.title
    }

    /// The ticker, a Java `Ticker`.
    pub fn ticker(&self) -> Option<ObjRef> {
        self.ticker
    }

    /// The commands in the order they were added.
    pub fn commands(&self) -> &[ObjRef] {
        &self.commands
    }

    /// Adds a command the screen itself provides (an `Alert`'s dismiss
    /// command, a `List`'s select command), unless it is there already.
    pub fn add_command(&mut self, command: ObjRef) {
        if !self.commands.contains(&command) {
            self.commands.push(command);
        }
    }

    /// Removes a command.
    pub fn remove_command(&mut self, command: ObjRef) {
        self.commands.retain(|&c| c != command);
    }

    /// The `CommandListener`.
    pub fn listener(&self) -> Option<ObjRef> {
        self.listener
    }

    /// The displayable to show after this `Alert` is dismissed, as given to
    /// `Display.setCurrent(Alert, Displayable)`.
    pub fn after_alert(&self) -> Option<ObjRef> {
        self.after_alert
    }

    /// Sets what follows this `Alert`.
    pub fn set_after_alert(&mut self, next: Option<ObjRef>) {
        self.after_alert = next;
    }

    /// Whether a canvas is in full-screen mode.
    pub fn is_full_screen(&self) -> bool {
        self.full_screen
    }

    /// Whether key callbacks for keys with a game action are suppressed,
    /// as `GameCanvas(true)` asks.
    pub fn suppresses_game_keys(&self) -> bool {
        self.suppress_game_keys
    }

    /// Asks for `GameCanvas(suppressKeyEvents)`'s behaviour: no
    /// `keyPressed`, `keyRepeated` or `keyReleased` for keys with a game
    /// action while this canvas is shown; other keys still arrive.
    pub fn set_suppress_game_keys(&mut self, suppress: bool) {
        self.suppress_game_keys = suppress;
    }

    /// The screen's handler, if it is a high-level screen.
    pub fn handler(&self) -> Option<&'static dyn ScreenHandler> {
        self.handler
    }

    /// The screen's state, if it is a `T`.
    pub fn state<T: ScreenState>(&self) -> Option<&T> {
        let state: &dyn Any = self.state.as_deref()?;
        state.downcast_ref()
    }

    /// The screen's state, mutably, if it is a `T`.
    pub fn state_mut<T: ScreenState>(&mut self) -> Option<&mut T> {
        let state: &mut dyn Any = self.state.as_deref_mut()?;
        state.downcast_mut()
    }
}

/// The payload of a `Displayable` object.
pub fn data<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> JResult<&'a DisplayableData> {
    ctx.payload::<DisplayableData>(obj)
}

/// The payload of a `Displayable` object, mutably.
pub fn data_mut<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut DisplayableData> {
    ctx.payload_mut::<DisplayableData>(obj)
}

/// The screen handler of `obj`, if it is a high-level screen.
pub fn handler(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<Option<&'static dyn ScreenHandler>> {
    Ok(data(ctx, obj)?.handler)
}

/// Where a displayable's parts go on the phone's screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The displayable's own pixels: a canvas, or a screen's content.
    pub content: Rect,
    /// A screen's title bar.
    pub title: Option<Rect>,
    /// A screen's ticker.
    pub ticker: Option<Rect>,
    /// The soft key bar.
    pub bar: Option<Rect>,
}

/// The height of the soft key bar and of a title bar.
pub(crate) fn bar_height(ctx: &mut Ctx<'_>) -> i32 {
    state(ctx).fonts.default_font().height() + 4
}

/// Lays `obj` out: a full-screen canvas takes the whole screen; any other
/// canvas loses the soft key bar's height when it has commands, as in J2ME
/// Loader; a screen has a title bar when it has a title, a ticker line
/// when it has a ticker, and always a soft key bar.
pub fn layout(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Layout> {
    let d = data(ctx, obj)?;
    let (kind, full, has_commands, has_title, has_ticker) = (
        d.kind,
        d.full_screen,
        !d.commands.is_empty(),
        d.title.is_some(),
        d.ticker.is_some(),
    );
    let bar_h = bar_height(ctx);
    let st = state(ctx);
    let (w, h) = (st.screen_width, st.screen_height);
    let ticker_h = st.fonts.default_font().height() + 2;
    let bar = Rect::new(0, h - bar_h, w, bar_h);
    Ok(match kind {
        Kind::Canvas if full || !has_commands => Layout {
            content: Rect::new(0, 0, w, h),
            title: None,
            ticker: None,
            bar: None,
        },
        Kind::Canvas => Layout {
            content: Rect::new(0, 0, w, (h - bar_h).max(1)),
            title: None,
            ticker: None,
            bar: Some(bar),
        },
        Kind::Screen => {
            let title = has_title.then(|| Rect::new(0, 0, w, bar_h));
            let top = title.map_or(0, |t| t.height);
            let ticker = has_ticker.then(|| Rect::new(0, top, w, ticker_h));
            let top = top + ticker.map_or(0, |t| t.height);
            Layout {
                content: Rect::new(0, top, w, (h - top - bar_h).max(1)),
                title,
                ticker,
                bar: Some(bar),
            }
        }
    })
}

/// `getWidth()` and `getHeight()` of `obj`.
pub fn size(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<(i32, i32)> {
    let content = layout(ctx, obj)?.content;
    Ok((content.width, content.height))
}

fn this(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    ctx.non_null(v)
}

/// Changes `obj` and tells the display, which re-lays it out if shown.
fn change(ctx: &mut Ctx<'_>, obj: ObjRef, f: impl FnOnce(&mut DisplayableData)) -> JResult<Value> {
    f(data_mut(ctx, obj)?);
    display::relayout(ctx, obj);
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .abstract_class()
        .payload_with(|| Box::new(DisplayableData::screen(None, None)))
        .method_with("<init>", "()V", 0, |_, _| Ok(Value::Void))
        .method("getTitle", "()Ljava/lang/String;", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(data(ctx, obj)?.title.into())
        })
        .method("setTitle", "(Ljava/lang/String;)V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            let title = a[1].obj();
            change(ctx, obj, |d| d.title = title)
        })
        .method(
            "getTicker",
            "()Ljavax/microedition/lcdui/Ticker;",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                Ok(data(ctx, obj)?.ticker.into())
            },
        )
        .method(
            "setTicker",
            "(Ljavax/microedition/lcdui/Ticker;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                let ticker = a[1].obj();
                change(ctx, obj, |d| d.ticker = ticker)
            },
        )
        .method("isShown", "()Z", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(Value::from(display::is_shown(ctx, obj)))
        })
        .method(
            "addCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                let command = ctx.non_null(a[1])?;
                change(ctx, obj, |d| d.add_command(command))
            },
        )
        .method(
            "removeCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                match a[1].obj() {
                    Some(command) => change(ctx, obj, |d| d.remove_command(command)),
                    None => Ok(Value::Void),
                }
            },
        )
        .method(
            "setCommandListener",
            "(Ljavax/microedition/lcdui/CommandListener;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                let listener = a[1].obj();
                change(ctx, obj, |d| d.listener = listener)
            },
        )
        .method("getWidth", "()I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(Value::Int(size(ctx, obj)?.0))
        })
        .method("getHeight", "()I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(Value::Int(size(ctx, obj)?.1))
        })
        .method_with("sizeChanged", "(II)V", access::PROTECTED, |_, _| {
            Ok(Value::Void)
        })
}
