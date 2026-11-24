//! The media registry and the media event thread.
//!
//! Every Java `Player` and Nokia `Sound` gets a serial and an entry here.
//! Events reach the entry from two places, through one channel so they
//! keep their order: the instance's sound group (its listener runs on the
//! audio thread or on whichever thread called the engine) and the MIDP
//! layer itself (silent players, stop times). One Java thread per
//! instance, started on first use, reads the channel and calls
//! `PlayerListener.playerUpdate` and `SoundListener.soundStateChanged` in
//! order, never on the audio thread and never with the mixer locked.
//!
//! An entry finds its Java object through a `java.lang.ref.WeakReference`,
//! so players the game forgot can be collected (which closes their engine
//! player). While a player plays it is also held by a [`GlobalRef`]: a
//! sound the game started and dropped keeps playing and its listeners
//! still hear the end. The hold is released when the event thread
//! delivers an event that ends the run and the player is not playing
//! again; the engine delivers each player's events in order, so nothing
//! of that run can follow.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use fastemu_audio::{Engine, EngineConfig, Group, PlayerId};
use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, GlobalRef, JResult, ObjRef, Throw, Value};

use super::event::{Event, Note};
use super::player;
use super::track::Track;
use crate::nokia::sound;

/// The longest the event thread sleeps without looking at the VM's stop
/// flag.
const IDLE: Duration = Duration::from_millis(200);

/// How many unsupported sounds an instance logs before it goes quiet.
const MAX_WARNINGS: u32 = 16;

/// The Java class a registry entry belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    /// A `javax.microedition.media.Player`.
    Player,
    /// A `com.nokia.mid.sound.Sound`.
    Sound,
}

struct Entry {
    owner: Owner,
    /// A `java.lang.ref.WeakReference` to the object.
    weak: GlobalRef,
    /// The object itself while it plays or has events of its run to come.
    hold: Option<GlobalRef>,
}

/// The media state of one instance, a VM extension.
struct MediaState {
    notes: Sender<Note>,
    next_serial: u64,
    entries: HashMap<u64, Entry>,
    /// Which entry each engine player belongs to.
    links: HashMap<PlayerId, u64>,
    /// A sound group that is never heard, for working out the length of
    /// sounds when the instance has no audio engine.
    probe: Option<Group>,
    warnings: u32,
}

fn media<'c>(ctx: &'c mut Ctx<'_>) -> Option<&'c mut MediaState> {
    ctx.extension_mut::<MediaState>()
}

/// The instance's sound group, if the app gave it one.
pub(crate) fn group(ctx: &Ctx<'_>) -> Option<Group> {
    crate::env::try_env(ctx).and_then(|env| env.audio.clone())
}

/// A group whose players are never rendered, for decoding sound only to
/// learn its length.
pub(crate) fn probe(ctx: &mut Ctx<'_>) -> JResult<Group> {
    ensure(ctx)?;
    let state = media(ctx).expect("ensured");
    Ok(state
        .probe
        .get_or_insert_with(|| Engine::new(EngineConfig { sample_rate: 8000 }).create_group())
        .clone())
}

/// Sets up the registry, the group's listener and the event thread, once.
fn ensure(ctx: &mut Ctx<'_>) -> JResult<()> {
    if ctx.extension::<MediaState>().is_some() {
        return Ok(());
    }
    let (notes, receiver) = mpsc::channel();
    if let Some(group) = group(ctx) {
        let to_thread = notes.clone();
        group.set_listener(Some(Arc::new(move |event| {
            let _ = to_thread.send(Note::Engine(event));
        })));
    }
    ctx.set_extension(MediaState {
        notes,
        next_serial: 1,
        entries: HashMap::new(),
        links: HashMap::new(),
        probe: None,
        warnings: 0,
    });
    ctx.spawn_thread("media-events", move |ctx| run(ctx, &receiver))?;
    Ok(())
}

/// Registers `obj` and returns the track it keeps in its payload.
pub(crate) fn register(ctx: &mut Ctx<'_>, obj: ObjRef, owner: Owner) -> JResult<Track> {
    ensure(ctx)?;
    let weak = ctx.construct(
        "java/lang/ref/WeakReference",
        "(Ljava/lang/Object;)V",
        &[obj.into()],
    )?;
    let weak = ctx.new_global(weak);
    let state = media(ctx).expect("ensured");
    let serial = state.next_serial;
    state.next_serial += 1;
    state.entries.insert(
        serial,
        Entry {
            owner,
            weak,
            hold: None,
        },
    );
    Ok(Track::new(serial, Some(state.notes.clone())))
}

