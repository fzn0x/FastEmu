//! `cargo xtask bench --android`: measures FastEmu's app against J2ME
//! Loader on an Android emulator, the same way on both, with no window and
//! no tap, and writes `android.md` and `android.json`. docs/benchmark.md
//! has the method.
//!
//! - [`device`] starts the SDK's emulator headless and talks to it
//!   through adb.
//! - [`apps`] installs both apps, gives them the JARs and starts a game in
//!   each with no tap.
//! - [`measure`] runs the measurements; [`report`] writes the results.
//! - [`parse`] reads what the emulator prints.
//!
//! FastEmu's side needs the bench log of `crates/fastemu-android`, which
//! `log.tag.FastEmuBench` turns on: an APK of FastEmu 0.1.1 has none.

mod apps;
mod device;
mod measure;
mod parse;
mod report;

use std::path::{Path, PathBuf};

use fastemu_midp::suite::ZipArchive;

use super::java::Built;
use super::measure::game_jars;
use super::report::Work;
use super::{Options, slug};
use apps::{App, Suite};
use device::{Emulator, Sdk};

/// What `--android` adds to the bench's options.
#[derive(Clone, Debug, PartialEq)]
pub struct AndroidOptions {
    /// FastEmu's APK, with the bench log.
    pub fastemu_apk: PathBuf,
    /// J2ME Loader's APK, `J2ME_Loader-1.9.0-open-release.apk`.
    pub j2ml_apk: PathBuf,
    pub avd: String,
    /// The emulator's `-gpu` mode.
    pub gpu: String,
    pub sdk: Option<PathBuf>,
}

/// The AVD the bench uses unless told another.
pub const DEFAULT_AVD: &str = "Pixel_8_API_35";
/// The emulator's GPU mode unless told another: the host's GPU for both
/// apps' drawing.
pub const DEFAULT_GPU: &str = "host";

/// One entry of a JAR or an APK, as text.
fn zip_text(path: &Path, name: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let zip = ZipArchive::new(bytes).map_err(|e| format!("{}: {e:?}", path.display()))?;
    let entry = zip
        .find_str(name)
        .ok_or_else(|| format!("{} has no {name}", path.display()))?;
    let data = zip
        .read(entry)
        .map_err(|e| format!("{} {name}: {e:?}", path.display()))?;
    Ok(String::from_utf8_lossy(&data).into_owned())
}

/// The `MIDlet-Name` of a JAR's manifest.
fn midlet_name(jar: &Path) -> Result<String, String> {
    let manifest = zip_text(jar, "META-INF/MANIFEST.MF")?;
    manifest
        .lines()
        .find_map(|l| l.strip_prefix("MIDlet-Name:").map(|v| v.trim().to_owned()))
        .ok_or_else(|| format!("{} names no MIDlet-Name", jar.display()))
}

fn dir(path: PathBuf) -> Result<PathBuf, String> {
    std::fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(path)
}

