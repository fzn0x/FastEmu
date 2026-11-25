- A damaged JAR with a long chain of large classes no longer makes the
  app hold hundreds of megabytes while they load. The 128 MiB a game's
  classes may take now holds for classes that wait for others to load,
  and for long line number tables and source file names (#47).
