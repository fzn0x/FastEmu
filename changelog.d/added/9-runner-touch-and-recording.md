- The key script of `fastemu run` can touch the screen: `tap@XxY`,
  `press@XxY`, `drag@XxY` and `release@XxY` play touch games such as
  Lines and Pixelus without a window, and `--touch` gives any phone
  profile a touch screen (#9).
- `fastemu run --data DIR` keeps the game's saves in DIR, so a later run
  continues from them (#9).
- `fastemu run --record DIR --every N` saves every Nth frame the game
  draws as a numbered PNG, for videos and pictures of real play (#9).
