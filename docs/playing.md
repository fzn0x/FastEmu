# Playing

How to add games, run several at once, arrange them in the window and
control them with a keyboard, a mouse or a touch screen.

## Adding games

There are three ways to add a game on the desktop:

- Drag one or more `.jar` or `.jad` files onto the window.
- Choose the **+** button at the top of the library (`Ctrl+O`, `Cmd+O` on
  macOS) and pick files.
- Pass files on the command line: `fastemu snake.jar tetris.jad`. Each
  one is added to the library and started.

On Android, choose **+** and pick one or more files from storage, or open a
`.jar` or `.jad` from a file manager with **Open with FastEmu**. Android
hands FastEmu a copy it can read, so the app keeps its own copy of each
game: picking the same file again finds the copy it already has, and a
different file with the same name, such as a newer version, gets a copy of
its own. A `.jad` and its `.jar` picked together stay side by side. The app
copies at most 64 MiB of a file, the size of the largest game it runs, and
**Open with** takes only the content links file managers send, not `file:`
links from other apps.

FastEmu reads the suite's name, vendor, version and icon from the manifest
or the descriptor and shows them in the library. When a `.jad` and the
`.jar` of the same name come together, the library keeps the `.jad`, which
names its JAR. Adding a file that is already in the library adds no second
entry.

## The library

The library lists every game you added, with its icon, name and vendor.
Click a game to start it; a badge counts its running copies. The sliders
button opens the game's own settings (see [Settings](/docs/settings/)).
The bin button removes the game after a second click on **Remove**; its
saves stay on disk, and a game added again later starts with no saves.

`Ctrl+B` (`Cmd+B` on macOS), or the button at the left of the toolbar,
shows or hides the library. On a narrow window, such as a phone held
upright, starting a game hides the library so the game gets the room.

## Several games at once

FastEmu runs many games in one window. Each game is its own instance: its
own virtual machine, threads, memory, record stores and sound. A crash or
a hang in one game does not stop the others.

Start another game from the library while one runs and it starts next to
it. You can start the same game more than once. Each running copy gets its
own save slot, so two copies never overwrite each other's saves, and the
first copy always uses the first slot, so a game started again finds its
saves.

Every game plays its sound at the same time. Mute a game from its title
bar, or set its volume in its settings.

When the Android app goes to the background, or the system pauses it, every
running game is paused, as a phone pauses its game for a call. They resume
when the app comes back. A game you paused yourself stays paused.

## Layouts

The toolbar switches between two layouts, and `Ctrl+G` (`Cmd+G`) toggles
them:

- **Grid**: every running game in a tile of the same size, in the rows and
  columns that fit the window best.
- **Focus**: the highlighted game large, the others as small live
  pictures in a strip on the right or at the bottom. Click one to bring it
  forward.

Screens are scaled by whole steps where they fit, so pixels stay sharp.

## A game's tile

The highlighted tile has a coloured border: it gets the keyboard. Click a
tile, or use `Ctrl+]` and `Ctrl+[` (`Cmd` on macOS), to highlight another.
The title bar of each tile has, from left to right:

| Button | What it does |
| --- | --- |
| Text | Types characters into the game instead of pressing keypad keys (see below) |
| Pause | Pauses the game (the MIDlet gets `pauseApp`), or resumes it |
| Restart | Stops the game and starts it again with its current settings |
| Sound | Mutes or unmutes the game |
| Camera | Saves the screen as a PNG |
| Log | Opens the game's log; a red dot means it has new errors or warnings |
| Close | Stops the game and closes the tile (`Ctrl+W`, `Cmd+W`) |

A game that ends shows **The game ended**. A game that crashes shows **The
game crashed** with the error, and one that could not start at all, such as
a file that is not a MIDlet suite, shows **The game could not start** with
the reason. Each offers **Restart** and **Close**.

## The log

`Ctrl+J` (`Cmd+J`) or the toolbar's log button opens the log of the
highlighted game under the tiles: what it printed, uncaught exceptions,
and the APIs it called that FastEmu does not have yet. The log follows new
lines while it shows the last one; scroll up to read older lines and it
stays put.

