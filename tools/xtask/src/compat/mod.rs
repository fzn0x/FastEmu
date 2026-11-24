//! `cargo xtask compat DIR`: runs every JAR under a folder without a
//! window for a number of seconds, pressing a short key script, and
//! writes what each game did as Markdown and JSON.
//!
//! Each game runs in a process of its own (this program again, as
//! `compat-run`, see [`child`]), so a game that hangs the runner is
//! killed after a deadline and the others still run. The runner reports
//! as it goes ([`wire`]); [`report`] turns that into a status (`runs`,
//! `crashed`, `exited`, `hung`), the frames presented, the exceptions,
//! the classes and members the game asked for and FastEmu lacks (the
//! virtual machine's diagnostics), a screenshot and the log.

pub mod child;
mod report;
mod script;
mod wire;

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use report::Game;
use wire::Record;

/// The key script a run presses when none is given: 5 after a pause (the
/// key most title screens wait for), then the arrows and fire.
pub const DEFAULT_KEYS: &str = "wait:1500 5 wait:1000 up wait:300 down wait:300 left wait:300 \
     right wait:300 fire wait:1000 5 wait:1000 fire";

/// How long past its own run time a runner may take before it is killed.
const GRACE: Duration = Duration::from_secs(30);

/// What `cargo xtask compat` was asked to do.
pub struct Options {
    /// The folder whose JARs run.
    pub dir: PathBuf,
    /// Where the report, screenshots and logs go.
    pub out: PathBuf,
    pub seconds: f64,
    pub keys: String,
    pub profile: String,
    /// How many games run at once.
    pub jobs: usize,
}

/// Every `.jar` under `dir`, sorted, skipping folders whose names start
/// with a dot.
fn jars(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let hidden = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if path.is_dir() && !hidden {
            jars(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("jar"))
        {
            out.push(path);
        }
    }
    out.sort();
}

/// A file name for `jar`'s screenshot and log, unique among `taken`.
fn slug(jar: &Path, taken: &mut Vec<String>) -> String {
    let stem = jar
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let base: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let mut name = base.clone();
    let mut n = 2;
    while taken.contains(&name) {
        name = format!("{base}-{n}");
        n += 1;
    }
    taken.push(name.clone());
    name
}

/// Runs one game in a runner process and collects its records.
fn run_one(options: &Options, jar: &Path, slug: &str) -> Result<Game, String> {
    let screenshot = options.out.join("screens").join(format!("{slug}.png"));
    let _ = std::fs::remove_file(&screenshot);
    let child_options = child::Options {
        suite: jar.to_path_buf(),
        seconds: options.seconds,
        profile: options.profile.clone(),
        keys: options.keys.clone(),
        screenshot,
        data: options.out.join("data").join(slug),
    };
    let exe = std::env::current_exe().map_err(|e| format!("finding this program: {e}"))?;
    let mut process = Command::new(exe)
        .arg("compat-run")
        .args(child_options.args())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("starting the runner: {e}"))?;
    let records: Arc<Mutex<Vec<Record>>> = Arc::default();
    let stdout = process.stdout.take().expect("the runner's output is piped");
    let stderr = process
        .stderr
        .take()
        .expect("the runner's errors are piped");
    let sink = Arc::clone(&records);
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(record) = Record::parse(&line) {
                sink.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(record);
            }
        }
    });
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            text.push_str(&line);
            text.push('\n');
        }
        text
    });
    let deadline = Instant::now() + Duration::from_secs_f64(options.seconds) + GRACE;
    let mut killed = None;
    loop {
        match process.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = process.kill();
                let _ = process.wait();
                killed = Some(format!(
                    "the runner did not finish within {:.0} s and was killed",
                    options.seconds + GRACE.as_secs_f64()
                ));
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(e) => return Err(format!("waiting for the runner: {e}")),
        }
    }
    let _ = reader.join();
    let stderr = errors.join().unwrap_or_default();
    let records = records
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let file = jar
        .strip_prefix(&options.dir)
        .unwrap_or(jar)
        .to_string_lossy()
        .replace('\\', "/");
    let game = Game::from_records(file, slug, &records, killed, stderr.trim());
    let log = options.out.join("logs").join(format!("{slug}.log"));
    std::fs::write(&log, report::log_text(&records))
        .map_err(|e| format!("{}: {e}", log.display()))?;
    Ok(game)
}

/// Runs every JAR under `options.dir` and writes the report.
pub fn run(options: &Options) -> Result<(), String> {
    let mut found = Vec::new();
    jars(&options.dir, &mut found);
    if found.is_empty() {
        return Err(format!("no .jar under {}", options.dir.display()));
    }
    for sub in ["screens", "logs", "data"] {
        let dir = options.out.join(sub);
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut taken = Vec::new();
    let work: Vec<(PathBuf, String)> = found
        .into_iter()
        .map(|jar| {
            let slug = slug(&jar, &mut taken);
            (jar, slug)
        })
        .collect();
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<Result<Game, String>>>> =
        Mutex::new((0..work.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..options.jobs.clamp(1, work.len()) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    let Some((jar, slug)) = work.get(i) else {
                        break;
                    };
                    let result = run_one(options, jar, slug);
                    if let Ok(game) = &result {
                        println!("{}: {} ({} frames)", game.file, game.status, game.frames);
                    }
                    results
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)[i] = Some(result);
                }
            });
        }
    });
    let games: Vec<Game> = results
        .into_inner()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .into_iter()
        .flatten()
        .collect::<Result<_, _>>()?;
    let _ = std::fs::remove_dir_all(options.out.join("data"));
    let md = options.out.join("compat.md");
    let json = options.out.join("compat.json");
    std::fs::write(&md, report::markdown(options, &games))
        .map_err(|e| format!("{}: {e}", md.display()))?;
    std::fs::write(&json, report::json(options, &games))
        .map_err(|e| format!("{}: {e}", json.display()))?;
    println!("wrote {} and {}", md.display(), json.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_file_names() {
        let mut taken = Vec::new();
        assert_eq!(
            slug(Path::new("a/Retro Snake.jar"), &mut taken),
            "retro-snake"
        );
        assert_eq!(
            slug(Path::new("b/retro_snake.JAR"), &mut taken),
            "retro-snake-2"
        );
    }

    #[test]
    fn the_default_script_parses() {
        assert!(script::parse(DEFAULT_KEYS).is_ok());
    }
}
