//! The measurements of `cargo xtask bench --android`, the same way on
//! both apps.
//!
//! Each measurement stops both apps, clears what a game kept, waits for a
//! quiet guest and records its load, and reads logcat from before the
//! start. Its start is the system's `START` line for the app's activity.
//! FastEmu and J2ME Loader take turns going first, as on the desktop: in
//! run `r`, the pair of measurement `i` starts with FastEmu when `r + i`
//! is odd.

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::apps::{self, App, Suite};
use super::device::{Device, Logcat};
use super::parse::{self, LogcatLine};
use crate::bench::keys;
use crate::bench::measure::Running;
use crate::bench::report::{End, KernelRound, Work};
use crate::bench::wire::Line;

/// The longest a kernel run may take on one app.
const KERNEL_LIMIT: Duration = Duration::from_secs(600);
/// The longest a game may take to show its first frame.
const FIRST_LIMIT: Duration = Duration::from_secs(60);
/// How long past `BENCH done` a kernel run may take to end.
const END_LIMIT: Duration = Duration::from_secs(30);
/// The longest wait for a quiet guest before a measurement.
const QUIET_LIMIT: Duration = Duration::from_secs(60);
/// How long the guest's load is read for.
const QUIET_SPAN: Duration = Duration::from_secs(2);
/// The guest's load, in busy milliseconds a second over all its
/// processors, under which it counts as quiet.
pub const QUIET_LOAD: f64 = 150.0;
/// When the scene and each game are sampled, and when they end, in
/// seconds after the start: the desktop bench's times.
const ONE: ([f64; 2], f64) = ([10.0, 40.0], 45.0);
/// The same for every game at once.
const ALL: ([f64; 2], f64) = ([15.0, 55.0], 60.0);
/// How often SurfaceFlinger is asked for a game's first present.
const PRESENT_POLL: Duration = Duration::from_millis(200);

/// What one measurement found.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub work: Work,
    pub app: App,
    pub run: usize,
    pub first_app: App,
    /// From the system's start of the activity to the first frame, in ms;
    /// for every game at once, to the last game's first frame.
    pub first_ms: Option<f64>,
    /// How the first frame was seen.
    pub first_by: &'static str,
    /// Frames a second SurfaceFlinger presented for the app between the
    /// samples.
    pub fps: Option<f64>,
    /// FastEmu's own frames a second, per game, from its bench log.
    pub own_fps: Vec<Option<f64>>,
    /// The game's process: processor time a second between the samples.
    pub cpu_ms_per_s: Option<f64>,
    /// A kernel run's processor and wall time, from `BENCH screen` to
    /// `BENCH done`.
    pub cpu_ms: Option<f64>,
    pub wall_ms: Option<f64>,
    /// The game's process at the last sample: its proportional set size,
    /// and its peak resident size.
    pub pss_kib: Option<u64>,
    pub peak_kib: Option<u64>,
    /// J2ME Loader's main process, which runs beside its game process.
    pub main_pss_kib: Option<u64>,
    /// The guest's load before the start, and the load of everything else
    /// between the samples, in busy ms a second.
    pub quiet: Option<f64>,
    pub other: Option<f64>,
    pub rounds: Vec<KernelRound>,
    pub frames_free: Option<f64>,
    pub screen: Option<(i64, i64)>,
    pub clock_step: Option<i64>,
    pub kernel_errors: Vec<(String, String)>,
    pub ends: Vec<Option<End>>,
    pub shot: Option<String>,
    pub problems: Vec<String>,
}

impl Measurement {
    fn new(work: Work, app: App, run: usize, first_app: App, games: usize) -> Self {
        Self {
            work,
            app,
            run,
            first_app,
            first_ms: None,
            first_by: "",
            fps: None,
            own_fps: vec![None; games],
            cpu_ms_per_s: None,
            cpu_ms: None,
            wall_ms: None,
            pss_kib: None,
            peak_kib: None,
            main_pss_kib: None,
            quiet: None,
            other: None,
            rounds: Vec::new(),
            frames_free: None,
            screen: None,
            clock_step: None,
            kernel_errors: Vec::new(),
            ends: vec![None; games],
            shot: None,
            problems: Vec::new(),
        }
    }
}

