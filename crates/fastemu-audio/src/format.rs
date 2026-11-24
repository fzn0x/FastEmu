//! Content types and format detection.
//!
//! Games pass a content type with their sound data, and many pass the
//! wrong one: WAV files labelled `audio/midi`, MIDI files labelled
//! `audio/x-wav`, or a made-up type. The engine trusts the bytes first:
//! [`Format::detect`] looks for each format's signature and falls back to
//! the declared type only when the data has none.

use crate::error::AudioError;

/// A kind of sound data the engine plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    /// RIFF WAVE: PCM, IEEE float, IMA and Microsoft ADPCM, mu-law, A-law.
    Wav,
    /// Standard MIDI files, format 0 and 1, bare or in a RIFF RMID wrapper.
    Midi,
    /// MPEG-1 and MPEG-2 audio layer III.
    Mp3,
    /// AMR narrowband in the RFC 4867 storage format (`#!AMR`).
    Amr,
    /// A JSR-135 `ToneControl` sequence.
    ToneSequence,
    /// A Nokia Smart Messaging ringing tone, as `com.nokia.mid.sound.Sound`
    /// takes with `FORMAT_TONE`.
    NokiaRingtone,
}

/// Every content type the engine accepts, for
/// `Manager.getSupportedContentTypes`.
pub const SUPPORTED_CONTENT_TYPES: &[&str] = &[
    "audio/x-wav",
    "audio/wav",
    "audio/midi",
    "audio/mid",
    "audio/x-midi",
    "audio/sp-midi",
    "audio/mpeg",
    "audio/mp3",
    "audio/amr",
    "audio/x-tone-seq",
];

impl Format {
    /// Every format, in the order [`Format::sniff`] tries them.
    pub const ALL: [Format; 6] = [
        Format::Wav,
        Format::Midi,
        Format::Amr,
        Format::Mp3,
        Format::ToneSequence,
        Format::NokiaRingtone,
    ];

    /// A short name for messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Wav => "WAV",
            Self::Midi => "MIDI",
            Self::Mp3 => "MP3",
            Self::Amr => "AMR",
            Self::ToneSequence => "tone sequence",
            Self::NokiaRingtone => "Nokia ringing tone",
        }
    }

    /// The content type `Player.getContentType` reports for this format.
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Wav => "audio/x-wav",
            Self::Midi => "audio/midi",
            Self::Mp3 => "audio/mpeg",
            Self::Amr => "audio/amr",
            Self::ToneSequence => "audio/x-tone-seq",
            Self::NokiaRingtone => "application/vnd.nokia.ringing-tone",
        }
    }

    /// The format a content type names. Case and parameters such as
    /// `; codecs=...` are ignored.
    pub fn from_content_type(content_type: &str) -> Option<Format> {
        let essence = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let format = match essence.as_str() {
            "audio/x-wav" | "audio/wav" | "audio/wave" | "audio/vnd.wave" => Self::Wav,
            "audio/midi" | "audio/mid" | "audio/x-midi" | "audio/sp-midi" => Self::Midi,
            "audio/mpeg" | "audio/mp3" | "audio/mpeg3" | "audio/x-mp3" | "audio/x-mpeg" => {
                Self::Mp3
            }
            "audio/amr" => Self::Amr,
            "audio/x-tone-seq" => Self::ToneSequence,
            "application/vnd.nokia.ringing-tone" => Self::NokiaRingtone,
            _ => return None,
        };
        Some(format)
    }

    /// The format whose signature `data` starts with, if any.
    pub fn sniff(data: &[u8]) -> Option<Format> {
        Self::ALL.into_iter().find(|format| format.matches(data))
    }

    /// The format to decode `data` as: the one its bytes show, or else
    /// the one `declared` names.
    pub fn detect(data: &[u8], declared: Option<&str>) -> Result<Format, AudioError> {
        if let Some(format) = Self::sniff(data) {
            return Ok(format);
        }
        match declared {
            Some(content_type) => Self::from_content_type(content_type)
                .ok_or_else(|| AudioError::Unsupported(format!("content type {content_type:?}"))),
            None => Err(AudioError::Unsupported(
                "the data matches no known sound format".into(),
            )),
        }
    }

    fn matches(self, data: &[u8]) -> bool {
        match self {
            Self::Wav => data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WAVE",
            Self::Midi => {
                data.starts_with(b"MThd")
                    || (data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"RMID")
            }
            Self::Amr => data.starts_with(b"#!AMR\n"),
            Self::Mp3 => data.starts_with(b"ID3") || is_mpeg_layer3_header(data),
            // VERSION (-2) followed by version 1.
            Self::ToneSequence => data.starts_with(&[0xFE, 0x01]),
            // A command count, then the 7-bit ringing-tone-programming
            // (0x25) or sound (0x1D) command.
            Self::NokiaRingtone => matches!(
                *data,
                [count, command, ..] if (1..=8).contains(&count) && matches!(command >> 1, 0x25 | 0x1D)
            ),
        }
    }
}

