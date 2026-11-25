# Audio

The `fastemu-audio` crate is the sound engine behind MMAPI (JSR-135,
`javax.microedition.media`) and the vendor sound APIs such as Nokia's
`com.nokia.mid.sound.Sound`. It is a plain Rust library: it knows nothing
about the virtual machine, and the MIDP crate wraps it in Java classes.

This page describes how the engine is built, which libraries it uses and
why, and how the MIDP layer should map each MMAPI call onto it.

## Shape

```text
             app (desktop, Android, headless runner)
              |  Engine::default(), AudioOutput::open or NullOutput::start
              v
  +--------------------------- Engine ----------------------------+
  |  Mixer (behind one mutex)                                     |
  |    Group 1 (instance 1): mute, pause, listener                |
  |      volume: an atomic its Group handles set with no lock     |
  |      Player -> Voice -> Source (WAV, MP3, MIDI, tones, ...)   |
  |      Player -> Voice -> Source                                |
  |    Group 2 (instance 2) ...                                   |
  |  render(): resample each playing voice, sum, headroom, limit  |
  +---------------------------------------------------------------+
              |  interleaved stereo f32 at the output rate
              v
       AudioOutput (cpal) or NullOutput or a test calling render()
```

- **Engine**: one per app. Cloning it is cheap. It does no work on its
  own: whoever owns the output calls `Engine::render(&mut [f32])` for each
  buffer. `set_sample_rate` follows the device.
- **Group**: one per running instance. It has a volume (0.0 to 1.0, a
  linear gain), mute, pause, and one event listener. Closing it, or
  dropping its last handle, stops and frees every player in it.
- **Player**: one MMAPI player. Made by a group from bytes. Dropping it
  closes it.
- **Source** (internal): a stream of stereo frames at its own rate with a
  length and a seek. Each format provides one.
- **Voice** (internal): a player inside the mixer. It resamples its source
  to the output rate and tracks loops, media time and the end of media.

## Threads and locking

All mixer state but each group's volume sits behind one lock, a
`parking_lot` mutex. It is fair over about a millisecond: a caller that
has waited that long gets it next, so a
thread that takes it again and again (a game that seeks a big MIDI file in a
loop) cannot keep the audio thread or another game's calls out. API calls
hold it briefly. The audio thread renders a buffer in slices of 128 frames
and frees the lock between them, so a call waits for the slice under way, or
about a millisecond while the slices are short. Decoding and parsing happen
before the lock is taken (when a player is created), so the only work under
the lock is rendering, which is the work the audio thread must do anyway.
The lock has no poison: a panic while it was held leaves the mixer as it was
rather than silencing every game.

What one game can make the lock cost is bounded:

- a game has at most 32 players started, and all games together 128 (see
  Players);
- a MIDI file of a few frames plays once, a file sends at most 512
  messages for each call to render, and a file holds at most 65536
  events (see Formats);
- listeners hear of a loop at most every 20 ms of output per player.

A seek in a MIDI file replays the controllers before the new position. It
is up to 65536 events under the lock, about a millisecond. That stays a
known limit, and the fair lock keeps it from starving anyone.

A group's volume is the one value outside the lock. `Group::set_volume`
stores it in an atomic that the group's handles share with the mixer,
and the mix reads it for every slice. The app sets a game's volume from
its UI thread, and mutes a game by setting its volume to 0, so that call
never waits for the audio thread or for a game's player calls. The mix
takes up a new volume at its next slice, and each voice slides to it
over 3 ms like any gain change, so it does not click. A player's own
level, `VolumeControl.setLevel`, stays under the lock: it comes from the
game's threads, which wait for the mixer in every player call anyway,
and its `VolumeChanged` event keeps its order among the player's other
events. `Group::set_muted`, `Group::set_paused` and
`Engine::set_master_volume` stay under the lock too; the app does not
call them from its UI thread.

