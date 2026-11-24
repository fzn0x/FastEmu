//! Samsung's, LG's and Motorola's APIs through Java programs on profiles
//! of those makers.

mod support;
mod vendor_support;

use fastemu_core::{DeviceProfile, default_profile};
use vendor_support::{audio, check, check_with, profile};

/// A profile of a maker FastEmu has no built-in profile for.
fn made_by(id: &str) -> DeviceProfile {
    DeviceProfile {
        id: id.to_owned(),
        ..default_profile()
    }
}

/// What `SamsungUtil` prints, with or without sound.
const SAMSUNG: &str = r"
        supported: true true true false
        clip played
        volume 6: java.lang.IllegalArgumentException
        loop 256: java.lang.IllegalArgumentException
        mmf played
        bad range: java.lang.ArrayIndexOutOfBoundsException
        null data: java.lang.NullPointerException
        missing file: java.io.IOException
        strength 6: java.lang.IllegalArgumentException
        message: +12345 555 hello
        letters in the number: java.lang.IllegalArgumentException
        81 characters: java.lang.IllegalArgumentException
        send: java.lang.IllegalStateException
        ";

#[test]
fn samsung_clips_lights_and_messages() {
    let Some(output) = check("SamsungUtil", &made_by("samsung-176x220"), SAMSUNG) else {
        return;
    };
    assert_eq!(
        output.info,
        [
            "backlight at 100%",
            "backlight at 0%",
            "vibrate for 300 ms",
            "a text message was not sent"
        ]
    );
}

#[test]
fn samsung_clips_play_through_the_sound_engine() {
    let (engine, group) = audio();
    let Some(output) = check_with(
        "SamsungUtil",
        &made_by("samsung-176x220"),
        Some(group),
        SAMSUNG,
    ) else {
        return;
    };
    // Only the MMF clip, which FastEmu cannot play, stays silent.
    assert_eq!(output.warn, ["an MMF sound clip stays silent"]);
    drop(engine);
}

#[test]
fn lg_backlight_and_vibration() {
    let Some(output) = check(
        "LgMedia",
        &made_by("lg-176x220"),
        r"
        backlight: 123456 of 16777216
        levels: 5
        ",
    ) else {
        return;
    };
    assert_eq!(
        output.info,
        [
            "backlight at 100%",
            "backlight colour #123456",
            "backlight at 0%",
            "vibrate for 250 ms"
        ]
    );
}

#[test]
fn motorola_backlight() {
    let Some(output) = check("MotorolaLighting", &profile("motorola-176x220"), "") else {
        return;
    };
    assert_eq!(output.info, ["backlight at 100%", "backlight at 0%"]);
}
