# The MIDP runtime

The `fastemu-midp` crate turns a JAR into a running, visible, playable
instance: it opens suites, starts each MIDlet in its own virtual machine
(`docs/architecture/jvm.md`), runs the MIDlet's lifecycle, serializes its
UI callbacks on an event thread, draws with `fastemu-gfx`
(`docs/architecture/graphics.md`) and hands frames to the app. This page
is for the people who build the rest of MIDP on it: the high-level
screens, `GameCanvas` and its layers, and the vendor APIs. The behaviour
follows `docs/compat/notes/`; where a rule comes from there, the note is
named.

## Modules

| Module | What it holds |
| --- | --- |
| `suite` | The lenient ZIP reader, the manifest and JAD parser, `Suite::open`, and `JarSource`, the VM's `ClassSource` over the JAR. |
| `launcher` | `MidpLauncher`, the real `fastemu_core::Launcher`, and the system properties of an instance. |
| `instance` | `MidpInstance`, the `fastemu_core::Instance` the app holds, and its control thread. |
| `host` | `HostLink`: the latest frame, the status, the log and the notifications, shared by the VM and the app. |
| `env` | `InstanceEnv`, what every native knows about its instance. |
| `midlet` | `javax.microedition.midlet` and the lifecycle calls. |
| `lcdui` | `javax.microedition.lcdui`: the event thread, `Display`, `Displayable`, `Canvas`, `Screen`, `Command`, `Graphics`, `Image`, `Font`, keys, soft keys and frames. |
| `lcdui::game`, `lcdui::highlevel` | Empty: `GameCanvas` and the layers, and `Form`, `List`, `TextBox`, `Alert`, `Ticker` and the items, come next. |
| `rms`, `media`, `io`, `wma`, `nokia`, `vendor` | The other APIs, built alongside. |

## Suites

`Suite::open` takes a `.jar` or a `.jad` (`docs/compat/notes/01-suite-loading.md`):

- **The JAR.** `ZipArchive` reads the central directory when it is sound
  and otherwise walks the local headers; it accepts stored and deflated
  entries with or without data descriptors, ignores CRC mismatches,
  keeps names as raw bytes, matches them exactly and then without ASCII
  case, takes the first of duplicates, and keeps what inflated of a
  deflate stream that breaks part way.
- **Attributes.** The manifest (found without case, `MANIFEST.FM` too)
  and the JAD share one parser: a BOM is stripped, lines split on CR, LF
  or CR LF, a line starting with a space continues the previous value
  with exactly one space removed, the first of repeated keys wins, and no
  bad line rejects the file. JAD attributes come first in
  `SuiteInfo::properties`, so lookups give the JAD's value.
- **The JAR of a JAD.** `MIDlet-Jar-URL` as a relative path, then its
  last segment, then the JAD's own name with `.jar`. A JAR picks up a JAD
  of the same name beside it.
- **MIDlets.** `MIDlet-1` up to the first gap; class after the last
  comma, name before the first, icon in between. With no `MIDlet-<n>` at
  all, the classes that extend `MIDlet` (found by walking superclasses in
  the JAR) are offered.
- **Resources.** The VM resolves a name relative to the package of the
  class it is called on; `JarSource` then turns `\` into `/`, collapses
  slashes and `.` segments, strips leading slashes, matches exactly and
  without case, and finally drops leading directories one at a time. Each
  miss is logged once at `LogLevel::Warn`. Classes under `java/` and
  `javax/` never load from the JAR.

Natives that read resources by name use `env(ctx).source` (the same
`JarSource`) inside `Ctx::without_lock`. `Image.createImage(String)`
treats every name as absolute.

## Launching

`MidpLauncher::new(audio)` takes an optional `fastemu_audio::Engine`: the
apps pass one with output, the headless runner and tests pass none.
`launch` makes one VM per instance:

- `heap_bytes` from the profile, `allow_exit: false`, the log into the
  instance's `HostLink`;
- the system properties of `launcher::system_properties`: the platform,
  configuration and profiles from the profile, encoding and locale from
  the settings, `supports.mixing` and the capture keys, a fake IMEI under
  the profile vendor's keys, and the settings' own properties last.
  Nothing comes from the host (`docs/compat/notes/08-system-properties.md`).
  A module that implements an optional API adds its `*.version` key there;
- `InstanceEnv` installed as an extension, with a sound `Group` from the
  engine at the settings' volume;
- every class of `fastemu_midp::classes()`;
- the event thread started, then the control thread and the first
  lifecycle thread.

## Threads

```text
  app thread                control thread              Java threads of the VM
  (MidpInstance)            (host thread, owns Vm)
     | send(input) ------------------------------------>  "event": every UI callback
     | pause/resume/stop -> runs each change in order ->  "MIDlet": constructor, startApp,
     |                      waits up to 1 s per call        pauseApp, destroyApp (one thread
     | frame/status/log <-- HostLink <------------------    per call)
                                                          game threads, Timer threads
