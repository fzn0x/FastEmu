//! The two ADPCM codecs WAV files carry: IMA (DVI, format 0x11) and
//! Microsoft (format 2). Both code 4 bits per sample in blocks that each
//! start with a header holding the decoder state, so a damaged block only
//! spoils itself.

const IMA_STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

const IMA_INDEX_STEP: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// One channel's IMA ADPCM decoder state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ImaState {
    pub predictor: i32,
    pub index: i32,
}

impl ImaState {
    /// Decodes one 4-bit code.
    pub fn decode(&mut self, code: u8) -> i16 {
        let step = IMA_STEPS[self.index as usize];
        let mut diff = step >> 3;
        if code & 1 != 0 {
            diff += step >> 2;
        }
        if code & 2 != 0 {
            diff += step >> 1;
        }
        if code & 4 != 0 {
            diff += step;
        }
        if code & 8 != 0 {
            diff = -diff;
        }
        self.predictor = (self.predictor + diff).clamp(-32768, 32767);
        self.index = (self.index + IMA_INDEX_STEP[usize::from(code & 7)]).clamp(0, 88);
        self.predictor as i16
    }
}

/// Decodes IMA ADPCM blocks of `block_align` bytes into interleaved
/// samples. A short last block is decoded as far as it goes.
pub(crate) fn decode_ima(
    data: &[u8],
    channels: usize,
    block_align: usize,
    samples_per_block: Option<usize>,
) -> Vec<i16> {
    let header = 4 * channels;
    let full = 1 + (block_align.saturating_sub(header)) * 2 / channels;
    let per_block = samples_per_block.map_or(full, |n| n.clamp(1, full));
    let mut out = Vec::with_capacity(data.len() / block_align.max(1) * per_block * channels);
    for block in data.chunks(block_align) {
        if block.len() < header {
            break;
        }
        let mut states: Vec<ImaState> = (0..channels)
            .map(|ch| {
                let h = &block[ch * 4..ch * 4 + 4];
                ImaState {
                    predictor: i32::from(i16::from_le_bytes([h[0], h[1]])),
                    index: i32::from(h[2]).min(88),
                }
            })
            .collect();
        let start = out.len();
        out.extend(states.iter().map(|s| s.predictor as i16));
        // After the headers, each channel in turn has 4 bytes holding
        // 8 samples, low nibble first.
        let body = &block[header..];
        let mut decoded = vec![0i16; 8 * channels];
        for group in body.chunks(4 * channels) {
            if group.len() < 4 * channels {
                break;
            }
            for (ch, state) in states.iter_mut().enumerate() {
                for (i, &byte) in group[ch * 4..ch * 4 + 4].iter().enumerate() {
                    decoded[(i * 2) * channels + ch] = state.decode(byte & 0x0F);
                    decoded[(i * 2 + 1) * channels + ch] = state.decode(byte >> 4);
                }
            }
            out.extend_from_slice(&decoded);
        }
        out.truncate(start + per_block.min((out.len() - start) / channels) * channels);
    }
    out
}

const MS_ADAPTATION: [i32; 16] = [
    230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230,
];

/// The predictor coefficient pairs every Microsoft ADPCM file uses unless
/// its format chunk lists its own.
pub(crate) const MS_STANDARD_COEFFICIENTS: [(i32, i32); 7] = [
    (256, 0),
    (512, -256),
    (0, 0),
    (192, 64),
    (240, 0),
    (460, -208),
    (392, -232),
];

#[derive(Clone, Copy, Debug, Default)]
struct MsState {
    coef1: i32,
    coef2: i32,
    delta: i32,
    sample1: i32,
    sample2: i32,
}

/// The largest step size kept. Damaged data can triple the step with
/// every code; capping it (as FFmpeg does) keeps the products below
/// inside `i32`.
const MS_MAX_DELTA: i32 = i32::MAX / 768;

impl MsState {
    fn decode(&mut self, code: u8) -> i16 {
        let signed = i32::from(code as i8) << 28 >> 28;
        // Coefficients listed in the file reach +-32768, so the two
        // products can sum past `i32`.
        let predicted = (i64::from(self.sample1) * i64::from(self.coef1)
            + i64::from(self.sample2) * i64::from(self.coef2))
            >> 8;
        let sample =
            (predicted + i64::from(signed) * i64::from(self.delta)).clamp(-32768, 32767) as i32;
        self.sample2 = self.sample1;
        self.sample1 = sample;
        self.delta =
            ((MS_ADAPTATION[usize::from(code & 0x0F)] * self.delta) >> 8).clamp(16, MS_MAX_DELTA);
        sample as i16
    }
}

