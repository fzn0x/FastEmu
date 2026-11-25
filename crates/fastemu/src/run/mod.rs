//! `fastemu run`: one game without a window, for tests and scripts.
//!
//! It launches the suite with the real launcher and no audio output, runs
//! it for a number of frames or seconds while pressing the keys of a
//! script, prints the game's log as it comes, saves the last frame if
//! asked, and exits with 1 when the game crashed or never showed a
//! picture. No GPUI code runs on this path.
//!
//! Frames are refreshes of the phone's screen, 60 a second, as video
//! emulators count them, and not the pictures the game draws: a puzzle
//! game that redraws once a second would otherwise make `--frames 300`
//! last five minutes, and a game-over screen would never reach it.

mod record;
mod script;

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fastemu_core::{
    DeviceProfile, Instance, InstanceSettings, InstanceStatus, LaunchRequest, Launcher, LogLevel,
    builtin_profiles, default_profile, escape_control,
};
use fastemu_midp::MidpLauncher;

use self::record::{Recorder, save_png};
use self::script::{Script, Step};

pub const USAGE: &str = "\
Usage: fastemu run SUITE [OPTIONS]

Runs a .jar or .jad without a window and prints what the game logs.

Options:
  --midlet N        Start MIDlet-<N> of the suite (default 1)
  --profile ID      Run on the phone profile ID (default generic-240x320)
  --touch           Give the phone a touch screen whatever the profile
  --frames N        Stop after N frames of the screen, 60 a second, from
                    the game's first picture (default 300)
  --seconds S       Stop after S seconds instead
  --screenshot PATH Save the last frame as a PNG
  --record DIR      Save presented frames in DIR as frame-000001.png and on;
                    DIR must hold no frames of an earlier run
  --every N         With --record, save every Nth presented frame (default 1)
  --keys SCRIPT     Press keys while the game runs (see below)
  --encoding NAME   The game's text encoding (default ISO-8859-1)
  --data DIR        Keep the game's record stores in DIR (default: a
                    temporary folder, deleted after the run)

SCRIPT is a list of steps separated by commas or spaces, run in order
from the first frame: a key name taps the key, NAME:MS holds it for MS
milliseconds, and wait:MS (or a bare number) waits. When SCRIPT names a
file, the steps are read from it, one or more per line; # starts a
comment. Key names: 0 to 9, star, pound, up, down, left, right, fire,
softleft, softright, clear, send, end.

Pointer steps touch the screen at phone pixel X,Y: press@XxY puts a
finger down, drag@XxY moves it, release@XxY lifts it, and tap@XxY does
all three (tap@XxY:MS holds for MS milliseconds). Games get them only on
a touch screen: a touch profile such as generic-360x640, or --touch.

Exits with 1 when the game crashes or shows nothing in its first 30
seconds, and with 2 for a bad command line.
";

/// How long the runner waits for the game's first picture.
const STALL: Duration = Duration::from_secs(30);
/// How often a phone's screen refreshes: the frames `--frames` counts.
const REFRESH_RATE: f64 = 60.0;
/// How long a stopping game gets to wind down.
const STOP_WAIT: Duration = Duration::from_secs(5);

/// When a run ends.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Limit {
    Frames(u64),
    Seconds(f64),
}

/// What `fastemu run` was asked to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    pub suite: PathBuf,
    /// Which MIDlet, from zero.
    pub midlet: usize,
    pub profile: DeviceProfile,
    pub limit: Limit,
    pub screenshot: Option<PathBuf>,
    /// Where `--record` saves frames, and every how many presented frames.
    pub record: Option<(PathBuf, u64)>,
    pub keys: Vec<Step>,
    pub encoding: Option<String>,
    /// Where the game keeps its record stores; `None` for a temporary
    /// folder that goes when the run ends.
    pub data: Option<PathBuf>,
}

fn value(args: &mut impl Iterator<Item = OsString>, option: &str) -> Result<String, String> {
    let value = args
        .next()
        .ok_or_else(|| format!("{option} needs a value"))?;
    value
        .into_string()
        .map_err(|_| format!("the value of {option} is not valid text"))
}

