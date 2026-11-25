- A Siemens game that adds the same graphic object to a manager without
  end, or makes sprites and backgrounds in a loop, now gets
  `OutOfMemoryError` at its heap limit instead of using up the
  computer's memory, and a manager keeps an object once (#33).
