# Media

The `media` module of `fastemu-midp` is MMAPI (JSR-135,
`javax.microedition.media` and its `control` and `protocol` packages)
for FastEmu games, and `nokia::sound` is Nokia's `com.nokia.mid.sound`.
Both are Java classes implemented in Rust (see [jvm.md](jvm.md)) on top
of the sound engine in `fastemu-audio` (see [audio.md](audio.md)).

This page describes how a Java `Player` maps onto the engine, which
thread calls the game's listeners, how players stay alive while they
play, and what an instance without an audio engine gets.

## Shape

```text
  game code (any Java thread)                 media-events (one Java thread
     |  Manager.createPlayer, Player.start,      per instance, Rust body)
     |  VolumeControl.setLevel, new Sound(...)       ^  reads the channel in order,
     v                                               |  calls playerUpdate and
  PlayerImpl / Sound object                          |  soundStateChanged
     payload: PlayerData / SoundData                 |
        Track ----------------------------------+    |
          Backend::Engine(fastemu_audio::Player) |    |
          Backend::Silent(Silent clock)          |    |
                                                 v    |
  MediaState (VM extension)              mpsc channel of Notes
     entries: serial -> WeakReference, hold    ^       ^
     links: engine PlayerId -> serial          |       |
                                   silent events     group listener (audio
                                   and stop times    thread or the caller)
```

- **Track** (`media/track.rs`): the sound of one Java object. It holds a
  backend, the volume level and mute, the content type and format, and
  the stop time. Every operation works the same on both backends.
- **Backend::Engine**: a player of the instance's sound group
  (`InstanceEnv.audio`). It plays, and the engine sends its events.
- **Backend::Silent** (`media/silent.rs`): a player that plays nothing
  but follows the engine's state machine exactly (states, loops, media
  time, the end of media) with the wall clock standing in for the audio
  thread. It makes its own events.
- **MediaState** (`media/events.rs`): the registry of every player and
  sound of the instance, kept as a VM extension. It holds no `ObjRef`s,
  only `GlobalRef`s.
- **media-events**: the Java thread that delivers events. It is started
  the first time the instance makes a player or a Nokia sound, from the
  host's threads past `max_threads`, so a game with as many threads as it
  may have still gets it.

## Modules

| File | What it holds |
| --- | --- |
| `media/api.rs` | The interfaces, constants and `MediaException`: `Control`, `Controllable`, `Player`, `PlayerListener`, `TimeBase` and every interface of the `control` package. |
| `media/manager.rs` | `Manager`: `createPlayer` from a stream, a locator and a `DataSource`, `playTone`, `getSupportedContentTypes`, `getSupportedProtocols`, `getSystemTimeBase`. |
| `media/player.rs` | `PlayerImpl`, the class of every player: the state machine, realizing, listeners, controls. |
| `media/controls.rs` | `VolumeControl`, `ToneControl`, `MIDIControl` and `StopTimeControl` implementations. |
| `media/track.rs`, `media/silent.rs` | The track and the silent clock. |
| `media/events.rs`, `media/event.rs` | The registry, the event thread, the event and note types. |
| `media/stream.rs` | Reading an `InputStream` or `SourceStream` to its end. |
| `media/protocol.rs` | `DataSource`, `SourceStream`, `ContentDescriptor`. |
| `media/time_base.rs` | The system `TimeBase`. |
| `nokia/sound.rs` | `Sound` and `SoundListener`. |

## Players

`Manager.createPlayer(InputStream, type)` reads the whole stream at once:
a plain `java.io.ByteArrayInputStream` (what `getResourceAsStream`
returns) is copied from its buffer, and any other stream is read through
its own `read(byte[], int, int)`, so a game's stream classes behave as on
a phone. An `IOException` from the stream passes through. The player
starts UNREALIZED and keeps the bytes until `realize`.

