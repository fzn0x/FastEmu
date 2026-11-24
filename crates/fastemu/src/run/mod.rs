//! `fastemu run`: one game without a window, for tests and scripts.
//!
//! It launches the suite with the real launcher and no audio output, runs
//! it for a number of frames or seconds while pressing the keys of a
//! script, prints the game's log as it comes, saves the last frame if
//! asked, and exits with 1 when the game crashed or stopped presenting
//! frames. No GPUI code runs on this path.

mod script;

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fastemu_core::{
    DeviceProfile, Frame, Instance, InstanceSettings, InstanceStatus, LaunchRequest, Launcher,
    LogLevel, builtin_profiles, default_profile,
};
use fastemu_midp::MidpLauncher;

use self::script::{Script, Step};

pub const USAGE: &str = "\
Usage: fastemu run SUITE [OPTIONS]

Runs a .jar or .jad without a window and prints what the game logs.

Options:
  --midlet N        Start MIDlet-<N> of the suite (default 1)
  --profile ID      Run on the phone profile ID (default generic-240x320)
  --frames N        Stop after N frames (default 300)
  --seconds S       Stop after S seconds instead
  --screenshot PATH Save the last frame as a PNG
  --keys SCRIPT     Press keys while the game runs (see below)
  --encoding NAME   The game's text encoding (default ISO-8859-1)

SCRIPT is a list of steps separated by commas or spaces, run in order
from the first frame: a key name taps the key, NAME:MS holds it for MS
milliseconds, and wait:MS (or a bare number) waits. When SCRIPT names a
file, the steps are read from it, one or more per line; # starts a
comment. Key names: 0 to 9, star, pound, up, down, left, right, fire,
softleft, softright, clear, send, end.

Exits with 1 when the game crashes or presents no new frame for 30
seconds, and with 2 for a bad command line.
";

/// How long the runner waits for a new frame before it gives up.
const STALL: Duration = Duration::from_secs(30);
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
    pub keys: Vec<Step>,
    pub encoding: Option<String>,
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
    let mut keys = Vec::new();
    let mut encoding = None;
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
            Some(option) if option.starts_with("--") => {
                return Err(format!("unknown option {option}"));
            }
            _ if suite.is_none() => suite = Some(PathBuf::from(arg)),
            _ => return Err("give one suite to run".to_owned()),
        }
    }
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
        keys,
        encoding,
    })
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

/// Runs `options`, printing to `out` and `err`, and returns the exit
/// code.
pub fn run(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let launcher = MidpLauncher::new(None);
    let file = options.suite.file_name().map_or_else(
        || options.suite.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let info = match launcher.inspect(&options.suite) {
        Ok(info) => info,
        Err(e) => {
            let _ = writeln!(err, "fastemu: {file}: {e}");
            return 1;
        }
    };
    let _ = writeln!(out, "loaded {file} ({} {})", info.name, info.version);
    let _ = writeln!(out, "profile {}, 1 instance", options.profile.id);
    let data_dir = std::env::temp_dir().join(format!(
        "fastemu-run-{}-{}",
        std::process::id(),
        fastemu_core::InstanceId::next().0
    ));
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
            let _ = writeln!(err, "fastemu: {file}: {e}");
            return 1;
        }
    };
    let code = drive(options, &*instance, out, err);
    let errors = instance.error_count();
    let frames = instance.frame().map_or(0, |f| f.serial());
    let last = instance.frame();
    instance.stop();
    let deadline = Instant::now() + STOP_WAIT;
    while !instance.status().is_finished() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    print_log(&*instance, out, err);
    let _ = std::fs::remove_dir_all(&data_dir);
    let _ = writeln!(out, "ran {frames} frames, {errors} errors");
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

/// Runs the game until its limit, a crash, its exit or a stall, and
/// returns the exit code so far.
fn drive(
    options: &Options,
    instance: &dyn Instance,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8 {
    let start = Instant::now();
    let mut script = Script::new(options.keys.clone());
    let mut serial = 0;
    let mut changed = start;
    loop {
        print_log(instance, out, err);
        let now = Instant::now();
        match instance.status() {
            InstanceStatus::Crashed { message } => {
                print_log(instance, out, err);
                let _ = writeln!(err, "fastemu: the game crashed: {message}");
                return 1;
            }
            InstanceStatus::Exited => {
                let _ = writeln!(out, "the game exited");
                return 0;
            }
            _ => {}
        }
        if let Some(frame) = instance.frame()
            && frame.serial() > serial
        {
            serial = frame.serial();
            changed = now;
        }
        if serial > 0 {
            script.advance(instance, now);
        }
        let done = match options.limit {
            Limit::Frames(n) => serial >= n,
            Limit::Seconds(s) => now.duration_since(start).as_secs_f64() >= s,
        };
        if done {
            return 0;
        }
        if now.duration_since(changed) > STALL {
            let _ = writeln!(
                err,
                "fastemu: no new frame for {} seconds; stopping",
                STALL.as_secs()
            );
            return 1;
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
            eprintln!("fastemu run: {e}\n\n{USAGE}");
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
        ])
        .unwrap();
        assert_eq!(options.midlet, 1);
        assert_eq!(options.profile.screen_width, 128);
        assert_eq!(options.limit, Limit::Seconds(1.5));
        assert_eq!(options.screenshot, Some(PathBuf::from("out.png")));
        assert_eq!(options.encoding.as_deref(), Some("UTF-8"));
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
        assert!(out.ends_with("ran 0 frames, 1 errors\n"), "{out}");
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
}
