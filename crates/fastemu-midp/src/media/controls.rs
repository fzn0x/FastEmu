//! The controls a player hands out: `VolumeControl` for every player,
//! `ToneControl` for tone players, `MIDIControl` for MIDI players and
//! `StopTimeControl` for every player. Each control object points back at
//! its player; the player's payload keeps the controls, so both stay
//! alive together.

use std::time::Instant;

use fastemu_audio::{Format, PlayerState};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::api::{self, STOP_TIME_RESET};
use super::{events, player, throw_audio};

const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";
const ILLEGAL_STATE: &str = "java/lang/IllegalStateException";

/// A kind of control a player offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControlKind {
    Volume,
    Tone,
    Midi,
    StopTime,
}

impl ControlKind {
    /// The interface's simple name, as `getControl` takes it.
    pub fn interface_name(self) -> &'static str {
        match self {
            Self::Volume => "VolumeControl",
            Self::Tone => "ToneControl",
            Self::Midi => "MIDIControl",
            Self::StopTime => "StopTimeControl",
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::Volume => "javax/microedition/media/VolumeControlImpl",
            Self::Tone => "javax/microedition/media/ToneControlImpl",
            Self::Midi => "javax/microedition/media/MIDIControlImpl",
            Self::StopTime => "javax/microedition/media/StopTimeControlImpl",
        }
    }
}

/// What a control object keeps: its player.
#[derive(Default)]
struct ControlData {
    player: Option<ObjRef>,
    /// `MIDIControl`: the channel volumes the game set, -1 where unknown.
    channel_volumes: Option<[i32; 16]>,
}

impl Payload for ControlData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.player);
    }
}

/// A new control of `kind` for `player`.
pub(crate) fn new_control(ctx: &mut Ctx<'_>, kind: ControlKind, player: ObjRef) -> JResult<ObjRef> {
    let class = ctx.find_class(kind.class())?;
    let control = ctx.new_object(class)?;
    ctx.payload_mut::<ControlData>(control)?.player = Some(player);
    Ok(control)
}

/// The player of the control `this`, rooted.
fn owner(ctx: &mut Ctx<'_>, this: Value) -> JResult<ObjRef> {
    let this = ctx.non_null(this)?;
    let player = ctx
        .payload::<ControlData>(this)?
        .player
        .expect("a control is made for a player");
    Ok(ctx.root(player))
}

/// A control class over `interface` whose instances carry a player.
fn control_class(kind: ControlKind, interface: &str) -> NativeClass {
    NativeClass::new(kind.class())
        .implements(interface)
        .final_class()
        .payload::<ControlData>()
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![volume(), tone(), midi(), stop_time()]
}

fn volume() -> NativeClass {
    control_class(ControlKind::Volume, api::VOLUME_CONTROL)
        .method("setLevel", "(I)I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            Ok(Value::Int(
                player::track(ctx, player)?.set_level(a[1].int()),
            ))
        })
        .method("getLevel", "()I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            Ok(Value::Int(player::track(ctx, player)?.level()))
        })
        .method("setMute", "(Z)V", |ctx, a| {
            let player = owner(ctx, a[0])?;
            player::track(ctx, player)?.set_muted(a[1].boolean());
            Ok(Value::Void)
        })
        .method("isMuted", "()Z", |ctx, a| {
            let player = owner(ctx, a[0])?;
            Ok(Value::from(player::track(ctx, player)?.is_muted()))
        })
}

fn tone() -> NativeClass {
    control_class(ControlKind::Tone, api::TONE_CONTROL).method("setSequence", "([B)V", |ctx, a| {
        let player = owner(ctx, a[0])?;
        let Some(sequence) = a[1].obj() else {
            return ctx.throw(ILLEGAL_ARGUMENT, "the sequence is null");
        };
        if player::is_closed(ctx, player)? {
            return ctx.throw(ILLEGAL_STATE, "the player is closed");
        }
        let bytes: Vec<u8> = ctx
            .array::<i8>(sequence)?
            .iter()
            .map(|&b| b as u8)
            .collect();
        // A silent tone player still checks the sequence and learns its
        // length, in the instance's group when it has one (the player is
        // silent because its first data did not parse) or in the probe.
        let measure = match events::group(ctx) {
            Some(group) => group,
            None => events::probe(ctx)?,
        };
        let result = player::track(ctx, player)?.set_tone_sequence(&bytes, |bytes| {
            measure
                .create_player_as(bytes, Format::ToneSequence)?
                .duration()
        });
        result.or_else(|e| throw_audio(ctx, e))?;
        Ok(Value::Void)
    })
}

