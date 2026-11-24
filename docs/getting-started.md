# Getting started

FastEmu runs Java ME (J2ME) games and apps from their `.jar` and `.jad`
files. This page takes you from a download to a running game.

## Install

Download the build for your system from the
[latest release](https://github.com/fzn0x/FastEmu/releases/latest).

| System  | What to download                         |
| ------- | ---------------------------------------- |
| Windows | the `.zip`, unpack it anywhere           |
| Linux   | the `.tar.gz`, unpack it anywhere        |
| macOS   | the `.zip`, move the app to Applications |
| Android | the `.apk`, install it from your files   |

FastEmu does not need an installer and does not need Java installed. It
carries its own virtual machine.

## Get a game

FastEmu does not ship games. Use `.jar` files you already own or that
their authors made free to share. A `.jad` file is a small text
descriptor that sits next to a `.jar`. FastEmu reads either one.

## Run your first game

1. Start FastEmu.
2. Drag a `.jar` onto the window, or choose **Add game** and pick the
   file. On Android, choose **Add game** and pick it from storage.
3. The game appears in the library. Open it to start it.

The game runs in its own phone. Its screen size, key codes and saves
come from a device profile. The default is a 240x320 phone with
Nokia key codes, which suits most games. See [Settings](/docs/settings/)
to change it.

## Play

On a keyboard, the arrow keys move, Enter is the fire key and the
number row and number pad press the phone keys. Soft keys are on `Q`
and `E`. On a touch screen, an on-screen keypad appears. The full list
is in [Playing](/docs/playing/).

## Run without a window

The same program can run a game with no window, which is useful for
tests:

```
fastemu run snake.jar --frames 300 --screenshot snake.png
```

It runs 300 frames, saves the last one as a PNG and prints a short
report. It exits with an error code if the game crashes.

## Next

- [Playing](/docs/playing/): the library, several games at once, layouts.
- [Compatibility](/docs/compatibility/): what runs and what does not yet.
- [FAQ](/docs/faq/): common questions.
