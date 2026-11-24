//! `javax.microedition.lcdui.Alert` and `AlertType`: a message shown for
//! a while or until dismissed, then the screen after it.
//!
//! An alert without commands of the game's own has `DISMISS_COMMAND`.
//! It goes after its timeout (two seconds unless set, never with
//! `FOREVER`, with two or more commands, or when its text needs
//! scrolling) or when a command fires. Without a `CommandListener` any
//! command dismisses it and shows the screen given to
//! `Display.setCurrent(alert, next)`, or the one shown before it; with a
//! listener, the listener hears the command (`DISMISS_COMMAND` on a
//! timeout) and decides. Showing an alert plays its type's sound.

use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value, classfile::access};

use super::draw::{Fonts, Painter, wrap};
use super::item::{self, Kind, gauge};
use super::{ticker_tick, wake_timer};
use crate::env::env;
use crate::lcdui::Rect;
use crate::lcdui::command;
use crate::lcdui::display;
use crate::lcdui::displayable::{self, DisplayableData};
use crate::lcdui::image::surface_of;
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase, ScreenHandler, ScreenState, ScreenTarget};
use crate::lcdui::state::state;

const CLASS: &str = "javax/microedition/lcdui/Alert";
const TYPE_CLASS: &str = "javax/microedition/lcdui/AlertType";
const COMMAND: &str = "Ljavax/microedition/lcdui/Command;";
const TYPE: &str = "Ljavax/microedition/lcdui/AlertType;";

/// `Alert.FOREVER`.
pub const FOREVER: i32 = -2;
/// `getDefaultTimeout()`, in milliseconds.
const DEFAULT_TIMEOUT: i32 = 2000;

/// The five alert types, by the order of their `AlertType` fields.
const TYPES: [&str; 5] = ["INFO", "WARNING", "ERROR", "ALARM", "CONFIRMATION"];

/// What a Java `AlertType` holds: which of the five it is, 1 to 5, or 0
/// for a game's own subclass.
#[derive(Debug, Default)]
pub struct AlertTypeData(i32);

impl Payload for AlertTypeData {}

/// The sound of an alert type: a MIDI note, how long, and a second note
/// for the two-tone types.
fn sound(kind: i32) -> Option<(i32, i32, Option<i32>)> {
    match kind {
        1 => Some((76, 120, None)),
        2 => Some((69, 180, Some(65))),
        3 => Some((57, 300, None)),
        4 => Some((84, 250, Some(88))),
        5 => Some((79, 100, Some(84))),
        _ => None,
    }
}

/// `AlertType.playSound`: a short tone on the instance's sound. Returns
/// whether anything played.
fn play(ctx: &mut Ctx<'_>, alert_type: ObjRef) -> bool {
    let kind = ctx
        .try_payload::<AlertTypeData>(alert_type)
        .map_or(0, |d| d.0);
    let (Some((note, ms, second)), Some(group)) = (sound(kind), env(ctx).audio.clone()) else {
        return false;
    };
    // Only a closed group (the instance ending) refuses, and silence is
    // right then.
    let played = group.play_tone(note, ms, 80).is_ok();
    if let Some(second) = second {
        let _ = group.play_tone(second, ms, 80);
    }
    played
}

/// What an `Alert` keeps.
#[derive(Default)]
pub struct AlertState {
    text: Option<ObjRef>,
    image: Option<ObjRef>,
    alert_type: Option<ObjRef>,
    timeout: i32,
    indicator: Option<ObjRef>,
    /// When it was last shown.
    shown_at: Option<Instant>,
    scroll: i32,
    /// The height of the content when it was last painted.
    height: i32,
}

impl ScreenState for AlertState {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.text);
        tracer.mark_opt(self.image);
        tracer.mark_opt(self.alert_type);
        tracer.mark_opt(self.indicator);
    }
}