/// Checks a MIDI channel number.
fn channel(ctx: &mut Ctx<'_>, channel: i32) -> JResult<u8> {
    if (0..16).contains(&channel) {
        Ok(channel as u8)
    } else {
        ctx.throw(ILLEGAL_ARGUMENT, &format!("MIDI channel {channel}"))
    }
}

/// Throws `IllegalStateException` unless `player` is prefetched or
/// started, as every `MIDIControl` method that touches the synthesizer
/// requires.
fn require_prefetched(ctx: &mut Ctx<'_>, player: ObjRef) -> JResult<()> {
    if player::is_prefetched(ctx, player)? {
        Ok(())
    } else {
        ctx.throw(ILLEGAL_STATE, "the player is not prefetched")
    }
}

/// Sends a short MIDI message to `player`'s synthesizer; a silent MIDI
/// player takes it and plays nothing.
fn send_midi(
    ctx: &mut Ctx<'_>,
    player: ObjRef,
    status: i32,
    data1: i32,
    data2: i32,
) -> JResult<()> {
    let result = match player::track(ctx, player)?.engine() {
        Some(engine) => engine.midi_short_event(status, data1, data2),
        None => Ok(()),
    };
    result.or_else(|e| throw_audio(ctx, e))
}

fn no_bank_queries<T>(ctx: &mut Ctx<'_>) -> JResult<T> {
    ctx.throw(api::MEDIA_EXCEPTION, "bank queries are not supported")
}

fn midi() -> NativeClass {
    control_class(ControlKind::Midi, api::MIDI_CONTROL)
        .method("isBankQuerySupported", "()Z", |_, _| Ok(Value::from(false)))
        .method("getProgram", "(I)[I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            channel(ctx, a[1].int())?;
            require_prefetched(ctx, player)?;
            no_bank_queries(ctx)
        })
        .method("getChannelVolume", "(I)I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let ch = channel(ctx, a[1].int())?;
            require_prefetched(ctx, player)?;
            let this = ctx.non_null(a[0])?;
            let volumes = ctx.payload::<ControlData>(this)?.channel_volumes;
            Ok(Value::Int(volumes.map_or(-1, |v| v[usize::from(ch)])))
        })
        .method("setProgram", "(III)V", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let ch = i32::from(channel(ctx, a[1].int())?);
            let (bank, program) = (a[2].int(), a[3].int());
            if !(-1..=16383).contains(&bank) || !(0..=127).contains(&program) {
                return ctx.throw(ILLEGAL_ARGUMENT, &format!("bank {bank}, program {program}"));
            }
            require_prefetched(ctx, player)?;
            if bank >= 0 {
                send_midi(ctx, player, 0xB0 | ch, 0, bank >> 7)?;
                send_midi(ctx, player, 0xB0 | ch, 32, bank & 0x7F)?;
            }
            send_midi(ctx, player, 0xC0 | ch, program, 0)?;
            Ok(Value::Void)
        })
        .method("setChannelVolume", "(II)V", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let ch = channel(ctx, a[1].int())?;
            let volume = a[2].int();
            if !(0..=127).contains(&volume) {
                return ctx.throw(ILLEGAL_ARGUMENT, &format!("volume {volume}"));
            }
            require_prefetched(ctx, player)?;
            send_midi(ctx, player, 0xB0 | i32::from(ch), 7, volume)?;
            let this = ctx.non_null(a[0])?;
            let data = ctx.payload_mut::<ControlData>(this)?;
            data.channel_volumes.get_or_insert([-1; 16])[usize::from(ch)] = volume;
            Ok(Value::Void)
        })
        .method("getBankList", "(Z)[I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            require_prefetched(ctx, player)?;
            no_bank_queries(ctx)
        })
        .method("getProgramList", "(I)[I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            require_prefetched(ctx, player)?;
            no_bank_queries(ctx)
        })
        .method("getProgramName", "(II)Ljava/lang/String;", |ctx, a| {
            let player = owner(ctx, a[0])?;
            require_prefetched(ctx, player)?;
            no_bank_queries(ctx)
        })
        .method("getKeyName", "(III)Ljava/lang/String;", |ctx, a| {
            let player = owner(ctx, a[0])?;
            require_prefetched(ctx, player)?;
            no_bank_queries(ctx)
        })
        .method("shortMidiEvent", "(III)V", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let (status, data1, data2) = (a[1].int(), a[2].int(), a[3].int());
            if !(0x80..=0xFF).contains(&status)
                || status == 0xF0
                || status == 0xF7
                || !(0..=127).contains(&data1)
                || !(0..=127).contains(&data2)
            {
                return ctx.throw(
                    ILLEGAL_ARGUMENT,
                    &format!("MIDI message {status:#X} {data1} {data2}"),
                );
            }
            require_prefetched(ctx, player)?;
            send_midi(ctx, player, status, data1, data2)?;
            Ok(Value::Void)
        })
        .method("longMidiEvent", "([BII)I", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let Some(array) = a[1].obj() else {
                return ctx.throw(ILLEGAL_ARGUMENT, "the data is null");
            };
            let (offset, length) = (a[2].int(), a[3].int());
            let size = ctx.array_len(array)?;
            let in_range = usize::try_from(offset)
                .ok()
                .zip(usize::try_from(length).ok())
                .and_then(|(o, l)| o.checked_add(l))
                .is_some_and(|end| end <= size);
            if !in_range {
                return ctx.throw(
                    ILLEGAL_ARGUMENT,
                    &format!("offset {offset} and length {length} for {size} bytes"),
                );
            }
            require_prefetched(ctx, player)?;
            let start = offset as usize;
            let bytes: Vec<u8> = ctx.array::<i8>(array)?[start..start + length as usize]
                .iter()
                .map(|&b| b as u8)
                .collect();
            for (status, data1, data2) in channel_messages(&bytes) {
                send_midi(ctx, player, status, data1, data2)?;
            }
            Ok(Value::Int(length))
        })
}

