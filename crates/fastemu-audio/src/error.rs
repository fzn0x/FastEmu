use std::fmt;

use crate::format::Format;

/// Why the engine refused a request.
///
/// The variants line up with the exceptions MMAPI throws, so the MIDP
/// layer can map them one to one: [`Unsupported`](Self::Unsupported) and
/// [`Malformed`](Self::Malformed) become `MediaException`,
/// [`InvalidToneSequence`](Self::InvalidToneSequence) and
/// [`InvalidArgument`](Self::InvalidArgument) become
/// `IllegalArgumentException`, [`InvalidState`](Self::InvalidState) and
/// [`Closed`](Self::Closed) become `IllegalStateException`,
/// [`TooLarge`](Self::TooLarge) and [`Busy`](Self::Busy) become
/// `MediaException` too, and [`OutOfMemory`](Self::OutOfMemory) becomes
/// `OutOfMemoryError`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioError {
    /// The data is in a format the engine cannot play, or the content
    /// type names one, and the bytes do not look like anything it knows.
    Unsupported(String),
    /// The data claims a supported format but cannot be decoded.
    Malformed { format: Format, reason: String },
    /// A JSR-135 tone sequence breaks the ToneControl grammar. `offset`
    /// is the index of the byte where reading stopped.
    InvalidToneSequence { offset: usize, reason: String },
    /// An argument is out of range, such as a loop count of zero or a
    /// tone note above 127.
    InvalidArgument(String),
    /// The call is not allowed in the player's current state, such as
    /// changing the loop count while it plays.
    InvalidState(String),
    /// The player or its group was closed.
    Closed,
    /// One sound is too big for any game to hold: it decodes to more than
    /// 16 MiB, or alone it is more than a group's whole sound budget.
    /// Trying again cannot help.
    TooLarge(String),
    /// The game's group holds as much sound, or as many players, as it
    /// may. Closing players frees room, so a later try can work.
    OutOfMemory(String),
    /// The engine plays as many players as it will, for this game or for
    /// every game together, so this one cannot start now. Stopping one,
    /// or letting one end, makes room. MMAPI's `start` calls that
    /// `MediaException`: the player cannot be started.
    Busy(String),
}

impl AudioError {
    /// Whether the engine refused for size: [`TooLarge`](Self::TooLarge)
    /// or [`OutOfMemory`](Self::OutOfMemory). A decoder that cannot read
    /// the data is not a size limit.
    pub fn is_size_limit(&self) -> bool {
        matches!(self, Self::TooLarge(_) | Self::OutOfMemory(_))
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "unsupported media: {what}"),
            Self::Malformed { format, reason } => {
                write!(f, "cannot decode {}: {reason}", format.name())
            }
            Self::InvalidToneSequence { offset, reason } => {
                write!(f, "invalid tone sequence at byte {offset}: {reason}")
            }
            Self::InvalidArgument(why) => write!(f, "invalid argument: {why}"),
            Self::InvalidState(why) => write!(f, "not allowed now: {why}"),
            Self::Closed => f.write_str("the player is closed"),
            Self::TooLarge(what) => write!(f, "sound too large: {what}"),
            Self::OutOfMemory(what) => write!(f, "out of sound memory: {what}"),
            Self::Busy(what) => write!(f, "too many players are started: {what}"),
        }
    }
}

impl std::error::Error for AudioError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_read_well() {
        assert_eq!(
            AudioError::Malformed {
                format: Format::Wav,
                reason: "no data chunk".into()
            }
            .to_string(),
            "cannot decode WAV: no data chunk"
        );
        assert_eq!(
            AudioError::InvalidToneSequence {
                offset: 3,
                reason: "bad tempo".into()
            }
            .to_string(),
            "invalid tone sequence at byte 3: bad tempo"
        );
        assert_eq!(AudioError::Closed.to_string(), "the player is closed");
        assert_eq!(
            AudioError::TooLarge("a WAV of 20 MiB".into()).to_string(),
            "sound too large: a WAV of 20 MiB"
        );
        assert_eq!(
            AudioError::OutOfMemory("128 players".into()).to_string(),
            "out of sound memory: 128 players"
        );
        assert_eq!(
            AudioError::Busy("a game may start 32".into()).to_string(),
            "too many players are started: a game may start 32"
        );
    }

    #[test]
    fn only_the_size_errors_are_size_limits() {
        assert!(AudioError::TooLarge(String::new()).is_size_limit());
        assert!(AudioError::OutOfMemory(String::new()).is_size_limit());
        assert!(!AudioError::Closed.is_size_limit());
        assert!(!AudioError::Busy(String::new()).is_size_limit());
        assert!(!AudioError::Unsupported(String::new()).is_size_limit());
    }
}