/// Runs the Android bench. `options.out` is claimed already, and `built`
/// holds the benchmark MIDlet's JARs.
pub fn run(options: &Options, android: &AndroidOptions, built: &Built) -> Result<(), String> {
    // Everything that can be refused is refused before the emulator starts.
    apps::check_j2ml_apk(&android.j2ml_apk)?;
    if !android.fastemu_apk.is_file() {
        return Err(format!(
            "no FastEmu APK at {}",
            android.fastemu_apk.display()
        ));
    }
    let system_properties = zip_text(&android.j2ml_apk, "assets/defaults/system.props")?;
    let games: Vec<PathBuf> = match &options.games {
        Some(dir) => game_jars(dir)?,
        None => Vec::new(),
    };
    let sdk = Sdk::find(android.sdk.as_deref())?;
    let out = dir(options.out.join("android"))?;
    let logs = dir(out.join("logs"))?;
    let shots = dir(out.join("shots"))?;
    let scratch = dir(out.join("scratch"))?;
    let config = scratch.join("j2meloader-config.json");
    std::fs::write(&config, parse::j2meloader_config(0, &system_properties))
        .map_err(|e| format!("{}: {e}", config.display()))?;
    let key_script = measure::bench_key_script()?;

    println!("starting {} with no window", android.avd);
    let emulator = Emulator::start(&sdk, &android.avd, &android.gpu, &logs.join("emulator.log"))?;
    let result = (|| {
        emulator.settle()?;
        let device = &emulator.device;
        apps::install(device, &android.fastemu_apk, &android.j2ml_apk)?;
        let mut facts = report::facts(&sdk, &emulator, android)?;
        facts.push((
            "J2ME Loader's settings".into(),
            "the defaults of a new game, written to its config.json: 240x320, GLES drawing, \
             no frame limit, keypad shown"
                .into(),
        ));

        // Each JAR once on the device, and converted once in J2ME Loader.
        let mut taken = Vec::new();
        let mut refused = Vec::new();
        let mut prepare = |jar: &Path| -> Result<(String, Suite), String> {
            let slug = slug(jar, &mut taken);
            println!("preparing {slug}");
            let media_id = apps::push(device, jar, &slug)?;
            let name = midlet_name(jar)?;
            // A game J2ME Loader cannot install runs on FastEmu alone.
            let j2ml_dir = match apps::j2ml_convert(device, media_id, &config, &scratch)? {
                Ok(dir) => Some(dir),
                Err(why) => {
                    println!("  J2ME Loader could not install it: {why}");
                    refused.push((name.clone(), why));
                    None
                }
            };
            Ok((
                slug,
                Suite {
                    media_id,
                    j2ml_dir,
                    name,
                },
            ))
        };
        let (_, kernels) = prepare(&built.bench_jar)?;
        let (_, scene) = prepare(&built.scene_jar)?;
        let mut game_suites = Vec::new();
        for jar in &games {
            game_suites.push(prepare(jar)?);
        }

        let bench = measure::Bench {
            device,
            logs: &logs,
            shots: &shots,
            key_script,
        };
        let mut works: Vec<(Work, Vec<&Suite>)> =
            vec![(Work::Kernels, vec![&kernels]), (Work::Scene, vec![&scene])];
        for (slug, suite) in &game_suites {
            works.push((Work::Game(slug.clone()), vec![suite]));
        }
        if game_suites.len() > 1 {
            works.push((Work::Eight, game_suites.iter().map(|(_, s)| s).collect()));
        }
        let mut measurements = Vec::new();
        for run in 0..options.runs {
            for (i, (work, suites)) in works.iter().enumerate() {
                // J2ME Loader runs one game at a time, and only the games
                // it could install.
                let order: Vec<App> =
                    if matches!(work, Work::Eight) || suites.iter().any(|s| s.j2ml_dir.is_none()) {
                        vec![App::FastEmu]
                    } else if (run + i) % 2 == 1 {
                        vec![App::FastEmu, App::J2meLoader]
                    } else {
                        vec![App::J2meLoader, App::FastEmu]
                    };
                for &app in &order {
                    println!(
                        "run {} of {}: {} on {}",
                        run + 1,
                        options.runs,
                        work.key(),
                        app.label()
                    );
                    let m = bench.measure(work, app, run, order[0], suites);
                    for problem in &m.problems {
                        println!("  {problem}");
                    }
                    measurements.push(m);
                }
            }
        }
        let games = game_suites
            .iter()
            .map(|(slug, suite)| (slug.clone(), suite.name.clone()))
            .collect();
        Ok::<_, String>(report::Report {
            facts,
            games,
            refused,
            measurements,
            runs: options.runs,
        })
    })();
    emulator.stop();
    let report = result?;
    let md = out.join("android.md");
    let json = out.join("android.json");
    std::fs::write(&md, report::markdown(&report)).map_err(|e| format!("{}: {e}", md.display()))?;
    std::fs::write(&json, report::json(&report)).map_err(|e| format!("{}: {e}", json.display()))?;
    println!("wrote {} and {}", md.display(), json.display());
    Ok(())
}
