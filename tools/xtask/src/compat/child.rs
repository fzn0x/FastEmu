//! The runner of one game: `cargo xtask compat` starts this program again
//! as `compat-run` for every JAR, so a game that hangs the runner can be
//! killed without losing the others. It launches the game with the real
//! launcher and no sound device, plays the key script from the first
//! frame, and writes [`Record`]s to its standard output as things happen.
//!
//! A game whose screen stood still for the last [`STALL`] of its run is
//! asked whether it waits, as on an About page, or is stuck ([`probe`]).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fastemu_core::{
    Frame, Instance, InstanceSettings, InstanceStatus, LaunchRequest, Launcher, LogLevel,
    builtin_profiles,
};
use fastemu_jvm::Activity;
use fastemu_midp::{MidpInstance, MidpLauncher};

use super::script::{self, Player};
use super::wire::Record;

/// How long a stopping game gets to wind down.
const STOP_WAIT: Duration = Duration::from_secs(5);
/// A game that presents no new frame for this long by the end of its run,
/// though the key script pressed keys, has a still screen. It counts as
/// running only when it shows that it waits ([`probe`]), threw no
/// exception and did not pause itself ([`still_end`]); otherwise it is
/// hung: a player sees a frozen screen.
const STALL: Duration = Duration::from_secs(10);
/// How long a game whose screen stood still gets to show that it waits:
/// its event thread answers a ping and every thread waits.
const PROBE_WAIT: Duration = Duration::from_secs(5);
/// How often the probe looks again.
const PROBE_STEP: Duration = Duration::from_millis(20);

/// What the probe of a still screen found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Probe {
    /// The event thread answered, and then every thread waited.
    Waits,
    /// The event thread did not answer within [`PROBE_WAIT`].
    Silent,
    /// A thread held the VM lock: the ping or a look could not get it.
    Locked,
    /// The event thread answered, but at the last look threads ran,
    /// waited for a lock, or tasks waited for the event thread.
    Busy {
        running: usize,
        blocked: usize,
        posted: usize,
    },
}

impl Probe {
    /// Why the game waits or is stuck, for the detail of its result.
    fn why(self) -> String {
        match self {
            Probe::Waits => "its threads wait and its event thread answers".into(),
            Probe::Silent => format!(
                "its event thread did not answer in {} s",
                PROBE_WAIT.as_secs()
            ),
            Probe::Locked => "a thread kept the virtual machine locked".into(),
            Probe::Busy { running, .. } if running > 0 => {
                format!("{running} of its threads kept running")
            }
            Probe::Busy { blocked, .. } if blocked > 0 => {
                format!("{blocked} of its threads waited for a lock")
            }
            Probe::Busy { posted: 1, .. } => "a task waited for its event thread".into(),
            Probe::Busy { posted, .. } if posted > 1 => {
                format!("{posted} tasks waited for its event thread")
            }
            Probe::Busy { .. } => "its threads ended".into(),
        }
    }
}

/// Asks a game whose screen stood still whether it waits: pings its event
/// thread, then looks at its threads until they all wait or
/// [`PROBE_WAIT`] passes.
fn probe(instance: &MidpInstance) -> Probe {
    let deadline = Instant::now() + PROBE_WAIT;
    let ping = instance.ping();
    watch(deadline, || ping.answered(), || look(instance))
}

/// What the game's threads are doing now, `None` only when a thread holds
/// the VM lock. A game that ended during the probe has no threads, and its
/// machine may be freed already, which `MidpInstance::activity` also
/// answers with `None`: that machine has no threads, it is not locked.
fn look(instance: &MidpInstance) -> Option<Activity> {
    instance
        .activity()
        .or_else(|| instance.vm_freed().then(Activity::default))
}

