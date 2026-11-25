//! Sound clips for the vendor sound APIs (Siemens' `Melody`, Samsung's
//! `AudioClip`): players in the instance's sound group, made from bytes.
//!
//! Without a sound group (the headless runner, tests) there is no player
//! and the clip is silent; the vendor APIs then behave as if it played,
//! as MMAPI's silent players do.
//!
//! The clips are players of the same group as MMAPI's, so the group's
//! limits hold for them: a game that holds all the sound it may gets
//! `OutOfMemoryError`.

use fastemu_audio::{Format, Player};
use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, JResult};

use crate::media::events;
use crate::media::{make_sound, throw_audio};

/// A player for `bytes` in `format` (or the format its bytes show), in
/// the instance's sound group. `None` when the instance is silent or the
/// engine cannot play the data, which is logged. A full group throws
/// `OutOfMemoryError`.
pub(crate) fn open(
    ctx: &mut Ctx<'_>,
    bytes: Vec<u8>,
    format: Option<Format>,
) -> JResult<Option<Player>> {
    let Some(group) = events::group(ctx) else {
        return Ok(None);
    };
    let opened = make_sound(ctx, || match format {
        Some(format) => group.create_player_as(&bytes, format),
        None => group.create_player(&bytes, None),
    });
    match opened {
        Ok(player) => Ok(Some(player)),
        Err(error @ fastemu_audio::AudioError::OutOfMemory(_)) => throw_audio(ctx, error),
        Err(e) => {
            ctx.log(LogLevel::Warn, format!("a sound clip does not play: {e}"));
            Ok(None)
        }
    }
}

/// The instance's sound group plays `note` (a MIDI note number) for
/// `duration_ms`, at full volume, and forgets it.
pub(crate) fn tone(ctx: &mut Ctx<'_>, note: i32, duration_ms: i32) {
    if let Some(group) = events::group(ctx) {
        // Only a closed group (the instance ending) refuses, and then
        // silence is right.
        let _ = group.play_tone(note.clamp(0, 127), duration_ms.max(1), 100);
    }
}