/// What a measurement learns from logcat as it runs.
struct Seen {
    fastemu: bool,
    /// The time of the first `START` of the app's activity.
    start: Option<f64>,
    /// Each game's first frame, by FastEmu's bench log.
    first: Vec<Option<f64>>,
    /// `BENCH screen` and `BENCH done`.
    screen_at: Option<f64>,
    done_at: Option<f64>,
    /// FastEmu's frame counts: `(n, ms)` per game.
    frames: Vec<Vec<(u64, u64)>>,
    /// Lines of `System.err` with an exception, J2ME Loader's errors.
    errors: u64,
}

impl Seen {
    fn new(app: App, games: usize) -> Self {
        Self {
            fastemu: app == App::FastEmu,
            start: None,
            first: vec![None; games],
            screen_at: None,
            done_at: None,
            frames: vec![Vec::new(); games],
            errors: 0,
        }
    }

    /// Takes in one logcat line.
    fn absorb(&mut self, line: &LogcatLine, app: App, m: &mut Measurement) {
        if line.tag == "ActivityTaskManager" {
            if self.start.is_none() && parse::is_start_of(&line.msg, app.package()) {
                self.start = Some(line.time);
            }
            return;
        }
        if !self.fastemu && line.tag == "System.err" && line.msg.contains("Exception") {
            self.errors += 1;
        }
        let Some(text) = parse::bench_line(line, self.fastemu) else {
            return;
        };
        let Some(parsed) = Line::parse(&text) else {
            return;
        };
        match parsed {
            Line::First { game, .. } => {
                if let Some(slot) = self.first.get_mut(game) {
                    slot.get_or_insert(line.time);
                }
            }
            Line::Frames { game, n, ms, .. } => {
                if let Some(counts) = self.frames.get_mut(game) {
                    counts.push((n, ms));
                }
            }
            Line::End {
                game,
                status,
                frames,
                errors,
                ..
            } => {
                if let Some(slot) = m.ends.get_mut(game) {
                    *slot = Some(End {
                        status,
                        frames,
                        errors,
                    });
                }
            }
            Line::Bench { name, fields, .. } => {
                let get = |k: &str| fields.get(k).copied();
                match name.as_str() {
                    "screen" => {
                        m.screen = get("w").zip(get("h"));
                        self.screen_at.get_or_insert(line.time);
                    }
                    "clock" => m.clock_step = get("step"),
                    "frames.free" => {
                        if let (Some(n), Some(ms)) = (get("frames"), get("ms"))
                            && ms > 0
                        {
                            m.frames_free = Some(n as f64 * 1000.0 / ms as f64);
                        }
                    }
                    "done" => {
                        self.done_at.get_or_insert(line.time);
                    }
                    _ => {
                        if let (Some(round), Some(reps), Some(ms), Some(first), Some(check)) = (
                            get("round"),
                            get("reps"),
                            get("ms"),
                            get("first"),
                            get("check"),
                        ) {
                            m.rounds.push(KernelRound {
                                name,
                                round,
                                reps,
                                ms,
                                first,
                                check,
                            });
                        }
                    }
                }
            }
            Line::BenchError { name, what, .. } => m.kernel_errors.push((name, what)),
            Line::Version { .. } | Line::Log { .. } | Line::Shot { .. } => {}
        }
    }

    /// FastEmu's own frames a second for `game`, between the counts
    /// nearest after `from_ms` and before `to_ms` of its log's clock.
    fn own_fps(&self, game: usize, from_ms: u64, to_ms: u64) -> Option<f64> {
        let counts = self.frames.get(game)?;
        let a = counts.iter().find(|&&(_, ms)| ms >= from_ms)?;
        let b = counts.iter().rev().find(|&&(_, ms)| ms <= to_ms)?;
        (b.1 > a.1).then(|| (b.0.saturating_sub(a.0)) as f64 * 1000.0 / (b.1 - a.1) as f64)
    }
}

/// One sample of the game's process and the guest.
#[derive(Clone, Copy)]
struct Sample {
    at: Instant,
    cpu_ms: Option<f64>,
    guest: Option<(u64, u64)>,
    /// The bench log's clock at the sample: the latest `ms` it wrote.
    log_ms: u64,
}

/// Everything the measurements share.
pub struct Bench<'a> {
    pub device: &'a Device,
    pub logs: &'a Path,
    pub shots: &'a Path,
    pub key_script: String,
}

