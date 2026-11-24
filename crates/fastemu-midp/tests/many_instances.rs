//! Many real instances at once in one process, as the apps run them.
//!
//! Eight copies of the MIDlet in `tests/many_java` start through
//! `MidpLauncher`, each in its own virtual machine with its own data
//! folder, all on one offline sound engine. The test checks that they all
//! present frames, that input, a pause, record stores and sound stay with
//! the instance they were meant for, and that stopping them in a mixed
//! order ends every one and frees its virtual machine. A hang fails the
//! test after [`LIMIT`] instead of stalling CI.
//!
//! `soak_real_games` runs real games from a folder, side by side and then
//! one at a time, and prints how each did. It is ignored by default; see
//! CONTRIBUTING.md.

mod media_support;
mod support;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use fastemu_audio::{Engine, EngineConfig, NullOutput};
use fastemu_core::{
    Frame, InputEvent, Instance, InstanceSettings, InstanceStatus, Key, KeyAction, LaunchRequest,
    LogLevel, Notify,
};
use fastemu_midp::host::HostLink;
use fastemu_midp::{MidpInstance, MidpLauncher};
use media_support::{RATE, sine_wav};
use support::rgb;

/// How many instances run at once.
const COUNT: usize = 8;

/// How long the whole test may take before it counts as hung.
const LIMIT: Duration = Duration::from_secs(120);

/// How long one condition may take: generous, for a loaded CI machine.
const DEADLINE: Duration = Duration::from_secs(30);

/// How long the sound the MIDlet plays lasts.
const SOUND_MS: u32 = 200;

const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Many\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: Many, , many.Multi\r\n";

/// The MIDlet's classes, compiled once against the stubs the other tests
/// use, or `None` (after saying why) when there is no JDK.
fn classes() -> Option<&'static Path> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let compiled = CLASSES.get_or_init(|| {
        let javac = support::tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let tests = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
        let stubs = std::env::join_paths([
            tests.join("java").join("stubs"),
            tests.join("rms_java").join("stubs"),
            tests.join("media_stubs"),
        ])
        .expect("joining the stub folders");
        let out = support::scratch("many-classes");
        // The stubs are only read: -implicit:none keeps their classes out
        // of the JAR, so the runtime's classes serve their names.
        let output = Command::new(javac)
            .args(["--release", "8", "-g", "-encoding", "UTF-8", "-nowarn"])
            .args(["-implicit:none", "-sourcepath"])
            .arg(stubs)
            .arg("-d")
            .arg(&out)
            .arg(tests.join("many_java").join("many").join("Multi.java"))
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(out)
    });
    match compiled {
        Ok(dir) => Some(dir),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "Java is required but unusable: {why}"
            );
            eprintln!("skipping: {why}");
            None
        }
    }
}

/// Packs the MIDlet and its sound into `dir/many.jar`.
fn write_suite(classes: &Path, dir: &Path) -> PathBuf {
    let mut entries = vec![(
        "META-INF/MANIFEST.MF".to_owned(),
        MANIFEST.as_bytes().to_vec(),
    )];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("many"))
        .expect("the compiled MIDlet")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let name = format!("many/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((name, std::fs::read(&file).expect("a class file")));
    }
    entries.push(("beep.wav".to_owned(), sine_wav(SOUND_MS)));
    let path = dir.join("many.jar");
    std::fs::write(&path, support::zip(&entries, true)).expect("writing the JAR");
    path
}

/// Runs `body` on a thread of its own and fails if it takes longer than
/// `limit`, so a deadlock fails the test instead of hanging it.
fn bounded(limit: Duration, body: impl FnOnce() + Send + 'static) {
    let (done, finished) = mpsc::channel();
    let worker = std::thread::Builder::new()
        .name("many instances".into())
        .spawn(move || {
            body();
            let _ = done.send(());
        })
        .expect("starting the test's thread");
    match finished.recv_timeout(limit) {
        Ok(()) | Err(RecvTimeoutError::Disconnected) => {
            if let Err(panic) = worker.join() {
                std::panic::resume_unwind(panic);
            }
        }
        Err(RecvTimeoutError::Timeout) => {
            panic!("the test did not finish within {limit:?}: something hangs")
        }
    }
}

