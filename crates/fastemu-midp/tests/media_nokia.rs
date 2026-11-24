//! Nokia's `com.nokia.mid.sound.Sound` running a Java program through the
//! virtual machine with an offline audio engine: ringing tones, beeps,
//! WAV, its states, gain and listener.

mod media_support;

use media_support::{audio, check};

#[test]
fn nokia_sounds_play_and_report_their_state() {
    let (engine, group) = audio();
    check(
        "NokiaSounds",
        Some(group),
        r"
        ring created: 1
        default gain: 255
        ring playing: 0
        heard: 0
        heard: 0 1
        ring after the end: 1
        heard: 0 1 0 1
        endless still playing: 0
        restart: 0 1 0 1 0 1 0
        heard: 0 1 0 1 0 1 0 1
        ring stopped: 1
        ring resumed: 0
        gain 300: 255
        gain -5: 0
        gain 128: 128
        play -1: java.lang.IllegalArgumentException
        released: 3
        heard: 0 1 0 1 0 1 0 1 0 1 3
        released does not play: 3
        beep created: 1
        beep heard: true
        beep after the end: 1
        re-initialized as wav: 1
        wav heard: true
        midi given as wav: 1
        junk: 3
        junk after play: 3
        null data: java.lang.NullPointerException
        format 99: java.lang.IllegalArgumentException
        duration 0: java.lang.IllegalArgumentException
        frequency 30000: java.lang.IllegalArgumentException
        formats: 1 5
        concurrent tones: 1
        concurrent 7: java.lang.IllegalArgumentException
        ",
    );
    drop(engine);
}
