//! MP3 playback with symphonia's pure Rust decoder.
//!
//! MP3 is the one format decoded while it plays rather than up front: a
//! two-minute background track would take tens of megabytes as PCM, and
//! decoding a frame takes microseconds.

use std::io::Cursor;
use std::sync::Arc;

use symphonia_bundle_mp3::{MpaDecoder, MpaReader};
use symphonia_core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia_core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia_core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia_core::units::Timestamp;

use crate::error::AudioError;
use crate::format::Format;
use crate::source::Source;

/// Consecutive undecodable frames tolerated before the stream counts as
/// ended.
const MAX_BAD_FRAMES: usize = 16;

/// The seconds of sound one call to `render` decodes only to drop it. A
/// seek the reader cannot do (a header in the middle of the file that it
/// refuses) falls back to decoding from the start up to the target, which
/// for a file of 16 MiB is half a second of work done with the mixer
/// locked. The decoding is spread over the calls that follow instead, at
/// about a millisecond each.
const MAX_SKIP_SECONDS_PER_CALL: u64 = 8;

fn malformed(reason: impl Into<String>) -> AudioError {
    AudioError::Malformed {
        format: Format::Mp3,
        reason: reason.into(),
    }
}

/// Skips ID3v2 tags, which the frame reader does not expect.
fn strip_id3(mut data: &[u8]) -> &[u8] {
    while data.len() >= 10 && data.starts_with(b"ID3") {
        // The size is four 7-bit bytes, not counting the header and the
        // optional footer.
        let size = data[6..10]
            .iter()
            .fold(0usize, |size, &b| (size << 7) | usize::from(b & 0x7F));
        let footer = if data[5] & 0x10 != 0 { 10 } else { 0 };
        data = data.get(10 + size + footer..).unwrap_or_default();
    }
    data
}

struct Stream {
    reader: MpaReader<'static>,
    decoder: MpaDecoder,
}

fn open_stream(data: &Arc<[u8]>) -> Result<Stream, AudioError> {
    let source = MediaSourceStream::new(
        Box::new(Cursor::new(Arc::clone(data))),
        MediaSourceStreamOptions::default(),
    );
    let reader = MpaReader::try_new(source, FormatOptions::default())
        .map_err(|err| malformed(format!("no MPEG audio frames: {err}")))?;
    let params = reader
        .tracks()
        .first()
        .and_then(|track| track.codec_params.as_ref())
        .and_then(|params| params.audio())
        .ok_or_else(|| malformed("no audio track"))?;
    let decoder = MpaDecoder::try_new(params, &AudioDecoderOptions::default())
        .map_err(|err| AudioError::Unsupported(format!("MPEG audio: {err}")))?;
    Ok(Stream { reader, decoder })
}

/// An MP3 file as a [`Source`], decoded a frame at a time.
pub(crate) struct Mp3Source {
    data: Arc<[u8]>,
    stream: Stream,
    rate: u32,
    channels: usize,
    total: u64,
    pos: u64,
    /// Decoded frames not yet handed out, interleaved at `channels`.
    pending: Vec<f32>,
    taken: usize,
    /// Frames still to drop after a seek landed before its target. While
    /// it is more than a call may decode, the call plays silence and the
    /// silence counts here too, so the sound that follows is the sound of
    /// its place in the file.
    skip: u64,
    /// Why decoding stopped early, until the voice takes it.
    error: Option<String>,
}

impl Mp3Source {
    /// Checks the data and measures it. The length comes from the Xing or
    /// LAME header when there is one (it leaves out the encoder's delay
    /// and padding, which the decoder trims), and otherwise from reading
    /// every frame header, which is far cheaper than decoding.
    pub fn new(data: &[u8]) -> Result<Self, AudioError> {
        let data: Arc<[u8]> = strip_id3(data).into();
        let mut stream = open_stream(&data)?;
        let params = stream.decoder.codec_params().clone();
        let rate = params
            .sample_rate
            .filter(|&r| r > 0)
            .ok_or_else(|| malformed("no sample rate"))?;
        let channels = params.channels.map_or(1, |c| c.count()).clamp(1, 2);
        let declared = stream.reader.tracks()[0].num_frames;
        let total = declared.unwrap_or_else(|| {
            let mut total = 0;
            while let Ok(Some(packet)) = stream.reader.next_packet() {
                total += packet.dur.get();
            }
            total
        });
        if total == 0 {
            return Err(malformed("no MPEG audio frames"));
        }
        let stream = open_stream(&data)?;
        Ok(Self {
            data,
            stream,
            rate,
            channels,
            total,
            pos: 0,
            pending: Vec::new(),
            taken: 0,
            skip: 0,
            error: None,
        })
    }

