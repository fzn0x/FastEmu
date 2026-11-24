//! `PlayerImpl`, the class of every `Player` `Manager` makes: the MMAPI
//! state machine over a [`Track`].
//!
//! UNREALIZED lives here (the sound data waits in [`Source`] until
//! `realize` decodes it, with the VM lock released); REALIZED,
//! PREFETCHED and STARTED are the track's; CLOSED is both. State changes
//! happen at once on the calling thread, so the state a game reads right
//! after `start()` is STARTED; listeners hear of them later, in order, on
//! the media event thread.

use std::sync::Arc;
use std::time::Instant;

use fastemu_audio::{Format, Group, PlayerState};
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

use super::api::{self, PLAYER_UPDATE, TIME_UNKNOWN, event_name};
use super::controls::{self, ControlKind};
use super::event::Event;
use super::events::{self, Owner};
use super::silent::{Silent, UNKNOWN_LENGTH_US};
use super::track::{Backend, Track};
use super::{throw_audio, time_base};

pub(crate) const PLAYER_IMPL: &str = "javax/microedition/media/PlayerImpl";
const ILLEGAL_STATE: &str = "java/lang/IllegalStateException";
const ILLEGAL_ARGUMENT: &str = "java/lang/IllegalArgumentException";

/// Where an unrealized player's sound comes from.
#[derive(Clone)]
pub(crate) enum Source {
    /// Sound data, the format it was found to be (`None`: none FastEmu
    /// plays) and the content type the game gave.
    Data {
        bytes: Arc<[u8]>,
        format: Option<Format>,
        declared: Option<String>,
    },
    /// `Manager.TONE_DEVICE_LOCATOR`: a tone player waiting for notes.
    ToneDevice,
}

/// What a `PlayerImpl` keeps beside its (empty) Java fields.
#[derive(Default)]
pub(crate) struct PlayerData {
    pub track: Track,
    /// The sound to decode on `realize`; dropped once realized or closed.
    source: Option<Source>,
    closed: bool,
    /// A loop count set while unrealized, applied on `realize`.
    loop_count: Option<i32>,
    listeners: Vec<ObjRef>,
    /// The controls handed out so far, one object per kind, so a game
    /// that asks twice gets the same one.
    controls: Vec<(ControlKind, ObjRef)>,
}

impl Payload for PlayerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &listener in &self.listeners {
            tracer.mark(listener);
        }
        for &(_, control) in &self.controls {
            tracer.mark(control);
        }
    }
}

impl PlayerData {
    /// The code `getState` returns.
    fn state_code(&mut self, now: Instant) -> i32 {
        if self.closed {
            0
        } else if !self.track.has_backend() {
            100
        } else {
            self.track.state(now).mmapi_code()
        }
    }

    fn is_closed(&mut self, now: Instant) -> bool {
        self.state_code(now) == 0
    }
}

/// A new unrealized player for `source`.
pub(crate) fn new_player(ctx: &mut Ctx<'_>, source: Source) -> JResult<ObjRef> {
    let class = ctx.find_class(PLAYER_IMPL)?;
    let obj = ctx.new_object(class)?;
    let track = events::register(ctx, obj, Owner::Player)?;
    let data = ctx.payload_mut::<PlayerData>(obj)?;
    data.track = track;
    data.source = Some(source);
    Ok(obj)
}

/// The player's payload, from a native's `this`.
fn data<'c>(ctx: &'c mut Ctx<'_>, this: ObjRef) -> JResult<&'c mut PlayerData> {
    ctx.payload_mut::<PlayerData>(this)
}

/// Throws `IllegalStateException` if the player is closed.
fn check_open(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    if data(ctx, this)?.is_closed(Instant::now()) {
        return ctx.throw(ILLEGAL_STATE, "the player is closed");
    }
    Ok(())
}

