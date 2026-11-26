# Security

## Reporting a vulnerability

Report security problems privately, through GitHub's
[private vulnerability reporting](https://github.com/fzn0x/FastEmu/security/advisories/new),
not in a public issue. Say what an attacker could do, and how to reproduce
it. If a crafted JAR triggers it, attach the file to the report. You will
hear back within a week, and be credited in the fix unless you prefer not to.

## Supported versions

FastEmu has no stable release yet. Fixes go into the newest release and
into main. Once there are several release lines, this table tells which
ones still get fixes.

| Version | Supported |
| --- | --- |
| The newest release | Yes |
| Older releases | No: update to the newest |

## What FastEmu protects

A Java ME game is code from someone else, often a file of unknown origin
from an old download site. FastEmu runs it in a virtual machine and treats
it as untrusted.

- **The sandbox.** A game reaches the outside only through the MIDP and
  JSR APIs FastEmu implements. It cannot read or write your files outside
  its own record stores and the files you give it, start programs, or touch
  the memory of FastEmu or of another game.
- **Each game on its own.** The games in one window do not share memory,
  record stores or threads. A crash or a runaway loop in one stays in that
  game.
- **The network.** A game opens connections only as far as the network
  policy allows, which you set per game. Reaching the network without that
  permission is a bug.
- **JAR and JAD files.** FastEmu reads them as hostile input: a damaged or
  crafted archive, class file or manifest must produce an error message,
  not a crash, a hang or unbounded memory use.

[Known limits](#known-limits) says where these promises stop.

## In scope

- A game escaping its sandbox: reading or writing outside its allowed files,
  running native code, or reaching another game's data.
- A crafted JAR, JAD or class file that crashes FastEmu, makes it hang or
  makes it use memory without a limit.
- A way around the network policy.
- A way for one game to take down the games next to it.

## Not in scope

- A game that does not run or looks wrong. Report it as a
  [game report](https://github.com/fzn0x/FastEmu/issues/new?template=game_report.yml).
- A game that harms itself, such as one that fills its own record store.
- Problems that need you to give a game permissions you were warned about.
- Findings in dependencies that FastEmu's use does not reach.

## Limits a game cannot pass

FastEmu holds each game to the limits below, so that a hostile game ends
in an error of its own and the other games run on. A game that gets past
one of them, or that makes FastEmu hold memory that none of them counts,
beyond what [Known limits](#known-limits) lists, is in scope.
Each limit is per game unless its row says otherwise. No setting changes
them, except the phone profile, which sets the heap. The last column
links to the page that describes the limit.

### Memory and threads

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Java heap | 2, 4, 8 or 16 MiB, set by the phone profile | `OutOfMemoryError` | [jvm.md][jvm-heap] |
| Memory the runtime makes for the game: strings, buffers, images, screens, sound bytes | counted on the heap | `OutOfMemoryError` | [jvm.md][jvm-heap] |
| One request of the runtime | at most the heap, and at most twice the heap with what the heap counts already | `OutOfMemoryError` before anything is built | [jvm.md][jvm-heap] |
| Java threads alive at once | 64, the event and lifecycle threads among them; the host may start 16 more, 80 in all | `OutOfMemoryError` from `Thread.start` or `new Timer()`, after one collection when a timer has no task left | [jvm.md][jvm-threads] |
| Stack of one thread | 131072 slots, 2 MiB | `StackOverflowError` | [jvm.md][jvm-limits] |
| Frames of one thread | 4096 | `StackOverflowError` | [jvm.md][jvm-config] |
| Calls from the runtime back into Java, nested on one thread, the runtime's own calls included | 256 | `StackOverflowError` | [jvm.md][jvm-config] |
| Work of a thread before a safepoint | 4096 backward branches or calls; a caught exception counts as 16 | a stop or a pause takes effect, and the other threads get a turn | [jvm.md][jvm-threads] |
| Stack trace of an exception | 256 frames | the rest is left out | [jvm.md][jvm-heap] |
| Name of a thread | 255 characters | cut | [jvm.md][jvm-heap] |
| Pause and resume calls waiting or running, besides the first `startApp` | 8 | later `pauseApp` and `startApp` calls are dropped, with one warning; the game still pauses and resumes | [midp.md][midp-threads] |
| Messages of the game waiting for its control thread (`resumeRequest`, `notifyDestroyed`, a crash) | one `resumeRequest` between two messages of the app, and one `notifyDestroyed` or crash | a `resumeRequest` right behind another that still waits is merged into it, and a message of the game after `notifyDestroyed` or a crash is dropped; the app's own messages, such as input, pause and stop, are never merged or dropped | [midp.md][midp-threads] |
| `callSerially` calls waiting, and posted tasks waiting | 10000 of each | `OutOfMemoryError` from `callSerially`; a posted task is dropped, with one warning | [midp.md][midp-events] |

[jvm-heap]: docs/architecture/jvm.md#the-heap-and-the-collector
[jvm-limits]: docs/architecture/jvm.md#limits
[jvm-threads]: docs/architecture/jvm.md#threads-and-the-vm-lock
[jvm-config]: docs/architecture/jvm.md#using-a-virtual-machine-from-the-host
[midp-threads]: docs/architecture/midp.md#threads
[midp-events]: docs/architecture/midp.md#the-event-loop

### Class files and suites

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| JAR file | 64 MiB | the game does not start, and the app says why | [midp.md][midp-suites] |
| JAD file | 1 MiB | the same; a JAD found beside a JAR is left out | [midp.md][midp-suites] |
| Manifest and JAD text | the first 1 MiB, 4096 attributes | the rest is not read | [midp.md][midp-suites] |
| Entries of a JAR | 65535, and 4 MiB of names | later entries are not seen | [midp.md][midp-suites] |
| One class file | 4 MiB | `NoClassDefFoundError`, or `ClassNotFoundException` from `Class.forName`; the log says so once | [midp.md][midp-suites] |
| JAR a JAD names in `MIDlet-Jar-URL` | a file in the JAD's folder or a folder below it; every part a plain name of at most 255 bytes, with no `..`, drive, share, stream or device name | that path is not tried; its last part, if a plain name, is tried in the JAD's folder, then the JAD's own name with `.jar`; with none of them the game does not start | [midp.md][midp-suites] |
| Classes of the runtime | a JAR cannot replace them, and no class under `java/` or `javax/` loads from it | the runtime's own class is used; a missing one is `NoClassDefFoundError` | [midp.md][midp-suites] |
| Class data | 128 MiB | `OutOfMemoryError`: "class memory limit reached" | [jvm.md][jvm-classes] |
| Class name | 255 bytes, 512 for an array class | `NoClassDefFoundError`, or `ClassNotFoundException` from `Class.forName` | [jvm.md][jvm-classes] |
| Array dimensions | 255 | `ClassFormatError` | [jvm.md][jvm-classes] |
| Classes on one path up a hierarchy | 256 | `ClassFormatError` | [jvm.md][jvm-classes] |
| MIDlet class a manifest or JAD names | 255 bytes, no brackets, no control characters | that MIDlet is left out | [midp.md][midp-suites] |
| Search for MIDlet classes, when no `MIDlet-<n>` gives a MIDlet | 8192 classes, 32 MiB inflated and 32 MiB compressed | the classes past it are not offered | [midp.md][midp-suites] |
| Search for MIDlet classes: classes between a class and `MIDlet` | 63 | that class is not offered | [midp.md][midp-suites] |
| A resource the game reads | what one request of the runtime may take: at most the heap, and at most twice the heap with what the heap counts already | `OutOfMemoryError`, and the read stops at that size | [midp.md][midp-suites] |
| Resources read at once | one for each game | the game's other threads wait for theirs | [midp.md][midp-suites] |
| Name of a resource | 1024 bytes or 64 segments | not found | [midp.md][midp-suites] |
| Missing classes, methods and fields reported, all in one count, and missing resources and class files over 4 MiB in another | 256 different names in each count, every name cut to 255 characters | no more are logged, and one line says so | [jvm.md][jvm-limits], [midp.md][midp-suites] |

[jvm-classes]: docs/architecture/jvm.md#class-loading-and-initialization
[midp-suites]: docs/architecture/midp.md#suites

### The log

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Log line | 4096 bytes | printed text is split into lines of 4096 bytes as the game encodes it; a line still longer once decoded or escaped, and any other long line, is cut, with a note of how much went | [jvm.md][jvm-limits] |
| Log kept between two reads of the app | 10000 lines and 1 MiB | the oldest lines go first | [midp.md][midp-log] |
| Message of a crash | 1 KiB | cut | [midp.md][midp-status] |

[midp-log]: docs/architecture/midp.md#the-log
[midp-status]: docs/architecture/midp.md#lifecycle-and-status

### Images and screens

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Pixels of an image | counted on the heap, 4 bytes a pixel | `OutOfMemoryError` before the pixels exist | [graphics.md][gfx-images] |
| One image | 2^25 pixels | `OutOfMemoryError` | [graphics.md][gfx-images] |
| Size an image file declares | checked against the heap before decoding; 16384 pixels a side | `OutOfMemoryError`; a JPEG over 16384 pixels a side that the heap could hold is a decode error instead: `IllegalArgumentException`, or `IOException` from `createImage(String)` and `createImage(InputStream)` | [graphics.md][gfx-decoding] |
| `Image.createImage(InputStream)` | 16 MiB, or 1000 reads in a row that give no bytes; the bytes read are counted on the heap as they grow | `IOException`; `OutOfMemoryError` when the heap cannot hold the bytes | [graphics.md][gfx-images] |
| Images one game decodes at once | one at a time: the other threads wait their turn with the VM lock released, before they read or copy their file; a stream is read first, and its bytes stay counted on the heap while the thread waits | no error of its own; streams read at once share the heap's room | [graphics.md][gfx-images] |
| First frame of a GIF with more pixels than its image | checked against the heap before it decodes, 5 bytes a pixel | the image does not decode: `IllegalArgumentException`, or `IOException` from `createImage(String)` and `createImage(InputStream)` | [graphics.md][gfx-decoding] |
| Repair of a damaged PNG | 64 MiB of rows | `OutOfMemoryError` | [graphics.md][gfx-decoding] |
| Extended XMP segments of a JPEG | 16 | the image does not decode | [graphics.md][gfx-decoding] |
| Nokia `drawPixels` block | 4194304 pixels | `IllegalArgumentException` | [graphics.md][gfx-nokia] |
| Glyphs kept for one font and size, for all games | 4096 above U+00FF | the cache empties and starts again | [graphics.md][gfx-text] |
| Cells of a `TiledLayer` | 2^24, 2^25 for Siemens' `TiledLayer`, and the heap | `OutOfMemoryError` | [midp.md][midp-game] |
| Animated tiles of a MIDP `TiledLayer` | 32767 | `OutOfMemoryError` | [midp.md][midp-game] |
| Tiles of a Siemens `TiledLayer`, static and animated | 32767 of each | `IllegalArgumentException` | [vendors.md][vendors-siemens] |
| Back buffer of a shown screen | counted on the heap, 4 bytes a pixel | counted, never refused; the game's next allocation fails | [midp.md][midp-frames] |
| Surface of a `CustomItem` | 8192 pixels a side, and the heap | a larger size is cut to 8192; a surface the heap cannot hold leaves the item blank, with one warning | [midp.md][midp-screens] |
| Text a `List` or `ChoiceGroup` row draws | 4096 UTF-16 units; past 256 elements, 1 Mi units shared by the rows, at least 128 a row | the row draws cut | [midp.md][midp-screens] |
| `drawArc`, or `drawRoundRect` with rounded corners | 65536 pixels wide or tall | nothing is drawn | [graphics.md][gfx-pixels] |
| Lights whose level is kept, every vendor API together | 16 | a change of any other light is logged each time and not kept | [vendors.md][vendors-lights] |

[gfx-images]: docs/architecture/graphics.md#images
[gfx-decoding]: docs/architecture/graphics.md#decoding
[gfx-nokia]: docs/architecture/graphics.md#nokia-directgraphics
[gfx-pixels]: docs/architecture/graphics.md#the-pixel-model
[gfx-text]: docs/architecture/graphics.md#text
[midp-game]: docs/architecture/midp.md#the-game-api
[midp-frames]: docs/architecture/midp.md#repaints-and-frames
[midp-screens]: docs/architecture/midp.md#the-high-level-screens
[vendors-siemens]: docs/architecture/vendors.md#siemens
[vendors-lights]: docs/architecture/vendors.md#lights-and-vibration

### Sound

The table gives what MMAPI does, and what the vendor sound APIs do
instead, as they throw no `MediaException`.

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| One sound in any format: decoded samples, the file of an MP3, the events of a MIDI file or a list of tones | 16 MiB | `MediaException`; a Nokia `Sound` stays uninitialized, and a Samsung or Siemens sound stays silent, with a warning | [audio.md][audio-players] |
| Sound of a game, every API together | 32 MiB and 128 players | `OutOfMemoryError`, after one collection; `ToneControl.setSequence` throws it without one | [audio.md][audio-players] |
| Players started at once | 32 for a game, 128 for all games | `MediaException` from `Player.start`; a Nokia, Samsung or Siemens sound stays silent | [audio.md][audio-players] |
| Time the audio thread spends rendering sound, in the desktop and Android apps | half of real time on average for all games together: an equal part for each game that plays sound; a game saves what it leaves unused, up to 20 ms of render time, and what it would save past that goes to the games that saved less; each game's oldest sound plays whatever it costs, and a game spends what it saved past its part, about a quarter of its part in each slice of 128 frames | the sounds the game started last fade out, or fall silent at once if nothing of them was heard, and wait in place, still started, with their media time held, until its sound fits again; no exception or event tells the game | [audio.md][audio-budget] |
| `playTone` tones playing | 16 | the oldest stops | [audio.md][audio-players] |
| Loop events of a player | one every 20 ms of sound | the loops between are not reported | [audio.md][audio-players] |
| Events of a MIDI file | 65536 | `MediaException`; a Nokia `Sound` stays uninitialized, and a Samsung or Siemens sound stays silent, with a warning | [audio.md][audio-formats] |
| Length of a MIDI file that loops | 20 ms | a shorter file plays once | [audio.md][audio-formats] |
| MIDI messages a MIDI player sends in one render call, a block of at most 256 frames | 512 | the rest go out in the next blocks | [audio.md][audio-formats] |
| Sample rate of a WAV file | 384000 Hz | the sound plays as silence, with a warning; a Nokia `Sound` stays uninitialized | [audio.md][audio-formats] |
| Notes of a Siemens `MelodyComposer` | 32767 | `appendNote` throws `Exception`; the constructor keeps the first 32767 | [vendors.md][vendors-siemens] |
| Tones of a Nokia ringing tone | 65536 | `MediaException`; a Nokia `Sound` stays uninitialized, and a Samsung sound stays silent, with a warning | [audio.md][audio-formats] |
| Tones of a tone sequence | 65536 in the sequence or in one block, 262144 in all its lists together | `IllegalArgumentException` from `setSequence`; a player of such data is silent | [audio.md][audio-formats] |
| AMR file | 30000 frames, ten minutes | `MediaException`; a Nokia `Sound` stays uninitialized, and a Samsung sound stays silent, with a warning | [audio.md][audio-formats] |
| Sound read from a stream | counted on the heap as it is read, so streams read at once share the heap's room; never more than the heap or 32 MiB | `OutOfMemoryError` | [media.md][media-players] |
| Sound events waiting | 1024 for a game, 4096 in the engine for all games; twice that for the events that end a run | the repeats are dropped first; `CLOSED` never is | [media.md][media-events] |
| An MP3 seek the reader cannot do | 8 s of sound caught up in each render call | silence until the place is reached | [audio.md][audio-gaps] |
| Hold of the audio thread on the shared mixer | one slice of 128 frames, with the lock handed to a waiting call after each slice | another game's call waits for the slice under way; a volume or mute change from the app never waits | [audio.md][audio-locking] |

[audio-players]: docs/architecture/audio.md#players
[audio-formats]: docs/architecture/audio.md#formats
[audio-gaps]: docs/architecture/audio.md#gaps
[audio-locking]: docs/architecture/audio.md#threads-and-locking
[audio-budget]: docs/architecture/audio.md#render-budget
[media-players]: docs/architecture/media.md#players
[media-events]: docs/architecture/media.md#listeners-and-the-event-thread

### Network and links

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Open connections: sockets, secure sockets, accepted sockets and HTTP | 64 | `IOException`, after one collection | [connections.md][net-hold] |
| Open server sockets | 8 | `IOException`, after one collection | [connections.md][net-hold] |
| Open datagram connections | 16 | `IOException`, after one collection | [connections.md][net-hold] |
| Request body of one HTTP connection | 4 MiB | `IOException` | [connections.md][net-hold] |
| Request bodies not sent yet, all connections together | 8 MiB | `IOException` | [connections.md][net-hold] |
| Request headers of one connection | 64, and 16 KiB | `IOException` | [connections.md][net-hold] |
| Connection name | 8 KiB | `IllegalArgumentException` | [connections.md][net-hold] |
| Interim `1xx` responses | 16 | `IOException` | [connections.md][net-hold] |
| Trailer of a chunked body | 16 KiB | `IOException` | [connections.md][net-hold] |
| Response head | 64 KiB | `IOException` | [connections.md][net-hold] |
| Chunk size line of a chunked body | 1 KiB, extensions included | `IOException` | [connections.md][net-hold] |
| Wait for a response head | 60 s | `InterruptedIOException` | [connections.md][net-blocking] |
| Name lookup and connect, TLS handshake, sending an HTTP request | 30 s each | `InterruptedIOException` | [connections.md][net-blocking] |
| Name lookups and connects under way, the ones the game stopped waiting for included | 16 | the next one waits for one to end, within its own 30 s; if none ends in time, `InterruptedIOException` | [connections.md][net-hold] |
| Datagram size | 65507 bytes | `IllegalArgumentException` from `newDatagram` | [connections.md][net-datagrams] |
| Web link a game asks to open | `http` or `https` with a host, printable ASCII with no space, no backslash, no user name, a port from 1 to 65535, at most 4096 bytes, not a `.jad` or `.jar` | `ConnectionNotFoundException` | [midp.md][midp-links] |

[net-hold]: docs/architecture/connections.md#what-a-game-may-hold
[net-blocking]: docs/architecture/connections.md#blocking-without-freezing-the-vm
[net-datagrams]: docs/architecture/connections.md#datagrams
[midp-links]: docs/architecture/midp.md#links

### Record stores

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Space of a suite's stores | 1 MiB | `RecordStoreFullException` | [rms.md][rms-quota] |
| Stores of a suite | 64 | `RecordStoreFullException` from the 65th `openRecordStore` | [rms.md][rms-quota] |
| Disk writes | a burst of 32 MiB, then 4 MiB a second; a write costs at least 4 KiB | the save waits, in slices of 50 ms; it is never refused | [rms.md][rms-budget] |
| Other suites whose stores a game opens | 16 | `RecordStoreException` | [rms.md][rms-where] |
| Name of a record store | 1 to 32 UTF-16 units | `IllegalArgumentException` | [rms.md][rms-names] |

[rms-quota]: docs/architecture/rms.md#sizes-and-the-quota
[rms-budget]: docs/architecture/rms.md#the-write-budget
[rms-where]: docs/architecture/rms.md#where-stores-live
[rms-names]: docs/architecture/rms.md#file-names

### The apps

| Limit | Bound | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Name, vendor, version and MIDlet names in the library | 100 characters | cut | [midp.md][midp-suites] |
| Description of a suite | 500 characters | cut | [midp.md][midp-suites] |
| Icon of a suite | 1 MiB of file, 512 pixels a side | no icon is shown | [midp.md][midp-suites] |
| Log a game's tile keeps | 2000 lines and 1 MiB | the oldest lines go first | [midp.md][midp-log] |
| Clicks on the prompt for a link | none for 500 ms after the app takes the request or its link changes | the buttons stay dimmed | [midp.md][midp-links] |
| A game opened on Android | `content:` links only, 64 MiB a file | a larger file is refused and its partial copy deleted; other links are ignored | [playing.md][playing-adding] |

[playing-adding]: docs/playing.md#adding-games

## Known limits

These facts hold even with the limits above. A report that shows only
one of them is not new. A way to make one of them worse than this
section says is in scope.

**Memory**

- The heap counts memory the way a phone does. A game made of very many
  tiny objects uses several times its heap limit in real memory, about
  ten times in a test with many small timer tasks.
- Each object a thread holds locked also takes an entry outside the heap,
  so a game that locks very many small objects at once uses more real
  memory than the ten times above.
- `String.intern` keeps a second copy of each interned string, and a
  table entry for it, outside the heap: about the heap's size again for
  long strings, and more for very short ones.
- A call into the runtime has each large request checked first, and is
  judged as a whole after it returns. It can take the heap past its limit
  for a moment, and its change stays: a `List.append` past the heap adds
  the element, then throws `OutOfMemoryError`.
- The back buffer of a shown screen is made on the event thread, so it
  is counted but never refused. A canvas keeps its buffer for as long as
  it lives, while a menu or a form that is hidden gives its buffer back.
- Each game has its own 32 MiB of sound, and nothing limits the sound of
  all games together: eight games running at once can hold 256 MiB.
- Each running game keeps its whole JAR in memory, up to 64 MiB, outside
  its heap.
- While a game makes an image from a file, FastEmu holds the file and
  the decoder's buffers beside the heap until the image is counted. A
  game's image decodes take turns, so this is one file of up to the
  heap's size and one decode for each game. A decode holds a few times
  the image's pixels, and a GIF whose first frame has more pixels than
  its image holds that frame too, of up to the heap's size.
- A resource that a call reads by name stays outside the heap until
  the thread gets the VM lock back and makes it a Java object. One
  resource inflates at a time for each game, but threads that read with
  `getResourceAsStream` or a sound locator at once can each hold one,
  of up to the heap's size, for that moment. A Samsung `AudioClip` made
  from a resource holds it until its sound is decoded, with the VM lock
  released.
- A Nokia `Sound` or a Samsung `AudioClip` made from a `byte[]` copies
  the bytes outside the heap and decodes them with the VM lock
  released. Threads that make such sounds at once can each hold a copy
  of up to the heap's size until their sound is decoded.
- The record stores a game holds open are kept in memory outside its heap
  count: at most about 1 MiB for its own suite and for each of the 16
  other suites it may open.
- A class file of up to 4 MiB is parsed before its size is judged. A
  class that is then refused holds a few times its file size meanwhile.
  The classes below it in a chain, which wait for it to load, are
  counted in the class data already.
- The class data is an estimate made from each class before it is
  built, and it is not exact. A set of classes with 60000 methods each,
  all with names of their own, held about a fifth more than it. A set of
  classes with 65535 static fields each kept 3% more than the 128 MiB.
  Just after the list of static values doubled, classes with instance
  fields filled the rest and kept 9% more, and 15% more for a moment.
- A game that has ended gives its memory back when its last thread
  ends, while its tile stays open. A thread still inside a call that
  cannot be stopped (see **Threads and time**) holds that memory until
  the call returns.

**Threads and time**

- A thread's stack holds at most 2 MiB. A game's machine has at most 80
  threads, 64 for the game and 16 more that only the host may start, so
  their stacks can hold 160 MiB at worst. While a stack grows, it
  holds its old buffer too, until the copy is done.
- Each thread also reserves 16 MiB of address space for its system
  stack. The system commits only what the thread uses.
- A `Timer` keeps its thread until `cancel()`, or until the game drops
  the timer and no task is left, as in Java. A game that keeps about 62
  timers, or makes them faster than their tasks run, gets
  `OutOfMemoryError`. A repeating task keeps the thread of a dropped
  timer until the task is cancelled.
- Only a lifecycle call that does not return, such as a `startApp` that
  runs the game's loop, or a very slow one, lets 8 pause and resume
  calls pile up; the ones after are dropped, as the table says.
- A game that pauses itself and asks to be resumed in a loop from a
  thread of its own gets a `startApp` for each request the control
  thread takes, as MIDP has it. Each runs on a new host thread, and
  nothing bounds how many start in a second: a debug build started
  about 400. They end at once, so only the processor pays.
- While a game's thread keeps the VM lock, the app waits a second at
  most. Then it drops the input it was posting and skips `pauseApp` or
  `destroyApp`, and a closed game shows as ended a few seconds later.
- FastEmu does not limit the CPU a game uses on its own threads. A game
  can keep the processor busy with drawing, decoding or computing for as
  long as it runs. Close its tile to stop it.
- A call into the runtime that does not call back into the game, such
  as a very large fill or the decoding of a sound, cannot be stopped. It
  runs on after you close the tile, and one that takes longer than the
  few seconds FastEmu waits for a closed game goes on after the game
  shows as ended. The limits above keep such calls short: a sound
  decodes to 16 MiB at most.
- A write on a secure connection checks for a stop only between its
  pieces of up to 64 KiB. A server that takes the data a little at a
  time keeps the game's thread inside one piece after the game is
  closed.

**Sound**

- A seek in a MIDI file replays up to 65536 events with the mixer held,
  about a millisecond each time. A game that seeks in a long MIDI file
  in a loop can make the sound of every game stutter. It cannot stop
  their sound. This is one way a runaway game reaches the others,
  beside the processor time above.
- A game can pass the render budget in the table in one slice. It can
  take up to two and a half times its part after slices in which it took
  less. On top of that comes its oldest sound, which only the other
  limits of sound hold, such as 512 MIDI messages and 8 s of MP3 caught
  up in each render call, up to two more sounds whose cost it learns
  only once they rendered, and the fade-outs of the audible sounds it
  holds back. Measured on a desktop that other work shared, four games
  that each started 32 MIDI players at once, and four or eight games
  whose silent MIDI players all started their notes at once, or that
  closed those players every 200 ms and started new ones, made at most
  2 buffers of a run late, by 1.3 ms in all. One game that started 32
  MIDI players beside a game playing a real game's MIDI file made 1
  buffer late, by 5.4 ms, in one of two runs, most likely the busy
  machine pausing the thread. When the players are heard before their
  notes, the fades of the ones held back render too: four such games
  made 2 of 375 buffers late in each run, by 1.2 to 1.4 ms in all. The
  budget reads the system's clock, so time the system takes from the
  audio thread is charged to the game whose sound was rendering: on a
  busy computer a normal game's newer sounds can pause. A computer too
  slow to render even the oldest sound of every game in time still
  stutters.

**Class files and suites**

- There is no type verifier. A crafted class can hand a runtime call an
  object of the wrong type. That stops only its own game, which shows as
  crashed with an internal error.
- A symlink or junction inside a JAD's own folder is still followed.
  Planting one there needs write access to that folder.
- One `getResourceAsStream` call may look up as many as 64 forms of its
  name, dropping one folder at a time. A hostile JAR can make each a
  long read, so one call can take seconds.

**Record stores**

- A game that never stops saving still writes 4 MiB a second to the
  disk.
- Reads and deletes are not charged. A loop that makes and deletes a
  store is held only by the 4 KiB each store costs, about 1000 stores a
  second.
- A save that waits holds the game's file lock. The game's other
  threads then wait to open, list or delete a store.
- `destroyApp` gets one second. A game past its burst can lose its last
  saves when it is closed, since a save still waiting then is not
  written.
- Each save rewrites the whole store file, so filling a store one record
  at a time costs time by the square of its size.
- Stores past the 64th that an older FastEmu made still open and change.

**Network**

- **Allow network access** means the whole network. A game with network
  access can reach programs on your own computer, devices on your local
  network and link-local addresses, not only the internet. Leave the
  network off for games that do not need it.
- A game you allow network access can also accept connections on every
  network interface of your computer, and can learn the address of your
  computer on the local network.
- A name lookup or a connect runs on a helper thread. When the game
  stops waiting for it, on an interrupt, at its deadline or because the
  game was closed, the helper runs on: a connect until its 30 s are up,
  unless its own name lookup lasts longer, and a lookup until the
  system's resolver gives up. A game has at most 16 helpers alive, and
  they are not among the 80 threads of its machine, so a closed game can
  leave 16 threads and their sockets behind for up to 30 s, or longer
  while a lookup lasts.
- Every game can read your time zone, with or without network access.

**Links and the apps**

- Links to `localhost`, private addresses and downloads other than a
  `.jad` or `.jar` are still offered. You read the whole link before you
  choose **Open**.
- The 500 ms in which a link prompt ignores clicks starts when the
  app takes the request, not when a tile in the focus layout comes
  forward.
- The Android app checks no free space before it copies a game, and its
  limit of 64 MiB is for each file.
- Debug builds of the Android app still open a file path given in the
  `fastemu.open` extra, for testing with adb. Release builds ignore it.
- A log record printed to the terminal stays on one line. Its line
  breaks show as `\n`.
