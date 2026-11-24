# Compatibility

FastEmu targets the Java ME that games of the 2000s used. This page
says what is covered and what is not. It describes the first release,
v0.1.0, and will change as support grows.

## What works

**CLDC 1.1.** The core language and class libraries of the Connected
Limited Device Configuration: threads, `java.lang`, `java.util`,
`java.io`, floating point, and the Generic Connection Framework.

**MIDP 2.0.** The Mobile Information Device Profile: the `MIDlet`
life cycle, `javax.microedition.lcdui` (forms and their items, lists,
text boxes, alerts, tickers and custom canvases), record stores for
saves, and the connections of `javax.microedition.io` (HTTP, HTTPS,
sockets and datagrams), which stay off until you allow a game network
access. The push registry takes a game's requests but never starts a
game by itself.

**Messaging.** The Wireless Messaging API (JSR-120) is there so games
that use it start, but no message is sent or received: a game that
sends an SMS is told the send failed, and the attempt shows in its log.

**Game API.** `javax.microedition.lcdui.game`: `Sprite`, `TiledLayer`,
`LayerManager` and `GameCanvas`.

**MMAPI sound.** The Mobile Media API (JSR 135) for sound: tones and
tone sequences, MIDI, WAV (PCM and ADPCM), MP3 and AMR playback, volume
control.

**Nokia UI API.** `com.nokia.mid.ui` for full-screen canvases,
`DirectGraphics` and device control such as vibration and backlight
calls, and `com.nokia.mid.sound` for the Nokia sound classes. Games
see them on the Nokia, Sony Ericsson and generic profiles.

**Other makers' APIs.** Siemens' game and colour game packages and its
`ui.Image` class on the Siemens profile, and Motorola's backlight class
on the Motorola profile, Samsung's sound clip, backlight and vibration
classes on the Samsung profile, and LG's backlight and vibration classes
on the LG profile.

## What does not work yet

- **3D with JSR-184 (Mobile 3D Graphics) or Mascot Capsule.** Games
  that need it will not run or will draw nothing.
- **Bluetooth (JSR-82).** Its classes are missing, as on a phone
  without Bluetooth: a game that checks for them first carries on
  without, and one that calls them fails with an error in its log.
- **File connections (JSR-75).** `file://` connections fail. Games keep
  their saves in record stores, which do work.
- **Rarer vendor APIs.** Siemens' phone, file and media classes,
  Motorola's game and FunLight packages, Sony Ericsson's Capuchin and
  LG's other MMPP classes are missing. A game that needs one may fail
  when it uses it.
- **Native libraries and games that need a particular phone's
  firmware.** These cannot run.

When a game calls something FastEmu does not have, it gets a clear
error in the game's log instead of a hang.

## Tested games

These are free games that FastEmu was run with on 24 November 2026.
`cargo xtask compat` ran each for 20 seconds without a window, on the
240x320 profile, pressing 5 after a pause and then the arrows and
fire, and the touch games were played with `fastemu run --touch`.
"Plays" means the game kept drawing new frames and its frames showed
the game being played; nobody played any game through, so a later
level may still fail.

| Game | License | Result | Notes |
| --- | --- | --- | --- |
| [Retro Snake, Block Fall, Brick Buster, Pixel Jumper, Traffic Dash, Tower Guard, Star Defender, Dungeon Quest, Rock Blaster, Ladder Loot](https://github.com/agneay/j2me-100-games) (J2ME 100 Games) | MIT | Plays | Press 5 on the title screen |
| [Tetris](https://github.com/MarkLagodych/j2me-tetris) by Mark Lagodych | WTFPL | Plays | It draws a new frame only when its piece moves |
| [Stick Fight](https://github.com/Dahmalahi/STICK-FIGHT-OFFICIEL-REPO) by Dash Animation V2 | MIT | Plays | `GameCanvas` and sound |
| [Shooter](https://github.com/RehanSaeed/Shooter) by Muhammad Rehan Saeed | MIT | Plays | A `List` menu, then `GameCanvas` with sprites and tiled layers |
| [BombMan](https://github.com/yaricom/bombman-RL-AI-J2ME) by Iaroslav Omelianenko | MIT | Plays | Nokia's `FullCanvas` and sound, so it needs a Nokia, Sony Ericsson or generic profile |
| [Skocko](https://github.com/manojlovic/skocko-j2me) by Milos Manojlovic | MIT | Plays | `Form` and `List` screens |
| [Lines](https://github.com/mrlordkaj/j2me-lines) by Thinh Pham | GPL-3.0-or-later | Plays | A touch game: use the `generic-360x640` profile, or turn on the touch screen in its settings |
| [Pixelus](https://github.com/mrlordkaj/j2me-pixelus) by Thinh Pham | GPL-3.0-or-later | Plays | A 320x240 touch game: use `generic-320x240` and turn on the touch screen in its settings |
| [mobapp-game](https://github.com/vipaoL/mobapp-game) by vipaoL | GPL-3.0-only | Plays | Played by touch |
| [JTube](https://github.com/shinovon/JTube) by Arman Jussupgaliyev | MIT | Runs | An app, not a game: its screens work, and it needs network access, off by default, to load anything |
| [3D Moto 2](https://github.com/mrlordkaj/j2me-3dmoto) by Thinh Pham | GPL-3.0-or-later | Shows its first screens | Needs Mobile 3D Graphics (JSR-184) to play |

The list comes from `cargo xtask compat`, which runs every JAR in a
folder this way and writes a report with each game's status, frames,
errors, the classes it asked for that FastEmu lacks, its first
exception, a screenshot and its log:

```
cargo xtask compat path/to/games --seconds 20
```

## Why a game may misbehave

Most problems come from one of four things:

1. **Wrong device profile.** The game expects another screen size or
   other key codes. See [Settings](/docs/settings/).
2. **No touch screen.** A game played by touch gets no taps on a phone
   without one. Turn on **Touch screen** in its settings.
3. **Wrong encoding.** Text shows as wrong characters. Change the
   encoding in the settings.
4. **A missing API.** The log names the class or method.

## Reporting a game

If a game does not run, open a
[game report](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).
Please include:

- the game's name, vendor and version, from the library, and the size
  of its JAR;
- your system and the FastEmu version;
- the device profile and settings you used;
- the log of the game, copied from the log panel;
- what you expected and what happened.

Do not attach copyrighted games. Say where the game can be found
instead. For a crash, the headless runner gives a log you can paste:

```
fastemu run game.jar --frames 300 --screenshot game.png
```
