- A JAR whose entries all point at the same compressed data, or a damaged
  JAR made of thousands of file headers, no longer keeps FastEmu busy for
  minutes or uses large amounts of memory when it is added or started.
  FastEmu looks for the MIDlets of a suite that lists none within 32 MiB
  of inflated data (#39).