/// Throws `IllegalStateException` if the player is closed or unrealized.
fn check_realized(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    check_open(ctx, this)?;
    if !data(ctx, this)?.track.has_backend() {
        return ctx.throw(ILLEGAL_STATE, "the player is unrealized");
    }
    Ok(())
}

/// A player's sound once decoded, made with the VM lock released.
struct Opened {
    backend: Backend,
    format: Option<Format>,
    content_type: String,
    /// Why the sound plays as silence, when it does for a reason the
    /// creation did not already log.
    silenced: Option<String>,
}

impl Opened {
    fn silent(duration_us: i64, format: Option<Format>, content_type: String) -> Self {
        Self {
            backend: Backend::Silent(Silent::new(duration_us)),
            format,
            content_type,
            silenced: None,
        }
    }
}

/// Decodes `source` into the instance's group, or, without one, decodes
/// it in `probe` only to learn its length for a silent player.
fn open(source: &Source, group: Option<&Group>, probe: Option<&Group>) -> Opened {
    match source {
        Source::ToneDevice => match group.map(Group::create_tone_player) {
            Some(Ok(player)) => Opened {
                backend: Backend::Engine(player),
                format: Some(Format::ToneSequence),
                content_type: Format::ToneSequence.content_type().to_owned(),
                silenced: None,
            },
            _ => Opened::silent(
                0,
                Some(Format::ToneSequence),
                Format::ToneSequence.content_type().to_owned(),
            ),
        },
        Source::Data {
            format: None,
            declared,
            ..
        } => Opened::silent(
            UNKNOWN_LENGTH_US,
            None,
            declared
                .clone()
                .filter(|t| !t.is_empty())
                .unwrap_or_else(|| "application/octet-stream".to_owned()),
        ),
        Source::Data {
            bytes,
            format: Some(format),
            ..
        } => {
            let content_type = format.content_type().to_owned();
            let made = match (group, probe) {
                (Some(group), _) => group.create_player_as(bytes, *format),
                (None, Some(probe)) => probe.create_player_as(bytes, *format),
                (None, None) => {
                    return Opened::silent(UNKNOWN_LENGTH_US, Some(*format), content_type);
                }
            };
            match made {
                Ok(player) if group.is_some() => Opened {
                    backend: Backend::Engine(player),
                    format: Some(*format),
                    content_type,
                    silenced: None,
                },
                Ok(player) => {
                    let duration = player.duration().unwrap_or(UNKNOWN_LENGTH_US);
                    Opened::silent(duration, Some(*format), content_type)
                }
                // The group closed: the instance is ending, and silence is
                // all it needs.
                Err(fastemu_audio::AudioError::Closed) => {
                    Opened::silent(UNKNOWN_LENGTH_US, Some(*format), content_type)
                }
                Err(error) => Opened {
                    silenced: Some(format!("{} that does not decode ({error})", format.name())),
                    ..Opened::silent(UNKNOWN_LENGTH_US, Some(*format), content_type)
                },
            }
        }
    }
}

/// `realize`: decodes the sound with the VM lock released. Sound FastEmu
/// cannot play becomes a silent player, never an exception.
pub(crate) fn realize(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    check_open(ctx, this)?;
    let player = data(ctx, this)?;
    if player.track.has_backend() {
        return Ok(());
    }
    let Some(source) = player.source.clone() else {
        return Ok(());
    };
    let group = events::group(ctx);
    let probe = match group {
        Some(_) => None,
        None => Some(events::probe(ctx)?),
    };
    let opened = ctx.without_lock(|| open(&source, group.as_ref(), probe.as_ref()));
    let player = data(ctx, this)?;
    // Closed meanwhile, or realized by another thread: this sound goes.
    if player.closed || player.track.has_backend() {
        return Ok(());
    }
    player.source = None;
    player
        .track
        .install(opened.backend, opened.format, opened.content_type);
    if let Some(count) = player.loop_count.take() {
        let _ = player.track.set_loop_count(count);
    }
    let serial = player.track.serial();
    if let Some(id) = player.track.engine_id() {
        events::link(ctx, id, serial);
    }
    if let (Some(why), Source::Data { bytes, .. }) = (opened.silenced, &source) {
        events::warn_silent(ctx, &why, bytes);
    }
    Ok(())
}

