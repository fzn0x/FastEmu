# 9. Crash causes and per-game settings

The most common reasons games crash or hang in the trackers, how each
was fixed, and the per-game settings the emulators expose. Link bases
are as in [note 1](01-suite-loading.md).

## How the list was built

The trackers were listed with `gh issue list` on 2026-11-24: J2ME Loader
882 issues (334 labelled `bug`, 17 `bytecode verification`, 12
`mascot capsule`), freej2me-plus 269, freej2me 161, MicroEmulator 117.
A keyword grouping of titles and bodies (crude; one issue can match
several groups) gives a sense of proportion:

| Group | J2ME Loader | freej2me-plus | freej2me | MicroEmulator |
| --- | --- | --- | --- | --- |
| Crash or error | 184 | 76 | 40 | 41 |
| Keys and input | 175 | 48 | 31 | 33 |
| Freeze or hang | 116 | 55 | 24 | 17 |
| Graphics glitches | 110 | 63 | 25 | 12 |
| Sound | 88 | 57 | 29 | 2 |
| 3D (M3G, Mascot Capsule) | 86 | 41 | 14 | 0 |
| Fonts, text, encoding | 80 | 53 | 20 | 33 |
| Speed and frame rate | 80 | 35 | 7 | 3 |
| Saves (RMS) | 42 | 29 | 14 | 10 |

Most reports carry a game name and a screenshot, not a stack trace; the
exceptions that are named most often are `NullPointerException`,
`ClassNotFoundException`/`NoClassDefFoundError`, `NoSuchMethodError`,
`IllegalArgumentException` and `StackOverflowError`. The ranking below
weighs how often a cause appears and how many games one fix rescued.
Each entry names the evidence. [tracker unless marked]

## The 20 most common crash causes

