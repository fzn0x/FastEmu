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
