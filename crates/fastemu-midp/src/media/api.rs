//! The interfaces, constants and exception of MMAPI that carry no code of
//! their own: `Control`, `Controllable`, `Player`, `PlayerListener`,
//! `TimeBase`, `MediaException` and every interface of
//! `javax.microedition.media.control`.
//!
//! Every control interface the inventory lists is declared, even those
//! FastEmu never hands out, so a game that names one in a field, a cast or
//! an `instanceof` still links. Members and constant values follow
//! `docs/compat/api/midp-media.md`.

use fastemu_jvm::{NativeClass, Value};

pub(crate) const CONTROL: &str = "javax/microedition/media/Control";
pub(crate) const CONTROLLABLE: &str = "javax/microedition/media/Controllable";
pub(crate) const PLAYER: &str = "javax/microedition/media/Player";
pub(crate) const PLAYER_LISTENER: &str = "javax/microedition/media/PlayerListener";
pub(crate) const TIME_BASE: &str = "javax/microedition/media/TimeBase";
pub(crate) const MEDIA_EXCEPTION: &str = "javax/microedition/media/MediaException";

pub(crate) const VOLUME_CONTROL: &str = "javax/microedition/media/control/VolumeControl";
pub(crate) const TONE_CONTROL: &str = "javax/microedition/media/control/ToneControl";
pub(crate) const MIDI_CONTROL: &str = "javax/microedition/media/control/MIDIControl";
pub(crate) const STOP_TIME_CONTROL: &str = "javax/microedition/media/control/StopTimeControl";

/// The descriptor of `PlayerListener.playerUpdate`.
pub(crate) const PLAYER_UPDATE: &str =
    "(Ljavax/microedition/media/Player;Ljava/lang/String;Ljava/lang/Object;)V";

/// `Player.TIME_UNKNOWN`.
pub(crate) const TIME_UNKNOWN: i64 = -1;
/// `StopTimeControl.RESET`.
pub(crate) const STOP_TIME_RESET: i64 = i64::MAX;

/// The names `PlayerListener` events carry, as its constants spell them.
pub(crate) mod event_name {
    pub const STARTED: &str = "started";
    pub const STOPPED: &str = "stopped";
    pub const END_OF_MEDIA: &str = "endOfMedia";
    pub const VOLUME_CHANGED: &str = "volumeChanged";
    pub const ERROR: &str = "error";
    pub const CLOSED: &str = "closed";
    pub const STOPPED_AT_TIME: &str = "stoppedAtTime";
}

/// Every class of this file.
pub(crate) fn classes() -> Vec<NativeClass> {
    let mut classes = vec![
        NativeClass::interface(CONTROL),
        NativeClass::interface(CONTROLLABLE)
            .abstract_method("getControls", "()[Ljavax/microedition/media/Control;")
            .abstract_method(
                "getControl",
                "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
            ),
        player(),
        player_listener(),
        NativeClass::interface(TIME_BASE).abstract_method("getTime", "()J"),
        media_exception(),
    ];
    classes.extend(controls());
    classes
}

fn player() -> NativeClass {
    let mut class = NativeClass::interface(PLAYER)
        .implements(CONTROLLABLE)
        .constant("UNREALIZED", "I", Value::Int(100))
        .constant("REALIZED", "I", Value::Int(200))
        .constant("PREFETCHED", "I", Value::Int(300))
        .constant("STARTED", "I", Value::Int(400))
        .constant("CLOSED", "I", Value::Int(0))
        .constant("TIME_UNKNOWN", "J", Value::Long(TIME_UNKNOWN));
    for (name, desc) in [
        ("realize", "()V"),
        ("prefetch", "()V"),
        ("start", "()V"),
        ("stop", "()V"),
        ("deallocate", "()V"),
        ("close", "()V"),
        ("setTimeBase", "(Ljavax/microedition/media/TimeBase;)V"),
        ("getTimeBase", "()Ljavax/microedition/media/TimeBase;"),
        ("setMediaTime", "(J)J"),
        ("getMediaTime", "()J"),
        ("getState", "()I"),
        ("getDuration", "()J"),
        ("getContentType", "()Ljava/lang/String;"),
        ("setLoopCount", "(I)V"),
        (
            "addPlayerListener",
            "(Ljavax/microedition/media/PlayerListener;)V",
        ),
        (
            "removePlayerListener",
            "(Ljavax/microedition/media/PlayerListener;)V",
        ),
    ] {
        class = class.abstract_method(name, desc);
    }
    class
}

