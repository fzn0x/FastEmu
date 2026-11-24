# Settings

Settings apply to one game, so two games can use different phones.
Global defaults, set once, apply to games you add later.

## Device profiles

A device profile is the phone a game believes it runs on: screen size,
the `microedition.platform` string, key codes, whether the screen takes
touch, the memory it reports and its font sizes. Many games check these
and refuse to start, or draw wrongly, on a phone they do not know.

FastEmu ships these profiles:

| Profile                      | Screen  | Touch |
| ---------------------------- | ------- | ----- |
| Generic 240x320 (the default)| 240x320 | no    |
| Generic 320x240 (landscape)  | 320x240 | no    |
| Generic 360x640 (touch)      | 360x640 | yes   |
| Nokia Series 40              | 128x160 | no    |
| Nokia Series 60              | 176x208 | no    |
| Sony Ericsson                | 176x220 | no    |
| Siemens                      | 132x176 | no    |
| Motorola                     | 176x220 | no    |

If a game looks too small or too large, try a profile that matches the
screen size it was made for. The game's name or its download page often
says which phone it targets.

## Vendor key codes

MIDP fixes the codes of the number keys but leaves the soft keys, arrow
keys and fire key to the phone maker. Games test for the codes of their
target phone. FastEmu has three sets:

| Key         | Nokia and most others | Siemens | Motorola |
| ----------- | --------------------- | ------- | -------- |
| Left soft   | -6                    | -1      | -21      |
| Right soft  | -7                    | -4      | -22      |
| Up          | -1                    | -59     | -1       |
| Down        | -2                    | -60     | -6       |
| Left        | -3                    | -61     | -2       |
| Right       | -4                    | -62     | -5       |
| Fire        | -5                    | -26     | -20      |

If a game's menu does not respond to the soft keys or arrows, the
profile's key codes are the first thing to change.

## Encodings and locale

The **Encoding** setting sets the `microedition.encoding` property, the
character set used when a game turns bytes into text. The default is
`UTF-8`. Games made for some regions expect the encoding of their
phones, for example `windows-1251` for Cyrillic text or `Shift_JIS` for
Japanese. If text shows as wrong characters, change this and restart the
game.

**Locale** sets `microedition.locale`, for example `en-US`. Some games
pick their language from it.

## Network

Network access is off by default. A game that opens an HTTP or socket
connection then fails the way it would on a phone with no signal.
Turn **Allow network** on for a game that needs it. Only allow games
you trust, since a game with network access can send data out.

## Other settings

- **Volume**: from silent to full, per game.
- **Frame limit**: an upper bound on frames per second.
- **Properties**: extra system properties, and overrides for the
  application properties in the game's `.jad`, one `name=value` per
  line.
