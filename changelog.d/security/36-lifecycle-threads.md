- A game that pauses and resumes itself again and again from a start that
  never returns no longer piles up threads without end; the later
  requests are dropped, and the game still gets its closing call when you
  stop it (#36).
