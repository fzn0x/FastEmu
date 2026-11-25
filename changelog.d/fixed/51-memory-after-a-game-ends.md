- A game that ends, by exiting or by crashing, gives its memory back
  once its threads have ended, while its tile still shows its last
  screen, its log and how it ended. Before, it held that memory until
  you closed the tile: over 30 MB for a game that filled an 8 MB phone
  with small objects, and closing that tile held up the window for 10
  to 20 ms (#51).
