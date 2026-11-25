//! The measurements of `cargo xtask bench`: which processes start, when
//! the sampler reads them, and what each measurement records.
//!
//! Every measurement waits for a quiet machine first and records the
//! load. FastEmu and MicroEmulator take turns going first: in run `r`,
//! the pair of measurement `i` starts with FastEmu when `r + i` is odd,
//! so over an even number of runs each goes first equally often. Every
//! process sits in a [`Running`] guard that kills it by its process id
//! when the measurement ends, on an error too.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use fastemu_core::default_profile;

use super::child;
use super::java::{Built, MICROEMU_JARS};
use super::keys::{self, BENCH_KEYS};
use super::report::{End, GameInfo, KernelRound, Measurement, NOISE_MARGIN, Report, Side, Work};
use super::sample::{self, Sample, Sampler};
use super::windowed;
use super::wire::Line;
use super::{Options, slug};

/// The phone both emulators show: 240x320, Nokia's key codes.
const PROFILE: &str = "generic-240x320";
/// The longest a kernel run may take on one emulator.
const KERNEL_LIMIT: Duration = Duration::from_secs(600);
/// How long past its end a timed measurement waits for its processes.
const GRACE: Duration = Duration::from_secs(30);
/// The longest wait for a quiet machine before a measurement.
const QUIET_LIMIT: Duration = Duration::from_secs(60);
/// How long the load must stay low.
const QUIET_SPAN: Duration = Duration::from_secs(2);
/// How long the idle baseline is measured.
const BASELINE: Duration = Duration::from_secs(20);
/// The heap of MicroEmulator's small-heap rows.
const SMALL_HEAP: [&str; 3] = ["-Xms16m", "-Xmx128m", "-XX:+UseSerialGC"];

/// A process the bench started. Dropping it kills the process by its id
/// if it still runs, and waits for it.
pub struct Running {
    child: Child,
}

impl Running {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// When a measurement samples and ends.
#[derive(Clone, Copy, Debug)]
enum Timing {
    /// At the first frame and at `BENCH done`, ending with the game.
    Kernels,
    /// At two times after the start, ending at `end` (the runners end
    /// themselves then) or, if `wait_end` is false, killed at `end`.
    Timed {
        at: [f64; 2],
        end: f64,
        wait_end: bool,
    },
}

/// One process of a measurement, and the index of its first game.
struct Proc {
    command: Command,
    game: usize,
}

/// One measurement, before it runs.
struct Plan {
    work: Work,
    side: Side,
    run: usize,
    first_side: Side,
    tag: String,
    procs: Vec<Proc>,
    games: usize,
    timing: Timing,
}

/// What the parent knows of one game while it runs.
#[derive(Default, Clone)]
struct GameState {
    first: Option<Instant>,
    frames: Vec<(u64, u64)>,
    size: Option<(i32, i32)>,
    end: Option<End>,
}

/// Everything the measurements share.
pub struct Bench<'a> {
    options: &'a Options,
    built: &'a Built,
    root: &'a Path,
    sampler: Sampler,
    exe: PathBuf,
    java: PathBuf,
    classpath: Option<std::ffi::OsString>,
    games: Vec<(GameInfo, PathBuf)>,
    events: String,
    baseline: f64,
    threshold: f64,
}

/// The JARs of `dir`, not its subfolders, sorted.
pub fn game_jars(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut jars: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("jar")))
        .collect();
    jars.sort();
    if jars.is_empty() {
        return Err(format!("no .jar in {}", dir.display()));
    }
    Ok(jars)
}