## Web links

Some games have a menu item that opens a web page, such as more games
from their maker. When a game asks to open one, its screen shows the
game's name and the whole address, with **Open** and **Dismiss**. Nothing
opens until you choose **Open**; the page then opens in your browser and
the game keeps running. **Dismiss** drops the request. The buttons
ignore clicks for half a second after the question appears, and look
faint until then, so a quick tap in the game cannot answer a question
you have not read. On a small screen, scroll the question to reach its
buttons, below the address.
Each game has its own question, and restarting or closing the game
drops it. In the focus
layout, a globe next to a game's name in the strip means it is waiting
for an answer.

FastEmu opens only web pages, `http` and `https` addresses without a
backslash or a port outside 1 to 65535, and only after you agree. A game
that asks to make a call, send a message, open a file, install another
game or open another app is told the phone cannot do it, and its log
names what it asked for.

## Keys

The keyboard plays the highlighted game. The default keys:

| Phone key | Keyboard |
| --- | --- |
| 0 to 9 | `0` to `9` on the number row or the number pad |
| `*` | `*` (`Shift+8`, or the number pad's `*`) |
| `#` | `#` (`Shift+3`), or the number pad's `/` |
| Up, down, left, right | Arrow keys |
| Fire | Enter or Space |
| Left soft key | F1 |
| Right soft key | F2 |
| Clear | Backspace |

Send and End have no key by default. The **Keys** tab of the settings
(`Ctrl+,`, `Cmd+,`) changes any of these: choose **Add** next to a phone
key and press the keyboard key to bind to it (`Esc` cancels). A keyboard
key presses one phone key; a phone key can have several keyboard keys.

On Android, a hardware keyboard works the same way, and these keys work
too: a D-pad's centre and a gamepad's A button press Fire, the Menu and
soft-left keys the left soft key, soft-right the right soft key, Clear the
Clear key, and the call and end-call keys of phones with a keypad Send and
End. Back puts the app behind the home screen with its games paused.

Other shortcuts:

| Shortcut | What it does |
| --- | --- |
| `Ctrl+O` | Add games |
| `Ctrl+,` | Settings |
| `Ctrl+G` | Switch between the grid and focus layouts |
| `Ctrl+K` | Show or hide the on-screen keypads |
| `Ctrl+J` | Show or hide the log |
| `Ctrl+B` | Show or hide the library |
| `Ctrl+W` | Close the highlighted game |
| `Ctrl+]`, `Ctrl+[` | Highlight the next or previous game |

On macOS each is `Cmd` instead of `Ctrl`, and `Cmd+Q` quits.

## Typing text

When a game shows a text box, typed characters go into it as text instead
of pressing keypad keys, and go back to the keypad when the text box
closes. The **Text** button on the title bar switches by hand; that choice
holds until the game opens or closes a text box. Keys that type nothing,
such as the arrows, F1 and F2, Enter and Backspace, still press their phone
keys.

On Android, when a game shows a text box, a dialog opens with the system
keyboard. Type and choose **OK** to send the text to the game. Once the
dialog closes, with **OK** or **Cancel**, keys press phone keys again, and
the **Text** button opens the dialog again.

## The on-screen keypad and touch

Each tile has a phone keypad under or beside the screen: the soft keys, the
D-pad with OK (fire), Clear, and the twelve number keys. A key stays down
for as long as the mouse button or the finger holds it, so games see held
keys. `Ctrl+K` (`Cmd+K`) or the toolbar's keypad button shows or hides the
keypads.

A click or a touch on a game's screen reaches the game as a pointer press,
drags and a release when its phone has a touch screen: a touch phone
profile, or any profile with **Touch screen** turned on in the game's
settings (see [Settings](/docs/settings/)). A press still held when
another game or another window takes the focus is let go.

The key codes a game sees depend on its phone profile, because phone makers
used different codes. See [Settings](/docs/settings/).
