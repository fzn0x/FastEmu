//! `Manager.getSystemTimeBase`: a `TimeBase` that counts microseconds on
//! the host's monotonic clock. Players keep it; no other time base is
//! accepted.

use std::sync::OnceLock;
use std::time::Instant;

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::api::TIME_BASE;

const SYSTEM_TIME_BASE: &str = "javax/microedition/media/SystemTimeBase";

/// The microseconds since FastEmu first read the clock.
fn now_us() -> i64 {
    static START: OnceLock<Instant> = OnceLock::new();
    let start = *START.get_or_init(Instant::now);
    i64::try_from(start.elapsed().as_micros()).unwrap_or(i64::MAX)
}

/// A system time base object.
pub(crate) fn system(ctx: &mut Ctx<'_>) -> JResult<ObjRef> {
    let class = ctx.find_class(SYSTEM_TIME_BASE)?;
    ctx.new_object(class)
}

/// Whether `base` is the system time base.
pub(crate) fn is_system(ctx: &mut Ctx<'_>, base: ObjRef) -> JResult<bool> {
    let class = ctx.find_class(SYSTEM_TIME_BASE)?;
    Ok(ctx.is_instance(base, class))
}

pub(crate) fn class() -> NativeClass {
    NativeClass::new(SYSTEM_TIME_BASE)
        .implements(TIME_BASE)
        .final_class()
        .method("getTime", "()J", |_, _| Ok(Value::Long(now_us())))
}
