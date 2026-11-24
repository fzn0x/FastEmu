<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logo-dark.svg">
    <source media="(prefers-color-scheme: light)" srcset="assets/logo-light.svg">
    <img src="assets/logo.svg" width="480" alt="FastEmu">
  </picture>
</p>

<p align="center">
  A Java ME emulator written in Rust.<br>
  It runs many MIDP 2.0 games at once, in one window.
</p>

<p align="center">
  <a href="https://github.com/fzn0x/FastEmu/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/fzn0x/FastEmu/actions/workflows/ci.yml/badge.svg"></a>
  <a href="#license"><img alt="GPL-3.0-or-later" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg"></a>
  <a href="https://fastemu.functionx.dev"><img alt="Website" src="https://img.shields.io/badge/site-fastemu.functionx.dev-blue.svg"></a>
</p>

---

FastEmu runs Java ME games, the `.jar` and `.jad` files made for phones
of the 2000s, on Windows, Linux, macOS and Android. It implements CLDC 1.1
and MIDP 2.0 with a virtual machine of its own, so it needs no Java
installed. Each game runs in its own virtual machine with its own memory,
threads, saves and sound, and one window shows as many games as you start.
A game that crashes or hangs does not stop the ones next to it.

[Install](#install) · [Usage](#usage) · [Supported APIs](#supported-apis) ·
[Build from source](#building-from-source) · [Contributing](CONTRIBUTING.md) ·
[Changelog](CHANGELOG.md) · [Bahasa Indonesia](README.id.md)

## Install

Download the file for your system from the
[Releases page](https://github.com/fzn0x/FastEmu/releases/latest). Each
release has a `SHA256SUMS` file to check the downloads against.

| System | What to do |
| --- | --- |
| Windows | Unpack `fastemu-<version>-windows-x64.zip` anywhere and run `fastemu.exe`. If SmartScreen warns, choose More info, then Run anyway: the program is not signed. |
| Linux | Unpack `fastemu-<version>-linux-x64.tar.gz` (or `-linux-arm64`) anywhere and run `fastemu`, or install `fastemu-<version>-linux-x64.deb` on Debian and Ubuntu. It needs a Vulkan driver and a Wayland or X11 session. |
| macOS | Unpack `fastemu-<version>-macos-universal.zip` and move the app to Applications. The app is not notarized: open it once from System Settings, Privacy & Security, Open Anyway. |
| Android | Open `fastemu-<version>-android.apk` from your files. Android asks you to allow installs from that source the first time. |

FastEmu does not include games. Use files you own or that their authors
made free to share.

## Usage

**Adding games.** Drag `.jar` or `.jad` files onto the window, choose Add
game, or pass the files on the command line:

```sh
fastemu snake.jar tetris.jar
```

On Android, choose Add game and pick the file from storage. FastEmu reads
the name, vendor, version and icon from the manifest or the descriptor and
lists the game in its library.

**Running several games.** Start another game from the library while one
runs and it opens next to the first. You can start the same game twice.
Sound plays from the game you focus. The window shows the running games in
a grid, or one large with the others beside it, or one alone.

**Keys.** The keyboard maps to the phone keypad:

| Phone key | Keyboard |
| --- | --- |
| 0 to 9 | `0` to `9` on the number row and the number pad |
| Up, down, left, right | Arrow keys |
| Fire | Enter or Space |
| Left and right soft keys | `Q` and `E` |

Every binding can be changed in the settings. On a touch screen FastEmu
draws a keypad, and games that read touch get the taps and drags. The key
codes a game sees come from its device profile, because phone makers used
different codes: Nokia, Siemens and Motorola layouts are built in.

The [documentation](https://fastemu.functionx.dev) has the full list of
keys, the settings and the device profiles.

## Supported APIs

FastEmu targets the platform that games of that time were written for:

- CLDC 1.1 and its `java.lang`, `java.util` and `java.io` subset
- MIDP 2.0: the `lcdui` screens and the `Canvas`, record stores, the
  Generic Connection Framework, and the MIDlet life cycle
- The MIDP 2.0 game API
- Mobile Media API (MMAPI, JSR 135) for sound and MIDI
- Nokia UI API

Which games work is tracked in the
[compatibility list](https://fastemu.functionx.dev), built from the
reports people send. A game that does not work belongs in a
[game report](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).

## Building from source

You need [rustup](https://rustup.rs). The repository pins its toolchain in
`rust-toolchain.toml`, so the right Rust installs on the first build.

```sh
git clone https://github.com/fzn0x/FastEmu.git
cd FastEmu
cargo test            # the emulator crates
cargo run -p fastemu  # the desktop app
```

On Linux the desktop app also needs a C compiler and the development
packages of GPUI: fontconfig, xkbcommon, Wayland, X11 and Vulkan. On
macOS it needs Xcode, whose Metal toolchain compiles GPUI's shaders. The
virtual machine's tests compare its results with a real JDK, so install JDK
8 or newer for them. [CONTRIBUTING.md](CONTRIBUTING.md) has the exact
packages, and [docs/building-android.md](docs/building-android.md) the
Android app's steps.

## How it is built

A Rust workspace of small crates. `fastemu-core` holds the types that the
runtime and the apps share: frames, input events, device profiles and the
launcher that starts a game. `fastemu-jvm` is the virtual machine and its
class libraries, `fastemu-gfx` the 2D graphics, `fastemu-audio` sound and
MIDI, and `fastemu-midp` the MIDP, game, media and Nokia APIs that games
call. `fastemu-ui` is the GPUI interface that the desktop program
`fastemu` and the Android app `fastemu-android` both use.

## Credits

[MicroEmulator](https://github.com/barteo/microemu) and
[J2ME Loader](https://github.com/nikita36078/J2ME-Loader) kept these games
playable for more than ten years. FastEmu follows their behavior where a
game depends on it, and owes them the knowledge of which quirks matter.

The interface is built with [GPUI](https://github.com/zed-industries/zed),
the UI framework of the Zed editor. GPUI on Android runs on the Zed Android
port by Dylan Murzello.

The Rust crates and fonts FastEmu uses, with their licenses, are listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The people who helped are
in [CONTRIBUTORS.md](CONTRIBUTORS.md).

## License

FastEmu is free software under the [GNU General Public License v3.0 or
later](LICENSE).
