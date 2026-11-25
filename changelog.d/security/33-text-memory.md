- A game that makes many text boxes or fields from one large text, or
  keeps inserting text into a box with a huge maximum size, now gets
  `OutOfMemoryError` at its heap limit instead of using up the
  computer's memory, and inserting text no longer copies the whole text
  each time (#33).
