- A game that recurses very deeply, or whose methods ask for huge
  frames, now gets StackOverflowError instead of using gigabytes of
  memory (#28).
