# Benchmark

This page compares FastEmu with MicroEmulator 2.0.4 on one Windows
computer. It measures how soon a game shows its first frame, how much
memory and processor time a game takes, many games at once, and how fast
each runs game code. It also says plainly where MicroEmulator is faster.

The numbers come from `cargo xtask bench`, a tool in FastEmu's
repository. It runs the same JARs on both emulators the same way, with no
window and no person at the keyboard. Anyone with a Windows computer can
run it again: see [Run it yourself](#run-it-yourself).

## Results

The results are not in yet. The final run waits for the last changes of
0.1.0 to the sound mixer, the timers and the way a game stops, since they
change what FastEmu costs. This section will then hold the date, the
FastEmu commit, the machine, the versions and the tables of `bench.md`.

## Where MicroEmulator is faster

MicroEmulator is a Java program. It runs a game's code on the Java
virtual machine you installed, and that machine compiles the code to
machine code as it runs. FastEmu has a virtual machine of its own that
interprets the code, one instruction at a time. So in long loops of
arithmetic, method calls and array work, MicroEmulator runs game code
many times faster. The kernel tables in the results show by how much.

A game that draws a few sprites a frame and then sleeps until the next
one spends little time in such loops. For it, the costs that show are
starting up, memory, and the processor time a frame takes. The results
compare those too, with real games.

The Java virtual machine's speed also uses more of your computer: its
compiler and its garbage collector run on other cores beside the game.
The bench counts the processor time of every thread, so that cost shows.

## How it was measured

### The two emulators

- **FastEmu** runs in `xtask bench-run`, a headless runner built from
  the same crates, with the same release settings, as the app. It starts
  games with the app's launcher and the app's sound engine, which mixes
  into a silent output. Games keep the app's frame cap of 60 frames a
  second, as a player gets them. The kernel run lifts the cap.
- **MicroEmulator 2.0.4**, the last release, from 2010, runs headless
  through MeBench, a small wrapper in FastEmu's repository. MeBench sets
  MicroEmulator up as its own headless runner does, and adds a frame
  count, key presses, a screenshot and an end. It opens no window. The
  command is:

  ```text
  java -Djava.awt.headless=true -Duser.home=OUT\me-home\NAME
    -cp ME\microemulator.jar;ME\lib\microemu-jsr-135.jar;ME\lib\microemu-nokiaui.jar;ME\lib\microemu-siemensapi.jar;OUT\mebench-real
    fastemu.bench.MeBench --seconds S --events KEYS --resizableDevice 240 320 GAME.jar
  ```

  `microemulator.jar` alone lacks the Nokia classes that some games
  need, since its manifest names no other jar. Even on the class path,
  MicroEmulator's strict class loading for a JAR does not reach them, so
  BombMan, built on Nokia's `FullCanvas`, ended before its first frame.
  MeBench asks for relaxed class loading, as MicroEmulator uses for a
  MIDlet started by its class name. MeBench keeps saves in
  files, as FastEmu does, in a home folder of its own for each process.
  MicroEmulator's own headless runner keeps them in memory.
- MicroEmulator's development snapshot on GitHub was not measured, only
  the 2.0.4 release.

### What runs

- **The benchmark MIDlet**, made for this. It uses only CLDC 1.1 and
  MIDP 2.0. Its kernels time themselves on the MIDlet's own clock. Each
  kernel runs 5 rounds, and each round repeats it until half a second
  passed. Warm is the median time of one repetition in rounds 2 to 5;
  cold is the very first repetition, before any compiler warmed up.
  - CPU: integer arithmetic, method calls, a sieve, a heap sort, objects
    with virtual calls, 64-bit math, float math, strings, and
    `Hashtable` with `Vector`.
  - Memory: small objects, arrays of mixed sizes, and one full
    collection with a live set to trace.
  - Drawing into a 240x320 image: rectangles, lines, opaque and alpha
    images, rotated regions, text, `drawRGB`, and the scene below.
  - Every CPU and memory kernel prints a check value. Both emulators
    must print the same one, or the report marks it `differs`.
  - An unpaced frame loop: repaint as fast as the emulator lets.
  - A scene of 48 sprites and a line of text at exactly 30 frames a
    second, under FastEmu's cap, so both draw the same frames.
- **Real games**, each for 45 s: Block Fall, BombMan, Shooter, Skocko,
  Star Defender, Stick Fight, Tower Guard and Traffic Dash. They are free
  games; the [compatibility page](/docs/compatibility/#tested-games)
  lists them with their licenses.
- **All the games at once** for 60 s: FastEmu runs them in one process,
  as its window does; MicroEmulator runs one process for each.
- **A small-heap setting** of MicroEmulator, `-Xms16m -Xmx128m
  -XX:+UseSerialGC`, for the scene and all games at once. Smaller
  settings exist, so this is one setting, not MicroEmulator's best case.
- **With windows**, only when asked: the FastEmu app with every game,
  against one MicroEmulator window for each game. Nobody presses keys,
  so the games stay on their title screens.

### Where each number comes from

| Number | Source | How |
| --- | --- | --- |
| Kernel times | The MIDlet's clock | Milliseconds per repetition, from `System.currentTimeMillis` |
| Start to first frame | The bench's clock | From starting the process to reading its first-frame line, with no window |
| Frames a second | Each emulator's own count | Frames between the two samples |
| Processor time | Windows, per process | Cycles from `QueryProcessCycleTime`, at the rate of a busy thread |
| Memory | Windows, per process | Peak working set, working set and private bytes, at the second sample |
| Other load | Windows, whole system | `GetSystemTimes`, minus the measured processes' `GetProcessTimes` |

The bench reads processor time as cycles because Windows counts its
usual CPU times by the clock tick, and misses most of the short bursts
that game threads run in.

The other load is the one exception. `GetSystemTimes` counts the whole
system's busy time by the clock tick. So the bench takes away the time
of the measured processes, the sampler and the audio engine as Windows
counts it the same way, with `GetProcessTimes`. Their cycles would not
do. In one kernel run, FastEmu ran 984 ms a second by its cycles but
only 637 by the tick, so taking its cycles away hid about 350 ms a
second of other load.

The tick also misses part of a busy thread's time when it moves between
processors, and most of a load that runs in short bursts. On the
computer it was tried on, a busy loop added about 1000 ms a second of
other load when held on one processor. When Windows moved it between
processors, it added from about 250 to 700, on different runs. Each is
over the margin below. But eight threads that each ran 1 ms in every
10 ms used about 510 ms a second by their cycles, and under 100 by the
tick. The flag can miss a load like that.

Start to first frame has no window in it. The app's window and its
graphics start are not counted.

Working set is the memory in RAM. Private bytes is what a process
reserved for itself. A Java virtual machine reserves a large heap at
start on a computer with much RAM, so its private bytes look large
while its working set stays modest. The results give both.

### Fair rules

- Both emulators run the same JAR on a 240x320 screen with Nokia's key
  codes. A full-screen game draws on 240x320 on both. A screen that is
  not full screen is 176x176 on MicroEmulator, so the report shows the
  size each game drew on, saves a picture of each, and leaves a game
  out of the ratios when its screen or its state differs.
- Both get the same keys at the same times after the game's first
  frame: the arrows and fire, never the digits, so the game actions are
  the same too. MicroEmulator gives 2, 4, 5, 6 and 8 no game action.
- FastEmu holds a game to 60 frames a second; MicroEmulator does not.
  So the report gives the frame rate beside every processor time, and
  the processor time of one frame. It compares processor times only
  when the two frame rates are within 5 percent. An optional row runs
  MicroEmulator capped as well (`--me-max-fps`).
- Both emulators make sound. FastEmu mixes into a silent output.
  MicroEmulator plays on the speakers, which are muted for the run. The
  Windows audio engine (`audiodg.exe`) is sampled too and shown beside
  the rows.
- MicroEmulator runs with the JVM's defaults, as `java -jar` gives a
  player. The cold column shows the compiler warming up.
- The two clocks differ. FastEmu's is precise; Java's can step 16 ms on
  Windows. The MIDlet measures its clock's step, and the report marks a
  time under 20 steps as within one step.
- Error counts are not compared: MicroEmulator counts every line a game
  writes to `System.err`, FastEmu counts uncaught exceptions.

### Runs and a quiet machine

Each measurement runs 4 times. In each pair, FastEmu and MicroEmulator
take turns going first, so each goes first equally often. Before each
measurement the bench waits until the computer is quiet: other load
under the idle baseline, measured for 20 s at the start, plus 100 ms of
processor time a second. It waits a minute at most and records the
load. Each number is the median of the runs, with the smallest and the
largest. A measurement is flagged as noisy when the other load during
it passed that threshold, and a number when its runs spread more than
15 percent from the median. `bench.md` gives the other load during the
measurements and just before them. An idle baseline over 1000 ms a
second, one processor's time, is flagged too: the computer was busy
before the bench began, and the flag cannot see load on top of what was
already there.

For the run, the computer is on mains power, in its best performance
mode, with browsers, editors, chat, OneDrive and Windows Update paused,
and the speakers muted.

## Run it yourself

You need Windows, a JDK 17 (any Java 8 or newer runs MicroEmulator),
Rust, and about 90 minutes. In PowerShell, download MicroEmulator 2.0.4
and check it:

```powershell
New-Item -ItemType Directory -Force C:\bench\microemu | Out-Null
curl.exe -L -o C:\bench\microemu\microemulator-2.0.4.zip https://downloads.sourceforge.net/project/microemulator/microemulator/2.0.4/microemulator-2.0.4.zip
(Get-FileHash C:\bench\microemu\microemulator-2.0.4.zip).Hash
# 8BEA86083AB1F6AADA2BF22FD2ACBC4F16921E77F60B0F4660871CC371272A75
Expand-Archive C:\bench\microemu\microemulator-2.0.4.zip C:\bench\microemu
```

Put the JARs of the games to run in `C:\bench\games`. Then build the
release tools and run the bench from the repository:

```powershell
cargo build --release --locked -p xtask -p fastemu
$x = '.\target\release\xtask.exe'
$me = 'C:\bench\microemu\microemulator-2.0.4'
& $x bench --build-only --microemu $me --out C:\bench\out
& $x bench --microemu $me --games C:\bench\games --out C:\bench\out
```

The second run takes about 20 minutes a run, 4 runs, plus the quiet
waits. It writes `bench.md` and `bench.json` to the output folder, with
a screenshot of each game on each emulator. Read the problems at the
end of `bench.md` before you trust a number.

The output folder must be new, empty, or one the bench wrote before.
The bench clears folders such as `data` and `logs` in it, so it refuses
a folder that holds other files.

- `--fastemu-only` measures FastEmu alone, with no MicroEmulator.
- `--runs N` changes the number of runs.
- If MicroEmulator rejects a class with `VerifyError`, add
  `--me-jvm-arg -XX:+UnlockDiagnosticVMOptions --me-jvm-arg
  -XX:-BytecodeVerificationRemote`; JDK 17 refuses the second without
  the first. If it cannot load images without a display, add
  `--me-no-headless-awt`; it still opens no window. Each applies to
  every MicroEmulator process of the run, and `bench.md` lists it.
- `--windows --app .\target\release\fastemu.exe` measures the app and
  MicroEmulator's own windows instead. It opens windows, so do not touch
  the mouse while it runs. It refuses to start while FastEmu is open,
  since it adds the games to the app's library. It backs up
  `library.json` and `settings.json` and puts them back after. If it is
  stopped part way, the backup stays in `app-config-backup` in the
  output folder, and the next run says how to put it back.

**MicroEmulator runs a game as an ordinary Java program, with all your
rights on the computer: it has no sandbox.** Run only games you trust in
it. FastEmu runs games in a virtual machine of its own, which reaches
only what FastEmu gives it.

## Not measured

- J2ME Loader, which runs only on Android. How FastEmu compares with it
  stays unmeasured.
- macOS and Linux.
- A cold disk cache: the bench runs each emulator once first, so files
  are in memory.
- Time on the graphics card.
