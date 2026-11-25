//! FastEmu's side of a bench: `xtask bench-run`, the process `cargo
//! xtask bench` starts for each FastEmu measurement.
//!
//! It runs one or more games in one process, as the app does: with the
//! real launcher, the app's sound engine (on a silent output, so the mixer
//! works as it does for a player but nothing reaches the speakers), and
//! the frame cap it is given. It writes [`Line`]s as things happen: the
//! first frame of each game the moment it comes, then once a second the
//! frames, the log, and finally how each game ended.
//!
//! Until a game shows its first frame the runner wakes on the game's own
//! notification, so the first frame's time is exact. After it, the
//! runner ignores the notifications, which come with every frame, and
//! reads the game only once a second and when a key is due: as little
//! work beside the game as MicroEmulator's wrapper does.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use fastemu_audio::{Engine, NullOutput};
use fastemu_core::{
    Frame, InputEvent, Instance, InstanceSettings, InstanceStatus, KeyAction, LaunchRequest,
    LogLevel, Notify, builtin_profiles,
};
use fastemu_midp::{MidpInstance, MidpLauncher};

use super::keys::{self, KeyStep};
use super::wire::Line;

/// How long the games get to stop at the end.
const STOP_WAIT: Duration = Duration::from_secs(5);
/// How often the runner reports frames and the log.
const TICK: Duration = Duration::from_secs(1);

/// What one `bench-run` is asked to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The games, each started in this process.
    pub suites: Vec<PathBuf>,
    /// How long the run lasts, unless every game ends sooner.
    pub seconds: f64,
    /// The frame cap: `None` keeps the runtime's 60, `Some(0)` lifts it.
    pub max_fps: Option<u32>,
    /// The key script, from each game's first frame.
    pub keys: String,
    pub profile: String,
    /// Where the games keep their record stores, a folder each.
    pub data: PathBuf,
    /// When to save each game's screen, in seconds, and where.
    pub shot_at: Option<f64>,
    pub shots: Option<PathBuf>,
}

impl Options {
    /// Parses the arguments after `bench-run`.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self {
            suites: Vec::new(),
            seconds: 30.0,
            max_fps: None,
            keys: String::new(),
            profile: "generic-240x320".into(),
            data: PathBuf::from("bench-data"),
            shot_at: None,
            shots: None,
        };
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if !arg.starts_with("--") {
                options.suites.push(PathBuf::from(arg));
                continue;
            }
            let value = iter
                .next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))?;
            let seconds = |v: &str| {
                v.parse::<f64>()
                    .ok()
                    .filter(|s| s.is_finite() && *s >= 0.0)
                    .ok_or_else(|| format!("bad number of seconds {v}"))
            };
            match arg.as_str() {
                "--seconds" => options.seconds = seconds(&value)?,
                "--max-fps" => {
                    options.max_fps = Some(
                        value
                            .parse()
                            .map_err(|_| format!("bad frame rate {value}"))?,
                    );
                }
                "--keys" => options.keys = value,
                "--profile" => options.profile = value,
                "--data" => options.data = value.into(),
                "--shot-at" => options.shot_at = Some(seconds(&value)?),
                "--shots" => options.shots = Some(value.into()),
                other => return Err(format!("unknown argument {other}")),
            }
        }
        if options.suites.is_empty() {
            return Err("bench-run needs a suite".into());
        }
        Ok(options)
    }

    /// The arguments [`Options::parse`] reads back.
    pub fn args(&self) -> Vec<std::ffi::OsString> {
        let mut args: Vec<std::ffi::OsString> =
            self.suites.iter().map(|s| s.clone().into()).collect();
        let mut pair = |name: &str, value: std::ffi::OsString| {
            args.push(name.into());
            args.push(value);
        };
        pair("--seconds", self.seconds.to_string().into());
        if let Some(fps) = self.max_fps {
            pair("--max-fps", fps.to_string().into());
        }
        pair("--keys", self.keys.clone().into());
        pair("--profile", self.profile.clone().into());
        pair("--data", self.data.clone().into_os_string());
        if let Some(at) = self.shot_at {
            pair("--shot-at", at.to_string().into());
        }
        if let Some(dir) = &self.shots {
            pair("--shots", dir.clone().into_os_string());
        }
        args
    }
}