/// One launched instance and what the test keeps of it.
struct Game {
    id: usize,
    instance: Option<Arc<MidpInstance>>,
    /// The instance's link, to read its status after it is dropped.
    host: HostLink,
    /// Shared with the instance's notify function, which lives as long as
    /// any part of the instance: its virtual machine, its threads, its
    /// control thread. Only this one is left once they are all gone.
    alive: Arc<()>,
    log: Vec<String>,
}

impl Game {
    fn launch(launcher: &MidpLauncher, jar: &Path, root: &Path, id: usize) -> Self {
        let alive = Arc::new(());
        let held = Arc::clone(&alive);
        let notify: Notify = Arc::new(move |_| {
            let _ = &held;
        });
        let request = LaunchRequest {
            settings: InstanceSettings {
                properties: vec![("Multi-Id".to_owned(), id.to_string())],
                ..InstanceSettings::default()
            },
            notify: Some(notify),
            ..LaunchRequest::new(jar, root.join(format!("data-{id}")))
        };
        let instance = launcher.launch_midp(request).expect("launching the MIDlet");
        Self {
            id,
            host: instance.host().clone(),
            instance: Some(instance),
            alive,
            log: Vec::new(),
        }
    }

    fn instance(&self) -> &MidpInstance {
        self.instance
            .as_deref()
            .expect("the instance is still held")
    }

    fn frame(&self) -> Option<Frame> {
        self.host.frame()
    }

    fn serial(&self) -> u64 {
        self.frame().map_or(0, |f| f.serial())
    }

    fn lines(&mut self) -> &[String] {
        self.log
            .extend(self.host.drain_log().into_iter().map(|line| line.text));
        &self.log
    }

    fn has(&mut self, needle: &str) -> bool {
        self.lines().iter().any(|l| l.contains(needle))
    }

    fn count(&mut self, line: &str) -> usize {
        self.lines().iter().filter(|l| *l == line).count()
    }

    /// The longest the MIDlet's game loop said it stood still, in its log
    /// lines from line `from` on.
    fn stood_still(&mut self, from: usize) -> Duration {
        let prefix = format!("stood still {} for ", self.id);
        let longest = self.lines()[from..]
            .iter()
            .filter_map(|l| l.strip_prefix(&prefix)?.parse().ok())
            .max()
            .unwrap_or(0);
        Duration::from_millis(longest)
    }

    fn tap(&self, key: Key) {
        for action in [KeyAction::Press, KeyAction::Release] {
            self.instance().send(InputEvent::Key { key, action });
        }
    }

    /// Whether the latest frame shows `key` as the last key and `presses`
    /// keys in all, and this instance's id.
    fn shows(&self, key: u32, presses: u32) -> bool {
        self.frame().is_some_and(|f| {
            rgb(&f, 0, 5) == 0x40_0000 | key
                && rgb(&f, 0, 15) == 0x10_0000 | presses
                && rgb(&f, 0, 25) == 0x20_0000 | self.id as u32
        })
    }
}

/// Every instance's status, frame count and last log lines, for failures.
fn report(games: &mut [Game]) -> String {
    let mut out = String::new();
    for game in games {
        let (id, status, serial) = (game.id, game.host.status(), game.serial());
        let lines = game.lines();
        let tail = &lines[lines.len().saturating_sub(8)..];
        let _ = writeln!(
            out,
            "instance {id}: {status:?}, frame {serial}\n    {}",
            tail.join("\n    ")
        );
    }
    out
}

