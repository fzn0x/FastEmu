- A game or a damaged JAR that loads thousands of classes with huge
  tables, or asks for thousands of array classes, now gets an
  OutOfMemoryError it can handle, instead of using gigabytes of memory
  and slowing the whole computer (#35).
