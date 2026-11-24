//! What happened to a player or a Nokia sound, on its way to the game's
//! listener.

use fastemu_audio::{PlayerEvent, PlayerEventKind};

/// A player event, with the media time in microseconds where it has one.
///
/// It is the engine's [`PlayerEventKind`] plus the one event the MIDP
/// layer makes itself: `STOPPED_AT_TIME`, for `StopTimeControl`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Started(i64),
    Stopped(i64),
    StoppedAtTime(i64),
    /// The end of the media; `last` is false when another pass of a loop
    /// follows.
    EndOfMedia {
        time_us: i64,
        last: bool,
    },
    VolumeChanged,
    Closed,
    Error(String),
}

impl Event {
    /// Whether the event ends a run of the player (and so its hold).
    pub fn ends_a_run(&self) -> bool {
        matches!(
            self,
            Self::Stopped(_)
                | Self::StoppedAtTime(_)
                | Self::EndOfMedia { last: true, .. }
                | Self::Closed
        )
    }
}

impl From<PlayerEventKind> for Event {
    fn from(kind: PlayerEventKind) -> Self {
        match kind {
            PlayerEventKind::Started { media_time_us } => Self::Started(media_time_us),
            PlayerEventKind::Stopped { media_time_us } => Self::Stopped(media_time_us),
            PlayerEventKind::EndOfMedia {
                media_time_us,
                last,
            } => Self::EndOfMedia {
                time_us: media_time_us,
                last,
            },
            PlayerEventKind::VolumeChanged { .. } => Self::VolumeChanged,
            PlayerEventKind::Closed => Self::Closed,
            PlayerEventKind::Error { message } => Self::Error(message),
        }
    }
}

/// A message to the media event thread.
#[derive(Debug)]
pub(crate) enum Note {
    /// An event of an engine player, from the group's listener on any
    /// thread (often the audio thread).
    Engine(PlayerEvent),
    /// An event the MIDP layer made for the player or sound `serial`: from
    /// a silent player, a closed unrealized player, or a stop time.
    Midp(u64, Event),
    /// The Java object of `serial` was collected.
    Gone(u64),
    /// A timer changed; the event thread works out when to wake again.
    Wake,
}
