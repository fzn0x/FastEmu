//! Volume scales.

use std::sync::atomic::{AtomicU32, Ordering};

/// The amplitude for an MMAPI volume level from 0 to 100.
///
/// JSR-135 asks for a scale that is linear in loudness: level 50 should
/// sound half as loud as level 100. Half the loudness is about 10 dB
/// down, so the amplitude is `(level / 100) ^ (10 / 6.02)`, which gives
/// 0.316 at level 50 and silence at 0.
pub(crate) fn level_gain(level: u8) -> f32 {
    let level = f32::from(level.min(100)) / 100.0;
    level.powf(10.0 / 6.0206)
}

/// Clamps a gain from the app to 0.0 to 1.0. A NaN (from a broken slider
/// or a division by zero) is silence: kept as NaN it would make the
/// gain ramp slide without end and blow up the mix.
pub(crate) fn unit_gain(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

/// A gain that one thread sets and the audio thread reads, with no lock:
/// a group's volume, which the app sets from its UI thread. It holds the
/// bits of an `f32` already passed through [`unit_gain`], so it is never
/// NaN.
#[derive(Debug)]
pub(crate) struct SharedGain(AtomicU32);

impl SharedGain {
    pub(crate) fn new(gain: f32) -> Self {
        Self(AtomicU32::new(unit_gain(gain).to_bits()))
    }

    /// Relaxed is enough: the gain is one value on its own, and nothing
    /// else is published with it. The audio thread reads it for each
    /// slice, so a new value reaches the mix at its next slice.
    pub(crate) fn set(&self, gain: f32) {
        self.0.store(unit_gain(gain).to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_level_is_ten_decibels_down() {
        assert_eq!(level_gain(0), 0.0);
        assert!((level_gain(100) - 1.0).abs() < 1e-6);
        let db = 20.0 * level_gain(50).log10();
        assert!((db + 10.0).abs() < 0.01, "{db}");
        assert_eq!(level_gain(200), level_gain(100));
    }
}
