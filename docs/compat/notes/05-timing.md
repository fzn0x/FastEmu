# 5. Timing

Frame-rate limits, `Thread.sleep`, the clocks, and the odd ways games
use `Thread.yield` and `System.gc` for timing. Link bases are as in
[note 1](01-suite-loading.md).

Phones of the J2ME era ran games on slow CPUs. Many games have no frame
limiter of their own and assume the phone is the limit. FastEmu's
interpreter on a modern host will be many times faster, so it must slow
games down on purpose, and it must do so on the game's own threads.

## Frame-rate throttling

| | Where it blocks | Default | Setting |
| --- | --- | --- | --- |
| J2ME Loader | the calling thread inside `Canvas.repaint` and `GameCanvas.flushGraphics`, until `1000 / fps` ms have passed since the previous frame | no limit (field defaults to 0) | per game, `FpsLimit` |
| freej2me-plus | the thread that presents a frame (`flushGraphics`, or the paint path), with `LockSupport.parkNanos` | 60 fps | per game, `fps`; skipped for a frame in which `Thread.yield` already waited |
| MicroEmulator | the event dispatcher delays paint events | optional `maxFps` | device setting |

Sources: [JL Canvas.limitFps](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L660),
[JL setLimitFps](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L219),
[FP MobilePlatform.limitFps](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MobilePlatform.java#L998),
[FP flushGraphics](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MobilePlatform.java#L925),
[ME EventDispatcher.run](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/org/microemu/device/ui/EventDispatcher.java#L52). [code]

Delaying only the presentation, as MicroEmulator does, does not slow a
game whose logic runs in its own thread; blocking in `repaint` and
`flushGraphics`, as the other two do, does, because the game's loop
calls them every frame.

Related tracker reports: Gradius Neo ran too fast even at 15 fps once
`System.gc` became a no-op (see below,
[issue 90](https://github.com/TASEmulators/freej2me-plus/issues/90));
Shadow Shoot and Puyo Pop Fever DX had frame counter and limiting
problems ([issue 68](https://github.com/TASEmulators/freej2me-plus/issues/68)). [tracker]

**For FastEmu:** `InstanceSettings::max_fps` is described as a bound on
presented frames. Presentation alone is not enough: apply the limit by
blocking the game thread in `repaint` (when no paint is pending),
`serviceRepaints` and `flushGraphics`, as J2ME Loader and freej2me-plus
do. A default cap (freej2me-plus uses 60) avoids games running at host
speed. Consider a per-game CPU budget (bytecodes per millisecond) for
games that compute frames without ever calling a drawing method;
neither Java emulator has one, because they run on a fast host JIT, but
an interpreter has a natural place to count. [unverified need]

## Thread.sleep

None of the emulators changes the meaning of `Thread.sleep`.
freej2me-plus wraps it so that a paused game parks inside `sleep`, and
so that its "unlock frame rate" and fast-forward options can turn every
sleep into a 1 ms sleep
([MIDletEnhancements.sleep](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletEnhancements.java#L32)). [code]

How accurate phone sleeps were is not recorded in any of the sources.
[unverified] Games commonly sleep `max(0, frameTime - elapsed)` and
expect at least that long. A host timer with 15 ms granularity (the
Windows default) would make `sleep(10)` last 15 ms and slow such games
down.

**For FastEmu:** implement `sleep` with a high-resolution timer per
instance; never wake early; let `Instance::pause` park sleeping threads
until resume and then subtract the paused time.

## System.currentTimeMillis

- J2ME Loader returns the host clock. [code]
- freej2me-plus returns a simulated clock: it starts at the host time,
  advances with real elapsed time, stops while the game is paused, and
  runs faster during fast-forward
  ([MIDletEnhancements.currentTimeMillis](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletEnhancements.java#L51)).
  A user's game (Tycoon Series: Fishing Legend) switches to night
  fishing at minutes 40 to 59 of every hour, so the clock must start at
  real wall time ([issue 97](https://github.com/TASEmulators/freej2me-plus/issues/97)). [code] [tracker]
- Some Chinese games refuse to run after a fixed date ("Authorization
  expires") read from the clock
  ([J2ME Loader issue 872](https://github.com/nikita36078/J2ME-Loader/issues/872)). [tracker]

**For FastEmu:** per instance, `currentTimeMillis` = host wall time at
launch + a per-game offset + time the instance has been running and not
paused. The offset setting lets users run date-locked games. Calendar
and `TimeZone` must agree with the same clock.

## Thread.yield and busy-waiting

- J2ME Loader rewrites every `Thread.yield()` in the game to
  `Thread.sleep(1)`; this fixed Super Action Hero
  ([AndroidMethodVisitor](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/dexlib/src/main/java/org/microemu/android/asm/AndroidMethodVisitor.java#L95),
  [commit 94d16356](https://github.com/nikita36078/J2ME-Loader/commit/94d16356)). [code]
- freej2me-plus turns `Thread.yield()` into one frame-limit wait per
  frame; its comment says this "can reduce cpu usage in some games, and
  even helps fix others like Super Action Hero"
  ([yieldOverride](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletEnhancements.java#L107)). [code]

Games also wait with loops such as
`while (System.currentTimeMillis() < next) { Thread.yield(); }` or with
no call in the loop at all. [unverified as a measured pattern; it is the
case the yield rewrite targets] With many instances running side by
side, one spinning game can starve the rest.

**For FastEmu:** make `Thread.yield()` sleep about 1 ms. In the
interpreter, a thread that calls `currentTimeMillis` many times without
any other blocking call within one millisecond can be made to yield;
measure before adding that.

## System.gc

freej2me-plus's history with `System.gc()`:

1. It made every call a no-op because some games call it between
   frames so often that they stutter (the next commit names Bomberman
   08)
   ([commit 9d7810d](https://github.com/TASEmulators/freej2me-plus/commit/9d7810d)).
2. Gradius Neo then ran too fast: it "uses System.gc ... calls to help
   with timing and synchronization", so ignoring them became a per-game
   setting ([issue 90](https://github.com/TASEmulators/freej2me-plus/issues/90),
   [commit 3367186](https://github.com/TASEmulators/freej2me-plus/commit/3367186)).
3. Ten days later the setting was removed again, "since the instances
   where it caused issues have been fixed"
   ([commit a298580](https://github.com/TASEmulators/freej2me-plus/commit/a298580)).
   Today every `System.gc()` in a game is a no-op
   ([noGC](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MIDletEnhancements.java#L104)). [code] [tracker]

J2ME Loader leaves `System.gc()` alone, so Android's collector runs. [code]

**For FastEmu:** a no-op is what freej2me-plus settled on once its frame
limiting was right. A quick, bounded collection is also fine and keeps
memory low for games that allocate a lot between levels; do not let it
take long enough to stutter.

## Fast-forward and pause

freej2me-plus's simulated clock, sleep wrapper and frame limiter work
together: fast-forward multiplies the clock (20 times by default) and
shortens sleeps to 1 ms; pause freezes the clock and parks every game
thread at its next sleep, yield, repaint or flush. [code] FastEmu can
offer the same per instance because it owns the clock and the
scheduler.