The bytes count on the Java heap while the player keeps them
(`PlayerData::heap_bytes`, counted when the player is made, recounted
when `realize` or `close` drops them), as they did in the game's own
stream. A stream read through its `read` grows in an object the heap
counts at the room of its vector (`StreamBytes`), and the heap is asked
before it grows (`Ctx::check_alloc`): a stream that never ends, or that
is longer than the heap could hold, ends in `OutOfMemoryError` at the
size of the heap, and never reads more than 32 MiB. The game's `read`
runs between the reads, and so do its other threads, so streams read at
once share the heap's room. Once read, the bytes go to the player before
any other thread runs. A sound of a format nothing plays keeps no bytes
at all.

`realize` decodes with the VM lock released (`Ctx::without_lock`), so
other Java threads of the game keep running while a long MP3 or MIDI
file is parsed; `prefetch` and `start` realize first when needed. After
that the bytes are dropped: the engine keeps the decoded sound. If the
player is closed, or realized by another thread, while it decodes, the
new sound is dropped.

The decoded sound is not on the Java heap: a phone kept it apart, and a
few megabytes of it would fill a game's 2 MiB heap. The instance's sound
group bounds it instead: 128 players and 32 MiB of sound for the game,
and 16 MiB for one sound ([audio.md](audio.md)). `realize` throws what
the engine refuses for size: `MediaException` for a sound that decodes to
more than 16 MiB (or an AMR file of more than ten minutes), and
`OutOfMemoryError` when the game holds all the sound or all the players
it may. The player stays UNREALIZED with its bytes, so it can realize
once the game has closed others. Before `OutOfMemoryError` the game gets
one collection and a second try (`make_sound`): games make a player for
each sound and drop it unclosed, and a phone's collector freed those.

