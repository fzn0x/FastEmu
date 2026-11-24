//! SoundFont playback through rustysynth, for users who load an `.sf2`.

use std::fmt;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use rustysynth::{Synthesizer, SynthesizerSettings};

use crate::error::AudioError;
use crate::format::Format;
use crate::midi::Synth;

/// A SoundFont 2 bank loaded for MIDI playback. Cloning it is cheap: the
/// samples are shared by every MIDI player that uses it.
#[derive(Clone)]
pub struct SoundFont {
    font: Arc<rustysynth::SoundFont>,
}

impl SoundFont {
    /// Reads a SoundFont from the bytes of an `.sf2` file.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AudioError> {
        let malformed = |reason: String| AudioError::Malformed {
            format: Format::Midi,
            reason,
        };
        // The reader indexes into data the file describes; a broken file
        // must not take the app down with it.
        let read = catch_unwind(AssertUnwindSafe(|| {
            rustysynth::SoundFont::new(&mut Cursor::new(bytes))
        }));
        match read {
            Ok(Ok(font)) => Ok(Self {
                font: Arc::new(font),
            }),
            Ok(Err(err)) => Err(malformed(format!("not a usable SoundFont: {err}"))),
            Err(_) => Err(malformed("not a usable SoundFont".into())),
        }
    }

    /// The bank's name, from its INFO chunk.
    pub fn name(&self) -> String {
        self.font.get_info().get_bank_name().to_owned()
    }

    /// How many presets (instruments and drum kits) the bank holds.
    pub fn preset_count(&self) -> usize {
        self.font.get_presets().len()
    }
}

impl fmt::Debug for SoundFont {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SoundFont")
            .field("name", &self.name())
            .field("presets", &self.preset_count())
            .finish()
    }
}

/// The rates rustysynth renders at.
const RATES: std::ops::RangeInclusive<u32> = 16_000..=192_000;

/// Drives rustysynth with one SoundFont.
pub(crate) struct SoundFontSynth {
    synth: Synthesizer,
}

impl SoundFontSynth {
    /// The rate the synthesizer renders at for an output rate: the same
    /// rate when rustysynth supports it.
    pub fn rate_for(output_rate: u32) -> u32 {
        if RATES.contains(&output_rate) {
            output_rate
        } else {
            44_100
        }
    }

    pub fn new(font: &SoundFont, rate: u32) -> Result<Self, AudioError> {
        let mut settings = SynthesizerSettings::new(Self::rate_for(rate) as i32);
        // Reverb and chorus cost more than the rest of the synthesizer,
        // and with many games playing at once that adds up.
        settings.enable_reverb_and_chorus = false;
        let synth =
            Synthesizer::new(&font.font, &settings).map_err(|err| AudioError::Malformed {
                format: Format::Midi,
                reason: format!("cannot start the SoundFont synthesizer: {err}"),
            })?;
        Ok(Self { synth })
    }
}

impl Synth for SoundFontSynth {
    fn message(&mut self, status: u8, data1: u8, data2: u8) {
        self.synth.process_midi_message(
            i32::from(status & 0x0F),
            i32::from(status & 0xF0),
            i32::from(data1),
            i32::from(data2),
        );
    }

    fn reset(&mut self) {
        self.synth.reset();
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_files_that_are_not_soundfonts() {
        assert!(matches!(
            SoundFont::from_bytes(b"RIFF\0\0\0\0sfbk"),
            Err(AudioError::Malformed { .. })
        ));
        assert!(SoundFont::from_bytes(&[]).is_err());
    }

    #[test]
    fn picks_a_rate_rustysynth_supports() {
        assert_eq!(SoundFontSynth::rate_for(48_000), 48_000);
        assert_eq!(SoundFontSynth::rate_for(8_000), 44_100);
    }
}
