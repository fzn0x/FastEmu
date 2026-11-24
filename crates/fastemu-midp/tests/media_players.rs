//! MMAPI players running Java programs through the virtual machine with an
//! offline audio engine: states and exceptions, listener events, loops,
//! media time, volume, tone sequences, `playTone`, content types, sound
//! FastEmu cannot decode, controls and hundreds of players.

mod media_support;

use media_support::{audio, check};

#[test]
fn states_and_exceptions() {
    let (_engine, group) = audio();
    check(
        "MediaStates",
        Some(group),
        r"
        created: 100
        duration unrealized: -1
        time unrealized: 0
        content type unrealized: java.lang.IllegalStateException
        seek unrealized: java.lang.IllegalStateException
        control unrealized: java.lang.IllegalStateException
        realized: 200
        content type: audio/x-wav
        duration: 200000
        realized again: 200
        prefetched: 300
        started: 400
        started again: 400
        loop count while started: java.lang.IllegalStateException
        stopped: 300
        stopped again: 300
        deallocated: 200
        started from realized: 400
        deallocated while started: 200
        loop count 0: java.lang.IllegalArgumentException
        null control: java.lang.IllegalArgumentException
        volume control: true
        same control by full name: true
        video control: null
        controls: 2
        closed: 0
        closed again: 0
        realize closed: java.lang.IllegalStateException
        prefetch closed: java.lang.IllegalStateException
        start closed: java.lang.IllegalStateException
        stop closed: java.lang.IllegalStateException
        deallocate closed: java.lang.IllegalStateException
        time closed: java.lang.IllegalStateException
        duration closed: java.lang.IllegalStateException
        loop count closed: java.lang.IllegalStateException
        listener closed: java.lang.IllegalStateException
        controls closed: java.lang.IllegalStateException
        null stream: java.lang.IllegalArgumentException
        null locator: java.lang.IllegalArgumentException
        capture locator: javax.microedition.media.MediaException
        midi device locator: javax.microedition.media.MediaException
        missing resource: java.io.IOException
        resource locator: audio/x-wav
        bare locator duration: 200000
        network locator: audio/midi
        time base runs forward: true
        player time base: true
        own time base: javax.microedition.media.MediaException
        types: audio/x-wav audio/wav audio/midi audio/mid audio/x-midi audio/sp-midi audio/mpeg audio/mp3 audio/amr audio/x-tone-seq
        device types: audio/x-tone-seq
        rtsp types: 0
        protocols: device file resource
        tone protocols: device file resource
        ",
    );
}

#[test]
fn listener_events_arrive_in_order() {
    let (_engine, group) = audio();
    let Some(output) = check(
        "MediaEvents",
        Some(group),
        r"
        state after start: 400
        events: started:0
        events: started:0 endOfMedia:200000
        state at the end: 300
        time at the end: 200000
        events: started:0 endOfMedia:200000 started:0 stopped:50000
        time after stop: 50000
        events: started:0 endOfMedia:200000 started:0 stopped:50000 started:50000
        events: started:0 endOfMedia:200000 started:0 stopped:50000 started:50000 closed:null
        names interned: true
        listener thread: media-events
        after a throwing listener: started:0
        removed listener: 0
        volume events: volumeChanged:VolumeControl volumeChanged:VolumeControl
        volume data is the control: true
        unrealized close: closed:null
        ",
    ) else {
        return;
    };
    // The throwing listener is reported and delivery goes on.
    assert!(
        output
            .err
            .iter()
            .any(|l| l.contains("Exception in PlayerListener.playerUpdate")),
        "{:?}",
        output.err
    );
    assert!(
        output
            .err
            .iter()
            .any(|l| l.contains("listener failed on started")),
        "{:?}",
        output.err
    );
}

#[test]
fn loops_media_time_and_volume() {
    let (_engine, group) = audio();
    check(
        "MediaPlayback",
        Some(group),
        r"
        three passes: started:0 endOfMedia:200000 started:0 endOfMedia:200000 started:0 endOfMedia:200000
        state after the passes: 300
        endless passes: 5
        still playing: 400
        time in the sixth pass: 50000
        stopped endless: true
        time after 100 ms: 100000
        seek: 150000
        time after seek: 150000
        time 50 ms later: 200000
        seek before the start: 0
        seek past the end: 1000000
        seek while stopped: 250000
        time while stopped: 250000
        level: 100
        set 50: 50
        set 150: 100
        set -3: 0
        set 80: 80
        heard at 80: true
        muted: true
        level kept while muted: 80
        heard while muted: false
        level 20 is quieter than 80: true
        ",
    );
}

