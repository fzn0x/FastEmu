//! The lights and the vibration motor that every vendor's API reaches.
//!
//! The emulated phone has a backlight, other lights and a motor, so these
//! requests never fail. `fastemu-core` has no notification for them yet,
//! so they are logged at `LogLevel::Info`, as MIDP's `Display.vibrate` is.
//! Games set the backlight every frame to keep the screen lit, so a light
//! is logged only when its level changes.

use std::collections::HashMap;

use fastemu_core::LogLevel;
use fastemu_jvm::Ctx;

/// The light number of the display's backlight.
pub(crate) const BACKLIGHT: i32 = 0;

/// How many lights the state keeps a level for. A phone has one to three;
/// Nokia's `DeviceControl.setLights` takes any number a game likes.
const MAX_LIGHTS: usize = 16;

/// What the lights and the motor were last told.
#[derive(Default)]
struct DeviceState {
    /// The last level of each light, in percent, for at most
    /// [`MAX_LIGHTS`] lights. A change of any other light is logged each
    /// time.
    lights: HashMap<i32, i32>,
    /// Whether the motor runs until told to stop.
    vibrating: bool,
}

fn state<'a>(ctx: &'a mut Ctx<'_>) -> &'a mut DeviceState {
    if ctx.extension::<DeviceState>().is_none() {
        ctx.set_extension(DeviceState::default());
    }
    ctx.extension_mut::<DeviceState>()
        .expect("the device state was installed above")
}

/// Sets light `number` ([`BACKLIGHT`] is the display) to `percent`,
/// clamped to 0 to 100, and logs the change.
pub(crate) fn set_light(ctx: &mut Ctx<'_>, number: i32, percent: i32) {
    let percent = percent.clamp(0, 100);
    let lights = &mut state(ctx).lights;
    if lights.get(&number) == Some(&percent) {
        return;
    }
    if lights.len() < MAX_LIGHTS || lights.contains_key(&number) {
        lights.insert(number, percent);
    }
    let text = if number == BACKLIGHT {
        format!("backlight at {percent}%")
    } else {
        format!("light {number} at {percent}%")
    };
    ctx.log(LogLevel::Info, text);
}

/// Flashes the lights for `duration_ms`.
pub(crate) fn flash_lights(ctx: &mut Ctx<'_>, duration_ms: i64) {
    ctx.log(
        LogLevel::Info,
        format!("flash the lights for {} ms", duration_ms.max(0)),
    );
}

/// Runs the motor for `duration_ms`, or until [`stop_vibrating`] when
/// `None`.
pub(crate) fn vibrate(ctx: &mut Ctx<'_>, duration_ms: Option<i64>) {
    let text = match duration_ms {
        Some(ms) => format!("vibrate for {} ms", ms.max(0)),
        None => "vibrate until stopped".to_owned(),
    };
    state(ctx).vibrating = duration_ms.is_none();
    ctx.log(LogLevel::Info, text);
}

/// Stops the motor; logged only when it ran without an end.
pub(crate) fn stop_vibrating(ctx: &mut Ctx<'_>) {
    if std::mem::take(&mut state(ctx).vibrating) {
        ctx.log(LogLevel::Info, "vibration off");
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use fastemu_core::LogLine;
    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;

    fn logged(f: impl FnOnce(&mut Ctx<'_>) + Send) -> Vec<String> {
        let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
        let sink = Arc::clone(&lines);
        let config = VmConfig {
            log: Arc::new(move |line| sink.lock().unwrap().push(line)),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(MemorySource::new()), Vec::new());
        vm.attach("test", f);
        let lines = lines.lock().unwrap();
        lines.iter().map(|l| l.text.clone()).collect()
    }

    #[test]
    fn lights_log_only_their_changes() {
        let lines = logged(|ctx| {
            set_light(ctx, BACKLIGHT, 100);
            set_light(ctx, BACKLIGHT, 100);
            set_light(ctx, BACKLIGHT, 250);
            set_light(ctx, 1, -3);
            set_light(ctx, BACKLIGHT, 0);
        });
        assert_eq!(
            lines,
            ["backlight at 100%", "light 1 at 0%", "backlight at 0%"]
        );
    }

    #[test]
    fn a_game_that_names_many_lights_makes_the_state_keep_16() {
        let (lines, kept) = {
            let kept = Arc::new(Mutex::new(0));
            let seen = Arc::clone(&kept);
            let lines = logged(move |ctx| {
                for number in 0..1000 {
                    set_light(ctx, number, 50);
                }
                // One that is kept still logs only its changes.
                set_light(ctx, 3, 50);
                set_light(ctx, 3, 60);
                *seen.lock().unwrap() = state(ctx).lights.len();
            });
            let kept = *kept.lock().unwrap();
            (lines, kept)
        };
        assert_eq!(kept, MAX_LIGHTS);
        assert_eq!(lines.len(), 1000 + 1);
        assert_eq!(lines.last().map(String::as_str), Some("light 3 at 60%"));
    }

    #[test]
    fn only_an_endless_vibration_logs_its_stop() {
        let lines = logged(|ctx| {
            vibrate(ctx, Some(200));
            stop_vibrating(ctx);
            vibrate(ctx, None);
            stop_vibrating(ctx);
            stop_vibrating(ctx);
            flash_lights(ctx, -1);
        });
        assert_eq!(
            lines,
            [
                "vibrate for 200 ms",
                "vibrate until stopped",
                "vibration off",
                "flash the lights for 0 ms"
            ]
        );
    }
}
