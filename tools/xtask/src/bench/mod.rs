//! `cargo xtask bench`: measures FastEmu against MicroEmulator 2.0.4 on
//! Windows, the same way on both, with no person and no window, and
//! writes `bench.md` and `bench.json`. docs/benchmark.md has the method.
//!
//! - [`java`] builds the benchmark MIDlet (CPU, memory and drawing
//!   kernels, an unpaced frame loop, and a scene at 30 frames a second)
//!   and MicroEmulator's wrapper, MeBench.
//! - [`child`] is FastEmu's side, `xtask bench-run`, in a process of its
//!   own; MeBench is MicroEmulator's. Both write the lines of [`wire`].
//! - [`keys`] is the key script both press in real games.
//! - [`sample`] reads CPU time and memory of each process on Windows.
//! - [`measure`] runs the measurements; [`windowed`] the pair with
//!   windows, only with `--windows`.
//! - [`report`] writes the results.
//!
//! The runtime is measured, not changed: nothing here is in the crates.

pub mod child;
mod java;
mod keys;
mod measure;
mod report;
mod sample;
mod windowed;
mod wire;

use std::path::{Path, PathBuf};

/// What `cargo xtask bench` was asked to do.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The unpacked microemulator-2.0.4 folder.
    pub microemu: Option<PathBuf>,
    /// A folder of game JARs, not searched below.
    pub games: Option<PathBuf>,
    pub runs: usize,
    pub out: PathBuf,
    pub fastemu_only: bool,
    pub build_only: bool,
    /// Measure the desktop app and MicroEmulator's windows instead.
    pub windows: bool,
    /// The desktop app, for `--windows`.
    pub app: Option<PathBuf>,
    /// The java that runs MicroEmulator (default: JAVA_HOME's).
    pub me_java: Option<PathBuf>,
    /// More JVM options for every MicroEmulator process.
    pub me_jvm_args: Vec<String>,
    /// Leaves out `-Djava.awt.headless=true`, for an AWT that cannot
    /// load images without a display. MeBench opens no window either way.
    pub me_no_headless_awt: bool,
    /// Adds rows of MicroEmulator capped at this frame rate.
    pub me_max_fps: Option<u32>,
}

impl Options {
    /// Parses the arguments after `bench`; `out` is the default output
    /// folder.
    pub fn parse(args: &[String], out: PathBuf) -> Result<Self, String> {
        let mut options = Self {
            microemu: None,
            games: None,
            runs: 4,
            out,
            fastemu_only: false,
            build_only: false,
            windows: false,
            app: None,
            me_java: None,
            me_jvm_args: Vec::new(),
            me_no_headless_awt: false,
            me_max_fps: None,
        };
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            let mut value = |name: &str| {
                iter.next()
                    .cloned()
                    .ok_or_else(|| format!("{name} needs a value"))
            };
            match arg.as_str() {
                "--microemu" => options.microemu = Some(value(arg)?.into()),
                "--games" => options.games = Some(value(arg)?.into()),
                "--runs" => {
                    let n = value(arg)?;
                    options.runs = n
                        .parse()
                        .ok()
                        .filter(|&n| n > 0)
                        .ok_or_else(|| format!("bad number of runs {n}"))?;
                }
                "--out" => options.out = value(arg)?.into(),
                "--fastemu-only" => options.fastemu_only = true,
                "--build-only" => options.build_only = true,
                "--windows" => options.windows = true,
                "--app" => options.app = Some(value(arg)?.into()),
                "--me-java" => options.me_java = Some(value(arg)?.into()),
                "--me-jvm-arg" => options.me_jvm_args.push(value(arg)?),
                "--me-no-headless-awt" => options.me_no_headless_awt = true,
                "--me-max-fps" => {
                    let n = value(arg)?;
                    options.me_max_fps = Some(
                        n.parse()
                            .ok()
                            .filter(|&n| n > 0)
                            .ok_or_else(|| format!("bad frame rate {n}"))?,
                    );
                }
                other => return Err(format!("unknown argument {other}")),
            }
        }
        if options.microemu.is_none() && !options.fastemu_only && !options.build_only {
            return Err(
                "bench needs --microemu DIR, the unpacked microemulator-2.0.4 folder, \
                 unless --fastemu-only or --build-only"
                    .into(),
            );
        }
        if options.windows && options.app.is_none() {
            return Err("--windows needs --app PATH, the desktop app to start".into());
        }
        if options.windows && options.build_only {
            return Err("--build-only runs nothing, so not --windows".into());
        }
        if options.windows && options.games.is_none() {
            return Err("--windows needs --games DIR".into());
        }
        Ok(options)
    }
}

/// A file name for `jar`'s results, unique among `taken`.
pub fn slug(jar: &Path, taken: &mut Vec<String>) -> String {
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

/// Refuses to measure where the numbers would mislead: in a debug build,
/// which runs games several times slower, and off Windows, whose process
/// counters the bench reads.
fn may_measure() -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Err("bench measures only in a release build: run it with \
             `cargo run --release -p xtask -- bench ...` (--build-only works in any build)"
            .into());
    }
    if !cfg!(windows) {
        return Err(
            "bench reads Windows process counters, so it measures only on Windows \
             (--build-only works everywhere)"
                .into(),
        );
    }
    Ok(())
}

