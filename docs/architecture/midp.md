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
| `host` | `HostLink`: the latest frame, the status, the log, the link the game asks to open and the notifications, shared by the VM and the app. |
| `env` | `InstanceEnv`, what every native knows about its instance. |
| `midlet` | `javax.microedition.midlet` and the lifecycle calls. |
| `lcdui` | `javax.microedition.lcdui`: the event thread, `Display`, `Displayable`, `Canvas`, `Screen`, `Command`, `Graphics`, `Image`, `Font`, keys, soft keys and frames. |
| `lcdui::game` | `javax.microedition.lcdui.game`: `GameCanvas`, `Layer`, `Sprite`, `TiledLayer` and `LayerManager`, over the plain Rust layer model in `lcdui::game::model`. |
| `lcdui::highlevel` | The high-level screens on the extension point below: `Form` and its items, `List`, `TextBox`, `Alert`, `AlertType` and `Ticker`. |
| `rms`, `media`, `io`, `wma` | The other APIs, built alongside. |
| `nokia`, `vendor` | Nokia's UI API and the other makers' classes, only on profiles of their maker (`docs/architecture/vendors.md`). |

## Suites

`Suite::open` takes a `.jar` or a `.jad` (`docs/compat/notes/01-suite-loading.md`):

- **The JAR.** `ZipArchive` reads the central directory when it is sound
  and otherwise walks the local headers; it accepts stored and deflated
  entries with or without data descriptors, ignores CRC mismatches,
  keeps names as raw bytes, matches them exactly and then without ASCII
  case, takes the first of duplicates, and keeps what inflated of a
  deflate stream that breaks part way.
- **Archive limits.** The index holds at most 65535 entries and 4 MiB of
  names (the JAR of the test folder with most names has 60 KB). A file
  with no central directory is walked to the same limits and no further.
  Entries may overlap, since damaged real JARs do, so every read says how
  much it may inflate: `read_limited(entry, max)` stops inflating at
  `max` and gives `ZipError::TooLarge`, `read_prefix` keeps the first
  `max` bytes, and `read` is `read_limited` at 64 MiB. An entry that
  expands a thousand times costs `max`, whatever its header says.
- **Attributes.** The manifest (found without case, `MANIFEST.FM` too)
  and the JAD share one parser: a BOM is stripped, lines split on CR, LF
  or CR LF, a line starting with a space continues the previous value
  with exactly one space removed, the first of repeated keys wins, and no
  bad line rejects the file. JAD attributes come first in
  `SuiteInfo::properties`, so lookups give the JAD's value.
- **Attribute limits.** The parser reads the first 1 MiB of a file (the
  largest manifest of the test folder has 1.7 KB) and keeps 4096
  attributes. A line is a slice of the text, and only a line that others
  continue is copied, once, so a parse takes time linear in the text.
  Repeated keys are found in a set. The manifest is read with
  `read_prefix` at 1 MiB, so a manifest that inflates to more costs 1 MiB.
  The suite's name, vendor, version and MIDlet names in `SuiteInfo` are
  cut at 100 characters and the description at 500, for the tiles and
  the launcher; `SuiteInfo::properties` keep every value whole, so
  `getAppProperty` is unchanged. An icon over 1 MiB is not kept.
- **The JAR of a JAD.** `MIDlet-Jar-URL` as a relative path, then its
  last segment, then the JAD's own name with `.jar`. A JAR picks up a JAD
  of the same name beside it.
- **The JAR path stays in the folder.** A JAD is hostile input, so
  `MIDlet-Jar-URL` is only ever a name inside the JAD's folder. The value
  loses a leading `file://` and anything from the first `?` or `#`, and
  is split on `/` and `\`. Empty parts and `.` are dropped. Every other
  part must be a plain name: no `..`, no `:` (so no drive letter and no
  NTFS stream), none of `< > " | * ?`, no control character, no dot or
  space at the end, at most 255 bytes, and on Windows no device name such
  as `NUL`. A guess with a part that is not plain is dropped, which is
  not an error. A value with `://` gives only its last part. Each part is
  added to the JAD's folder one at a time, so no drive, `\\host\share`
  or `..` can replace or leave it. This is text work, done before any
  file is looked at. It gives the same answer on every system, except
  that only Windows refuses device names.