1. **Missing API classes.** `ClassNotFoundException` or
   `NoClassDefFoundError` for vendor or optional APIs: Nokia UI 2.0
   (`com.nokia.mid.ui.frameanimator`), `com.nokia.mid.s40.io`
   ([J2ME Loader 1093](https://github.com/nikita36078/J2ME-Loader/issues/1093),
   [758](https://github.com/nikita36078/J2ME-Loader/issues/758)),
   BlackBerry `net.rim` ([809](https://github.com/nikita36078/J2ME-Loader/issues/809);
   [freej2me-plus 291](https://github.com/TASEmulators/freej2me-plus/issues/291),
   [306](https://github.com/TASEmulators/freej2me-plus/issues/306)),
   `javax.bluetooth.ServiceRecord`
   ([freej2me 99](https://github.com/hex007/freej2me/issues/99)).
   Fix: stub classes that load, return neutral values and log; resolve
   classes lazily so a missing one fails only when used.
2. **Missing methods in implemented classes.** `NoSuchMethodError`:
   Turrican calls `Sprite.setRefPixelPosition`, which freej2me-plus had
   removed by mistake ([36](https://github.com/TASEmulators/freej2me-plus/issues/36)).
   Fix: implement the whole MIDP 2.0 and CLDC 1.1 API surface and test it
   against the JSR class lists.
3. **Wrong exception type from the runtime.** Games catch the exception
   the spec names; Blue Sphere Games titles loop over images until
   `IOException` and died on `NullPointerException`
   ([119](https://github.com/TASEmulators/freej2me-plus/issues/119)).
   Fix: throw exactly the documented exceptions.
4. **Resource not found.** Case, leading or doubled slashes, backslashes,
   relative names and locale-specific names
   ([freej2me 131](https://github.com/hex007/freej2me/issues/131); J2ME
   Loader commits 4c650d52, 9ed5231e, e0c70fa5). Fix: see
   [resource lookup](01-suite-loading.md#resource-lookup).
5. **Manifest or JAD not parsed.** Lower-case `meta-inf`, wrapped lines,
   a BOM, `MANIFEST.FM`, a missing comma in `MIDlet-1`, a JAD whose name
   no longer matches the JAR
   ([freej2me 62](https://github.com/hex007/freej2me/issues/62),
   [226](https://github.com/hex007/freej2me/issues/226);
   [J2ME Loader 865](https://github.com/nikita36078/J2ME-Loader/issues/865)).
   Fix: see [parsing](01-suite-loading.md#manifest-and-jad-parsing).
6. **Main class not started.** `startApp` inherited from the game's own
   base class ([freej2me 6](https://github.com/hex007/freej2me/issues/6)),
   no `MIDlet-1` at all. Fix: virtual dispatch; find `MIDlet` subclasses
   when the descriptor names none.
7. **Thread subclasses with `stop`, `suspend` or `resume`.** A
   `LinkageError` on Java SE based runtimes
   ([J2ME Loader MethodRenamer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/MethodRenamer.java)). [code]
   Fix for FastEmu: a CLDC-only `java.lang.Thread`.
8. **Bytecode the verifier rejects.** `VerifyError` from `boolean`
   returns (SimCity Societies and others,
   [J2ME Loader 194](https://github.com/nikita36078/J2ME-Loader/issues/194)),
   from bad stub signatures in early Siemens APIs
   ([freej2me-plus 123](https://github.com/TASEmulators/freej2me-plus/issues/123)),
   from odd frames ([J2ME Loader 814](https://github.com/nikita36078/J2ME-Loader/issues/814)).
   Fix: an interpreter that type-checks lazily and narrows small types as
   the JVMS says; correct stub signatures.
9. **Stack overflow.** Plants vs. Zombies
   ([742](https://github.com/nikita36078/J2ME-Loader/issues/742)),
   Viber 2.22 ([758](https://github.com/nikita36078/J2ME-Loader/issues/758)),
   Snow Rally Canada ([712](https://github.com/nikita36078/J2ME-Loader/issues/712))
   overflow a 1 MiB thread stack; `callSerially` run inline and listener
   recursion overflow too (J2ME Loader commit f78c98c8,
   [freej2me 200](https://github.com/hex007/freej2me/issues/200)).
   Fix: interpreter frames on the heap or large thread stacks, a depth
   limit that throws `StackOverflowError` into the game instead of
   killing the host thread, and queued `callSerially`.
10. **Emulator and host detection.** See [below](#emulator-detection).
11. **Text encoding.** Garbled text, and saves that fail for games with
    non-ASCII names ([freej2me-plus 167](https://github.com/TASEmulators/freej2me-plus/issues/167),
    [140](https://github.com/TASEmulators/freej2me-plus/issues/140)).
    Fix: per-game encoding for every implicit conversion; store names
    inside files, not as file names.
12. **Record store failures.** Negative next id (Pinball 3D Timeshock,
    [freej2me 152](https://github.com/hex007/freej2me/issues/152)),
    corrupted stores that hang the game at every start
    ([J2ME Loader 536](https://github.com/nikita36078/J2ME-Loader/issues/536),
    commit 9d308f83), endless enumerations (freej2me-plus commit d5bbd5a),
    saves not loaded after a restart (Ghouls 'n Ghosts,
    [140](https://github.com/TASEmulators/freej2me-plus/issues/140);
    Need for Speed Most Wanted, [247](https://github.com/TASEmulators/freej2me-plus/issues/247)).
    Fix: see [record stores](06-rms.md).
13. **Players used off-spec.** No content type
    ([freej2me 202](https://github.com/hex007/freej2me/issues/202)),
    Samsung `AudioClip` with loop -1 or a `null` stream
    ([freej2me-plus 122](https://github.com/TASEmulators/freej2me-plus/issues/122),
    [216](https://github.com/TASEmulators/freej2me-plus/issues/216)),
    `stop()` on a stopped player and `deallocate` crashes in Orcs and
    Elves ([freej2me 200](https://github.com/hex007/freej2me/issues/200),
    [J2ME Loader 560](https://github.com/nikita36078/J2ME-Loader/issues/560)),
    Disney-Pixar Up crashing on effects
    ([freej2me-plus 20](https://github.com/TASEmulators/freej2me-plus/issues/20)).
    Fix: see [sound](07-sound.md).
14. **Freezes from locking in the event and paint paths.** Ratatouille
    and Garfield's House froze, Heroes Lore lagged on input
    ([freej2me-plus 17](https://github.com/TASEmulators/freej2me-plus/issues/17),
    [88](https://github.com/TASEmulators/freej2me-plus/issues/88),
    [7](https://github.com/TASEmulators/freej2me-plus/issues/7)).
    freej2me-plus also has an "immediate repaints" flag for games that
    "might deadlock by queuing up paint events that acquire a lock
    required by something else"
    ([commit a298580](https://github.com/TASEmulators/freej2me-plus/commit/a298580)). [code]
    Fix: see [events](02-events-threads.md#servicerepaints).
15. **Uncaught exceptions in callbacks.** A game's own exception in
    `paint` or a serial runnable ends the event thread: Metal Gear Classic
    (`NullPointerException` in `Main.run` on the event thread,
    [154](https://github.com/TASEmulators/freej2me-plus/issues/154)),
    Abracadaball ("Exception hit in paint(graphics)/ by zero",
    [47](https://github.com/TASEmulators/freej2me-plus/issues/47)).
    Fix: catch, log and continue, per event.
16. **Exceptions in TimerTasks.** On Java SE they stop the whole timer;
    J2ME Loader cancels only the failing task
    ([custom Timer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/custom/Timer.java#L288)). [code]
17. **Nokia DirectGraphics formats.** Virtual Me passed the native format
    8888 to the `short[]` `getPixels` and crashed
    ([J2ME Loader 802](https://github.com/nikita36078/J2ME-Loader/issues/802)).
    Fix: native format 565 or 444, all formats supported.
18. **Strict argument checks in Graphics.** `copyArea` outside the image
    (Battle City, [freej2me-plus 181](https://github.com/TASEmulators/freej2me-plus/issues/181)),
    out-of-range `drawRegion`, unusual anchors. Fix: clip instead of
    throwing where the spec allows it; accept lenient anchors.
19. **Model checks.** Games compare `microedition.platform` with the
    phone they were built for (Harry Potter: Find Scabbers,
    [18](https://github.com/TASEmulators/freej2me-plus/issues/18);
    Nokia 3250 Tunes Studio, [143](https://github.com/TASEmulators/freej2me-plus/issues/143)).
    Fix: profiles with real platform strings.
20. **Lifecycle and background.** Games that loop inside `startApp`
    (wie starts its event loop only after `startApp`), games that freeze
    after the app was in the background with music playing
    ([J2ME Loader 678](https://github.com/nikita36078/J2ME-Loader/issues/678),
    [669](https://github.com/nikita36078/J2ME-Loader/issues/669)), games
    that never finish `destroyApp`. Fix: see
    [lifecycle](02-events-threads.md#the-lifecycle-thread).

3D is a large group of its own (M3G and Mascot Capsule: 86 J2ME Loader
and 41 freej2me-plus reports) but mostly about rendering, not crashes,
and is out of FastEmu's first scope.

## Emulator detection

From [freej2me-plus issue 158](https://github.com/TASEmulators/freej2me-plus/issues/158),
[J2ME Loader issue 872](https://github.com/nikita36078/J2ME-Loader/issues/872)
and freej2me-plus's code, games (mostly Chinese releases) check:

| Check | Example | What FastEmu should return |
| --- | --- | --- |
| `Runtime.totalMemory() == 8000000` | Zombie Crisis parts 1 and 2; The Fox Immortal's Romance | the profile heap, a power of two |
| total memory far above a phone's | "can only be played on mobile phones" | the profile heap |
| `Class.forName("emulator.Emulator")` (KEmulator's class) | The Fox Immortal's Romance | `ClassNotFoundException` |
| `Class.forName("java.applet.Applet")` | The Fox Immortal's Romance | `ClassNotFoundException` |
| `Class.forName` of vendor classes such as `com.sprintpcs.*` | Chinese DRM ([MIDletEnhancements.checkClassName](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletEnhancements.java#L118)) [code] | `ClassNotFoundException` unless the profile is that vendor's |
| `os.name` contains `Windows` | The Great Condor Hero; J2ME Loader issue 863 | `null` |
| `platformRequest("http://...")` succeeding | The Fox Immortal's Romance quits if it returns `true` | `false`, or a `ConnectionNotFoundException` when networking is off |
| the clock past a release date | "Authorization expires" | a per-game clock offset |

freej2me-plus's maintainer declined to add per-game bypasses beyond the
class check because "Memory alone could be any given value depending on
the target device". [tracker] Profiles that look like real phones avoid
most of these checks without game-specific code. Expose vendor APIs per
profile, not all at once: a game that finds `com.siemens.mp.game.Light`
may decide it runs on a Siemens and use Siemens key codes. [unverified
as a tracker report; it follows from the class-probe pattern above]

## Per-game settings the emulators expose

**J2ME Loader** (one JSON profile per game,
[ProfileModel](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/ru/playsoftware/j2meloader/config/ProfileModel.java)) [code]:

- screen width and height (default 240x320), background colour, scale
  ratio, scale type, gravity, orientation, bilinear filter, shaders;
- graphics mode (software, OpenGL, hardware canvas), parallel redraw,
  immediate event mode;
- show FPS, FPS limit, force full screen;
- font sizes small, medium, large (default 18, 22, 26), "apply
  dimensions" (density scaling), font anti-aliasing;
- touch input on or off; on-screen keypad type, shape, opacity, colours,
  hide delay, haptic feedback;
- key code layout (default, Siemens, Motorola, Nokia E62, custom) and
  host key mappings;
- MIDI sound bank;
- system properties (free text, `key: value` lines);
- disable `pauseApp`;
- M3G "force disable lighting".

**freej2me-plus** (one `game.conf` per game,
[Config](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/freej2me/Config.java#L157)) [code]:

- `scrwidth`, `scrheight`, `rotate`, `phone` (key layout: Standard,
  Nokia, Nokia keyboard, Siemens, Motorola, Motorola V8, Motorola
  triplets, Motorola A1000, Sagem, LG, KDDI, SKT, BlackBerry), `fps`
  (default 60), `fontoffset`, `backlightcolor`;
- speed hacks: `spdhacknoalpha` (blank images without alpha),
  `fpshack` (unlock frame rate), M3G and Mascot Capsule half resolution
  and no lighting;
- compatibility flags, each added for named games: `compatfantasyzonefix`,
  `compattranstooriginonreset`, `compatimmediaterepaints`,
  `compatrepaintonsetcurrent` (Need for Speed: Most Wanted 2005,
  Ratatouille), `compatoverrideplatchecks` (on by default),
  `compatsiemensfriendlydrawing`, `compatignorevolumechanges`,
  `compatnotranslatedrawrgb` (Peggle, Sony Ericsson versions),
  `compatmcv3horizfovfix` (a `System.gc` setting existed for two weeks
  and was removed, see [timing](05-timing.md#systemgc));
- M3G quality settings (anti-aliasing, bilinear filtering, dithering,
  perspective correction, mipmaps, fog);
- `dojaversion` for Japanese DoJa games.

**Settings FastEmu already has:** `DeviceProfile` (screen, platform, key
codes, touch, colours, alpha levels, heap, font sizes) and
`InstanceSettings` (encoding, locale, volume, network, `max_fps`,
extra properties).

**Settings worth adding**, each backed by a section of these notes:
font size offset ([4](04-graphics.md#font-sizes)), game-action table
and key layout separate from the profile ([3](03-input.md)), force full
screen and soft-key bar behaviour ([3](03-input.md#soft-keys-and-commands-on-a-canvas)),
native pixel format ([4](04-graphics.md#nokia-directgraphics)), RMS quota
([6](06-rms.md)), clock offset ([5](05-timing.md)),
disable `pauseApp`, and named compatibility flags added only together
with the game that needs them.
