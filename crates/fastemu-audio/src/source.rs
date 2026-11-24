//! The interface every decoder and synthesizer offers the mixer.

/// A stream of stereo frames at its own sample rate: decoded PCM, a MIDI
/// sequence driving a synthesizer, or a tone list. The mixer resamples it
/// to the output rate, so a source never needs to know that rate.
///
/// Positions are in frames at [`Source::rate`]; the player turns them
/// into media time.
pub(crate) trait Source: Send {
    /// Frames per second of what [`Source::render`] writes.
    fn rate(&self) -> u32;

    /// The length of the media in frames.
    fn frames(&self) -> u64;

    /// Writes up to `out.len() / 2` interleaved stereo frames from the
    /// current position and returns how many it wrote. Fewer than asked
    /// means the end of the media was reached.
    fn render(&mut self, out: &mut [f32]) -> usize;

    /// Moves to `frame`, clamped to the length.
    fn seek(&mut self, frame: u64);

    /// The reason the source stopped before its length, if it did: a
    /// decoder that gave up on damaged data. Taking it clears it.
    fn take_error(&mut self) -> Option<String> {
        None
    }

    /// Sends a MIDI channel message to the synthesizer behind the source,
    /// as MMAPI's `MIDIControl.shortMidiEvent` does. Returns whether the
    /// source has a synthesizer to take it.
    fn midi_message(&mut self, _status: u8, _data1: u8, _data2: u8) -> bool {
        false
    }
}
