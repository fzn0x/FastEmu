//! `javax.microedition.media.Manager`: making players, `playTone`, and
//! what the phone says it supports.

use std::sync::Arc;

use fastemu_audio::{Format, SUPPORTED_CONTENT_TYPES};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::player::{self, Source};
use super::{events, protocol, stream, time_base};

const MANAGER: &str = "javax/microedition/media/Manager";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const MEDIA_EXCEPTION: &str = super::api::MEDIA_EXCEPTION;

/// The protocols `getSupportedProtocols` reports: tones from the device,
/// and sound from the suite's JAR (`resource://` and `file://` locators
/// and bare resource paths all read it).
const PROTOCOLS: &[&str] = &["device", "file", "resource"];

/// The shortest tone `playTone` plays, in milliseconds. Karma Studios
/// games beep for 10 ms, which is hardly heard; freej2me-plus lengthens
/// tones to 50 ms for them (`docs/compat/notes/07-sound.md`).
const MIN_TONE_MS: i32 = 50;

/// The format to play `bytes` as: the one the bytes show, since games
/// often pass the wrong type, or else the one the declared type names.
/// `None` when FastEmu plays neither.
fn decide(bytes: &[u8], declared: Option<&str>) -> Option<Format> {
    Format::sniff(bytes).or_else(|| declared.and_then(Format::from_content_type))
}

/// The content type a file name's extension suggests, for locators.
fn type_for_name(path: &str) -> Option<&'static str> {
    let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "wav" => "audio/x-wav",
        "mid" | "midi" | "kar" => "audio/midi",
        "mp3" => "audio/mpeg",
        "amr" => "audio/amr",
        "jts" => "audio/x-tone-seq",
        "ott" | "rng" => "application/vnd.nokia.ringing-tone",
        _ => return None,
    })
}

/// A player for sound data, logging sound FastEmu plays as silence.
pub(super) fn player_for(
    ctx: &mut Ctx<'_>,
    bytes: Vec<u8>,
    declared: Option<String>,
) -> JResult<ObjRef> {
    let format = decide(&bytes, declared.as_deref());
    let what = match &declared {
        Some(content_type) => format!("{content_type:?}"),
        None => "a sound with no content type".to_owned(),
    };
    // The player keeps the bytes until it realizes, and the heap counts
    // them: a request the heap could never hold is refused before the
    // copy. A sound of no known format is never decoded, so it keeps none.
    let kept: Arc<[u8]> = if format.is_some() {
        ctx.check_alloc(bytes.len())?;
        Arc::from(bytes.as_slice())
    } else {
        Arc::from(Vec::new())
    };
    let source = Source::Data {
        bytes: kept,
        format,
        declared,
    };
    // Made first: the warning goes through the registry it sets up.
    let obj = player::new_player(ctx, source)?;
    if format.is_none() {
        events::warn_silent(ctx, &what, &bytes);
    }
    Ok(obj)
}

fn create_from_stream(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let Some(stream) = a[0].obj() else {
        return ctx.throw(ILLEGAL_ARGUMENT, "the stream is null");
    };
    let declared = ctx.opt_string(a[1])?;
    let bytes = stream::read_all(ctx, stream)?;
    Ok(player_for(ctx, bytes, declared)?.into())
}

/// What a locator names.
#[derive(Debug, PartialEq, Eq)]
enum Locator<'a> {
    ToneDevice,
    MidiDevice,
    /// A resource of the suite, as an absolute path.
    Resource(String),
    /// A device or capture locator FastEmu has no device for.
    NoDevice,
    /// Media somewhere FastEmu does not go (the network): a silent player.
    Elsewhere(&'a str),
}

fn parse_locator(locator: &str) -> Locator<'_> {
    let lower = locator.to_ascii_lowercase();
    if lower == "device://tone" {
        return Locator::ToneDevice;
    }
    if lower == "device://midi" {
        return Locator::MidiDevice;
    }
    if lower.starts_with("device:") || lower.starts_with("capture:") {
        return Locator::NoDevice;
    }
    for scheme in ["resource:", "file:"] {
        if lower.starts_with(scheme) {
            let path = locator[scheme.len()..].trim_start_matches('/');
            return Locator::Resource(format!("/{path}"));
        }
    }
    match locator.split_once("://") {
        Some((scheme, _)) if !scheme.contains('/') => Locator::Elsewhere(locator),
        _ => Locator::Resource(format!("/{}", locator.trim_start_matches('/'))),
    }
}

