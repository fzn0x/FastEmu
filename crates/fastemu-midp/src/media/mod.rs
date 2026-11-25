//! `javax.microedition.media` (MMAPI, JSR-135): players and controls on
//! the instance's sound group.
//!
//! `docs/architecture/media.md` describes the design: how a `Player` maps
//! onto a `fastemu_audio::Player` or a silent stand-in, which thread runs
//! the listeners, and how players stay alive while they play.

mod api;
mod controls;
mod event;
pub(crate) mod events;
mod manager;
mod player;
mod protocol;
mod silent;
mod stream;
mod time_base;
pub(crate) mod track;

use fastemu_audio::AudioError;
use fastemu_jvm::{Ctx, JResult, NativeClass};

pub(crate) use event::Event;

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = api::classes();
    classes.extend(controls::classes());
    classes.extend(protocol::classes());
    classes.push(manager::class());
    classes.push(player::class());
    classes.push(time_base::class());
    classes
}

/// Makes a sound in the engine with the VM lock released: `make` decodes
/// and adds a player to the instance's group.
///
/// A game that holds all the sound its group allows gets one collection
/// and a second try before the engine's `OutOfMemory` reaches it. On a
/// phone the collector freed the memory of players nobody referenced, and
/// games rely on it: they make a player for each sound and never close
/// it, and nothing on the Java heap grows enough to start a collection
/// of its own. A game that really holds the sound is refused again.
pub(crate) fn make_sound<R>(
    ctx: &mut Ctx<'_>,
    mut make: impl FnMut() -> Result<R, AudioError>,
) -> Result<R, AudioError> {
    let made = ctx.without_lock(&mut make);
    if !matches!(made, Err(AudioError::OutOfMemory(_))) {
        return made;
    }
    ctx.gc();
    ctx.without_lock(&mut make)
}

/// Throws the Java exception JSR-135 names for an engine error:
/// `MediaException` for media the engine cannot handle or that is too
/// large, `IllegalArgumentException` for bad arguments and tone
/// sequences, `IllegalStateException` for calls in the wrong state, and
/// `OutOfMemoryError` when the game's sound memory is full.
pub(crate) fn throw_audio<T>(ctx: &mut Ctx<'_>, error: AudioError) -> JResult<T> {
    let class = match error {
        AudioError::Unsupported(_) | AudioError::Malformed { .. } | AudioError::TooLarge(_) => {
            api::MEDIA_EXCEPTION
        }
        AudioError::OutOfMemory(_) => "java/lang/OutOfMemoryError",
        AudioError::InvalidToneSequence { .. } | AudioError::InvalidArgument(_) => {
            "java/lang/IllegalArgumentException"
        }
        AudioError::InvalidState(_) | AudioError::Closed => "java/lang/IllegalStateException",
    };
    ctx.throw(class, &error.to_string())
}
