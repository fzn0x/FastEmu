- A game that asks for thousands of `callSerially` calls while its
  screen is busy gets an OutOfMemoryError once 10000 are waiting, and
  the other work a game can queue for the screen thread is limited too
  (#30).