    /// Decodes the next frame into `pending`; false at the end.
    fn decode_next(&mut self) -> bool {
        let mut bad = 0;
        loop {
            let packet = match self.stream.reader.next_packet() {
                Ok(Some(packet)) => packet,
                // The data simply ran out.
                Ok(None) => return false,
                Err(err) => {
                    self.error = Some(format!("cannot read the MP3 data: {err}"));
                    return false;
                }
            };
            match self.stream.decoder.decode(&packet) {
                Ok(buffer) => {
                    if buffer.spec().channels().count() != self.channels {
                        self.error = Some("the MP3 changes its channel count".into());
                        return false;
                    }
                    buffer.copy_to_vec_interleaved(&mut self.pending);
                    self.taken = 0;
                    return true;
                }
                Err(err) => {
                    bad += 1;
                    if bad > MAX_BAD_FRAMES {
                        self.error = Some(format!("cannot decode the MP3 data: {err}"));
                        return false;
                    }
                }
            }
        }
    }

    fn pending_frames(&self) -> usize {
        self.pending.len() / self.channels - self.taken
    }
}

impl Source for Mp3Source {
    fn rate(&self) -> u32 {
        self.rate
    }

    fn frames(&self) -> u64 {
        self.total
    }

    fn render(&mut self, out: &mut [f32]) -> usize {
        let wanted = (out.len() / 2).min(usize::try_from(self.total - self.pos).unwrap_or(0));
        let mut done = 0;
        let mut budget = MAX_SKIP_SECONDS_PER_CALL * u64::from(self.rate);
        while done < wanted {
            if self.skip > 0 && budget == 0 {
                // Still on the way to the target: this call is silence, and
                // the frames of it are behind the target now as well.
                out[done * 2..wanted * 2].fill(0.0);
                self.skip += (wanted - done) as u64;
                done = wanted;
                break;
            }
            if self.pending_frames() == 0 && !self.decode_next() {
                break;
            }
            if self.skip > 0 {
                let dropped = self.skip.min(self.pending_frames() as u64).min(budget);
                self.taken += dropped as usize;
                self.skip -= dropped;
                budget -= dropped;
                continue;
            }
            let count = (wanted - done).min(self.pending_frames());
            let start = self.taken * self.channels;
            let input = &self.pending[start..start + count * self.channels];
            let frames = out[done * 2..(done + count) * 2].as_chunks_mut::<2>().0;
            if self.channels == 1 {
                for (frame, &value) in frames.iter_mut().zip(input) {
                    *frame = [value; 2];
                }
            } else {
                for (frame, pair) in frames.iter_mut().zip(input.as_chunks::<2>().0) {
                    *frame = *pair;
                }
            }
            self.taken += count;
            done += count;
        }
        self.pos += done as u64;
        done
    }

    fn take_error(&mut self) -> Option<String> {
        self.error.take()
    }

    /// The file, which the source keeps to decode from, and the frame it
    /// decoded last.
    fn memory_bytes(&self) -> usize {
        self.data.len() + self.pending.capacity() * size_of::<f32>()
    }

