- On Windows, games that pause between frames now run at full speed. A
  game's 10 ms pause took about 16 ms, which held simple game loops near
  52 frames a second; pauses and timed waits now end within about a
  millisecond of their time on every platform (#1).
