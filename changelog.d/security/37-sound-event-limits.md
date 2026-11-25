- A game whose sound listener is slow, and that loops a sound of a few
  frames or flips a volume thousands of times a second, can no longer
  fill the computer's memory with sound events. Listeners hear of a
  loop at most every 20 ms, and the events that wait for a listener are
  limited (#37).