/// Keeps `obj` alive until the event thread delivers the end of its run:
/// call it when starting a sound and when closing one.
pub(crate) fn hold(ctx: &mut Ctx<'_>, serial: u64, obj: ObjRef) {
    let held = media(ctx)
        .and_then(|state| state.entries.get(&serial))
        .is_none_or(|entry| entry.hold.is_some());
    if held {
        return;
    }
    let global = ctx.new_global(obj);
    if let Some(entry) = media(ctx).and_then(|state| state.entries.get_mut(&serial)) {
        entry.hold = Some(global);
    }
}

/// Routes the events of engine player `id` to entry `serial`.
pub(crate) fn link(ctx: &mut Ctx<'_>, id: PlayerId, serial: u64) {
    if let Some(state) = media(ctx) {
        state.links.insert(id, serial);
    }
}

/// Tells the event thread a timer changed.
pub(crate) fn wake(ctx: &mut Ctx<'_>) {
    if let Some(state) = media(ctx) {
        let _ = state.notes.send(Note::Wake);
    }
}

/// Logs a sound FastEmu plays as silence: what it was said to be and how
/// it starts, so the format can be added later. Quiet after a few.
pub(crate) fn warn_silent(ctx: &mut Ctx<'_>, what: &str, data: &[u8]) {
    let Some(state) = media(ctx) else {
        return;
    };
    state.warnings += 1;
    let count = state.warnings;
    if count > MAX_WARNINGS {
        return;
    }
    let line = if data.is_empty() {
        format!("media: playing {what} as silence")
    } else {
        let head: Vec<String> = data.iter().take(16).map(|b| format!("{b:02x}")).collect();
        format!(
            "media: playing {what} as silence ({} bytes, starting {})",
            data.len(),
            head.join(" ")
        )
    };
    ctx.log(LogLevel::Warn, line);
    if count == MAX_WARNINGS {
        ctx.log(
            LogLevel::Warn,
            "media: more sounds play as silence; not logging them",
        );
    }
}

