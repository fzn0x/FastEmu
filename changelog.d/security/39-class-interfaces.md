- A damaged JAR with a class that names one interface thousands of times,
  or lists thousands of fields of one very long type, no longer makes
  FastEmu use gigabytes of memory or keeps it busy for seconds when the
  JAR is added or the class loads (#39).
