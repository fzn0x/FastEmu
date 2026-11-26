- A game with network access can no longer leave a thread and a socket
  behind for every connection it gives up on. It has at most 16 name
  lookups and connects under way at once, and a connect it gave up on
  stops trying within 30 s (#46).
