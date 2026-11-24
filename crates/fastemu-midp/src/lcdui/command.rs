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

/// What a soft key shows for a command of `kind` without a label.
fn type_name(kind: i32) -> &'static str {
    match kind {
        BACK => "Back",
        3 => "Cancel",
        4 => "OK",
        5 => "Help",
        6 => "Stop",
        EXIT => "Exit",
        _ => "Select",
    }
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
    let mut short_label = text(data.label)?;
    if short_label.is_empty() {
        // `Alert.DISMISS_COMMAND` and `List.SELECT_COMMAND` have empty
        // labels; a soft key says what the type does instead.
        short_label = type_name(data.kind).encode_utf16().collect();
    }
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
/// the listener's `commandAction` on the event thread. Both run as
/// callbacks, inside the callback lock; an exception either throws is
/// logged.
pub fn fire(ctx: &mut Ctx<'_>, displayable: ObjRef, command: ObjRef) -> JResult<()> {
    if let Some(handler) = displayable::handler(ctx, displayable)? {
        let mut handled = false;
        callback(ctx, "a screen's command", |ctx| {
            handled = handler.command(ctx, displayable, command)?;
            Ok(())
        })?;
        if handled {
            return Ok(());
        }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;
    use crate::env::InstanceEnv;
    use crate::lcdui::displayable::DisplayableData;
    use crate::lcdui::screen::{ScreenHandler, ScreenState, ScreenTarget};
    use crate::lcdui::state::state;

    /// A screen whose command hook throws and checks the callback lock.
    struct Throws;

    impl ScreenHandler for Throws {
        fn paint(&self, _: &mut Ctx<'_>, _: ObjRef, _: &mut ScreenTarget<'_>) -> JResult<()> {
            Ok(())
        }

        fn command(&self, ctx: &mut Ctx<'_>, _: ObjRef, _: ObjRef) -> JResult<bool> {
            let me = ctx.thread_id();
            if state(ctx).callback.holder != Some(me) {
                return ctx.throw("java/lang/IllegalStateException", "outside the lock");
            }
            ctx.throw("java/lang/IllegalStateException", "from the screen")
        }
    }

    static THROWS: Throws = Throws;

    struct NoState;

    impl ScreenState for NoState {}

    #[test]
    fn a_screen_command_runs_as_a_callback() {
        let env = InstanceEnv::for_tests("unused");
        let host = env.host.clone();
        let mut classes = crate::classes(&fastemu_core::default_profile());
        classes.push(
            NativeClass::new("t/Throwing")
                .extends("javax/microedition/lcdui/Screen")
                .payload_with(|| Box::new(DisplayableData::for_screen(&THROWS, NoState)))
                .method("<init>", "()V", |_, _| Ok(Value::Void)),
        );
        let config = VmConfig {
            log: host.log_sink(),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(MemorySource::new()), classes);
        let fired = vm.attach("test", move |ctx| {
            ctx.set_extension(env);
            let screen = ctx.construct("t/Throwing", "()V", &[])?;
            let label = ctx.new_string("Go")?;
            let command = ctx.construct(
                CLASS,
                "(Ljava/lang/String;II)V",
                &[label.into(), Value::Int(SCREEN), Value::Int(1)],
            )?;
            fire(ctx, screen, command)
        });
        assert!(fired.is_ok(), "the exception escaped the event thread");
        let log: Vec<String> = host.drain_log().into_iter().map(|l| l.text).collect();
        assert!(log.iter().any(|l| l.contains("from the screen")), "{log:?}");
        assert_eq!(host.error_count(), 1);
    }
}