/// The arguments of one MeBench process after `java`.
#[allow(clippy::too_many_arguments)]
pub fn microemu_args(
    options: &Options,
    side: Side,
    home: &Path,
    classpath: &std::ffi::OsStr,
    seconds: f64,
    events: &str,
    shot: Option<(f64, &Path)>,
    suite: &Path,
) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    if side == Side::MicroEmuSmallHeap {
        args.extend(SMALL_HEAP.iter().map(Into::into));
    }
    args.extend(options.me_jvm_args.iter().map(Into::into));
    if !options.me_no_headless_awt {
        args.push("-Djava.awt.headless=true".into());
    }
    let mut user_home = std::ffi::OsString::from("-Duser.home=");
    user_home.push(home);
    args.push(user_home);
    args.push("-cp".into());
    args.push(classpath.into());
    args.push("fastemu.bench.MeBench".into());
    args.push("--seconds".into());
    args.push(seconds.to_string().into());
    if !events.is_empty() {
        args.push("--events".into());
        args.push(events.into());
    }
    if side == Side::MicroEmuCapped
        && let Some(cap) = options.me_max_fps
    {
        args.push("--max-fps".into());
        args.push(cap.to_string().into());
    }
    if let Some((at, path)) = shot {
        args.push("--shot-at".into());
        args.push(at.to_string().into());
        args.push("--shot".into());
        args.push(path.into());
    }
    for arg in ["--resizableDevice", "240", "320"] {
        args.push(arg.into());
    }
    args.push(suite.into());
    args
}

/// Runs a command and returns its standard output, trimmed, or its
/// standard error when it printed nothing else (as `java -version`).
fn output(command: &mut Command) -> String {
    command.stdin(Stdio::null()).output().map_or_else(
        |e| format!("n/a ({e})"),
        |o| {
            let out = String::from_utf8_lossy(&o.stdout);
            let text = if out.trim().is_empty() {
                String::from_utf8_lossy(&o.stderr)
            } else {
                out
            };
            text.trim().to_owned()
        },
    )
}

const MACHINE: &str = "$p = Get-CimInstance Win32_Processor | Select-Object -First 1; \
$c = Get-CimInstance Win32_ComputerSystem; $o = Get-CimInstance Win32_OperatingSystem; \
'date ' + (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd HH:mm') + ' UTC'; \
'cpu ' + $p.Name.Trim(); \
'cores ' + $p.NumberOfCores + ' cores, ' + $c.NumberOfLogicalProcessors + ' logical processors'; \
'ram ' + [math]::Round($c.TotalPhysicalMemory / 1GB, 1) + ' GB'; \
'os ' + $o.Caption + ', build ' + $o.BuildNumber";

