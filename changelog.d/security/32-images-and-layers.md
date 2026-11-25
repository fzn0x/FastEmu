- A game that makes too many or too large images, tile layers or sprites
  now gets OutOfMemoryError, as on a phone, instead of using up the
  computer's memory. A small image file that claims a huge size no longer
  makes the app reserve that much memory before the game sees an error
  (#32).
- A tile layer holds at most 32767 animated tiles; a game that asks for
  more gets OutOfMemoryError, where before it could grow without a limit
  (#32).
