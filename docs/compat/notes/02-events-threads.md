# 2. Events and threads

Which thread runs which callback, how repaints are merged, and what
happens around `startApp`, `setCurrent`, pausing, vibration and the
backlight. Link bases are as in [note 1](01-suite-loading.md).

## What the specification requires

From the JSR 118 `javax.microedition.lcdui` package summary [spec]:

- "All UI callbacks are serialized, so they will never occur in
  parallel." The serialized set is: `Canvas` `hideNotify`, `keyPressed`,
  `keyRepeated`, `keyReleased`, `paint`, the three pointer methods,
  `showNotify`, `sizeChanged`; `CommandListener.commandAction`; the
  `CustomItem` callbacks; `Displayable.sizeChanged`;
  `ItemCommandListener.commandAction`; `ItemStateListener.itemStateChanged`;
  and `run()` of a `Runnable` passed to `Display.callSerially`.
- "The implementation also guarantees that the call to `run()` requested
  by a call to `callSerially()` is made after any pending repaint
  requests have been satisfied."
- `serviceRepaints` is the one exception: it "causes the `Canvas.paint`
  method to be called and waits for it to complete. This occurs even if
  the caller of `serviceRepaints` is itself within an active callback."
  The caller "must not hold any locks needed by the `paint` method."
- "`Timer` events are not considered UI events. Timer callbacks may run
  concurrently with UI event callbacks, although `TimerTask` callbacks
  scheduled on the same `Timer` are serialized with each other."
- "The application manager makes `Displayable` really visible when
  `startApp()` returns."

## One event thread

| | Event thread | Callback lock | Exceptions in callbacks |
| --- | --- | --- | --- |
| J2ME Loader | `MIDletEventQueue`, one per process | `callbackLock` held around every event | caught and logged per event; `paint` catches `Throwable` |
| freej2me-plus | `EventProcessing-Thread`, started by `Display` | none; order comes from the single thread | caught and logged in `repaintRequest` and `setCurrent` |
| MicroEmulator | `EventDispatcher` thread | none | `paint` catches `Throwable`; other events are not caught in the dispatcher |
| wie | a task spawned after `startApp` returns | n/a | propagate |

