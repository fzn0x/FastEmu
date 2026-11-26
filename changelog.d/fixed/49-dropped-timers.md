- A game that starts a new timer for each screen, level or effect, and
  lets the old ones go without stopping them, no longer fails with
  OutOfMemoryError after about 62 of them. The timers it has let go of
  give their threads back once they have nothing left to run (#49).
