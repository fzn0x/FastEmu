//! `com.nokia.mid.sound.Sound` and `SoundListener`: Nokia's sound API from
//! before MMAPI, on the same tracks, registry and event thread as MMAPI
//! players (see `docs/architecture/media.md`).
//!
//! It plays Smart Messaging ringing tones (`FORMAT_TONE`), WAV
//! (`FORMAT_WAV`, though any format FastEmu plays is accepted) and beeps.
//! Following the emulators games were tested on, data that does not
//! decode leaves the sound `SOUND_UNINITIALIZED` instead of throwing, and
//! playing it does nothing.

use std::time::Instant;

use fastemu_audio::{AudioError, Format, Group, PlayerState};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use crate::media::Event;
use crate::media::events::{self, Owner};
use crate::media::track::{Backend, Track};

const SOUND: &str = "com/nokia/mid/sound/Sound";
const SOUND_LISTENER: &str = "com/nokia/mid/sound/SoundListener";
const SOUND_STATE_CHANGED: &str = "(Lcom/nokia/mid/sound/Sound;I)V";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";

const FORMAT_TONE: i32 = 1;
const FORMAT_WAV: i32 = 5;
const SOUND_PLAYING: i32 = 0;
const SOUND_STOPPED: i32 = 1;
const SOUND_UNINITIALIZED: i32 = 3;

/// The gain a sound has until the game sets one: full.
const DEFAULT_GAIN: i32 = 255;

/// What a `Sound` keeps beside its (empty) Java fields.
#[derive(Default)]
pub(crate) struct SoundData {
    pub track: Track,
    /// The gain the game set, 0 to 255.
    gain: Option<i32>,
    listener: Option<ObjRef>,
    /// Whether the listener heard `SOUND_PLAYING` for the run going on, so
    /// the passes of a loop do not repeat it.
    told_playing: bool,
}

impl Payload for SoundData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.listener);
    }
}

/// What a sound is initialized with.
enum Sounding {
    Data { bytes: Vec<u8>, format: i32 },
    Beep { frequency: i32, duration_ms: i64 },
}

/// Makes the sound in the instance's group, or without one in `probe`
/// only to learn its length for a silent player.
fn open(
    sounding: &Sounding,
    group: Option<&Group>,
    probe: Option<&Group>,
) -> Result<(Backend, Format), AudioError> {
    let target = group.or(probe).ok_or(AudioError::Closed)?;
    let (player, format) = match sounding {
        Sounding::Beep {
            frequency,
            duration_ms,
        } => (
            target.create_beep_player(*frequency, *duration_ms)?,
            Format::ToneSequence,
        ),
        Sounding::Data { bytes, format } => {
            // Trust the bytes when they show a format: games label MIDI
            // and WAV loosely.
            let declared = if *format == FORMAT_TONE {
                Format::NokiaRingtone
            } else {
                Format::Wav
            };
            let format = match Format::sniff(bytes) {
                Some(Format::NokiaRingtone) | None => declared,
                Some(found) => found,
            };
            (target.create_player_as(bytes, format)?, format)
        }
    };
    if group.is_some() {
        return Ok((Backend::Engine(player), format));
    }
    let duration = player.duration()?;
    Ok((Backend::silent(duration), format))
}

fn data<'c>(ctx: &'c mut Ctx<'_>, this: ObjRef) -> JResult<&'c mut SoundData> {
    ctx.payload_mut::<SoundData>(this)
}