/// Parses the arguments after `run`.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Options, String> {
    let mut args = args.into_iter();
    let mut suite = None;
    let mut midlet = 0;
    let mut profile = default_profile();
    let mut frames = None;
    let mut seconds = None;
    let mut screenshot = None;
    let mut record = None;
    let mut every = None;
    let mut keys = Vec::new();
    let mut encoding = None;
    let mut data = None;
    let mut touch = false;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--midlet") => {
                let n: usize = value(&mut args, "--midlet")?
                    .parse()
                    .map_err(|_| "--midlet takes a number from 1".to_owned())?;
                midlet = n.checked_sub(1).ok_or("--midlet takes a number from 1")?;
            }
            Some("--profile") => {
                let id = value(&mut args, "--profile")?;
                let profiles = builtin_profiles();
                profile = profiles
                    .iter()
                    .find(|p| p.id == id)
                    .cloned()
                    .ok_or_else(|| {
                        let ids: Vec<_> = profiles.iter().map(|p| p.id.as_str()).collect();
                        format!("unknown profile {id}; the profiles are {}", ids.join(", "))
                    })?;
            }
            Some("--touch") => touch = true,
            Some("--frames") => {
                let n = value(&mut args, "--frames")?;
                frames = Some(
                    n.parse::<u64>()
                        .map_err(|_| format!("bad frame count {n}"))?,
                );
            }
            Some("--seconds") => {
                let s = value(&mut args, "--seconds")?;
                let s = s
                    .parse::<f64>()
                    .ok()
                    .filter(|s| s.is_finite() && *s >= 0.0)
                    .ok_or_else(|| format!("bad number of seconds {s}"))?;
                seconds = Some(s);
            }
            Some("--screenshot") => {
                screenshot = Some(PathBuf::from(value(&mut args, "--screenshot")?));
            }
            Some("--record") => record = Some(PathBuf::from(value(&mut args, "--record")?)),
            Some("--every") => {
                let n = value(&mut args, "--every")?;
                every = Some(
                    n.parse::<u64>()
                        .ok()
                        .filter(|&n| n > 0)
                        .ok_or_else(|| format!("--every takes a number from 1, not {n}"))?,
                );
            }
            Some("--keys") => {
                let script = value(&mut args, "--keys")?;
                let text = if Path::new(&script).is_file() {
                    std::fs::read_to_string(&script)
                        .map_err(|e| format!("reading the key script {script}: {e}"))?
                } else {
                    script
                };
                keys = script::parse(&text)?;
            }
            Some("--encoding") => encoding = Some(value(&mut args, "--encoding")?),
            Some("--data") => data = Some(PathBuf::from(value(&mut args, "--data")?)),
            Some(option) if option.starts_with("--") => {
                return Err(format!("unknown option {option}"));
            }
            _ if suite.is_none() => suite = Some(PathBuf::from(arg)),
            _ => return Err("give one suite to run".to_owned()),
        }
    }
    if touch {
        profile.touch = true;
    }
    let record = match (record, every) {
        (Some(dir), every) => Some((dir, every.unwrap_or(1))),
        (None, Some(_)) => return Err("--every goes with --record".to_owned()),
        (None, None) => None,
    };
    let limit = match (frames, seconds) {
        (Some(_), Some(_)) => return Err("give --frames or --seconds, not both".to_owned()),
        (_, Some(s)) => Limit::Seconds(s),
        (n, None) => Limit::Frames(n.unwrap_or(300)),
    };
    Ok(Options {
        suite: suite.ok_or("give a .jar or .jad to run")?,
        midlet,
        profile,
        limit,
        screenshot,
        record,
        keys,
        encoding,
        data,
    })
}

/// Prints the game's new log lines: what it printed to `System.out` on
/// `out`, everything else on `err`.
fn print_log(instance: &dyn Instance, out: &mut dyn Write, err: &mut dyn Write) {
    for line in instance.drain_log() {
        let _ = match line.level {
            LogLevel::Out => writeln!(out, "{}", line.text),
            LogLevel::Err => writeln!(err, "{}", line.text),
            LogLevel::Warn => writeln!(err, "warning: {}", line.text),
            LogLevel::Info => writeln!(err, "info: {}", line.text),
        };
    }
}

/// `text` as the terminal may show it. A game's name, version and error
/// texts, and the suite's file name, are the game's to choose, so their
/// control and bidirectional override characters are escaped as the log's
/// are (see [`escape_control`]).
fn shown(text: impl std::fmt::Display) -> String {
    escape_control(text.to_string())
}

