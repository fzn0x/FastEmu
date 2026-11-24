//! Many real instances at once in one process, as the apps run them.
//!
//! Eight copies of the MIDlet in `tests/many_java` start through
//! `MidpLauncher`, each in its own virtual machine with its own data
//! folder, all on one offline sound engine. The test checks that they all
//! present frames, that input, a pause, record stores and sound stay with
//! the instance they were meant for, and that stopping them in a mixed
//! order ends every one and frees its virtual machine. A hang fails the
//! test after [`LIMIT`] instead of stalling CI.

mod media_support;
mod support;

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use fastemu_audio::{Engine, EngineConfig};
use fastemu_core::{
    Frame, InputEvent, Instance, InstanceSettings, InstanceStatus, Key, KeyAction, LaunchRequest,
    Notify,
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
    games[PAUSED].instance().resume();
    wait_until("instance 3 runs again", &mut games, |games| {
        games[PAUSED].host.status() == InstanceStatus::Running
            && games[PAUSED].serial() > held
            && games[PAUSED].count("startApp 3") == 2
    });

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