    fn seek(&mut self, frame: u64) {
        let frame = frame.min(self.total);
        self.pending.clear();
        self.taken = 0;
        self.pos = frame;
        self.skip = 0;
        if frame == self.total {
            // Nothing is left to play, so nothing needs decoding.
            return;
        }
        if frame > 0 {
            let target = SeekTo::Timestamp {
                ts: Timestamp::new(frame as i64),
                track_id: 0,
            };
            if let Ok(seeked) = self.stream.reader.seek(SeekMode::Accurate, target) {
                self.stream.decoder.reset();
                self.skip = frame.saturating_sub(seeked.actual_ts.get().max(0) as u64);
                return;
            }
        }
        // From the start, or when seeking failed: reopen and decode up to
        // the target. The data opened once already, so it opens again.
        if let Ok(stream) = open_stream(&self.data) {
            self.stream = stream;
            self.skip = frame;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One second of a 440 Hz sine, mono at 22050 Hz, 32 kbit/s, with an
    /// ID3v2 tag and a LAME header, made with
    /// `ffmpeg -f lavfi -i sine=frequency=440:sample_rate=22050:duration=1
    /// -ac 1 -codec:a libmp3lame -b:a 32k`.
    const SINE: &[u8] = include_bytes!("../tests/data/sine-440hz-1s.mp3");

    fn render_all(source: &mut Mp3Source) -> Vec<f32> {
        let mut out = vec![0.0; 2 * 1000];
        let mut all = Vec::new();
        loop {
            let n = source.render(&mut out);
            all.extend_from_slice(&out[..n * 2]);
            if n < 1000 {
                return all;
            }
        }
    }

    #[test]
    fn decodes_a_sine() {
        let mut source = Mp3Source::new(SINE).unwrap();
        assert_eq!(source.rate(), 22050);
        // The LAME header gives the length without the encoder's delay and
        // padding: exactly one second.
        assert_eq!(source.frames(), 22050);
        let all = render_all(&mut source);
        assert_eq!(all.len() as u64, source.frames() * 2);
        let left: Vec<f32> = all.iter().step_by(2).copied().collect();
        let middle = &left[2000..20000];
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        // 18000 samples at 22050 Hz of 440 Hz: about 359 cycles.
        assert!((355..=363).contains(&crossings), "{crossings}");
        // ffmpeg's sine source has an amplitude of 1/8.
        let peak = middle.iter().fold(0.0f32, |m, v| m.max(v.abs()));
        assert!((0.1..0.15).contains(&peak), "{peak}");
    }

    #[test]
    fn the_file_is_the_memory_once_the_tag_is_stripped() {
        let source = Mp3Source::new(SINE).unwrap();
        assert_eq!(source.memory_bytes(), strip_id3(SINE).len());
    }

    #[test]
    fn seeking_matches_playing_through() {
        let mut through = Mp3Source::new(SINE).unwrap();
        let all = render_all(&mut through);
        let mut source = Mp3Source::new(SINE).unwrap();
        source.seek(10_000);
        let mut out = vec![0.0; 2 * 2000];
        assert_eq!(source.render(&mut out), 2000);
        // After the decoder settles, the samples line up with a straight
        // decode of the same span.
        let reference = &all[2 * 10_000..2 * 12_000];
        let error = out[2 * 1000..]
            .iter()
            .zip(&reference[2 * 1000..])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(error < 0.05, "{error}");
        source.seek(0);
        assert_eq!(render_all(&mut source), all);
    }

    /// A long stream the reader cannot seek in: the frames of the sine
    /// file once more after each copy, with whatever trails a copy of the
    /// file between them. The reader refuses a header there, so a seek
    /// past the first copy falls back to decoding from the start.
    fn long_and_damaged(copies: usize) -> Vec<u8> {
        let stripped = strip_id3(SINE);
        // The first frame holds the Xing and LAME header. Leave it out:
        // the stream then has no length, and `new` walks it to find one.
        let mut first = 100;
        while !(stripped[first] == 0xFF && stripped[first + 1] & 0xE0 == 0xE0) {
            first += 1;
        }
        stripped[first..].repeat(copies)
    }

    #[test]
    fn a_seek_the_reader_cannot_do_is_caught_up_over_several_calls() {
        let data = long_and_damaged(100);
        let mut source = Mp3Source::new(&data).unwrap();
        let rate = u64::from(source.rate());
        let total = source.frames();
        assert!(total > 100 * rate, "{total} frames");
        let target = total - 5 * rate;
        source.seek(target);
        // The fallback: all the frames before the target are to be dropped.
        assert!(
            source.skip > 60 * rate,
            "the reader could seek: {}",
            source.skip
        );
        // Dropping them is spread over the calls that follow: the first
        // one plays silence and does not decode all of it.
        let mut out = vec![0.0; 2 * 256];
        assert_eq!(source.render(&mut out), 256);
        assert!(out.iter().all(|&v| v == 0.0));
        assert!(source.skip > 40 * rate, "{} left", source.skip);
        // The calls after it catch up, and the sound that comes is the
        // sound of the last seconds of the file.
        let (mut played, mut calls, mut heard) = (256, 1, false);
        loop {
            let count = source.render(&mut out);
            played += count as u64;
            calls += 1;
            heard |= out[..count * 2].iter().any(|v| v.abs() > 0.1);
            if count < 256 {
                break;
            }
        }
        assert!(heard, "the last seconds are silent");
        // Most of the last five seconds played (the damaged stream ends a
        // little short of its counted length).
        assert!(played > 3 * rate, "{played} frames played");
        assert!(calls < 600, "{calls} calls");
    }

    #[test]
    fn refuses_data_that_is_not_mpeg_audio() {
        assert!(Mp3Source::new(b"ID3\x03\0\0\0\0\0\0").is_err());
        assert!(Mp3Source::new(&[0u8; 4096]).is_err());
    }

    #[test]
    fn id3_tags_are_skipped() {
        let stripped = strip_id3(SINE);
        assert_eq!(&stripped[..2], &[0xFF, 0xF3]);
        assert_eq!(strip_id3(b"ID3"), b"ID3");
    }
}
