//! Motorola's `com.motorola.multimedia.Lighting`, on Motorola profiles
//! only: the backlight on and off.

use fastemu_jvm::{NativeClass, Value};

use super::device;

/// The classes of Motorola's API.
pub fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("com/motorola/multimedia/Lighting")
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .static_method("backlightOn", "()V", |ctx, _| {
                device::set_light(ctx, device::BACKLIGHT, 100);
                Ok(Value::Void)
            })
            .static_method("backlightOff", "()V", |ctx, _| {
                device::set_light(ctx, device::BACKLIGHT, 0);
                Ok(Value::Void)
            }),
    ]
}