```

- **The event thread** runs every serialized callback: `paint`, the key
  and pointer callbacks, `showNotify`, `hideNotify`, `sizeChanged`,
  `commandAction`, `callSerially` runnables, and the screen hooks.
- **Lifecycle threads** run the constructor and `startApp` on one thread,
  and each later `startApp`, `pauseApp` or `destroyApp` on a fresh one, so
  a game that loops inside `startApp` still gets paints and keys
  (`docs/compat/notes/02-events-threads.md`).
- **The control thread** is a host thread that owns the `Vm` and applies
  pause, resume, stop, `notifyDestroyed`, `resumeRequest` and crashes one
  at a time. `Instance` methods only send it a message.

## The event loop

Each pass of `lcdui::event::run`:

1. runs the tasks posted to the VM (`Vm::post` from host threads,
   `Ctx::post` from natives). The app's input arrives this way; other
   modules may post work for the event thread the same way, for example
   `PlayerListener` events read from `Group::subscribe` on a host thread;
2. delivers the queued input events;
3. sends `keyRepeated` when the held key's time comes;
4. relays out the shown displayable if its commands, title, ticker or
   full-screen mode changed (`sizeChanged` when its size did);
5. applies the latest `setCurrent`: `sizeChanged` if the size differs
   from the last one the displayable heard, `hideNotify` on the old one,
   `showNotify` on the new one, a full repaint;
6. gives a shown high-level screen its timer tick;
7. paints the pending region when the frame rate allows;
8. runs the `callSerially` runnables queued before this pass, stopping
   when one asks for a repaint so the paint comes first.

Then it sleeps until something is posted or queued, or a deadline passes
(a key repeat, a deferred paint, a screen timer).

Every callback runs inside `lcdui::event::callback(ctx, what, f)`, which
holds the **callback lock** and logs an exception `f` throws as
`Exception in <what>: ...` with its stack trace, at `LogLevel::Err`, and
counts it in `MidpInstance::error_count`. The event thread never dies of
a game's exception.

`setCurrent` only records the latest request; `getCurrent` returns it at
once. `callSerially` always queues (running it inline overflows the stack
of games that re-post from `run`); `null` is ignored.

## Repaints and frames

- **Back buffer.** Every displayable has a screen-sized buffer
  (`canvas::buffer`), made on first use and kept between paints, since
  games repaint only what changed. A canvas paints at the top left of it.
- **Repaint.** `repaint(x, y, w, h)` on a shown canvas grows one dirty
  rectangle (clamped to the canvas); requests for a canvas that is not
  shown are dropped. The event thread resets the canvas's `Graphics`
  (one per canvas, made once), clips it to the dirty rectangle and calls
  `paint`.
- **serviceRepaints.** Returns at once when nothing is pending. On the
  event thread it paints inline. On another thread it takes the callback
  lock and paints on the calling thread, as J2ME Loader does; when the
  lock is not free within a second it logs a warning once and paints
  anyway. A paint the event thread has already started counts as
  pending there: the call waits for it to finish.
- **Presenting.** After each paint (and flush, and soft key or menu
  change) `frame::present` composes the frame: the displayable's pixels,
  the title bar and ticker of a screen, the soft key bar, the options
  menu, and hands a `Frame` to `HostLink::present`. A full-screen canvas
  without a menu is presented straight from its buffer. Frame serials
  count presented frames from 1.
- **Frame rate.** `InstanceSettings::max_fps`, 60 when unset
  (`lcdui::state::DEFAULT_MAX_FPS`), `Some(0)` for no bound. Game threads
  wait in `repaint` (when no paint is pending) and in flushes until one
  frame period has passed since the previous such call, as J2ME Loader
  paces them; spacing the calls rather than waiting on the last frame
  keeps a loop that sleeps a little each frame at the full rate. The event
  thread defers its paint instead (`docs/compat/notes/05-timing.md`).
- **Notifications.** `HostLink` calls the app's `Notify` on a new frame,
  status or log line, once until the app reads the frame, the status or
  the log again.

## Input

The app sends one press and one release per stroke; the runtime makes
the repeats (`docs/compat/notes/03-input.md`).

- **Codes.** `InputEvent::Key` maps through the profile's `KeyCodes`;
  `RawKey` passes its code through.
- **Game actions** (`lcdui::keys::game_action`): the profile's arrows and
  fire key, and `2`, `4`, `6`, `8`, `5`, give `UP`, `LEFT`, `RIGHT`,
  `DOWN`, `FIRE`; `1`, `3`, `7`, `9` give `GAME_A` to `GAME_D`; everything
  else 0. Never throws. `getKeyCode` inverts it and throws
  `IllegalArgumentException` only for a value that is no action.
  `getKeyName` names every code: `UP`, `SELECT`, `SOFT1`, the character
  for printable codes, `KEY <code>` otherwise.
- **Repeats.** The last key pressed repeats after 400 ms, then every
  120 ms, to `keyRepeated` only, until released. A key the soft keys took
  never repeats. A slow game gets fewer repeats, not a backlog.
- **Pointer.** Events land on the canvas only when the profile has
  `touch`, only when the press was inside the canvas, in canvas
  coordinates; a release follows every delivered press; drags that do not
  move are dropped and queued drags merge. A press on the soft key bar
  acts as the soft key under it, whatever the profile.
- **Text** (`InputEvent::Text`) goes to a high-level screen's handler.
- **Background.** While the app has the instance paused, input is
  dropped.

## Soft keys and commands

`lcdui::softkeys` follows J2ME Loader's rules: commands sort by type and
priority with the first `BACK` or `EXIT` moved to the front; one command
is labelled on the left and fires from both soft keys; two put the front
command on the right; more put the front command on the right and an
options menu on the left. In full-screen mode no labels are drawn, the
left soft key opens the menu, and with one command the right soft key
goes to the game. A canvas without a `CommandListener` passes its soft
keys to the game. A soft key press the runtime takes is not passed to the
game, and neither is its release.

A non-full-screen canvas with commands loses the bar's height (the
default font's height plus 4); a screen always has the bar. While the
menu is open every key goes to it: up and down move, fire, 5 or the left
soft key choose, the right soft key, clear or end close it.
`command::fire(ctx, displayable, command)` fires a command as the soft
keys do: the screen's own handling first, then `commandAction`.

## Lifecycle and status

| Event | What happens | Status |
| --- | --- | --- |
| launch | constructor on the `MIDlet` thread | `Starting` |
| constructor returns | `startApp` | `Running` |
| `startApp` throws `MIDletStateChangeException` | logged; the MIDlet waits for a resume | `Paused` |
| class missing, constructor or `startApp` throws, VM internal error | logged; `destroyApp(true)` if there is a MIDlet; every thread stops | `Crashed { message }` |
| `Instance::pause` | `hideNotify`, `pauseApp` (each up to 1 s), then the VM and sound group freeze | `Paused` |
| `Instance::resume` or `resumeRequest` | VM and sound thaw, `showNotify` and a full repaint, `startApp` | `Running` |
| `notifyPaused` | logged | `Paused` |
| `Instance::stop` | `destroyApp(true)` (up to 1 s), then every thread stops and the sound group closes | `Exited` |
| `notifyDestroyed` | every thread stops, no `destroyApp` | `Exited` |

Exceptions in callbacks, posted tasks and game threads are logged and
the game keeps running; only the cases above end it. Dropping a
`MidpInstance` stops it.

`getAppProperty` reads the settings' properties, then the JAD, then the
manifest, an exact name before one that differs in case.
`platformRequest` logs the URL and returns `false`. `checkPermission`
answers network permissions from the network policy and denies the rest.

## The extension point for high-level screens

`Form`, `List`, `TextBox` and `Alert` are classes implemented in Rust that
extend `Screen`. A screen class gives its objects a payload made with
`DisplayableData::for_screen(&HANDLER, State::default())`:

```rust
static FORM: FormHandler = FormHandler;