- **Size limits.** `read_capped` reads each JAR and JAD. A JAR over
  64 MiB (`MAX_JAR_BYTES`) and a JAD over 1 MiB (`MAX_JAD_BYTES`) give
  `LaunchError::InvalidSuite` with the file name, its size and the limit,
  before the file is read. So does a path that is not a regular file. The
  read stops one byte past the limit, so a file that grows cannot pass it.
  When a JAR is opened and the JAD of the same name beside it is over
  1 MiB, the JAD is left out with a note and the JAR still opens. The JAR
  stays in the one buffer it was read into.
- **MIDlets.** `MIDlet-1` up to the first gap; class after the last
  comma, name before the first, icon in between. A class name of over 255
  bytes, or with a bracket or a control character, is no class, so that
  MIDlet is dropped before it reaches the loader and a suite whose list
  comes to nothing shows the error for a suite that lists no MIDlet.
  With no `MIDlet-<n>` at all, the classes that extend `MIDlet` (found
  by walking superclasses in the JAR) are offered.
- **The search for MIDlet classes.** It reads each `.class` entry up to
  4 MiB, looks at 8192 of them at most, and stops when 32 MiB have been
  inflated in all. A read that went over its 4 MiB counts the 4 MiB. So
  a JAR whose entries all point at one compressed bomb costs about 8
  inflates, not one for each entry. It also adds up what each entry holds
  in the archive, as the entry says, whether its read gives bytes or not,
  and passes over an entry that holds more than is left of 32 MiB. A
  stream of empty deflate blocks inflates to nothing, so it is the bytes
  it holds that bound the work of entries that share it. The largest JAR
  of the test folder has 1046 classes of 4 MB in all, and its largest
  class has 200 KB. A MIDlet class past the budget is missed, which only
  a suite with no `MIDlet-1` can lose. A class file names itself, so the
  search holds the name of a class and of its superclass to the rule of a
  `MIDlet-<n>` entry: a class that fails it is no MIDlet and no link of a
  chain to one.
- **Resources.** The VM resolves a name relative to the package of the
  class it is called on; `JarSource` then turns `\` into `/`, collapses
  slashes and `.` segments, strips leading slashes, matches exactly and
  without case, and finally drops leading directories one at a time. A
  name of over 1024 bytes or 64 segments is a miss at once, before that
  walk (the deepest name in the JARs of the test folder has 8 segments).
  Each miss is logged once at `LogLevel::Warn`, with the name cut to 255
  characters. After 256 different misses the source logs once that it
  stops, and keeps no more names; what the JAR has still loads. Classes
  under `java/` and `javax/` never load from the JAR, and a class file of
  over 4 MiB does not load either: the log says so once, and the game
  gets the error for a missing class.
- **Reading a resource.** A native that hands a resource to the game
  calls `Ctx::read_resource` (`Class.getResourceAsStream`, which the
  media `Manager` uses for its locators, `Image.createImage(String)`,
  the Samsung `AudioClip(String)` and the Siemens bitmap loader). It asks
  the source for the resource with a limit, which is what `check_alloc`
  would let through at that moment, never more than the heap. The JAR
  source stops inflating there, and a bigger resource is an
  `OutOfMemoryError` before its bytes exist, as it was after. One
  resource inflates at a time for each game, so the buffers of its
  threads never add up: the host holds at most one buffer of the heap's
  size beyond what the heap counts.

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
- every class of `fastemu_midp::classes(&profile)`: the vendor APIs only
  on profiles of their maker (`docs/architecture/vendors.md`);
- the event thread started, then the control thread and the first
  lifecycle thread.

## Threads

```text
  app thread                control thread              Java threads of the VM
  (MidpInstance)            (host thread, owns Vm)
     | send(input) ------> runs each message ---------->  "event": every UI callback
     | pause/resume/stop -> in the order it came ------>  "MIDlet": constructor, startApp,
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
  (`docs/compat/notes/02-events-threads.md`). The calls still run one at
  a time, in the order the control thread started them, as on a phone's
  single lifecycle thread: a later `startApp` or `pauseApp` waits until
  the call before it has returned. Only `destroyApp` runs at once, beside
  a `startApp` that never returns; the calls still waiting then give up.
  Each waiting call is a Java thread, and a game that pauses and resumes
  itself in a loop from a `startApp` that never returns would make one for
  every request. An instance keeps at most 8 lifecycle threads alive:
  later pause and resume calls are dropped, with one warning in the log,
  but `destroyApp` never is. Lifecycle threads count against the machine's
  limit on threads too (`max_threads`, 64, and 16 more for the host). The
  bound of 8 keeps room in that limit for `destroyApp`, the event thread
  and the media thread.