/// Runs `cargo xtask bench`. `root` is the repository.
pub fn run(options: &Options, root: &Path) -> Result<(), String> {
    if !options.build_only {
        may_measure()?;
    }
    let built = java::build(&options.out, root, options.microemu.as_deref())?;
    if options.build_only {
        println!(
            "built {}, {} and {}, and MeBench in {} (against {})",
            built.bench_jar.display(),
            built.scene_jar.display(),
            built.check_jar.display(),
            built.mebench.display(),
            if built.mebench_real {
                "microemulator.jar"
            } else {
                "the stand-ins"
            }
        );
        return Ok(());
    }
    let report = measure::run(options, &built, root)?;
    let md = options.out.join("bench.md");
    let json = options.out.join("bench.json");
    std::fs::write(&md, report::markdown(&report)).map_err(|e| format!("{}: {e}", md.display()))?;
    std::fs::write(&json, report::json(&report)).map_err(|e| format!("{}: {e}", json.display()))?;
    println!("wrote {} and {}", md.display(), json.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use report::Side;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn options_and_child_args_round_trip() {
        let out = PathBuf::from("target/bench");
        // MicroEmulator's folder is needed unless only FastEmu runs or
        // nothing runs; --windows needs the app.
        assert!(Options::parse(&[], out.clone()).is_err());
        assert!(Options::parse(&args("--fastemu-only"), out.clone()).is_ok());
        assert!(Options::parse(&args("--build-only"), out.clone()).is_ok());
        assert!(Options::parse(&args("--microemu me --windows --games g"), out.clone()).is_err());
        assert!(
            Options::parse(
                &args("--microemu me --windows --games g --app a.exe"),
                out.clone()
            )
            .is_ok()
        );
        assert!(Options::parse(&args("--fastemu-only --runs 0"), out.clone()).is_err());
        assert!(Options::parse(&args("--fastemu-only --what"), out.clone()).is_err());
        assert!(Options::parse(&args("--fastemu-only --me-max-fps x"), out.clone()).is_err());
        let o = Options::parse(
            &args(
                "--microemu me --games g --runs 2 --out o --me-jvm-arg -XX:-BytecodeVerificationRemote \
                 --me-jvm-arg -Xshare:off --me-no-headless-awt --me-max-fps 60 --me-java j.exe",
            ),
            out.clone(),
        )
        .unwrap();
        assert_eq!(o.runs, 2);
        assert_eq!(o.out, PathBuf::from("o"));
        assert_eq!(
            o.me_jvm_args,
            ["-XX:-BytecodeVerificationRemote", "-Xshare:off"]
        );
        assert!(o.me_no_headless_awt);
        assert_eq!(o.me_max_fps, Some(60));
        assert_eq!(o.me_java, Some(PathBuf::from("j.exe")));
        assert_eq!(
            Options::parse(&args("--fastemu-only"), out.clone())
                .unwrap()
                .runs,
            4
        );

        // Every MicroEmulator process gets the same options, the cap only
        // on the capped rows, and the small heap only on its own.
        let line = |side| {
            measure::microemu_args(
                &o,
                side,
                Path::new("home"),
                std::ffi::OsStr::new("cp"),
                45.0,
                "1500:-5:p",
                Some((40.0, Path::new("shot.png"))),
                Path::new("g.jar"),
            )
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ")
        };
        assert_eq!(
            line(Side::MicroEmuCapped),
            "-XX:-BytecodeVerificationRemote -Xshare:off -Duser.home=home -cp cp \
             fastemu.bench.MeBench --seconds 45 --events 1500:-5:p --max-fps 60 \
             --shot-at 40 --shot shot.png --resizableDevice 240 320 g.jar"
        );
        let plain = Options::parse(&args("--microemu me"), out).unwrap();
        let small = measure::microemu_args(
            &plain,
            Side::MicroEmuSmallHeap,
            Path::new("h"),
            std::ffi::OsStr::new("cp"),
            5.0,
            "",
            None,
            Path::new("s.jar"),
        );
        let small: Vec<String> = small
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            small.join(" "),
            "-Xms16m -Xmx128m -XX:+UseSerialGC -Djava.awt.headless=true -Duser.home=h -cp cp \
             fastemu.bench.MeBench --seconds 5 --resizableDevice 240 320 s.jar"
        );

        // FastEmu's runner reads back what the bench writes.
        let run = child::Options {
            suites: vec!["a.jar".into(), "b.jar".into()],
            seconds: 60.0,
            max_fps: Some(0),
            keys: keys::BENCH_KEYS.into(),
            profile: "generic-240x320".into(),
            data: "data".into(),
            shot_at: Some(55.0),
            shots: Some("shots".into()),
        };
        let printed: Vec<String> = run
            .args()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(child::Options::parse(&printed).unwrap(), run);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn measuring_refuses_a_debug_build() {
        let out = std::env::temp_dir().join(format!("fastemu-bench-debug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let options = Options::parse(
            &[
                "--fastemu-only".to_owned(),
                "--out".to_owned(),
                out.display().to_string(),
            ],
            PathBuf::new(),
        )
        .unwrap();
        let refused = run(&options, Path::new(".")).unwrap_err();
        assert!(refused.contains("--release"), "{refused}");
        // Nothing was built or started.
        assert!(!out.exists());
    }

    #[test]
    fn slugs_are_unique() {
        let mut taken = Vec::new();
        assert_eq!(
            slug(Path::new("a/Star Defender.jar"), &mut taken),
            "star-defender"
        );
        assert_eq!(
            slug(Path::new("b/star_defender.JAR"), &mut taken),
            "star-defender-2"
        );
    }
}
