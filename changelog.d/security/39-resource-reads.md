- A game that reads a resource that is far bigger than its memory, or
  that several threads read at once, no longer makes FastEmu build the
  whole file first. The game gets OutOfMemoryError, as before (#39).
