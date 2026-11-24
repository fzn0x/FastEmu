use std::fmt;

use crate::format::Format;

/// Why the engine refused a request.
///
/// The variants line up with the exceptions MMAPI throws, so the MIDP
/// layer can map them one to one: [`Unsupported`](Self::Unsupported) and
/// [`Malformed`](Self::Malformed) become `MediaException`,
/// [`InvalidToneSequence`](Self::InvalidToneSequence) and
/// [`InvalidArgument`](Self::InvalidArgument) become
/// `IllegalArgumentException`, and [`InvalidState`](Self::InvalidState) and
/// [`Closed`](Self::Closed) become `IllegalStateException`.
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
    }
}