fn emit(out: &mut impl Write, line: &Line) {
    let _ = writeln!(out, "{}", line.text());
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

/// What wakes the runner: a game's notification before its first frame.
#[derive(Default)]
struct Wake {
    flag: Mutex<bool>,
    cond: Condvar,
}

impl Wake {
    fn ring(&self) {
        *self.flag.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.cond.notify_all();
    }

    /// Waits until rung or `timeout` passed.
    fn wait(&self, timeout: Duration) {
        let guard = self.flag.lock().unwrap_or_else(PoisonError::into_inner);
        let (mut guard, _) = self
            .cond
            .wait_timeout_while(guard, timeout, |rung| !*rung)
            .unwrap_or_else(PoisonError::into_inner);
        *guard = false;
    }
}

/// One game of the run.
struct Game {
    instance: Option<Arc<MidpInstance>>,
    /// Set at the first frame: from then on its notifications are ignored.
    shown: Arc<AtomicBool>,
    first: Option<Instant>,
    next_key: usize,
    serial: u64,
    ended: bool,
}

impl Game {
    fn drain(&self, i: usize, out: &mut impl Write) {
        let Some(instance) = &self.instance else {
            return;
        };
        for line in instance.drain_log() {
            emit(
                out,
                &Line::Log {
                    game: i,
                    level: level(line.level).into(),
                    text: line.text.replace(['\n', '\r'], " "),
                },
            );
        }
    }
}

fn millis(since: Instant, now: Instant) -> u64 {
    now.duration_since(since).as_millis() as u64
}

/// Runs the games and writes their records to `out`.
pub fn run(options: &Options, out: &mut impl Write) -> Result<(), String> {
    let start = Instant::now();
    let profile = builtin_profiles()
        .into_iter()
        .find(|p| p.id == options.profile)
        .ok_or_else(|| format!("unknown profile {}", options.profile))?;
    let steps: Vec<KeyStep> = keys::steps(&options.keys)?;
    let engine = Engine::default();
    let _output = NullOutput::start(&engine);
    let launcher = MidpLauncher::new(Some(engine));
    let wake = Arc::new(Wake::default());
    if let Some(dir) = &options.shots {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut games = Vec::new();
    for (i, suite) in options.suites.iter().enumerate() {
        let data = options.data.join(format!("game-{i}"));
        let _ = std::fs::remove_dir_all(&data);
        let shown = Arc::new(AtomicBool::new(false));
        let notify: Notify = {
            let shown = Arc::clone(&shown);
            let wake = Arc::clone(&wake);
            Arc::new(move |_| {
                if !shown.load(Ordering::Acquire) {
                    wake.ring();
                }
            })
        };
        let request = LaunchRequest {
            profile: profile.clone(),
            settings: InstanceSettings {
                max_fps: options.max_fps,
                ..InstanceSettings::default()
            },
            notify: Some(notify),
            ..LaunchRequest::new(suite, &data)
        };
        let instance = match launcher.launch_midp(request) {
            Ok(instance) => Some(instance),
            Err(e) => {
                emit(
                    out,
                    &Line::Log {
                        game: i,
                        level: "err".into(),
                        text: format!("the launch failed: {e}"),
                    },
                );
                emit(
                    out,
                    &Line::End {
                        game: i,
                        status: "crashed".into(),
                        frames: 0,
                        errors: 0,
                        ms: millis(start, Instant::now()),
                    },
                );
                None
            }
        };
        let ended = instance.is_none();
        games.push(Game {
            instance,
            shown,
            first: None,
            next_key: 0,
            serial: 0,
            ended,
        });
    }
    let end_at = start + Duration::from_secs_f64(options.seconds);
    let shot_at = options
        .shots
        .as_ref()
        .and(options.shot_at)
        .map(|s| start + Duration::from_secs_f64(s));
    let mut shot_taken = shot_at.is_none();
    let mut next_tick = start + TICK;
    loop {
        let now = Instant::now();
        for (i, game) in games.iter_mut().enumerate() {
            let Some(instance) = game.instance.clone() else {
                continue;
            };
            if game.ended {
                continue;
            }
            if game.first.is_none() {
                if let Some(frame) = instance.frame() {
                    game.first = Some(now);
                    game.serial = frame.serial();
                    game.shown.store(true, Ordering::Release);
                    emit(
                        out,
                        &Line::First {
                            game: i,
                            ms: millis(start, now),
                        },
                    );
                }
                // A game that ends before its first frame ends now.
                if instance.status().is_finished() {
                    finish(i, game, &instance, start, out);
                    continue;
                }
            }
            if let Some(first) = game.first {
                while let Some(step) = steps.get(game.next_key) {
                    if first + Duration::from_millis(step.at_ms) > now {
                        break;
                    }
                    instance.send(InputEvent::Key {
                        key: step.key,
                        action: if step.press {
                            KeyAction::Press
                        } else {
                            KeyAction::Release
                        },
                    });
                    game.next_key += 1;
                }
            }
        }
        if now >= next_tick {
            next_tick += TICK;
            for (i, game) in games.iter_mut().enumerate() {
                let Some(instance) = game.instance.clone() else {
                    continue;
                };
                if game.ended {
                    continue;
                }
                game.drain(i, out);
                let size = instance.frame().map(|frame| {
                    game.serial = frame.serial();
                    (frame.width() as i32, frame.height() as i32)
                });
                emit(
                    out,
                    &Line::Frames {
                        game: i,
                        n: game.serial,
                        ms: millis(start, now),
                        size,
                    },
                );
                if instance.status().is_finished() {
                    finish(i, game, &instance, start, out);
                }
            }
        }
        if !shot_taken && shot_at.is_some_and(|at| now >= at) {
            shot_taken = true;
            if let Some(dir) = &options.shots {
                for (i, game) in games.iter().enumerate() {
                    let Some(frame) = game.instance.as_ref().and_then(|g| g.frame()) else {
                        continue;
                    };
                    let path = dir.join(format!("game-{i}.png"));
                    match save_png(&frame, &path) {
                        Ok(()) => emit(
                            out,
                            &Line::Shot {
                                game: i,
                                w: frame.width() as i32,
                                h: frame.height() as i32,
                            },
                        ),
                        Err(e) => emit(
                            out,
                            &Line::Log {
                                game: i,
                                level: "warn".into(),
                                text: format!("the screenshot failed: {e}"),
                            },
                        ),
                    }
                }
            }
        }
        if now >= end_at || games.iter().all(|g| g.ended) {
            break;
        }
        // The next thing due: the tick, a key, the screenshot or the end.
        let mut due = next_tick.min(end_at);
        if let Some(at) = shot_at.filter(|_| !shot_taken) {
            due = due.min(at);
        }
        for game in &games {
            if let (Some(first), Some(step), false) =
                (game.first, steps.get(game.next_key), game.ended)
            {
                due = due.min(first + Duration::from_millis(step.at_ms));
            }
        }
        wake.wait(due.saturating_duration_since(Instant::now()));
    }
    let now = Instant::now();
    for (i, game) in games.iter_mut().enumerate() {
        let Some(instance) = game.instance.clone() else {
            continue;
        };
        if !game.ended {
            game.drain(i, out);
            if let Some(frame) = instance.frame() {
                game.serial = frame.serial();
            }
            emit(
                out,
                &Line::End {
                    game: i,
                    status: "running".into(),
                    frames: game.serial,
                    errors: instance.error_count(),
                    ms: millis(start, now),
                },
            );
            game.ended = true;
        }
        instance.stop();
    }
    let deadline = Instant::now() + STOP_WAIT;
    for game in &games {
        if let Some(instance) = &game.instance {
            while !instance.status().is_finished() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    for i in 0..options.suites.len() {
        let _ = std::fs::remove_dir_all(options.data.join(format!("game-{i}")));
    }
    Ok(())
}

/// Reports how a game that ended itself ended.
fn finish(
    i: usize,
    game: &mut Game,
    instance: &MidpInstance,
    start: Instant,
    out: &mut impl Write,
) {
    game.drain(i, out);
    if let Some(frame) = instance.frame() {
        game.serial = frame.serial();
    }
    let status = match instance.status() {
        InstanceStatus::Crashed { message } => {
            emit(
                out,
                &Line::Log {
                    game: i,
                    level: "err".into(),
                    text: format!("the game crashed: {}", message.replace(['\n', '\r'], " ")),
                },
            );
            "crashed"
        }
        _ => "exited",
    };
    emit(
        out,
        &Line::End {
            game: i,
            status: status.into(),
            frames: game.serial,
            errors: instance.error_count(),
            ms: millis(start, Instant::now()),
        },
    );
    game.ended = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_print_and_parse_back() {
        let options = Options {
            suites: vec!["a b.jar".into(), "c.jad".into()],
            seconds: 45.5,
            max_fps: Some(0),
            keys: "wait:1500 fire".into(),
            profile: "generic-240x320".into(),
            data: "d a t a".into(),
            shot_at: Some(40.0),
            shots: Some("shots".into()),
        };
        let args: Vec<String> = options
            .args()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(Options::parse(&args).unwrap(), options);
        let plain = Options {
            max_fps: None,
            shot_at: None,
            shots: None,
            keys: String::new(),
            ..options
        };
        let args: Vec<String> = plain
            .args()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(Options::parse(&args).unwrap(), plain);
        assert!(Options::parse(&[]).is_err());
        assert!(Options::parse(&["a.jar".into(), "--max-fps".into(), "x".into()]).is_err());
        assert!(Options::parse(&["a.jar".into(), "--seconds".into()]).is_err());
        assert!(Options::parse(&["a.jar".into(), "--what".into(), "1".into()]).is_err());
    }
}
