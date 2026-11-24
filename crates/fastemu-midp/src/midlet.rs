//! `javax.microedition.midlet`: the MIDlet class and its lifecycle.
//!
//! Lifecycle calls (the constructor, `startApp`, `pauseApp`,
//! `destroyApp`) run on their own Java threads, never on the event
//! thread, so a game that never returns from `startApp` (it runs its game
//! loop there) still gets painted and receives keys. The instance's
//! control thread starts them; see [`crate::instance`].
//!
//! They run one at a time, in the order the control thread started them,
//! as on a phone's single lifecycle thread: each call has a turn, and its
//! thread waits until the call before it has returned. A `startApp` that
//! is still loading therefore sees no `pauseApp` and no second `startApp`
//! beside it. `destroyApp` alone does not wait, so a game that never
//! returns from `startApp` can still be stopped; the calls still waiting
//! then give up.

use std::sync::Arc;

use fastemu_core::{InstanceStatus, LogLevel, NetworkPolicy};
use fastemu_jvm::{
    Ctx, GlobalRef, JResult, NativeClass, ObjRef, Throw, Value, Wake, classfile::access,
};

use crate::env::env;
use crate::host::Control;
use crate::lcdui::state::log_exception;
use crate::suite::attributes;

const CLASS: &str = "javax/microedition/midlet/MIDlet";
const STATE_CHANGE: &str = "javax/microedition/midlet/MIDletStateChangeException";

/// Which lifecycle method a lifecycle thread calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifecycle {
    /// `startApp` again after a pause.
    Resume,
    /// `pauseApp`.
    Pause,
    /// `destroyApp(true)`.
    Destroy,
}

/// The instance's MIDlet, once constructed.
#[derive(Default)]
struct MidletState {
    midlet: Option<Arc<GlobalRef>>,
    /// Set while the runtime runs the MIDlet's constructor, the only time
    /// `MIDlet()` may run.
    constructing: bool,
    /// The turn of the lifecycle call that may run now; [`launch`] has
    /// turn 0.
    turn: u64,
    /// Set when `destroyApp` starts, so calls waiting for their turn give
    /// up.
    destroying: bool,
}

fn midlet_state<'a>(ctx: &'a mut Ctx<'_>) -> &'a mut MidletState {
    if ctx.extension::<MidletState>().is_none() {
        ctx.set_extension(MidletState::default());
    }
    ctx.extension_mut::<MidletState>().expect("installed above")
}

/// The instance's MIDlet object, if it has been constructed.
pub fn midlet(ctx: &mut Ctx<'_>) -> Option<ObjRef> {
    let global = midlet_state(ctx).midlet.clone();
    global.map(|g| g.get(ctx))
}

/// Logs `exception` and ends the instance as crashed with a message that
/// starts with `what`.
fn crash(ctx: &mut Ctx<'_>, what: &str, exception: ObjRef) {
    log_exception(ctx, what, exception);
    let description = ctx.describe_exception(exception);
    env(ctx)
        .host
        .control(Control::Crashed(format!("{what}: {description}")));
}

fn crash_with(ctx: &mut Ctx<'_>, message: String) {
    ctx.log(LogLevel::Err, message.clone());
    let host = &env(ctx).host;
    host.count_error();
    host.control(Control::Crashed(message));
}

/// Runs `f`, turning a Java exception into a crash described by `what`.
fn or_crash<T>(ctx: &mut Ctx<'_>, what: &str, result: JResult<T>) -> JResult<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(Throw::Exception(e)) => {
            crash(ctx, what, e);
            Ok(None)
        }
    }
}

/// Waits until lifecycle call `turn` may run. Returns `false` when the
/// MIDlet is being destroyed instead.
fn wait_for_turn(ctx: &mut Ctx<'_>, turn: u64) -> JResult<bool> {
    loop {
        let st = midlet_state(ctx);
        if st.destroying {
            return Ok(false);
        }
        if st.turn == turn {
            return Ok(true);
        }
        match ctx.wait_until(None)? {
            Wake::Paused => ctx.yield_now()?,
            Wake::Interrupted => {
                ctx.take_interrupt();
            }
            Wake::Notified | Wake::TimedOut => {}
        }
    }
}