/// The event thread's body.
fn run(ctx: &mut Ctx<'_>, receiver: &Receiver<Note>) -> JResult<()> {
    loop {
        let next = carry_on(ctx, |ctx| timers(ctx, Instant::now()))?.flatten();
        let wait = next.map_or(IDLE, |at| {
            at.saturating_duration_since(Instant::now()).min(IDLE)
        });
        let first = ctx.without_lock(|| receiver.recv_timeout(wait));
        if ctx.is_stopping() {
            return Err(Throw::Stop);
        }
        // A safepoint: parks here while the VM is paused.
        ctx.yield_now()?;
        let first = match first {
            Ok(note) => Some(note),
            Err(RecvTimeoutError::Timeout) => None,
            // The registry holds a sender for the VM's whole life.
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        for note in first
            .into_iter()
            .chain(receiver.try_iter().collect::<Vec<_>>())
        {
            carry_on(ctx, |ctx| handle(ctx, note))?;
        }
    }
}

/// Runs one piece of the event thread's work in its own local frame. A
/// Java exception from it (an `OutOfMemoryError` making event data) is
/// logged and the thread goes on; only a stopping VM ends it.
fn carry_on<R>(
    ctx: &mut Ctx<'_>,
    f: impl FnOnce(&mut Ctx<'_>) -> JResult<R>,
) -> JResult<Option<R>> {
    match ctx.local_frame(f) {
        Ok(value) => Ok(Some(value)),
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(Throw::Exception(exception)) => {
            report_exception(ctx, "the media event thread", exception);
            Ok(None)
        }
    }
}

/// Lets every playing track do what time brought, and returns when the
/// next one wants to be woken.
fn timers(ctx: &mut Ctx<'_>, now: Instant) -> JResult<Option<Instant>> {
    let held: Vec<u64> = match media(ctx) {
        Some(state) => state
            .entries
            .iter()
            .filter(|(_, entry)| entry.hold.is_some())
            .map(|(&serial, _)| serial)
            .collect(),
        None => return Ok(None),
    };
    let mut next: Option<Instant> = None;
    for serial in held {
        let Some((obj, owner)) = held_object(ctx, serial) else {
            continue;
        };
        let track = track_mut(ctx, obj, owner)?;
        track.tick(now);
        if let Some(at) = track.deadline(now) {
            next = Some(next.map_or(at, |n| n.min(at)));
        }
    }
    Ok(next)
}

/// The held object of entry `serial`, rooted.
fn held_object(ctx: &mut Ctx<'_>, serial: u64) -> Option<(ObjRef, Owner)> {
    let entry = media(ctx)?.entries.remove(&serial)?;
    let obj = entry.hold.as_ref().map(|hold| ctx.global(hold));
    let owner = entry.owner;
    media(ctx)?.entries.insert(serial, entry);
    obj.map(|obj| (obj, owner))
}

/// The track in `obj`'s payload.
fn track_mut<'c>(ctx: &'c mut Ctx<'_>, obj: ObjRef, owner: Owner) -> JResult<&'c mut Track> {
    Ok(match owner {
        Owner::Player => &mut ctx.payload_mut::<player::PlayerData>(obj)?.track,
        Owner::Sound => &mut ctx.payload_mut::<sound::SoundData>(obj)?.track,
    })
}

fn handle(ctx: &mut Ctx<'_>, note: Note) -> JResult<()> {
    let (serial, event) = match note {
        Note::Engine(engine_event) => {
            let Some(state) = media(ctx) else {
                return Ok(());
            };
            let serial = if engine_event.kind == fastemu_audio::PlayerEventKind::Closed {
                state.links.remove(&engine_event.player)
            } else {
                state.links.get(&engine_event.player).copied()
            };
            let Some(serial) = serial else {
                return Ok(());
            };
            (serial, Event::from(engine_event.kind))
        }
        Note::Midp(serial, event) => (serial, event),
        Note::Gone(serial) => {
            if let Some(state) = media(ctx) {
                state.entries.remove(&serial);
            }
            return Ok(());
        }
        Note::Wake => return Ok(()),
    };
    deliver(ctx, serial, event)
}

/// Hands `event` to the listeners of entry `serial`, after releasing the
/// entry's hold if the event ends its run.
fn deliver(ctx: &mut Ctx<'_>, serial: u64, event: Event) -> JResult<()> {
    let Some(mut entry) = media(ctx).and_then(|state| state.entries.remove(&serial)) else {
        return Ok(());
    };
    let obj = match &entry.hold {
        Some(hold) => Some(ctx.global(hold)),
        None => {
            let weak = ctx.global(&entry.weak);
            ctx.call_virtual(weak, "get", "()Ljava/lang/Object;", &[])?
                .obj()
        }
    };
    // Collected: the entry goes, and its events with it.
    let Some(obj) = obj else {
        return Ok(());
    };
    let owner = entry.owner;
    if event.ends_a_run() && !track_mut(ctx, obj, owner)?.is_started(Instant::now()) {
        entry.hold = None;
    }
    // A closed player never plays again; a Nokia sound can be initialized
    // anew and keeps its entry.
    let keep = !(owner == Owner::Player && event == Event::Closed);
    if keep && let Some(state) = media(ctx) {
        state.entries.insert(serial, entry);
    }
    match owner {
        Owner::Player => player::deliver(ctx, obj, event),
        Owner::Sound => sound::deliver(ctx, obj, event),
    }
}

/// Logs an exception thrown in `place` (a listener); the event thread
/// carries on.
fn report_exception(ctx: &mut Ctx<'_>, place: &str, exception: ObjRef) {
    ctx.log(LogLevel::Err, format!("Exception in {place}:"));
    let _ = ctx.call_virtual(exception, "printStackTrace", "()V", &[]);
}

/// Calls a listener, catching what it throws.
pub(crate) fn call_listener(
    ctx: &mut Ctx<'_>,
    callback: &str,
    listener: ObjRef,
    name: &str,
    desc: &str,
    args: &[Value],
) -> JResult<()> {
    match ctx.call_interface(listener, name, desc, args) {
        Ok(_) => Ok(()),
        Err(Throw::Stop) => Err(Throw::Stop),
        Err(Throw::Exception(exception)) => {
            report_exception(ctx, callback, exception);
            Ok(())
        }
    }
}
