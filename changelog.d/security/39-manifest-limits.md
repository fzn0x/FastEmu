- A JAR or JAD with a huge or crafted manifest no longer keeps FastEmu
  busy for minutes, or uses hundreds of megabytes of memory, when it is
  added or started. FastEmu reads the first 1 MiB of the file and its
  first 4096 attributes (#39).
