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

These numbers come from one run of the bench, 4 runs of each
measurement, with no window.

| What | Value |
| --- | --- |
| Date | 2026-11-26, from 21:43 to 23:05 UTC |
| FastEmu | main at commit `e945f8b`, the release build, with the sound engine on a silent output |
| MicroEmulator | 2.0.4, headless through MeBench, relaxed class loading, the JVM's default options |
| Java | Java SE 21.0.2, HotSpot 64-Bit Server VM |
| Processor | Intel Core i7-12700F, 12 cores, 20 logical processors |
| Memory | 31.8 GB |
| Windows | Windows 11 Pro, build 22621, the Balanced power plan |
| Load | idle baseline 353 ms a second; other load during the measurements 307 (247 to 574) |

Each number is the median of the 4 runs, with the smallest and the
largest in brackets. Times are in milliseconds and memory in MiB.

### In short

- **Start.** FastEmu showed a game's first frame 2.5 to 7.2 times
  sooner: 28 to 88 ms against 198 to 223 ms in the six games compared.
- **Memory.** In the six games compared, one game took 38 to 50 MiB of
  RAM in FastEmu and 66 to 190 MiB in MicroEmulator, 1.7 to 4.6 times
  more. All eight games at once took 87 MiB in
  FastEmu and 803 MiB in MicroEmulator's eight processes, or 671 MiB
  with the small heap.
- **Processor time in real games.** At the same frame rate, FastEmu used
  1.6 to 5.8 times less processor time in four games. In Stick Fight it
  used 1.1 times more. All eight at once used 63.7 ms a second in
  FastEmu and 190.7 in MicroEmulator.
