# Compatibility

FastEmu targets the Java ME that games of the 2000s used. This page
says what is covered and what is not. It describes the first release,
v0.1.0, and will change as support grows.

## What works

**CLDC 1.1.** The core language and class libraries of the Connected
Limited Device Configuration: threads, `java.lang`, `java.util`,
`java.io`, floating point, and the Generic Connection Framework.

**MIDP 2.0.** The Mobile Information Device Profile: the `MIDlet`
life cycle, `javax.microedition.lcdui` (forms, lists, alerts and
custom canvases), `GameCanvas`, record stores for saves, and the
Push-free parts of `javax.microedition.io`.

**Game API.** `javax.microedition.lcdui.game`: `Sprite`, `TiledLayer`,
`LayerManager` and `GameCanvas`.

**MMAPI sound.** The Mobile Media API (JSR 135) for sound: tone
sequences, MIDI and WAV playback, volume control.

**Nokia UI API.** `com.nokia.mid.ui` for full-screen canvases and
device control such as vibration and backlight calls, and
`com.nokia.mid.sound` for the Nokia sound classes.

## What does not work yet

- **3D with JSR-184 (Mobile 3D Graphics).** Games that need it will
  not run or will draw nothing.
- **Bluetooth (JSR-82).** Calls fail as if the phone had no Bluetooth.
- **File connections (JSR-75).** `file://` connections fail. Games keep
  their saves in record stores, which do work.
- **Other vendor APIs.** Samsung, Siemens, Motorola and Sony Ericsson
  extras are mostly missing. A game that needs one may fail when it
  uses it.
- **Native libraries and games that need a particular phone's
  firmware.** These cannot run.

When a game calls something FastEmu does not have, it gets a clear
error in the game's log instead of a hang.

## Tested games

These are free games that FastEmu was run with on 24 November 2026.
Each ran for 20 seconds without a window, on the 240x320 profile,
with 5 pressed after a pause and then the arrows and fire. "Plays"
means the game kept drawing new frames to the end and its last frame
showed the game being played; nobody played any game through, so a
later level may still fail. The build of that day did not yet have the
high-level screens (`Form`, `List`, `Alert`), `GameCanvas` with its
layers, or Nokia's `FullCanvas`, which are being written for v0.1.0;
most of the games that stop here stop on one of them.

| Game | License | Result | What it stops on |
| --- | --- | --- | --- |
| [Retro Snake, Block Fall, Brick Buster, Pixel Jumper, Traffic Dash, Tower Guard, Star Defender, Dungeon Quest, Rock Blaster, Ladder Loot](https://github.com/agneay/j2me-100-games) (J2ME 100 Games) | MIT | Plays | Nothing in this run |
| [Tetris](https://github.com/MarkLagodych/j2me-tetris) by Mark Lagodych | WTFPL | Plays | Nothing in this run |
| [3D Moto 2](https://github.com/mrlordkaj/j2me-3dmoto) by Thinh Pham | GPL-3.0-or-later | Stays on its "Fetching Data.." screen | `DateField`, in its loading thread; it also asks for `GameCanvas`, and it needs Mobile 3D Graphics (JSR-184) to play |
| [JTube](https://github.com/shinovon/JTube) by Arman Jussupgaliyev | MIT | Shows its splash screen, then stops drawing | `GameCanvas`, in its interface thread |
| [Stick Fight](https://github.com/Dahmalahi/STICK-FIGHT-OFFICIEL-REPO) by Dash Animation V2 | MIT | Stops at start | `GameCanvas` |
| [Lines](https://github.com/mrlordkaj/j2me-lines) by Thinh Pham | GPL-3.0-or-later | Stops at start | `GameCanvas`; it is played with a touch screen |
| [Pixelus](https://github.com/mrlordkaj/j2me-pixelus) by Thinh Pham | GPL-3.0-or-later | Stops at start | `GameCanvas`; it is played with a touch screen |
| [mobapp-game](https://github.com/vipaoL/mobapp-game) by vipaoL | GPL-3.0-only | Stops at start | `GameCanvas` |
| [Shooter](https://github.com/RehanSaeed/Shooter) by Muhammad Rehan Saeed | MIT | Stops at start | `List`; it also uses `GameCanvas` and the layers |
| [Skocko](https://github.com/manojlovic/skocko-j2me) by Milos Manojlovic | MIT | Stops at start | `Form` |
| [BombMan](https://github.com/yaricom/bombman-RL-AI-J2ME) by Iaroslav Omelianenko | MIT | Closes itself at start | Nokia's `FullCanvas` |

Tetris draws a new frame only when its piece moves, about once a
second, so `fastemu run Tetris.jar --frames 300` takes some four
minutes; give it `--seconds` instead.

The list comes from `cargo xtask compat`, which runs every JAR in a
folder this way and writes a report with each game's status, frames,
errors, the classes it asked for that FastEmu lacks, its first
exception, a screenshot and its log:

```
cargo xtask compat path/to/games --seconds 20
```

## Why a game may misbehave

Most problems come from one of three things:

1. **Wrong device profile.** The game expects another screen size or
   other key codes. See [Settings](/docs/settings/).
2. **Wrong encoding.** Text shows as wrong characters. Change the
   encoding in the settings.
3. **A missing API.** The log names the class or method.

## Reporting a game

If a game does not run, open an issue at
[github.com/fzn0x/FastEmu/issues](https://github.com/fzn0x/FastEmu/issues).
Please include:

- the game's name, vendor and version, from the library;
- your system and the FastEmu version;
- the device profile and settings you used;
- the log of the game, copied from the log panel;
- what you expected and what happened.

Do not attach copyrighted games. Say where the game can be found
instead. For a crash, the headless runner gives a log you can paste:

```
fastemu run game.jar --frames 300 --screenshot game.png
```
