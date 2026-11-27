# Changelog

All notable changes to FastEmu are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-11-28

### Added

- The README and the website show FastEmu playing games: the window with
  six games at once on a macOS desktop, beside the Android app (#82).
- A setup for Windows: `fastemu-<version>-windows-x64-setup.exe` installs
  FastEmu for your account without administrator rights. It adds a Start
  menu shortcut, a desktop shortcut when you tick its box, and FastEmu
  under Open with for `.jar` and `.jad` files, and Settings, Apps
  uninstalls it. The zip is now `fastemu-<version>-windows-x64-portable.zip`
  (#84).
- A disk image for macOS: open `fastemu-<version>-macos-universal.dmg`
  and drag FastEmu onto Applications. The zip with the same app stays
  (#85).
- More ways to install on Linux, for x64 and Arm64 alike: an AppImage
  that runs on most distributions, an `.rpm` for Fedora and other RPM
  systems, and a `.deb` for Arm64 beside the x64 one (#86).
- A Flatpak for any Linux, for x64 and Arm64: install
  `fastemu-<version>-linux-x64.flatpak` with `flatpak install` or your
  software center. In its sandbox FastEmu sees the display, sound and the
  network, and only the files you pick (#87).
- One line installs FastEmu from a terminal, checked against the
  release's `SHA256SUMS`: `curl -fsSL .../install.sh | sh` on Linux and
  macOS, and `irm .../install.ps1 | iex` in PowerShell on Windows. Both
  take a version to install and uninstall again (#88).
- FastEmu in Homebrew and Scoop: `brew install --cask fzn0x/tap/fastemu`
  on macOS, and `scoop install fastemu` from the fzn0x bucket on Windows
  (#89).

### Fixed

- On Windows, opening FastEmu from Explorer, a shortcut or the Start menu
  no longer opens a console window beside it. Typed in Command Prompt or
  PowerShell, `fastemu` still prints into the terminal and returns the
  exit code, through `fastemu.com`, the console program beside
  `fastemu.exe` (#83).

## [0.1.0] - 2026-11-27

### Added

- Games can read and write text in the encodings of Japan, China, Taiwan
  and Korea (Shift_JIS, EUC-JP, ISO-2022-JP, GBK and GB2312, GB18030,
  Big5, EUC-KR), in UTF-16, and in the ISO-8859 code pages for Greek,
  Hebrew, Arabic, Cyrillic, Turkish, Baltic and other European
  languages, under the names Java accepts for them, so their menus and
  dialogue show up instead of question marks or a startup error (#2).
- Games can use the whole CLDC 1.1 class library: lists and tables,
  random numbers that match a phone's for the same seed, dates and
  calendars in the computer's time zone, timers, and reading and writing
  saved data and text in the game's encoding. A timer task that fails
  stops only that task, so the rest of the game keeps running (#2).
- FastEmu opens MIDlet suites from a .jar or a .jad, including JARs with a
  damaged central directory, manifests with a byte order mark or wrapped
  lines, and JADs whose JAR sits elsewhere, and runs each MIDlet's
  lifecycle: start, pause, resume, stop and its own exit (#3).
- Games draw on their canvas with shapes, text and images, get keys with
  repeats and touch events, and show their commands on the soft keys or
  in an options menu (#5).
- Games made with MIDP 2.0's game API now run: game canvases read their
  keys and draw off screen, and sprites, tiled backgrounds and layer
  managers draw, animate and collide as they did on phones (#6).
- Games keep their saves, settings and high scores between runs. Each
  instance keeps its own saves, a save survives the emulator being closed
  or killed mid-write, and a damaged save file still opens with whatever
  could be read (#7).
- The desktop window runs real games, with sound through the system's
  audio device (silent, and still running, when there is none). Games
  named on the command line are added to the library and started, and
  `fastemu --mock` shows test patterns instead, for work on the window
  itself (#8).
- Typing switches to text on its own while a game shows a text box, and
  back to the keypad when it closes; the text button on a game's title
  bar still switches by hand (#8).
- A game that cannot start gets its tile all the same, with the reason
  and a button to try again, and a crash shows its message in the
  game's tile (#8).
- A game's settings have a touch screen switch, which gives the phone a
  touch screen whatever its profile, so a touch game made for a 320x240
  phone, such as Pixelus, can be played (#8).
- `fastemu run` runs a game without a window for a number of frames or
  seconds, presses keys from a script, prints the game's log and saves a
  screenshot, for testing games in scripts and CI (#9).
- The key script of `fastemu run` can touch the screen: `tap@XxY`,
  `press@XxY`, `drag@XxY` and `release@XxY` play touch games such as
  Lines and Pixelus without a window, and `--touch` gives any phone
  profile a touch screen (#9).
- `fastemu run --data DIR` keeps the game's saves in DIR, so a later run
  continues from them (#9).
- `fastemu run --record DIR --every N` saves every Nth frame the game
  draws as a numbered PNG, for videos and pictures of real play (#9).
- The Android app runs real games, several at once, with sound, in the
  same library, grid and focus layouts, keypad and settings as the
  desktop window. Picked games are copied into a folder of their own, so
  a newer file with the same name never replaces a running game's JAR
  (#10).
- On a touch screen, a finger holds an on-screen keypad key for as long
  as it stays down, and touching a game's screen sends the game presses,
  drags and releases (#10).
- Games pause when the app goes to the background and the ones it
  paused resume when it comes back; a game you paused stays paused (#10).
- On Android, a game's text box opens a dialog with the system keyboard,
  and what you type there goes into the game (#10).
- Games play sound: tones and tone sequences, WAV (PCM and ADPCM), MIDI
  through a built-in synthesizer, MP3 and AMR, through MMAPI players and
  Nokia's Sound API. Each game has its own volume and mute, and a format
  FastEmu cannot play stays silent instead of stopping the game (#13).
- Games show their menus, forms, text boxes and alerts: lists, text
  fields, choice groups, gauges, date fields, scrolling tickers and the
  items a game draws itself, in a phone's look at any screen size. Text
  boxes take typing from the keyboard or multi-tap on the keypad, and the
  app knows when to send typed text (#14).
- Samsung 176x220 and LG 176x220 phone profiles, so games made for
  those phones find Samsung's sound clips, backlight and vibration, or
  LG's soft key codes and backlight (#15).
- Games written for Nokia phones run: full-screen `FullCanvas` games get
  their soft keys, and Nokia's `DirectGraphics` draws and reads pixels in
  every Nokia format, with flips, rotations and translucent colours (#15).
- Games made for Siemens phones run on the Siemens profile: its game and
  colour game packages draw sprites, tiled backgrounds and layers, play
  melodies, and give pixel access to black and white pictures (#15).
- Samsung sound clips play on Samsung profiles, and every maker's light
  and vibration requests show in the game's log. Text messages are never
  sent (#15).
- Each phone profile offers only its own maker's extra classes, so games
  that look for them pick the right phone (#15).
- Games can reach the network when you allow it for them: web requests
  over HTTP and HTTPS, sockets and datagrams. With the network off, the
  default, they see no signal and carry on. Games that try to send an SMS
  are told the send failed (#16).
- A game can ask to open a web page, such as more games from its maker.
  Its screen then shows the address with Open and Dismiss, and nothing
  opens until you choose Open. Calls, messages, files and installs that
  a game asks for are refused (#16).
- The README opens with a picture of six free games running in FastEmu,
  recorded with its headless runner (#18).
- The compatibility page lists the free games FastEmu was run with, what
  each one does today and what it still needs (#19).
- The guides have a benchmark page that compares FastEmu with
  MicroEmulator 2.0.4 on one Windows computer: how soon a game shows its
  first frame without a window, how much memory and processor time a
  game takes, many games at once, and where MicroEmulator is faster,
  which is running game code through Java's compiler (#56).

### Fixed

- On Windows, games that pause between frames now run at full speed. A
  game's 10 ms pause took about 16 ms, which held simple game loops near
  52 frames a second; pauses and timed waits now end within about a
  millisecond of their time on every platform (#1).
- A game that builds its own list, stack, table or print stream on the
  library's (changing how adding, removing, counting or writing works)
  now has its changes used by the library's other methods too, as on a
  phone, instead of having some of them skipped (#2).
- A game that runs on a repeating timer no longer races through the
  ticks it missed when it is resumed after a pause (#2).
- A key you hold when you pause a game is let go in the game, instead
  of staying down after you resume it (#8).
- Holding a phone key on the keyboard and on the on-screen keypad at
  once no longer lets it go when only one of them is released (#8).
- A click or touch held on a game's screen is let go when another game
  or another window takes the focus, instead of staying down in the
  game (#8).
- The log panel keeps up with a game's new lines while it shows the
  last one, also for a paused or stopped game, and stays put while you
  read older lines (#8).
- The window stays smooth when you press keys or tap while many games
  run on a busy computer. Each key press waited until the game it went
  to let go of its machine: with eight games and every core busy, up to
  74 ms, longer than four frames. Now it never waits for a game and
  returns within a few milliseconds (#8).
- `fastemu run --frames N` counts frames of the screen, 60 a second,
  instead of pictures the game draws, so a game that redraws rarely,
  such as Tetris with one picture every 800 ms, no longer keeps the
  runner going for minutes. A still screen, such as a game-over screen,
  no longer fails the run after 30 seconds; only a game that shows
  nothing at all does (#9).
- The README, the guides and the website say what FastEmu does today:
  touch games play on any phone profile with the touch screen switch,
  text messages are never sent, Samsung and LG classes wait for phone
  profiles of their own, and the FAQ says where saves are, why a game
  may ignore taps and whether 3D games run (#18).
- The contributing guide lists the test tools (the conformance MIDlets
  and `cargo xtask stubs`, `coverage` and `compat`) and asks for a JDK
  9 or newer, which the tests need (#18).
- Games that make new pictures all the time, such as Shooter, no longer
  take more and more memory while they run. Shooter grew by about 5 MB
  a second, so eight games side by side passed 400 MB in a minute; now
  they stay near 100 MB (#19).
- The compatibility report no longer calls a game hung when it waits for
  a key on a still screen, such as an About page. A game whose screen
  stays still counts as running when it still answers and none of its
  threads is busy; a stuck game stays hung, and the report says why
  (#48).
- A game that starts a new timer for each screen, level or effect, and
  lets the old ones go without stopping them, no longer fails with
  OutOfMemoryError after about 62 of them. The timers it has let go of
  give their threads back once they have nothing left to run (#49).
- A game that ends, by exiting or by crashing, gives its memory back
  once its threads have ended, while its tile still shows its last
  screen, its log and how it ended. Before, it held that memory until
  you closed the tile: over 30 MB for a game that filled an 8 MB phone
  with small objects, and closing that tile held up the window for 10
  to 20 ms (#51).
- Changing a running game's volume, muting it or changing its settings
  no longer makes the window wait for the sound. While games played
  many MIDI sounds at once, each such click could hold the window for up
  to about 10 ms for every running copy of the game. Now it never waits,
  and the new volume still slides in without a click (#52).

### Security

- A .jad file can no longer make FastEmu open a file outside its own
  folder or a network share through MIDlet-Jar-URL, and a JAR over 64 MiB
  or a JAD over 1 MiB is refused with a clear message (#24).
- A game or a damaged file with a very deep class name or a very long
  chain of classes or interfaces no longer closes the whole app; it gets
  a Java error (#25).
- A game or a damaged file with many layers of interfaces no longer
  freezes its window while it looks up a field (#25).
- A game that nests lists or tables very deeply, and then prints or
  joins them into text, no longer closes the whole app and every other
  game with it. The game gets an error it can handle, as on a phone
  (#26).
- A game stuck in a long loop inside the runtime, such as skipping
  through a stream that never ends, no longer stays open until you quit
  the app: closing its tile ends it within a few seconds (#26).
- A game that fills memory with long strings, text buffers, exceptions
  or resource streams now gets OutOfMemoryError, as on a phone, instead
  of using up the computer's memory (#27).
- A stack trace now shows at most 256 frames and a thread's name at most
  255 characters, so a game cannot make either use up the computer's
  memory (#27).
- A game that builds a huge array of arrays now gets OutOfMemoryError
  at once, as on a phone, instead of building a million arrays first
  (#28).
- A game stuck in an endless loop of throwing and catching exceptions
  can now be closed: closing its tile ends it, where before it kept
  running until you quit the app (#28).
- A game that recurses very deeply, or whose methods ask for huge
  frames, now gets StackOverflowError instead of using gigabytes of
  memory (#28).
- A damaged or crafted game class that returns the wrong kind of value
  from a method can no longer use up the computer's memory in a loop.
  The game gets a VerifyError the first time it calls that method (#29).
- A game thread that ends by throwing something that is not an exception
  no longer leaves its game half stopped: the thread ends, its locks are
  released and the game is shown as crashed (#29).
- A damaged or crafted game class that calls a method on an object of
  another class no longer stops its game with an internal error; it gets
  an IncompatibleClassChangeError, as on a phone (#29).
- A game that asks for thousands of `callSerially` calls while its
  screen is busy gets an OutOfMemoryError once 10000 are waiting, and
  the other work a game can queue for the screen thread is limited too
  (#30).
- Control characters in a game's text are escaped in the log and in the
  crash message, so a game can no longer clear your terminal, change its
  title or fake a line of FastEmu's when you run it on the command line
  (#30).
- A game's log lines are cut at 4096 bytes and its log keeps at most
  1 MiB of text, so a game that logs huge lines can no longer fill the
  computer's memory (#30).
- A game that keeps asking for classes, resources or other suites'
  record stores that are not there no longer fills the computer's
  memory. FastEmu reports the first 256 different missing classes,
  methods and fields, and the first 256 missing resources and class
  files over 4 MiB, then says that it stops. A game can open the record
  stores of 16 other suites at most (#30).
- A game that prints text without ever ending a line no longer fills the
  computer's memory: what it prints comes out in lines of at most 4096
  bytes (#30).
- A game can no longer use up the computer's memory with one Nokia
  drawPixels call over 4194304 pixels, or by asking for every
  character of every font: the font cache is now limited. A call over
  the limit throws IllegalArgumentException (#31).
- A JPEG padded with thousands of metadata segments no longer keeps a
  game busy for seconds or minutes in one image call. The game gets an
  image error instead (#31).
- A game whose icon file is made to crash the PNG reader no longer closes
  FastEmu, and every game running in it, when the library scans it. The
  game shows no icon instead (#31).
- A PNG with an impossible bit depth or a broken palette no longer makes
  a game stop or ask the computer for gigabytes. The game gets an image
  error it can handle, as on a phone (#31).
- A game that gives Image.createImage a stream that never ends now gets
  an IOException after 16 MiB, instead of filling the computer's memory
  (#32).
- A game that makes too many or too large images, tile layers or sprites
  now gets OutOfMemoryError, as on a phone, instead of using up the
  computer's memory. A small image file that claims a huge size no longer
  makes the app reserve that much memory before the game sees an error
  (#32).
- A tile layer holds at most 32767 animated tiles; a game that asks for
  more gets OutOfMemoryError, where before it could grow without a limit
  (#32).
- A game that shows many screens, or makes custom items of a huge size,
  now gets OutOfMemoryError once they fill its memory, as on a phone,
  instead of using up the computer's memory. A menu or a form that is no
  longer shown gives its screen buffer back (#32).
- A game that adds elements to a list or choice group without end, or
  draws a long list whose elements share one huge text, now gets
  `OutOfMemoryError` at its heap limit instead of using up the
  computer's memory (#33).
- A Siemens game that adds the same graphic object to a manager without
  end, or makes sprites and backgrounds in a loop, now gets
  `OutOfMemoryError` at its heap limit instead of using up the
  computer's memory, and a manager keeps an object once (#33).
- A game that makes many text boxes or fields from one large text, or
  keeps inserting text into a box with a huge maximum size, now gets
  `OutOfMemoryError` at its heap limit instead of using up the
  computer's memory, and inserting text no longer copies the whole text
  each time (#33).
- A sound file made to be tiny and to decode to something huge, such as
  an AMR file of empty frames, a Nokia ringing tone that plays its
  patterns over and over, or a sound stream that never ends, now fails
  at once instead of taking gigabytes of memory and minutes of decoding
  (#34).
- A game that makes too many or too long sounds now gets an error
  instead of using up the computer's memory. It can hold 128 players and
  32 MiB of decoded sound at once, and no one sound can decode to more
  than 16 MiB. Nokia, Samsung and Siemens sounds count the same (#34).
- A damaged JAR with one class that names the same long text, the same
  long method descriptor or the same interface thousands of times no
  longer makes the app use gigabytes of memory while the class loads
  (#35).
- A game or a damaged JAR that loads thousands of classes with huge
  tables, or asks for thousands of array classes, now gets an
  OutOfMemoryError it can handle, instead of using gigabytes of memory
  and slowing the whole computer (#35).
- A game that pauses and resumes itself again and again from a start that
  never returns no longer piles up threads without end; the later
  requests are dropped, and the game still gets its closing call when you
  stop it (#36).
- A game can no longer have an unlimited number of threads or timers at
  once. Past 64 threads, the app's own for the game among them, it gets
  OutOfMemoryError, as on a phone, instead of using up the computer's
  threads and leaving none for the app and the other games (#36).
- A game that names a thread with a zero character no longer stops itself
  with an internal error (#36).
- One game's sound can no longer make the sound of other games stall: a
  game that seeks a big MIDI file or starts and stops sounds in a tight
  loop used to keep the shared mixer for itself, and every other game's
  sound calls waited for it (#37).
- A MIDI file with more than 65536 events is refused with a
  MediaException, and one shorter than 20 ms plays once even when the
  game loops it, so a crafted file can no longer keep the audio thread
  busy for minutes (#37).
- Seeking in an MP3 with a damaged header no longer holds up the sound
  of every game while the file is decoded up to the new position. The
  sound arrives a moment later instead (#37).
- A game whose sound listener is slow, and that loops a sound of a few
  frames or flips a volume thousands of times a second, can no longer
  fill the computer's memory with sound events. Listeners hear of a
  loop at most every 20 ms, and the events that wait for a listener are
  limited (#37).
- A sound that makes a decoder or the synthesizer fail no longer ends
  the audio for every game. That one sound stops with an error event
  and the others play on (#37).
- A game can have 32 sounds playing at once, and all games together 128.
  Starting one more throws a MediaException, the exception for a player
  that cannot start, until another one stops or ends (#37).
- A game can hold at most 64 open connections, 8 server sockets and 16
  datagram sockets, so one game can no longer use up the computer's file
  handles (#38).
- Network and record store errors reach a game with fixed messages, so
  the system's own text, which names the computer, stays out of the game
  (#38).
- A server that sends interim responses, endless trailers, or a response
  or a secure handshake one byte at a time can no longer keep a closed
  game running (#38).
- A game with network access can no longer fill the computer's memory
  with a request body, request headers or connection names. They are
  limited to 4 MiB, 16 KiB and 8 KiB, and all the bodies of a game
  together to 8 MiB (#38).
- A JAR whose entries all point at the same compressed data, or a damaged
  JAR made of thousands of file headers, no longer keeps FastEmu busy for
  minutes or uses large amounts of memory when it is added or started.
  FastEmu looks for the MIDlets of a suite that lists none within 32 MiB
  of inflated data and 32 MiB of the data the entries hold (#39).
- A damaged JAR with a class that names one interface thousands of times,
  or lists thousands of fields of one very long type, no longer makes
  FastEmu use gigabytes of memory or keeps it busy for seconds when the
  JAR is added or the class loads (#39).
- A class file of over 4 MiB in a JAR is no longer loaded, and the log
  says so. A damaged JAR can no longer make FastEmu build hundreds of
  megabytes while it reads one class (#39).
- A JAR or JAD with a huge or crafted manifest no longer keeps FastEmu
  busy for minutes, or uses hundreds of megabytes of memory, when it is
  added or started. FastEmu reads the first 1 MiB of the file and its
  first 4096 attributes (#39).
- A damaged JAR with a class of thousands of methods that all take
  thousands of parameters no longer keeps FastEmu busy for minutes while
  the class loads. It is refused in milliseconds (#39).
- A game that asks for a resource with a very long name, or one with
  more than 64 parts, gets the usual not found at once, instead of
  keeping its thread busy for minutes (#39).
- A game that reads a resource that is far bigger than its memory, or
  that several threads read at once, no longer makes FastEmu build the
  whole file first. The game gets OutOfMemoryError, as before (#39).
- A suite whose name, vendor or version is very long is shown cut at 100
  characters in the library and on its tile. Its description is cut at
  500. A MIDlet class name with brackets or control characters, or over
  255 bytes, is not accepted, whether the manifest gives it or a class
  file does, and the suite says it lists no MIDlet (#39).
- A game can have at most 64 record stores. The 65th gives
  RecordStoreFullException, as a full store does, so a game can no
  longer fill a folder with tens of thousands of record store files
  (#40).
- A game that saves its record stores in an endless loop is held to a
  burst of 32 MiB and then 4 MiB of disk writes a second, so it can no
  longer wear out the disk or slow the other games. Saves stay
  synchronous and durable, and a game that saves normally never waits
  (#40).
- On Android, "Open with FastEmu" takes only content links, not `file:`
  links from other apps, and copies at most 64 MiB of a file instead of
  filling the storage with a file that never ends (#41).
- Games added after `library.json` was lost or damaged no longer get the
  saves of an older game: a new game always gets a number above every
  saves folder (#41).
- A suite's name, vendor and version are kept in the library cut at 100
  characters, also when an older library file holds longer ones (#41).
- Web links with a backslash, such as `http://good.com\.evil.com/`, or
  with a port outside 1 to 65535 are no longer offered by `platformRequest`
  (#41).
- The question that asks before a game opens a web page ignores clicks
  for half a second after the game asks, so a quick tap in the game
  cannot answer it before you have read the link (#41).
- A game file that crashes the code that reads its details, in the
  library or when the game starts, now shows as broken or not started
  with a fixed message, instead of closing the whole app (#41).
- `fastemu run` and the desktop app's terminal messages escape control
  characters in a game's name, version, file name and errors, so a game
  can no longer rewrite your terminal through them (#41).
- SECURITY.md now lists the limits a game cannot pass, with their
  numbers, and the known limits that remain, so you can tell a known
  limit from a new problem before you report it (#42).
- The settings guide says that a game you allow on the network can also
  reach programs on your computer and devices on your local network
  (#42).
- A game can no longer make FastEmu hold many times its own memory while
  it loads pictures and sounds. Pictures and sounds read from a stream
  count against the game's memory as they are read, a game's threads
  load pictures one at a time, and a GIF whose first frame needs more
  memory than the game has is refused (#44).
- A game that pauses itself and asks to be resumed in a tight loop no
  longer fills the computer's memory, and closing it stops it at once.
  A game you paused also stays paused until you resume it, even if it
  asks to be resumed (#45).
- A game with network access can no longer leave a thread and a socket
  behind for every connection it gives up on. It has at most 16 name
  lookups and connects under way at once, and a connect it gave up on
  stops trying within 30 s (#46).
- A damaged JAR with a long chain of large classes no longer makes the
  app hold hundreds of megabytes while they load. The 128 MiB a game's
  classes may take now holds for classes that wait for others to load,
  and for long line number tables and source file names (#47).
- A game that makes very deep calls no longer makes the app reserve
  twice the memory its thread's stack may use (#47).
- One game that plays dozens of sounds at once no longer keeps the sound
  of every other game breaking up. The games share the time the computer
  has for sound: a game over its part keeps its oldest sounds playing,
  and the sounds it started last pause until there is room (#50).