- **Game code.** MicroEmulator ran the CPU kernels 5.5 to 131 times
  faster once its compiler warmed up, and 4.0 to 54.8 times faster on
  the very first repetition. See
  [Where MicroEmulator is faster](#where-microemulator-is-faster).
- **Drawing.** Mixed: FastEmu was faster in five of the eight drawing
  kernels, MicroEmulator in two, and one was about the same.

### Headline

| Measure | FastEmu | MicroEmulator | Ratio |
| --- | --- | --- | --- |
| Start to first frame, scene, ms | 46 (27 to 308) | 207 (196 to 329) | FastEmu 4.6x faster |
| Real games, start to first frame | see below | see below | FastEmu 2.5x to 7.2x faster in 6 |
| Real games, CPU at the same frame rate | see below | see below | FastEmu 1.6x to 5.8x less in 4; MicroEmulator 1.1x less in 1 |
| Real games, peak working set | see below | see below | FastEmu 1.7x to 4.6x less in 6 |
| Scene at 30 fps, CPU ms a second | 7.2 (6.7 to 7.7) | 12.0 (11.7 to 12.8) | FastEmu 1.7x less |
| Scene, peak working set, MiB | 23.8 (23.8 to 23.9) | 76.7 (76.4 to 76.8) | FastEmu 3.2x less |
| Scene, private bytes, MiB | 18.3 (18.1 to 18.4) | 626.4 (626.2 to 626.5) | FastEmu 34.2x less |
| Scene, peak working set, MiB, MicroEmulator small heap | 23.8 (23.8 to 23.9) | 72.1 (71.9 to 72.2) | FastEmu 3.0x less |
| Eight at once, peak working set, MiB (MicroEmulator: sum of 8 processes) | 87.1 (86.9 to 87.8) | 802.6 (782.9 to 815.2) | FastEmu 9.2x less |
| Eight at once, private bytes, MiB (MicroEmulator: sum) | 79.8 (79.0 to 80.5) | 5054.7 (5050.9 to 5069.0) | FastEmu 63.4x less |
| Eight at once, CPU ms a second | 63.7 (63.1 to 66.1) | 190.7 (159.5 to 192.1) | FastEmu 3.0x less |
| CPU kernels, warm | see below | see below | MicroEmulator 5.5x to 131.0x faster in 9 |
| CPU kernels, cold | see below | see below | MicroEmulator 4.0x to 54.8x faster in 9 |
| Memory kernels, warm | see below | see below | MicroEmulator 11.2x to 95.8x faster in 2; FastEmu 3.2x faster in 1 |
| Drawing kernels, warm | see below | see below | FastEmu 1.6x to 6.9x faster in 5; MicroEmulator 1.3x to 1.8x faster in 2; about the same in 1 |
| Drawing kernels, cold | see below | see below | FastEmu 1.7x to 6.7x faster in 4; MicroEmulator 1.5x faster in 2; about the same in 1 |
| Unpaced frame loop, frames a second | 17883.5 (17656.0 to 18132.7) | 7992.5 (7910.0 to 8009.0) | FastEmu 2.2x more frames |
| Kernel run, cores in use (CPU time over wall time) | 0.99 (0.99 to 0.99) | 1.03 (1.03 to 1.04) | about the same |

### The real games

CPU is processor time. Peak WS is the most RAM the process held;
private is what it reserved for itself. For all at once, the first
frame is when the last game showed its first, and MicroEmulator's
numbers are the sums of its processes. To keep the table narrow, only
the first frame and CPU a second show their ranges; `bench.md` has the
rest.

| Game | Emulator | First frame, ms | fps | CPU ms/s | CPU ms/frame | Peak WS, MiB | Private, MiB | Screen |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Scene, 30 fps | FastEmu | 46 (27 to 308) | 30.0 | 7.2 (6.7 to 7.7) | 0.24 | 23.8 | 18.3 | 240x320 |
| Scene, 30 fps | MicroEmulator | 207 (196 to 329) | 30.0 | 12.0 (11.7 to 12.8) | 0.40 | 76.7 | 626.4 | 240x320 |
| Scene, 30 fps | MicroEmulator, small heap | 199 (198 to 200) | 30.0 | 11.2 (11.0 to 11.5) | 0.37 | 72.1 | 53.7 | 240x320 |
| Block Fall | FastEmu | 50 (46 to 91) | 19.9 | 5.9 (5.8 to 6.0) | 0.30 | 38.4 | 32.2 | 240x320 |
| Block Fall | MicroEmulator | 198 (195 to 218) | 19.3 | 24.1 (24.0 to 35.6) | 1.24 | 91.1 | 635.0 | 240x320 |
| BombMan | FastEmu | 28 (27 to 30) | 29.8 | 17.3 (16.7 to 17.5) | 0.58 | 41.1 | 34.3 | 240x320 |
| BombMan | MicroEmulator | 200 (196 to 221) | 39.6 | 63.2 (62.7 to 67.0) | 1.60 | 190.2 | 636.9 | 240x320 |
| Shooter | FastEmu | 31 (28 to 39) | 0.0 | 1.9 (1.9 to 2.0) | n/a | 38.2 | 31.6 | 240x320 |
| Shooter | MicroEmulator | 194 (193 to 197) | 0.0 | 1.1 (0.9 to 1.1) | n/a | 57.4 | 624.8 | 176x176 |
| Skocko | FastEmu | 30 (29 to 34) | 1.0 | 2.1 (2.1 to 2.1) | 2.12 | 24.1 | 18.3 | 240x320 |
| Skocko | MicroEmulator | 204 (194 to 209) | 1.0 | 1.6 (1.5 to 4.4) | 1.64 | 59.1 | 625.9 | 176x176 |
| Star Defender | FastEmu | 48 (46 to 105) | 19.9 | 5.7 (5.6 to 5.8) | 0.29 | 38.4 | 32.0 | 240x320 |
| Star Defender | MicroEmulator | 198 (195 to 201) | 19.7 | 21.9 (20.4 to 39.2) | 1.12 | 90.2 | 630.3 | 240x320 |
| Stick Fight | FastEmu | 88 (85 to 90) | 30.0 | 31.8 (30.8 to 32.4) | 1.06 | 50.4 | 44.2 | 240x320 |
| Stick Fight | MicroEmulator | 223 (216 to 228) | 29.4 | 27.8 (26.8 to 30.8) | 0.95 | 105.5 | 636.1 | 240x320 |
| Tower Guard | FastEmu | 47 (47 to 96) | 19.9 | 6.6 (6.4 to 6.7) | 0.33 | 38.9 | 32.5 | 240x320 |
| Tower Guard | MicroEmulator | 204 (196 to 206) | 19.2 | 38.1 (37.7 to 38.2) | 1.99 | 89.5 | 629.7 | 240x320 |
| Traffic Dash | FastEmu | 49 (47 to 52) | 19.9 | 7.4 (7.1 to 8.0) | 0.37 | 38.7 | 32.4 | 240x320 |
| Traffic Dash | MicroEmulator | 203 (193 to 248) | 19.3 | 11.5 (10.8 to 11.7) | 0.60 | 66.5 | 628.3 | 240x320 |
| All at once | FastEmu | 69 (68 to 111) | 140.3 | 63.7 (63.1 to 66.1) | 0.45 | 87.1 | 79.8 | 240x320 |
| All at once | MicroEmulator | 373 (347 to 392) | 147.2 | 190.7 (159.5 to 192.1) | 1.29 | 802.6 | 5054.7 | 176x176, 240x320 |
| All at once | MicroEmulator, small heap | 364 (350 to 717) | 147.2 | 160.2 (159.2 to 170.8) | 1.09 | 670.5 | 503.9 | 176x176, 240x320 |

Three games are left out of some ratios:

- **Shooter and Skocko** draw on a screen that is not full screen. On
  MicroEmulator that screen is 176x176, so the two emulators drew
  different pictures, and these games are left out of every ratio.
  Shooter's title screen drew no new frame while it was measured, on
  either emulator.
- **BombMan** ran at 29.8 frames a second on FastEmu and 39.6 on
  MicroEmulator, so its processor times are not compared. Its loop waits
  50 ms less the whole time of the frame before, waits included. So it
  takes turns between a short frame and a long one. Under FastEmu's
  frame cap, the short frame lasts one 60 Hz slot, 16.7 ms, and a pair
  takes 66.7 ms: 30 frames a second. MicroEmulator has no cap, so the
  short frame takes almost no time, and a pair takes 50 ms: 40 frames a
  second.

Block Fall, Star Defender, Tower Guard and Traffic Dash ran at 19.9
frames a second on FastEmu and at 19.2 to 19.7 on MicroEmulator. Stick
Fight ran at 30.0 and 29.4.

### CPU kernels

Time of one repetition, ms. Warm is the median of rounds 2 to 5; cold is
the first repetition. `+/-S` marks a time under 20 steps of the clock:
it is within S ms of the true time.

| Kernel | FastEmu warm | FastEmu cold | MicroEmulator warm | MicroEmulator cold | Warm ratio |
| --- | --- | --- | --- | --- | --- |
| cpu.int | 465.62 (464.25 to 466.25) | 466 | 13.54 (13.50 to 13.58) | 16 (+/-2) | MicroEmulator 34.4x faster |
| cpu.call | 164.75 (164.50 to 166.00) | 164 | 2.27 (2.26 to 2.27) | 3 (+/-2) | MicroEmulator 72.6x faster |
| cpu.sieve | 172.08 (171.83 to 188.83) | 172 | 6.08 (6.06 to 6.11) | 12 (+/-2) | MicroEmulator 28.3x faster |
| cpu.sort | 148.56 (148.38 to 148.88) | 148 | 8.27 (8.22 to 8.31) | 10 (+/-2) | MicroEmulator 18.0x faster |
| cpu.actors | 178.58 (177.67 to 179.33) | 178 | 1.36 (1.36 to 1.38) | 4 (+/-2) | MicroEmulator 131.0x faster |
| cpu.long | 209.17 (207.67 to 227.17) | 210 | 3.87 (3.86 to 3.90) | 6 (+/-2) | MicroEmulator 54.0x faster |
| cpu.float | 243.50 (242.67 to 246.00) | 243 | 22.67 (22.63 to 22.75) | 21 (+/-2) | MicroEmulator 10.7x faster |
| cpu.string | 135.44 (133.00 to 136.12) | 136 | 10.82 (10.80 to 10.88) | 18 (+/-2) | MicroEmulator 12.5x faster |
| cpu.collections | 161.00 (159.50 to 169.42) | 158 | 29.30 (29.19 to 29.33) | 40 | MicroEmulator 5.5x faster |

### Memory kernels

Time of one repetition, ms; `mem.gc` is one full collection.

| Kernel | FastEmu warm | FastEmu cold | MicroEmulator warm | MicroEmulator cold | Warm ratio |
| --- | --- | --- | --- | --- | --- |
| mem.small | 240.58 (234.17 to 256.00) | 240 | 2.51 (2.43 to 2.56) | 6 (+/-2) | MicroEmulator 95.8x faster |
| mem.arrays | 149.18 (102.70 to 193.33) | 192 | 13.37 (13.13 to 14.41) | 15 (+/-2) | MicroEmulator 11.2x faster |
| mem.gc | 3.46 (3.03 to 3.73) | 5 (+/-1) | 11.00 (10.89 to 11.08) | 12 (+/-2) | FastEmu 3.2x faster |

Every CPU and memory kernel printed the same check value on both
emulators.

### Drawing kernels

Time of one repetition, ms, into a 240x320 image.

| Kernel | FastEmu warm | FastEmu cold | MicroEmulator warm | MicroEmulator cold | Warm ratio |
| --- | --- | --- | --- | --- | --- |
| draw.fillrect | 142.62 (142.00 to 143.38) | 142 | 224.08 (198.50 to 230.50) | 238 | FastEmu 1.6x faster |
| draw.line | 223.00 (221.83 to 226.83) | 224 | 170.42 (163.50 to 174.33) | 154 | MicroEmulator 1.3x faster |
| draw.image | 118.20 (117.10 to 119.00) | 118 | 112.90 (112.10 to 113.90) | 122 | about the same |
| draw.alpha | 172.25 (172.00 to 174.17) | 172 | 283.75 (280.75 to 284.75) | 288 | FastEmu 1.6x faster |
| draw.region | 221.58 (220.67 to 222.67) | 221 | 655.75 (650.50 to 679.00) | 668 | FastEmu 3.0x faster |
| draw.text | 185.25 (184.83 to 186.17) | 186 | 104.70 (102.10 to 105.20) | 130 | MicroEmulator 1.8x faster |
| draw.rgb | 138.56 (138.25 to 139.25) | 138 | 960.75 (953.50 to 970.50) | 923 | FastEmu 6.9x faster |
| draw.scene | 0.05 (0.05 to 0.05) | 0 (+/-1) | 0.10 (0.10 to 0.11) | 0 (+/-2) | FastEmu 2.2x faster |

### The kernel run

| Emulator | Clock step, ms | CPU time, ms | Wall time, ms | Cores in use | Unpaced frames a second |
| --- | --- | --- | --- | --- | --- |
| FastEmu | 1 | 63525 (62471 to 64285) | 64405 (62989 to 64988) | 0.99 | 17883.5 (17656.0 to 18132.7) |
| MicroEmulator | 2 | 60160 (59827 to 60596) | 58291 (58119 to 58446) | 1.03 | 7992.5 (7910.0 to 8009.0) |

### What the bench flagged

- Three measurements ran while other load passed the threshold: Tower
  Guard on FastEmu in run 1, at 574 ms a second, and Tower Guard and
  Traffic Dash on MicroEmulator in run 2, at 480 and 489. Each gave about
  the numbers of its other three runs.
- MicroEmulator's processor time spread more than 15 percent in five
  rows: Block Fall, Shooter, Skocko, Star Defender and all at once. In
  all but Shooter, one run of the four differs, and the median sits
  with the other three. Shooter's runs spread from 0.9 to 1.1 ms a
  second.

The run was on the Balanced power plan, with a few programs open but
idle, not in the best performance mode that
[Runs and a quiet machine](#runs-and-a-quiet-machine) suggests. The load
numbers above say how quiet the computer was.

## Where MicroEmulator is faster

MicroEmulator is a Java program. It runs a game's code on the Java
virtual machine you installed, and that machine compiles the code to
machine code as it runs. FastEmu has a virtual machine of its own that
interprets the code, one instruction at a time. So in long loops of
arithmetic, method calls and array work, MicroEmulator runs game code
many times faster. In the CPU kernels it was 5.5 to 131 times faster
once warm, and 4.0 to 54.8 times faster on the very first repetition,
before its compiler had warmed up. It also made small objects and
arrays 95.8 and 11.2 times faster. FastEmu was faster at one of them: a
full collection took 3.46 ms against 11.00.

In drawing, MicroEmulator was faster at lines, 1.3 times, and at text,
1.8 times. Drawing images was about the same.

A game that draws a few sprites a frame and then sleeps until the next
one spends little time in such loops. For it, the costs that show are
starting up, memory, and the processor time a frame takes, and there
FastEmu used less in four of the five games compared. Stick Fight is
the one where FastEmu used more: 31.8 ms a second against 27.8, 1.1
times. A game that runs more of its own code each frame than these can
cost more on FastEmu, by up to the kernels' ratios.

The Java virtual machine's compiler and garbage collector run on other
cores beside the game, and the bench counts the processor time of every
thread. In the kernel run, that came to 1.03 cores in use against
FastEmu's 0.99.

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

For a run, put the computer on mains power and in its best performance
mode, pause browsers, editors, chat, OneDrive and Windows Update, and
mute the speakers.

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
