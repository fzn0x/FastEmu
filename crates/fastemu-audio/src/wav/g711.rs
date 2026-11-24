//! ITU-T G.711 mu-law and A-law expansion to 16-bit linear samples.

/// Expands one mu-law byte.
pub(crate) fn mu_law(byte: u8) -> i16 {
    let u = !byte;
    let mut magnitude = ((i32::from(u & 0x0F)) << 3) + 0x84;
    magnitude <<= (u & 0x70) >> 4;
    let sample = if u & 0x80 == 0 {
        magnitude - 0x84
    } else {
        0x84 - magnitude
    };
    sample as i16
}

/// Expands one A-law byte.
pub(crate) fn a_law(byte: u8) -> i16 {
    let a = byte ^ 0x55;
    let mut magnitude = i32::from(a & 0x0F) << 4;
    let segment = (a & 0x70) >> 4;
    match segment {
        0 => magnitude += 8,
        1 => magnitude += 0x108,
        _ => {
            magnitude += 0x108;
            magnitude <<= segment - 1;
        }
    }
    let sample = if a & 0x80 == 0 { -magnitude } else { magnitude };
    sample as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_law_matches_the_reference_points() {
        assert_eq!(mu_law(0xFF), 0);
        assert_eq!(mu_law(0x7F), 0);
        assert_eq!(mu_law(0x80), 32124);
        assert_eq!(mu_law(0x00), -32124);
        assert_eq!(mu_law(0xEF), 132);
    }

    #[test]
    fn a_law_matches_the_reference_points() {
        assert_eq!(a_law(0xD5), 8);
        assert_eq!(a_law(0x55), -8);
        assert_eq!(a_law(0xAA), 32256);
        assert_eq!(a_law(0x2A), -32256);
    }

    #[test]
    fn both_laws_are_monotonic_per_sign() {
        // Over the positive codes, louder codes give larger samples.
        let mu: Vec<i16> = (0x80..=0xFFu8).rev().map(mu_law).collect();
        assert!(mu.windows(2).all(|w| w[0] <= w[1]));
        let a: Vec<i16> = (0..=0x7Fu8).map(|c| a_law((c | 0x80) ^ 0x55)).collect();
        assert!(a.windows(2).all(|w| w[0] <= w[1]));
    }
}
