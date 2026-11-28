//! The bench log: what the Android bench (`cargo xtask bench --android`)
//! reads from logcat, since it cannot see into the app.
//!
//! It is off unless `adb shell setprop log.tag.FastEmuBench DEBUG` was set
//! before the app started, the way Android's own `Log.isLoggable` is turned
//! on. Only adb and the system can set that property. When it is off, the
//! app runs the launcher as it is; when it is on, [`BenchLauncher`] wraps it
//! and writes the desktop bench's lines (`tools/xtask/src/bench/wire.rs`)
//! for each game it starts, numbered from 0 in the order they start:
//!
//! ```text
//! RUN first game=0 ms=86
//! log 0 out BENCH cpu.int round=2 reps=3 ms=1404 first=470 check=127
//! RUN frames game=0 n=120 ms=4000 w=240 h=320
//! RUN end game=0 status=exited frames=600 errors=0 ms=9000
//! ```
//!
//! `ms` counts from the launcher's start. The bench times a start from
//! logcat's own clock, and uses `ms` only for what happens inside the app.
//!
//! This module needs neither Android nor GPUI, so it builds and tests on
//! any host.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::thread;
use std::time::{Duration, Instant};

use fastemu_core::{
    Frame, InputEvent, Instance, InstanceId, InstanceStatus, LaunchError, LaunchRequest, Launcher,
    LogLevel, LogLine, Notify, SuiteInfo,
};

/// The name of the bench log: `log.tag.FastEmuBench` turns it on, and the
/// app's logcat lines (tag `fastemu`) that carry `FastEmuBench: ` are its
/// lines, each after that mark.
pub const NAME: &str = "FastEmuBench";

/// How often a running game's frame count is written.
const TICK: Duration = Duration::from_secs(1);

/// Where the lines go: logcat in the app, a list in tests.
pub type Sink = Arc<dyn Fn(&str) + Send + Sync>;

/// Whether a value of `log.tag.FastEmuBench` turns the bench log on:
/// `DEBUG` or `VERBOSE`, as `Log.isLoggable(tag, Log.DEBUG)` reads it.
/// Android reads only the first letter.
pub fn turned_on(value: &str) -> bool {
    matches!(value.trim().chars().next(), Some('D' | 'd' | 'V' | 'v'))
}

/// A launcher that writes the bench log for every game it starts.
pub struct BenchLauncher {
    inner: Arc<dyn Launcher>,
    shared: Arc<Shared>,
}

impl BenchLauncher {
    /// Wraps `inner`; the lines go to `sink`.
    pub fn new(inner: Arc<dyn Launcher>, sink: Sink) -> Self {
        Self::with_ticker(inner, sink, true)
    }

    /// With `ticker` false, no thread ticks: only [`Self::tick`] does.
    fn with_ticker(inner: Arc<dyn Launcher>, sink: Sink, ticker: bool) -> Self {
        Self {
            inner,
            shared: Arc::new(Shared {
                sink,
                start: Instant::now(),
                next_game: AtomicUsize::new(0),
                games: Mutex::new(Vec::new()),
                // As if a thread ticked already, so none starts.
                ticking: AtomicBool::new(!ticker),
            }),
        }
    }

    /// Writes what changed since the last tick: each game's frame count,
    /// and the end of each game that finished. A thread of its own calls
    /// it every [`TICK`] once a game has started.
    pub fn tick(&self) {
        self.shared.tick();
    }
}

impl Launcher for BenchLauncher {
    fn inspect(&self, suite: &Path) -> Result<SuiteInfo, LaunchError> {
        self.inner.inspect(suite)
    }

    fn launch(&self, mut request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
        let game = Arc::new(Game {
            index: self.shared.next_game.fetch_add(1, Ordering::Relaxed),
            sink: Arc::clone(&self.shared.sink),
            start: self.shared.start,
            inner: OnceLock::new(),
            shown: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            errors: AtomicU64::new(0),
            held: Mutex::new(Vec::new()),
        });
        // The game's own notifications tell the moment of its first frame.
        let app_notify = request.notify.take();
        let weak_game = Arc::downgrade(&game);
        let notify: Notify = Arc::new(move |id| {
            if let Some(game) = weak_game.upgrade() {
                game.check_first();
            }
            if let Some(notify) = &app_notify {
                notify(id);
            }
        });
        request.notify = Some(notify);
        let inner = self.inner.launch(request)?;
        game.inner.set(Arc::downgrade(&inner)).ok();
        game.check_first();
        self.shared.add(&game);
        Ok(Arc::new(BenchInstance { inner, game }))
    }
}

