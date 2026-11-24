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
