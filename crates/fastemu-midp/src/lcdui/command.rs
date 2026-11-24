//! `javax.microedition.lcdui.Command` and `CommandListener`, and how the
//! runtime fires a command.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::displayable;
use super::event::callback;

const CLASS: &str = "javax/microedition/lcdui/Command";
const LISTENER: &str = "javax/microedition/lcdui/CommandListener";
const COMMAND_ACTION: &str =
    "(Ljavax/microedition/lcdui/Command;Ljavax/microedition/lcdui/Displayable;)V";

/// `Command.SCREEN`.
pub const SCREEN: i32 = 1;
/// `Command.BACK`.
pub const BACK: i32 = 2;
/// `Command.EXIT`.
pub const EXIT: i32 = 7;
/// `Command.ITEM`, the highest type.
pub const ITEM: i32 = 8;

/// What a Java `Command` holds.
#[derive(Debug, Default)]
pub struct CommandData {
    pub label: Option<ObjRef>,
    pub long_label: Option<ObjRef>,
    pub kind: i32,
    pub priority: i32,
}

impl Payload for CommandData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.label);
        tracer.mark_opt(self.long_label);
    }
}

/// A command's type, priority and label, read for the soft keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandInfo {
    pub kind: i32,
    pub priority: i32,
    /// The label as UTF-16 units, the long label when the command has one.
    pub label: Vec<u16>,
    pub short_label: Vec<u16>,
}

/// What the soft keys and menus need of `command`.
pub fn info(ctx: &Ctx<'_>, command: ObjRef) -> JResult<CommandInfo> {
    let data = ctx.payload::<CommandData>(command)?;
    let text = |s: Option<ObjRef>| -> JResult<Vec<u16>> {
        match s {
            Some(s) => Ok(ctx.string_utf16(s)?.to_vec()),
            None => Ok(Vec::new()),
        }
    };
    let short_label = text(data.label)?;
    let long = text(data.long_label)?;
    Ok(CommandInfo {
        kind: data.kind,
        priority: data.priority,
        label: if long.is_empty() {
            short_label.clone()
        } else {
            long
        },
        short_label,
    })
}

/// Fires `command` on `displayable`, as a soft key or menu choice does:
/// the screen's own handling first (an `Alert` dismissing itself), then
/// the listener's `commandAction` on the event thread. An exception the
/// listener throws is logged.
pub fn fire(ctx: &mut Ctx<'_>, displayable: ObjRef, command: ObjRef) -> JResult<()> {
    if let Some(handler) = displayable::handler(ctx, displayable)?
        && handler.command(ctx, displayable, command)?
    {
        return Ok(());
    }
    let Some(listener) = displayable::data(ctx, displayable)?.listener else {
        return Ok(());
    };
    let listener = ctx.root(listener);
    callback(ctx, "commandAction", |ctx| {
        ctx.call_interface(
            listener,
            "commandAction",
            COMMAND_ACTION,
            &[command.into(), displayable.into()],
        )
        .map(drop)
    })
}

fn init(
    ctx: &mut Ctx<'_>,
    this: Value,
    short: Value,
    long: Value,
    kind: i32,
    priority: i32,
) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let short = ctx.non_null(short)?;
    if !(SCREEN..=ITEM).contains(&kind) {
        return ctx.throw("java/lang/IllegalArgumentException", "invalid command type");
    }
    let data = ctx.payload_mut::<CommandData>(this)?;
    data.label = Some(short);
    data.long_label = long.obj();
    data.kind = kind;
    data.priority = priority;
    Ok(Value::Void)
}

fn this_data<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a CommandData> {
    let this = ctx.non_null(this)?;
    ctx.payload::<CommandData>(this)
}

/// The classes: `Command` and `CommandListener`.
pub fn classes() -> Vec<NativeClass> {
    let int = |v| Value::Int(v);
    let command = NativeClass::new(CLASS)
        .payload::<CommandData>()
        .constant("SCREEN", "I", int(SCREEN))
        .constant("BACK", "I", int(BACK))
        .constant("CANCEL", "I", int(3))
        .constant("OK", "I", int(4))
        .constant("HELP", "I", int(5))
        .constant("STOP", "I", int(6))
        .constant("EXIT", "I", int(EXIT))
        .constant("ITEM", "I", int(ITEM))
        .method("<init>", "(Ljava/lang/String;II)V", |ctx, a| {
            init(ctx, a[0], a[1], Value::NULL, a[2].int(), a[3].int())
        })
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/String;II)V",
            |ctx, a| init(ctx, a[0], a[1], a[2], a[3].int(), a[4].int()),
        )
        .method("getLabel", "()Ljava/lang/String;", |ctx, a| {
            Ok(this_data(ctx, a[0])?.label.into())
        })
        .method("getLongLabel", "()Ljava/lang/String;", |ctx, a| {
            Ok(this_data(ctx, a[0])?.long_label.into())
        })
        .method("getCommandType", "()I", |ctx, a| {
            Ok(Value::Int(this_data(ctx, a[0])?.kind))
        })
        .method("getPriority", "()I", |ctx, a| {
            Ok(Value::Int(this_data(ctx, a[0])?.priority))
        });
    let listener =
        NativeClass::interface(LISTENER).abstract_method("commandAction", COMMAND_ACTION);
    vec![command, listener]
}
