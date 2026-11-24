//! Siemens' APIs through Java programs on a Siemens profile: the game
//! package (device calls, melodies, bitmaps, `ExtendedImage`, sprites and
//! tiled backgrounds).

mod support;
mod vendor_support;

use vendor_support::{check, profile};

const SIEMENS: &str = "siemens-132x176";

#[test]
fn the_game_package_draws_bitmaps_sprites_and_tiles() {
    let Some(output) = check(
        "SiemensGame",
        &profile(SIEMENS),
        r"
        composer: 2 of 32767
        composed: 2
        reset: 0
        bitmap: #.#...../....####
        mirrored: .....#.#/####....
        flipped: ####..../.....#.#
        two bits: t.##
        rgb: rt
        ui image: 16x8 native 8
        rgb type 3: java.io.IOException
        same image: true
        pixels: 1 0 0
        extended: .#....../........
        set pixels: ####..../.......#
        pixel bytes: f0 1
        cleared: ########/########
        width 7: java.lang.IllegalArgumentException
        x 3: java.lang.IllegalArgumentException
        two-bit: 0120 copied true
        two-bit set: #.####.t
        sprite: ............/..####....../............
        frame 1 offset: ............/.......####./............ at 2,1 frame 1
        hidden: ............/............/............ false
        collide: true false pos true false
        small rectangle: true
        beside it: false
        image sprite: ##rrrrrr/##rrrrrr
        three frames: java.lang.IllegalArgumentException
        transparent pixels: java.lang.IllegalArgumentException
        short bits: java.lang.IllegalArgumentException
        tiles: #.#.r#.#
        scrolled: ###r.###
        map changed: ###.r###
        tile 9: java.lang.IllegalArgumentException
        odd tile bytes: java.lang.IllegalArgumentException
        positions: 0 1 2 first true
        after deleting: 0 -1
        object 5: java.lang.ArrayIndexOutOfBoundsException
        paint on transparent: java.lang.IllegalArgumentException
        ",
    ) else {
        return;
    };
    assert_eq!(
        output.info,
        [
            "backlight at 100%",
            "backlight at 0%",
            "vibrate for 150 ms",
            "vibrate until stopped",
            "vibration off"
        ]
    );
}
