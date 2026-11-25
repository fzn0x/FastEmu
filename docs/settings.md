# Settings

FastEmu has settings for the whole app and settings for each game. Open the
app's settings with the gear button on the toolbar (`Ctrl+,`, `Cmd+,` on
macOS), and a game's settings with the sliders button next to it in the
library.

## App settings

The **General** tab:

| Setting | What it does | Default |
| --- | --- | --- |
| Theme | Dark or Light | Dark |
| Layout | Grid sizes all games alike; Focus enlarges one | Grid |
| On-screen keypad | Soft keys, D-pad and number keys beside each game | On |
| Default phone | The phone profile games start with unless their own settings pick another | Generic 240x320 |

The **Keys** tab binds keyboard keys to phone keys; see
[Playing](/docs/playing/) for the defaults. **Reset to defaults** puts them
back.

## Game settings

Each game in the library keeps its own:

| Setting | What it does | Default |
| --- | --- | --- |
| Volume | From silent to full in tenths | Full |
| Allow network access | Whether the game may open connections | Off |
| Text encoding | The game's `microedition.encoding` | ISO-8859-1 |
| Phone | A phone profile, or the app's default phone | The app's default |
| Touch screen | Gives the phone a touch screen, whatever its profile | Off |

The volume changes at once. The phone, the touch screen, the network and
the encoding apply when the game starts again: use **Restart** on its tile.

## Phone profiles

A phone profile is the phone a game believes it runs on: screen size, the
`microedition.platform` string, key codes, whether the screen takes touch,
the memory it reports and its font sizes. Many games check these and
refuse to start, or draw wrongly, on a phone they do not know.

FastEmu ships these profiles:

| Profile | Id | Screen | Touch | Key codes |
| --- | --- | --- | --- | --- |
| Generic 240x320 (the default) | `generic-240x320` | 240x320 | no | Nokia |
| Generic 320x240 (landscape) | `generic-320x240` | 320x240 | no | Nokia |
| Generic 360x640 (touch) | `generic-360x640` | 360x640 | yes | Nokia |
| Nokia Series 40 128x160 | `nokia-s40-128x160` | 128x160 | no | Nokia |
| Nokia Series 60 176x208 | `nokia-s60-176x208` | 176x208 | no | Nokia |
| Sony Ericsson 176x220 | `sonyericsson-176x220` | 176x220 | no | Nokia |
| Siemens 132x176 | `siemens-132x176` | 132x176 | no | Siemens |
| Motorola 176x220 | `motorola-176x220` | 176x220 | no | Motorola |
| Samsung 176x220 | `samsung-176x220` | 176x220 | no | Nokia |
| LG 176x220 | `lg-176x220` | 176x220 | no | LG |

If a game looks too small or too large, try a profile that matches the
screen size it was made for. The game's name or its download page often
says which phone it targets. Games that are played by touch need a touch
screen: on the others, clicks and taps on the screen do not reach the
game. Pick the touch profile, or turn on **Touch screen** in the game's
settings to keep another screen size, as a 320x240 touch game needs. The
ids are what `fastemu run --profile` takes; `--touch` adds the touch
screen there.

## Vendor key codes

MIDP fixes the codes of the number keys but leaves the soft keys, arrow
keys and fire key to the phone maker. Games test for the codes of their
target phone. FastEmu has four sets:

| Key | Nokia and most others | Siemens | Motorola | LG |
| --- | --- | --- | --- | --- |
| Left soft | -6 | -1 | -21 | -202 |
| Right soft | -7 | -4 | -22 | -203 |
| Up | -1 | -59 | -1 | -1 |
| Down | -2 | -60 | -6 | -2 |
| Left | -3 | -61 | -2 | -3 |
| Right | -4 | -62 | -5 | -4 |
| Fire | -5 | -26 | -20 | -5 |
| Clear | -8 | -8 | -8 | -204 |
| Send | -10 | -10 | -10 | -10 |
| End | -11 | -11 | -11 | -11 |

If a game's menu does not respond to the soft keys or arrows, the
profile's key codes are the first thing to change.

## Encodings

**Text encoding** sets the `microedition.encoding` property, the character
set a game uses when it turns bytes into text and names none. The default
is `ISO-8859-1`, as on Nokia's phones. Games made for some regions expect
the encoding of their phones, for example `windows-1251` for Cyrillic text
or `Shift_JIS` for Japanese. If text shows as wrong characters, pick
another and restart the game.

## Network

Network access is off by default. A game that opens an HTTP or socket
connection then fails the way it would on a phone with no signal. Turn
**Allow network access** on for a game that needs it. Only allow games
you trust, since a game with network access can send data out.

Network access means the whole network, not only the internet. A game
allowed it can reach programs on your own computer (loopback) and
devices on your local network. It can also listen for connections on
every network interface of your computer, and learn your computer's
address on the local network. Leave it off for games that do not need
it.

Text messages are never sent, with or without network access. A game
that sends an SMS, often a paid one, is told the send failed, and the
attempt shows in its log.

This setting does not cover a web page the game asks to open. That
page opens in your browser, and only when you choose **Open**. Read the
whole address first: a game can put data in it.

## Where FastEmu keeps its files

| What | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Library and settings (`library.json`, `settings.json`) | `%APPDATA%\FastEmu` | `~/Library/Application Support/FastEmu` | `~/.config/fastemu` |
| Saves | `%APPDATA%\FastEmu\games` | `~/Library/Application Support/FastEmu/games` | `~/.local/share/fastemu/games` |
| Screenshots | `Pictures\FastEmu` | `~/Pictures/FastEmu` | `~/Pictures/FastEmu` |

Each game's saves are in a folder named after its number in the library,
with one folder per running copy inside: `games/3/0` is the first copy of
game 3, `games/3/1` the second copy running beside it. The record stores
are in its `rms` folder. Where the system names no Pictures folder,
screenshots go to a `screenshots` folder beside `games`.

Removing a game from the library keeps its saves, and so does losing
`library.json`. A game added later always gets a number above every
folder in `games`, so it never finds the saves of an older game. The
saves of games the library no longer lists stay where they are until you
delete their folders.

A damaged `library.json` or `settings.json` is reported in the window
and set aside as `library.json.broken` or `settings.json.broken`, and
the defaults are used. One that cannot be read at all is reported and
left where it is.

On Android the library, settings, saves and the app's copies of games are
in the app's private storage, and screenshots in
`Android/data/io.github.fzn0x.fastemu/files/screenshots` on the shared
storage.

## Messages on the terminal

The desktop app prints its warnings to the terminal it was started from.
Set `FASTEMU_LOG` to `info` or `debug` for more, or to `off` for none.
`fastemu --mock` starts test patterns instead of games, for working on the
window itself.