/// Lets the next lifecycle call run.
fn end_turn(ctx: &mut Ctx<'_>) {
    midlet_state(ctx).turn += 1;
    ctx.notify_waiters();
}

/// The body of the first lifecycle thread, turn 0: loads the MIDlet class
/// named `class_name` (a binary name such as `com.acme.Game`), runs its
/// constructor and then `startApp`. A missing class, a class that is not
/// a MIDlet, or an exception from the constructor crashes the instance.
pub fn launch(ctx: &mut Ctx<'_>, class_name: &str) -> JResult<()> {
    let result = construct_and_start(ctx, class_name);
    end_turn(ctx);
    result
}

fn construct_and_start(ctx: &mut Ctx<'_>, class_name: &str) -> JResult<()> {
    let internal = class_name.replace('.', "/");
    let loaded = ctx.find_class(&internal);
    let what = format!("loading the MIDlet class {class_name}");
    let Some(class) = or_crash(ctx, &what, loaded)? else {
        return Ok(());
    };
    let base = ctx.find_class(CLASS)?;
    if !ctx.is_assignable(class, base) {
        crash_with(ctx, format!("{class_name} does not extend MIDlet"));
        return Ok(());
    }
    midlet_state(ctx).constructing = true;
    let constructed = ctx.construct_in(class, "()V", &[]);
    midlet_state(ctx).constructing = false;
    let Some(midlet) = or_crash(ctx, "the MIDlet's constructor", constructed)? else {
        return Ok(());
    };
    let global = Arc::new(ctx.new_global(midlet));
    midlet_state(ctx).midlet = Some(global);
    start_app(ctx, midlet)
}

/// Calls `startApp`. A `MIDletStateChangeException` leaves the MIDlet
/// paused, as MIDP says; any other exception crashes the instance.
fn start_app(ctx: &mut Ctx<'_>, midlet: ObjRef) -> JResult<()> {
    env(ctx).host.set_status(InstanceStatus::Running);
    match ctx.call_virtual(midlet, "startApp", "()V", &[]) {
        Ok(_) => Ok(()),
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(Throw::Exception(e)) => {
            let refused = ctx
                .find_class(STATE_CHANGE)
                .is_ok_and(|c| ctx.is_instance(e, c));
            if refused {
                log_exception(ctx, "startApp, which asked to stay paused", e);
                env(ctx).host.set_status(InstanceStatus::Paused);
            } else {
                crash(ctx, "startApp", e);
            }
            Ok(())
        }
    }
}

/// The body of a later lifecycle thread. `turn` is the call's place in
/// the order the control thread started lifecycle calls, counting
/// [`launch`] as 0; the call waits until every earlier one has returned.
/// `Lifecycle::Destroy` ignores it and runs at once. Without a MIDlet
/// (its constructor failed) the call does nothing.
pub fn lifecycle(ctx: &mut Ctx<'_>, which: Lifecycle, turn: u64) -> JResult<()> {
    if which == Lifecycle::Destroy {
        midlet_state(ctx).destroying = true;
        ctx.notify_waiters();
    } else if !wait_for_turn(ctx, turn)? {
        return Ok(());
    }
    let result = run_lifecycle(ctx, which);
    if which != Lifecycle::Destroy {
        end_turn(ctx);
    }
    result
}