/// The bytes of resource `path` (absolute) through
/// `Class.getResourceAsStream`, so the suite's lookup rules apply.
fn resource(ctx: &mut Ctx<'_>, path: &str) -> JResult<Option<Vec<u8>>> {
    let class = ctx.find_class(MANAGER)?;
    let mirror = ctx.class_object(class);
    let name = ctx.new_string(path)?;
    let stream = ctx
        .call_virtual(
            mirror,
            "getResourceAsStream",
            "(Ljava/lang/String;)Ljava/io/InputStream;",
            &[name.into()],
        )?
        .obj();
    match stream {
        Some(stream) => Ok(Some(stream::read_all(ctx, stream)?)),
        None => Ok(None),
    }
}

fn create_from_locator(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let Some(locator) = ctx.opt_string(a[0])? else {
        return ctx.throw(ILLEGAL_ARGUMENT, "the locator is null");
    };
    let player = match parse_locator(&locator) {
        Locator::ToneDevice => player::new_player(ctx, Source::ToneDevice)?,
        Locator::MidiDevice => {
            let player = player::new_player(ctx, Source::MidiDevice)?;
            events::warn_silent(ctx, &format!("{locator:?} (live MIDI)"), &[]);
            player
        }
        Locator::NoDevice => {
            return ctx.throw(MEDIA_EXCEPTION, &format!("no device for {locator}"));
        }
        Locator::Resource(path) => {
            let Some(bytes) = resource(ctx, &path)? else {
                return ctx.throw("java/io/IOException", &format!("no resource {path}"));
            };
            player_for(ctx, bytes, type_for_name(&path).map(str::to_owned))?
        }
        Locator::Elsewhere(place) => {
            events::warn_silent(ctx, &format!("{place:?} (FastEmu does not fetch it)"), &[]);
            let source = Source::Data {
                bytes: Arc::from(Vec::new()),
                format: None,
                declared: type_for_name(place).map(str::to_owned),
            };
            player::new_player(ctx, source)?
        }
    };
    Ok(player.into())
}

fn play_tone(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (note, duration, volume) = (a[0].int(), a[1].int(), a[2].int());
    if !(0..=127).contains(&note) {
        return ctx.throw(ILLEGAL_ARGUMENT, &format!("note {note}"));
    }
    if duration <= 0 {
        return ctx.throw(ILLEGAL_ARGUMENT, &format!("duration {duration}"));
    }
    if let Some(group) = events::group(ctx) {
        // Only a closed group (the instance ending) refuses, and then
        // silence is right.
        let duration = duration.max(MIN_TONE_MS);
        let _ = group.play_tone(note, duration, volume.clamp(0, 100));
    }
    Ok(Value::Void)
}

/// A new `String[]` holding `items`.
fn strings(ctx: &mut Ctx<'_>, items: &[&str]) -> JResult<Value> {
    let class = ctx.find_class("java/lang/String")?;
    let array = ctx.new_object_array(class, items.len() as i32)?;
    for (i, item) in items.iter().enumerate() {
        let text = ctx.new_string(item)?;
        ctx.array_mut::<Option<ObjRef>>(array)?[i] = Some(text);
    }
    Ok(array.into())
}

/// `getSupportedContentTypes`: every type for the JAR protocols (and for
/// `null`), tone sequences for `device`, nothing for the rest.
fn content_types_for(protocol: Option<&str>) -> Vec<&'static str> {
    match protocol.map(str::to_ascii_lowercase).as_deref() {
        None | Some("file" | "resource") => SUPPORTED_CONTENT_TYPES.to_vec(),
        Some("device") => vec![Format::ToneSequence.content_type()],
        Some(_) => Vec::new(),
    }
}

/// `getSupportedProtocols`: the protocols that give the content type.
fn protocols_for(content_type: Option<&str>) -> Vec<&'static str> {
    let Some(content_type) = content_type else {
        return PROTOCOLS.to_vec();
    };
    PROTOCOLS
        .iter()
        .copied()
        .filter(|protocol| {
            content_types_for(Some(protocol))
                .iter()
                .any(|t| t.eq_ignore_ascii_case(content_type.trim()))
        })
        .collect()
}

