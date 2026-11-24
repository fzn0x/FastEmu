//! Siemens' APIs through Java programs on a Siemens profile: the game
//! package (device calls, melodies, bitmaps, `ExtendedImage`, sprites and
//! tiled backgrounds), the colour game package, and a `GameCanvas`
//! MIDlet through the real launcher.

mod support;
mod vendor_support;

use fastemu_core::Key;
use support::{Running, rgb};
use vendor_support::{check, profile, suite};

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

#[test]
fn the_color_game_layers_behave_as_midps() {
    check(
        "SiemensColor",
        &profile(SIEMENS),
        r"
        frames: 2 sequence 2 size 2x2
        frame 0: ....../.##.../.##...
        frame 1: ....../.#..../.#.... index 1
        sequence: 3 at 2
        sequence frame: ....../.##.../.##...
        frame 3: java.lang.IndexOutOfBoundsException
        raw frame 2: java.lang.ArrayIndexOutOfBoundsException
        empty sequence: java.lang.IllegalArgumentException
        width 3: java.lang.IllegalArgumentException
        copy: 1,1 sequence 3 at 2
        default sequence: 2 at 0
        rectangles: true false
        pixels: rectangle true pixel false
        opaque corner: true
        image: true false false
        small rectangle: false
        hidden: false false
        tiled: 3x2 cells 2x2 size 6x4
        cells: ##rr##/##rr##/rr####/rr####
        animated: ##rr../##rr../rr####/rr#### cell -1 shows 2
        cell 3,0: java.lang.IndexOutOfBoundsException
        tile 3: java.lang.IndexOutOfBoundsException
        animated -2: java.lang.IndexOutOfBoundsException
        no columns: java.lang.IllegalArgumentException
        layers: 2 top true
        managed: ####../####../..####/..####
        view: .##.../.##.../.##.../.##... restored 0 6
        own layer: ####../####../..####/..###r
        removed: 2
        layer 2: java.lang.IndexOutOfBoundsException
        fewer tiles: 0
        ",
    );
}

#[test]
fn a_siemens_game_canvas_flushes_and_reads_its_keys() {
    let Some(jar) = suite("siemens-canvas", &["SiemensCanvas"]) else {
        return;
    };
    let mut game = Running::launch_on(&jar, 0, profile(SIEMENS));
    game.wait_log("flushed 132x176");
    let frame = game.wait_frame(|f| f.pixel(131, 175).is_some_and(|p| p & 0xFF_FFFF == 0xFF));
    assert_eq!(rgb(&frame, 0, 0), 0x00_00FF);
    game.tap(Key::Fire);
    game.wait_log("fire pressed");
    let frame = game.wait_frame(|f| f.pixel(0, 0).is_some_and(|p| p & 0xFF_FFFF == 0xFF_0000));
    assert_eq!(rgb(&frame, 20, 20), 0x00_00FF);
}