impl Bench<'_> {
    /// Waits up to [`QUIET_LIMIT`] for the guest to be quiet, and returns
    /// its load, in busy ms a second, as last read.
    fn wait_quiet(&self) -> Option<f64> {
        let until = Instant::now() + QUIET_LIMIT;
        loop {
            let a = self.device.guest_jiffies()?;
            let t = Instant::now();
            thread::sleep(QUIET_SPAN);
            let b = self.device.guest_jiffies()?;
            let seconds = t.elapsed().as_secs_f64();
            let busy_ms = b.0.saturating_sub(a.0) as f64 * 10.0;
            let now = busy_ms / seconds;
            if now < QUIET_LOAD || Instant::now() > until {
                return Some(now);
            }
        }
    }

    fn sample(&self, app: App, log_ms: u64) -> Sample {
        let cpu_ms = self
            .device
            .pid(app.game_process())
            .and_then(|pid| self.device.cpu_ms(pid));
        Sample {
            at: Instant::now(),
            cpu_ms,
            guest: self.device.guest_jiffies(),
            log_ms,
        }
    }

    /// Reads `/proc` and `dumpsys meminfo` of the game's process, and of
    /// J2ME Loader's main process.
    fn memory(&self, app: App, m: &mut Measurement) {
        if let Some(pid) = self.device.pid(app.game_process()) {
            m.pss_kib = parse::pss_kib(&self.device.shell_any(&format!("dumpsys meminfo {pid}")));
            m.peak_kib = parse::status_kib(
                &self.device.shell_any(&format!("cat /proc/{pid}/status")),
                "VmHWM",
            );
        }
        if app == App::J2meLoader
            && let Some(pid) = self.device.pid(apps::J2ML)
        {
            m.main_pss_kib =
                parse::pss_kib(&self.device.shell_any(&format!("dumpsys meminfo {pid}")));
        }
    }

    /// Fills in the rates between two samples.
    fn rates(m: &mut Measurement, a: Sample, b: Sample) {
        let seconds = (b.at - a.at).as_secs_f64();
        if seconds <= 0.0 {
            return;
        }
        if let (Some(x), Some(y)) = (a.cpu_ms, b.cpu_ms) {
            m.cpu_ms_per_s = Some((y - x) / seconds);
        }
        if let (Some(x), Some(y)) = (a.guest, b.guest) {
            let busy = y.0.saturating_sub(x.0) as f64 * 10.0 / seconds;
            m.other = Some((busy - m.cpu_ms_per_s.unwrap_or(0.0)).max(0.0));
        }
    }

    /// Runs one measurement of `work` on `app`, with `suites` started in
    /// order (more than one only for every game at once).
    pub fn measure(
        &self,
        work: &Work,
        app: App,
        run: usize,
        first_app: App,
        suites: &[&Suite],
    ) -> Measurement {
        let mut m = Measurement::new(work.clone(), app, run, first_app, suites.len());
        if let Err(problem) = self.run_one(work, suites, &mut m) {
            m.problems.push(problem);
        }
        // Nothing of this measurement runs on into the next.
        let _ = apps::reset(self.device, &[]);
        m
    }

    fn run_one(&self, work: &Work, suites: &[&Suite], m: &mut Measurement) -> Result<(), String> {
        let app = m.app;
        apps::reset(self.device, suites)?;
        m.quiet = self.wait_quiet();
        let tag = format!("{}-{}-r{}", work.key(), app.key(), m.run);
        let logcat = self.device.logcat(&self.logs.join(format!("{tag}.log")))?;
        let mut seen = Seen::new(app, suites.len());
        let real_game = matches!(work, Work::Game(_) | Work::Eight);
        let watch_present = app == App::J2meLoader && real_game;
        if watch_present {
            self.device
                .shell_any("dumpsys SurfaceFlinger --timestats -disable -clear -enable");
        }
        let started = Instant::now();
        for suite in suites {
            apps::start(self.device, app, suite)?;
        }
        match work {
            Work::Kernels => self.kernels(&logcat, &mut seen, m),
            _ => {
                let (at, end) = if matches!(work, Work::Eight) {
                    ALL
                } else {
                    ONE
                };
                self.timed(&logcat, &mut seen, m, started, at, end, real_game, &tag)
            }
        }
    }

    /// Reads logcat until `until` or until `stop` says so.
    fn read_until(
        logcat: &Logcat,
        seen: &mut Seen,
        m: &mut Measurement,
        until: Instant,
        mut stop: impl FnMut(&Seen) -> bool,
    ) {
        while !stop(seen) {
            let Some(line) = logcat.next_until(until) else {
                return;
            };
            seen.absorb(&line, m.app, m);
        }
    }

    /// The bench MIDlet's kernels: sampled at `BENCH screen` and at
    /// `BENCH done`, ending with the game.
    fn kernels(&self, logcat: &Logcat, seen: &mut Seen, m: &mut Measurement) -> Result<(), String> {
        let start = Instant::now();
        Self::read_until(logcat, seen, m, start + FIRST_LIMIT, |s| {
            s.screen_at.is_some()
        });
        let Some(screen_at) = seen.screen_at else {
            return Err("no BENCH screen line: the MIDlet did not paint".into());
        };
        let first = self.sample(m.app, 0);
        m.first_by = "BENCH screen";
        m.first_ms = seen.start.map(|s| (screen_at - s) * 1000.0);
        Self::read_until(logcat, seen, m, start + KERNEL_LIMIT, |s| {
            s.done_at.is_some()
        });
        let Some(done_at) = seen.done_at else {
            return Err(format!(
                "no BENCH done line in {} s",
                KERNEL_LIMIT.as_secs()
            ));
        };
        let done = self.sample(m.app, 0);
        if let (Some(a), Some(b)) = (first.cpu_ms, done.cpu_ms) {
            m.cpu_ms = Some(b - a);
        }
        m.wall_ms = Some((done_at - screen_at) * 1000.0);
        self.memory(m.app, m);
        // The MIDlet lingers, then ends itself.
        let until = Instant::now() + END_LIMIT;
        match m.app {
            App::FastEmu => {
                Self::read_until(logcat, seen, m, until, |_| false);
                let ended = m.ends.first().is_some_and(Option::is_some);
                if !ended {
                    m.problems.push("the MIDlet did not end".into());
                }
            }
            App::J2meLoader => {
                while Instant::now() < until && self.device.pid(apps::J2ML_GAME_PROCESS).is_some() {
                    Self::read_until(
                        logcat,
                        seen,
                        m,
                        Instant::now() + Duration::from_millis(500),
                        |_| false,
                    );
                }
                let gone = self.device.pid(apps::J2ML_GAME_PROCESS).is_none();
                m.ends = vec![Some(End {
                    status: if gone { "exited" } else { "running" }.into(),
                    frames: 0,
                    errors: seen.errors,
                })];
            }
        }
        Ok(())
    }

    /// The first present on `app`'s game surface, in seconds of the
    /// monotonic clock, not before `after`: SurfaceFlinger keeps the last
    /// 127 presents of a layer, so it is asked soon after the game starts
    /// drawing.
    fn first_present(&self, app: App, after: f64) -> Option<f64> {
        let list = self.device.shell_any("dumpsys SurfaceFlinger --list");
        parse::surface_layers(&list, app.package())
            .iter()
            .flat_map(|layer| {
                parse::presents(&self.device.shell_any(&format!(
                    "dumpsys SurfaceFlinger --latency {}",
                    parse::shell_quote(layer)
                )))
            })
            .filter(|&t| t >= after)
            .reduce(f64::min)
    }

    /// The scene, a game or every game at once: sampled at `at`, ended at
    /// `end` seconds after the start.
    #[allow(clippy::too_many_arguments)]
    fn timed(
        &self,
        logcat: &Logcat,
        seen: &mut Seen,
        m: &mut Measurement,
        started: Instant,
        at: [f64; 2],
        end: f64,
        real_game: bool,
        tag: &str,
    ) -> Result<(), String> {
        let app = m.app;
        let games = m.ends.len();
        let secs = |s: f64| started + Duration::from_secs_f64(s);
        let mut keys: Vec<Running> = Vec::new();
        let mut keyed = vec![false; games];
        // One game: its first present on screen. FastEmu's surface shows
        // its library before the game, so its first present counts from
        // the game's first frame in its bench log.
        let one_game = real_game && games == 1;
        let mut present: Option<f64> = None;
        let mut next_poll = Instant::now();
        // Up to the first sample: the first frames, and the keys of each
        // game from its first frame.
        while Instant::now() < secs(at[0]) {
            let polling = one_game && present.is_none();
            let step = if polling {
                next_poll.min(secs(at[0]))
            } else {
                secs(at[0])
            };
            // A game's first frame stops the read, so its keys start then.
            Self::read_until(logcat, seen, m, step, |s| {
                real_game && (0..games).any(|g| s.first[g].is_some() && !keyed[g])
            });
            if polling && Instant::now() >= next_poll {
                let after = match app {
                    App::FastEmu => seen.first[0],
                    App::J2meLoader => seen.start,
                };
                if let Some(after) = after {
                    present = self.first_present(app, after);
                }
                next_poll = Instant::now() + PRESENT_POLL;
            }
            if real_game {
                for (game, keyed) in keyed.iter_mut().enumerate() {
                    let shown = match app {
                        App::FastEmu => seen.first[game].is_some(),
                        App::J2meLoader => present.is_some(),
                    };
                    if shown && !*keyed {
                        *keyed = true;
                        keys.push(self.press_keys()?);
                    }
                }
            }
        }
        // The first frame.
        if !real_game {
            m.first_by = "BENCH screen";
            m.first_ms = seen
                .start
                .zip(seen.screen_at)
                .map(|(s, f)| (f - s) * 1000.0);
        } else if one_game {
            m.first_by = "first present on screen";
            m.first_ms = seen.start.zip(present).map(|(s, f)| (f - s) * 1000.0);
        } else {
            m.first_by = "last game's first frame in its bench log";
            if seen.first.iter().all(Option::is_some) {
                let last = seen
                    .first
                    .iter()
                    .flatten()
                    .copied()
                    .fold(f64::MIN, f64::max);
                m.first_ms = seen.start.map(|s| (last - s) * 1000.0);
            }
        }
        if (0..games).any(|g| m.ends[g].is_some()) {
            m.problems
                .push("a game ended before the first sample".into());
        }
        let log_ms = |s: &Seen| {
            s.frames
                .iter()
                .filter_map(|c| c.last().map(|&(_, ms)| ms))
                .max()
                .unwrap_or(0)
        };
        let a = self.sample(app, log_ms(seen));
        self.device
            .shell_any("dumpsys SurfaceFlinger --timestats -disable -clear -enable");
        Self::read_until(logcat, seen, m, secs(at[1]), |_| false);
        let b = self.sample(app, log_ms(seen));
        let dump = self
            .device
            .shell_any("dumpsys SurfaceFlinger --timestats -dump");
        self.device
            .shell_any("dumpsys SurfaceFlinger --timestats -disable");
        Self::rates(m, a, b);
        let seconds = (b.at - a.at).as_secs_f64();
        if let Some(frames) = parse::package_frames(&parse::timestats(&dump), app.package())
            && seconds > 0.0
        {
            m.fps = Some(frames as f64 / seconds);
        }
        if app == App::FastEmu {
            for game in 0..games {
                m.own_fps[game] = seen.own_fps(game, a.log_ms, b.log_ms);
            }
        }
        self.memory(app, m);
        Self::read_until(logcat, seen, m, secs(at[1] + 1.0), |_| false);
        let shot = self.shots.join(format!("{tag}.png"));
        match self.device.screenshot(&shot) {
            Ok(()) => m.shot = Some(shot.display().to_string()),
            Err(problem) => m.problems.push(problem),
        }
        Self::read_until(logcat, seen, m, secs(end), |_| false);
        drop(keys);
        if app == App::J2meLoader {
            let running = self.device.pid(apps::J2ML_GAME_PROCESS).is_some();
            m.ends = vec![
                Some(End {
                    status: if running { "running" } else { "exited" }.into(),
                    frames: 0,
                    errors: seen.errors,
                });
                games
            ];
        } else {
            // The bench log writes a game's end when it ends; one that ran
            // to the end is still running.
            for slot in &mut m.ends {
                if slot.is_none() {
                    *slot = Some(End {
                        status: "running".into(),
                        frames: 0,
                        errors: 0,
                    });
                }
            }
        }
        Ok(())
    }

    /// Starts pressing the bench's keys, on the device, from now.
    fn press_keys(&self) -> Result<Running, String> {
        let child = Command::new(self.device_adb())
            .args(["-s", &self.device.serial, "shell", &self.key_script])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("adb shell keys: {e}"))?;
        Ok(Running::new(child))
    }

    fn device_adb(&self) -> &Path {
        self.device.adb()
    }
}