- **The control thread** is a host thread that owns the `Vm` and applies
  the app's input, pause, resume and stop, and `notifyDestroyed`,
  `resumeRequest` and crashes, one at a time in the order they came.
  `Instance` methods only send it a message over a channel, which never
  blocks, so the app's thread never waits for the VM lock.

## The event loop

Each pass of `lcdui::event::run`:

1. runs the tasks posted to the VM (`Vm::post` from host threads,
   `Ctx::post` from natives). The app's input arrives this way, from the
   control thread; other modules may post work for the event thread the
   same way, for example `PlayerListener` events read from
   `Group::subscribe` on a host thread. At most `max_posted` (10000)
   tasks wait; a task posted past that is dropped, with one warning;
2. delivers the queued input events;
3. sends `keyRepeated` when the held key's time comes;
4. relays out the shown displayable if its commands, title, ticker or
   full-screen mode changed (`sizeChanged` when its size did). The
   request waits once however often the game makes it;
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
of games that re-post from `run`); `null` is ignored. At most
`max_posted` (10000) runnables wait, as many as tasks may be posted; the
call after that throws `OutOfMemoryError`, which a game can catch, and
the runnables already queued still run. Games queue a handful.

## Repaints and frames

- **Back buffer.** Every displayable has a screen-sized buffer
  (`canvas::buffer`), made on first use and kept between paints, since
  games repaint only what changed. A canvas paints at the top left of it.
  The heap counts the buffer with the displayable, 4 bytes a pixel, from
  the moment it is made: 307 KB at 240 by 320. It is counted and not
  refused, because it is made on the event thread, where an
  `OutOfMemoryError` has no game to go to; a game that keeps many shown
  canvases fills the heap and its next allocation fails. A screen that is
  no longer shown gives its buffer back and makes a new one when it is
  shown again, since it paints all of its content each time. A canvas
  keeps its buffer and its `Graphics`, which a game may hold on to.
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

- **Path.** `Instance::send` hands the event to the control thread and
  returns at once. The control thread posts it to the VM, with the input
  queued right behind it in one task, and the event thread delivers it.
  Only the control thread waits for the VM lock, which a game thread
  holds while it runs Java; the app never does. Input keeps its place
  among pause, resume and stop, and waits behind one still under way: a
  pause can take up to a second each for `hideNotify` and `pauseApp`, a
  stop up to three seconds. Input sent after a pause is dropped, even
  while the pause is still under way; input sent after a resume reaches
  the game after `showNotify`; input sent after a stop is dropped. Input
  sent before a pause reaches the game before `hideNotify`, so the keys
  the apps let go of just before they pause are released in the game.
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
  `MidpInstance::wants_text` (kept in `HostLink`) is true while an
  editable `TextBox` is shown or a `TextField` or `DateField` has the
  focus; the app should then send typed characters as text.
- **Background.** While the app has the instance paused, input is
  dropped: the control thread drops what comes after the pause. Input
  the event thread still holds when the pause comes was sent before it,
  so it delivers that first, then sends `hideNotify`.

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
keys do: the screen's own handling first, then `commandAction`. A
command with an empty label (`Alert.DISMISS_COMMAND`) shows its type's
name: OK, Back, Cancel, Exit, Help, Stop or Select.

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

The message of `Crashed` is the game's own text, an exception's message
for one. It is cut to 1 KiB and its control characters are escaped, as
the log's lines are (see [The log](#the-log)).

Exceptions in callbacks, posted tasks and game threads are logged and
the game keeps running; only the cases above end it. Dropping a
`MidpInstance` stops it.

