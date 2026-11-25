//! Samsung's `com.samsung.util`, on Samsung profiles only: `AudioClip`,
//! `LCDLight`, `Vibration`, and text messages (`SM`, `SMS`) that are never
//! sent.
//!
//! `AudioClip` follows freej2me-plus, which games were tested on: a type
//! outside 1 to 3 is taken for MMF, the bytes' signature decides the
//! format, `play` starts from the beginning, a loop count of 0 or 255
//! loops forever, and the volume runs from 0 to 5 (0 to 100 for MMF).
//! FastEmu cannot play MMF (Yamaha SMAF), so such clips stay silent, as
//! any sound the engine cannot play does. `SMS.isSupported()` is false
//! and `SMS.send` throws the `IllegalStateException` the javadoc names
//! for phones without messaging, after logging the attempt.

use std::sync::Arc;

use fastemu_audio::Player;
use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::{clip, device};
use crate::env::env;
use crate::suite::source::normalize;

const AUDIO_CLIP: &str = "com/samsung/util/AudioClip";
const SM: &str = "com/samsung/util/SM";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";

const TYPE_MMF: i32 = 1;
const TYPE_MP3: i32 = 2;
const TYPE_MIDI: i32 = 3;
/// The longest text an `SM` holds.
const MAX_TEXT: usize = 80;

/// What an `AudioClip` holds.
#[derive(Default)]
struct Clip {
    /// `None` when the instance is silent or the sound does not play.
    player: Option<Player>,
    mmf: bool,
}

impl Payload for Clip {}

/// Whether `bytes` start as Yamaha's MMF files do.
fn is_mmf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"MMMD")
}

/// Makes `this` play `bytes`, of the declared `kind`.
fn open(ctx: &mut Ctx<'_>, this: ObjRef, kind: i32, bytes: Vec<u8>) -> JResult<()> {
    let mmf = is_mmf(&bytes) || !(TYPE_MP3..=TYPE_MIDI).contains(&kind);
    let player = if is_mmf(&bytes) {
        ctx.log(LogLevel::Warn, "an MMF sound clip stays silent");
        None
    } else {
        clip::open(ctx, bytes, None)?
    };
    *ctx.payload_mut::<Clip>(this)? = Clip { player, mmf };
    Ok(())
}

fn from_bytes(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let data = ctx.non_null(a[2])?;
    let (offset, length) = (a[3].int(), a[4].int());
    let all = ctx.array::<i8>(data)?;
    let range = usize::try_from(offset)
        .ok()
        .zip(usize::try_from(length).ok())
        .filter(|&(o, l)| o.checked_add(l).is_some_and(|end| end <= all.len()));
    let Some((offset, length)) = range else {
        return ctx.throw(
            "java/lang/ArrayIndexOutOfBoundsException",
            "audio data range",
        );
    };
    let bytes: Vec<u8> = all[offset..offset + length]
        .iter()
        .map(|&b| b as u8)
        .collect();
    open(ctx, this, a[1].int(), bytes)?;
    Ok(Value::Void)
}

fn from_resource(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let name = ctx.non_null(a[2])?;
    let name = ctx.string(name)?;
    let path = normalize(&name);
    let source = Arc::clone(&env(ctx).source);
    let Some(bytes) = ctx.without_lock(|| source.resource(&path)) else {
        return ctx.throw(
            "java/io/IOException",
            &format!("resource not found: {name}"),
        );
    };
    open(ctx, this, a[1].int(), bytes)?;
    Ok(Value::Void)
}

/// Runs `f` on the clip's player, if it has one.
fn with_player(ctx: &mut Ctx<'_>, this: Value, f: impl FnOnce(&Player)) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    if let Some(player) = &ctx.payload::<Clip>(this)?.player {
        f(player);
    }
    Ok(Value::Void)
}

/// `play(loop, volume)`: from the beginning, `loop` times (0 or 255 for
/// ever), at `volume`.
fn play(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let this = ctx.non_null(a[0])?;
    let (count, volume) = (a[1].int(), a[2].int());
    let mmf = ctx.payload::<Clip>(this)?.mmf;
    let (loops, volumes) = if mmf {
        (-1..=255, 0..=100)
    } else {
        (0..=255, 0..=5)
    };
    if !loops.contains(&count) || !volumes.contains(&volume) {
        return ctx.throw(ILLEGAL_ARGUMENT, "loop or volume out of range");
    }
    let count = if matches!(count, 0 | 255 | -1) {
        -1
    } else {
        count
    };
    let level = if mmf && volume > 5 {
        volume
    } else {
        volume * 20
    };
    // A player the instance has closed refuses quietly; the clip is silent.
    with_player(ctx, a[0], |p| {
        let _ = p.stop();
        let _ = p.set_media_time(0);
        let _ = p.set_loop_count(count);
        let _ = p.set_level(level);
        let _ = p.start();
    })
}

fn audio_clip() -> NativeClass {
    NativeClass::new(AUDIO_CLIP)
        .payload::<Clip>()
        .constant("TYPE_MMF", "I", Value::Int(TYPE_MMF))
        .constant("TYPE_MP3", "I", Value::Int(TYPE_MP3))
        .constant("TYPE_MIDI", "I", Value::Int(TYPE_MIDI))
        .method("<init>", "(I[BII)V", from_bytes)
        .method("<init>", "(ILjava/lang/String;)V", from_resource)
        .method("play", "(II)V", play)
        .method("stop", "()V", |ctx, a| {
            with_player(ctx, a[0], |p| {
                let _ = p.stop();
                let _ = p.set_media_time(0);
            })
        })
        .method("pause", "()V", |ctx, a| {
            with_player(ctx, a[0], |p| {
                let _ = p.stop();
            })
        })
        .method("resume", "()V", |ctx, a| {
            with_player(ctx, a[0], |p| {
                let _ = p.start();
            })
        })
        .static_method("isSupported", "()Z", |_, _| Ok(Value::from(true)))
}

