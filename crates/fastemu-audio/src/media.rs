//! Turns sound data into a playable [`Source`] for its format.

use std::sync::Arc;

use crate::error::AudioError;
use crate::format::Format;
use crate::midi::Synth;
use crate::midi::builtin::BuiltinSynth;
use crate::midi::sequencer::MidiSource;
use crate::midi::smf;
use crate::midi::soundfont::{SoundFont, SoundFontSynth};
use crate::mp3::Mp3Source;
use crate::pcm::PcmSource;
use crate::source::Source;
use crate::tone::{ToneEvent, ToneSource, sequence};
use crate::{amr, nokia, wav};

/// The lowest rate synthesized sources render at, whatever the output.
const MIN_SYNTH_RATE: u32 = 8000;

/// Decodes or parses `data` as `format`. Synthesized formats (MIDI and
/// tones) render at `rate`, the output rate when the player is created;
/// MIDI uses `soundfont` when one is set and the built-in synthesizer
/// otherwise.
pub(crate) fn open(
    data: &[u8],
    format: Format,
    rate: u32,
    soundfont: Option<&SoundFont>,
) -> Result<Box<dyn Source>, AudioError> {
    let rate = rate.max(MIN_SYNTH_RATE);
    let source: Box<dyn Source> = match format {
        Format::Wav => Box::new(PcmSource::new(Arc::new(wav::decode(data)?))),
        Format::Amr => Box::new(PcmSource::new(Arc::new(amr::decode(data)?))),
        Format::Mp3 => Box::new(Mp3Source::new(data)?),
        Format::Midi => {
            let sequence = Arc::new(smf::parse(data)?);
            let (synth, rate): (Box<dyn Synth>, u32) = match soundfont {
                Some(font) => (
                    Box::new(SoundFontSynth::new(font, rate)?),
                    SoundFontSynth::rate_for(rate),
                ),
                None => (Box::new(BuiltinSynth::new(rate)), rate),
            };
            Box::new(MidiSource::new(sequence, synth, rate))
        }
        Format::ToneSequence => tones(sequence::parse(data)?, rate),
        Format::NokiaRingtone => tones(nokia::parse_ringtone(data)?, rate),
    };
    Ok(source)
}

/// A source that plays a list of tones.
pub(crate) fn tones(events: Vec<ToneEvent>, rate: u32) -> Box<dyn Source> {
    Box::new(ToneSource::new(events.into(), rate.max(MIN_SYNTH_RATE)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_each_format() {
        let wav = crate::wav::tests::pcm16_wav(8000, 1, &[0, 100, 200]);
        assert_eq!(open(&wav, Format::Wav, 48000, None).unwrap().frames(), 3);
        let tone = [0xFE, 0x01, 69, 16];
        let source = open(&tone, Format::ToneSequence, 48000, None).unwrap();
        assert_eq!(source.frames(), 24000);
        let midi = crate::midi::smf::tests::smf(0, 96, &[&[0x00, 0xFF, 0x2F, 0x00]]);
        assert_eq!(open(&midi, Format::Midi, 1000, None).unwrap().rate(), 8000);
        assert!(open(b"junk", Format::Mp3, 48000, None).is_err());
        assert!(open(b"junk", Format::Amr, 48000, None).is_err());
        assert!(open(b"junk", Format::NokiaRingtone, 48000, None).is_err());
    }
}