/// Waits until `done` holds, failing with every instance's state after
/// [`DEADLINE`].
fn wait_until(what: &str, games: &mut [Game], mut done: impl FnMut(&mut [Game]) -> bool) {
    let start = Instant::now();
    while !done(games) {
        assert!(
            start.elapsed() < DEADLINE,
            "{what}: not within {DEADLINE:?}\n{}",
            report(games)
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Each instance's frame count now.
fn serials(games: &[Game]) -> Vec<u64> {
    games.iter().map(Game::serial).collect()
}

/// Sound frames of the offline engine in `ms`.
fn sound_frames(ms: u32) -> usize {
    (RATE * ms / 1000) as usize
}

#[test]
fn many_instances_run_side_by_side() {
    let Some(classes) = classes() else { return };
    let root = support::scratch("many-instances");
    let jar = write_suite(classes, &root);
    bounded(LIMIT, move || side_by_side(&jar, &root));
}

fn side_by_side(jar: &Path, root: &Path) {
    let engine = Engine::new(EngineConfig { sample_rate: RATE });
    let launcher = MidpLauncher::new(Some(engine.clone()));
    let mut games: Vec<Game> = (0..COUNT)
        .map(|id| Game::launch(&launcher, jar, root, id))
        .collect();

    // Every instance presents frames, each its own.
    wait_until("every instance presents frames", &mut games, |games| {
        games.iter().all(|g| g.serial() >= 10)
    });
    wait_until("every frame shows its instance", &mut games, |games| {
        games.iter().all(|g| g.shows(0, 0))
    });

    // Input reaches only the instance it is sent to: instance i gets the
    // key i + 1, whose code is '1' + i.
    let keys = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
    ];
    for (game, key) in games.iter().zip(keys) {
        game.tap(key);
    }
    let code = |game: &Game| u32::from(b'1') + game.id as u32;
    wait_until("each instance shows its own key", &mut games, |games| {
        games.iter().all(|g| g.shows(code(g), 1))
    });
    // A key that went astray could still be on its way: look again a few
    // frames later.
    let marks = serials(&games);
    wait_until("five more frames each", &mut games, |games| {
        games.iter().all(|g| g.serial() >= marks[g.id] + 5)
    });
    for game in &games {
        assert!(
            game.shows(code(game), 1),
            "instance {} got a key that was not its own",
            game.id
        );
    }

    // Pausing one freezes it alone; resuming brings it back.
    const PAUSED: usize = 3;
    games[PAUSED].instance().pause();
    wait_until("instance 3 pauses", &mut games, |games| {
        games[PAUSED].host.status() == InstanceStatus::Paused
    });
    let paused_at = Instant::now();
    let pause_line = games[PAUSED].lines().len();
    assert!(games[PAUSED].has("pauseApp 3"), "{}", report(&mut games));
    let held = games[PAUSED].serial();
    let marks = serials(&games);
    wait_until(
        "the others present while one is paused",
        &mut games,
        |games| {
            games
                .iter()
                .filter(|g| g.id != PAUSED)
                .all(|g| g.serial() >= marks[g.id] + 20)
        },
    );
    assert_eq!(
        games[PAUSED].serial(),
        held,
        "a paused instance presents no frames"
    );
    for game in games.iter_mut().filter(|g| g.id != PAUSED) {
        assert!(!game.has("pauseApp"), "instance {} paused too", game.id);
        assert_eq!(game.host.status(), InstanceStatus::Running);
    }
    let paused_for = paused_at.elapsed();
    games[PAUSED].instance().resume();
    wait_until("instance 3 runs again", &mut games, |games| {
        games[PAUSED].host.status() == InstanceStatus::Running
            && games[PAUSED].serial() > held
            && games[PAUSED].count("startApp 3") == 2
    });
    // A hidden canvas presents nothing even while its threads run on, so
    // the frames above cannot tell a pause that stops nothing. The game
    // loop stands still from the pause to the resume, longer than the
    // test saw it paused; half of that leaves room for a loop that ran a
    // moment past the pause.
    wait_until(
        "instance 3's game loop stood still while it was paused",
        &mut games,
        |games| games[PAUSED].stood_still(pause_line) >= paused_for / 2,
    );

    // Record stores stay apart: every instance wrote its id at its first
    // start to a store of the same name, and reads back only its own.
    wait_until("every instance writes its record", &mut games, |games| {
        games.iter_mut().all(|g| {
            let line = format!("rms wrote {}", g.id);
            g.has(&line)
        })
    });
    for game in &games {
        game.tap(Key::Pound);
    }
    wait_until("every instance reads its store", &mut games, |games| {
        games.iter_mut().all(|g| {
            let line = format!("rms read {}:", g.id);
            g.has(&line)
        })
    });
    for game in &mut games {
        let id = game.id;
        let prefix = format!("rms read {id}:");
        let read = game
            .lines()
            .iter()
            .find(|l| l.starts_with(&prefix))
            .cloned()
            .unwrap_or_default();
        assert_eq!(read, format!("rms read {id}: {id},"), "instance {id}");
        assert!(root.join(format!("data-{id}")).join("rms").is_dir());
    }

    // Two instances play at once on the shared engine, and neither holds
    // the other up.
    const A: usize = 5;
    const B: usize = 6;
    games[A].tap(Key::Star);
    games[B].tap(Key::Star);
    wait_until("two instances start a sound", &mut games, |games| {
        games[A].has("sound started 5") && games[B].has("sound started 6")
    });
    assert_eq!(engine.playing_count(), 2, "both sounds play at once");
    engine.advance(sound_frames(SOUND_MS / 2));
    assert_eq!(engine.playing_count(), 2, "both still play halfway");
    assert!(!games[A].has("sound ended") && !games[B].has("sound ended"));
    engine.advance(sound_frames(SOUND_MS));
    wait_until("both sounds end", &mut games, |games| {
        games[A].has("sound ended 5") && games[B].has("sound ended 6")
    });
    assert_eq!(engine.playing_count(), 0);

    for game in &mut games {
        let id = game.id;
        assert_eq!(game.instance().error_count(), 0, "instance {id}");
        assert!(!game.has("failed"), "instance {id}: {:?}", game.lines());
        if id != A && id != B {
            assert!(!game.has("sound"), "instance {id} played a sound");
        }
    }

    // Stopping in a mixed order ends them all: one while paused, one by
    // dropping it, the rest by stop, without waiting between them.
    const PAUSED_AT_STOP: usize = 6;
    const DROPPED: usize = 3;
    games[PAUSED_AT_STOP].instance().pause();
    wait_until("instance 6 pauses", &mut games, |games| {
        games[PAUSED_AT_STOP].host.status() == InstanceStatus::Paused
    });
    for id in [5, 0, 7, 2, 4, 1, PAUSED_AT_STOP, DROPPED] {
        if id == DROPPED {
            games[id].instance = None;
        } else {
            games[id].instance().stop();
        }
    }
    wait_until("every instance exits", &mut games, |games| {
        games
            .iter()
            .all(|g| g.host.status() == InstanceStatus::Exited)
    });
    for game in &mut games {
        let id = game.id;
        assert!(
            game.has(&format!("destroyApp {id}")),
            "instance {id}: {:?}",
            game.lines()
        );
    }

    // Every thread of every instance ends and its virtual machine is
    // freed: then nothing but the test holds what the notify function
    // owns.
    let alive: Vec<(usize, Arc<()>)> = games.into_iter().map(|g| (g.id, g.alive)).collect();
    let start = Instant::now();
    loop {
        let left: Vec<usize> = alive
            .iter()
            .filter(|(_, token)| Arc::strong_count(token) > 1)
            .map(|(id, _)| *id)
            .collect();
        if left.is_empty() {
            break;
        }
        assert!(
            start.elapsed() < DEADLINE,
            "instances {left:?} still have threads or a virtual machine"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(engine.player_count(), 0, "a stopped instance left players");
}

/// The key script of the soak test, from launch: nothing for 1.5 s, `5`,
/// then the arrows and fire in turn, one every 250 ms, each held 100 ms.
struct Script {
    next: usize,
}

impl Script {
    const LOOP: [Key; 5] = [Key::Up, Key::Right, Key::Down, Key::Left, Key::Fire];

    /// When event `n` is due and what it is.
    fn event(n: usize) -> (Duration, Key, KeyAction) {
        let action = |n: usize| {
            if n.is_multiple_of(2) {
                KeyAction::Press
            } else {
                KeyAction::Release
            }
        };
        if n < 2 {
            let at = 1500 + 100 * n as u64;
            return (Duration::from_millis(at), Key::Num5, action(n));
        }
        let step = (n - 2) / 2;
        let at = 2000 + 250 * step as u64 + 100 * ((n - 2) % 2) as u64;
        (
            Duration::from_millis(at),
            Self::LOOP[step % Self::LOOP.len()],
            action(n - 2),
        )
    }

    /// The events due by `elapsed`, in order.
    fn due(&mut self, elapsed: Duration) -> Vec<InputEvent> {
        let mut events = Vec::new();
        loop {
            let (at, key, action) = Self::event(self.next);
            if at > elapsed {
                return events;
            }
            events.push(InputEvent::Key { key, action });
            self.next += 1;
        }
    }
}

/// When the soak test starts counting frames for the frame rate, past
/// loading screens.
const WARMUP: Duration = Duration::from_secs(3);

/// How one game did in a soak run.
#[derive(Debug, Default)]
struct Outcome {
    name: String,
    frames: u64,
    /// Frames presented between [`WARMUP`] and the end of the window.
    window_frames: u64,
    errors: u64,
    error_lines: Vec<String>,
    status: Option<InstanceStatus>,
    /// The longest an `Instance::send` call took.
    slowest_send: Duration,
    /// Whether every thread ended and the virtual machine was freed after
    /// the stop.
    freed: bool,
}

/// Runs `jars` side by side for `length` with the key script, counting
/// frames presented between [`WARMUP`] and `window`, then stops them all.
fn soak(
    launcher: &MidpLauncher,
    jars: &[PathBuf],
    data: &Path,
    length: Duration,
    window: Duration,
) -> Vec<Outcome> {
    struct Running {
        instance: Arc<MidpInstance>,
        started: Instant,
        script: Script,
        alive: Arc<()>,
        warm: Option<u64>,
        outcome: Outcome,
    }
    let mut games: Vec<Running> = jars
        .iter()
        .enumerate()
        .map(|(i, jar)| {
            let alive = Arc::new(());
            let held = Arc::clone(&alive);
            let notify: Notify = Arc::new(move |_| {
                let _ = &held;
            });
            let request = LaunchRequest {
                notify: Some(notify),
                ..LaunchRequest::new(jar, data.join(format!("{i}")))
            };
            let name = jar.file_stem().unwrap().to_string_lossy().into_owned();
            let instance = launcher
                .launch_midp(request)
                .unwrap_or_else(|e| panic!("launching {name}: {e}"));
            Running {
                instance,
                started: Instant::now(),
                script: Script { next: 0 },
                alive,
                warm: None,
                outcome: Outcome {
                    name,
                    ..Outcome::default()
                },
            }
        })
        .collect();
    let start = Instant::now();
    while start.elapsed() < length {
        for game in &mut games {
            let elapsed = game.started.elapsed();
            for event in game.script.due(elapsed) {
                let sent = Instant::now();
                game.instance.send(event);
                game.outcome.slowest_send = game.outcome.slowest_send.max(sent.elapsed());
            }
            let serial = game.instance.frame().map_or(0, |f| f.serial());
            if elapsed >= WARMUP && game.warm.is_none() {
                game.warm = Some(serial);
            }
            if elapsed <= window {
                game.outcome.window_frames = serial - game.warm.unwrap_or(serial);
            }
            game.outcome.frames = serial;
            for line in game.instance.drain_log() {
                if line.level == LogLevel::Err && game.outcome.error_lines.len() < 3 {
                    game.outcome.error_lines.push(line.text);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    for game in &mut games {
        game.outcome.status = Some(game.instance.status());
        game.outcome.errors = game.instance.error_count();
        game.instance.stop();
    }
    let stop = Instant::now();
    let mut outcomes = Vec::new();
    for game in games {
        let host = game.instance.host().clone();
        while !host.status().is_finished() && stop.elapsed() < DEADLINE {
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(host);
        drop(game.instance);
        while Arc::strong_count(&game.alive) > 1 && stop.elapsed() < DEADLINE {
            std::thread::sleep(Duration::from_millis(5));
        }
        let mut outcome = game.outcome;
        outcome.freed = Arc::strong_count(&game.alive) == 1;
        outcomes.push(outcome);
    }
    outcomes
}

/// The process's CPU time so far and its peak and current memory, where
/// the platform tells them without extra crates.
#[derive(Clone, Copy, Debug)]
struct Usage {
    cpu: Duration,
    peak_bytes: u64,
    now_bytes: u64,
}

/// Prints the CPU time in milliseconds, the peak working set and the
/// working set of process `PID`.
const WINDOWS_USAGE: &str = "
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices;
public static class FastEmuCycles {
    [DllImport(\"kernel32.dll\")] public static extern bool QueryProcessCycleTime(IntPtr p, out ulong c);
    [DllImport(\"kernel32.dll\")] public static extern bool QueryThreadCycleTime(IntPtr t, out ulong c);
    [DllImport(\"kernel32.dll\")] public static extern IntPtr GetCurrentThread();
}'
$t = [FastEmuCycles]::GetCurrentThread()
$a = [uint64]0; [void][FastEmuCycles]::QueryThreadCycleTime($t, [ref]$a)
$w = [Diagnostics.Stopwatch]::StartNew(); while ($w.ElapsedMilliseconds -lt 200) {}
$b = [uint64]0; [void][FastEmuCycles]::QueryThreadCycleTime($t, [ref]$b)
$rate = ($b - $a) / $w.Elapsed.TotalSeconds
$p = Get-Process -Id PID
$c = [uint64]0; [void][FastEmuCycles]::QueryProcessCycleTime($p.Handle, [ref]$c)
'{0} {1} {2}' -f [uint64]($c / $rate * 1000), $p.PeakWorkingSet64, $p.WorkingSet64
";

fn usage() -> Option<Usage> {
    if cfg!(target_os = "linux") {
        let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
        // The fields after the command, which is in parentheses.
        let fields: Vec<&str> = stat.rsplit_once(')')?.1.split_whitespace().collect();
        let ticks: u64 =
            fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?;
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let kib = |key: &str| -> Option<u64> {
            let line = status.lines().find(|l| l.starts_with(key))?;
            line.split_whitespace().nth(1)?.parse().ok()
        };
        return Some(Usage {
            // Linux counts in USER_HZ, 100 a second on every platform it
            // supports.
            cpu: Duration::from_millis(ticks * 10),
            peak_bytes: kib("VmHWM:")? * 1024,
            now_bytes: kib("VmRSS:")? * 1024,
        });
    }
    if cfg!(windows) {
        // Windows charges CPU time by the clock tick, which misses most of
        // the short bursts game threads run in, so count cycles instead,
        // at the rate a busy thread of PowerShell's runs them.
        let script = WINDOWS_USAGE.replace("PID", &std::process::id().to_string());
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.split_whitespace();
        return Some(Usage {
            cpu: Duration::from_millis(parts.next()?.parse().ok()?),
            peak_bytes: parts.next()?.parse().ok()?,
            now_bytes: parts.next()?.parse().ok()?,
        });
    }
    None
}

fn mib(bytes: u64) -> String {
    format!("{:.0} MiB", bytes as f64 / (1024.0 * 1024.0))
}

/// CPU time per second of `length`, as a share of one core.
fn cpu_share(before: Option<Usage>, after: Option<Usage>, length: Duration) -> String {
    match (before, after) {
        (Some(b), Some(a)) => format!(
            "{:.0} ms/s",
            a.cpu.saturating_sub(b.cpu).as_secs_f64() * 1000.0 / length.as_secs_f64()
        ),
        _ => "n/a".to_owned(),
    }
}

/// Real games side by side: every JAR in `FASTEMU_GAMES_DIR` at once for
/// `FASTEMU_SOAK_SECONDS` (60 by default), then each alone for up to
/// 20 s, all with the same key script. Prints per game the frames, the
/// errors, the final status and the frame rate together and alone, with
/// the process's CPU time and memory. Run it with
/// `cargo test --release -p fastemu-midp --test many_instances -- --ignored --nocapture`.
#[test]
#[ignore = "runs real games from FASTEMU_GAMES_DIR for minutes; see CONTRIBUTING.md"]
fn soak_real_games() {
    let dir = std::env::var_os("FASTEMU_GAMES_DIR")
        .map(PathBuf::from)
        .expect("FASTEMU_GAMES_DIR names a folder of JARs");
    let seconds: u64 = std::env::var("FASTEMU_SOAK_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let length = Duration::from_secs(seconds.max(5));
    let solo = length.min(Duration::from_secs(20));
    let mut jars: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("reading FASTEMU_GAMES_DIR")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jar")))
        .collect();
    jars.sort();
    assert!(!jars.is_empty(), "no JARs in {}", dir.display());
    let data = tempfile::tempdir().expect("a data folder");
    let engine = Engine::default();
    let _output = NullOutput::start(&engine);
    let launcher = MidpLauncher::new(Some(engine.clone()));

    let before = usage();
    println!("{} games together for {}s", jars.len(), length.as_secs());
    let together = soak(
        &launcher,
        &jars,
        &data.path().join("together"),
        length,
        solo,
    );
    let after = usage();
    let mut alone = Vec::new();
    let mut solo_cpu = Vec::new();
    for (i, jar) in jars.iter().enumerate() {
        let start = usage();
        let mut outcome = soak(
            &launcher,
            std::slice::from_ref(jar),
            &data.path().join(format!("alone-{i}")),
            solo,
            solo,
        );
        let end = usage();
        solo_cpu.push(cpu_share(start, end, solo));
        alone.push(outcome.remove(0));
        if let Some(end) = end {
            println!("after {} alone: {}", alone[i].name, mib(end.now_bytes));
        }
    }

    let span = solo.saturating_sub(WARMUP).as_secs_f64();
    println!(
        "\n{:<24} {:>7} {:>8} {:>9} {:>8} {:>6} {:>8} {:>7} {:>9} {:>6}  status",
        "game",
        "frames",
        "fps all",
        "fps with",
        "fps solo",
        "ratio",
        "cpu solo",
        "errors",
        "send max",
        "freed"
    );
    for ((with, solo_run), cpu) in together.iter().zip(&alone).zip(&solo_cpu) {
        let fps_with = with.window_frames as f64 / span;
        let fps_solo = solo_run.window_frames as f64 / span;
        let ratio = if fps_solo > 0.0 {
            fps_with / fps_solo
        } else {
            f64::NAN
        };
        println!(
            "{:<24} {:>7} {:>8.1} {:>9.1} {:>8.1} {:>6.2} {:>8} {:>7} {:>9} {:>6}  {:?}",
            with.name,
            with.frames,
            with.frames as f64 / length.as_secs_f64(),
            fps_with,
            fps_solo,
            ratio,
            cpu,
            with.errors,
            format!("{:.1?}", with.slowest_send),
            with.freed && solo_run.freed,
            with.status.as_ref().unwrap_or(&InstanceStatus::Starting),
        );
        for line in &with.error_lines {
            println!("    {line}");
        }
    }
    println!(
        "\ntogether: cpu {}, peak memory {}, memory after they stopped {}",
        cpu_share(before, after, length),
        after.map_or("n/a".to_owned(), |u| mib(u.peak_bytes)),
        after.map_or("n/a".to_owned(), |u| mib(u.now_bytes)),
    );
    for outcome in together.iter().chain(&alone) {
        assert!(
            outcome.freed,
            "{} did not end and free its machine",
            outcome.name
        );
    }
}