NativeClass::new("javax/microedition/lcdui/Form")
    .extends("javax/microedition/lcdui/Screen")
    .payload_with(|| Box::new(DisplayableData::for_screen(&FORM, FormState::default())))
```

- `ScreenHandler` (in `lcdui::screen`) is a `static` with the behaviour;
  `ScreenState` is the per-object state, kept in the payload and traced by
  it, so it may hold Java references (items, images, strings).
- `paint(ctx, screen, target)` draws the content area: `target.surface`
  is cleared to the theme's background, `target.width` and `height` give
  the area, `target.fonts` and `target.theme` match what the runtime
  draws. Titles, the ticker, the soft key bar and menus are the
  runtime's.
- `key`, `pointer` (content coordinates) and `text` get what the soft
  keys and menu leave; `shown` and `hidden` stand for `showNotify` and
  `hideNotify`; `command` sees each fired command first and returns
  whether it handled it; `tick` is called while the screen is shown and
  returns when it wants the next call (an `Alert`'s timeout, a ticker).
- Every hook runs on the event thread, inside the callback lock. A hook
  reaches its state with
  `displayable::data(ctx, screen)?.state::<FormState>()` (or `data_mut`
  and `state_mut`), asks for a repaint with `display::repaint(ctx,
  screen)`, shows another displayable with `display::set_current`, and
  fires commands with `command::fire`.
- A screen may add commands of its own with
  `DisplayableData::add_command` (an `Alert`'s `DISMISS_COMMAND`, a
  `List`'s `SELECT_COMMAND`).
- `Display.setCurrent(alert, next)` stores `next` in the alert's payload
  (`DisplayableData::after_alert`) and shows the alert; the `Alert`
  handler shows `after_alert` when it is dismissed.
- `Displayable.setTicker` stores the `Ticker`; the frame draws the text
  its `getString()` returns.

## What GameCanvas uses

- **Key states.** `lcdui::keys::take_key_states(ctx)` returns the
  `*_PRESSED` bits of the keys held now or pressed since the last call,
  and clears the latched ones. `getKeyStates` returns 0 itself when
  `display::is_shown(ctx, this)` is false. Showing a canvas resets both
  masks, and a key held while it appeared counts only after it is pressed
  again.
- **Suppressed key events.** `GameCanvas(true)` calls
  `DisplayableData::set_suppress_game_keys(true)`: keys with a game action
  then get no key callbacks; other keys still do.
- **The off-screen buffer.** Make a mutable `Surface` at the full screen
  size, share it as `image::SharedSurface`, and hand
  `graphics::new_graphics(ctx, buffer)` to `getGraphics`.
  `flushGraphics` calls `canvas::flush(ctx, this, &buffer, region)`, which
  copies the region onto the canvas's back buffer, waits for the frame
  slot off the event thread and presents a frame. `paint(g)` draws the
  buffer with the ordinary `Graphics` calls.
- `Canvas` already owns the payload type, so `GameCanvas` declares no
  payload of its own.

## Adding a class

1. Write a `NativeClass` in the module of its area and add it to that
   module's `classes()`. Take every signature and constant from
   `docs/compat/api/`, and give games every constructor they may call.
2. Keep per-object state in a payload that traces its references, or in
   Java fields; keep per-instance state in an extension that holds
   `GlobalRef`s (wrapped in `Arc` when several places need them), never
   `ObjRef`s. Root references read from payloads or arrays before calling
   Java (`ctx.root`).
3. Run UI callbacks through `lcdui::event::callback`, never directly from
   another thread. Read the instance through `env::env(ctx)`.
4. Throw what the spec names; `lcdui::throw_gfx` turns a `GfxError` into
   its exception. Log with `ctx.log`, and use
   `lcdui::state::warn_once` for warnings a game could trigger every frame.
5. Test it beside the code, and through the launcher with a small MIDlet:
   `tests/support` compiles the Java under `tests/java` with stubs of the
   MIDP classes (signatures and constant values from the inventory; javac
   inlines constants), packs JARs and drives an instance.

## Not done yet

- The high-level screens, `Ticker`, `GameCanvas` and the layers, and
  `Display.setCurrentItem` (the next wave).
- Telling the app about `vibrate` and `flashBacklight` (they log and
  return `true`): `fastemu-core` has no notification for them.
- Sending `KEY_END` before `destroyApp`, a per-game choice in J2ME Loader.
- Whether a non-full-screen canvas loses height to the soft key bar is
  fixed (it does when it has commands); notes 03 asks for a profile
  setting and a per-game override.
- Holding the first `setCurrent` until `startApp` returns, which the spec
  asks for and games that loop in `startApp` cannot have.
