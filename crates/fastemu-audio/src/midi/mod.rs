//! MIDI playback: a standard MIDI file parser, a sequencer that plays it
//! by media time, and two synthesizers behind one trait. The built-in
//! General MIDI synthesizer needs no files; a SoundFont the user loads
//! sounds better.

pub(crate) mod smf;
