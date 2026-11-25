- A game can no longer start an unlimited number of threads or timers.
  Past 64 threads, the app's own for the game among them, it gets
  OutOfMemoryError, as on a phone, instead of using up
  the computer's threads and leaving none for the app and the other games
  (#36).