fn player_listener() -> NativeClass {
    [
        ("STARTED", event_name::STARTED),
        ("STOPPED", event_name::STOPPED),
        ("END_OF_MEDIA", event_name::END_OF_MEDIA),
        ("DURATION_UPDATED", "durationUpdated"),
        ("DEVICE_UNAVAILABLE", "deviceUnavailable"),
        ("DEVICE_AVAILABLE", "deviceAvailable"),
        ("VOLUME_CHANGED", event_name::VOLUME_CHANGED),
        ("ERROR", event_name::ERROR),
        ("CLOSED", event_name::CLOSED),
        ("STOPPED_AT_TIME", event_name::STOPPED_AT_TIME),
        ("SIZE_CHANGED", "sizeChanged"),
        ("RECORD_STARTED", "recordStarted"),
        ("RECORD_STOPPED", "recordStopped"),
        ("RECORD_ERROR", "recordError"),
        ("BUFFERING_STARTED", "bufferingStarted"),
        ("BUFFERING_STOPPED", "bufferingStopped"),
    ]
    .into_iter()
    .fold(
        NativeClass::interface(PLAYER_LISTENER),
        |class, (name, text)| class.string_constant(name, text),
    )
    .abstract_method("playerUpdate", PLAYER_UPDATE)
}

/// `MediaException`, whose constructors pass the message to `Exception`.
fn media_exception() -> NativeClass {
    NativeClass::new(MEDIA_EXCEPTION)
        .extends("java/lang/Exception")
        .method("<init>", "()V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special("java/lang/Exception", this, "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special(
                "java/lang/Exception",
                this,
                "<init>",
                "(Ljava/lang/String;)V",
                &[a[1]],
            )?;
            Ok(Value::Void)
        })
}

/// An interface of `javax.microedition.media.control` with abstract
/// methods.
fn control(name: &str, extends: &str, methods: &[(&str, &str)]) -> NativeClass {
    let class = NativeClass::interface(&format!("javax/microedition/media/control/{name}"))
        .implements(extends);
    methods.iter().fold(class, |class, (name, desc)| {
        class.abstract_method(name, desc)
    })
}

fn controls() -> Vec<NativeClass> {
    let gui = "javax/microedition/media/control/GUIControl";
    let rate = "javax/microedition/media/control/RateControl";
    vec![
        control(
            "VolumeControl",
            CONTROL,
            &[
                ("setMute", "(Z)V"),
                ("isMuted", "()Z"),
                ("setLevel", "(I)I"),
                ("getLevel", "()I"),
            ],
        ),
        tone_control(),
        midi_control(),
        control(
            "StopTimeControl",
            CONTROL,
            &[("setStopTime", "(J)V"), ("getStopTime", "()J")],
        )
        .constant("RESET", "J", Value::Long(STOP_TIME_RESET)),
        control(
            "FramePositioningControl",
            CONTROL,
            &[
                ("seek", "(I)I"),
                ("skip", "(I)I"),
                ("mapFrameToTime", "(I)J"),
                ("mapTimeToFrame", "(J)I"),
            ],
        ),
        control(
            "GUIControl",
            CONTROL,
            &[("initDisplayMode", "(ILjava/lang/Object;)Ljava/lang/Object;")],
        )
        .constant("USE_GUI_PRIMITIVE", "I", Value::Int(0)),
        control(
            "VideoControl",
            gui,
            &[
                ("setDisplayLocation", "(II)V"),
                ("getDisplayX", "()I"),
                ("getDisplayY", "()I"),
                ("setVisible", "(Z)V"),
                ("setDisplaySize", "(II)V"),
                ("setDisplayFullScreen", "(Z)V"),
                ("getSourceWidth", "()I"),
                ("getSourceHeight", "()I"),
                ("getDisplayWidth", "()I"),
                ("getDisplayHeight", "()I"),
                ("getSnapshot", "(Ljava/lang/String;)[B"),
            ],
        )
        .constant("USE_DIRECT_VIDEO", "I", Value::Int(1)),
        control(
            "MetaDataControl",
            CONTROL,
            &[
                ("getKeys", "()[Ljava/lang/String;"),
                ("getKeyValue", "(Ljava/lang/String;)Ljava/lang/String;"),
            ],
        )
        .string_constant("AUTHOR_KEY", "author")
        .string_constant("COPYRIGHT_KEY", "copyright")
        .string_constant("DATE_KEY", "date")
        .string_constant("TITLE_KEY", "title"),
        control(
            "PitchControl",
            CONTROL,
            &[
                ("setPitch", "(I)I"),
                ("getPitch", "()I"),
                ("getMaxPitch", "()I"),
                ("getMinPitch", "()I"),
            ],
        ),
        control(
            "RateControl",
            CONTROL,
            &[
                ("setRate", "(I)I"),
                ("getRate", "()I"),
                ("getMaxRate", "()I"),
                ("getMinRate", "()I"),
            ],
        ),
        control(
            "TempoControl",
            rate,
            &[("setTempo", "(I)I"), ("getTempo", "()I")],
        ),
        control(
            "RecordControl",
            CONTROL,
            &[
                ("setRecordStream", "(Ljava/io/OutputStream;)V"),
                ("setRecordLocation", "(Ljava/lang/String;)V"),
                ("getContentType", "()Ljava/lang/String;"),
                ("startRecord", "()V"),
                ("stopRecord", "()V"),
                ("commit", "()V"),
                ("setRecordSizeLimit", "(I)I"),
                ("reset", "()V"),
            ],
        ),
    ]
}

