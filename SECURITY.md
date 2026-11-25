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
one of them, or that makes FastEmu hold memory outside them, is in scope.
Each limit is per game unless its row says otherwise. No setting changes
them, except the phone profile, which sets the heap. The last column
links to the page that describes the limit.

### Memory and threads

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Java heap | 2, 4, 8 or 16 MiB, set by the phone profile | `OutOfMemoryError` | [jvm.md][jvm-heap] |
| Memory the runtime makes for the game: strings, buffers, images, screens, sound bytes | counted on the heap | `OutOfMemoryError` | [jvm.md][jvm-heap] |
| One request of the runtime | the whole heap, or twice the heap with what is counted already | `OutOfMemoryError` before anything is built | [jvm.md][jvm-heap] |
| Java threads alive at once | 64, the event and lifecycle threads among them; the host may start 16 more, 80 in all | `OutOfMemoryError` from `Thread.start` or `new Timer()` | [jvm.md][jvm-threads] |
| Stack of one thread | 131072 slots, 2 MiB | `StackOverflowError` | [jvm.md][jvm-limits] |
| Frames of one thread | 4096 | `StackOverflowError` | [jvm.md][jvm-config] |
| Calls from the runtime into the game, nested on one thread | 256 | `StackOverflowError` | [jvm.md][jvm-config] |
| Work of a thread before a safepoint | 4096 backward branches or calls; a caught exception counts as 16 | a stop or a pause takes effect, and the other threads get a turn | [jvm.md][jvm-threads] |
| Stack trace of an exception | 256 frames | the rest is left out | [jvm.md][jvm-heap] |
| Name of a thread | 255 characters | cut | [jvm.md][jvm-heap] |
| Lifecycle threads | 8 | later pause and resume requests are dropped, with one warning | [midp.md][midp-threads] |
| `callSerially` calls and posted tasks waiting | 10000 | `OutOfMemoryError` from `callSerially`; a posted task is dropped | [midp.md][midp-events] |

[jvm-heap]: docs/architecture/jvm.md#the-heap-and-the-collector
[jvm-limits]: docs/architecture/jvm.md#limits
[jvm-threads]: docs/architecture/jvm.md#threads-and-the-vm-lock
[jvm-config]: docs/architecture/jvm.md#using-a-virtual-machine-from-the-host
[midp-threads]: docs/architecture/midp.md#threads
[midp-events]: docs/architecture/midp.md#the-event-loop

### Class files and suites

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| JAR file | 64 MiB | the game does not start, and the app says why | [midp.md][midp-suites] |
| JAD file | 1 MiB | the same; a JAD found beside a JAR is left out | [midp.md][midp-suites] |
| Manifest and JAD text | the first 1 MiB, 4096 attributes | the rest is not read | [midp.md][midp-suites] |
| Entries of a JAR | 65535, and 4 MiB of names | later entries are not seen | [midp.md][midp-suites] |
| One class file | 4 MiB | `NoClassDefFoundError` | [midp.md][midp-suites] |
| Class data | 128 MiB | `OutOfMemoryError`: "class memory limit reached" | [jvm.md][jvm-classes] |
| Class name | 255 bytes | `NoClassDefFoundError`, or `ClassNotFoundException` from `Class.forName` | [jvm.md][jvm-classes] |
| Array dimensions | 255 | `ClassFormatError` | [jvm.md][jvm-classes] |
| Classes on one path up a hierarchy | 256 | `ClassFormatError` | [jvm.md][jvm-classes] |
| MIDlet class a manifest names | 255 bytes, no brackets, no control characters | that MIDlet is left out | [midp.md][midp-suites] |
| Search for MIDlet classes in a JAR without `MIDlet-1` | 8192 classes, 32 MiB inflated and 32 MiB compressed | the classes past it are not offered | [midp.md][midp-suites] |
| A resource the game reads | the room left in the heap | `OutOfMemoryError` before it is read | [midp.md][midp-suites] |
| Name of a resource | 1024 bytes or 64 segments | not found | [midp.md][midp-suites] |
| Missing classes, methods and fields reported, and missing resources apart | 256 of each, every name cut to 255 characters | no more are logged | [jvm.md][jvm-limits] |

[jvm-classes]: docs/architecture/jvm.md#class-loading-and-initialization
[midp-suites]: docs/architecture/midp.md#suites

### The log

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Log line | 4096 bytes | printed text is split into lines of that size; other lines are cut, with a note of how much went | [jvm.md][jvm-limits] |
| Log kept between two reads of the app | 10000 lines and 1 MiB | the oldest lines go first | [midp.md][midp-log] |
| Message of a crash | 1 KiB | cut | [midp.md][midp-status] |

[midp-log]: docs/architecture/midp.md#the-log
[midp-status]: docs/architecture/midp.md#lifecycle-and-status

