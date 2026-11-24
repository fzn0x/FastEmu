//! `com.nokia.mid.ui.DeviceControl`: the backlight and the vibration
//! motor.
//!
//! The javadoc lets these throw for lights a phone lacks, levels outside 0
//! to 100 and phones without a motor. The emulated phone has every light
//! and a motor, and Nokia models differed in which lights they had, so
//! FastEmu clamps the values and never throws (as freej2me-plus, which
//! ignores the calls). Requests are logged by `vendor::device`.

use fastemu_jvm::{NativeClass, Value};

use crate::vendor::device;

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new("com/nokia/mid/ui/DeviceControl")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("setLights", "(II)V", |ctx, a| {
            device::set_light(ctx, a[0].int(), a[1].int());
            Ok(Value::Void)
        })
        .static_method("flashLights", "(J)V", |ctx, a| {
            device::flash_lights(ctx, a[0].long());
            Ok(Value::Void)
        })
        .static_method("startVibra", "(IJ)V", |ctx, a| {
            // Frequency 0 asks for no vibration at all.
            if a[0].int() > 0 {
                device::vibrate(ctx, Some(a[1].long()));
            }
            Ok(Value::Void)
        })
        .static_method("stopVibra", "()V", |ctx, _| {
            device::stop_vibrating(ctx);
            Ok(Value::Void)
        })
}
