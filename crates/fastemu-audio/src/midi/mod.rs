//! MIDI playback: a standard MIDI file parser, a sequencer that plays it
//! by media time, and two synthesizers behind one trait. The built-in
//! General MIDI synthesizer needs no files; a SoundFont the user loads
//! sounds better.

pub(crate) mod builtin;
pub(crate) mod sequencer;
pub(crate) mod smf;
pub(crate) mod soundfont;

/// A General MIDI synthesizer the sequencer drives.
pub(crate) trait Synth: Send {
    /// Takes one channel message: note on and off, controllers, program
    /// changes, pitch bend.
    fn message(&mut self, status: u8, data1: u8, data2: u8);

    /// Silences every voice and returns every channel to its power-on
    /// state, as before the first note of a file.
    fn reset(&mut self);

    /// Writes the next `left.len()` frames, replacing what is there.
    /// Both slices have the same length.
    fn render(&mut self, left: &mut [f32], right: &mut [f32]);
}
