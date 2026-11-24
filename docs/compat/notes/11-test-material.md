# 11. Test material

MIDlets, class files and test suites that FastEmu can use in automated
tests, with their licenses. Licenses were read from the GitHub API or
the repository on 2026-11-24. Commercial games are not listed: they
cannot be redistributed, so tests must not depend on them; keep them for
manual runs on a developer's own machine.

## Ready-made class files and JARs

| What | License | Where | Use |
| --- | --- | --- | --- |
| RustJava test data: 137 test programs as `.class` files with expected output in `.txt`, plus `test.jar`, `jar/no-manifest.jar`, `jar/no-main-class.jar` | MIT | [RustJava/test-data](https://github.com/dlunch/RustJava/tree/df24aaa7a440815b12a96e2d7845fec2399cfbf3/test-data) | JVM conformance: arrays, casts, class init failures, dispatch, exceptions, strings; JAR edge cases |
| MicroEmulator `bytecode-test-app.jar` | LGPL-2.1+ or Apache-2.0 | [microemu-javase/src/test/resources](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/test/resources) | resource loading, static initializers, timers, `Throwable` handling |
| wie `resource.jar` | MIT | [wie-skvm/tests/data](https://github.com/dlunch/wie/tree/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-skvm/tests/data) | resource lookup |
| Stick Fight, `StickFightMIDlet.jar` (157 KB) | MIT | [Dahmalahi/STICK-FIGHT-OFFICIEL-REPO releases](https://github.com/Dahmalahi/STICK-FIGHT-OFFICIEL-REPO/releases) | a small Canvas game |
| 3D Moto 2.0, `3DMoto2.zip` (4.4 MB) | GPL-3.0 | [mrlordkaj/j2me-3dmoto releases](https://github.com/mrlordkaj/j2me-3dmoto/releases) | a full game |
| JTube 2.90.1, three JARs (generic, Samsung and BlackBerry builds, 157 to 160 KB) | MIT | [shinovon/JTube releases](https://github.com/shinovon/JTube/releases) | a high-level UI app: `Form`, `List`, `Command`, images, threads; network off |

The licenses above are what the GitHub API reports for each repository;
the content of the release archives was not inspected. [unverified]

## Sources to build test MIDlets from

| What | License | Where | Covers |
| --- | --- | --- | --- |
| MicroEmulator demo MIDlet (`SimpleDemoMIDlet` with Alert, Canvas, GameCanvas, key, pointer, Form item, List, TextBox panels) | LGPL-2.1+ or Apache-2.0 | [microemu-examples/microemu-demo](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-examples/microemu-demo) | LCDUI, input |
| MicroEmulator test MIDlets (`RecordStoreForm`, `ThreadTestsForm`, `ErrorHandlingCanvas`, `ItemsOnForm`, ...) | LGPL-2.1+ or Apache-2.0 | [microemu-tests](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-tests) | RMS, threads, error handling |
| Mummy Maze, Lines, Pixelus, Sliding Puzzle | GPL-3.0 | [mrlordkaj/j2me-mumy](https://github.com/mrlordkaj/j2me-mumy), [j2me-lines](https://github.com/mrlordkaj/j2me-lines), [j2me-pixelus](https://github.com/mrlordkaj/j2me-pixelus), [j2me-sliding-puzzle](https://github.com/mrlordkaj/j2me-sliding-puzzle) | full puzzle games, NetBeans projects |
| Shooter (2006) | MIT | [RehanSaeed/Shooter](https://github.com/RehanSaeed/Shooter) | a Canvas game |
| freebitva | GPL-3.0 | [Lincor/freebitva](https://github.com/Lincor/freebitva) | an RPG clone |
| J2ME Game Engine prototype | MIT | [mrzacarias/J2ME-Game-Engine](https://github.com/mrzacarias/J2ME-Game-Engine) | small engine |

These need a Java compiler. A CI job can compile them with `javac
--release 8` against a MIDP API jar (MicroEmulator's `microemu-midp`
classes serve as a compile-time API) and keep the resulting JARs as test
fixtures. FastEmu does not need preverified classes because it does not
run the CLDC verifier.

Not usable: [jsmucr/2048-for-J2ME](https://github.com/jsmucr/2048-for-J2ME)
has no license, so it cannot be redistributed even though freej2me used
it in a bug report.

## Test suites

- **SquirrelJME** (MPL-2.0, [SquirrelJME/SquirrelJME](https://github.com/SquirrelJME/SquirrelJME)):
  a Java ME runtime with its own test framework (`modules/tac`) and
  tests in `src/test/java` of its modules: 161 for `cldc-compact`, 21 for
  `meep-rms`, 7 for `midp-lcdui`, 2 for `media-api` (counted from the
  repository tree on 2026-11-24). Most target its own runtime, but the
  CLDC tests check plain library behaviour and could be adapted. The
  counts come from the GitHub tree; the test contents were not read.
  [unverified] freej2me-plus's README says its save format was "developed jointly with
  SquirrelJME".
- **J2ME Loader `GraphicsTest`** (Apache-2.0,
  [GraphicsTest.java](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/androidTest/java/javax/microedition/lcdui/GraphicsTest.java)):
  seven pixel-level checks for lines, images, `drawRGB`, `drawRegion` and
  clipping; easy to port to Rust tests for `fastemu-gfx`.
- **RustJava's test data** above doubles as a JVM test suite: each
  `.txt` holds the expected standard output of the `.class` next to it.
- **The JSR 118 and CLDC TCKs** are not publicly available. [unverified]
- **JBenchmark** (JBenchmark2 and others) was suggested for accuracy
  testing in [freej2me issue 3](https://github.com/hex007/freej2me/issues/3);
  it is closed freeware, archived only on web.archive.org, and cannot be
  redistributed. Use it manually.

## Writing FastEmu's own test MIDlets

The notes point at behaviours worth a dedicated, small MIDlet each,
written for FastEmu and licensed with it:

- a resource-lookup MIDlet: relative names from classes in and out of
  the default package, doubled slashes, backslashes, wrong case, one
  `read(byte[])` of a large resource ([note 1](01-suite-loading.md));
- a manifest corpus: BOM, wrapped `MIDlet-1`, lower-case `meta-inf`,
  `MANIFEST.FM`, missing comma, CRLF and LF ([note 1](01-suite-loading.md));
- an event-order MIDlet that logs the order of `showNotify`,
  `sizeChanged`, `paint`, `keyPressed`, `callSerially` and timer runs,
  and calls `serviceRepaints` from a game thread and from a callback
  ([note 2](02-events-threads.md));
- a `GameCanvas` key-state MIDlet that checks latching and suppression
  ([note 3](03-input.md));
- an RMS MIDlet that adds, deletes and re-adds records, closes and
  reopens, and enumerates ([note 6](06-rms.md));
- a player MIDlet that plays every format with and without a content
  type and logs listener events ([note 7](07-sound.md)).

Each can print results with `System.out`, which the headless runner reads
through `Instance::drain_log`, so the tests need no screen.