struct Shared {
    sink: Sink,
    start: Instant,
    next_game: AtomicUsize,
    games: Mutex<Vec<Weak<Game>>>,
    ticking: AtomicBool,
}

impl Shared {
    fn add(self: &Arc<Self>, game: &Arc<Game>) {
        let mut games = self.games.lock().unwrap_or_else(PoisonError::into_inner);
        games.retain(|game| game.strong_count() > 0);
        games.push(Arc::downgrade(game));
        drop(games);
        if !self.ticking.swap(true, Ordering::AcqRel) {
            let shared = Arc::downgrade(self);
            let spawned = thread::Builder::new()
                .name("bench log".to_owned())
                .spawn(move || {
                    loop {
                        thread::sleep(TICK);
                        let Some(shared) = shared.upgrade() else {
                            break;
                        };
                        shared.tick();
                    }
                });
            if spawned.is_err() {
                self.ticking.store(false, Ordering::Release);
            }
        }
    }

    fn tick(&self) {
        let games: Vec<Arc<Game>> = self
            .games
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .filter_map(Weak::upgrade)
            .collect();
        for game in games {
            game.tick();
        }
    }
}

/// One game's part of the log.
struct Game {
    index: usize,
    sink: Sink,
    start: Instant,
    /// The runtime's instance, once `launch` returned it.
    inner: OnceLock<Weak<dyn Instance>>,
    /// Whether its first frame was written.
    shown: AtomicBool,
    /// Whether its end was written.
    ended: AtomicBool,
    /// Uncaught exceptions in its log so far.
    errors: AtomicU64,
    /// Lines taken from the instance to write the end, which the app has
    /// not taken yet.
    held: Mutex<Vec<LogLine>>,
}

impl Game {
    fn write(&self, line: &str) {
        (self.sink)(line);
    }

    fn millis(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    fn instance(&self) -> Option<Arc<dyn Instance>> {
        self.inner.get().and_then(Weak::upgrade)
    }

    /// Writes the first frame the first time the instance has one.
    fn check_first(&self) {
        if self.shown.load(Ordering::Acquire) {
            return;
        }
        let Some(instance) = self.instance() else {
            return;
        };
        if instance.frame().is_some() && !self.shown.swap(true, Ordering::AcqRel) {
            self.write(&format!(
                "RUN first game={} ms={}",
                self.index,
                self.millis()
            ));
        }
    }

    /// Takes the instance's new lines and writes each one.
    fn drain(&self, instance: &dyn Instance) -> Vec<LogLine> {
        let lines = instance.drain_log();
        for line in &lines {
            if line.level == LogLevel::Err && line.text.starts_with("Exception in thread") {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
            self.write(&format!(
                "log {} {} {}",
                self.index,
                level(line.level),
                line.text.replace(['\n', '\r'], " ")
            ));
        }
        lines
    }

    fn tick(&self) {
        if self.ended.load(Ordering::Acquire) {
            return;
        }
        let Some(instance) = self.instance() else {
            return;
        };
        self.check_first();
        if instance.status().is_finished() {
            self.finish(instance.as_ref());
            return;
        }
        if self.shown.load(Ordering::Acquire)
            && let Some(frame) = instance.frame()
        {
            self.write(&format!(
                "RUN frames game={} n={} ms={} w={} h={}",
                self.index,
                frame.serial(),
                self.millis(),
                frame.width(),
                frame.height()
            ));
        }
    }

    /// Writes how the game ended, once, after the last of its lines.
    fn finish(&self, instance: &dyn Instance) {
        if self.ended.swap(true, Ordering::AcqRel) {
            return;
        }
        self.check_first();
        let lines = self.drain(instance);
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(lines);
        let status = match instance.status() {
            InstanceStatus::Crashed { message } => {
                self.write(&format!(
                    "log {} err the game crashed: {}",
                    self.index,
                    message.replace(['\n', '\r'], " ")
                ));
                "crashed"
            }
            InstanceStatus::Exited => "exited",
            // Closed by the user while it ran.
            _ => "running",
        };
        let frames = instance.frame().map_or(0, |frame| frame.serial());
        self.write(&format!(
            "RUN end game={} status={status} frames={frames} errors={} ms={}",
            self.index,
            self.errors.load(Ordering::Relaxed),
            self.millis()
        ));
    }
}

fn level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Out => "out",
        LogLevel::Err => "err",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
    }
}

/// An instance whose log lines and frames also go to the bench log.
struct BenchInstance {
    inner: Arc<dyn Instance>,
    game: Arc<Game>,
}