impl<'a> Bench<'a> {
    pub fn new(options: &'a Options, built: &'a Built, root: &'a Path) -> Result<Self, String> {
        let exe = std::env::current_exe().map_err(|e| format!("finding this program: {e}"))?;
        // FastEmu alone starts no java at all.
        let java = match (&options.me_java, options.fastemu_only) {
            (_, true) => PathBuf::new(),
            (Some(java), false) => java.clone(),
            (None, false) => {
                crate::api::stubs::jdk_tool("java").ok_or("no java in JAVA_HOME or on the PATH")?
            }
        };
        let classpath = match &options.microemu {
            Some(_) if options.fastemu_only => None,
            Some(dir) => {
                if !built.mebench_real {
                    return Err("MeBench was not built against microemulator.jar".into());
                }
                let mut parts: Vec<PathBuf> = MICROEMU_JARS.iter().map(|j| dir.join(j)).collect();
                if let Some(missing) = parts.iter().find(|p| !p.is_file()) {
                    return Err(format!("{} is missing", missing.display()));
                }
                parts.push(built.mebench.clone());
                Some(std::env::join_paths(parts).map_err(|e| e.to_string())?)
            }
            None => None,
        };
        let mut games = Vec::new();
        if let Some(dir) = &options.games {
            let mut taken = Vec::new();
            for jar in game_jars(dir)? {
                let file = jar
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                games.push((
                    GameInfo {
                        slug: slug(&jar, &mut taken),
                        file,
                    },
                    jar,
                ));
            }
        }
        for sub in ["logs", "shots", "data", "me-home"] {
            let dir = options.out.join(sub);
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let sampler = Sampler::start(&options.out)?;
        Ok(Self {
            options,
            built,
            root,
            sampler,
            exe,
            java,
            classpath,
            games,
            events: keys::timeline(BENCH_KEYS, &default_profile().key_codes)?,
            baseline: 0.0,
            threshold: f64::INFINITY,
        })
    }

    /// The machine, the versions and the settings, for the report.
    fn facts(&mut self) -> Vec<(String, String)> {
        let mut facts = Vec::new();
        let mut machine = Command::new("powershell");
        machine.args(["-NoProfile", "-NonInteractive", "-Command", MACHINE]);
        let text = output(&mut machine);
        for (key, label) in [
            ("date", "Date"),
            ("cpu", "Processor"),
            ("cores", "Cores"),
            ("ram", "Memory"),
            ("os", "Windows"),
        ] {
            let value = text
                .lines()
                .find_map(|l| l.strip_prefix(&format!("{key} ")))
                .unwrap_or("n/a");
            facts.push((label.to_owned(), value.trim().to_owned()));
        }
        facts.push((
            "Power plan".into(),
            output(Command::new("powercfg").arg("/getactivescheme")),
        ));
        let commit = output(
            Command::new("git")
                .arg("-C")
                .arg(self.root)
                .args(["rev-parse", "HEAD"]),
        );
        let dirty = !output(Command::new("git").arg("-C").arg(self.root).args([
            "status",
            "--porcelain",
            "--untracked-files=no",
        ]))
        .is_empty();
        facts.push((
            "FastEmu".into(),
            format!(
                "commit {commit}{}, release build, headless, sound engine on a silent output",
                if dirty {
                    " with uncommitted changes"
                } else {
                    ""
                }
            ),
        ));
        if !self.options.fastemu_only {
            let java_version = output(Command::new(&self.java).arg("-version"));
            facts.push((
                "Java".into(),
                java_version.lines().collect::<Vec<_>>().join("; "),
            ));
            if let Some(classpath) = &self.classpath {
                let mut version = Command::new(&self.java);
                version
                    .arg("-cp")
                    .arg(classpath)
                    .args(["fastemu.bench.MeBench", "--version"]);
                let said = output(&mut version);
                let me = said
                    .lines()
                    .find_map(|l| match Line::parse(l) {
                        Some(Line::Version { microemulator }) => Some(microemulator),
                        _ => None,
                    })
                    .unwrap_or_else(|| format!("n/a ({said})"));
                facts.push((
                    "MicroEmulator".into(),
                    format!(
                        "{me}, headless through MeBench, class path {}",
                        MICROEMU_JARS.join(", ")
                    ),
                ));
            }
            let mut jvm = vec!["the JVM's defaults".to_owned()];
            jvm.extend(self.options.me_jvm_args.iter().cloned());
            jvm.push(if self.options.me_no_headless_awt {
                "without java.awt.headless".into()
            } else {
                "-Djava.awt.headless=true".into()
            });
            facts.push(("MicroEmulator's JVM options".into(), jvm.join(", ")));
            facts.push(("MicroEmulator, small heap".into(), SMALL_HEAP.join(" ")));
            if let Some(cap) = self.options.me_max_fps {
                facts.push((
                    "MicroEmulator, capped".into(),
                    format!("at {cap} frames a second (EventDispatcher.maxFps)"),
                ));
            }
        }
        facts.push((
            "Sampler".into(),
            format!(
                "a busy thread runs {:.3} GHz of cycles",
                self.sampler.rate / 1e9
            ),
        ));
        facts.push(("Runs".into(), self.options.runs.to_string()));
        facts.push(("Phone".into(), format!("{PROFILE}, Nokia key codes")));
        facts.push(("Keys in real games".into(), BENCH_KEYS.into()));
        facts
    }

    /// The machine's busy CPU per second while nothing is measured.
    fn load(&mut self, span: Duration) -> Result<(f64, Sample, Sample), String> {
        let a = self.sampler.sample(&[])?;
        std::thread::sleep(span);
        let b = self.sampler.sample(&[])?;
        Ok((sample::system_busy(&a, &b), a, b))
    }

    /// Waits until the load stays under the threshold for a moment, at
    /// most [`QUIET_LIMIT`], and returns it.
    fn wait_quiet(&mut self) -> Option<f64> {
        let deadline = Instant::now() + QUIET_LIMIT;
        loop {
            let (load, _, _) = self.load(QUIET_SPAN).ok()?;
            if load <= self.threshold || Instant::now() >= deadline {
                return Some(load);
            }
        }
    }

    fn fastemu(
        &self,
        tag: &str,
        suites: Vec<PathBuf>,
        seconds: f64,
        max_fps: Option<u32>,
        keys: &str,
        shot_at: Option<f64>,
    ) -> Proc {
        let options = child::Options {
            suites,
            seconds,
            max_fps,
            keys: keys.to_owned(),
            profile: PROFILE.into(),
            data: self.options.out.join("data").join(tag),
            shot_at,
            shots: shot_at.map(|_| self.options.out.join("shots").join(tag)),
        };
        let mut command = Command::new(&self.exe);
        command.arg("bench-run").args(options.args());
        Proc { command, game: 0 }
    }

    #[allow(clippy::too_many_arguments)]
    fn microemu(
        &self,
        side: Side,
        tag: &str,
        game: usize,
        suite: &Path,
        seconds: f64,
        events: &str,
        shot_at: Option<f64>,
    ) -> Proc {
        let home = self
            .options
            .out
            .join("me-home")
            .join(format!("{tag}-{game}"));
        let _ = fs::create_dir_all(&home);
        let shots = self.options.out.join("shots").join(tag);
        let _ = fs::create_dir_all(&shots);
        let shot_path = shots.join(format!("game-{game}.png"));
        let classpath = self.classpath.clone().unwrap_or_default();
        let mut command = Command::new(&self.java);
        command.args(microemu_args(
            self.options,
            side,
            &home,
            &classpath,
            seconds,
            events,
            shot_at.map(|at| (at, shot_path.as_path())),
            suite,
        ));
        Proc { command, game }
    }

    /// The plan of `work` on `side`.
    fn plan(&self, work: &Work, side: Side, run: usize, first_side: Side) -> Plan {
        let tag = format!("{}-{}-r{run}", work.key(), side.key());
        let timed = |at: [f64; 2], end: f64| Timing::Timed {
            at,
            end,
            wait_end: true,
        };
        let me = side != Side::FastEmu;
        let (procs, games, timing) = match work {
            Work::Kernels => {
                let seconds = KERNEL_LIMIT.as_secs_f64();
                let proc = if me {
                    self.microemu(side, &tag, 0, &self.built.bench_jar, seconds, "", None)
                } else {
                    self.fastemu(
                        &tag,
                        vec![self.built.bench_jar.clone()],
                        seconds,
                        Some(0),
                        "",
                        None,
                    )
                };
                (vec![proc], 1, Timing::Kernels)
            }
            Work::Scene => {
                let proc = if me {
                    self.microemu(side, &tag, 0, &self.built.scene_jar, 45.0, "", Some(40.0))
                } else {
                    self.fastemu(
                        &tag,
                        vec![self.built.scene_jar.clone()],
                        45.0,
                        None,
                        "",
                        Some(40.0),
                    )
                };
                (vec![proc], 1, timed([10.0, 40.0], 45.0))
            }
            Work::Game(slug) => {
                let jar = self
                    .games
                    .iter()
                    .find(|(g, _)| &g.slug == slug)
                    .map(|(_, j)| j.clone())
                    .unwrap_or_default();
                let proc = if me {
                    self.microemu(side, &tag, 0, &jar, 45.0, &self.events, Some(40.0))
                } else {
                    self.fastemu(&tag, vec![jar], 45.0, None, BENCH_KEYS, Some(40.0))
                };
                (vec![proc], 1, timed([10.0, 40.0], 45.0))
            }
            Work::Eight => {
                let jars: Vec<PathBuf> = self.games.iter().map(|(_, j)| j.clone()).collect();
                let procs = if me {
                    jars.iter()
                        .enumerate()
                        .map(|(i, jar)| {
                            self.microemu(side, &tag, i, jar, 60.0, &self.events, Some(55.0))
                        })
                        .collect()
                } else {
                    vec![self.fastemu(&tag, jars.clone(), 60.0, None, BENCH_KEYS, Some(55.0))]
                };
                (procs, jars.len(), timed([15.0, 55.0], 60.0))
            }
            Work::Windowed => {
                let jars: Vec<PathBuf> = self.games.iter().map(|(_, j)| j.clone()).collect();
                let procs = if me {
                    jars.iter()
                        .enumerate()
                        .map(|(i, jar)| {
                            let home = self.options.out.join("me-home").join(format!("{tag}-{i}"));
                            let _ = fs::create_dir_all(&home);
                            Proc {
                                command: windowed::microemu_window(
                                    &self.java,
                                    self.options,
                                    &home,
                                    jar,
                                ),
                                game: i,
                            }
                        })
                        .collect()
                } else {
                    let mut command = Command::new(self.options.app.clone().unwrap_or_default());
                    command.args(&jars);
                    vec![Proc { command, game: 0 }]
                };
                (
                    procs,
                    jars.len(),
                    Timing::Timed {
                        at: [15.0, 75.0],
                        end: 75.0,
                        wait_end: false,
                    },
                )
            }
        };
        Plan {
            work: work.clone(),
            side,
            run,
            first_side,
            tag,
            procs,
            games,
            timing,
        }
    }

    /// Runs one measurement.
    fn measure(&mut self, plan: Plan) -> Measurement {
        let mut m = Measurement::new(plan.work.clone(), plan.side, plan.run, plan.first_side);
        eprintln!(
            "bench: {} run {}, {}",
            plan.work.key(),
            plan.run,
            plan.side.label()
        );
        m.quiet = self.wait_quiet();
        let logs = self.options.out.join("logs");
        let (send, lines) = mpsc::channel::<(usize, Instant, String)>();
        let mut running = Vec::new();
        let start = Instant::now();
        let mut bases = Vec::new();
        for (i, mut proc) in plan.procs.into_iter().enumerate() {
            let err_path = logs.join(format!("{}-{i}.err", plan.tag));
            let err = match File::create(&err_path) {
                Ok(f) => Stdio::from(f),
                Err(_) => Stdio::null(),
            };
            proc.command
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(err);
            match proc.command.spawn() {
                Ok(mut child) => {
                    let stdout = child.stdout.take().expect("the runner's output is piped");
                    let send = send.clone();
                    std::thread::spawn(move || {
                        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                            if send.send((i, Instant::now(), line)).is_err() {
                                break;
                            }
                        }
                    });
                    running.push(Running { child });
                    bases.push(proc.game);
                }
                Err(e) => m.problems.push(format!("process {i} did not start: {e}")),
            }
        }
        drop(send);
        let mut asked: Vec<String> = running.iter().map(|r| r.pid().to_string()).collect();
        let procs = asked.len();
        asked.push("audiodg".into());
        asked.push(self.sampler.pid().to_string());
        let mut games = vec![GameState::default(); plan.games];
        let mut samples: Vec<Sample> = Vec::new();
        let mut done_seen = false;
        let (deadline, at) = match plan.timing {
            Timing::Kernels => (start + KERNEL_LIMIT + GRACE, None),
            Timing::Timed { at, end, wait_end } => (
                start
                    + Duration::from_secs_f64(end)
                    + if wait_end { GRACE } else { Duration::ZERO },
                Some(at),
            ),
        };
        let sample =
            |sampler: &mut Sampler, m: &mut Measurement, samples: &mut Vec<Sample>| match sampler
                .sample(&asked)
            {
                Ok(s) => {
                    m.samples.push(format!(
                        "{} {}",
                        start.elapsed().as_millis(),
                        sample_text(&s)
                    ));
                    samples.push(s);
                }
                Err(e) => m.problems.push(format!("a sample failed: {e}")),
            };
        if running.is_empty() {
            m.problems.push("nothing started".into());
        }
        while !running.is_empty() {
            let now = Instant::now();
            let next_sample = at.and_then(|at| {
                at.get(samples.len())
                    .map(|s| start + Duration::from_secs_f64(*s))
            });
            if let Some(due) = next_sample
                && now >= due
            {
                sample(&mut self.sampler, &mut m, &mut samples);
                continue;
            }
            let all_ended = games.iter().all(|g| g.end.is_some());
            let timed_out = now >= deadline;
            match plan.timing {
                Timing::Timed {
                    wait_end: false, ..
                } if timed_out => break,
                _ if timed_out => {
                    m.problems.push(format!(
                        "did not end within {:.0} s and was stopped",
                        deadline.duration_since(start).as_secs_f64()
                    ));
                    break;
                }
                _ if all_ended && next_sample.is_none() => break,
                _ => {}
            }
            let wake = next_sample.map_or(deadline, |s| s.min(deadline));
            match lines.recv_timeout(wake.saturating_duration_since(now)) {
                Ok((proc, at_line, text)) => {
                    m.lines.push(format!("{proc} {text}"));
                    let Some(line) = Line::parse(&text) else {
                        continue;
                    };
                    let base = bases.get(proc).copied().unwrap_or(0);
                    let game = base + line.game().unwrap_or(0);
                    let Some(state) = games.get_mut(game) else {
                        continue;
                    };
                    match line {
                        Line::First { .. } => {
                            state.first.get_or_insert(at_line);
                            if matches!(plan.timing, Timing::Kernels) && samples.is_empty() {
                                sample(&mut self.sampler, &mut m, &mut samples);
                            }
                        }
                        Line::Frames { n, ms, size, .. } => {
                            state.frames.push((ms, n));
                            if size.is_some() {
                                state.size = size;
                            }
                        }
                        Line::End {
                            status,
                            frames,
                            errors,
                            ..
                        } => {
                            state.end = Some(End {
                                status,
                                frames,
                                errors,
                            });
                        }
                        Line::Bench { name, fields, .. } => {
                            let get = |k: &str| fields.get(k).copied();
                            match name.as_str() {
                                "screen" => m.screen = get("w").zip(get("h")),
                                "clock" => m.clock_step = get("step"),
                                "frames.free" => {
                                    if let (Some(n), Some(ms)) = (get("frames"), get("ms"))
                                        && ms > 0
                                    {
                                        m.frames_free = Some(n as f64 * 1000.0 / ms as f64);
                                    }
                                }
                                "done" => {
                                    if matches!(plan.timing, Timing::Kernels) && !done_seen {
                                        done_seen = true;
                                        sample(&mut self.sampler, &mut m, &mut samples);
                                    }
                                }
                                _ => {
                                    if let (
                                        Some(round),
                                        Some(reps),
                                        Some(ms),
                                        Some(first),
                                        Some(check),
                                    ) = (
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
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    // Every process closed its output: they ended.
                    if next_sample.is_some() {
                        m.problems
                            .push("the processes ended before the last sample".into());
                    }
                    break;
                }
            }
        }
        drop(running);
        // Results.
        let firsts: Vec<Option<f64>> = games
            .iter()
            .map(|g| {
                g.first
                    .map(|f| f.duration_since(start).as_secs_f64() * 1000.0)
            })
            .collect();
        if plan.work != Work::Windowed {
            m.first_ms = firsts
                .iter()
                .copied()
                .collect::<Option<Vec<f64>>>()
                .and_then(|v| v.into_iter().reduce(f64::max));
        }
        if let Timing::Timed { at, .. } = plan.timing
            && plan.work != Work::Windowed
        {
            let window = ((at[0] * 1000.0) as u64, (at[1] * 1000.0) as u64);
            m.game_fps = games.iter().map(|g| fps(&g.frames, window)).collect();
            m.fps = m
                .game_fps
                .iter()
                .copied()
                .collect::<Option<Vec<f64>>>()
                .map(|v| v.iter().sum());
        }
        m.sizes = games.iter().map(|g| g.size).collect();
        m.ends = games.iter().map(|g| g.end.clone()).collect();
        m.shots = (0..plan.games)
            .map(|i| {
                let rel = format!("shots/{}/game-{i}.png", plan.tag);
                self.options.out.join(&rel).is_file().then_some(rel)
            })
            .collect();
        if let [a, b, ..] = samples.as_slice() {
            // The processes alive at both samples; one that ended is
            // named, and left out of the sums.
            let alive: Vec<usize> = (0..procs)
                .filter(|&i| sample::usage(a, b, self.sampler.rate, &[i]).gone.is_empty())
                .collect();
            if alive.len() < procs {
                m.problems.push(format!(
                    "{} of {procs} processes were gone at a sample and are not in the sums",
                    procs - alive.len()
                ));
            }
            if !alive.is_empty() {
                let usage = sample::usage(a, b, self.sampler.rate, &alive);
                m.cpu_ms_per_s = Some(usage.cpu_ms_per_s);
                m.memory = usage.memory;
                if matches!(plan.timing, Timing::Kernels) {
                    m.cpu_ms = Some(usage.cpu_ms);
                    m.wall_ms = Some(usage.span_ms);
                }
            }
            let audio = sample::usage(a, b, self.sampler.rate, &[procs]);
            if audio.gone.is_empty() {
                m.audio_ms_per_s = Some(audio.cpu_ms_per_s);
            }
            let own = sample::usage(a, b, self.sampler.rate, &[procs + 1]);
            m.sampler_ms_per_s = Some(own.cpu_ms_per_s);
            let all: Vec<usize> = (0..procs + 2).collect();
            m.other = Some(sample::other_load(a, b, self.sampler.rate, &all));
        } else if procs > 0 {
            m.problems.push("fewer than two samples".into());
        }
        let _ = fs::remove_dir_all(self.options.out.join("me-home"));
        let _ = fs::create_dir_all(self.options.out.join("me-home"));
        let _ = fs::remove_dir_all(self.options.out.join("data").join(&plan.tag));
        m
    }

    /// A short unrecorded run of the scene on each emulator, so file
    /// caches and the virus scanner settle first.
    fn warm_up(&mut self) {
        let mut sides = vec![Side::FastEmu];
        if self.classpath.is_some() {
            sides.push(Side::MicroEmu);
        }
        for side in sides {
            let tag = format!("warm-up-{}", side.key());
            let mut proc = if side == Side::FastEmu {
                self.fastemu(
                    &tag,
                    vec![self.built.scene_jar.clone()],
                    5.0,
                    None,
                    "",
                    None,
                )
            } else {
                self.microemu(side, &tag, 0, &self.built.scene_jar, 5.0, "", None)
            };
            proc.command
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            if let Ok(child) = proc.command.spawn() {
                let mut running = Running { child };
                let deadline = Instant::now() + GRACE;
                while matches!(running.child.try_wait(), Ok(None)) && Instant::now() < deadline {
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

/// A sample as the sampler printed it, for bench.json.
fn sample_text(s: &Sample) -> String {
    let procs: Vec<String> = s
        .procs
        .iter()
        .map(|p| match p {
            sample::Proc::Alive {
                pid,
                cycles,
                memory: Some(x),
            } => format!("{pid} {cycles} {} {} {}", x.peak_ws, x.ws, x.private),
            sample::Proc::Alive { pid, cycles, .. } => format!("{pid} {cycles} - - -"),
            sample::Proc::Gone { asked } => format!("{asked} gone"),
        })
        .collect();
    let mut text = format!("{} sys {} {} {}", s.unix_ms, s.idle, s.kernel, s.user);
    for p in procs {
        text.push_str(" | ");
        text.push_str(&p);
    }
    text
}

/// Frames a second from a runner's frame counts (`(ms, n)` on its own
/// clock) inside `window` (from, to, in ms).
pub fn fps(frames: &[(u64, u64)], window: (u64, u64)) -> Option<f64> {
    let inside: Vec<&(u64, u64)> = frames
        .iter()
        .filter(|(ms, _)| *ms >= window.0 && *ms <= window.1)
        .collect();
    let (first, last) = (inside.first()?, inside.last()?);
    (last.0 > first.0)
        .then(|| last.1.saturating_sub(first.1) as f64 * 1000.0 / (last.0 - first.0) as f64)
}

/// Runs every measurement and returns what they found.
pub fn run(options: &Options, built: &Built, root: &Path) -> Result<Report, String> {
    if options.windows {
        // Before anything starts: the owner's app must be closed, and a
        // backup a stopped run left must be put back by hand first.
        windowed::refuse_running(&windowed::running_processes()?)?;
        windowed::refuse_stale(&options.out.join(windowed::BACKUP))?;
    }
    let mut bench = Bench::new(options, built, root)?;
    let mut report = Report {
        fastemu_only: options.fastemu_only,
        me_cap: options.me_max_fps,
        games: bench.games.iter().map(|(g, _)| g.clone()).collect(),
        ..Report::default()
    };
    report.facts = bench.facts();
    eprintln!(
        "bench: measuring the idle machine for {} s",
        BASELINE.as_secs()
    );
    let (baseline, _, _) = bench.load(BASELINE)?;
    bench.baseline = baseline;
    bench.threshold = baseline + NOISE_MARGIN;
    report.baseline = Some(baseline);
    report.facts.push((
        "Load".into(),
        format!(
            "idle baseline {baseline:.0} ms/s; each measurement waits up to {} s for less than {:.0} ms/s",
            QUIET_LIMIT.as_secs(),
            bench.threshold
        ),
    ));
    if options.windows {
        windows_pair(&mut bench, options, &mut report)?;
        return Ok(report);
    }
    bench.warm_up();
    let me: Vec<Side> = if options.fastemu_only {
        Vec::new()
    } else {
        vec![Side::MicroEmu]
    };
    for run in 1..=options.runs {
        let mut works = vec![Work::Kernels, Work::Scene];
        works.extend(bench.games.iter().map(|(g, _)| Work::Game(g.slug.clone())));
        if bench.games.len() > 1 {
            works.push(Work::Eight);
        }
        for (i, work) in works.iter().enumerate() {
            let fastemu_first = (run + i) % 2 == 1;
            let mut sides = if fastemu_first {
                vec![Side::FastEmu]
            } else {
                Vec::new()
            };
            sides.extend(me.iter().copied());
            if !fastemu_first {
                sides.push(Side::FastEmu);
            }
            let first_side = sides[0];
            if !options.fastemu_only {
                if matches!(work, Work::Scene | Work::Eight) {
                    sides.push(Side::MicroEmuSmallHeap);
                }
                if options.me_max_fps.is_some() && matches!(work, Work::Game(_) | Work::Eight) {
                    sides.push(Side::MicroEmuCapped);
                }
            }
            for side in sides {
                let plan = bench.plan(work, side, run, first_side);
                let m = bench.measure(plan);
                report.measurements.push(m);
            }
        }
    }
    Ok(report)
}

/// `--windows`: the app with every game, then MicroEmulator's windows,
/// in turns, with the app's library backed up around the app's runs.
fn windows_pair(
    bench: &mut Bench<'_>,
    options: &Options,
    report: &mut Report,
) -> Result<(), String> {
    let config = windowed::app_config_dir().ok_or("APPDATA is not set")?;
    let backup = options.out.join(windowed::BACKUP);
    for run in 1..=options.runs {
        let mut sides = vec![Side::FastEmu];
        if bench.classpath.is_some() {
            if run % 2 == 1 {
                sides.push(Side::MicroEmu);
            } else {
                sides.insert(0, Side::MicroEmu);
            }
        }
        let first_side = sides[0];
        for side in sides {
            let plan = bench.plan(&Work::Windowed, side, run, first_side);
            if side != Side::FastEmu {
                report.measurements.push(bench.measure(plan));
                continue;
            }
            windowed::refuse_running(&windowed::running_processes()?)?;
            let before = windowed::save_folders(&config);
            windowed::back_up(&config, &backup)?;
            let m = bench.measure(plan);
            if let Err(e) = windowed::restore(&config, &backup) {
                return Err(format!(
                    "{e}. The app's files are still in {}: put them back by hand",
                    backup.display()
                ));
            }
            let made: Vec<String> = windowed::save_folders(&config)
                .into_iter()
                .filter(|f| !before.contains(f))
                .collect();
            if !made.is_empty() {
                eprintln!(
                    "bench: the app made these save folders in {}: {}",
                    config.join("games").display(),
                    made.join(", ")
                );
            }
            report.measurements.push(m);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_rates_come_from_the_window() {
        let frames = [
            (1000, 10),
            (10_000, 100),
            (20_000, 400),
            (40_000, 1000),
            (44_000, 1200),
        ];
        assert_eq!(fps(&frames, (10_000, 40_000)), Some(30.0));
        assert_eq!(fps(&frames, (41_000, 42_000)), None);
        assert_eq!(fps(&[(10_000, 5)], (0, 50_000)), None);
    }
}
