- The desktop window runs real games, with sound through the system's
  audio device (silent, and still running, when there is none). Games
  named on the command line are added to the library and started, and
  `fastemu --mock` shows test patterns instead, for work on the window
  itself (#8).
- Typing switches to text on its own while a game shows a text box, and
  back to the keypad when it closes; the text button on a game's title
  bar still switches by hand (#8).
- A game that cannot start gets its tile all the same, with the reason
  and a button to try again, and a crash shows its message in the
  game's tile (#8).
