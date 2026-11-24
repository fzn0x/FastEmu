//! Siemens' game APIs, on Siemens profiles only: `com.siemens.mp.game`
//! (lights, vibration, sound and melodies, black and white bitmaps,
//! sprites and tiled backgrounds), `com.siemens.mp.color_game` (Siemens'
//! copy of MIDP 2.0's game package for its MIDP 1.0 colour phones) and
//! `com.siemens.mp.ui.Image`. Signatures and constants come from
//! `docs/compat/api/vendors.md`.

mod bitmap;
mod color_game;
mod extended_image;
mod graphic_object;
mod sound;

use fastemu_jvm::{NativeClass, Value};

use super::device;

/// `com.siemens.mp.misc.NativeMem`, the empty superclass of many Siemens
/// classes.
const NATIVE_MEM: &str = "com/siemens/mp/misc/NativeMem";

/// The classes of Siemens' APIs.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = vec![
        NativeClass::new(NATIVE_MEM).method("<init>", "()V", |_, _| Ok(Value::Void)),
        light(),
        vibrator(),
        extended_image::class(),
        bitmap::class(),
    ];
    classes.extend(sound::classes());
    classes.extend(graphic_object::classes());
    classes.extend(color_game::classes());
    classes
}

/// `com.siemens.mp.game.Light`: the backlight on and off.
fn light() -> NativeClass {
    NativeClass::new("com/siemens/mp/game/Light")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("setLightOn", "()V", |ctx, _| {
            device::set_light(ctx, device::BACKLIGHT, 100);
            Ok(Value::Void)
        })
        .static_method("setLightOff", "()V", |ctx, _| {
            device::set_light(ctx, device::BACKLIGHT, 0);
            Ok(Value::Void)
        })
}

/// `com.siemens.mp.game.Vibrator`: the motor for a while, or until
/// stopped.
fn vibrator() -> NativeClass {
    NativeClass::new("com/siemens/mp/game/Vibrator")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .static_method("triggerVibrator", "(I)V", |ctx, a| {
            device::vibrate(ctx, Some(a[0].int().into()));
            Ok(Value::Void)
        })
        .static_method("startVibrator", "()V", |ctx, _| {
            device::vibrate(ctx, None);
            Ok(Value::Void)
        })
        .static_method("stopVibrator", "()V", |ctx, _| {
            device::stop_vibrating(ctx);
            Ok(Value::Void)
        })
}
