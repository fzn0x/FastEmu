- A game thread that ends by throwing something that is not an exception
  no longer leaves its game half stopped: the thread ends, its locks are
  released and the game is shown as crashed (#29).