[SECURITY.md](../../SECURITY.md#limits-a-game-cannot-pass) lists these
limits with every other limit a game meets.

| Call | What happens |
| --- | --- |
| `getState` | UNREALIZED (100) before `realize`, then the track's state: REALIZED 200, PREFETCHED 300, STARTED 400; CLOSED 0 after `close` or when the instance's group closed. Never throws. |
| `realize`, `prefetch` | As above; nothing on a player already there. `MediaException` for a sound that is too large, `OutOfMemoryError` when the game's sound group is full. |
| `start` | Prefetches, then starts. The state is STARTED when `start` returns, however short the sound. After the end it plays again from the start. Nothing on a started player. `MediaException` when the game has 32 players started already, or all games together 128: the player stays PREFETCHED, and a stop or an end makes room. In the apps, a started player of a game whose sound is over its render budget can wait, silent, with its media time held ([audio.md](audio.md#render-budget)). |
| `stop` | Stops and keeps the media time; nothing on a player that is not started. |
| `deallocate` | Back to REALIZED, stopping first; nothing while UNREALIZED. |
| `close` | CLOSED for good; the engine player and the bytes are freed. Twice is harmless. |
| `setLoopCount(n)` | 0 throws `IllegalArgumentException`; a started or closed player throws `IllegalStateException`. Any negative count loops forever. Kept until `realize` on an unrealized player. |
| `getMediaTime` | Microseconds; 0 while UNREALIZED. |
| `setMediaTime(t)` | Clamped to the media; returns the time set. UNREALIZED throws. |
| `getDuration` | Microseconds; `TIME_UNKNOWN` (-1) while UNREALIZED. |
| `getContentType` | The format found (which may differ from the type the game gave); UNREALIZED throws. |
| `add/removePlayerListener` | `null` is ignored; a listener added twice is called once. |
| `getControl(name)`, `getControls` | See Controls. `null` throws `IllegalArgumentException`; UNREALIZED and CLOSED throw `IllegalStateException`. |
| `setTimeBase`, `getTimeBase` | Only the system time base is accepted; another throws `MediaException`. |

Calls on a closed player throw `IllegalStateException`, except
`getState` and `close`. Engine errors map as [audio.md](audio.md)
says: `IllegalArgumentException` for bad arguments and tone sequences,
`IllegalStateException` for the wrong state, `MediaException` for media.

### Formats and silence

The format comes from the bytes first (`Format::sniff`) and from the
content type second, because games pass wrong types and `null`. Sound
FastEmu cannot play never throws (rule 14 of
[emulator-notes.md](../compat/emulator-notes.md)):

- Data whose format is unknown becomes a silent player that lasts one
  second: it goes through every state, sends `END_OF_MEDIA` at the end
  (so a game waiting for a jingle carries on) and reports the content
  type the game gave, or `application/octet-stream`.
- Data that claims a format but does not decode (a WAV header over
  junk) becomes the same one-second silent player.
- Each is logged once at `LogLevel::Warn` with the declared type and the
  first 16 bytes, so the format can be added later. An instance logs 16
  such lines, then says it stops.

A sound that is too large, or a game that holds all the sound it may, is
not silence: it throws (see Players), so a game cannot take the whole of
the computer's memory by making sounds, and one that cannot hold a sound
says so.

### Locators

| Locator | Player |
| --- | --- |
| `device://tone` | A tone player with an empty sequence; give it notes with `ToneControl`. |
| `device://midi` | A silent MIDI player of length 0, logged: its `MIDIControl` takes every event and plays nothing. |
| Any other `device://`, `capture://` | `MediaException`: FastEmu has no such device. |
| `resource:...`, `file:...`, `/path`, `path` | The suite resource at that path, read through `Class.getResourceAsStream` so the suite's lookup rules apply; the extension suggests a content type. A missing resource throws `IOException`. |
| Anything else (`http://`, `rtsp://`) | A silent player, logged: FastEmu does not fetch from the network. |

`getSupportedContentTypes` reports the engine's
`SUPPORTED_CONTENT_TYPES` for `null`, `file` and `resource`, tone
sequences for `device`, and nothing for other protocols.
`getSupportedProtocols` reports `device`, `file` and `resource`, or
those that give the type asked about.

`Manager.playTone` checks the note (0 to 127) and the duration (above
0), clamps the volume, lengthens tones shorter than 50 ms to 50 ms (as
freej2me-plus does for games that beep for 10 ms) and plays through
`Group::play_tone`; without a group it does nothing. It never blocks.

`Manager.createPlayer(DataSource)` connects and starts the game's
source, reads its first stream through the game's `read` method and
plays the bytes like a stream's. The source stays connected.

## Listeners and the event thread

Events come from two places and meet in one channel, so they keep the
order they happened in:

- the instance's sound group. The media module installs the group's
  listener the first time it is used; the listener only sends the event
  into the channel, on whichever thread caused it (the audio thread for
  the end of media and loops, the caller for `start`, `stop`, `close`
  and volume changes);
- the MIDP layer itself: silent players' events, the `CLOSED` of a
  player that was never realized, and `STOPPED_AT_TIME`.

The `media-events` thread waits for the channel with the VM lock
released (`Ctx::wait_without_lock`, so `Vm::activity` counts it as
waiting, not running), then calls each listener in order. It never
runs on the audio thread and never calls Java with the mixer locked.
While the VM is paused it parks before delivering. Each note runs in
its own local frame; an exception a listener throws is logged with its
stack trace and delivery goes on (as rule 8 of the emulator notes asks
for UI callbacks), and so is any other Java exception on the thread.
Only a stopping VM ends it.

Event data follows JSR-135 and J2ME Loader: `STARTED`, `STOPPED`,
`STOPPED_AT_TIME` and `END_OF_MEDIA` carry the media time as a `Long`,
`VOLUME_CHANGED` the player's own `VolumeControl`, `ERROR` the message,
and `CLOSED` `null`. The event names are the interned strings, so a game
that compares `event == PlayerListener.END_OF_MEDIA` works. A loop sends
`END_OF_MEDIA` at the end of every pass and `STARTED` at the start of
the next.

The thread also runs the timers: every playing track gets a `tick` (a
silent player's loops and end, a stop time reached) and says when it
next needs one, and the thread sleeps until the earliest, at most 200
milliseconds, so a stopping VM is noticed quickly.

The channel is bounded by a count, because a game can make events faster
than its listener takes them (a volume that flips, a start and a stop in
a loop, a sound that loops a few frames) and the Java heap would never
show it. Each note that waits adds one to a count, and the thread takes
one off before it handles the note, so a batch it is working through
still counts. Past 1024 waiting notes the repeats are dropped: `STARTED`,
`VOLUME_CHANGED` and the `END_OF_MEDIA` of a pass that another follows.
Past 2048 the events that end a run are dropped too: `STOPPED`,
`STOPPED_AT_TIME`, the last `END_OF_MEDIA` and `ERROR`. Then the player
stays held until its next run ends or it closes. `CLOSED` is never
dropped, and neither is the notice that an object was collected. The
engine's own queue ahead of this one holds 4096 events for all games and
drops in the same order (see [audio.md](audio.md)). A listener that keeps
up hears every event. A wake is queued once, however many starts ask for
one.

A start that fails (the game has 32 players started) lets go of the
player at once: no event would end a run that never began. A Nokia
`Sound.play` that is refused does the same, and the sound stays stopped.
An `ERROR`
event is also written to the host log as a warning, 16 times for an
instance, so a decoder that gave up or a sound that panicked leaves a
trace.

## Keeping players alive

Games create hundreds of players: one per effect, often never closed,
often started and dropped at once. The registry handles both:

- Each entry finds its Java object through a
  `java.lang.ref.WeakReference` held by a `GlobalRef`. A player the game
  forgot is collected; dropping its payload closes its engine player and
  frees the decoded sound, and tells the registry to drop the entry.
- While a player plays it is also held by a `GlobalRef` (taken by
  `start`, `play`, `resume`, `close` and `release`), so a sound the game
  started and dropped plays to its end and its listeners still hear
  `END_OF_MEDIA`. The event thread lets go when it delivers an event
  that ends the run (`STOPPED`, `STOPPED_AT_TIME`, `CLOSED`, or an
  `END_OF_MEDIA` the engine marks as the last pass) and the player is not
  playing again. The engine delivers a player's events in order, so
  nothing of that run comes after.

Listeners and controls live in the player's payload, which traces them;
a control points back at its player, so both stay alive together.

## Controls

| Control | Offered to | Notes |
| --- | --- | --- |
| `VolumeControl` | every player | `setLevel` clamps to 0 to 100 and returns it; `getLevel` is 100 until set; mute keeps the level. A change sends `VOLUME_CHANGED`. The level is kept across the engine player closing. |
| `ToneControl` | tone players (`device://tone` and `audio/x-tone-seq` data) | `setSequence` only while REALIZED, as the spec asks; a bad sequence throws `IllegalArgumentException`. The sequence is parsed with the VM lock held: it is small. |
| `MIDIControl` | MIDI players | `shortMidiEvent`, `longMidiEvent` (split into channel messages; system exclusive skipped), `setProgram` (bank select then program change), `setChannelVolume` (CC 7). Before PREFETCHED they throw `IllegalStateException`. `getChannelVolume` returns what the game set, else -1. Bank and program queries are not supported: `isBankQuerySupported` is false and the queries throw `MediaException`. |
| `StopTimeControl` | every player | The player stops itself when its media time reaches the stop time and sends `STOPPED_AT_TIME`; the stop time then resets. A second stop time on a started player throws. |

Every other control (`VideoControl`, `FramePositioningControl`,
`MetaDataControl`, `RateControl`, `TempoControl`, `PitchControl`,
`RecordControl`, `GUIControl`) is declared so games link, and
`getControl` returns `null` for it, as a phone without it does.

## Without an audio engine

`InstanceEnv.audio` is `None` when the app runs no engine (a headless
tool, a test). Players then get silent backends with the real lengths:
the sound is decoded once into a group of a private engine that is never
rendered, only to learn its length (and to check tone sequences and
beeps), and dropped. Everything else, states, events, loops, media time,
volume, stop times, works the same, timed by the wall clock.

## Nokia Sound

`com.nokia.mid.sound.Sound` uses the same tracks, registry and event
thread.

| Nokia | Behaviour |
| --- | --- |
| `Sound(byte[], FORMAT_TONE)` | A Smart Messaging ringing tone, or whatever format the bytes show. |
| `Sound(byte[], FORMAT_WAV)` | WAV, or whatever format the bytes show (MIDI given as WAV plays). |
| `Sound(freq, duration)` | A beep; a frequency outside 0 to 20000 Hz or a duration of 0 or less throws `IllegalArgumentException`. |
| Data that does not decode | No exception: the sound is `SOUND_UNINITIALIZED` and `play` does nothing (J2ME Loader does the same; Turrican spams errors otherwise), and so it is for a sound that decodes to more than 16 MiB. A format other than 1 or 5 throws `IllegalArgumentException`; `null` data `NullPointerException`. |
| A full sound group | `OutOfMemoryError` from `Sound(...)` or `init`, after one collection, when the game holds 128 players or 32 MiB of sound with MMAPI players and Nokia sounds together. The old sound stays. |
| `init(...)` | Replaces the sound; the old one is closed quietly. |
| `play(loop)` | Stops a playing sound, then plays from the start; 0 loops forever, a negative loop throws. |
| `stop`, `resume` | Stop and keep the place; resume from it. |
| `release` | Closes; the state becomes `SOUND_UNINITIALIZED`. |
| `setGain(0..255)`, `getGain` | Clamped; the level is `gain * 100 / 255`. `getGain` is 255 until set. |
| `getState` | `SOUND_PLAYING` while started, `SOUND_UNINITIALIZED` without a sound or after `release`, else `SOUND_STOPPED`. |
| `getSupportedFormats` | `{FORMAT_TONE, FORMAT_WAV}`. |
| `getConcurrentSoundCount` | 1 for either format, as Nokia phones played them; another format throws. |

`SoundListener.soundStateChanged` hears `SOUND_PLAYING` once per run,
`SOUND_STOPPED` at a stop or at the end of the last pass of a loop
(Harry Potter 3 waits for it), and `SOUND_UNINITIALIZED` after
`release`.

## For the launcher

- Install the `InstanceEnv` with `audio: Some(group)` before any MIDP
  class runs. The media module sets the group's listener itself on first
  use: the launcher must not call `Group::subscribe` or
  `Group::set_listener` on that group.
- Pausing an instance: `Vm::pause` and `group.set_paused(true)`. Players
  keep their states and send no events.
- Ending an instance: `group.close()`. Every engine player closes and its
  listeners hear `CLOSED`.

## Testing

- Unit tests sit beside the code: the silent clock, locators, content
  type decisions, MIDI message splitting, the declared constants.
- `tests/media_players.rs` and `tests/media_nokia.rs` compile the
  programs in `tests/media_java` with `javac --release 8` against the
  stubs in `tests/media_stubs` and run them on the VM with the MIDP
  classes and an offline engine at 8 kHz, which the programs move
  forward with the test-only `Pump` class (`advance`, `peak`,
  `players`). They cover states and exceptions, listener order and data,
  loops, media time, volume heard in the mix, tone sequences,
  `playTone`, wrong and missing content types, undecodable data,
  `MIDIControl`, `StopTimeControl`, hundreds of players opened, closed
  and forgotten, an instance without an engine, `DataSource` and Nokia
  `Sound`. `MediaMemory` and `NokiaMemory` cover what a game's sound
  costs: the heap counting a player's bytes and dropping them, the
  32 MiB and 128 player limits, forgotten players making room, an AMR
  file of empty frames and a stream that never ends. Without a JDK they
  pass with a note unless `FASTEMU_REQUIRE_JAVA=1`.

## Gaps

- `device://midi` (live MIDI with no file) is silent; the engine
  renders MIDI only while a file plays.
- The controls listed as returning `null` above are not provided.
- Players of the network (`http://`) are silent.
- Silent players keep wall-clock time while the VM is paused, so one
  that should have ended during the pause ends when it resumes.
- A stop time is checked when the event thread wakes for it, so the stop
  lands a few milliseconds late, and later when the engine renders in
  large steps.
- Players that are reachable but idle keep their decoded sound; only
  closed and collected players free it. A game holds at most 32 MiB of it
  and 128 players, and past that it gets `OutOfMemoryError`.
- Which classes a game sees (Nokia's on a Nokia profile only) is for the
  launcher to decide; `classes()` returns them all.