fn prefetch(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    realize(ctx, this)?;
    let result = data(ctx, this)?.track.prefetch();
    result.or_else(|e| throw_audio(ctx, e))
}

fn start(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    prefetch(ctx, this)?;
    let serial = data(ctx, this)?.track.serial();
    events::hold(ctx, serial, this);
    let result = data(ctx, this)?.track.start(Instant::now());
    events::wake(ctx);
    result.or_else(|e| throw_audio(ctx, e))
}

fn close(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<()> {
    let player = data(ctx, this)?;
    if player.closed {
        return Ok(());
    }
    let serial = player.track.serial();
    events::hold(ctx, serial, this);
    let player = data(ctx, this)?;
    player.closed = true;
    player.source = None;
    if player.track.has_backend() {
        player.track.close();
    } else {
        player.track.announce(Event::Closed);
    }
    Ok(())
}

/// The control of `kind` the player offers, made on first use.
pub(crate) fn control(ctx: &mut Ctx<'_>, this: ObjRef, kind: ControlKind) -> JResult<ObjRef> {
    if let Some(&(_, control)) = data(ctx, this)?.controls.iter().find(|(k, _)| *k == kind) {
        return Ok(ctx.root(control));
    }
    let control = controls::new_control(ctx, kind, this)?;
    data(ctx, this)?.controls.push((kind, control));
    Ok(control)
}

/// The kinds of control the player offers, in `getControls` order.
fn offered(track: &Track) -> Vec<ControlKind> {
    let mut kinds = vec![ControlKind::Volume];
    match track.format() {
        Some(Format::ToneSequence) => kinds.push(ControlKind::Tone),
        Some(Format::Midi) => kinds.push(ControlKind::Midi),
        _ => {}
    }
    kinds.push(ControlKind::StopTime);
    kinds
}

fn get_control(ctx: &mut Ctx<'_>, this: ObjRef, name: Value) -> JResult<Value> {
    let Some(name) = ctx.opt_string(name)? else {
        return ctx.throw(ILLEGAL_ARGUMENT, "the control type is null");
    };
    check_realized(ctx, this)?;
    let short = name
        .strip_prefix("javax.microedition.media.control.")
        .unwrap_or(&name);
    let wanted = offered(&data(ctx, this)?.track)
        .into_iter()
        .find(|kind| kind.interface_name() == short);
    match wanted {
        Some(kind) => Ok(control(ctx, this, kind)?.into()),
        None => Ok(Value::NULL),
    }
}

fn get_controls(ctx: &mut Ctx<'_>, this: ObjRef) -> JResult<Value> {
    check_realized(ctx, this)?;
    let kinds = offered(&data(ctx, this)?.track);
    let class = ctx.find_class(api::CONTROL)?;
    let array = ctx.new_object_array(class, kinds.len() as i32)?;
    for (i, kind) in kinds.into_iter().enumerate() {
        let control = control(ctx, this, kind)?;
        ctx.array_mut::<Option<ObjRef>>(array)?[i] = Some(control);
    }
    Ok(array.into())
}

fn set_time_base(ctx: &mut Ctx<'_>, this: ObjRef, base: Value) -> JResult<()> {
    check_realized(ctx, this)?;
    if data(ctx, this)?.track.is_started(Instant::now()) {
        return ctx.throw(ILLEGAL_STATE, "the player is started");
    }
    match base.obj() {
        Some(base) if !time_base::is_system(ctx, base)? => ctx.throw(
            api::MEDIA_EXCEPTION,
            "only the system time base is supported",
        ),
        _ => Ok(()),
    }
}

fn add_listener(ctx: &mut Ctx<'_>, this: ObjRef, listener: Value) -> JResult<()> {
    check_open(ctx, this)?;
    if let Some(listener) = listener.obj() {
        let listeners = &mut data(ctx, this)?.listeners;
        if !listeners.contains(&listener) {
            listeners.push(listener);
        }
    }
    Ok(())
}

fn remove_listener(ctx: &mut Ctx<'_>, this: ObjRef, listener: Value) -> JResult<()> {
    check_open(ctx, this)?;
    if let Some(listener) = listener.obj() {
        data(ctx, this)?.listeners.retain(|&l| l != listener);
    }
    Ok(())
}

/// Hands `event` to the player's listeners (on the media event thread).
pub(crate) fn deliver(ctx: &mut Ctx<'_>, player: ObjRef, event: Event) -> JResult<()> {
    let data = data(ctx, player)?;
    let event = match event {
        Event::Stopped(time) if data.track.take_stopped_at_time(time) => Event::StoppedAtTime(time),
        other => other,
    };
    let listeners = data.listeners.clone();
    if listeners.is_empty() {
        return Ok(());
    }
    // A listener may remove another one, which a collection could then
    // free before its turn.
    for &listener in &listeners {
        ctx.root(listener);
    }
    let (name, event_data) = match event {
        Event::Started(time) => (event_name::STARTED, long(ctx, time)?),
        Event::Stopped(time) => (event_name::STOPPED, long(ctx, time)?),
        Event::StoppedAtTime(time) => (event_name::STOPPED_AT_TIME, long(ctx, time)?),
        Event::EndOfMedia { time_us, .. } => (event_name::END_OF_MEDIA, long(ctx, time_us)?),
        Event::VolumeChanged => (
            event_name::VOLUME_CHANGED,
            control(ctx, player, ControlKind::Volume)?.into(),
        ),
        Event::Closed => (event_name::CLOSED, Value::NULL),
        Event::Error(message) => (event_name::ERROR, ctx.new_string(&message)?.into()),
    };
    // The interned string, so `event == PlayerListener.STARTED` holds.
    let name: Vec<u16> = name.encode_utf16().collect();
    let name = ctx.intern(&name);
    for listener in listeners {
        events::call_listener(
            ctx,
            "PlayerListener.playerUpdate",
            listener,
            "playerUpdate",
            PLAYER_UPDATE,
            &[player.into(), name.into(), event_data],
        )?;
    }
    Ok(())
}

fn long(ctx: &mut Ctx<'_>, value: i64) -> JResult<Value> {
    Ok(ctx
        .construct("java/lang/Long", "(J)V", &[Value::Long(value)])?
        .into())
}

/// Runs a native on `this` that returns nothing.
fn void(
    f: impl Fn(&mut Ctx<'_>, ObjRef) -> JResult<()> + Send + Sync + 'static,
) -> impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static {
    move |ctx, a| {
        let this = ctx.non_null(a[0])?;
        f(ctx, this)?;
        Ok(Value::Void)
    }
}

pub(crate) fn class() -> NativeClass {
    NativeClass::new(PLAYER_IMPL)
        .implements(api::PLAYER)
        .final_class()
        .payload::<PlayerData>()
        .method("realize", "()V", void(realize))
        .method("prefetch", "()V", void(prefetch))
        .method("start", "()V", void(start))
        .method(
            "stop",
            "()V",
            void(|ctx, this| {
                check_open(ctx, this)?;
                let track = &mut data(ctx, this)?.track;
                if track.has_backend() {
                    let result = track.stop(Instant::now());
                    result.or_else(|e| throw_audio(ctx, e))?;
                }
                Ok(())
            }),
        )
        .method(
            "deallocate",
            "()V",
            void(|ctx, this| {
                check_open(ctx, this)?;
                let track = &mut data(ctx, this)?.track;
                if track.has_backend() {
                    let result = track.deallocate(Instant::now());
                    result.or_else(|e| throw_audio(ctx, e))?;
                }
                Ok(())
            }),
        )
        .method("close", "()V", void(close))
        .method("setLoopCount", "(I)V", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let count = a[1].int();
            check_open(ctx, this)?;
            if count == 0 {
                return ctx.throw(ILLEGAL_ARGUMENT, "a loop count of 0");
            }
            let player = data(ctx, this)?;
            if !player.track.has_backend() {
                player.loop_count = Some(count);
                return Ok(Value::Void);
            }
            let result = player.track.set_loop_count(count);
            result.or_else(|e| throw_audio(ctx, e))?;
            Ok(Value::Void)
        })
        .method("getMediaTime", "()J", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            check_open(ctx, this)?;
            let track = &mut data(ctx, this)?.track;
            if !track.has_backend() {
                return Ok(Value::Long(0));
            }
            let result = track.media_time(Instant::now());
            Ok(Value::Long(result.or_else(|e| throw_audio(ctx, e))?))
        })
        .method("setMediaTime", "(J)J", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            check_realized(ctx, this)?;
            let result = data(ctx, this)?
                .track
                .set_media_time(a[1].long(), Instant::now());
            events::wake(ctx);
            Ok(Value::Long(result.or_else(|e| throw_audio(ctx, e))?))
        })
        .method("getDuration", "()J", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            check_open(ctx, this)?;
            let track = &mut data(ctx, this)?.track;
            if !track.has_backend() {
                return Ok(Value::Long(TIME_UNKNOWN));
            }
            let result = track.duration();
            Ok(Value::Long(result.or_else(|e| throw_audio(ctx, e))?))
        })
        .method("getContentType", "()Ljava/lang/String;", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            check_realized(ctx, this)?;
            let content_type = data(ctx, this)?.track.content_type().to_owned();
            Ok(ctx.new_string(&content_type)?.into())
        })
        .method("getState", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(Value::Int(data(ctx, this)?.state_code(Instant::now())))
        })
        .method(
            "addPlayerListener",
            "(Ljavax/microedition/media/PlayerListener;)V",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                add_listener(ctx, this, a[1])?;
                Ok(Value::Void)
            },
        )
        .method(
            "removePlayerListener",
            "(Ljavax/microedition/media/PlayerListener;)V",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                remove_listener(ctx, this, a[1])?;
                Ok(Value::Void)
            },
        )
        .method(
            "getControl",
            "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                get_control(ctx, this, a[1])
            },
        )
        .method(
            "getControls",
            "()[Ljavax/microedition/media/Control;",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                get_controls(ctx, this)
            },
        )
        .method(
            "setTimeBase",
            "(Ljavax/microedition/media/TimeBase;)V",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                set_time_base(ctx, this, a[1])?;
                Ok(Value::Void)
            },
        )
        .method(
            "getTimeBase",
            "()Ljavax/microedition/media/TimeBase;",
            |ctx, a| {
                let this = ctx.non_null(a[0])?;
                check_realized(ctx, this)?;
                Ok(time_base::system(ctx)?.into())
            },
        )
}

/// Whether the player is in a state where a control that needs a
/// prefetched player works: prefetched or started.
pub(crate) fn is_prefetched(ctx: &mut Ctx<'_>, player: ObjRef) -> JResult<bool> {
    let state = data(ctx, player)?.state_code(Instant::now());
    Ok(state == PlayerState::Prefetched.mmapi_code() || state == PlayerState::Started.mmapi_code())
}

/// The player's track, for its controls.
pub(crate) fn track<'c>(ctx: &'c mut Ctx<'_>, player: ObjRef) -> JResult<&'c mut Track> {
    Ok(&mut data(ctx, player)?.track)
}

/// Whether the player is closed, for its controls.
pub(crate) fn is_closed(ctx: &mut Ctx<'_>, player: ObjRef) -> JResult<bool> {
    Ok(data(ctx, player)?.is_closed(Instant::now()))
}
