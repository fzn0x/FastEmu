- FastEmu opens MIDlet suites from a .jar or a .jad, including JARs with a
  damaged central directory, manifests with a byte order mark or wrapped
  lines, and JADs whose JAR sits elsewhere, and runs each MIDlet's
  lifecycle: start, pause, resume, stop and its own exit (#3).