impl Instance for BenchInstance {
    fn id(&self) -> InstanceId {
        self.inner.id()
    }

    fn title(&self) -> String {
        self.inner.title()
    }

    fn status(&self) -> InstanceStatus {
        self.inner.status()
    }

    fn frame(&self) -> Option<Frame> {
        let frame = self.inner.frame();
        if frame.is_some() {
            self.game.check_first();
        }
        frame
    }

    fn send(&self, event: InputEvent) {
        self.inner.send(event);
    }

    fn pause(&self) {
        self.inner.pause();
    }

    fn resume(&self) {
        self.inner.resume();
    }

    fn stop(&self) {
        self.inner.stop();
    }

    fn set_volume(&self, volume: f32) {
        self.inner.set_volume(volume);
    }

    fn drain_log(&self) -> Vec<LogLine> {
        let mut lines = std::mem::take(
            &mut *self
                .game
                .held
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        lines.extend(self.game.drain(self.inner.as_ref()));
        lines
    }

    fn wants_text(&self) -> bool {
        self.inner.wants_text()
    }

    fn take_link_request(&self) -> Option<String> {
        self.inner.take_link_request()
    }
}

impl Drop for BenchInstance {
    /// A game the app closes ends its log here, if the tick has not yet.
    fn drop(&mut self) {
        self.game.finish(self.inner.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fastemu_core::{MidletInfo, default_profile};

    use super::*;

    /// An instance the test moves by hand.
    struct Scripted {
        id: InstanceId,
        status: Mutex<InstanceStatus>,
        frame: Mutex<Option<Frame>>,
        log: Mutex<Vec<LogLine>>,
        notify: Option<Notify>,
    }

    impl Scripted {
        fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
            m.lock().unwrap_or_else(PoisonError::into_inner)
        }

        fn present(&self, serial: u64) {
            *Self::lock(&self.frame) = Some(Frame::filled(240, 320, 0xFF00_0000, serial));
            if let Some(notify) = &self.notify {
                notify(self.id);
            }
        }

        fn print(&self, level: LogLevel, text: &str) {
            Self::lock(&self.log).push(LogLine {
                level,
                text: text.to_owned(),
            });
        }

        fn end(&self, status: InstanceStatus) {
            *Self::lock(&self.status) = status;
        }
    }

    impl Instance for Scripted {
        fn id(&self) -> InstanceId {
            self.id
        }
        fn title(&self) -> String {
            "Scripted".to_owned()
        }
        fn status(&self) -> InstanceStatus {
            Self::lock(&self.status).clone()
        }
        fn frame(&self) -> Option<Frame> {
            Self::lock(&self.frame).clone()
        }
        fn send(&self, _: InputEvent) {}
        fn pause(&self) {}
        fn resume(&self) {}
        fn stop(&self) {
            self.end(InstanceStatus::Exited);
        }
        fn set_volume(&self, _: f32) {}
        fn drain_log(&self) -> Vec<LogLine> {
            std::mem::take(&mut *Self::lock(&self.log))
        }
    }

    /// Launches [`Scripted`] instances and keeps them for the test.
    #[derive(Default)]
    struct ScriptedLauncher {
        launched: Mutex<Vec<Arc<Scripted>>>,
    }

    impl Launcher for ScriptedLauncher {
        fn inspect(&self, _: &Path) -> Result<SuiteInfo, LaunchError> {
            Ok(SuiteInfo {
                name: "Scripted".to_owned(),
                vendor: "FastEmu".to_owned(),
                version: "1.0".to_owned(),
                description: None,
                icon: None,
                midlets: vec![MidletInfo {
                    name: "Scripted".to_owned(),
                    icon_path: None,
                    class_name: "Scripted".to_owned(),
                }],
                properties: Vec::new(),
                jar_bytes: 0,
            })
        }

        fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
            let instance = Arc::new(Scripted {
                id: InstanceId::next(),
                status: Mutex::new(InstanceStatus::Running),
                frame: Mutex::new(None),
                log: Mutex::new(Vec::new()),
                notify: request.notify,
            });
            Scripted::lock(&self.launched).push(Arc::clone(&instance));
            Ok(instance)
        }
    }

    struct Bench {
        launcher: BenchLauncher,
        runtime: Arc<ScriptedLauncher>,
        lines: Arc<Mutex<Vec<String>>>,
        app_notified: Arc<AtomicUsize>,
    }

