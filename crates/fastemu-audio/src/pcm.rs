//! Decoded audio held in memory, and the source that plays it.

use std::sync::Arc;

use crate::source::Source;

/// Decoded sound: interleaved 16-bit samples, one or two channels.
///
/// Sixteen bits are plenty for phone game audio and take half the memory
/// of `f32`, which matters when a game keeps dozens of sounds loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pcm {
    pub rate: u32,
    /// 1 or 2; decoders fold anything wider down to stereo.
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

/// Plays a [`Pcm`] buffer. Mono plays on both sides.
pub(crate) struct PcmSource {
    pcm: Arc<Pcm>,
    pos: usize,
}

impl PcmSource {
    pub fn new(pcm: Arc<Pcm>) -> Self {
        Self { pcm, pos: 0 }
    }
}

const SCALE: f32 = 1.0 / 32768.0;

impl Source for PcmSource {
    fn rate(&self) -> u32 {
        self.pcm.rate
    }

    fn frames(&self) -> u64 {
        self.pcm.frames() as u64
    }

    fn render(&mut self, out: &mut [f32]) -> usize {
        let wanted = out.len() / 2;
        let count = wanted.min(self.pcm.frames().saturating_sub(self.pos));
        let out = &mut out[..count * 2];
        match self.pcm.channels {
            1 => {
                let input = &self.pcm.samples[self.pos..self.pos + count];
                for (frame, &sample) in out.as_chunks_mut::<2>().0.iter_mut().zip(input) {
                    let value = f32::from(sample) * SCALE;
                    *frame = [value; 2];
                }
            }
            _ => {
                let input = &self.pcm.samples[self.pos * 2..(self.pos + count) * 2];
                for (value, &sample) in out.iter_mut().zip(input) {
                    *value = f32::from(sample) * SCALE;
                }
            }
        }
        self.pos += count;
        count
    }

    fn seek(&mut self, frame: u64) {
        self.pos = usize::try_from(frame)
            .unwrap_or(usize::MAX)
            .min(self.pcm.frames());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(channels: u16, samples: Vec<i16>) -> PcmSource {
        PcmSource::new(Arc::new(Pcm {
            rate: 8000,
            channels,
            samples,
        }))
    }

    #[test]
    fn mono_plays_on_both_sides() {
        let mut src = source(1, vec![16384, -16384, 0]);
        let mut out = [9.0; 8];
        assert_eq!(src.render(&mut out), 3);
        assert_eq!(&out[..6], &[0.5, 0.5, -0.5, -0.5, 0.0, 0.0]);
        assert_eq!(src.render(&mut out), 0);
    }

    #[test]
    fn stereo_keeps_its_sides_and_seeks() {
        let mut src = source(2, vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(src.frames(), 3);
        src.seek(1);
        let mut out = [0.0; 4];
        assert_eq!(src.render(&mut out), 2);
        assert_eq!(out, [3.0, 4.0, 5.0, 6.0].map(|v| v * SCALE));
        src.seek(99);
        assert_eq!(src.render(&mut out), 0);
    }
}
