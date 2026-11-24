//! RIFF WAVE decoding.
//!
//! Phone games ship WAV files from many tools, and plenty of them write
//! wrong sizes: a RIFF size that ignores the file, a data size of zero or
//! past the end, odd chunks without their pad byte. The decoder takes the
//! bytes that are there instead of trusting the sizes.

mod adpcm;
mod g711;

use crate::error::AudioError;
use crate::format::Format;
use crate::pcm::Pcm;

const PCM: u16 = 0x0001;
const MS_ADPCM: u16 = 0x0002;
const IEEE_FLOAT: u16 = 0x0003;
const A_LAW: u16 = 0x0006;
const MU_LAW: u16 = 0x0007;
const IMA_ADPCM: u16 = 0x0011;
const EXTENSIBLE: u16 = 0xFFFE;

/// The parts of a `fmt ` chunk the decoders use.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FormatChunk {
    tag: u16,
    channels: u16,
    rate: u32,
    block_align: u16,
    bits: u16,
    /// The bytes after the 16-byte base, for codecs that keep settings
    /// there (ADPCM, the extensible format's real tag).
    extra: Vec<u8>,
}

fn malformed(reason: impl Into<String>) -> AudioError {
    AudioError::Malformed {
        format: Format::Wav,
        reason: reason.into(),
    }
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// Whether four bytes could be a chunk id: printable ASCII.
fn is_chunk_id(id: &[u8]) -> bool {
    id.len() == 4 && id.iter().all(|b| (0x20..0x7F).contains(b))
}

/// Decodes a whole WAV file.
pub(crate) fn decode(data: &[u8]) -> Result<Pcm, AudioError> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err(malformed("not a RIFF WAVE file"));
    }
    let mut format = None;
    let mut samples: Option<&[u8]> = None;
    let mut at = 12;
    while at + 8 <= data.len() {
        let id = &data[at..at + 4];
        if !is_chunk_id(id) {
            break;
        }
        let declared = u32_at(data, at + 4).unwrap_or(0) as usize;
        let body_start = at + 8;
        let available = data.len() - body_start;
        let size = declared.min(available);
        let body = &data[body_start..body_start + size];
        match id {
            b"fmt " => format = Some(parse_format(body)?),
            // Streaming writers leave the data size at 0 or 0xFFFFFFFF;
            // either way the samples run to the end of the file.
            b"data" if declared == 0 || declared > available => {
                samples = Some(&data[body_start..]);
            }
            b"data" => samples = Some(body),
            _ => {}
        }
        if samples.is_some() && format.is_some() {
            break;
        }
        let mut next = body_start + size;
        // Chunks pad to an even size, but some writers skip the pad byte:
        // step over it only when no chunk starts right away.
        if size % 2 == 1 && !data.get(next..next + 4).is_some_and(is_chunk_id) {
            next += 1;
        }
        at = next;
    }
    let format = format.ok_or_else(|| malformed("no fmt chunk"))?;
    let samples = match samples {
        Some(samples) => samples,
        // The chunk walk got lost; look for the data chunk by name.
        None => find_data(data).ok_or_else(|| malformed("no data chunk"))?,
    };
    decode_samples(&format, samples)
}

fn find_data(data: &[u8]) -> Option<&[u8]> {
    let at = data.windows(4).position(|w| w == b"data")?;
    data.get(at + 8..)
}

fn parse_format(body: &[u8]) -> Result<FormatChunk, AudioError> {
    if body.len() < 14 {
        return Err(malformed("fmt chunk too short"));
    }
    let channels = u16_at(body, 2).unwrap_or(0);
    let block_align = u16_at(body, 12).unwrap_or(0);
    let bits = u16_at(body, 14).unwrap_or_else(|| {
        // WAVEFORMAT without the bits field: derive them from the block.
        (block_align / channels.max(1)) * 8
    });
    let mut format = FormatChunk {
        tag: u16_at(body, 0).unwrap_or(0),
        channels,
        rate: u32_at(body, 4).unwrap_or(0),
        block_align,
        bits,
        extra: body.get(18..).unwrap_or_default().to_vec(),
    };
    if format.tag == EXTENSIBLE {
        // Valid bits, channel mask, then the sub-format GUID whose first
        // two bytes are the real tag.
        format.tag = u16_at(&format.extra, 6)
            .ok_or_else(|| malformed("extensible fmt chunk without a sub-format"))?;
    }
    if format.channels == 0 {
        return Err(malformed("zero channels"));
    }
    if format.rate == 0 {
        return Err(malformed("zero sample rate"));
    }
    Ok(format)
}