fn run_lifecycle(ctx: &mut Ctx<'_>, which: Lifecycle) -> JResult<()> {
    let Some(midlet) = midlet(ctx) else {
        return Ok(());
    };
    match which {
        Lifecycle::Resume => start_app(ctx, midlet),
        Lifecycle::Pause => {
            let result = ctx.call_virtual(midlet, "pauseApp", "()V", &[]);
            or_crash(ctx, "pauseApp", result).map(drop)
        }
        Lifecycle::Destroy => {
            // Exceptions from destroyApp(true) are ignored: the MIDlet ends
            // whatever it throws.
            let result = ctx.call_virtual(midlet, "destroyApp", "(Z)V", &[Value::from(true)]);
            match result {
                Err(Throw::Exception(e)) => {
                    log_exception(ctx, "destroyApp", e);
                    Ok(())
                }
                other => other.map(drop),
            }
        }
    }
}

/// `getAppProperty`: the user's overrides first, then the JAD, then the
/// manifest; an exact name first, then one that differs in case.
fn app_property(ctx: &mut Ctx<'_>, key: &str) -> Option<String> {
    let env = env(ctx);
    attributes::lookup(&env.settings.properties, key)
        .or_else(|| attributes::lookup(&env.suite.properties, key))
        .map(str::to_owned)
}

/// `checkPermission`: network permissions follow the instance's network
/// policy; every other permission is denied.
fn permission(ctx: &mut Ctx<'_>, name: &str) -> i32 {
    if name.starts_with("javax.microedition.io.Connector.") {
        i32::from(env(ctx).settings.network == NetworkPolicy::Allow)
    } else {
        0
    }
}

fn string_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<String> {
    let s = ctx.non_null(v)?;
    ctx.string(s)
}

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    let protected = access::PROTECTED;
    let final_method = access::PUBLIC | access::FINAL;
    let midlet = NativeClass::new(CLASS)
        .abstract_class()
        .method_with("<init>", "()V", protected, |ctx, _| {
            if midlet_state(ctx).constructing {
                Ok(Value::Void)
            } else {
                ctx.throw(
                    "java/lang/SecurityException",
                    "only the application manager creates MIDlets",
                )
            }
        })
        .abstract_method("startApp", "()V")
        .abstract_method("pauseApp", "()V")
        .abstract_method("destroyApp", "(Z)V")
        .method_with("notifyDestroyed", "()V", final_method, |ctx, _| {
            env(ctx).host.control(Control::Exited);
            Ok(Value::Void)
        })
        .method_with("notifyPaused", "()V", final_method, |ctx, _| {
            ctx.log(LogLevel::Info, "the MIDlet paused itself");
            env(ctx).host.set_status(InstanceStatus::Paused);
            Ok(Value::Void)
        })
        .method_with("resumeRequest", "()V", final_method, |ctx, _| {
            env(ctx).host.control(Control::ResumeRequest);
            Ok(Value::Void)
        })
        .method_with(
            "getAppProperty",
            "(Ljava/lang/String;)Ljava/lang/String;",
            final_method,
            |ctx, a| {
                let key = string_arg(ctx, a[1])?;
                match app_property(ctx, &key) {
                    Some(value) => Ok(ctx.new_string(&value)?.into()),
                    None => Ok(Value::NULL),
                }
            },
        )
        .method_with(
            "platformRequest",
            "(Ljava/lang/String;)Z",
            final_method,
            |ctx, a| {
                if let Some(url) = ctx.opt_string(a[1])?.filter(|u| !u.is_empty()) {
                    ctx.log(LogLevel::Info, format!("the game asked to open {url}"));
                }
                Ok(Value::from(false))
            },
        )
        .method_with(
            "checkPermission",
            "(Ljava/lang/String;)I",
            final_method,
            |ctx, a| {
                let name = string_arg(ctx, a[1])?;
                Ok(Value::Int(permission(ctx, &name)))
            },
        );
    let state_change = NativeClass::new(STATE_CHANGE)
        .extends("java/lang/Exception")
        .method("<init>", "()V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special("java/lang/Exception", this, "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special(
                "java/lang/Exception",
                this,
                "<init>",
                "(Ljava/lang/String;)V",
                &[a[1]],
            )?;
            Ok(Value::Void)
        });
    vec![midlet, state_change]
}