pub(crate) fn class() -> NativeClass {
    NativeClass::new(MANAGER)
        .final_class()
        .string_constant("TONE_DEVICE_LOCATOR", "device://tone")
        .string_constant("MIDI_DEVICE_LOCATOR", "device://midi")
        .static_method(
            "createPlayer",
            "(Ljava/io/InputStream;Ljava/lang/String;)Ljavax/microedition/media/Player;",
            create_from_stream,
        )
        .static_method(
            "createPlayer",
            "(Ljava/lang/String;)Ljavax/microedition/media/Player;",
            create_from_locator,
        )
        .static_method(
            "createPlayer",
            "(Ljavax/microedition/media/protocol/DataSource;)Ljavax/microedition/media/Player;",
            protocol::create_player,
        )
        .static_method("playTone", "(III)V", play_tone)
        .static_method(
            "getSupportedContentTypes",
            "(Ljava/lang/String;)[Ljava/lang/String;",
            |ctx, a| {
                let protocol = ctx.opt_string(a[0])?;
                strings(ctx, &content_types_for(protocol.as_deref()))
            },
        )
        .static_method(
            "getSupportedProtocols",
            "(Ljava/lang/String;)[Ljava/lang/String;",
            |ctx, a| {
                let content_type = ctx.opt_string(a[0])?;
                strings(ctx, &protocols_for(content_type.as_deref()))
            },
        )
        .static_method(
            "getSystemTimeBase",
            "()Ljavax/microedition/media/TimeBase;",
            |ctx, _| Ok(time_base::system(ctx)?.into()),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locators_name_resources_devices_and_the_network() {
        assert_eq!(parse_locator("device://tone"), Locator::ToneDevice);
        assert_eq!(parse_locator("DEVICE://TONE"), Locator::ToneDevice);
        assert_eq!(parse_locator("device://midi"), Locator::MidiDevice);
        assert_eq!(parse_locator("device://video"), Locator::NoDevice);
        assert_eq!(parse_locator("capture://audio"), Locator::NoDevice);
        for (locator, path) in [
            ("resource:///snd/a.mid", "/snd/a.mid"),
            ("resource://a.wav", "/a.wav"),
            ("resource:/a.wav", "/a.wav"),
            ("file:///a.mid", "/a.mid"),
            ("/music.mid", "/music.mid"),
            ("music.mid", "/music.mid"),
            ("snd/x.wav", "/snd/x.wav"),
        ] {
            assert_eq!(
                parse_locator(locator),
                Locator::Resource(path.into()),
                "{locator}"
            );
        }
        assert_eq!(
            parse_locator("http://example.com/a.mid"),
            Locator::Elsewhere("http://example.com/a.mid")
        );
    }

    #[test]
    fn the_bytes_decide_before_the_declared_type() {
        let wav = b"RIFF\0\0\0\0WAVEfmt ";
        assert_eq!(decide(wav, Some("audio/midi")), Some(Format::Wav));
        assert_eq!(decide(b"zz", Some("audio/midi")), Some(Format::Midi));
        assert_eq!(decide(b"zz", Some("audio/x-unknown")), None);
        assert_eq!(decide(b"zz", None), None);
        assert_eq!(type_for_name("/a/B.MID"), Some("audio/midi"));
        assert_eq!(type_for_name("/a/b"), None);
    }

    #[test]
    fn supported_types_and_protocols_agree() {
        assert!(content_types_for(None).contains(&"audio/x-wav"));
        assert_eq!(content_types_for(Some("device")), vec!["audio/x-tone-seq"]);
        assert!(content_types_for(Some("rtsp")).is_empty());
        assert_eq!(
            protocols_for(Some("audio/x-tone-seq")),
            vec!["device", "file", "resource"]
        );
        assert_eq!(protocols_for(Some("audio/midi")), vec!["file", "resource"]);
        assert!(protocols_for(Some("video/mpeg")).is_empty());
        assert_eq!(protocols_for(None), PROTOCOLS.to_vec());
    }
}