/// Whether `data` starts with a plausible MPEG audio layer III frame
/// header: the 11-bit sync, a known version, and a usable bitrate and
/// sample rate.
fn is_mpeg_layer3_header(data: &[u8]) -> bool {
    let [b0, b1, b2, ..] = *data else {
        return false;
    };
    let sync = b0 == 0xFF && b1 & 0xE0 == 0xE0;
    let version_known = (b1 >> 3) & 0b11 != 0b01;
    let layer3 = (b1 >> 1) & 0b11 == 0b01;
    let bitrate_valid = b2 >> 4 != 0b1111;
    let rate_valid = (b2 >> 2) & 0b11 != 0b11;
    sync && version_known && layer3 && bitrate_valid && rate_valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_types_ignore_case_and_parameters() {
        assert_eq!(Format::from_content_type("audio/X-WAV"), Some(Format::Wav));
        assert_eq!(
            Format::from_content_type(" audio/midi ; rate=1"),
            Some(Format::Midi)
        );
        assert_eq!(
            Format::from_content_type("audio/sp-midi"),
            Some(Format::Midi)
        );
        assert_eq!(Format::from_content_type("audio/mp3"), Some(Format::Mp3));
        assert_eq!(Format::from_content_type("video/mpeg"), None);
        for content_type in SUPPORTED_CONTENT_TYPES {
            assert!(Format::from_content_type(content_type).is_some());
        }
        for format in Format::ALL {
            assert_eq!(
                Format::from_content_type(format.content_type()),
                Some(format)
            );
        }
    }

    #[test]
    fn sniffing_finds_signatures() {
        assert_eq!(Format::sniff(b"RIFF\0\0\0\0WAVEfmt "), Some(Format::Wav));
        assert_eq!(Format::sniff(b"RIFF\0\0\0\0RMIDdata"), Some(Format::Midi));
        assert_eq!(Format::sniff(b"MThd\0\0\0\x06"), Some(Format::Midi));
        assert_eq!(Format::sniff(b"#!AMR\n\x3c"), Some(Format::Amr));
        assert_eq!(Format::sniff(b"ID3\x03\0"), Some(Format::Mp3));
        assert_eq!(Format::sniff(&[0xFF, 0xFB, 0x90, 0x00]), Some(Format::Mp3));
        assert_eq!(
            Format::sniff(&[0xFE, 0x01, 0xFD, 30]),
            Some(Format::ToneSequence)
        );
        assert_eq!(
            Format::sniff(&[0x02, 0x4A, 0x3A, 0x40]),
            Some(Format::NokiaRingtone)
        );
        assert_eq!(Format::sniff(b"hello"), None);
        assert_eq!(Format::sniff(&[]), None);
    }

    #[test]
    fn mpeg_headers_are_checked() {
        // Layer II and a reserved sample rate are rejected.
        assert!(!is_mpeg_layer3_header(&[0xFF, 0xFD, 0x90]));
        assert!(!is_mpeg_layer3_header(&[0xFF, 0xFB, 0x9C]));
        assert!(!is_mpeg_layer3_header(&[0xFF, 0xFB, 0xF0]));
        assert!(is_mpeg_layer3_header(&[0xFF, 0xF3, 0x40]));
    }

    #[test]
    fn detection_trusts_the_bytes_over_the_label() {
        let wav = b"RIFF\0\0\0\0WAVEfmt ";
        assert_eq!(Format::detect(wav, Some("audio/midi")), Ok(Format::Wav));
        assert_eq!(
            Format::detect(&[0x01, 0x02], Some("audio/x-tone-seq")),
            Ok(Format::ToneSequence)
        );
        assert!(matches!(
            Format::detect(b"zzz", Some("audio/x-unknown")),
            Err(AudioError::Unsupported(_))
        ));
        assert!(matches!(
            Format::detect(b"zzz", None),
            Err(AudioError::Unsupported(_))
        ));
    }
}
