- A game can no longer use up the computer's memory with one Nokia
  drawPixels call over 4 million pixels, or by asking for every
  character of every font: the font cache is now limited. A call over
  the limit throws IllegalArgumentException (#31).
