# Playing

## Adding games

There are three ways to add a game:

- Drag one or more `.jar` or `.jad` files onto the window.
- Choose **Add game** and pick files.
- Pass files on the command line: `fastemu snake.jar tetris.jar`.

FastEmu reads the suite's name, vendor, version and icon from the
manifest and shows them in the library. A suite with more than one
MIDlet shows one entry per MIDlet.

## The library

The library lists every game you added. Each game keeps its own
settings and its own saves. Removing a game from the library asks
whether to keep or delete its saves.

Search by name from the box at the top. Sort by name or by last played.

## Several games at once

FastEmu can run many games in one window. Each game is its own
instance: its own virtual machine, threads, memory, record stores and
sound. A crash or a hang in one game does not stop the others.

Open another game from the library while one is running and it starts
next to it. You can also start the same game twice. Each copy gets its
own save slot, so two copies do not overwrite each other.

Sound plays from the game you focus. Other games are muted unless you
turn on **Play sound from all games** in the settings.

Games you are not looking at can be paused. Use **Pause hidden games**
to stop them from using the processor until you bring them back.

## Layouts

The window can show games in a few layouts:

- **Grid**: every running game in equal tiles.
- **Focus**: one large game with the others in a strip beside it.
- **Single**: one game fills the window.

Drag a tile to reorder. Double-click a tile to switch to Focus on it.

## Keys and touch

The keyboard maps to the phone keypad like this:

| Phone key     | Keyboard                      |
| ------------- | ----------------------------- |
| 0 to 9        | `0` to `9`, number pad        |
| `*` and `#`   | `Shift+8` and `Shift+3`, or number pad `/` and `*` |
| Up, down, left, right | arrow keys, or `W` `A` `S` `D` |
| Fire          | Enter, Space                  |
| Left soft key | `Q`                           |
| Right soft key| `E`                           |
| Send, End     | `Z`, `X`                      |

You can change any of these in the settings.

On a touch screen FastEmu draws a keypad below or beside the game. Games
that handle touch themselves, on profiles with a touch screen, get
pointer events from taps and drags directly.

The key codes a game sees depend on the device profile, because phone
makers used different codes. See [Settings](/docs/settings/).