/// The key script of [`keys::BENCH_KEYS`].
pub fn bench_key_script() -> Result<String, String> {
    apps::key_script(&keys::steps(keys::BENCH_KEYS)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(time: f64, tag: &str, msg: &str) -> LogcatLine {
        LogcatLine {
            time,
            pid: 1,
            tid: 1,
            level: 'I',
            tag: tag.into(),
            msg: msg.into(),
        }
    }

    #[test]
    fn fastemu_lines_fill_a_measurement() {
        let mut m = Measurement::new(Work::Kernels, App::FastEmu, 0, App::FastEmu, 1);
        let mut seen = Seen::new(App::FastEmu, 1);
        let lines = [
            line(
                1.0,
                "ActivityTaskManager",
                "START u0 {act=android.intent.action.VIEW cmp=io.github.fzn0x.fastemu/.MainActivity}",
            ),
            line(
                1.5,
                "fastemu",
                "fastemu_android::android::app: FastEmuBench: RUN first game=0 ms=400",
            ),
            line(
                1.6,
                "fastemu",
                "fastemu_android::android::app: FastEmuBench: log 0 out BENCH screen w=240 h=320",
            ),
            line(
                1.6,
                "fastemu",
                "x: FastEmuBench: log 0 out BENCH clock step=1",
            ),
            line(
                1.7,
                "fastemu",
                "x: FastEmuBench: log 0 out BENCH cpu.int round=2 reps=3 ms=30 first=9 check=7",
            ),
            line(
                1.8,
                "fastemu",
                "x: FastEmuBench: RUN frames game=0 n=60 ms=1000 w=240 h=320",
            ),
            line(
                2.8,
                "fastemu",
                "x: FastEmuBench: RUN frames game=0 n=120 ms=2000 w=240 h=320",
            ),
            line(
                2.9,
                "fastemu",
                "x: FastEmuBench: log 0 out BENCH error name=mem.gc what=x",
            ),
            line(
                3.0,
                "fastemu",
                "x: FastEmuBench: log 0 out BENCH done rounds=2",
            ),
            line(
                4.0,
                "fastemu",
                "x: FastEmuBench: RUN end game=0 status=exited frames=130 errors=0 ms=3000",
            ),
            // Another app's start and J2ME Loader's lines are not FastEmu's.
            line(4.1, "System.out", "BENCH done rounds=9"),
        ];
        for l in &lines {
            seen.absorb(l, App::FastEmu, &mut m);
        }
        assert_eq!(seen.start, Some(1.0));
        assert_eq!(seen.first[0], Some(1.5));
        assert_eq!(seen.screen_at, Some(1.6));
        assert_eq!(seen.done_at, Some(3.0));
        assert_eq!(m.screen, Some((240, 320)));
        assert_eq!(m.clock_step, Some(1));
        assert_eq!(m.rounds.len(), 1);
        assert_eq!(m.rounds[0].ms, 30);
        assert_eq!(m.kernel_errors, [("mem.gc".to_owned(), "x".to_owned())]);
        assert_eq!(m.ends[0].as_ref().unwrap().frames, 130);
        assert_eq!(seen.own_fps(0, 900, 2100), Some(60.0));
        assert_eq!(seen.own_fps(0, 1500, 2100), None);
    }

    #[test]
    fn j2me_loader_lines_fill_a_measurement() {
        let mut m = Measurement::new(Work::Kernels, App::J2meLoader, 1, App::FastEmu, 1);
        let mut seen = Seen::new(App::J2meLoader, 1);
        for l in [
            line(
                5.0,
                "ActivityTaskManager",
                "START u0 {act=android.intent.action.VIEW cmp=io.github.fzn0x.fastemu/.MainActivity}",
            ),
            line(
                6.0,
                "ActivityTaskManager",
                "START u0 {act=android.intent.action.VIEW dat=/storage/x cmp=ru.playsoftware.j2meloader/.config.ConfigActivity (has extras)}",
            ),
            line(6.5, "System.out", "BENCH screen w=240 h=294"),
            line(
                6.6,
                "System.out",
                "BENCH mem.small round=1 reps=2 ms=20 first=11 check=5",
            ),
            line(6.7, "System.err", "java.lang.RuntimeException: x"),
            line(
                6.8,
                "fastemu",
                "x: FastEmuBench: log 0 out BENCH done rounds=1",
            ),
        ] {
            seen.absorb(&l, App::J2meLoader, &mut m);
        }
        assert_eq!(seen.start, Some(6.0));
        assert_eq!(seen.screen_at, Some(6.5));
        assert_eq!(seen.done_at, None);
        assert_eq!(m.screen, Some((240, 294)));
        assert_eq!(m.rounds[0].first, 11);
        assert_eq!(seen.errors, 1);
    }

    #[test]
    fn the_key_script_builds() {
        assert!(bench_key_script().unwrap().contains("input keyevent 66"));
    }
}