fn lcd_light() -> NativeClass {
    NativeClass::new("com/samsung/util/LCDLight")
        .final_class()
        .static_method("isSupported", "()Z", |_, _| Ok(Value::from(true)))
        .static_method("on", "(I)V", |ctx, a| {
            if a[0].int() < 0 {
                return ctx.throw(ILLEGAL_ARGUMENT, "negative duration");
            }
            device::set_light(ctx, device::BACKLIGHT, 100);
            Ok(Value::Void)
        })
        .static_method("off", "()V", |ctx, _| {
            device::set_light(ctx, device::BACKLIGHT, 0);
            Ok(Value::Void)
        })
}

fn vibration() -> NativeClass {
    NativeClass::new("com/samsung/util/Vibration")
        .final_class()
        .static_method("isSupported", "()Z", |_, _| Ok(Value::from(true)))
        .static_method("start", "(II)V", |ctx, a| {
            let (duration, strength) = (a[0].int(), a[1].int());
            if duration < 0 || !(1..=5).contains(&strength) {
                return ctx.throw(ILLEGAL_ARGUMENT, "duration or strength out of range");
            }
            device::vibrate(ctx, Some(duration.into()));
            Ok(Value::Void)
        })
        .static_method("stop", "()V", |ctx, _| {
            device::stop_vibrating(ctx);
            Ok(Value::Void)
        })
}

/// What an `SM` holds: its addresses and text, Java strings.
#[derive(Default)]
struct Message {
    dest: Option<ObjRef>,
    callback: Option<ObjRef>,
    data: Option<ObjRef>,
}

impl Payload for Message {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.dest);
        tracer.mark_opt(self.callback);
        tracer.mark_opt(self.data);
    }
}

fn message<'a>(ctx: &'a mut Ctx<'_>, this: Value) -> JResult<&'a mut Message> {
    let this = ctx.non_null(this)?;
    ctx.payload_mut::<Message>(this)
}

/// `IllegalArgumentException` unless `address` looks like a phone
/// number: digits, with `+`, `*` and `#`.
fn check_address(ctx: &mut Ctx<'_>, address: Value) -> JResult<()> {
    let Some(address) = address.obj() else {
        return Ok(());
    };
    let text = ctx.string(address)?;
    if text.is_empty()
        || !text
            .chars()
            .all(|c| c.is_ascii_digit() || "+*#".contains(c))
    {
        return ctx.throw(ILLEGAL_ARGUMENT, "not a phone number");
    }
    Ok(())
}

/// `IllegalArgumentException` for a text longer than 80 characters.
fn check_text(ctx: &mut Ctx<'_>, text: Value) -> JResult<()> {
    if let Some(text) = text.obj()
        && ctx.string_utf16(text)?.len() > MAX_TEXT
    {
        return ctx.throw(ILLEGAL_ARGUMENT, "a text message longer than 80 characters");
    }
    Ok(())
}

fn sm() -> NativeClass {
    let string = "Ljava/lang/String;";
    NativeClass::new(SM)
        .payload::<Message>()
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .method(
            "<init>",
            &format!("({string}{string}{string})V"),
            |ctx, a| {
                check_address(ctx, a[1])?;
                check_text(ctx, a[3])?;
                let m = message(ctx, a[0])?;
                (m.dest, m.callback, m.data) = (a[1].obj(), a[2].obj(), a[3].obj());
                Ok(Value::Void)
            },
        )
        .method("setDestAddress", &format!("({string})V"), |ctx, a| {
            check_address(ctx, a[1])?;
            message(ctx, a[0])?.dest = a[1].obj();
            Ok(Value::Void)
        })
        .method("setCallbackAddress", &format!("({string})V"), |ctx, a| {
            message(ctx, a[0])?.callback = a[1].obj();
            Ok(Value::Void)
        })
        .method("setData", &format!("({string})V"), |ctx, a| {
            check_text(ctx, a[1])?;
            message(ctx, a[0])?.data = a[1].obj();
            Ok(Value::Void)
        })
        .method("getDestAddress", &format!("(){string}"), |ctx, a| {
            Ok(message(ctx, a[0])?.dest.into())
        })
        .method("getCallbackAddress", &format!("(){string}"), |ctx, a| {
            Ok(message(ctx, a[0])?.callback.into())
        })
        .method("getData", &format!("(){string}"), |ctx, a| {
            Ok(message(ctx, a[0])?.data.into())
        })
}

fn sms() -> NativeClass {
    NativeClass::new("com/samsung/util/SMS")
        .static_method("isSupported", "()Z", |_, _| Ok(Value::from(false)))
        .static_method("send", &format!("(L{SM};)V"), |ctx, _| {
            ctx.log(LogLevel::Info, "a text message was not sent");
            ctx.throw(
                "java/lang/IllegalStateException",
                "this phone sends no text messages",
            )
        })
}

/// The classes of Samsung's API.
pub fn classes() -> Vec<NativeClass> {
    vec![audio_clip(), lcd_light(), vibration(), sm(), sms()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmf_files_are_known_by_their_signature() {
        assert!(is_mmf(b"MMMD\0\0"));
        assert!(!is_mmf(b"MThd"));
        assert!(!is_mmf(b"MM"));
    }
}
