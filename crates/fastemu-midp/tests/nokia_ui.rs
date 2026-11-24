//! Nokia's UI API through Java programs: `DirectGraphics` pixel by pixel
//! in every format, `DirectUtils`' images, and a `FullCanvas` MIDlet's
//! size, key codes and device requests through the real launcher.

mod support;
mod vendor_support;

use fastemu_core::{Key, default_profile};
use support::{Running, rgb};
use vendor_support::{check, profile, suite};

#[test]
fn direct_graphics_draws_and_reads_every_format() {
    check(
        "NokiaGraphics",
        &default_profile(),
        r"
        same object: true
        cast: true
        native format: 565
        transparent: 0 0 0 0 mutable true
        translucent: 40ff0000
        8888 alpha: ffff7f7f ffffffff ff0000ff ffffffff
        8888 opaque: ffff0000 ff00ff00 ff0000ff ff000000
        888: ff345678 ff000000 ffffffff ffabcdef
        4444 alpha: ff880000 ffffffff ffff7777 ff00ff00
        4444 opaque: ff880000 ffffffff ffff0000 ff00ff00
        444: ffff0000 ff00ff00 ff0000ff ff112233
        565: ffff0000 ff00ff00 ff0000ff ff848284
        555: ffff0000 ff00ff00 ff0000ff ff848484
        1555 alpha: ffff0000 ffffffff ff00ff00 ffffffff
        1 gray: #.#..#.# ....####
        1 gray masked: #.#.rrrr ....####
        1 gray vertical: #. .. .. .. .. .. .. .#
        2 gray: ff000000 ff555555 ffaaaaaa ffffffff
        4 gray: ff000000 ffffffff ff777777 ff888888
        8 gray: ff000000 ff808080 ffffffff ff404040
        332: ffff0000 ff00ff00 ff0000ff ff9292aa
        rotate 90: ff0000ff ffffffff ffff0000 ffffffff
        flip: ff0000ff ffff0000
        translated: .. .#
        get 8888: ffff0000 ff00ff00 ff0000ff 7fffffff
        get 888: ff0000 ff00 ff ffffff
        get 4444: ff00 f0f0 f00f 7fff
        get 565: f800 7e0 1f ffff
        get 444: f00 f0 f fff
        get 555: 7c00 3e0 1f 7fff
        get 1555: fc00 83e0 801f 7fff
        get 8 gray: 4c 96 1d 0
        get 332: e0 1c 3 0
        get 1 gray: a0 mask e0
        alpha 128 colour 0
        blended fill: ff7f7f7f
        setColor alpha: 255
        fill triangle: #### ###. ##.. #...
        draw triangle: #### #.#. ##.. #...
        fill polygon: ####. ####. ####. ####. .....
        draw polygon: ####. #..#. #..#. ####. .....
        image rotate 90: ff0000ff ffffffff ffff0000 ffffffff
        image flips: ffffffff ffffffff ff0000ff ffff0000
        decoded: ffff0000 ff00ff00 ff0000ff 80ffffff mutable true
        drawn on: ff000000 ff00ff00 ff0000ff 80ffffff
        size 0: java.lang.IllegalArgumentException
        junk bytes: java.lang.IllegalArgumentException
        bad range: java.lang.ArrayIndexOutOfBoundsException
        null bytes: java.lang.NullPointerException
        int pixels as 565: java.lang.IllegalArgumentException
        short array too small: java.lang.ArrayIndexOutOfBoundsException
        manipulation 45: java.lang.IllegalArgumentException
        null pixels: java.lang.NullPointerException
        negative getPixels: java.lang.IllegalArgumentException
        null polygon: java.lang.NullPointerException
        short polygon: java.lang.ArrayIndexOutOfBoundsException
        baseline image: java.lang.IllegalArgumentException
        null image: java.lang.NullPointerException
        ",
    );
}

/// Starts the `NokiaKeys` MIDlet on `profile_id` and waits until it has
/// painted.
fn full_canvas(profile_id: &str) -> Option<Running> {
    let jar = suite("nokia-keys", &["NokiaKeys"])?;
    let mut game = Running::launch_on(&jar, 0, profile(profile_id));
    game.wait_log("painted");
    Some(game)
}

#[test]
fn a_full_canvas_fills_the_screen_and_has_no_commands() {
    let Some(mut game) = full_canvas("generic-240x320") else {
        return;
    };
    game.wait_log("shown 240x320");
    assert!(game.has("addCommand: IllegalStateException"));
    assert!(game.has("setCommandListener: IllegalStateException"));
    // No soft key bar: the green reaches the bottom corners.
    let frame = game.wait_frame(|f| f.pixel(0, 319).is_some_and(|p| p & 0xFF_FFFF == 0xFF00));
    assert_eq!(rgb(&frame, 239, 319), 0x00FF00);
}

#[test]
fn full_canvas_soft_keys_reach_the_game_as_nokia_codes() {
    for id in [
        "generic-240x320",
        "sonyericsson-176x220",
        "nokia-s40-128x160",
    ] {
        let Some(mut game) = full_canvas(id) else {
            return;
        };
        for (key, line) in [
            (Key::SoftLeft, "pressed -6 action 0"),
            (Key::SoftRight, "pressed -7 action 0"),
            (Key::Fire, "pressed -5 action 8"),
            (Key::Up, "pressed -1 action 1"),
            (Key::Right, "pressed -4 action 5"),
            (Key::End, "pressed -11 action 0"),
        ] {
            game.tap(key);
            game.wait_log(line);
        }
        game.wait_log("released -11");
    }
}

#[test]
fn device_control_logs_lights_and_vibration() {
    let Some(mut game) = full_canvas("generic-240x320") else {
        return;
    };
    game.tap(Key::Num1);
    game.wait_log("light 7 at 100%");
    let lines = game.lines();
    let device: Vec<&str> = lines
        .iter()
        .map(String::as_str)
        .filter(|l| l.contains("light") || l.contains("vibrat"))
        .collect();
    assert_eq!(
        device,
        ["backlight at 100%", "vibrate for 300 ms", "light 7 at 100%"]
    );
}
