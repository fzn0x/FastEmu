- `fastemu run --frames N` counts frames of the screen, 60 a second,
  instead of pictures the game draws, so a game that redraws rarely,
  such as Tetris with one picture every 800 ms, no longer keeps the
  runner going for minutes. A still screen, such as a game-over screen,
  no longer fails the run after 30 seconds; only a game that shows
  nothing at all does (#9).