    impl Bench {
        fn new() -> Self {
            let runtime = Arc::new(ScriptedLauncher::default());
            let lines = Arc::new(Mutex::new(Vec::new()));
            let sink: Sink = {
                let lines = Arc::clone(&lines);
                Arc::new(move |line: &str| Scripted::lock(&lines).push(line.to_owned()))
            };
            Self {
                // The test ticks by hand, so no line comes at a moment of
                // its own.
                launcher: BenchLauncher::with_ticker(runtime.clone(), sink, false),
                runtime,
                lines,
                app_notified: Arc::new(AtomicUsize::new(0)),
            }
        }

        fn launch(&self) -> (Arc<dyn Instance>, Arc<Scripted>) {
            let notified = Arc::clone(&self.app_notified);
            let request = LaunchRequest {
                profile: default_profile(),
                notify: Some(Arc::new(move |_| {
                    notified.fetch_add(1, Ordering::Relaxed);
                })),
                ..LaunchRequest::new(PathBuf::from("game.jar"), PathBuf::from("data"))
            };
            let instance = self.launcher.launch(request).expect("launch");
            let scripted = Scripted::lock(&self.runtime.launched)
                .last()
                .cloned()
                .expect("launched");
            (instance, scripted)
        }

        /// The lines so far, with each `ms=N` made `ms=_`.
        fn lines(&self) -> Vec<String> {
            Scripted::lock(&self.lines)
                .iter()
                .map(|line| {
                    line.split(' ')
                        .map(|word| {
                            if word.starts_with("ms=") {
                                "ms=_"
                            } else {
                                word
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect()
        }
    }

    #[test]
    fn only_debug_or_verbose_turn_it_on() {
        for on in ["DEBUG", "D", "VERBOSE", "v", " debug\n"] {
            assert!(turned_on(on), "{on:?}");
        }
        for off in [
            "", "INFO", "WARN", "ERROR", "ASSERT", "SUPPRESS", "1", "true",
        ] {
            assert!(!turned_on(off), "{off:?}");
        }
    }

    #[test]
    fn a_game_writes_its_first_frame_lines_frames_and_end() {
        let bench = Bench::new();
        let (instance, game) = bench.launch();
        bench.launcher.tick();
        assert!(bench.lines().is_empty(), "nothing before the first frame");

        game.present(1);
        game.print(
            LogLevel::Out,
            "BENCH cpu.int round=0 reps=3 ms=14 first=4 check=127",
        );
        game.print(LogLevel::Err, "two\nlines");
        assert_eq!(
            instance.drain_log().len(),
            2,
            "the app still gets every line"
        );
        game.present(7);
        bench.launcher.tick();
        game.print(
            LogLevel::Err,
            "Exception in thread \"main\" java.lang.Error",
        );
        game.end(InstanceStatus::Exited);
        bench.launcher.tick();
        bench.launcher.tick();

        assert_eq!(
            bench.lines(),
            [
                "RUN first game=0 ms=_",
                "log 0 out BENCH cpu.int round=0 reps=3 ms=_ first=4 check=127",
                "log 0 err two lines",
                "RUN frames game=0 n=7 ms=_ w=240 h=320",
                "log 0 err Exception in thread \"main\" java.lang.Error",
                "RUN end game=0 status=exited frames=7 errors=1 ms=_",
            ]
        );
        // The line drained to write the end still reaches the app, once.
        let late = instance.drain_log();
        assert_eq!(late.len(), 1);
        assert!(instance.drain_log().is_empty());
        assert!(
            bench.app_notified.load(Ordering::Relaxed) >= 2,
            "the app is notified"
        );
    }

    #[test]
    fn games_are_numbered_in_the_order_they_start() {
        let bench = Bench::new();
        let (_first, a) = bench.launch();
        let (_second, b) = bench.launch();
        b.present(1);
        a.present(1);
        assert_eq!(
            bench.lines(),
            ["RUN first game=1 ms=_", "RUN first game=0 ms=_"]
        );
    }

    #[test]
    fn a_crash_and_a_closed_game_end_their_logs() {
        let bench = Bench::new();
        let (crashing, a) = bench.launch();
        let (closed, b) = bench.launch();
        a.present(3);
        a.end(InstanceStatus::Crashed {
            message: "out of\nmemory".to_owned(),
        });
        bench.launcher.tick();
        b.present(5);
        drop(closed);
        drop(crashing);
        assert_eq!(
            bench.lines(),
            [
                "RUN first game=0 ms=_",
                "log 0 err the game crashed: out of memory",
                "RUN end game=0 status=crashed frames=3 errors=0 ms=_",
                "RUN first game=1 ms=_",
                "RUN end game=1 status=running frames=5 errors=0 ms=_",
            ]
        );
    }
}