/// `Sound(...)` and `init(...)`: replaces the sound's media. A beep with
/// bad values throws `IllegalArgumentException`; data that does not decode
/// leaves the sound uninitialized.
fn init(ctx: &mut Ctx<'_>, this: ObjRef, sounding: Sounding) -> JResult<()> {
    if data(ctx, this)?.track.serial() == 0 {
        let track = events::register(ctx, this, Owner::Sound)?;
        data(ctx, this)?.track = track;
    }
    let group = events::group(ctx);
    let probe = match group {
        Some(_) => None,
        None => Some(events::probe(ctx)?),
    };
    let opened = ctx.without_lock(|| open(&sounding, group.as_ref(), probe.as_ref()));
    let sound = data(ctx, this)?;
    sound.told_playing = false;
    match opened {
        Ok((backend, format)) => {
            let old = sound
                .track
                .install(backend, Some(format), format.content_type().to_owned());
            let (serial, id) = (sound.track.serial(), sound.track.engine_id());
            // The old sound closes here; its `Closed` event finds a new
            // sound in place and is not passed on.
            drop(old);
            if let Some(id) = id {
                events::link(ctx, id, serial);
            }
            Ok(())
        }
        Err(error @ AudioError::InvalidArgument(_)) => {
            ctx.throw(ILLEGAL_ARGUMENT, &error.to_string())
        }
        Err(error) => {
            drop(sound.track.uninstall());
            if let Sounding::Data { bytes, .. } = &sounding {
                events::warn_silent(ctx, &format!("a Nokia sound ({error})"), bytes);
            }
            Ok(())
        }
    }
}

fn init_data(ctx: &mut Ctx<'_>, this: ObjRef, a: &[Value]) -> JResult<()> {
    let array = ctx.non_null(a[1])?;
    let format = a[2].int();
    if format != FORMAT_TONE && format != FORMAT_WAV {
        return ctx.throw(ILLEGAL_ARGUMENT, &format!("sound format {format}"));
    }
    let bytes = ctx.array::<i8>(array)?.iter().map(|&b| b as u8).collect();
    init(ctx, this, Sounding::Data { bytes, format })
}

fn init_beep(ctx: &mut Ctx<'_>, this: ObjRef, a: &[Value]) -> JResult<()> {
    let sounding = Sounding::Beep {
        frequency: a[1].int(),
        duration_ms: a[2].long(),
    };
    init(ctx, this, sounding)
}

/// The `SOUND_*` state of `track`.
fn state_code(track: &mut Track) -> i32 {
    if !track.has_backend() {
        return SOUND_UNINITIALIZED;
    }
    match track.state(Instant::now()) {
        PlayerState::Started => SOUND_PLAYING,
        PlayerState::Closed => SOUND_UNINITIALIZED,
        _ => SOUND_STOPPED,
    }
}

/// Starts the sound from where it is.
fn start(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    let serial = data(ctx, this)?.track.serial();
    events::hold(ctx, serial, this);
    let _ = data(ctx, this)?.track.start(Instant::now());
    events::wake(ctx);
    Ok(())
}

fn play(ctx: &mut Ctx<'_>, this: ObjRef, loops: i32) -> JResult<()> {
    if loops < 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, &format!("loop {loops}"));
    }
    let track = &mut data(ctx, this)?.track;
    if state_code(track) == SOUND_UNINITIALIZED {
        return Ok(());
    }
    let now = Instant::now();
    // Restarting a playing sound stops it first, then it plays from the
    // start; 0 loops forever.
    let _ = track.stop(now);
    let _ = track.set_loop_count(if loops == 0 { -1 } else { loops });
    let _ = track.set_media_time(0, now);
    start(ctx, this)
}

fn release(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    let sound = data(ctx, this)?;
    if state_code(&mut sound.track) == SOUND_UNINITIALIZED {
        return Ok(());
    }
    let serial = sound.track.serial();
    events::hold(ctx, serial, this);
    data(ctx, this)?.track.close();
    Ok(())
}

/// Tells the sound's listener what `event` changed (on the media event
/// thread). Only the end of the last pass of a loop counts as stopping.
pub(crate) fn deliver(ctx: &mut Ctx<'_>, sound: ObjRef, event: Event) -> JResult<()> {
    let data = data(ctx, sound)?;
    let code = match event {
        Event::Started(_) if !data.told_playing => {
            data.told_playing = true;
            SOUND_PLAYING
        }
        Event::EndOfMedia { last: true, .. } => {
            data.told_playing = false;
            SOUND_STOPPED
        }
        Event::Stopped(_) | Event::StoppedAtTime(_) => {
            data.told_playing = false;
            SOUND_STOPPED
        }
        // The close of a sound that `init` replaced is not news.
        Event::Closed if state_code(&mut data.track) == SOUND_UNINITIALIZED => {
            data.told_playing = false;
            SOUND_UNINITIALIZED
        }
        _ => return Ok(()),
    };
    let Some(listener) = data.listener else {
        return Ok(());
    };
    let listener = ctx.root(listener);
    events::call_listener(
        ctx,
        "SoundListener.soundStateChanged",
        listener,
        "soundStateChanged",
        SOUND_STATE_CHANGED,
        &[sound.into(), Value::Int(code)],
    )
}