fn decode_samples(format: &FormatChunk, data: &[u8]) -> Result<Pcm, AudioError> {
    let channels = usize::from(format.channels);
    let samples = match format.tag {
        PCM => decode_pcm(format, data)?,
        IEEE_FLOAT => decode_float(format, data)?,
        MU_LAW => data.iter().map(|&b| g711::mu_law(b)).collect(),
        A_LAW => data.iter().map(|&b| g711::a_law(b)).collect(),
        IMA_ADPCM => {
            let block_align = adpcm_block_align(format, 4 * channels + 4)?;
            let per_block = u16_at(&format.extra, 0).map(usize::from);
            adpcm::decode_ima(data, channels, block_align, per_block)
        }
        MS_ADPCM => {
            let block_align = adpcm_block_align(format, 7 * channels + 1)?;
            let per_block = u16_at(&format.extra, 0).map(usize::from);
            let listed = usize::from(u16_at(&format.extra, 2).unwrap_or(0));
            let coefficients: Vec<(i32, i32)> = (0..listed)
                .map_while(|i| {
                    let at = 4 + i * 4;
                    let c1 = u16_at(&format.extra, at)? as i16;
                    let c2 = u16_at(&format.extra, at + 2)? as i16;
                    Some((i32::from(c1), i32::from(c2)))
                })
                .collect();
            let coefficients = if coefficients.is_empty() {
                adpcm::MS_STANDARD_COEFFICIENTS.to_vec()
            } else {
                coefficients
            };
            adpcm::decode_ms(data, channels, block_align, per_block, &coefficients)
        }
        tag => {
            return Err(AudioError::Unsupported(format!(
                "WAV sample format 0x{tag:04X}"
            )));
        }
    };
    Ok(fold_to_stereo(format.rate, channels, samples))
}

/// The block size of an ADPCM stream, which must at least hold the
/// headers and one group of codes.
fn adpcm_block_align(format: &FormatChunk, minimum: usize) -> Result<usize, AudioError> {
    let block_align = usize::from(format.block_align);
    if block_align < minimum {
        return Err(malformed(format!("ADPCM block size {block_align}")));
    }
    Ok(block_align)
}

/// Bytes each sample takes: from the block size when it agrees with the
/// channel count, else from the bit depth.
fn container_bytes(format: &FormatChunk) -> usize {
    let channels = usize::from(format.channels);
    let block_align = usize::from(format.block_align);
    if block_align >= channels && block_align % channels == 0 && block_align / channels <= 8 {
        block_align / channels
    } else {
        usize::from(format.bits).div_ceil(8)
    }
}

fn decode_pcm(format: &FormatChunk, data: &[u8]) -> Result<Vec<i16>, AudioError> {
    let width = container_bytes(format);
    let take = data.len() / (width * usize::from(format.channels)) * usize::from(format.channels);
    let chunks = data.chunks_exact(width).take(take);
    let samples = match width {
        1 => chunks.map(|b| (i16::from(b[0]) - 128) << 8).collect(),
        2 => chunks.map(|b| i16::from_le_bytes([b[0], b[1]])).collect(),
        3 => chunks.map(|b| i16::from_le_bytes([b[1], b[2]])).collect(),
        4 => chunks.map(|b| i16::from_le_bytes([b[2], b[3]])).collect(),
        _ => return Err(malformed(format!("{}-bit PCM", format.bits))),
    };
    Ok(samples)
}

fn decode_float(format: &FormatChunk, data: &[u8]) -> Result<Vec<i16>, AudioError> {
    let to_i16 = |v: f64| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16;
    let channels = usize::from(format.channels);
    let samples = match container_bytes(format) {
        4 => data
            .as_chunks::<4>()
            .0
            .iter()
            .take(data.len() / (4 * channels) * channels)
            .map(|&b| to_i16(f64::from(f32::from_le_bytes(b))))
            .collect(),
        8 => data
            .as_chunks::<8>()
            .0
            .iter()
            .take(data.len() / (8 * channels) * channels)
            .map(|&b| to_i16(f64::from_le_bytes(b)))
            .collect(),
        _ => return Err(malformed(format!("{}-bit float", format.bits))),
    };
    Ok(samples)
}