fn tone_control() -> NativeClass {
    [
        ("VERSION", -2),
        ("TEMPO", -3),
        ("RESOLUTION", -4),
        ("BLOCK_START", -5),
        ("BLOCK_END", -6),
        ("PLAY_BLOCK", -7),
        ("SET_VOLUME", -8),
        ("REPEAT", -9),
        ("C4", 60),
        ("SILENCE", -1),
    ]
    .into_iter()
    .fold(
        control("ToneControl", CONTROL, &[("setSequence", "([B)V")]),
        |class, (name, value)| class.constant(name, "B", Value::Int(value)),
    )
}

fn midi_control() -> NativeClass {
    control(
        "MIDIControl",
        CONTROL,
        &[
            ("isBankQuerySupported", "()Z"),
            ("getProgram", "(I)[I"),
            ("getChannelVolume", "(I)I"),
            ("setProgram", "(III)V"),
            ("setChannelVolume", "(II)V"),
            ("getBankList", "(Z)[I"),
            ("getProgramList", "(I)[I"),
            ("getProgramName", "(II)Ljava/lang/String;"),
            ("getKeyName", "(III)Ljava/lang/String;"),
            ("shortMidiEvent", "(III)V"),
            ("longMidiEvent", "([BII)I"),
        ],
    )
    .constant("NOTE_ON", "I", Value::Int(144))
    .constant("CONTROL_CHANGE", "I", Value::Int(176))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{Ctx, MemorySource, Value, Vm, VmConfig};

    use super::*;

    fn constant(ctx: &mut Ctx<'_>, class: &str, name: &str, desc: &str) -> Value {
        let field = ctx.static_field_id(class, name, desc).unwrap();
        ctx.get_static(field).unwrap()
    }

    #[test]
    fn every_class_links_with_the_inventory_constants() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            classes(),
        );
        vm.attach("test", |ctx| {
            for class in classes() {
                ctx.find_class(class.name()).unwrap();
            }
            assert_eq!(constant(ctx, PLAYER, "STARTED", "I"), Value::Int(400));
            assert_eq!(constant(ctx, PLAYER, "TIME_UNKNOWN", "J"), Value::Long(-1));
            let name = constant(ctx, PLAYER_LISTENER, "END_OF_MEDIA", "Ljava/lang/String;");
            assert_eq!(ctx.string(name.obj().unwrap()).unwrap(), "endOfMedia");
            let tone = "javax/microedition/media/control/ToneControl";
            assert_eq!(constant(ctx, tone, "SILENCE", "B"), Value::Int(-1));
            assert_eq!(constant(ctx, tone, "C4", "B"), Value::Int(60));
            let stop = "javax/microedition/media/control/StopTimeControl";
            assert_eq!(
                constant(ctx, stop, "RESET", "J"),
                Value::Long(STOP_TIME_RESET)
            );
            let video = "javax/microedition/media/control/VideoControl";
            assert_eq!(constant(ctx, video, "USE_DIRECT_VIDEO", "I"), Value::Int(1));
            assert_eq!(
                constant(ctx, video, "USE_GUI_PRIMITIVE", "I"),
                Value::Int(0)
            );
        });
    }

    #[test]
    fn media_exception_keeps_its_message() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            classes(),
        );
        vm.attach("test", |ctx| {
            let message = ctx.new_string("no tone device").unwrap();
            let exception = ctx
                .construct(MEDIA_EXCEPTION, "(Ljava/lang/String;)V", &[message.into()])
                .unwrap();
            let got = ctx
                .call_virtual(exception, "getMessage", "()Ljava/lang/String;", &[])
                .unwrap();
            assert_eq!(ctx.string(got.obj().unwrap()).unwrap(), "no tone device");
            let exception_class = ctx.find_class("java/lang/Exception").unwrap();
            assert!(ctx.is_instance(exception, exception_class));
        });
    }
}
