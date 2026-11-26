- A game can no longer make FastEmu hold many times its own memory while
  it loads pictures and sounds. Pictures and sounds read from a stream
  count against the game's memory as they are read, a game's threads
  load pictures one at a time, and a GIF whose first frame needs more
  memory than the game has is refused (#44).