/// The verdict of a probe from whether the event thread `answered` and
/// what a `look` at the threads finds (`None` when a thread holds the VM
/// lock): [`Probe::Waits`] at the first idle look after an answer,
/// otherwise what the last look found once `deadline` passes. Without an
/// answer, the ping may have been dropped because a thread held the VM
/// lock, so one look tells [`Probe::Locked`] from [`Probe::Silent`].
fn watch(
    deadline: Instant,
    answered: impl Fn() -> bool,
    mut look: impl FnMut() -> Option<Activity>,
) -> Probe {
    let mut seen = None;
    loop {
        if answered() {
            seen = Some(match look() {
                Some(a) if a.is_idle() => return Probe::Waits,
                Some(a) => Probe::Busy {
                    running: a.running(),
                    blocked: a.blocked,
                    posted: a.posted,
                },
                None => Probe::Locked,
            });
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(PROBE_STEP);
    }
    seen.unwrap_or_else(|| match look() {
        Some(_) => Probe::Silent,
        None => Probe::Locked,
    })
}

/// The status and detail of a run whose last new frame came at `at`
/// seconds, [`STALL`] or more before its end, after `errors` exceptions.
/// The game runs when it waits on its still screen, threw nothing and is
/// not paused; otherwise it is hung, and the detail says why. An escaped
/// exception may be what stopped the game, and a game that paused itself
/// waits to be resumed, which the run never does.
fn still_end(at: f64, probe: Probe, errors: u64, paused: bool) -> (&'static str, String) {
    let why = probe.why();
    if probe == Probe::Waits && errors == 0 && !paused {
        return ("runs", format!("a still screen from {at:.1} s: {why}"));
    }
    let threw = match (probe, errors) {
        (Probe::Waits, 0) => String::new(),
        (Probe::Waits, 1) => ", but it threw an exception".into(),
        (Probe::Waits, n) => format!(", but it threw {n} exceptions"),
        _ => String::new(),
    };
    let paused = if paused { "; the game is paused" } else { "" };
    (
        "hung",
        format!("no new frame after {at:.1} s; {why}{threw}{paused}"),
    )
}

/// What one run is asked to do.
pub struct Options {
    pub suite: PathBuf,
    pub seconds: f64,
    pub profile: String,
    pub keys: String,
    pub screenshot: PathBuf,
    pub data: PathBuf,
}

impl Options {
    /// Parses the arguments after `compat-run`.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut iter = args.iter();
        let suite = iter.next().ok_or("compat-run needs a suite")?.into();
        let mut options = Self {
            suite,
            seconds: 20.0,
            profile: "generic-240x320".into(),
            keys: String::new(),
            screenshot: PathBuf::from("screen.png"),
            data: PathBuf::from("data"),
        };
        while let Some(arg) = iter.next() {
            let value = iter
                .next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))?;
            match arg.as_str() {
                "--seconds" => {
                    options.seconds = value
                        .parse()
                        .map_err(|_| format!("bad number of seconds {value}"))?;
                }
                "--profile" => options.profile = value,
                "--keys" => options.keys = value,
                "--screenshot" => options.screenshot = value.into(),
                "--data" => options.data = value.into(),
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(options)
    }

    /// The arguments [`Options::parse`] reads back.
    pub fn args(&self) -> Vec<std::ffi::OsString> {
        let mut args: Vec<std::ffi::OsString> = vec![self.suite.clone().into()];
        let pairs = [
            ("--seconds", self.seconds.to_string().into()),
            ("--profile", self.profile.clone().into()),
            ("--keys", self.keys.clone().into()),
            ("--screenshot", self.screenshot.clone().into_os_string()),
            ("--data", self.data.clone().into_os_string()),
        ];
        for (name, value) in pairs {
            args.push(name.into());
            args.push(value);
        }
        args
    }
}

fn emit(out: &mut impl Write, record: &Record) {
    let _ = writeln!(out, "{}", record.line());
    let _ = out.flush();
}

fn level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Out => "out",
        LogLevel::Err => "err",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
    }
}

fn drain(instance: &dyn Instance, out: &mut impl Write) {
    for line in instance.drain_log() {
        emit(
            out,
            &Record::Log {
                level: level(line.level).into(),
                text: line.text,
            },
        );
    }
}

/// Writes `frame` as a PNG.
fn save_png(frame: &Frame, path: &Path) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder =
        png::Encoder::new(std::io::BufWriter::new(file), frame.width(), frame.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&frame.to_rgba8())
        .map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())
}

fn end(out: &mut impl Write, status: &str, detail: String) {
    emit(
        out,
        &Record::End {
            status: status.into(),
            detail,
        },
    );
}