Sources: [JL EventQueue](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/event/EventQueue.java#L171),
[JL CanvasEvent.process](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/event/CanvasEvent.java#L88),
[FP Display.processEvents](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Display.java#L109),
[ME EventDispatcher.run](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/org/microemu/device/ui/EventDispatcher.java#L52),
[WIE Launcher.start_midlet](https://github.com/dlunch/wie/blob/eb5b809d693ee4f0023f06acd0ad6393006578fc/wie-midp/src/classes/net/wie/launcher.rs#L51). [code]

freej2me-plus's loop order is worth copying. Each pass it takes a
snapshot of the queue sizes and then runs, in order: every input event
that was queued, the one pending paint, every `callSerially` runnable
that was queued, and the latest `setCurrent` request. Runnables queued
during the pass wait for the next pass, so a game that re-posts itself
from `run()` still lets paints happen between iterations. [code]

J2ME Loader has an "immediate mode" setting that runs events on the
posting thread and breaks serialization; its own comment calls the
results unpredictable, and it switches itself off when more than 100
runnables pile up because games overflowed the stack
([commit f78c98c8](https://github.com/nikita36078/J2ME-Loader/commit/f78c98c8)). [code]

**For FastEmu:** one event thread per instance runs every serialized
callback. Catch every exception a callback throws, print its stack
trace to the instance log as `LogLevel::Err`, and carry on with the next
event; a game's uncaught `NullPointerException` in `keyPressed` must not
end the event thread. Use freej2me-plus's snapshot loop.

## Repaint coalescing

- **J2ME Loader** keeps one `PaintEvent` per `Canvas`. `repaint(x, y, w, h)`
  grows a dirty rectangle (union, clamped to the canvas) and queues the
  event only if it is not already pending. `paint` runs with the clip
  set to the dirty rectangle, into an off-screen image that is **not
  cleared** between paints; that image is then copied for display. A
  paint request for a Canvas that is not visible is dropped
  ([Canvas.repaint](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L612),
  [PaintEvent](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L909)). [code]
- **freej2me-plus** does the same with a `needsRepaint` flag and a union
  rectangle, and ignores `repaint` while the Canvas is not shown
  ([Canvas.repaint](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Canvas.java#L218)).
  If `paint` itself called `flushGraphics` (Fast and Furious does), the
  extra flush after `paint` is skipped to avoid flicker. [code]
- **MicroEmulator** merges a new paint into the one already queued and
  can hold paints back to a maximum frame rate
  ([EventDispatcher.put](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/org/microemu/device/ui/EventDispatcher.java#L115)). [code]

The persistent back buffer matters: games call `repaint(x, y, w, h)` for
a small area and draw only that area. J2ME Loader made the Canvas back
buffer start transparent instead of white to fix flicker in Siemens
games ([commit a095290e](https://github.com/nikita36078/J2ME-Loader/commit/a095290e)). [code]

**For FastEmu:** one pending paint per Canvas with a union dirty
rectangle; paint into a persistent per-Canvas buffer with the clip set to
that rectangle and the translation at 0, 0; present the buffer after
`paint` returns. Drop paints for a Canvas that is not current.

## Input event coalescing

J2ME Loader allows at most two queued `keyRepeated` events and at most
two queued `pointerDragged` events; extra ones are dropped
([CanvasEvent.placeableAfter](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/event/CanvasEvent.java#L181)).
MicroEmulator replaces the coordinates of a queued drag with the newest
ones. Presses and releases are never merged. [code]

**For FastEmu:** never drop or merge presses and releases; merge drags
and repeats so a slow game does not fall behind.

## serviceRepaints

| | Called on the event thread | Called on another thread |
| --- | --- | --- |
| J2ME Loader | paints at once | takes the callback lock and runs the paint **on the calling thread** |
| freej2me-plus | paints at once | waits up to 1 s for the event thread, then paints on the calling thread |
| MicroEmulator | asks the device display to repaint at once | waits for the queued paint, no timeout |

Sources: [JL EventQueue.serviceRepaints](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/event/EventQueue.java#L207),
[FP Canvas.serviceRepaints](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Canvas.java#L339),
[ME Display.serviceRepaints](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/lcdui/Display.java#L628). [code]

The freej2me-plus timeout exists to "rescue any stalled frames". Its
freezes in Ratatouille ([issue 17](https://github.com/TASEmulators/freej2me-plus/issues/17)),
Garfield's House ([issue 88](https://github.com/TASEmulators/freej2me-plus/issues/88))
and Heroes Lore came from locking in the paint and input paths; they
were fixed "while dropping a ton of lock usage throughout FreeJ2ME"
([commit fb47eb4](https://github.com/TASEmulators/freej2me-plus/commit/fb47eb4f98bb28fca408cf8e611108ea159f1657),
"Fully rework input handling again"). [tracker]

**For FastEmu:** if no paint is pending, return at once. On the event
thread, paint inline. On another thread, take the callback lock and
paint on the calling thread as J2ME Loader does; if the lock is not free
within about one second, log a warning and paint anyway. Hold no
runtime lock that game code could also be waiting for while calling
`paint`.

## callSerially

J2ME Loader and freej2me-plus queue the runnable behind pending events
([JL Display.callSerially](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Display.java#L152),
[FP Display.callSerially](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Display.java#L76)).
freej2me-plus ignores a `null` runnable. [code] Some games run their
whole loop as a runnable that calls `callSerially(this)` at the end;
running it inline instead of queueing overflows the stack. [code
comment, commit f78c98c8]

**For FastEmu:** always queue, run after the pending paint, never inline.

## The lifecycle thread

J2ME Loader runs the MIDlet's constructor, `startApp`, `pauseApp` and
`destroyApp` on a separate `MidletMain` thread, not on the event thread
([MidletThread.handleMessage](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MidletThread.java#L125)). [code]
That lets a game that never returns from `startApp` (it runs its game
loop there) still receive keys and paints. wie spawns its event loop
only after `startApp` returns, so such a game gets no events at all.
[code] The JSR 118 example says `startApp` "should return immediately
to keep the dispatcher from hanging". [spec] That a warning was needed
suggests some games did not; no tracker report studied here names such
a game. [unverified]

J2ME Loader's state machine:

- Constructor fails: the launch fails.
- `startApp` throws `MIDletStateChangeException`: the MIDlet stays paused
  and nothing else happens. Any other throwable ends the app.
- `pauseApp` throws: `destroyApp(true)` is called and the app ends.
- Destroy: J2ME Loader first sends `KEY_END` press and release to the
  current Canvas (games save on the end key), then calls
  `destroyApp(true)`, and kills the process after one second if the
  MIDlet has not finished ([destroyApp](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/MidletThread.java#L100)). [code]
- `notifyDestroyed` ends the app immediately. [code]

**For FastEmu:** give each instance a lifecycle thread next to its event
thread. `Instance::stop` should call `destroyApp(true)` with a deadline
and then end every thread of the instance whatever the MIDlet does.
Whether to send `KEY_END` first is a per-game choice; it is harmless for
games that ignore it. Event delivery may start before `startApp`
returns, but the first `setCurrent` should not be shown before then if
the game sets a splash and then a menu inside `startApp`. [spec]

## Pause, background, showNotify and hideNotify

- J2ME Loader calls `pauseApp` when the Android activity pauses, unless
  the per-game "disable pause" setting is on, and calls `startApp` again
  on resume. `showNotify` and `hideNotify` follow the drawing surface
  being created and destroyed ([surfaceCreated](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L1205),
  [surfaceDestroyed](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L1225)). [code]
  Games that played music froze after returning from the background
  ([issue 678](https://github.com/nikita36078/J2ME-Loader/issues/678),
  duplicate of 388). [tracker]
- freej2me-plus has two separate actions. "Pause" freezes the game:
  `repaint`, `serviceRepaints`, `flushGraphics`, `Thread.sleep` and
  `Thread.yield` park the calling thread while paused, the event loop
  stops, and running players are stopped and later restarted.
  "Background" calls `hideNotify` and `pauseApp`, then `showNotify` and
  `startApp` ([MobilePlatform](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/org/recompile/mobile/MobilePlatform.java)). [code]

FastEmu's `Instance::pause` is documented as "Freezes the game's threads
and calls `pauseApp`". The two freej2me-plus actions show that freezing
and notifying are separate needs: freezing is for the user (no game code
runs), notifying is for the game (it can show its own pause menu). Many
games show their pause menu from `hideNotify`. [unverified as a count]

## showNotify, hideNotify and sizeChanged order

- freej2me-plus, inside `setCurrent` on the event thread: the new Canvas
  gets `showNotify`, then the old one gets `hideNotify`
  ([setCurrent](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Display.java#L304)). [code]
- J2ME Loader sends `sizeChanged` before the first show and again when
  the size really changes, and sends it on `setFullScreenMode` only while
  the Canvas is visible
  ([surfaceChanged](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L1184),
  [setFullScreenMode](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Canvas.java#L566),
  [commit a1a94948](https://github.com/nikita36078/J2ME-Loader/commit/a1a94948)). [code]
- freej2me did not call `sizeChanged` before the first show; the report
  quotes the javadoc: "This method is called at least once before the
  Displayable is shown for the first time"
  ([freej2me issue 195](https://github.com/hex007/freej2me/issues/195)). [tracker]

**For FastEmu:** on `setCurrent(next)`: if `next` is current, do nothing;
if its size differs from the last size it saw, call `sizeChanged` first;
call `hideNotify` on the old Displayable, then `showNotify` on the new
one (the natural order; freej2me-plus's reverse order is not backed by
a game), then queue a full repaint. Call `sizeChanged` immediately when
the size of the visible Canvas changes, and remember a change for later
when it happens while hidden.

## setCurrent timing

- J2ME Loader updates `current` on the calling thread and posts the view
  change to the Android UI thread; an Alert gets its return screen set
  ([Display.setCurrent](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Display.java#L107)). [code]
- freej2me-plus keeps only the **latest** request (an `AtomicReference`)
  and applies it on the event thread after inputs, the paint and the
  runnables of that pass; `setCurrent(null)` is ignored. Without the
  "repaint on setCurrent" compatibility flag it shows the new
  Displayable's existing buffer; with it, it calls `repaint`. The flag
  exists because Need for Speed: Most Wanted (2005) and Ratatouille
  expect a repaint while other games break with one. [code]

`getCurrent()` returning the new Displayable right after `setCurrent` is
the safer choice: games check it in loops. [unverified as a game
requirement]

## Timers

J2ME Loader rewrites every use of `java.util.Timer` and `TimerTask` in
the game to its own copy whose only change is: when a task throws, that
task is cancelled and the timer thread keeps running the others
([custom Timer](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/shell/custom/Timer.java#L288),
[commit caa6436e](https://github.com/nikita36078/J2ME-Loader/commit/caa6436e)).
On Java SE an exception kills the timer thread and cancels every task.
[code]

**For FastEmu:** each `Timer` has its own thread; tasks on one timer run
one at a time, concurrently with UI callbacks as the spec says. A task
that throws is logged and cancelled; the timer survives.

## vibrate and flashBacklight

| | `vibrate(ms)` | `flashBacklight(ms)` |
| --- | --- | --- |
| J2ME Loader | drives a gamepad motor if one is present, else the phone; returns `false` without a vibrator; `0` stops; negative throws `IllegalStateException` | returns `false`, does nothing |
| freej2me-plus | negative throws `IllegalArgumentException`; stores the duration for the frontend (libretro rumble); returns `true` | negative throws `IllegalArgumentException`; tints the screen for the duration on a helper thread, `Integer.MAX_VALUE` keeps it on; returns `true` |

Sources: [JL VibrationController](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/util/VibrationController.java#L46),
[JL Display](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/lcdui/Display.java#L156),
[FP Display](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/lcdui/Display.java#L183). [code]

The JSR 118 rule for negative values was not checked here; J2ME Loader's
`IllegalStateException` looks like a mistake. [unverified]

**For FastEmu:** both return `true` (the emulated phone has a motor and
a backlight) and throw `IllegalArgumentException` for negative values.
`fastemu-core` has no way to tell the app that a game asked for
vibration; an `Instance` notification for it would let the desktop app
rumble a gamepad and the Android app use the motor.