### Images and screens

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Pixels of an image | counted on the heap, 4 bytes a pixel | `OutOfMemoryError` before the pixels exist | [graphics.md][gfx-images] |
| One image | 2^25 pixels | `OutOfMemoryError` | [graphics.md][gfx-images] |
| Size an image file declares | checked against the heap before decoding; 16384 pixels a side | `OutOfMemoryError` | [graphics.md][gfx-decoding] |
| `Image.createImage(InputStream)` | 16 MiB, or 1000 reads in a row that give no bytes | `IOException` | [graphics.md][gfx-images] |
| Repair of a damaged PNG | 64 MiB of rows | `OutOfMemoryError` | [graphics.md][gfx-decoding] |
| Extended XMP segments of a JPEG | 16 | the image does not decode | [graphics.md][gfx-decoding] |
| Nokia `drawPixels` block | 4194304 pixels | `IllegalArgumentException` | [graphics.md][gfx-nokia] |
| Glyphs kept for one font and size, for all games | 4096 above U+00FF | the cache empties and starts again | [graphics.md][gfx-text] |
| Cells of a `TiledLayer` | 2^24, and the heap | `OutOfMemoryError` | [midp.md][midp-game] |
| Animated tiles of a layer | 32767 | `OutOfMemoryError` | [midp.md][midp-game] |
| Back buffer of a shown screen | counted on the heap, 4 bytes a pixel | counted, never refused; the game's next allocation fails | [midp.md][midp-frames] |
| Surface of a `CustomItem` | the heap | the item stays blank, with one warning | [midp.md][midp-screens] |
| Text a list row draws | 4096 UTF-16 units | the row draws cut | [midp.md][midp-screens] |

[gfx-images]: docs/architecture/graphics.md#images
[gfx-decoding]: docs/architecture/graphics.md#decoding
[gfx-nokia]: docs/architecture/graphics.md#nokia-directgraphics
[gfx-text]: docs/architecture/graphics.md#text
[midp-game]: docs/architecture/midp.md#the-game-api
[midp-frames]: docs/architecture/midp.md#repaints-and-frames
[midp-screens]: docs/architecture/midp.md#the-high-level-screens

### Sound

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| One sound, decoded, in any format | 16 MiB | `MediaException`; a Nokia `Sound` stays uninitialized | [audio.md][audio-players] |
| Sound of a game, every API together | 32 MiB and 128 players | `OutOfMemoryError`, after one collection | [audio.md][audio-players] |
| Players started at once | 32 for a game, 128 for all games | `MediaException` from `start` | [audio.md][audio-players] |
| `playTone` tones playing | 16 | the oldest stops | [audio.md][audio-players] |
| Loop events of a player | one every 20 ms of sound | the loops between are not reported | [audio.md][audio-players] |
| Events of a MIDI file | 65536 | `MediaException` | [audio.md][audio-formats] |
| Length of a MIDI file that loops | 20 ms | a shorter file plays once | [audio.md][audio-formats] |
| MIDI messages sent in one render call | 512 | the rest go out in the next blocks | [audio.md][audio-formats] |
| Tones of a Nokia ringing tone | 65536 | `MediaException`; a Nokia `Sound` stays uninitialized | [audio.md][audio-formats] |
| Tones of a tone sequence | 65536 | `IllegalArgumentException` from `setSequence`; a player of such data is silent | [audio.md][audio-formats] |
| AMR file | 30000 frames, ten minutes | `MediaException` | [audio.md][audio-formats] |
| Sound read from a stream | the room left in the heap | `OutOfMemoryError` | [media.md][media-players] |
| Sound events waiting | 1024 for a game, 4096 in the engine for all games; twice that for the events that end a run | the repeats are dropped first; `CLOSED` never is | [media.md][media-events] |
| An MP3 seek the reader cannot do | 8 s of sound caught up in each render call | silence until the place is reached | [audio.md][audio-gaps] |
| Hold of the audio thread on the shared mixer | one slice of 128 frames | another game's call waits for one slice at most | [audio.md][audio-locking] |

[audio-players]: docs/architecture/audio.md#players
[audio-formats]: docs/architecture/audio.md#formats
[audio-gaps]: docs/architecture/audio.md#gaps
[audio-locking]: docs/architecture/audio.md#threads-and-locking
[media-players]: docs/architecture/media.md#players
[media-events]: docs/architecture/media.md#listeners-and-the-event-thread

### Network and links

| Limit | Number | When a game reaches it | Described in |
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
| Wait for a response head | 60 s | `InterruptedIOException` | [connections.md][net-blocking] |
| Web link a game asks to open | `http` or `https` with a host, no backslash, a port from 1 to 65535, 4096 bytes | `ConnectionNotFoundException` | [midp.md][midp-links] |

[net-hold]: docs/architecture/connections.md#what-a-game-may-hold
[net-blocking]: docs/architecture/connections.md#blocking-without-freezing-the-vm
[midp-links]: docs/architecture/midp.md#links

### Record stores

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Space of a suite's stores | 1 MiB | `RecordStoreFullException` | [rms.md][rms-quota] |
| Stores of a suite | 64 | `RecordStoreFullException` from the 65th `openRecordStore` | [rms.md][rms-quota] |
| Disk writes | a burst of 32 MiB, then 4 MiB a second; a write costs at least 4 KiB | the save waits, in slices of 50 ms; it is never refused | [rms.md][rms-budget] |
| Other suites whose stores a game opens | 16 | `RecordStoreException` | [rms.md][rms-where] |

[rms-quota]: docs/architecture/rms.md#sizes-and-the-quota
[rms-budget]: docs/architecture/rms.md#the-write-budget
[rms-where]: docs/architecture/rms.md#where-stores-live

### The apps

| Limit | Number | When a game reaches it | Described in |
| --- | --- | --- | --- |
| Name, vendor and version in the library | 100 characters | cut | [midp.md][midp-log] |
| Log a game's tile keeps | 2000 lines and 1 MiB | the oldest lines go first | [midp.md][midp-log] |
| Clicks on the prompt for a link | none for 500 ms after it appears or its link changes | the buttons stay dimmed | [midp.md][midp-links] |
| A game opened on Android | `content:` links only, 64 MiB a file | a larger file is refused and its partial copy deleted; other links are ignored | [playing.md][playing-adding] |

[playing-adding]: docs/playing.md#adding-games