/// Runs `options`, printing to `out` and `err`, and returns the exit
/// code.
pub fn run(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let launcher = MidpLauncher::new(None);
    let file = shown(options.suite.file_name().map_or_else(
        || options.suite.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    ));
    let info = match launcher.inspect(&options.suite) {
        Ok(info) => info,
        Err(e) => {
            let _ = writeln!(err, "fastemu: {file}: {}", shown(e));
            return 1;
        }
    };
    let _ = writeln!(
        out,
        "loaded {file} ({} {})",
        shown(&info.name),
        shown(&info.version)
    );
    let _ = writeln!(out, "profile {}, 1 instance", options.profile.id);
    let temporary = options.data.is_none();
    let data_dir = options.data.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!(
            "fastemu-run-{}-{}",
            std::process::id(),
            fastemu_core::InstanceId::next().0
        ))
    });
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        let _ = writeln!(err, "fastemu: {}: {e}", data_dir.display());
        return 1;
    }
    let mut recorder = match &options.record {
        Some((dir, every)) => match Recorder::new(dir.clone(), *every) {
            Ok(recorder) => Some(recorder),
            Err(e) => {
                let _ = writeln!(err, "fastemu: could not record: {e}");
                return 1;
            }
        },
        None => None,
    };
    let mut settings = InstanceSettings::default();
    if let Some(encoding) = &options.encoding {
        settings.encoding.clone_from(encoding);
    }
    let request = LaunchRequest {
        midlet: options.midlet,
        profile: options.profile.clone(),
        settings,
        ..LaunchRequest::new(&options.suite, &data_dir)
    };
    let instance = match launcher.launch_midp(request) {
        Ok(instance) => instance,
        Err(e) => {
            let _ = writeln!(err, "fastemu: {file}: {}", shown(e));
            return 1;
        }
    };
    let Ran { code, frames } = drive(options, &*instance, recorder.as_mut(), out, err);
    let errors = instance.error_count();
    let last = instance.frame();
    let drawn = last.as_ref().map_or(0, fastemu_core::Frame::serial);
    instance.stop();
    let deadline = Instant::now() + STOP_WAIT;
    while !instance.status().is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    print_log(&*instance, out, err);
    if temporary {
        let _ = std::fs::remove_dir_all(&data_dir);
    }
    let _ = writeln!(out, "ran {frames} frames ({drawn} drawn), {errors} errors");
    if let Some(recorder) = &recorder {
        let _ = writeln!(
            out,
            "recorded {} frames in {}",
            recorder.saved(),
            recorder.dir().display()
        );
    }
    let mut code = code;
    if let Some(path) = &options.screenshot {
        match last {
            Some(frame) => match save_png(&frame, path) {
                Ok(()) => {
                    let _ = writeln!(
                        out,
                        "saved {} ({}x{})",
                        path.display(),
                        frame.width(),
                        frame.height()
                    );
                }
                Err(e) => {
                    let _ = writeln!(err, "fastemu: could not save {}: {e}", path.display());
                    code = 1;
                }
            },
            None => {
                let _ = writeln!(err, "fastemu: no frame to save");
                code = 1;
            }
        }
    }
    code
}

/// How a run went.
struct Ran {
    /// The exit code so far.
    code: u8,
    /// Screen refreshes from the first picture to the end.
    frames: u64,
}

/// The screen refreshes in `time`.
fn refreshes(time: Duration) -> u64 {
    (time.as_secs_f64() * REFRESH_RATE) as u64
}

/// The frames a run that went on for `time` after the first picture
/// reports. The runner notices a frame limit only between its polls, a
/// refresh or more late on a busy machine, but the run stops at the
/// limit, so it never reports more.
fn counted(time: Duration, limit: Limit) -> u64 {
    let frames = refreshes(time);
    match limit {
        Limit::Frames(n) => frames.min(n),
        Limit::Seconds(_) => frames,
    }
}

