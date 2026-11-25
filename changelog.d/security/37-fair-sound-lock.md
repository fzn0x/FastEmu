- One game's sound can no longer make the sound of other games stall: a
  game that seeks a big MIDI file or starts and stops sounds in a tight
  loop used to keep the shared mixer for itself, and every other game's
  sound calls waited for it (#37).
