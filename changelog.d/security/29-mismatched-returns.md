- A damaged or crafted game class that returns the wrong kind of value
  from a method can no longer use up the computer's memory in a loop.
  The game gets a VerifyError the first time it calls that method (#29).