#[test]
fn tone_sequences_and_play_tone() {
    let (_engine, group) = audio();
    check(
        "MediaTones",
        Some(group),
        r"
        tone player type: audio/x-tone-seq
        empty duration: 0
        tone control: true
        null sequence: java.lang.IllegalArgumentException
        bad tempo: java.lang.IllegalArgumentException
        duration: 500000
        heard: true
        events: started:0 endOfMedia:500000
        sequence while prefetched: java.lang.IllegalStateException
        new duration: 500000
        stream duration: 500000
        stream tone control: true
        stream controls: 3
        playTone heard: true
        note 128: java.lang.IllegalArgumentException
        note -1: java.lang.IllegalArgumentException
        duration 0: java.lang.IllegalArgumentException
        ",
    );
}

#[test]
fn wrong_content_types_and_unsupported_sound() {
    let (_engine, group) = audio();
    let Some(output) = check(
        "MediaFormats",
        Some(group),
        r"
        wav labelled midi: audio/x-wav
        its duration: 200000
        heard: true
        midi labelled wav: audio/midi
        no content type: audio/x-wav
        made-up content type: audio/x-wav
        unsupported created: 100
        unsupported type: audio/x-unknown
        unsupported duration: 1000000
        unsupported started: 400
        unsupported heard: false
        unsupported events: started:0 endOfMedia:1000000
        ended after about a second: true
        unsupported at the end: 300
        unsupported time at the end: 1000000
        unsupported closed: started:0 endOfMedia:1000000 closed:null
        unknown with no type: application/octet-stream
        broken wav: 400 audio/x-wav 1000000
        empty stream: 400
        ",
    ) else {
        return;
    };
    let warned = |text: &str| output.warn.iter().any(|l| l.contains(text));
    assert!(warned("\"audio/x-unknown\""), "{:?}", output.warn);
    assert!(warned("6e 6f 74 20 61"), "{:?}", output.warn);
    assert!(warned("WAV that does not decode"), "{:?}", output.warn);
}

#[test]
fn midi_and_stop_time_controls() {
    let (_engine, group) = audio();
    check(
        "MediaControls",
        Some(group),
        r"
        midi controls: 3
        midi control: true
        event before prefetch: java.lang.IllegalStateException
        status 0x70: java.lang.IllegalArgumentException
        system exclusive: java.lang.IllegalArgumentException
        data 128: java.lang.IllegalArgumentException
        bank query: false
        program query: javax.microedition.media.MediaException
        channel 16: java.lang.IllegalArgumentException
        volume unknown: -1
        volume set: 100
        volume 128: java.lang.IllegalArgumentException
        program 200: java.lang.IllegalArgumentException
        long event: 8
        long event past the end: java.lang.IllegalArgumentException
        midi heard: true
        wav midi control: null
        wav tone control: null
        no stop time: true
        stop time: 300000
        second stop time while started: java.lang.IllegalStateException
        stopped at time: true
        state after the stop time: 300
        near the stop time: true
        stop time cleared: true
        ",
    );
}

#[test]
fn hundreds_of_players_open_and_close() {
    let (engine, group) = audio();
    check(
        "MediaMany",
        Some(group),
        r"
        after closing 300: 0
        after a collection: 0
        playing after a collection: 5
        ends heard: 5
        after they ended: 0
        ",
    );
    assert_eq!(engine.player_count(), 0);
}

#[test]
fn without_an_engine_players_keep_time_in_silence() {
    check(
        "MediaSilent",
        None,
        r"
        type: audio/x-wav
        duration: 200000
        state after start: 400
        events: started:0 endOfMedia:200000
        took about the length: true
        state at the end: 300
        time at the end: 200000
        two passes: started:0 endOfMedia:200000 started:0 endOfMedia:200000 started:0 endOfMedia:200000
        stopped inside the media: true
        stop event: true
        seek: 120000
        volume event: true
        level: 30
        closed: true
        tone duration: 500000
        bad tempo: java.lang.IllegalArgumentException
        ",
    );
}
