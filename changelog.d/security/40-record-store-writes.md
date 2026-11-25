- A game that saves its record stores in an endless loop is held to a
  burst of 32 MiB and then 4 MiB of disk writes a second, so it can no
  longer wear out the disk or slow the other games. Saves stay
  synchronous and durable, and a game that saves normally never waits
  (#40).