After `destroyApp`, a stop ends in this order: the VM's stop flag, then
the wake of paused threads, then `flush_output`, then the wait for the
threads (2 s at most), then the sound group, then the status. The flag
comes before every wait for the VM lock that follows, since a game thread
stuck in a native loop around calls into Java holds the lock, and its next
call into Java ends it. The control thread waits for the lock a second at
most for a lifecycle call or posted input, and gives up on it, so a game
that holds the lock for ever can still be paused and closed: its
`pauseApp` and `destroyApp` do not run, and the status changes after about
two seconds.

`getAppProperty` reads the settings' properties, then the JAD, then the
manifest, an exact name before one that differs in case.
`checkPermission` answers network permissions from the network policy
and denies the rest.

## The log

Every line an instance logs passes `HostLink::log`, whether the VM, a
native or `JarSource` wrote it, and the app and the command line runner
print only what comes out of it. The link:

- cuts a line to the machine's `max_log_line_bytes` (4096) with the same
  ` ... (N more characters)` note the VM uses, and the launcher gives the
  link, the `JarSource` and the VM one `VmLimits`, so the cut is the same
  everywhere;
- escapes control characters, since a game chooses much of the text
  (`System.out`, an sms address, a push connection string, a missing
  name): CR and LF become the two characters `\r` and `\n`, and every
  other control character below U+0020 but the tab, U+007F and U+0080 to
  U+009F becomes U+FFFD. ESC is one, so no sequence that clears a screen,
  sets a title or writes to the clipboard reaches a terminal, and no line
  can pass for a message of FastEmu's. The bidirectional embeddings,
  overrides and isolates (U+202A to U+202E, U+2066 to U+2069) become
  U+FFFD too, so a line shows its text in the order it has;
- keeps at most 10000 lines and 1 MiB of text between two `drain_log`
  calls, dropping the oldest first. A line is at most 4096 bytes, so the
  megabyte holds at least 256 of them.

