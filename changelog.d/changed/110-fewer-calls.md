- On Android and Linux, FastEmu's window makes fewer graphics calls in
  each frame, and it reuses the texture that holds a game's screen. On
  Android a moving game takes 16 to 20 percent less processor time than
  before (#110).