/// Decodes Microsoft ADPCM blocks of `block_align` bytes into interleaved
/// samples.
pub(crate) fn decode_ms(
    data: &[u8],
    channels: usize,
    block_align: usize,
    samples_per_block: Option<usize>,
    coefficients: &[(i32, i32)],
) -> Vec<i16> {
    let header = 7 * channels;
    let full = 2 + (block_align.saturating_sub(header)) * 2 / channels;
    let per_block = samples_per_block.map_or(full, |n| n.clamp(2, full));
    let mut out = Vec::with_capacity(data.len() / block_align.max(1) * per_block * channels);
    let read_i16 = |b: &[u8], at: usize| i32::from(i16::from_le_bytes([b[at], b[at + 1]]));
    for block in data.chunks(block_align) {
        if block.len() < header {
            break;
        }
        let mut states: Vec<MsState> = (0..channels)
            .map(|ch| {
                let (coef1, coef2) = coefficients
                    .get(usize::from(block[ch]))
                    .copied()
                    .unwrap_or((256, 0));
                MsState {
                    coef1,
                    coef2,
                    delta: read_i16(block, channels + ch * 2),
                    sample1: read_i16(block, 3 * channels + ch * 2),
                    sample2: read_i16(block, 5 * channels + ch * 2),
                }
            })
            .collect();
        let start = out.len();
        out.extend(states.iter().map(|s| s.sample2 as i16));
        out.extend(states.iter().map(|s| s.sample1 as i16));
        // Nibbles run high first, and the channels take turns.
        let mut ch = 0;
        for &byte in &block[header..] {
            for code in [byte >> 4, byte & 0x0F] {
                out.push(states[ch].decode(code));
                ch = (ch + 1) % channels;
            }
        }
        let decoded = (out.len() - start) / channels;
        out.truncate(start + per_block.min(decoded) * channels);
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A plain IMA ADPCM encoder, so the tests can build real files.
    pub(crate) fn encode_ima_mono(samples: &[i16], block_align: usize) -> Vec<u8> {
        let per_block = 1 + (block_align - 4) * 2;
        let mut out = Vec::new();
        for chunk in samples.chunks(per_block) {
            let mut state = ImaState {
                predictor: i32::from(chunk[0]),
                index: 0,
            };
            out.extend_from_slice(&chunk[0].to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            let mut codes = Vec::new();
            for &sample in &chunk[1..] {
                let step = IMA_STEPS[state.index as usize];
                let mut diff = i32::from(sample) - state.predictor;
                let mut code = 0u8;
                if diff < 0 {
                    code = 8;
                    diff = -diff;
                }
                let mut threshold = step;
                for bit in [4u8, 2, 1] {
                    if diff >= threshold {
                        code |= bit;
                        diff -= threshold;
                    }
                    threshold >>= 1;
                }
                state.decode(code);
                codes.push(code);
            }
            codes.resize((block_align - 4) * 2, 0);
            out.extend(codes.chunks(2).map(|pair| pair[0] | (pair[1] << 4)));
        }
        out
    }

    #[test]
    fn ima_follows_a_ramp() {
        let ramp: Vec<i16> = (0..505).map(|i| (i * 40 - 10000) as i16).collect();
        let encoded = encode_ima_mono(&ramp, 256);
        let decoded = decode_ima(&encoded, 1, 256, None);
        assert_eq!(decoded.len(), 505);
        assert_eq!(decoded[0], ramp[0]);
        // ADPCM adapts within a few dozen samples and then tracks closely.
        for (i, (&want, &got)) in ramp.iter().zip(&decoded).enumerate().skip(40) {
            assert!((i32::from(want) - i32::from(got)).abs() < 200, "sample {i}");
        }
    }

    #[test]
    fn ima_stereo_interleaves_groups_of_eight() {
        // Two channel headers, then 4 bytes for the left and 4 for the right.
        let mut block = vec![0x10, 0x00, 0, 0, 0xF0, 0xFF, 0, 0];
        block.extend_from_slice(&[0x11; 4]);
        block.extend_from_slice(&[0x99; 4]);
        let decoded = decode_ima(&block, 2, 16, None);
        assert_eq!(decoded.len(), 9 * 2);
        assert_eq!((decoded[0], decoded[1]), (16, -16));
        // Code 1 raises the left; code 9 lowers the right.
        assert!(decoded[2] > 16 && decoded[3] < -16);
        assert!(decoded[16] > decoded[2] && decoded[17] < decoded[3]);
    }

    #[test]
    fn ms_adpcm_decodes_a_hand_built_block() {
        // Mono block: predictor 0 (coefficients 256, 0), delta 16,
        // sample1 100, sample2 50, then codes 1 and 0xF (-1).
        let block = [0, 16, 0, 100, 0, 50, 0, 0x1F];
        let decoded = decode_ms(&block, 1, 8, None, &MS_STANDARD_COEFFICIENTS);
        // sample2, sample1, then 100 + 1 * 16, then 116 - 16 (delta stays 16).
        assert_eq!(decoded, vec![50, 100, 116, 100]);
    }

    #[test]
    fn ms_adpcm_survives_damaged_blocks() {
        // Code 8 triples the step every sample: without a cap it leaves
        // i32 within a few dozen codes. Extreme listed coefficients and
        // samples overflow the prediction too.
        let mut block = vec![0, 0xFF, 0x7F, 0x00, 0x80, 0x00, 0x80];
        block.extend_from_slice(&[0x88; 120]);
        let decoded = decode_ms(&block, 1, block.len(), None, &[(-32768, -32768)]);
        assert_eq!(decoded.len(), 2 + 240);
    }

    #[test]
    fn ms_adpcm_clamps_bad_coefficient_indexes() {
        let block = [200, 16, 0, 10, 0, 0, 0, 0x00];
        let decoded = decode_ms(&block, 1, 8, None, &MS_STANDARD_COEFFICIENTS);
        assert_eq!(decoded, vec![0, 10, 10, 10]);
    }
}