fn alert<'a>(ctx: &'a Ctx<'_>, obj: ObjRef) -> JResult<&'a AlertState> {
    match displayable::data(ctx, obj)?.state::<AlertState>() {
        Some(s) => Ok(s),
        None => unreachable!("an Alert's payload is made by its class"),
    }
}

fn alert_mut<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut AlertState> {
    match displayable::data_mut(ctx, obj)?.state_mut::<AlertState>() {
        Some(s) => Ok(s),
        None => unreachable!("an Alert's payload is made by its class"),
    }
}

/// `Alert.DISMISS_COMMAND`.
fn dismiss_command(ctx: &mut Ctx<'_>) -> JResult<ObjRef> {
    let field = ctx.static_field_id(CLASS, "DISMISS_COMMAND", COMMAND)?;
    let value = ctx.get_static(field)?;
    ctx.non_null(value)
}

/// Whether the alert stays until a command dismisses it: `FOREVER`, or
/// two or more commands.
fn modal(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<bool> {
    Ok(alert(ctx, obj)?.timeout == FOREVER || displayable::data(ctx, obj)?.commands().len() >= 2)
}

/// Shows what follows the alert.
fn dismiss(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
    alert_mut(ctx, obj)?.shown_at = None;
    match displayable::data(ctx, obj)?.after_alert() {
        Some(next) if next != obj => {
            let next = ctx.root(next);
            display::set_current(ctx, next);
        }
        _ => {}
    }
    Ok(())
}

/// The icon of an alert type, drawn in `r`.
fn icon(p: &mut Painter<'_>, kind: i32, r: Rect) {
    let theme = *p.theme;
    let (color, mark) = match kind {
        1 => (theme.highlighted_background, "i"),
        2 => (0xE0_9000, "!"),
        3 => (0xC0_2020, "x"),
        4 => (0xE0_6000, "!"),
        5 => (0x20_9040, "v"),
        _ => return,
    };
    p.g.set_color(color);
    p.g.fill_arc(p.surface, r.x, r.y, r.width, r.height, 0, 360);
    let font = p.fonts.label.clone();
    let text: Vec<u16> = mark.encode_utf16().collect();
    let x = r.x + (r.width - super::draw::width(&font, &text)) / 2;
    let y = r.y + (r.height - font.height()) / 2;
    p.text(&font, &text, x, y, 0xFF_FFFF);
}

/// The kind (1 to 5) of the alert's type, 0 for none.
fn type_kind(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<i32> {
    Ok(alert(ctx, obj)?
        .alert_type
        .and_then(|t| ctx.try_payload::<AlertTypeData>(t))
        .map_or(0, |d| d.0))
}

/// The indicator gauge's state, if the alert has one.
fn indicator(ctx: &Ctx<'_>, obj: ObjRef) -> JResult<Option<gauge::GaugeState>> {
    let Some(g) = alert(ctx, obj)?.indicator else {
        return Ok(None);
    };
    Ok(match item::data(ctx, g)?.kind {
        Kind::Gauge(state) => Some(state),
        _ => None,
    })
}

struct AlertHandler;

static ALERT: AlertHandler = AlertHandler;

impl ScreenHandler for AlertHandler {
    fn paint(&self, ctx: &mut Ctx<'_>, obj: ObjRef, target: &mut ScreenTarget<'_>) -> JResult<()> {
        let kind = type_kind(ctx, obj)?;
        let gauge = indicator(ctx, obj)?;
        let a = alert(ctx, obj)?;
        let (image, scroll) = (a.image, a.scroll);
        let text = item::units(ctx, a.text)?;
        let Some(mut p) = Painter::new(
            target.surface,
            target.width,
            target.height,
            target.fonts,
            target.theme,
        ) else {
            return Ok(());
        };
        let theme = *p.theme;
        let font = p.fonts.text.clone();
        let lines = wrap(&font, &text, target.width - 12);
        let side = p.fonts.label.height() * 2;
        let picture = match image {
            Some(image) => surface_of(ctx, image)?.lock().height() as i32 + 6,
            None if kind != 0 => side + 6,
            None => 0,
        };
        let bar = if gauge.is_some() {
            font.height() + 10
        } else {
            0
        };
        // Short content sits in the middle; long content scrolls.
        let content = picture + lines.len() as i32 * font.height() + bar + 12;
        let mut y = (target.height - content).max(0) / 2 + 6 - scroll;
        match image {
            Some(image) => {
                let surface = surface_of(ctx, image)?;
                let pixels = surface.lock();
                let x = (target.width - pixels.width() as i32) / 2;
                p.image(&pixels, x, y);
            }
            None if kind != 0 => {
                let r = Rect::new((target.width - side) / 2, y, side, side);
                icon(&mut p, kind, r);
            }
            None => {}
        }
        y += picture;
        for line in &lines {
            let units = &text[line.clone()];
            let x = (target.width - super::draw::width(&font, units)) / 2;
            p.text(&font, units, x.max(6), y, theme.foreground);
            y += font.height();
        }
        if let Some(g) = gauge {
            let r = Rect::new(12, y + 6, target.width - 24, font.height() + 4);
            gauge::draw(&mut p, g, r, false);
        }
        p.scrollbar(content, target.height, scroll);
        alert_mut(ctx, obj)?.height = content;
        Ok(())
    }

    fn key(&self, ctx: &mut Ctx<'_>, obj: ObjRef, event: KeyEvent) -> JResult<bool> {
        if event.phase == KeyPhase::Released {
            return Ok(false);
        }
        let view = displayable::layout(ctx, obj)?.content.height;
        let line = Fonts::new(&state(ctx).fonts).text.height();
        match event.game_action {
            keys::UP | keys::DOWN => {
                let a = alert_mut(ctx, obj)?;
                let delta = if event.game_action == keys::UP {
                    -line
                } else {
                    line
                };
                let next = (a.scroll + delta).clamp(0, (a.height - view).max(0));
                let moved = next != a.scroll;
                a.scroll = next;
                if moved {
                    display::repaint(ctx, obj)?;
                }
                Ok(true)
            }
            keys::FIRE if event.phase == KeyPhase::Pressed => {
                let commands = displayable::data(ctx, obj)?.commands().to_vec();
                if let [only] = commands[..] {
                    let only = ctx.root(only);
                    command::fire(ctx, obj, only)?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn shown(&self, ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<()> {
        let a = alert_mut(ctx, obj)?;
        a.shown_at = Some(Instant::now());
        a.scroll = 0;
        if let Some(t) = alert(ctx, obj)?.alert_type {
            let t = ctx.root(t);
            play(ctx, t);
        }
        Ok(())
    }

    fn command(&self, ctx: &mut Ctx<'_>, obj: ObjRef, _: ObjRef) -> JResult<bool> {
        if displayable::data(ctx, obj)?.listener().is_some() {
            return Ok(false);
        }
        dismiss(ctx, obj)?;
        Ok(true)
    }

    fn tick(&self, ctx: &mut Ctx<'_>, obj: ObjRef, now: Instant) -> JResult<Option<Instant>> {
        let mut next = ticker_tick(ctx, obj, now)?;
        let mut sooner = |at: Instant| next = Some(next.map_or(at, |n| n.min(at)));
        if indicator(ctx, obj)?.is_some_and(|g| g.running()) {
            display::repaint(ctx, obj)?;
            sooner(now + gauge::FRAME);
        }
        let view = displayable::layout(ctx, obj)?.content.height;
        let a = alert(ctx, obj)?;
        let scrolls = a.height > view;
        let (Some(shown), timeout) = (a.shown_at, a.timeout) else {
            return Ok(next);
        };
        if modal(ctx, obj)? || scrolls {
            return Ok(next);
        }
        let due = shown + Duration::from_millis(timeout.max(0) as u64);
        if now >= due {
            let dismiss = dismiss_command(ctx)?;
            alert_mut(ctx, obj)?.shown_at = None;
            command::fire(ctx, obj, dismiss)?;
            return Ok(next);
        }
        sooner(due);
        Ok(next)
    }
}

fn this(ctx: &mut Ctx<'_>, v: Value) -> JResult<ObjRef> {
    ctx.non_null(v)
}

/// Repaints a changed alert and restarts its timer.
fn changed(ctx: &mut Ctx<'_>, obj: ObjRef) -> JResult<Value> {
    wake_timer(ctx, obj);
    display::repaint(ctx, obj)?;
    Ok(Value::Void)
}

fn init(ctx: &mut Ctx<'_>, a: &[Value], rest: Option<(Value, Value, Value)>) -> JResult<Value> {
    let obj = this(ctx, a[0])?;
    let dismiss = dismiss_command(ctx)?;
    let d = displayable::data_mut(ctx, obj)?;
    d.title = a[1].obj();
    d.add_command(dismiss);
    let s = alert_mut(ctx, obj)?;
    s.timeout = DEFAULT_TIMEOUT;
    if let Some((text, image, kind)) = rest {
        s.text = text.obj();
        s.image = image.obj();
        s.alert_type = kind.obj();
    }
    Ok(Value::Void)
}

/// `setIndicator`: the gauge must be free and plain, as MIDP requires.
fn set_indicator(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let obj = this(ctx, a[0])?;
    if let Some(old) = alert(ctx, obj)?.indicator {
        let d = item::data_mut(ctx, old)?;
        d.owner = None;
        d.in_alert = false;
    }
    let Some(g) = a[1].obj() else {
        alert_mut(ctx, obj)?.indicator = None;
        return changed(ctx, obj);
    };
    let d = item::data(ctx, g)?;
    let suitable = matches!(&d.kind, Kind::Gauge(state) if !state.interactive)
        && d.owner.is_none()
        && d.label.is_none()
        && d.commands.is_empty()
        && d.listener.is_none()
        && d.layout == 0
        && d.preferred == (-1, -1);
    if !suitable {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "an Alert's indicator is a free, non-interactive Gauge with no label, \
             commands or layout",
        );
    }
    let d = item::data_mut(ctx, g)?;
    d.owner = Some(obj);
    d.in_alert = true;
    alert_mut(ctx, obj)?.indicator = Some(g);
    changed(ctx, obj)
}

/// The class `Alert`.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Screen")
        .payload_with(|| Box::new(DisplayableData::for_screen(&ALERT, AlertState::default())))
        .constant("FOREVER", "I", int(FOREVER))
        .field_with(
            "DISMISS_COMMAND",
            COMMAND,
            access::PUBLIC | access::STATIC | access::FINAL,
        )
        .static_init(|ctx, _| {
            let label = ctx.new_string("")?;
            let dismiss = ctx.construct(
                "javax/microedition/lcdui/Command",
                "(Ljava/lang/String;II)V",
                &[label.into(), Value::Int(4), Value::Int(0)],
            )?;
            let field = ctx.static_field_id(CLASS, "DISMISS_COMMAND", COMMAND)?;
            ctx.set_static(field, dismiss.into())?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            init(ctx, a, None)
        })
        .method(
            "<init>",
            &format!(
                "(Ljava/lang/String;Ljava/lang/String;Ljavax/microedition/lcdui/Image;{TYPE})V"
            ),
            |ctx, a| init(ctx, a, Some((a[2], a[3], a[4]))),
        )
        .method("getDefaultTimeout", "()I", |_, _| {
            Ok(Value::Int(DEFAULT_TIMEOUT))
        })
        .method("getTimeout", "()I", |ctx, a| {
            let obj = this(ctx, a[0])?;
            if modal(ctx, obj)? {
                return Ok(Value::Int(FOREVER));
            }
            Ok(Value::Int(alert(ctx, obj)?.timeout))
        })
        .method("setTimeout", "(I)V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            let time = a[1].int();
            if time <= 0 && time != FOREVER {
                return ctx.throw(
                    "java/lang/IllegalArgumentException",
                    "a timeout is positive or FOREVER",
                );
            }
            let s = alert_mut(ctx, obj)?;
            s.timeout = time;
            if s.shown_at.is_some() {
                s.shown_at = Some(Instant::now());
            }
            changed(ctx, obj)
        })
        .method("getType", &format!("(){TYPE}"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(alert(ctx, obj)?.alert_type.into())
        })
        .method("setType", &format!("({TYPE})V"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            alert_mut(ctx, obj)?.alert_type = a[1].obj();
            changed(ctx, obj)
        })
        .method("getString", "()Ljava/lang/String;", |ctx, a| {
            let obj = this(ctx, a[0])?;
            Ok(alert(ctx, obj)?.text.into())
        })
        .method("setString", "(Ljava/lang/String;)V", |ctx, a| {
            let obj = this(ctx, a[0])?;
            alert_mut(ctx, obj)?.text = a[1].obj();
            changed(ctx, obj)
        })
        .method(
            "getImage",
            "()Ljavax/microedition/lcdui/Image;",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                Ok(alert(ctx, obj)?.image.into())
            },
        )
        .method(
            "setImage",
            "(Ljavax/microedition/lcdui/Image;)V",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                alert_mut(ctx, obj)?.image = a[1].obj();
                changed(ctx, obj)
            },
        )
        .method(
            "setIndicator",
            "(Ljavax/microedition/lcdui/Gauge;)V",
            set_indicator,
        )
        .method(
            "getIndicator",
            "()Ljavax/microedition/lcdui/Gauge;",
            |ctx, a| {
                let obj = this(ctx, a[0])?;
                Ok(alert(ctx, obj)?.indicator.into())
            },
        )
        .method("addCommand", &format!("({COMMAND})V"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let cmd = ctx.non_null(a[1])?;
            let dismiss = dismiss_command(ctx)?;
            if cmd != dismiss {
                let d = displayable::data_mut(ctx, obj)?;
                d.remove_command(dismiss);
                d.add_command(cmd);
                display::relayout(ctx, obj);
            }
            changed(ctx, obj)
        })
        .method("removeCommand", &format!("({COMMAND})V"), |ctx, a| {
            let obj = this(ctx, a[0])?;
            let Some(cmd) = a[1].obj() else {
                return Ok(Value::Void);
            };
            let dismiss = dismiss_command(ctx)?;
            if cmd == dismiss {
                return Ok(Value::Void);
            }
            let d = displayable::data_mut(ctx, obj)?;
            d.remove_command(cmd);
            if d.commands().is_empty() {
                d.add_command(dismiss);
            }
            display::relayout(ctx, obj);
            changed(ctx, obj)
        })
}

/// The class `AlertType`.
pub fn type_class() -> NativeClass {
    let mut class = NativeClass::new(TYPE_CLASS).payload::<AlertTypeData>();
    for name in TYPES {
        class = class.field_with(name, TYPE, access::PUBLIC | access::STATIC | access::FINAL);
    }
    class
        .static_init(|ctx, _| {
            let class = ctx.find_class(TYPE_CLASS)?;
            for (i, name) in TYPES.iter().enumerate() {
                let obj = ctx.new_object(class)?;
                ctx.set_payload(obj, AlertTypeData(i as i32 + 1));
                let field = ctx.static_field_id(TYPE_CLASS, name, TYPE)?;
                ctx.set_static(field, obj.into())?;
            }
            Ok(Value::Void)
        })
        .method_with("<init>", "()V", access::PROTECTED, |_, _| Ok(Value::Void))
        .method(
            "playSound",
            "(Ljavax/microedition/lcdui/Display;)Z",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                ctx.non_null(a[1])?;
                Ok(Value::from(play(ctx, this)))
            },
        )
}
