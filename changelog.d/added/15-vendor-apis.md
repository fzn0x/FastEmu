- Games written for Nokia phones run: full-screen `FullCanvas` games get
  their soft keys, and Nokia's `DirectGraphics` draws and reads pixels in
  every Nokia format, with flips, rotations and translucent colours (#15).
- Games made for Siemens phones run on the Siemens profile: its game and
  colour game packages draw sprites, tiled backgrounds and layers, play
  melodies, and give pixel access to black and white pictures (#15).
- Samsung sound clips play on Samsung profiles, and every maker's light
  and vibration requests show in the game's log. Text messages are never
  sent (#15).
- Each phone profile offers only its own maker's extra classes, so games
  that look for them pick the right phone (#15).
