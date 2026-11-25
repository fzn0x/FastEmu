- The compatibility report no longer calls a game hung when it waits for
  a key on a still screen, such as an About page. A game whose screen
  stays still counts as running when it still answers and none of its
  threads is busy; a stuck game stays hung, and the report says why
  (#48).
