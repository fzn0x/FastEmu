//! Volume scales.

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
