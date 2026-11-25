- A MIDI file with more than 65,536 events is refused with a
  MediaException, and one shorter than 20 ms plays once even when the
  game loops it, so a crafted file can no longer keep the audio thread
  busy for minutes (#37).
