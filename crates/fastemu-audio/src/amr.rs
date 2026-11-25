//! AMR narrowband decoding with the pure Rust amr-nb crate, a port of
//! opencore-amr. AMR is speech at 8 kHz, so the whole file is decoded up
//! front: a minute takes under a megabyte.
//!
//! A frame can be one byte (a NO_DATA frame, which the decoder conceals
//! as 160 samples), so a file decodes to up to 320 times its size. The
//! frames are counted before any is decoded, and a file of more than ten
//! minutes is refused.

use amr_nb::{Decoder, L_FRAME, MAX_SERIAL_SIZE, Mode, RxFrameType, rfc4867};

use crate::error::AudioError;
use crate::format::Format;
use crate::pcm::Pcm;

const RATE: u32 = 8000;

/// The most frames one file decodes: ten minutes at 8 kHz, 9.6 MB of
/// samples.
const MAX_FRAMES: usize = 600 * RATE as usize / L_FRAME;

/// Decodes a single-channel `.amr` file (RFC 4867 storage format).
pub(crate) fn decode(data: &[u8]) -> Result<Pcm, AudioError> {
    let malformed = |reason: String| AudioError::Malformed {
        format: Format::Amr,
        reason,
    };
    let frames = rfc4867::read_storage(data).map_err(|err| malformed(err.to_string()))?;
    let count = count_frames(frames)?;
    let frames = rfc4867::read_storage(data).map_err(|err| malformed(err.to_string()))?;
    let mut decoder = Decoder::new();
    let mut out = [0i16; L_FRAME];
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(count * L_FRAME)
        .map_err(|_| AudioError::OutOfMemory("no room for the decoded AMR sound".into()))?;
    for frame in frames {
        match frame {
            Ok(frame) => decoder.decode(frame.frame_type, frame.mode, &frame.bits, &mut out),
            // Another codec's comfort noise frame: conceal it like a lost
            // frame so the timing holds.
            Err(rfc4867::Error::UnsupportedFrameType(_)) => decoder.decode(
                RxFrameType::NoData,
                Mode::MR475,
                &[0; MAX_SERIAL_SIZE],
                &mut out,
            ),
            // Cut short: keep what decoded.
            Err(_) => break,
        }
        samples.extend_from_slice(&out);
    }
    if samples.is_empty() {
        return Err(malformed("no frames".into()));
    }
    Ok(Pcm {
        rate: RATE,
        channels: 1,
        samples,
    })
}

/// How many frames [`decode`] will decode from `frames`, which it reads
/// the same way: up to the first frame that cannot be read. Reading the
/// headers decodes nothing, so a file that is too long is refused at once.
fn count_frames(
    frames: impl Iterator<Item = Result<rfc4867::Frame, rfc4867::Error>>,
) -> Result<usize, AudioError> {
    let mut count = 0;
    for frame in frames {
        match frame {
            Ok(_) | Err(rfc4867::Error::UnsupportedFrameType(_)) => count += 1,
            Err(_) => break,
        }
        if count > MAX_FRAMES {
            return Err(AudioError::TooLarge(format!(
                "an AMR sound of more than {} seconds",
                MAX_FRAMES * L_FRAME / RATE as usize
            )));
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use amr_nb::{Dtx, Encoder};

    use super::*;

    fn encode_sine(frequency: f32, seconds: f32) -> Vec<u8> {
        let count = (seconds * RATE as f32) as usize / L_FRAME * L_FRAME;
        let pcm: Vec<i16> = (0..count)
            .map(|i| {
                let t = i as f32 / RATE as f32;
                ((t * frequency * std::f32::consts::TAU).sin() * 8000.0) as i16
            })
            .collect();
        let mut encoder = Encoder::new(Dtx::Off);
        let frames: Vec<rfc4867::Frame> = pcm
            .as_chunks::<L_FRAME>()
            .0
            .iter()
            .map(|chunk| encoder.encode(Mode::MR122, chunk))
            .collect();
        rfc4867::write_storage(&frames).unwrap()
    }

    #[test]
    fn decodes_what_the_encoder_made() {
        let file = encode_sine(440.0, 1.0);
        assert_eq!(Format::sniff(&file), Some(Format::Amr));
        let pcm = decode(&file).unwrap();
        assert_eq!(pcm.rate, 8000);
        assert_eq!(pcm.channels, 1);
        assert_eq!(pcm.frames(), 8000);
        // The codec is lossy, but a steady tone keeps its pitch.
        let middle = &pcm.samples[1000..7000];
        let crossings = middle.windows(2).filter(|w| w[0] <= 0 && w[1] > 0).count();
        assert!((325..=335).contains(&crossings), "{crossings}");
    }

    #[test]
    fn keeps_what_decoded_when_cut_short() {
        let file = encode_sine(300.0, 0.2);
        let cut = &file[..file.len() - 5];
        let pcm = decode(cut).unwrap();
        assert_eq!(pcm.frames(), 1440);
    }

    /// `#!AMR` and then `frames` frames of type 15 (NO_DATA), one byte
    /// each, which decode to 160 samples apiece.
    fn no_data_file(frames: usize) -> Vec<u8> {
        let mut file = b"#!AMR\n".to_vec();
        file.resize(file.len() + frames, 0x7C);
        file
    }

    #[test]
    fn a_file_of_empty_frames_is_refused_before_it_decodes() {
        // 40000 bytes would decode to 6.4 million samples, 12.8 MB.
        let file = no_data_file(40_000);
        let error = decode(&file).expect_err("ten minutes is the most");
        assert!(matches!(error, AudioError::TooLarge(_)), "{error}");
        assert_eq!(
            error.to_string(),
            "sound too large: an AMR sound of more than 600 seconds"
        );
        let file = no_data_file(MAX_FRAMES + 1);
        assert!(matches!(decode(&file), Err(AudioError::TooLarge(_))));
    }

    #[test]
    fn empty_frames_decode_as_concealed_silence_under_the_cap() {
        let pcm = decode(&no_data_file(100)).unwrap();
        assert_eq!(pcm.frames(), 100 * L_FRAME);
        assert_eq!(pcm.samples.capacity(), pcm.samples.len());
    }

    #[test]
    fn refuses_other_data() {
        assert!(decode(b"not amr").is_err());
        assert!(decode(b"#!AMR\n").is_err());
    }
}