/// Runs a native on `this` that returns nothing.
fn void(
    f: impl Fn(&mut Ctx<'_>, ObjRef, &[Value]) -> JResult<()> + Send + Sync + 'static,
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static {
    move |ctx, a| {
        let this = ctx.non_null(a[0])?;
        f(ctx, this, a)?;
        Ok(Value::Void)
    }
}

fn sound() -> NativeClass {
    NativeClass::new(SOUND)
        .payload::<SoundData>()
        .constant("FORMAT_TONE", "I", Value::Int(FORMAT_TONE))
        .constant("FORMAT_WAV", "I", Value::Int(FORMAT_WAV))
        .constant("SOUND_PLAYING", "I", Value::Int(SOUND_PLAYING))
        .constant("SOUND_STOPPED", "I", Value::Int(SOUND_STOPPED))
        .constant("SOUND_UNINITIALIZED", "I", Value::Int(SOUND_UNINITIALIZED))
        .method("<init>", "([BI)V", void(init_data))
        .method("<init>", "(IJ)V", void(init_beep))
        .method("init", "([BI)V", void(init_data))
        .method("init", "(IJ)V", void(init_beep))
        .method("getState", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int(state_code(&mut data(ctx, this)?.track)))
        })
        .method(
            "play",
            "(I)V",
            void(|ctx, this, a| play(ctx, this, a[1].int())),
        )
        .method(
            "stop",
            "()V",
            void(|ctx, this, _| {
                let _ = data(ctx, this)?.track.stop(Instant::now());
                Ok(())
            }),
        )
        .method(
            "resume",
            "()V",
            void(|ctx, this, _| {
                if state_code(&mut data(ctx, this)?.track) == SOUND_STOPPED {
                    start(ctx, this)?;
                }
                Ok(())
            }),
        )
        .method("release", "()V", void(|ctx, this, _| release(ctx, this)))
        .method(
            "setGain",
            "(I)V",
            void(|ctx, this, a| {
                let gain = a[1].int().clamp(0, 255);
                let sound = data(ctx, this)?;
                sound.gain = Some(gain);
                sound.track.set_level(gain * 100 / 255);
                Ok(())
            }),
        )
        .method("getGain", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int(data(ctx, this)?.gain.unwrap_or(DEFAULT_GAIN)))
        })
        .method(
            "setSoundListener",
            "(Lcom/nokia/mid/sound/SoundListener;)V",
            void(|ctx, this, a| {
                data(ctx, this)?.listener = a[1].obj();
                Ok(())
            }),
        )
        .static_method("getConcurrentSoundCount", "(I)I", |ctx, a| {
            match a[0].int() {
                // One at a time, as Nokia's phones played them.
                FORMAT_TONE | FORMAT_WAV => Ok(Value::Int(1)),
                other => ctx.throw(ILLEGAL_ARGUMENT, &format!("sound format {other}")),
            }
        })
        .static_method("getSupportedFormats", "()[I", |ctx, _| {
            let array = ctx.new_array(fastemu_jvm::ArrayType::Int, 2)?;
            ctx.array_mut::<i32>(array)?
                .copy_from_slice(&[FORMAT_TONE, FORMAT_WAV]);
            Ok(array.into())
        })
}

/// The classes of this module.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        sound(),
        NativeClass::interface(SOUND_LISTENER)
            .abstract_method("soundStateChanged", SOUND_STATE_CHANGED),
    ]
}
