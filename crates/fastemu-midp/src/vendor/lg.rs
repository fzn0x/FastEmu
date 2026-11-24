//! LG's `mmpp.media` backlight and vibration, on LG profiles only. The
//! values follow freej2me-plus: the backlight takes any RGB colour and
//! vibration has five levels.

use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, NativeClass, Value};

use super::device;

/// The backlight colour a game set, white until it sets one.
struct BacklightColor(i32);

fn color(ctx: &Ctx<'_>) -> i32 {
    ctx.extension::<BacklightColor>().map_or(0xFF_FFFF, |c| c.0)
}

fn back_light() -> NativeClass {
    NativeClass::new("mmpp/media/BackLight")
        .final_class()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("on", "(I)V", |ctx, _| {
            device::set_light(ctx, device::BACKLIGHT, 100);
            Ok(Value::Void)
        })
        .static_method("off", "()V", |ctx, _| {
            device::set_light(ctx, device::BACKLIGHT, 0);
            Ok(Value::Void)
        })
        .static_method("numColors", "()I", |_, _| Ok(Value::Int(1 << 24)))
        .static_method("setColor", "(I)V", |ctx, a| {
            let rgb = a[0].int() & 0xFF_FFFF;
            if color(ctx) != rgb {
                ctx.log(LogLevel::Info, format!("backlight colour #{rgb:06x}"));
            }
            ctx.set_extension(BacklightColor(rgb));
            Ok(Value::Void)
        })
        .static_method("getColor", "()I", |ctx, _| Ok(Value::Int(color(ctx))))
}

fn vibration() -> NativeClass {
    NativeClass::new("mmpp/media/Vibration")
        .final_class()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("getLevelNum", "()I", |_, _| Ok(Value::Int(5)))
        .static_method("start", "(II)V", |ctx, a| {
            device::vibrate(ctx, Some(a[1].int().into()));
            Ok(Value::Void)
        })
        .static_method("stop", "()V", |ctx, _| {
            device::stop_vibrating(ctx);
            Ok(Value::Void)
        })
}

/// The classes of LG's API.
pub fn classes() -> Vec<NativeClass> {
    vec![back_light(), vibration()]
}