/// Keeps mono and stereo as they are and takes the front left and right
/// of anything wider.
fn fold_to_stereo(rate: u32, channels: usize, samples: Vec<i16>) -> Pcm {
    if channels <= 2 {
        return Pcm {
            rate,
            channels: channels as u16,
            samples,
        };
    }
    let samples = samples
        .chunks_exact(channels)
        .flat_map(|frame| [frame[0], frame[1]])
        .collect();
    Pcm {
        rate,
        channels: 2,
        samples,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a WAV file from a format tag, its fields and raw data.
    pub(crate) fn wav_bytes(
        tag: u16,
        channels: u16,
        rate: u32,
        bits: u16,
        block_align: u16,
        extra: &[u8],
        data: &[u8],
    ) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&tag.to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&rate.to_le_bytes());
        fmt.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
        fmt.extend_from_slice(&block_align.to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        if !extra.is_empty() {
            fmt.extend_from_slice(&(extra.len() as u16).to_le_bytes());
            fmt.extend_from_slice(extra);
        }
        let mut out = b"RIFF\0\0\0\0WAVE".to_vec();
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        out.extend_from_slice(&fmt);
        if fmt.len() % 2 == 1 {
            out.push(0);
        }
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(data);
        let riff = (out.len() - 8) as u32;
        out[4..8].copy_from_slice(&riff.to_le_bytes());
        out
    }

    pub(crate) fn pcm16_wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        wav_bytes(PCM, channels, rate, 16, 2 * channels, &[], &data)
    }

    #[test]
    fn decodes_16_bit_stereo() {
        let pcm = decode(&pcm16_wav(22050, 2, &[1, -1, 1000, -1000])).unwrap();
        assert_eq!(pcm.rate, 22050);
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.samples, vec![1, -1, 1000, -1000]);
    }

    #[test]
    fn decodes_8_bit_unsigned() {
        let wav = wav_bytes(PCM, 1, 8000, 8, 1, &[], &[0, 128, 255]);
        assert_eq!(decode(&wav).unwrap().samples, vec![-32768, 0, 32512]);
    }

    #[test]
    fn decodes_24_and_32_bit() {
        let wav = wav_bytes(
            PCM,
            1,
            8000,
            24,
            3,
            &[],
            &[0x00, 0x00, 0x40, 0xFF, 0xFF, 0xFF],
        );
        assert_eq!(decode(&wav).unwrap().samples, vec![0x4000, -1]);
        let wav = wav_bytes(PCM, 1, 8000, 32, 4, &[], &[0, 0, 0, 0x80, 0, 0, 0xFF, 0x7F]);
        assert_eq!(decode(&wav).unwrap().samples, vec![i16::MIN, i16::MAX]);
    }

    #[test]
    fn decodes_ieee_float() {
        let data: Vec<u8> = [0.5f32, -1.0, 2.0]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let wav = wav_bytes(IEEE_FLOAT, 1, 8000, 32, 4, &[], &data);
        assert_eq!(decode(&wav).unwrap().samples, vec![16384, -32767, 32767]);
        let data: Vec<u8> = [0.25f64].iter().flat_map(|v| v.to_le_bytes()).collect();
        let wav = wav_bytes(IEEE_FLOAT, 1, 8000, 64, 8, &[], &data);
        assert_eq!(decode(&wav).unwrap().samples, vec![8192]);
    }

    #[test]
    fn decodes_g711() {
        let wav = wav_bytes(MU_LAW, 1, 8000, 8, 1, &[], &[0xFF, 0x80]);
        assert_eq!(decode(&wav).unwrap().samples, vec![0, 32124]);
        let wav = wav_bytes(A_LAW, 1, 8000, 8, 1, &[], &[0xD5, 0x2A]);
        assert_eq!(decode(&wav).unwrap().samples, vec![8, -32256]);
    }

    #[test]
    fn decodes_ima_adpcm() {
        let ramp: Vec<i16> = (0..1010).map(|i| ((i % 505) * 30) as i16).collect();
        let data = adpcm::tests::encode_ima_mono(&ramp, 256);
        let wav = wav_bytes(IMA_ADPCM, 1, 8000, 4, 256, &505u16.to_le_bytes(), &data);
        let pcm = decode(&wav).unwrap();
        assert_eq!(pcm.frames(), 1010);
        assert_eq!(pcm.samples[505], 0);
        assert!((i32::from(pcm.samples[1000]) - 495 * 30).abs() < 300);
    }

    #[test]
    fn decodes_ms_adpcm_with_listed_coefficients() {
        let mut extra = Vec::new();
        extra.extend_from_slice(&4u16.to_le_bytes());
        extra.extend_from_slice(&1u16.to_le_bytes());
        extra.extend_from_slice(&256i16.to_le_bytes());
        extra.extend_from_slice(&0i16.to_le_bytes());
        let block = [0, 16, 0, 100, 0, 50, 0, 0x1F];
        let wav = wav_bytes(MS_ADPCM, 1, 8000, 4, 8, &extra, &block);
        assert_eq!(decode(&wav).unwrap().samples, vec![50, 100, 116, 100]);
    }

    #[test]
    fn extensible_uses_the_sub_format() {
        let mut extra = vec![16, 0, 4, 0, 0, 0];
        extra.extend_from_slice(&PCM.to_le_bytes());
        extra.extend_from_slice(&[0; 14]);
        let wav = wav_bytes(EXTENSIBLE, 1, 8000, 16, 2, &extra, &[0x34, 0x12]);
        assert_eq!(decode(&wav).unwrap().samples, vec![0x1234]);
    }

    #[test]
    fn wider_files_keep_front_left_and_right() {
        let wav = pcm16_wav(8000, 4, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let pcm = decode(&wav).unwrap();
        assert_eq!(pcm.channels, 2);
        assert_eq!(pcm.samples, vec![1, 2, 5, 6]);
    }

    #[test]
    fn tolerates_wrong_sizes() {
        let samples = [10i16, 20, 30, 40];
        // A data size past the end of the file.
        let mut wav = pcm16_wav(8000, 1, &samples);
        let data_size_at = wav.len() - 8 - 4;
        wav[data_size_at..data_size_at + 4].copy_from_slice(&1_000_000u32.to_le_bytes());
        assert_eq!(decode(&wav).unwrap().samples, samples);
        // A data size of zero, as streaming writers leave it.
        wav[data_size_at..data_size_at + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode(&wav).unwrap().samples, samples);
        // A RIFF size that means nothing.
        wav[4..8].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(decode(&wav).unwrap().samples, samples);
        // A torn last sample is dropped.
        wav.push(0x55);
        assert_eq!(decode(&wav).unwrap().samples, samples);
    }

    #[test]
    fn tolerates_a_missing_pad_byte() {
        let mut wav = b"RIFF\0\0\0\0WAVE".to_vec();
        // An odd-sized chunk with no pad byte before the next one.
        wav.extend_from_slice(b"LIST\x03\0\0\0abc");
        let rest = pcm16_wav(8000, 1, &[7, 8]);
        wav.extend_from_slice(&rest[12..]);
        assert_eq!(decode(&wav).unwrap().samples, vec![7, 8]);
    }

    #[test]
    fn finds_data_after_garbage() {
        let mut wav = pcm16_wav(8000, 1, &[5, 6]);
        // Break the id of the data chunk's neighbour: insert junk between
        // fmt and data.
        let data_at = wav.windows(4).position(|w| w == b"data").unwrap();
        wav.splice(data_at..data_at, [0x01, 0x02, 0x03]);
        assert_eq!(decode(&wav).unwrap().samples, vec![5, 6]);
    }

    #[test]
    fn reports_broken_files() {
        assert!(matches!(
            decode(b"RIFF\0\0\0\0WAVE"),
            Err(AudioError::Malformed { .. })
        ));
        assert!(matches!(decode(b"nope"), Err(AudioError::Malformed { .. })));
        let wav = wav_bytes(PCM, 0, 8000, 16, 2, &[], &[0, 0]);
        assert!(matches!(decode(&wav), Err(AudioError::Malformed { .. })));
        let wav = wav_bytes(PCM, 1, 0, 16, 2, &[], &[0, 0]);
        assert!(matches!(decode(&wav), Err(AudioError::Malformed { .. })));
        let wav = wav_bytes(0x0055, 1, 8000, 16, 2, &[], &[0, 0]);
        assert!(matches!(decode(&wav), Err(AudioError::Unsupported(_))));
    }
}
