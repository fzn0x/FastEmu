//! `javax.microedition.io.PushRegistry`.
//!
//! A phone could start a MIDlet when a message or connection arrived, or
//! at a set time. FastEmu starts games only when the user does, so it
//! checks a registration's arguments as the javadoc asks, logs it, and
//! keeps nothing: no connection is ever listed and no alarm is ever set,
//! so `registerAlarm` always reports that none was set before (0).

use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, JResult, NativeClass, Value};

use super::url;
use crate::env;

const PUSH_REGISTRY: &str = "javax/microedition/io/PushRegistry";

/// Throws `ClassNotFoundException` unless `midlet` names one of the
/// suite's MIDlets. A suite that lists none (a test VM) accepts any name.
fn check_midlet(ctx: &mut Ctx<'_>, midlet: Value) -> JResult<String> {
    let Some(name) = ctx.opt_string(midlet)? else {
        return ctx.throw(
            "java/lang/ClassNotFoundException",
            "the MIDlet name is null",
        );
    };
    let listed = env::try_env(ctx).is_none_or(|e| {
        e.suite.midlets.is_empty() || e.suite.midlets.iter().any(|m| m.class_name == name)
    });
    if !listed {
        let message = format!("{name} is not a MIDlet of this suite");
        return ctx.throw("java/lang/ClassNotFoundException", &message);
    }
    Ok(name)
}

/// The `PushRegistry` class.
pub(crate) fn class() -> NativeClass {
    NativeClass::new(PUSH_REGISTRY)
        .static_method(
            "registerConnection",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
            |ctx, a| {
                let connection = ctx.opt_string(a[0])?;
                let valid = connection
                    .as_deref()
                    .is_some_and(|c| url::split_scheme(c).is_ok());
                if !valid {
                    let message = format!("invalid connection {}", connection.as_deref().unwrap_or("null"));
                    return ctx.throw("java/lang/IllegalArgumentException", &message);
                }
                if a[2].is_null() {
                    return ctx.throw("java/lang/IllegalArgumentException", "the filter is null");
                }
                let midlet = check_midlet(ctx, a[1])?;
                let connection = connection.unwrap_or_default();
                ctx.log(
                    LogLevel::Warn,
                    format!("push registration of {connection} for {midlet} ignored: FastEmu starts games only when asked"),
                );
                Ok(Value::Void)
            },
        )
        .static_method(
            "unregisterConnection",
            "(Ljava/lang/String;)Z",
            |_, _| Ok(Value::from(false)),
        )
        .static_method("listConnections", "(Z)[Ljava/lang/String;", |ctx, _| {
            let string = ctx.find_class("java/lang/String")?;
            Ok(ctx.new_object_array(string, 0)?.into())
        })
        .static_method(
            "getMIDlet",
            "(Ljava/lang/String;)Ljava/lang/String;",
            |_, _| Ok(Value::Ref(None)),
        )
        .static_method(
            "getFilter",
            "(Ljava/lang/String;)Ljava/lang/String;",
            |_, _| Ok(Value::Ref(None)),
        )
        .static_method("registerAlarm", "(Ljava/lang/String;J)J", |ctx, a| {
            let midlet = check_midlet(ctx, a[0])?;
            ctx.log(
                LogLevel::Warn,
                format!("alarm for {midlet} ignored: FastEmu starts games only when asked"),
            );
            Ok(Value::Long(0))
        })
}