The app's own buffer of a game's log (`fastemu-ui`'s `LogBuffer`) keeps
2000 lines and 1 MiB of text, whichever is reached first.

The escaping is `fastemu_core::escape_control`, which every other place
that prints text a game or a file name chooses uses too. `fastemu run`
passes a suite's name, version, file name and its launch and crash errors
through it, and so do its complaints about the command line. The desktop
app's terminal logger passes every record through it, since records hold
a game's name and launch errors. The suite's name, vendor and version are
cut at 100 characters at the source (`suite::MAX_DISPLAY_CHARS`), but not
escaped there: the text is only escaped where it reaches a terminal.

## Links

`platformRequest` lives in `midlet` (`platform_request`). FastEmu
handles a request beside the running MIDlet, so it always returns
`false`: the MIDlet need not exit.

- **A web link**, `http` or `https` in any case with a host after `//`,
  is logged at `Info` and waits on the `HostLink` until the app takes it
  with `Instance::take_link_request`. A newer link replaces one not yet
  taken, so a game cannot pile them up, and the app is notified. The
  scheme goes to the app in lower case. A link must be printable ASCII
  with no backslash (a browser reads one as a slash, so
  `http://good.com\.evil.com/` would open `evil.com`), no user name
  before the host, a port, when it has one, of digits from 1 to 65535,
  and at most 4096 bytes, so the link the user reads is the page that
  opens. `fastemu_core::is_web_link` holds these checks, and the app
  runs the same function again on what the runtime hands over, so
  neither relies on the other. Links to `localhost`, private addresses
  and downloads are still allowed: the user reads the whole link before
  choosing **Open**.
- **An empty string** takes back a link the app has not taken yet.
- **Anything else** throws `ConnectionNotFoundException`, which MIDP
  allows for requests the phone cannot handle, and logs a `Warn` line
  naming the URL: calls (`tel:`), messages (`sms:`), files (`file:`),
  suites to install (a `.jad` or `.jar` link) and other apps' schemes.
  These never reach the host; opening them is not safe.
- **`null`** throws `NullPointerException`, as in the reference
  implementation and J2ME Loader. MIDP cancels with an empty string, not
  `null`.

MIDP requires the phone to ask the user before it acts on a request. The
app does: it shows the link in the game's tile and opens it in the
system's browser only when the user chooses **Open**
(`docs/playing.md`). A link asked for just before the game ends is still
handed over, as MIDP has the phone handle a request after its suite
exits.

The prompt ignores clicks on **Open** and **Dismiss** for 500 ms after it
appears or its link changes (`session::LINK_ARMING`), and shows the
buttons dimmed until then, so a player who taps fast in a game cannot
answer a question they have not read. The same link asked for again does
not restart the wait. The app reads the time from GPUI's clock, so tests
move it by hand.

## Many instances in one process

The apps run many games at once in one process, and two copies of one
game are two instances like any others.

Each instance has its own:

- virtual machine: heap, classes and their static fields (each instance
  parses the JAR's classes itself), threads, monitors, interned strings
  and system properties. Natives keep per-instance state in the VM's
  extensions, never in a Rust `static`;
- event thread, lifecycle threads, media event thread and game threads,
  and a control thread on the host side;
- `HostLink`: the latest frame, the status and the log;
- `InstanceEnv`: the profile, the settings and the data folder, where the
  record stores live, so two instances of one game keep separate saves;
- sound `Group`, with its own volume, pause and listener.

No Java object, static field or VM lock is shared, so a game thread never
waits on another game's lock, and a panic on a game's thread stops only
that instance. The process shares:

- the sound engine, one per app. Its mixer sits behind one mutex, which
  the audio thread holds while it renders and every player call takes
  briefly (`docs/architecture/audio.md`);
- fonts: each bundled face is parsed once, and its glyphs are rasterized
  once per pixel height into caches that every instance reads
  (`fastemu-gfx`'s `font::face`);
- read-only tables: the charsets, the time zone database and the TLS
  roots;
- the allocator and the OS: every Java thread is an OS thread.

The app's threads never wait on an instance's VM lock, which a Java
thread gives up only at its next safepoint or wait. Input, pause, resume
and stop go through the control thread's channel. `frame`, `status`,
`wants_text`, `drain_log` and `take_link_request` take only the
`HostLink`'s own lock or an atomic; the VM's threads hold that lock only
to swap in a frame, add a log line, set the status or a link, never
while they wait for anything. `set_volume` takes the mixer's lock, which
the audio thread holds while it renders a buffer and game threads hold
in player calls, but never while they wait for their VM lock.
Stopping or dropping an instance ends its threads and closes its group,
which takes its players out of the mixer; once the app lets go of the
instance, its virtual machine is freed.

`tests/many_instances.rs` runs eight instances of one MIDlet at once and
checks frames, input, a pause, record stores, sound and the stop of each.
Its soak test runs real games side by side and alone and compares their
frame rates; CONTRIBUTING.md says how to run it.

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
- `prepare(ctx, screen, width, height)` runs before each paint, before
  the screen's buffer is locked: lay the content out and call Java there
  (a `CustomItem`'s sizes and `paint`), never in `paint`.
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
- `Display.setCurrent(alert)` alone stores what was shown before it (or
  what that alert would have gone back to) the same way.
- `Displayable.setTicker` stores the `Ticker`; the frame draws the text
  its `getString()` returns where `highlevel::ticker::position` says it
  has scrolled to, and `display::relayout` runs the screen's timer at
  once.

## The high-level screens

`lcdui::highlevel` builds `Form`, `List`, `TextBox` and `Alert` on that
extension point. `draw` holds what they draw with (wrapped and cut
text, focus frames, check boxes, radio buttons, arrows, the scroll bar)
in the theme and the profile's fonts; labels are the bold medium font,
notes the small one. Each screen's timer is the `tick` hook: a ticker
presents a frame every 50 ms, an open multi-tap letter ends on time, an
alert times out, a running gauge repaints every 80 ms; anything that
changes one of these calls `highlevel::wake_timer`.

- **Choices.** `choice::ChoiceModel` keeps the elements of a `List` or
  `ChoiceGroup` with `Choice`'s rules: one selected element in the
  exclusive kinds, any in `MULTIPLE`, the selection moving on when the
  selected element is deleted, and in `IMPLICIT` lists the selection
  following the focus, so a soft key command sees the row the player is
  on. `rows` draws them, wrapped with `TEXT_WRAP_ON` and cut with an
  ellipsis otherwise. A `List` wraps its focus round at the ends, and
  left and right move it a screen of rows at a time; select
  fires `SELECT_COMMAND` (a static `Command("", SCREEN, 0)` that is
  never on the soft keys) or the command `setSelectCommand` chose. The
  elements are a native vector, so they count against the game's heap.
  `ChoiceModel::heap_bytes` is the room of the vector: 16 bytes an
  element, and up to twice that after it grew by doubling. The strings
  and images are Java objects and count as themselves. `append`,
  `insert`, `delete`, `deleteAll` and the constructors recount the
  object. A list that takes elements without end gets
  `OutOfMemoryError` at the heap limit, and deleting gives the room
  back. A row keeps at most 4096 units of its element's text, fewer in a
  choice of many elements (a budget of 1 Mi units between them), since
  elements may share one long string that each row would copy.
- **Text.** `text::editor::TextEditor` is the text, caret, maximum size
  and constraints of a `TextBox` or `TextField`, and `text` the Java
  methods both declare. Host text goes in at the caret, filtered by the
  constraints. The keypad types by multi-tap: the same key within
  900 ms changes the letter just typed (underlined until then), `#`
  cycles Abc, abc, ABC and 123, `0` is a space, `1` punctuation (with
  `@` first for `EMAILADDR` and `URL`), `*` symbols; `NUMERIC` takes
  digits and `*` or `#` for the minus, `DECIMAL` `*` for the point,
  `PHONENUMBER` `*` for `*+pw`. Clear deletes, the left and right arrows
  move the caret (`4` and `6` type), up and down move it a line in a
  text box, and a tap on a text box's text puts it there. `PASSWORD`
  shows stars but for the letter being chosen. The text is a native
  vector too, and counts against the heap at 2 bytes a unit of its room
  (`TextEditor::heap_bytes`), not of its `maxSize`: a box with a maximum
  size of `Integer.MAX_VALUE` counts what it holds. The constructors and
  every edit that makes the text longer first ask the heap whether a text
  that long fits and throw `OutOfMemoryError` if it does not; typed text
  that does not fit is dropped. An insert or a delete changes the text
  in place after checking the constraints on the text it would give, so
  it costs what it moves and not a copy of the whole text.
- **Forms.** An item is an `item::ItemData` payload: the label, layout,
  commands, preferred size and owner every item has, and its kind's
  state (`item::Kind`). `form::layout::arrange` places them in MIDP 2.0
  rows; an item without `LAYOUT_2` gets a row of its own (as J2ME Loader
  and freej2me show every item), and a `LAYOUT_2` `StringItem` whose
  text starts or ends with a line feed breaks its row there. The focus
  starts on the first item that takes it if that is on the first screen.
  Up and down go to the item an item leaves them to (a choice group
  moves between its elements first, a `CustomItem` traverses inside
  while `traverse` returns true); when the next focusable item is more
  than a step below the view, or the focused item runs off it, they
  scroll instead. The focused item's commands are added to the form's
  for the soft keys and fire to its `ItemCommandListener`; select fires
  its default command, or its only one. A change the player makes goes
  to the `ItemStateListener`; `notifyStateChanged` posts one to the
  event thread.
- **Items.** `StringItem` draws `HYPERLINK` underlined in the highlight
  colour and `BUTTON` in a rounded frame; `ImageItem` draws its image or
  its alternative text; `TextField` is the editor in a frame that grows
  with its text; `ChoiceGroup` draws radio buttons or check boxes, or a
  closed `POPUP` line whose list the form draws over the items below
  while it is open; an interactive `Gauge` moves with left and right or
  a tap; `DateField` shows `dd/mm/yyyy hh:mm` in its time zone's raw
  offset, left and right pick a part, digits type it and select sets an
  empty field to now; `Spacer` is empty space. A `CustomItem` paints in
  `prepare` into a surface of its content size, which can reach 240 by
  8192 pixels, through a `Graphics` it keeps. The surface is checked
  against the heap before it is made and counted with the item. One the
  heap cannot hold leaves the item blank, with a warning once. Its key
  callbacks get every key but the up and down arrows,
  which are traversal, and `showNotify` and `hideNotify` follow the
  form's.
- **Alerts.** An `Alert` centres its image or its type's icon, its text
  and its indicator `Gauge`, and plays its type's tone on the instance's
  sound group when shown. Like a choice element's and an `ImageItem`'s,
  its image is a snapshot when the game gives a mutable one
  (`highlevel::snapshot`). It holds `DISMISS_COMMAND` until the game adds
  a command. Its timeout (2000 ms by default) is ignored with `FOREVER`,
  two or more commands, or text that needs scrolling. When the time is
  up `DISMISS_COMMAND` fires; without a `CommandListener` any command
  dismisses the alert and shows what follows it.
- **The app.** `HostLink::set_wants_text` records whether the shown
  screen takes text, for `MidpInstance::wants_text`; a screen sets it
  when shown, when its focus moves and when its constraints change, and
  clears it when hidden.

`tests/screens.rs` runs the MIDlets in `tests/screens_java` through the
launcher and checks their frames, logs and `wants_text`.

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
- **The off-screen buffer.** A mutable `Image` at the full screen size,
  white at first, made by the constructor and kept in a private field
  (`buffer`), so the collector counts its pixels and keeps it alive.
  `getGraphics` hands `graphics::new_graphics(ctx, surface)` out at every
  call. `flushGraphics` calls `canvas::flush_shared(ctx, this,
  &surface, region)`, which waits for the frame slot off the event
  thread, checks the canvas is still shown, and only then locks the
  buffer to copy the region onto the canvas's back buffer and present a
  frame; a region of no width or height does nothing.
  `canvas::flush(ctx, canvas, &Surface, region)` does the same from a
  surface the caller owns, such as a copy, and must never get one
  borrowed from a locked image. `paint(g)` draws the buffer through
  `g`'s translation and clip, so a repaint shows what was drawn last.
- `Canvas` already owns the payload type, so `GameCanvas` declares no
  payload of its own.

## The game API

`lcdui::game` splits each class in two. `lcdui::game::model` holds the
rules as plain Rust with no virtual machine: `Sprite` (frames, the frame
sequence, the transform, the reference pixel, the collision rectangle),
`TiledLayer` (cells, static and animated tiles) and `collide` (the three
`collidesWith` tests). Its unit tests check them pixel by pixel on
`fastemu-gfx` surfaces. The Java classes keep one model in a
`LayerData` payload with the layer's image, which the payload traces.

- **Arithmetic.** It follows the MIDP 2.0 reference implementation, which
  J2ME Loader copies. Positions wrap like Java's `int`. The reference
  pixel is a pixel: under `TRANS_MIRROR` its `x` in a `w` wide frame
  becomes `w - 1 - x`, as `Transform::apply` maps pixels. `setTransform`
  replaces the transform and moves the sprite so the reference pixel
  stays on the same screen pixel; `setImage` with a new frame size does
  the same, and resets the collision rectangle to the frame.
- **Frames.** The default frame sequence is never built as a list, so a
  sprite of thousands of tiny frames costs nothing. `setFrameSequence`
  copies the array. `setImage` keeps the current frame and a custom
  sequence when the new image has at least as many frames, and resets
  both otherwise.
- **Drawing.** `Sprite.paint` draws its frame with `fastemu-gfx`'s
  `draw_region` at the sprite's position, anchored top left; a sprite
  whose image is the destination throws `drawRegion`'s
  `IllegalArgumentException`. `TiledLayer.paint` draws only the cells the
  clip touches, an animated tile through its static tile; cells holding
  0 draw nothing. Invisible layers draw nothing.
- **Collisions.** Collision rectangles collide when their insides
  overlap, the reference implementation's test, so an empty rectangle
  inside another still collides by rectangle. A pixel-level test looks
  only at pixels inside both rectangles and both frames, after
  transforms, and counts a pixel when its alpha is `0xFF`. Against a
  tiled layer, a cell that is not 0 collides by rectangle even when it is
  an animated tile showing nothing; pixel by pixel only the opaque pixels
  of the tiles the cells show count. Two sprites sharing an image lock it
  once.
- **Tiled layers.** `setStaticTileSet` keeps the cells and animated
  tiles when the new set has at least as many tiles, and otherwise
  empties every cell and deletes the animated tiles. A grid of more than
  2^24 cells is the `OutOfMemoryError` a phone would throw, and one wider
  or taller than an `int` is `IllegalArgumentException`. The grid is 4
  bytes a cell. The constructor asks the heap for it before it makes the
  grid and counts it after, so a layer larger than the heap, or one more
  than the heap holds, is an `OutOfMemoryError` and a game that keeps
  layers in a list ends in one. A layer holds at most 32767 animated
  tiles, as the Siemens layer does; `createAnimatedTile` past that throws
  `OutOfMemoryError`. Each animated tile is counted when it is made, as it
  makes no Java object. A `Sprite` counts its frame sequence the same way,
  and a snapshot of a mutable image that a sprite, a screen or an item
  keeps is checked against the heap before the copy is made.
- **LayerManager.** It keeps its layers in a traced payload, index 0 on
  top. `append` and `insert` move a layer already present; `insert`
  checks its index against the other layers' count, as J2ME Loader does.
  `paint(g, x, y)` saves `g`'s clip, translates by `(x - viewX, y -
  viewY)`, clips to the view window, paints the last layer first, then
  translates back and restores the clip.
- **Copies.** `Sprite(Sprite)` copies every property; a mutable image is
  copied as it is at that moment, as `Image.createImage(Image)` would.
- **Wrong arguments.** An argument of the wrong class, which only code
  that skipped the verifier can pass, throws `ClassCastException` instead
  of stopping the virtual machine.

`tests/game_api.rs` runs the MIDlets `tests/java/midlets/t/Game*.java`
through the launcher: key states, suppression and flush pacing on a
`GameCanvas`, every rule above read back from images the layers paint
on, and a `LayerManager` scene checked frame by frame.

## Adding a class

1. Write a `NativeClass` in the module of its area and add it to that
   module's `classes()`. Take every signature and constant from
   `docs/compat/api/`, and give games every constructor they may call.
2. Keep per-object state in a payload that traces its references, or in
   Java fields; keep per-instance state in an extension that holds
   `GlobalRef`s (wrapped in `Arc` when several places need them), never
   `ObjRef`s. Root references read from payloads or arrays before calling
   Java (`ctx.root`).
3. Never hold the lock of an `Image`'s surface across a call into Java
   or anything else that can collect (rule 3 of
   `docs/architecture/jvm.md` lists them). The lock is not reentrant, and
   the collector locks every image's surface to count its bytes
   (`ImageData::heap_bytes`), so the thread would wait on itself. For the
   same reason a `ScreenHandler`'s `paint` must not lock its screen's
   buffer again: `target.surface` is that buffer, already locked.
4. Run UI callbacks through `lcdui::event::callback`, never directly from
   another thread. Read the instance through `env::env(ctx)`.
5. Throw what the spec names; `lcdui::throw_gfx` turns a `GfxError` into
   its exception. Log with `ctx.log`, and use
   `lcdui::state::warn_once` for warnings a game could trigger every frame.
6. Test it beside the code, and through the launcher with a small MIDlet:
   `tests/support` compiles the Java under `tests/java` with stubs of the
   MIDP classes (signatures and constant values from the inventory; javac
   inlines constants), packs JARs and drives an instance.

## Not done yet

- In the high-level screens: horizontal traversal inside a `CustomItem`
  (only `TRAVERSE_VERTICAL` is offered); daylight saving time in
  `DateField` (only the zone's raw offset); predictive text; the
  full-screen editor some phones open for a `TextField`.
- Telling the app about `vibrate` and `flashBacklight` (they log and
  return `true`): `fastemu-core` has no notification for them.
- Sending `KEY_END` before `destroyApp`, a per-game choice in J2ME Loader.
- Whether a non-full-screen canvas loses height to the soft key bar is
  fixed (it does when it has commands); notes 03 asks for a profile
  setting and a per-game override.
- Holding the first `setCurrent` until `startApp` returns, which the spec
  asks for and games that loop in `startApp` cannot have.
