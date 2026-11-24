# How mature J2ME emulators keep real games running

These notes collect what J2ME Loader, freej2me-plus, freej2me,
MicroEmulator, wie and RustJava do to make real MIDlets work, so that
FastEmu gets the same behaviour right the first time. They are research
notes for the people building `fastemu-jvm`, `fastemu-midp`,
`fastemu-gfx` and `fastemu-audio`. They are not a specification; where
the MIDP 2.0 specification is quoted, the quote was checked against the
JSR 118 javadoc that Oracle still hosts.

## Sources

Every code link points at the commit studied, so the line numbers stay
valid.

| Project | Commit studied | License | Language |
| --- | --- | --- | --- |
| [J2ME Loader](https://github.com/nikita36078/J2ME-Loader) | `e270ed91e67b` (2026-11-23) | Apache-2.0, some files LGPL-2.1+ or Apache-2.0, M3G native code EPL-1.0 | Java, C++ (Android) |
| [freej2me-plus](https://github.com/TASEmulators/freej2me-plus) | `aab3350832c0` (2026-11-24, branch `devel`) | GPL-3.0-or-later, bundled parts MIT, BSD-3, LGPL-2.1, Apache-2.0 | Java (desktop, libretro) |
| [freej2me](https://github.com/hex007/freej2me) | `eb4ac87e9a0b` (2026-10-04) | GPL-3.0-or-later, ASM is BSD-3 | Java |
| [MicroEmulator](https://github.com/barteo/microemu) | `e47b656d983d` (2023-03-28) | LGPL-2.1-or-later or Apache-2.0, per file | Java |
| [wie](https://github.com/dlunch/wie) | `eb5b809d693e` (2026-11-24) | MIT | Rust |
| [RustJava](https://github.com/dlunch/RustJava) | `df24aaa7a440` (2026-11-24) | MIT | Rust |

Issue trackers read: J2ME Loader (882 issues), freej2me-plus (269),
freej2me (161), MicroEmulator (117), wie (97), listed with `gh issue
list` on 2026-11-24.

## How to read the notes

Each claim carries a marker that says how it was checked:

- **[code]** read in the source at the commit above.
- **[spec]** checked against the JSR 118 (MIDP 2.0) javadoc.
- **[tracker]** stated in an issue or a commit message; the behaviour
  was not reproduced here.
- **[unverified]** believed true but not confirmed by any of the above.

## The notes

1. [Suite loading](notes/01-suite-loading.md): broken JAR files,
   manifest and JAD parsing, `MIDlet-<n>`, encodings, resource lookup,
   class loading quirks.
2. [Events and threads](notes/02-events-threads.md): which thread runs
   what, repaint coalescing, `serviceRepaints`, `callSerially`, the
   lifecycle around `startApp`, `setCurrent`, vibration and backlight.
3. [Input](notes/03-input.md): vendor key codes with corrections to the
   presets in `crates/fastemu-core/src/device.rs`, `getGameAction`, key
   repeat, `GameCanvas` key states, pointers, soft keys and Commands.
4. [Graphics](notes/04-graphics.md): font sizes, anti-aliasing, drawing
   rules, PNG repair, `getRGB` and `drawRGB`, mutable images, Nokia
   `DirectGraphics`.
5. [Timing](notes/05-timing.md): frame limits, `Thread.sleep`, clocks,
   `Thread.yield` and `System.gc` in game loops.
6. [Record stores](notes/06-rms.md): record ids, sizes, naming, storage,
   corruption handling.
7. [Sound](notes/07-sound.md): formats, unsupported formats,
   `playTone`, `PlayerListener`, Nokia `Sound`.
8. [System properties](notes/08-system-properties.md): what games read
   and what the emulators answer.
9. [Crash causes and per-game settings](notes/09-crashes-and-settings.md):
   the 20 most common crash causes and the settings the emulators expose.
10. [Licenses](notes/10-licenses.md): what FastEmu may reuse and the
    attribution it needs.
11. [Test material](notes/11-test-material.md): freely redistributable
    MIDlets and test suites.

## The rules that matter most

The short version, for anyone who reads nothing else. Each rule links to
the note that backs it.

1. Never let a broken ZIP stop a game: fall back to local file headers,
   ignore CRC errors, decode names as bytes. ([1](notes/01-suite-loading.md#broken-jar-and-zip-files))
2. Find the manifest without regard to case, strip a UTF-8 BOM, and
   join continuation lines by removing exactly one leading space.
   ([1](notes/01-suite-loading.md#manifest-and-jad-parsing))
3. JAD attributes override manifest attributes for untrusted suites.
   ([1](notes/01-suite-loading.md#precedence))
4. Resources: strip leading slashes, collapse `//`, turn `\` into `/`,
   resolve relative names against the package of the class
   `getResourceAsStream` is called on, and fall back to a
   case-insensitive match. ([1](notes/01-suite-loading.md#resource-lookup))
5. Resource streams are fully buffered: `available()` returns the whole
   remaining size and `read(byte[])` fills the buffer. ([1](notes/01-suite-loading.md#resource-streams))
6. `Image.createImage(String)` throws `IOException` for a missing
   resource, never `NullPointerException`; games load images in a loop
   until the exception. ([4](notes/04-graphics.md#image-creation-and-decoding))
7. Run lifecycle calls (`startApp`, `pauseApp`, `destroyApp`) on their
   own thread, not on the event thread, so a game that never returns
   from `startApp` still gets events. ([2](notes/02-events-threads.md#the-lifecycle-thread))
8. Serialize every UI callback on one event thread, catch and log every
   exception thrown by a callback, and keep the thread alive.
   ([2](notes/02-events-threads.md#one-event-thread))
9. `serviceRepaints` called on the event thread paints at once; called
   elsewhere it waits, with a timeout. ([2](notes/02-events-threads.md#servicerepaints))
10. `callSerially` always queues; running it inline overflows the stack
    for games that re-post from `run()`. ([2](notes/02-events-threads.md#callserially))
11. `getGameAction` maps both the arrows and the 2, 4, 6, 8, 5 keys.
    ([3](notes/03-input.md#getgameaction))
12. `GameCanvas.getKeyStates` latches presses until read and returns 0
    while not shown. ([3](notes/03-input.md#gamecanvas-key-states))
13. Return `null` for host properties such as `os.name`; never leak the
    host. Report a realistic power-of-two heap. Fail `Class.forName` for
    vendor APIs the profile does not have. ([8](notes/08-system-properties.md), [9](notes/09-crashes-and-settings.md#emulator-detection))
14. Unsupported audio becomes a silent player, not an exception; detect
    the format from magic bytes when the content type is missing or wrong.
    ([7](notes/07-sound.md#unsupported-formats-and-wrong-content-types))
15. Keep the Canvas back buffer between paints; games repaint only what
    changed. ([2](notes/02-events-threads.md#repaint-coalescing))
16. An exception in a `TimerTask` cancels that task only, not the whole
    `Timer`. ([2](notes/02-events-threads.md#timers))
17. A Thread subclass may declare `stop()`, `suspend()` and `resume()`;
    CLDC has none, so FastEmu's `java.lang.Thread` must not either.
    ([1](notes/01-suite-loading.md#class-loading-quirks))
18. Do not run a strict bytecode verifier; obfuscators emit code javac
    never would. ([1](notes/01-suite-loading.md#class-loading-quirks))
19. Give game threads deep stacks; several games overflow 1 MiB.
    ([9](notes/09-crashes-and-settings.md#the-20-most-common-crash-causes))
20. Treat encoding as a per-game setting that applies to every implicit
    byte-to-char conversion, not only `new String(bytes)`.
    ([1](notes/01-suite-loading.md#encodings))

## Corrections for `fastemu-core`

The full reasoning is in the notes; this is the list for the lead.

- `KeyCodes::NOKIA` matches J2ME Loader, freej2me-plus and
  MicroEmulator. [code]
- `KeyCodes::SIEMENS`: the soft keys, arrows and fire match
  freej2me-plus. The `clear: -12`, `send: -11` and `end: -12` values
  appear in none of the studied emulators, and clear sharing a code with
  end is suspicious. Treat them as unverified. ([3](notes/03-input.md#siemens))
- `KeyCodes::MOTOROLA`: soft keys, arrows and fire match J2ME Loader and
  freej2me-plus. `clear`, `send` and `end` are not defined by either.
  Motorola shipped at least three other layouts. ([3](notes/03-input.md#motorola))
- Presets worth adding: LG, Sagem, Motorola V8, Motorola "triplets",
  Motorola A1000, Nokia E61/E62 QWERTY, Sony Ericsson gaming keys.
  ([3](notes/03-input.md#presets-to-add))
- `SuiteInfo::property` compares names without case and says MIDP asks
  for that. JSR 118 says the opposite for the descriptor: "Attribute
  names are case-sensitive and MUST match exactly." The JAR manifest
  format itself compares names without case; that part was not
  verified here. Ignoring case is harmless, so only the comment needs
  fixing. ([1](notes/01-suite-loading.md#attribute-name-case))
- `FontSizes` should say what the numbers are (pixel line height, the
  value of `Font.getHeight()`); the emulators disagree widely on them.
  ([4](notes/04-graphics.md#font-sizes))