/// Runs the game until its limit, a crash, its exit or a start that
/// shows nothing, saving pictures into `recorder` as they come.
fn drive(
    options: &Options,
    instance: &dyn Instance,
    mut recorder: Option<&mut Recorder>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Ran {
    let start = Instant::now();
    let mut script = Script::new(options.keys.clone());
    let mut serial = 0;
    let mut first: Option<Instant> = None;
    let ran = |code, first: Option<Instant>| Ran {
        code,
        frames: first.map_or(0, |first| counted(first.elapsed(), options.limit)),
    };
    loop {
        print_log(instance, out, err);
        let now = Instant::now();
        match instance.status() {
            InstanceStatus::Crashed { message } => {
                print_log(instance, out, err);
                let _ = writeln!(err, "fastemu: the game crashed: {}", shown(message));
                return ran(1, first);
            }
            InstanceStatus::Exited => {
                let _ = writeln!(out, "the game exited");
                return ran(0, first);
            }
            _ => {}
        }
        if let Some(frame) = instance.frame()
            && frame.serial() > serial
        {
            serial = frame.serial();
            first.get_or_insert(now);
            if let Some(recorder) = recorder.as_deref_mut()
                && let Err(e) = recorder.offer(&frame)
            {
                let _ = writeln!(err, "fastemu: could not record: {e}");
                return ran(1, first);
            }
        }
        if first.is_some() {
            script.advance(instance, now);
        }
        let done = match options.limit {
            Limit::Frames(n) => first.is_some_and(|first| refreshes(now - first) >= n),
            Limit::Seconds(s) => now.duration_since(start).as_secs_f64() >= s,
        };
        if done {
            return ran(0, first);
        }
        if first.is_none() && now.duration_since(start) > STALL {
            let _ = writeln!(
                err,
                "fastemu: the game showed nothing for {} seconds; stopping",
                STALL.as_secs()
            );
            return ran(1, first);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// `fastemu run` from the command line.
pub fn main(args: Vec<OsString>) -> std::process::ExitCode {
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }
    match parse(args) {
        Ok(options) => {
            let code = run(&options, &mut std::io::stdout(), &mut std::io::stderr());
            std::process::ExitCode::from(code)
        }
        Err(e) => {
            eprintln!("fastemu run: {}\n\n{USAGE}", shown(e));
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_strs(args: &[&str]) -> Result<Options, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn defaults_run_300_frames_of_the_first_midlet() {
        let options = parse_strs(&["snake.jar"]).unwrap();
        assert_eq!(options.suite, PathBuf::from("snake.jar"));
        assert_eq!(options.midlet, 0);
        assert_eq!(options.profile.id, "generic-240x320");
        assert!(!options.profile.touch);
        assert_eq!(options.limit, Limit::Frames(300));
        assert!(options.keys.is_empty() && options.screenshot.is_none());
    }

    #[test]
    fn options_set_what_they_name() {
        let options = parse_strs(&[
            "--profile",
            "nokia-s40-128x160",
            "game.jad",
            "--midlet",
            "2",
            "--seconds",
            "1.5",
            "--screenshot",
            "out.png",
            "--encoding",
            "UTF-8",
            "--data",
            "saves",
            "--touch",
            "--record",
            "frames",
            "--every",
            "4",
        ])
        .unwrap();
        assert_eq!(options.midlet, 1);
        assert_eq!(options.profile.screen_width, 128);
        assert!(options.profile.touch, "--touch counts after --profile too");
        assert_eq!(options.record, Some((PathBuf::from("frames"), 4)));
        assert_eq!(
            parse_strs(&["a.jar", "--record", "all"]).unwrap().record,
            Some((PathBuf::from("all"), 1))
        );
        assert_eq!(options.limit, Limit::Seconds(1.5));
        assert_eq!(options.screenshot, Some(PathBuf::from("out.png")));
        assert_eq!(options.encoding.as_deref(), Some("UTF-8"));
        assert_eq!(options.data, Some(PathBuf::from("saves")));
    }

    #[test]
    fn a_frame_limit_noticed_late_still_reports_the_limit() {
        let late = Duration::from_millis(2_040);
        assert_eq!(refreshes(late), 122);
        assert_eq!(counted(late, Limit::Frames(120)), 120);
        assert_eq!(
            counted(Duration::from_millis(1_000), Limit::Frames(120)),
            60
        );
        assert_eq!(counted(late, Limit::Seconds(2.0)), 122);
    }

    #[test]
    fn bad_command_lines_say_why() {
        assert!(parse_strs(&[]).unwrap_err().contains(".jar"));
        assert!(parse_strs(&["a.jar", "b.jar"]).is_err());
        assert!(parse_strs(&["a.jar", "--frames", "1", "--seconds", "1"]).is_err());
        assert!(parse_strs(&["a.jar", "--midlet", "0"]).is_err());
        assert!(
            parse_strs(&["a.jar", "--profile", "nope"])
                .unwrap_err()
                .contains("generic-240x320")
        );
        assert!(parse_strs(&["a.jar", "--frames"]).is_err());
        assert!(parse_strs(&["a.jar", "--fast"]).is_err());
        assert!(parse_strs(&["a.jar", "--every", "2"]).is_err());
        assert!(parse_strs(&["a.jar", "--record", "out", "--every", "0"]).is_err());
    }

    /// A JAR of stored entries, enough for the runner to open.
    fn jar(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for (name, data) in entries {
            let offset = out.len() as u32;
            let mut header = vec![20, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            header.extend_from_slice(&[0; 4]); // the CRC, which readers ignore
            header.extend_from_slice(&(data.len() as u32).to_le_bytes());
            header.extend_from_slice(&(data.len() as u32).to_le_bytes());
            header.extend_from_slice(&(name.len() as u16).to_le_bytes());
            header.extend_from_slice(&[0, 0]);
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&header);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            directory.extend_from_slice(b"PK\x01\x02\x14\x00");
            directory.extend_from_slice(&header);
            directory.extend_from_slice(&[0; 10]);
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(name.as_bytes());
        }
        let start = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(b"PK\x05\x06\x00\x00\x00\x00");
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    #[test]
    fn a_game_that_cannot_start_prints_the_summary_and_fails() {
        let dir = std::env::temp_dir().join(format!("fastemu-run-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("snake.jar");
        let manifest: &[u8] =
            b"MIDlet-Name: Snake\nMIDlet-Version: 1.2\nMIDlet-1: Snake, , no.such.Game\n";
        std::fs::write(&path, jar(&[("META-INF/MANIFEST.MF", manifest)])).unwrap();
        let options = parse([path.clone().into_os_string()]).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run(&options, &mut out, &mut err), 1);
        let out = String::from_utf8(out).unwrap();
        let err = String::from_utf8(err).unwrap();
        assert!(
            out.starts_with("loaded snake.jar (Snake 1.2)\nprofile generic-240x320, 1 instance\n"),
            "{out}"
        );
        assert!(out.ends_with("ran 0 frames (0 drawn), 1 errors\n"), "{out}");
        assert!(err.contains("the game crashed"), "{err}");
        assert!(err.contains("no.such.Game"), "{err}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_suite_fails_with_one() {
        let options = parse_strs(&["no-such-game.jar"]).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run(&options, &mut out, &mut err), 1);
        assert!(String::from_utf8_lossy(&err).contains("no-such-game.jar"));
    }

    #[test]
    fn a_game_name_that_holds_escape_sequences_is_printed_escaped() {
        let dir = std::env::temp_dir().join(format!("fastemu-run-escape-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("evil.jar");
        // A name that sets the terminal's title and a version that clears
        // its screen, then one with a right to left override.
        let manifest: &[u8] = b"MIDlet-Name: Evil\x1b]0;owned\x07\n\
            MIDlet-Version: 1.\x1b[2J\xe2\x80\xae0\n\
            MIDlet-1: Evil, , no.such.Game\n";
        std::fs::write(&path, jar(&[("META-INF/MANIFEST.MF", manifest)])).unwrap();
        let options = parse([path.into_os_string()]).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run(&options, &mut out, &mut err), 1);
        let out = String::from_utf8(out).unwrap();
        let err = String::from_utf8(err).unwrap();
        assert!(
            out.starts_with(
                "loaded evil.jar (Evil\u{fffd}]0;owned\u{fffd} 1.\u{fffd}[2J\u{fffd}0)\n"
            ),
            "{out:?}"
        );
        for text in [&out, &err] {
            assert!(
                !text.chars().any(|c| c.is_control() && c != '\n') && !text.contains('\u{202e}'),
                "{text:?}"
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_name_that_holds_escape_sequences_is_printed_escaped() {
        let name = "evil\u{1b}]0;owned\u{7}\u{202e}.jar";
        let options = parse_strs(&[name]).unwrap();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        assert_eq!(run(&options, &mut out, &mut err), 1);
        let err = String::from_utf8(err).unwrap();
        assert!(
            err.starts_with("fastemu: evil\u{fffd}]0;owned\u{fffd}\u{fffd}.jar: "),
            "{err:?}"
        );
        assert!(
            !err.trim_end().chars().any(char::is_control) && !err.contains('\u{202e}'),
            "{err:?}"
        );
        // So are the command line's own complaints.
        let complaint = parse_strs(&["--evil\u{1b}[2J"]).unwrap_err();
        assert!(complaint.contains('\u{1b}'), "the parser keeps the text");
        assert_eq!(shown(complaint), "unknown option --evil\u{fffd}[2J");
    }
}