/// The channel messages in a run of MIDI bytes, with running status;
/// system exclusive and other system messages are skipped.
fn channel_messages(bytes: &[u8]) -> Vec<(i32, i32, i32)> {
    let mut messages = Vec::new();
    let mut status: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        if byte >= 0xF0 {
            // System messages: skip a system exclusive up to its end.
            if byte == 0xF0 {
                while i < bytes.len() && bytes[i] != 0xF7 {
                    i += 1;
                }
            }
            status = None;
            i += 1;
            continue;
        }
        if byte >= 0x80 {
            status = Some(byte);
            i += 1;
        }
        let Some(current) = status else {
            i += 1;
            continue;
        };
        let length = if matches!(current & 0xF0, 0xC0 | 0xD0) {
            1
        } else {
            2
        };
        let Some(data) = bytes.get(i..i + length) else {
            break;
        };
        // A status byte where data was due starts the next message.
        if let Some(at) = data.iter().position(|&d| d >= 0x80) {
            i += at;
            continue;
        }
        let data2 = if length == 2 { i32::from(data[1]) } else { 0 };
        messages.push((i32::from(current), i32::from(data[0]), data2));
        i += length;
    }
    messages
}

fn stop_time() -> NativeClass {
    control_class(ControlKind::StopTime, api::STOP_TIME_CONTROL)
        .method("setStopTime", "(J)V", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let time = a[1].long();
            let track = player::track(ctx, player)?;
            if track.state(Instant::now()) == PlayerState::Started && track.stop_time().is_some() {
                return ctx.throw(ILLEGAL_STATE, "a stop time is already set");
            }
            track.set_stop_time((time != STOP_TIME_RESET).then_some(time));
            events::wake(ctx);
            Ok(Value::Void)
        })
        .method("getStopTime", "()J", |ctx, a| {
            let player = owner(ctx, a[0])?;
            let time = player::track(ctx, player)?.stop_time();
            Ok(Value::Long(time.unwrap_or(STOP_TIME_RESET)))
        })
}

#[cfg(test)]
mod tests {
    use super::channel_messages;

    #[test]
    fn long_midi_events_split_into_channel_messages() {
        let bytes = [
            0x90, 60, 100, // note on
            62, 90, // running status
            0xF0, 1, 2, 3, 0xF7, // system exclusive
            0xC1, 5, // program change: one data byte
            0x90, 0x80, 60, 0, // a note on cut off by a note off
            0xB0, 7, // cut short
        ];
        assert_eq!(
            channel_messages(&bytes),
            vec![(0x90, 60, 100), (0x90, 62, 90), (0xC1, 5, 0), (0x80, 60, 0)]
        );
    }
}
