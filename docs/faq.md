# FAQ

## What is Java ME?

Java ME (J2ME) was the Java edition for feature phones. Most games from
the 2000s on Nokia, Sony Ericsson, Siemens and Motorola phones are
`.jar` files for it. FastEmu runs them on current computers and phones.

## Does FastEmu come with games?

No. Use files you own or that their authors allow you to share.

## Is it free?

Yes. FastEmu is free software under the GPL-3.0-or-later license. The
source is on [GitHub](https://github.com/fzn0x/FastEmu).

## Do I need Java installed?

No. FastEmu has its own virtual machine for Java ME.

## Which systems does it run on?

Windows on x64, Linux on x64 and arm64, macOS 11 or later on Apple
silicon and Intel, and Android 12 or later with a Vulkan driver.

## Why many games at once?

Each game runs as its own phone, with its own saves and sound, so you
can keep several in one window: compare two, or keep a few paused and
switch between them. See [Playing](/docs/playing/).

## A game shows a white or black screen. What now?

Check the game's log first. Then try another device profile with the
screen size the game was made for. See
[Compatibility](/docs/compatibility/).

## A game does not react to clicks or taps on its screen.

Its phone has no touch screen. Turn on **Touch screen** in the game's
settings, or pick the touch profile, then restart the game. See
[Settings](/docs/settings/).

## Does it run 3D games?

Not yet. Games that need Mobile 3D Graphics (JSR-184) do not run, or
draw nothing. See [Compatibility](/docs/compatibility/).

## Text shows as wrong characters.

Change the encoding in the game's settings. See
[Settings](/docs/settings/).

## Where are my saves?

Each running copy of a game has its own record stores, in a `games`
folder of FastEmu's data folder.
[Settings](/docs/settings/#where-fastemu-keeps-its-files) says where that
is on each system. Each game's folder is named after its number in the
library, so back up or move the `games` folder together with
`library.json`.

## Can a game use the internet?

Only if you allow it for that game. It is off by default.

## Does it work with other emulators' saves?

Not yet. Saves from other emulators use their own formats.

## How is FastEmu related to MicroEmulator and J2ME Loader?

They are separate projects. FastEmu is a new program, written in Rust,
that owes much to them. Both kept these games playable for more than
ten years. FastEmu's interface builds on Zed's GPUI.

## How do I report a bug or a game that fails?

See [Reporting a game](/docs/compatibility/#reporting-a-game).
