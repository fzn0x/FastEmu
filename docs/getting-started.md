# Getting started

Modern and faster version of MicroEmu and J2ME Loader with multi instances support, available in Windows, Linux, MacOS and Android built in Zed GPUI.

FastEmu runs Java ME (J2ME) games and apps from their `.jar` and `.jad`
files. This page takes you from a download to a running game.

## Install

Download the build for your system from the
[latest release](https://github.com/fzn0x/FastEmu/releases/latest).

| System  | What to download                                                                  |
| ------- | --------------------------------------------------------------------------------- |
| Windows | `fastemu-<version>-windows-x64.zip`, unpack it anywhere                           |
| Linux   | `fastemu-<version>-linux-x64.tar.gz` or `-linux-arm64.tar.gz`, unpack it anywhere |
| macOS   | `fastemu-<version>-macos-universal.zip`, move the app to Applications             |
| Android | `fastemu-<version>-android.apk`, install it from your files                       |

FastEmu does not need an installer and does not need Java installed. It
carries its own virtual machine. On Debian and Ubuntu,
`fastemu-<version>-linux-x64.deb` installs it with a menu entry instead.

## Get a game

FastEmu does not ship games. Use `.jar` files you already own or that
their authors made free to share. A `.jad` file is a small text
descriptor that sits next to a `.jar`. FastEmu reads either one.

## Run your first game

1. Start FastEmu.
2. Drag a `.jar` onto the window, or choose **+** at the top of the
   library and pick the file. On Android, choose **+** and pick it from
   storage. You can also start FastEmu with the file:
   `fastemu snake.jar` adds it to the library and starts it.
3. The game appears in the library. Click it to start it. Click it again
   to start a second copy beside the first.

The game runs in its own phone. Its screen size, key codes and saves
come from a phone profile. The default is a 240x320 phone with Nokia key
codes, which suits most games. See [Settings](/docs/settings/) to change
it.

## Play

On a keyboard, the arrow keys move, Enter or Space is the fire key, the
number row and the number pad press the phone's number keys, and F1 and F2
are the soft keys. Each game also has a phone keypad under its screen, for
the mouse or a finger. The keyboard plays the game with the highlighted
border; click another game to play it. The full list is in
[Playing](/docs/playing/).

## Run without a window

The same program runs a game with no window, for tests, scripts and
pictures of a game:

```
fastemu run snake.jar --frames 300 --keys "wait:1500 5" --screenshot snake.png
```

It runs the game for 300 frames of its screen (five seconds, at 60 frames a
second), presses 5 after a second and a half, saves the last picture as a
PNG and prints what the game logged. It exits with 1 if the game crashes or
shows nothing in its first 30 seconds, and with 2 for a mistake on the
command line.

On Windows, type `fastemu` in Command Prompt or PowerShell, in FastEmu's
folder or with it on `PATH`. That runs `fastemu.com`, the console program
beside `fastemu.exe`, which waits for the game, prints into the terminal
and returns the exit code. `fastemu.exe` is the app: opened from Explorer
or a shortcut, it shows no console window, and typed as `fastemu.exe` it
prints too, but the terminal does not wait for it.

The options:

| Option | What it does |
| --- | --- |
| `--midlet N` | Starts the suite's Nth MIDlet (default 1) |
| `--profile ID` | Runs on the phone profile ID, such as `nokia-s40-128x160` (default `generic-240x320`) |
| `--touch` | Gives the phone a touch screen, whatever the profile |
| `--frames N` | Stops after N frames of the screen, 60 a second, counted from the game's first picture (default 300) |
| `--seconds S` | Stops after S seconds instead |
| `--screenshot PATH` | Saves the last picture as a PNG |
| `--record DIR` | Saves the pictures the game draws in DIR as `frame-000001.png`, `frame-000002.png` and on; a DIR that holds frames of an earlier run is refused |
| `--every N` | With `--record`, saves every Nth picture the game draws (default 1) |
| `--keys SCRIPT` | Presses keys and touches the screen while the game runs |
| `--encoding NAME` | The game's text encoding (default `ISO-8859-1`) |
| `--data DIR` | Keeps the game's record stores in DIR, so a later run continues from them (default: a temporary folder, deleted after the run) |

The key script is a list of steps separated by spaces or commas, run in
order from the game's first picture:

| Step | What it does |
| --- | --- |
| `5`, `fire`, `softleft` | Taps the key for a tenth of a second |
| `down:300` | Holds the key for 300 milliseconds |
| `wait:500` or `500` | Waits 500 milliseconds |
| `tap@120x160` | Touches the screen at pixel 120, 160 and lets go a tenth of a second later |
| `tap@120x160:400` | Touches it for 400 milliseconds |
| `press@40x40`, `drag@80x40`, `release@80x40` | Puts a finger down, moves it, lifts it |

Key names are `0` to `9`, `star`, `pound`, `up`, `down`, `left`, `right`,
`fire`, `softleft`, `softright`, `clear`, `send` and `end`. Touch steps
reach a game only on a touch screen: the `generic-360x640` profile, or
`--touch`. When `SCRIPT` names a file, the steps are read from it, with
`#` starting a comment.

For example, to record a touch game at a few pictures a second:

```
fastemu run puzzle.jar --profile generic-320x240 --touch --seconds 20 \
  --keys "wait:2000 tap@160x120" --record frames --every 10
```

## Next

- [Playing](/docs/playing/): the library, several games at once, layouts.
- [Settings](/docs/settings/): phone profiles, key codes and where saves live.
- [Compatibility](/docs/compatibility/): what runs and what does not yet.
- [FAQ](/docs/faq/): common questions.