[SECURITY.md](../../SECURITY.md#limits-a-game-cannot-pass) lists the
limits of sound with every other limit a game meets, and its
[known limits](../../SECURITY.md#known-limits) say what this lock still
lets one game do to the others.

The audio thread never unwinds. Each voice renders inside `catch_unwind`:
a source that panics (a decoder or synthesizer bug on damaged data) is
replaced by an empty one, stopped, and its run ends with an `Error` event
and the last `EndOfMedia`. The other voices play on. `Engine::render` also
catches anything else that panics while mixing, and a listener that
panics, and leaves silence for that slice. The output callback catches
what is left and writes silence.

Events are queued while the lock is held, so their order is the order
things happened, and delivered after it is released, so a listener may
call back into the engine. A listener runs on whichever thread caused the
event: the caller's thread for `start`, `stop`, `close` and volume
changes, the audio thread for the end of media and loops. It must return
quickly. `Group::subscribe` gives a channel instead, which is what the
MIDP layer should use: its own event thread reads the channel and calls
`PlayerListener.playerUpdate`, which MMAPI requires to happen
asynchronously anyway.

The event queue is bounded, because a listener may block and a game can
make events faster than a listener reads them. It holds 4096 events
(`MAX_QUEUED`). Past that it drops the repeats: `Started`,
`VolumeChanged` and the `EndOfMedia` of a pass that another follows
(`EventClass::Repeat`). Past twice that it drops the events that end a run
too (`Stopped`, the last `EndOfMedia`, `Error`; `EventClass::RunEnd`).
`Closed` is never dropped, so a player's data can always be freed.
`Engine::dropped_events` counts what was lost. A listener that keeps up
loses nothing. `Group::subscribe` is for tests and examples: the channel
it makes has no bound, so a receiver nobody reads grows.

Listeners are dropped with the lock released, because a listener may own
player or group handles whose drops take the lock.

## Players

A player follows the MMAPI state machine, except that UNREALIZED lives in
the MIDP layer (see the mapping below):

| State        | `mmapi_code()` | Meaning                                   |
| ------------ | -------------- | ----------------------------------------- |
| `Realized`   | 200            | Created: data checked and decoded or parsed |
| `Prefetched` | 300            | Ready; also where a player lands at the end |
| `Started`    | 400            | Playing                                   |
| `Closed`     | 0              | Gone; every call but `state` fails        |

- `start` plays from the current media time, or from the start if the
  last run reached the end. `stop` keeps the media time exactly.
- `set_loop_count(n)`: `n` passes, `-1` forever, `0` is refused, and not
  while started. Each pass ends with `EndOfMedia`, and each new pass
  starts with `Started { media_time_us: 0 }`, as JSR-135 asks. A stopped
  player resumes the remaining passes. `EndOfMedia` says with `last`
  whether the player stopped there or another pass follows: by the time
  a listener reads the event, the player's state may already be past the
  whole loop.
- A loop is heard with no gap, but listeners are told of it at most once
  every 20 ms of output per player (`MIN_LOOP_US`): a sound of one frame
  at 384 kHz loops 384000 times a second, and each pass would be two
  events. The first loop after a start, and the end of a run, are always
  told.
- At most 32 players of a game and 128 of all games together are started
  at once (`MAX_STARTED_PER_GROUP`, `MAX_STARTED_TOTAL`). `start` on one
  more fails with `AudioError::Busy`, which the MIDP layer throws as the
  `MediaException` of a player that cannot start. A player that stopped,
  ended or closed gives its place back. `playTone` tones are not players
  and have their own cap.
- Media time is in microseconds. `set_media_time` clamps to the media and
  returns the time set; it works while playing.
- `set_level(0..=100)` clamps and returns the level. The scale follows
  JSR-135: level 50 sounds half as loud as 100 (about 10 dB down), so the
  gain is `(level / 100) ^ 1.66`. `set_muted` keeps the level. Both post
  `VolumeChanged` when something changes.
- Events: `Started`, `Stopped`, `EndOfMedia` (each with the media time),
  `VolumeChanged { level, muted }`, `Closed`, and `Error { message }` when
  a decoder gives up on damaged data partway (an MP3 that stops decoding).
- Loop and end events fire when the playhead reaches them, not when the
  source is read ahead, so media time and events agree to the frame.

Decoded sound is kept as 16-bit PCM (half the size of `f32`). MP3 is the
exception: it is decoded a frame at a time while it plays, because a long
background track would take tens of megabytes as PCM.

Games create hundreds of players over a session. A closed player is
removed from the mixer at once and its data dropped. A stop, seek, pause
or close of a playing player leaves a short fade-out ("tail", a few
milliseconds pre-rendered) in the mix instead of keeping the player
around; the tail list is capped so closing a game with a hundred players
stays cheap.

A game's sound is capped, because the Java heap never sees it. A group
holds at most 128 players and 32 MiB of sound (`GroupLimits`; the host
changes them with `Group::set_limits`, tests with tiny values). The bytes
are what each source says it holds (`Source::memory_bytes`): the samples
of decoded sound, the file of an MP3, the events and time signatures of
a MIDI file, the tones of a tone sequence. They are added up from the
sources the group holds now, so a closed or dropped player gives its
bytes back, and a tone sequence that replaces another counts in its
place. `Group::insert` checks both
limits, and every player is made there: MMAPI players, tone players,
beeps, and the Nokia, Samsung and Siemens sounds. `playTone` tones are
not players and keep their own cap of 16, below. A sound that is bigger
than 16 MiB (`MAX_DECODED_BYTES`, in any format) or than the whole budget
is `TooLarge`; any other request the group has no room for is
`OutOfMemory`, which closing a player cures. A request the group refuses
leaves nothing behind.

`Group::play_tone` (for `Manager.playTone`) plays a tone with no handle
that leaves the mixer when it ends. A group keeps at most 16 such tones;
a new one replaces the oldest, because games call `playTone` in loops.

## Formats

`Format::detect(data, content_type)` trusts the bytes first and the
declared type second, because games often pass the wrong type:

| Format          | Signature                        | Content types                                          |
| --------------- | -------------------------------- | ------------------------------------------------------ |
| WAV             | `RIFF....WAVE`                   | `audio/x-wav`, `audio/wav` (and `audio/wave`, `audio/vnd.wave`) |
| MIDI            | `MThd`, or `RIFF....RMID`        | `audio/midi`, `audio/mid`, `audio/x-midi`, `audio/sp-midi` |
| AMR narrowband  | `#!AMR\n`                        | `audio/amr`                                            |
| MP3             | `ID3`, or an MPEG layer III header | `audio/mpeg`, `audio/mp3` (and `audio/mpeg3`, `audio/x-mp3`, `audio/x-mpeg`) |
| Tone sequence   | `0xFE 0x01` (VERSION 1)          | `audio/x-tone-seq`                                     |
| Nokia ringtone  | a command count, then the programming or sound command | `application/vnd.nokia.ringing-tone` |

`SUPPORTED_CONTENT_TYPES` lists what `Manager.getSupportedContentTypes`
should report.

- **WAV**: PCM 8 (unsigned), 16, 24 and 32-bit, IEEE float (32 and 64-bit),
  IMA ADPCM, Microsoft ADPCM (with listed or standard coefficients), mu-law,
  A-law, and the extensible format. Wrong sizes are tolerated: a data size
  of zero or past the end runs to the end of the file, a RIFF size is
  ignored, a missing pad byte after an odd chunk is noticed, a torn last
  frame is dropped, and a lost chunk walk searches for `data`. Wider than
  stereo keeps the front left and right. A rate of 0 or above 384 kHz is
  `Malformed`: the mixer reads a source at its rate over the output rate, so
  a garbage rate would read billions of frames for each frame played. A file
  decodes up front, to at most four times its size (ADPCM) and twice for
  mu-law, A-law and 8-bit PCM. What it will decode to is worked out from the
  bytes there are, never from a size the header claims, and a file that
  would decode to more than 16 MiB (`MAX_DECODED_BYTES`) is `TooLarge`
  before anything is allocated.
- **MIDI**: standard MIDI files, format 0 and 1 (format 2 plays as 1),
  bare or in RMID. Tracks are merged and timed in microseconds through the
  tempo map (SMPTE division too); running status is followed; system
  exclusive data is skipped; time signatures are kept. The length runs to
  the end of the longest track, trailing rests included, so loops keep
  their rhythm. A track length that does not land on another chunk is
  wrong: the track is read to its end-of-track event instead. Seeking
  resets the synthesizer and replays program changes, controllers and
  pitch bends before the new position (not notes). A file of more than
  65536 events, tempo and time signature changes counted, is `TooLarge`;
  game music has some thousands. Starting a file over for a loop sends all
  its events again, so a file shorter than 20 ms (`Source::min_loop_frames`)
  plays once whatever its loop count, and ends with the last
  `EndOfMedia`. Sound that is not MIDI loops however short it is. The
  sequencer sends at most 512 messages for each call to `render`, which is
  a block of at most 256 frames: a file with a few hundred events at its
  start plays as before, and a pile of tens of thousands at one instant is
  sent in order over the next blocks.
- **MP3**: MPEG-1 and MPEG-2 layer III. The length comes from the Xing or
  LAME header without the encoder delay and padding (which the decoder
  trims), or else from the frame headers. ID3v2 tags are skipped. An MP3
  that fails to decode more than 16 frames in a row ends there, with an
  `Error` event.
- **AMR narrowband**: the RFC 4867 storage format (`#!AMR\n`), mono 8 kHz,
  decoded up front. Frames of other codecs are concealed like lost frames;
  a file cut short keeps what decoded. AMR wideband (`#!AMR-WB\n`) and the
  multi-channel AMR header are not supported. A frame can be one byte (a
  NO_DATA frame becomes 160 concealed samples, 320 bytes), so the frame
  headers are counted first and a file of more than ten minutes (30000
  frames, 9.6 MB of samples) is `TooLarge` before any frame is decoded.
  The samples are reserved once, with `try_reserve_exact`.
- **Tone sequences**: the JSR-135 `ToneControl` grammar: `VERSION 1`,
  optional `TEMPO` (5 to 127, times 4 bpm) and `RESOLUTION` (1 to 127) in
  either order, blocks (`BLOCK_START n ... BLOCK_END n`, defined before
  use, not nested), `PLAY_BLOCK`, `SET_VOLUME` (0 to 100), `REPEAT` (2 to
  127), notes 0 to 127 and `SILENCE`. A sequence or a block holds at most
  65536 tones, and all its lists together 262144, `SET_VOLUME` entries
  included. A note lasts `duration * 60000 * 4 / (resolution * tempo)` ms.
  Errors carry the byte offset where reading stopped. Sequences that
  unroll past 65536 tones are refused, since blocks that play blocks grow
  without bound.
- **Nokia ringing tones**: the Smart Messaging bit stream (commands,
  basic and temporary songs, patterns with loops and back references, note,
  scale, style, tempo and volume instructions). Volume 1 to 15 maps to
  MMAPI levels 44 to 100. A pattern loop value of 15 (forever) plays once;
  the `Sound` object's own loop count covers repetition. A pattern that
  was defined can play again from a header of two bytes, so a file of a
  kilobyte can unroll to millions of tones: the tones are counted as they
  are made, and one that unrolls past 65536 is `TooLarge`.
- **Tones** (sequences, `playTone`, Nokia songs and beeps) play as a
  band-limited square wave with 2 ms edges, the way phone tone generators
  sound.

## MIDI synthesis

The preferred path was a pure Rust SoundFont synthesizer with a small
General MIDI SoundFont shipped inside FastEmu. The synthesizer is
[rustysynth](https://github.com/sinshu/rustysynth) (MIT). No SoundFont
fits, after reading the licenses:

| SoundFont        | Size    | License                                    | Why not |
| ---------------- | ------- | ------------------------------------------ | ------- |
| TimGM6mb         | 6 MB    | GPL-2.0 only (Debian's copyright file: "version 2"); the author said some samples are his and the rest "either public domain or fall under the terms of the GNU General Public License" | GPL-2.0-only cannot combine with GPL-3.0-or-later, and the sample sources are vague |
| GeneralUser GS 2.0.3 | 32 MB | Free use, may be shipped in software; the author cannot vouch for every sample's origin | Four times the size budget |
| FluidR3_GM, FluidR3Mono | 141 MB, 23 MB (SF3) | MIT | Too large; SF3 is Vorbis-compressed, which rustysynth does not read |
| MuseScore General | 208 MB (SF2), 38 MB (SF3 lite) | MIT | Too large |
| FreePats General MIDI | 227 MB | GPL-3.0-or-later with an exception | Too large, and incomplete |

So MIDI always plays through a **built-in General MIDI synthesizer** that
needs no files, and users who want better sound **load their own `.sf2`**
(`SoundFont::from_bytes`, then `Engine::set_soundfont`), which rustysynth
plays. The app can point users at GeneralUser GS; its license asks that
downloads link to the author's site rather than to his files.

The built-in synthesizer gives each of the sixteen GM instrument families
an oscillator (sine, band-limited saw, square or pulse, a drawbar organ,
two-operator FM, detuned saws, or noise), an ADSR envelope and a
key-tracked low pass. Channel 10 plays drums from pitch-swept sine tones
and filtered noise, mapped by key (kicks, snares, toms, hats with the
closed hat choking the open one, cymbals, hand drums). It follows note
on and off with velocity, program changes, volume (CC 7), pan (CC 10),
expression (CC 11), sustain (CC 64), pitch bend with the RPN 0 bend
range, all sound off (CC 120), reset controllers (CC 121) and all notes
off (CC 123 to 127). It has 32 voices and steals the quietest released
voice, or else the oldest. It aims at a phone's MIDI chip, not a sampler.

A SoundFont player renders with reverb and chorus off: they cost more
than the rest of the synthesizer, and many games may play MIDI at once.
A SoundFont that fails to parse, even one that makes the parser panic, is
reported as `Malformed`. Players keep the synthesizer they were created
with; changing the SoundFont affects players created afterwards.

## Mixing

- Each playing voice is resampled from its source rate to the output
  rate with 4-point Catmull-Rom interpolation. At the same rate it passes
  samples through untouched. MIDI and tones render at the output rate the
  engine had when the player was created (rustysynth needs 16 to 192 kHz;
  outside that it uses 44.1 kHz).
- The gain of a voice is its level times its group's volume, zero when
  either is muted; it slides to a new value over 3 ms, and a start ramps
  up from silence over 3 ms.
- Voices are summed in f32, scaled by a headroom of 0.7 and the master
  volume, then limited: a peak limiter with instant attack and 150 ms
  release holds peaks under 0.9, and a tanh soft clip above 0.9 keeps the
  output inside -1 to 1 whatever hundreds of games do.

## Output

- Feature `output` adds `AudioOutput::open(&engine)`: the default device
  through [cpal](https://github.com/RustAudio/cpal) (WASAPI on Windows,
  CoreAudio on macOS, ALSA on Linux, AAudio on Android) at its preferred
  rate and format (f32, f64, i32, i16, i8, u16 or u8), with stereo folded
  to mono or padded for wider layouts. A thread owns the stream. When the
  device goes away it reopens the default device, retrying every second.
- Without a device, `NullOutput::start(&engine)` renders in real time and
  throws the sound away. Games that wait for `END_OF_MEDIA` (an intro
  jingle, a voice line) need time to move, so the apps should fall back
  to it when `AudioOutput::open` fails, and the headless runner should use
  it, or call `Engine::advance(frames)` on its own clock.
- Tests call `Engine::render` directly and inspect the samples.

## How the MIDP layer maps MMAPI

[media.md](media.md) describes the MIDP side as built; this section is
the engine's view of it.

### The app and the instance

| Event                            | Engine call |
| -------------------------------- | ----------- |
| App start                        | `Engine::default()`, then `AudioOutput::open(&engine)` or `NullOutput::start(&engine)`; `engine.set_soundfont(...)` if the user chose one |
| Instance launch                  | `let group = engine.create_group(); group.set_volume(settings.volume)`; the MIDP media module sets the group's listener on first use and reads events on its own thread, so nothing else subscribes |
| `Instance::set_volume(v)`        | `group.set_volume(v)` (0.0 also mutes); it takes no lock, so the app's UI thread never waits for the mixer |
| `Instance::pause` / `resume`     | `group.set_paused(true)` / `false`, besides `pauseApp` and `startApp` |
| `Instance::stop`, crash, exit    | `group.close()` (or drop every clone) |

### `javax.microedition.media`

| MMAPI                                         | Engine |
| --------------------------------------------- | ------ |
| `Manager.createPlayer(InputStream, type)`     | Read the stream to the end and keep the bytes and type in the Java `Player` (UNREALIZED). |
| `Manager.createPlayer(locator)`               | `device://tone`: keep a flag, and on realize call `group.create_tone_player()`. `file://`, `resource` paths and `http://` (when the network policy allows): fetch the bytes, then as above. Other `device://` locators: `MediaException`. |
| `Player.realize()`                            | `group.create_player(&bytes, Some(type))`; map errors (see below). Drop the Java copy of the bytes afterwards, which the Java heap counted until then. A game whose group is full gets one collection and a second try before the error. |
| `Player.prefetch()`                           | `player.prefetch()` (realizing first if needed) |
| `Player.start()`                              | `player.start()` (realizing and prefetching first if needed) |
| `Player.stop()`                               | `player.stop()` |
| `Player.deallocate()`                         | `player.deallocate()` |
| `Player.close()`                              | `player.close()`; also when the Java object is finalized |
| `Player.getState()`                           | `player.state().mmapi_code()`, or 100 while UNREALIZED |
| `Player.setLoopCount(n)`                      | `player.set_loop_count(n)` |
| `Player.getMediaTime()` / `setMediaTime(t)`   | `player.media_time()` / `player.set_media_time(t)` |
| `Player.getDuration()`                        | `player.duration()`, or `TIME_UNKNOWN` (-1) while UNREALIZED |
| `Player.getContentType()`                     | `player.content_type()` (the detected format, which may differ from what the game passed) |
| `Player.addPlayerListener(l)`                 | Keep listeners per player in Java; route each `PlayerEvent` from the group's channel by `event.player`. |
| `Player.getControl("VolumeControl")`          | `set_level`, `level`, `set_muted`, `is_muted` |
| `Player.getControl("ToneControl")`            | Only for tone players: `setSequence(bytes)` is `player.set_tone_sequence(bytes)` |
| `Player.getControl("MIDIControl")`            | Only for MIDI players: `shortMidiEvent(type, d1, d2)` is `player.midi_short_event(type, d1, d2)` (type 0x80 to 0xFF except 0xF0 and 0xF7; a realized player gives `InvalidState`, as JSR-135 asks); `setProgram(ch, bank, prog)` sends CC 0 and CC 32 (bank) then a program change, or only the program change when the bank is -1 (the default bank); `setChannelVolume(ch, v)` sends CC 7; `getChannelVolume` may return -1 (unknown); `isBankQuerySupported` returns false. |
| Other controls                                | Return `null` (not provided yet, see the gaps) |
| `Manager.playTone(note, duration, volume)`    | `group.play_tone(note, duration, volume)` |
| `Manager.getSupportedContentTypes(protocol)`  | `SUPPORTED_CONTENT_TYPES` |

Errors map to exceptions:

| `AudioError`                          | Java exception |
| ------------------------------------- | -------------- |
| `Unsupported`, `Malformed`, `TooLarge`, `Busy` | `MediaException` |
| `InvalidToneSequence`, `InvalidArgument` | `IllegalArgumentException` |
| `InvalidState`, `Closed`              | `IllegalStateException` |
| `OutOfMemory`                         | `OutOfMemoryError` |

Events map to `PlayerListener` constants and event data:

| `PlayerEventKind`                 | Event            | Event data |
| --------------------------------- | ---------------- | ---------- |
| `Started { media_time_us }`       | `STARTED`        | `Long` media time |
| `Stopped { media_time_us }`       | `STOPPED`        | `Long` media time |
| `EndOfMedia { media_time_us, .. }` | `END_OF_MEDIA`  | `Long` media time |
| `VolumeChanged { .. }`            | `VOLUME_CHANGED` | the player's `VolumeControl` |
| `Closed`                          | `CLOSED`         | `null` |
| `Error { message }`               | `ERROR`          | `String` message |

### `com.nokia.mid.sound.Sound`

| Nokia                                  | Engine |
| -------------------------------------- | ------ |
| `Sound(byte[] data, FORMAT_TONE)`      | `group.create_player_as(&data, Format::NokiaRingtone)` |
| `Sound(byte[] data, FORMAT_WAV)`       | `group.create_player_as(&data, Format::Wav)` (or `create_player(&data, None)` to sniff) |
| `Sound(int freq, long duration)`       | `group.create_beep_player(freq, duration)` |
| `init(...)`                            | Close the old player and create a new one |
| `play(loop)`                           | `set_loop_count(if loop == 0 { -1 } else { loop })`, then `start()` |
| `stop()` / `resume()`                  | `stop()` / `start()` |
| `setGain(0..255)` / `getGain()`        | `set_level(gain * 100 / 255)`; keep the gain in Java to return it |
| `getState()`                           | `Started` is `SOUND_PLAYING`, a closed player is `SOUND_UNINITIALIZED`, anything else `SOUND_STOPPED` |
| `release()`                            | `close()` |
| `SoundListener.soundStateChanged`      | From the group's events: `Started` to `SOUND_PLAYING`, `Stopped` and `EndOfMedia { last: true, .. }` to `SOUND_STOPPED` |
| `getConcurrentSoundCount(type)`        | Any small number the games accept; the engine plays as many as asked |

Samsung's `AudioClip` hands its bytes to `create_player`, which finds the
format from them. Siemens' `Melody` plays the MIDI file its
`MelodyComposer` builds, through `create_player_as`. Motorola, LG and Sony
Ericsson have no sound API of their own.

## Licenses

| Crate                  | Version | License    | Used for |
| ---------------------- | ------- | ---------- | -------- |
| symphonia-core, symphonia-bundle-mp3 | 0.6.1 | MPL-2.0 | MP3 decoding |
| amr-nb                 | 0.2.2   | Apache-2.0 (a port of opencore-amr, Apache-2.0, with 3GPP TS 26.073 portions distributed under its terms) | AMR narrowband decoding |
| rustysynth             | 1.3.8   | MIT        | SoundFont synthesis |
| cpal (feature `output`) | 0.18.2 | Apache-2.0 | The audio device |

All are compatible with GPL-3.0-or-later. No SoundFont or other sample
data ships with FastEmu. The one binary test file,
`crates/fastemu-audio/tests/data/sine-440hz-1s.mp3`, is a generated sine
tone (ffmpeg's `sine` source encoded with LAME).

## Gaps

- `device://midi` (a MIDI player with no file, for `MIDIControl` alone)
  is silent: the MIDP side gives a player that takes every event and
  plays nothing, as a MIDI player renders only while it plays its file.
- No `RateControl`, `TempoControl`, `PitchControl`, `MetaDataControl` or
  `FramePositioningControl`. `StopTimeControl` is there, on the MIDP
  side ([media.md](media.md)).
- Players of `http://` locators are silent: the MIDP side does not
  fetch them, whatever the network policy.
- No AMR wideband, AAC, iMelody, MMF (SMAF) or XMF.
- Media time does not account for the device's output latency (usually
  10 to 30 ms).
- MP3 seeks that the reader cannot do accurately (a damaged header in the
  middle of the file) fall back to decoding from the start. A render call
  decodes at most 8 seconds of that, so the mixer is not held for long,
  and plays silence until the target is reached: the silence takes its
  share of the timeline, so the sound that follows is the sound of its
  place in the file. MIDI seeks replay controllers but not notes already
  sounding.