/// Runs one game and writes its records to `out`.
pub fn run(options: &Options, out: &mut impl Write) -> Result<(), String> {
    let profile = builtin_profiles()
        .into_iter()
        .find(|p| p.id == options.profile)
        .ok_or_else(|| format!("unknown profile {}", options.profile))?;
    let steps = script::parse(&options.keys)?;
    let launcher = MidpLauncher::new(None);
    let info = match launcher.inspect(&options.suite) {
        Ok(info) => info,
        Err(e) => {
            end(out, "crashed", format!("the suite does not open: {e}"));
            return Ok(());
        }
    };
    emit(
        out,
        &Record::Info {
            name: info.name,
            vendor: info.vendor,
            version: info.version,
        },
    );
    let _ = std::fs::remove_dir_all(&options.data);
    let request = LaunchRequest {
        profile,
        settings: InstanceSettings::default(),
        ..LaunchRequest::new(&options.suite, &options.data)
    };
    let start = Instant::now();
    let instance = match launcher.launch_midp(request) {
        Ok(instance) => instance,
        Err(e) => {
            end(out, "crashed", format!("the launch failed: {e}"));
            return Ok(());
        }
    };
    let mut player = Player::new(steps);
    let mut serial = 0;
    let mut last_frame = None;
    let finished = loop {
        drain(&*instance, out);
        let now = Instant::now();
        let status = instance.status();
        if status.is_finished() {
            break Some(status);
        }
        if let Some(frame) = instance.frame()
            && frame.serial() > serial
        {
            serial = frame.serial();
            let secs = now.duration_since(start).as_secs_f64();
            last_frame = Some(secs);
            emit(out, &Record::Frame { serial, secs });
        }
        if serial > 0 {
            player.advance(&*instance, now);
        }
        if now.duration_since(start).as_secs_f64() >= options.seconds {
            break None;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let stalled = finished.is_none()
        && last_frame.is_some_and(|at| options.seconds - at >= STALL.as_secs_f64());
    let probed = stalled.then(|| probe(&instance));
    // The errors are counted after the probe, so they match its verdict.
    drain(&*instance, out);
    let errors = instance.error_count();
    emit(out, &Record::Errors(errors));
    if let Some(frame) = instance.frame() {
        match save_png(&frame, &options.screenshot) {
            Ok(()) => emit(
                out,
                &Record::Screenshot(options.screenshot.display().to_string()),
            ),
            Err(e) => emit(out, &Record::Note(format!("the screenshot failed: {e}"))),
        }
    }
    let paused = instance.status() == InstanceStatus::Paused;
    instance.stop();
    let deadline = Instant::now() + STOP_WAIT;
    while !instance.status().is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    if !instance.status().is_finished() {
        emit(
            out,
            &Record::Note(format!(
                "the game did not stop within {} s",
                STOP_WAIT.as_secs()
            )),
        );
    }
    drain(&*instance, out);
    let _ = std::fs::remove_dir_all(&options.data);
    match finished {
        Some(InstanceStatus::Crashed { message }) => end(out, "crashed", message),
        Some(_) => end(out, "exited", "the game ended itself".into()),
        None if serial == 0 => end(out, "hung", format!("no frame in {} s", options.seconds)),
        None => {
            let at = last_frame.unwrap_or_default();
            if let Some(probe) = probed {
                let (status, detail) = still_end(at, probe, errors, paused);
                end(out, status, detail);
            } else {
                let paused = if paused { "; the game is paused" } else { "" };
                end(
                    out,
                    "runs",
                    format!("the last new frame came at {at:.1} s{paused}"),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn activity(threads: usize, waiting: usize, blocked: usize, posted: usize) -> Activity {
        Activity {
            threads,
            waiting,
            blocked,
            posted,
        }
    }

    fn busy(running: usize, blocked: usize, posted: usize) -> Probe {
        Probe::Busy {
            running,
            blocked,
            posted,
        }
    }

    #[test]
    fn a_still_screen_that_waits_runs() {
        let (status, detail) = still_end(4.4, Probe::Waits, 0, false);
        assert_eq!(status, "runs");
        assert_eq!(
            detail,
            "a still screen from 4.4 s: its threads wait and its event thread answers"
        );
    }

    #[test]
    fn a_still_screen_after_an_exception_stays_hung() {
        let (status, detail) = still_end(0.2, Probe::Waits, 1, false);
        assert_eq!(status, "hung");
        assert_eq!(
            detail,
            "no new frame after 0.2 s; its threads wait and its event thread answers, \
             but it threw an exception"
        );
        let (status, detail) = still_end(0.2, Probe::Waits, 3, false);
        assert_eq!(status, "hung");
        assert!(detail.ends_with(", but it threw 3 exceptions"), "{detail}");
    }

    #[test]
    fn a_still_screen_of_a_game_that_paused_itself_stays_hung() {
        let (status, detail) = still_end(4.4, Probe::Waits, 0, true);
        assert_eq!(status, "hung");
        assert_eq!(
            detail,
            "no new frame after 4.4 s; its threads wait and its event thread answers; \
             the game is paused"
        );
    }

    #[test]
    fn a_stuck_game_stays_hung_and_says_why() {
        let cases = [
            (Probe::Silent, "its event thread did not answer in 5 s"),
            (Probe::Locked, "a thread kept the virtual machine locked"),
            (busy(1, 0, 0), "1 of its threads kept running"),
            (busy(2, 1, 1), "2 of its threads kept running"),
            (busy(0, 2, 0), "2 of its threads waited for a lock"),
            (busy(0, 0, 1), "a task waited for its event thread"),
            (busy(0, 0, 3), "3 tasks waited for its event thread"),
            (busy(0, 0, 0), "its threads ended"),
        ];
        for (probe, why) in cases {
            // Exceptions add nothing to a game that is stuck anyway.
            let (status, detail) = still_end(4.4, probe, 2, false);
            assert_eq!(status, "hung", "{probe:?}");
            assert_eq!(detail, format!("no new frame after 4.4 s; {why}"));
        }
        let (status, detail) = still_end(4.4, Probe::Silent, 0, true);
        assert_eq!(status, "hung");
        assert_eq!(
            detail,
            "no new frame after 4.4 s; its event thread did not answer in 5 s; \
             the game is paused"
        );
    }

    #[test]
    fn a_probe_that_gets_no_answer_tells_a_locked_machine_from_a_silent_one() {
        // The deadline has passed: one look decides.
        let now = Instant::now();
        let idle = activity(3, 3, 0, 0);
        assert_eq!(watch(now, || false, || Some(idle)), Probe::Silent);
        assert_eq!(watch(now, || false, || None), Probe::Locked);
    }

    #[test]
    fn a_probe_after_an_answer_says_what_the_last_look_found() {
        let now = Instant::now();
        assert_eq!(
            watch(now, || true, || Some(activity(2, 2, 0, 0))),
            Probe::Waits
        );
        assert_eq!(watch(now, || true, || None), Probe::Locked);
        assert_eq!(
            watch(now, || true, || Some(activity(3, 1, 1, 0))),
            busy(1, 1, 0)
        );
        assert_eq!(
            watch(now, || true, || Some(activity(2, 2, 0, 1))),
            busy(0, 0, 1)
        );
        assert_eq!(
            watch(now, || true, || Some(activity(0, 0, 0, 0))),
            busy(0, 0, 0)
        );
    }

    #[test]
    fn a_probe_waits_for_the_answer_and_the_first_idle_look() {
        // Far off, so only the looks end the probe, however slow the machine.
        let deadline = Instant::now() + Duration::from_secs(600);
        let asked = Cell::new(0);
        let looked = Cell::new(0);
        let answered = || {
            asked.set(asked.get() + 1);
            asked.get() > 2
        };
        let look = || {
            looked.set(looked.get() + 1);
            match looked.get() {
                1 => Some(activity(2, 1, 0, 0)),
                2 => None,
                _ => Some(activity(2, 2, 0, 0)),
            }
        };
        assert_eq!(watch(deadline, answered, look), Probe::Waits);
        assert_eq!((asked.get(), looked.get()), (5, 3));
    }

    /// The CRC-32 a ZIP entry carries.
    fn crc32(data: &[u8]) -> u32 {
        let mut crc = !0u32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    /// A JAR that holds only `manifest`, stored.
    fn jar(manifest: &str) -> Vec<u8> {
        let name = b"META-INF/MANIFEST.MF";
        let data = manifest.as_bytes();
        let mut header = Vec::new();
        // Version, flags, method (stored), time and date.
        header.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        header.extend_from_slice(&crc32(data).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(name.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        let mut out = 0x0403_4b50u32.to_le_bytes().to_vec();
        out.extend_from_slice(&header);
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        let start = out.len() as u32;
        out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&header);
        // Comment length, disk, attributes and the entry's offset.
        out.extend_from_slice(&[0; 14]);
        out.extend_from_slice(name);
        let size = out.len() as u32 - start;
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        out.extend_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    #[test]
    fn a_game_whose_machine_is_freed_has_no_threads_and_is_not_locked() {
        let root = std::env::temp_dir().join(format!("xtask-compat-look-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("a folder for the suite");
        let suite = root.join("gone.jar");
        let manifest = "Manifest-Version: 1.0\r\nMIDlet-Name: Gone\r\n\
                        MIDlet-Vendor: FastEmu\r\nMIDlet-Version: 1.0\r\n\
                        MIDlet-1: Gone, , no.such.Game\r\n";
        std::fs::write(&suite, jar(manifest)).expect("writing the suite");
        // The MIDlet class is missing, so the game crashes at once and its
        // machine is freed while the probe still holds the instance.
        let instance = MidpLauncher::new(None)
            .launch_midp(LaunchRequest::new(&suite, root.join("data")))
            .expect("launching the suite");
        let start = Instant::now();
        while !instance.vm_freed() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "the machine of a game that crashed is freed"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(instance.status().is_finished());
        // The instance answers a freed machine as it does a locked one;
        // the probe must not.
        assert_eq!(instance.activity(), None);
        assert_eq!(look(&instance), Some(Activity::default()));
        drop(instance);
        let _ = std::fs::remove_dir_all(&root);
    }
}
