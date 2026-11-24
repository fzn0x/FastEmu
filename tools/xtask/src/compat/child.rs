//! The runner of one game: `cargo xtask compat` starts this program again
//! as `compat-run` for every JAR, so a game that hangs the runner can be
//! killed without losing the others. It launches the game with the real
//! launcher and no sound device, plays the key script from the first
//! frame, and writes [`Record`]s to its standard output as things happen.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fastemu_core::{
    Frame, Instance, InstanceSettings, InstanceStatus, LaunchRequest, Launcher, LogLevel,
    builtin_profiles,
};
use fastemu_midp::MidpLauncher;

use super::script::{self, Player};
use super::wire::Record;

/// How long a stopping game gets to wind down.
const STOP_WAIT: Duration = Duration::from_secs(5);
/// A game that presents no new frame for this long by the end of its run,
/// though the key script pressed keys, counts as hung: a player sees a
/// frozen screen.
const STALL: Duration = Duration::from_secs(10);

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
    emit(out, &Record::Errors(instance.error_count()));
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
            let paused = if paused { "; the game is paused" } else { "" };
            if options.seconds - at >= STALL.as_secs_f64() {
                end(out, "hung", format!("no new frame after {at:.1} s{paused}"));
            } else {
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
