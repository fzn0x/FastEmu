//! The two emulators on the device: FastEmu's app and J2ME Loader. How
//! each is installed, gets a JAR, starts it with no tap, and is reset
//! between measurements.

use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use super::device::Device;
use super::parse::{self, shell_quote};

pub const FASTEMU: &str = "io.github.fzn0x.fastemu";
pub const J2ML: &str = "ru.playsoftware.j2meloader";
/// The process J2ME Loader runs a game in.
pub const J2ML_GAME_PROCESS: &str = "ru.playsoftware.j2meloader:midlet";
/// The J2ME Loader the bench measures: its newest release when the bench
/// was written, `J2ME_Loader-1.9.0-open-release.apk` from its GitHub
/// releases, checked by this digest before it is installed.
pub const J2ML_VERSION: &str = "1.9.0";
pub const J2ML_SHA256: &str = "32f0cf8124d3e608e583799d604cbd7815a790e746f5c49ebd79e13247c85c96";
/// J2ME Loader's work folder on Android 11 and later.
const J2ML_FILES: &str = "/sdcard/Android/data/ru.playsoftware.j2meloader/files";
/// Where the bench puts the JARs both apps open.
const DROP: &str = "/sdcard/Download/fastemu-bench";
/// The longest J2ME Loader may take to convert one JAR.
const CONVERT_LIMIT: Duration = Duration::from_secs(180);

/// Which emulator a measurement ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum App {
    FastEmu,
    J2meLoader,
}

impl App {
    pub fn key(self) -> &'static str {
        match self {
            App::FastEmu => "fastemu",
            App::J2meLoader => "j2meloader",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            App::FastEmu => "FastEmu",
            App::J2meLoader => "J2ME Loader",
        }
    }

    pub fn package(self) -> &'static str {
        match self {
            App::FastEmu => FASTEMU,
            App::J2meLoader => J2ML,
        }
    }

    /// The process that runs the game.
    pub fn game_process(self) -> &'static str {
        match self {
            App::FastEmu => FASTEMU,
            App::J2meLoader => J2ML_GAME_PROCESS,
        }
    }
}

/// A JAR on the device, ready for both apps.
#[derive(Clone, Debug)]
pub struct Suite {
    /// Its MediaStore row, which FastEmu opens.
    pub media_id: u64,
    /// The folder J2ME Loader converted it into, under `converted/`, or
    /// `None` when J2ME Loader could not install it.
    pub j2ml_dir: Option<String>,
    /// Its `MIDlet-Name`.
    pub name: String,
}

/// Fails unless `apk` is the J2ME Loader the bench pins.
pub fn check_j2ml_apk(apk: &Path) -> Result<(), String> {
    let bytes = std::fs::read(apk).map_err(|e| format!("{}: {e}", apk.display()))?;
    let digest = parse::sha256_hex(&bytes);
    if digest == J2ML_SHA256 {
        Ok(())
    } else {
        Err(format!(
            "{} is not J2ME Loader {J2ML_VERSION}: its SHA-256 is {digest}, not {J2ML_SHA256}",
            apk.display()
        ))
    }
}

/// The `versionName` the package manager gives `package`.
pub fn version(device: &Device, package: &str) -> String {
    device
        .shell_any(&format!("dumpsys package {package}"))
        .lines()
        .find_map(|l| l.trim().strip_prefix("versionName=").map(str::to_owned))
        .unwrap_or_else(|| "unknown".into())
}

/// Installs both apps fresh, compiles each one's own code ahead of time
/// so neither starts interpreted, and turns on FastEmu's bench log.
pub fn install(device: &Device, fastemu_apk: &Path, j2ml_apk: &Path) -> Result<(), String> {
    for package in [FASTEMU, J2ML] {
        device.shell_any(&format!("pm uninstall {package}"));
    }
    device.install(fastemu_apk)?;
    device.install(j2ml_apk)?;
    for package in [FASTEMU, J2ML] {
        device.shell(&format!("cmd package compile -m speed -f {package}"))?;
    }
    device.shell(&format!(
        "setprop log.tag.{} DEBUG",
        parse::BENCH_LOG_MARK.trim_end_matches([':', ' '])
    ))?;
    device.shell(&format!("mkdir -p {DROP}"))?;
    Ok(())
}

/// Copies `jar` to the device as `<slug>.jar`, and returns its MediaStore
/// row. Both apps read it through a `content:` URI the shell grants.
pub fn push(device: &Device, jar: &Path, slug: &str) -> Result<u64, String> {
    let name = format!("{slug}.jar");
    device.push(jar, &format!("{DROP}/{name}"))?;
    let rows = device.shell(&format!(
        "content query --uri content://media/external/file --projection _id \
         --where {}",
        shell_quote(&format!("_display_name='{name}'"))
    ))?;
    parse::content_id(&rows).ok_or_else(|| format!("no MediaStore row for {name}: {rows}"))
}

fn uri(media_id: u64) -> String {
    format!("content://media/external/file/{media_id}")
}

/// The folders under J2ME Loader's `converted/`.
fn converted(device: &Device) -> Vec<String> {
    device
        .shell_any(&format!("ls {J2ML_FILES}/converted"))
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
}

/// The line in which J2ME Loader's converter gave up on a JAR, from the
/// `System.err` lines logcat holds.
fn convert_error(device: &Device) -> Option<String> {
    device
        .shell_any("logcat -d -v monotonic -s System.err:W")
        .lines()
        .find(|l| l.contains("Uncaught translation error") || l.contains("Dexing error"))
        .and_then(parse::logcat_line)
        .map(|l| l.msg)
}

/// Has J2ME Loader convert the JAR of `media_id` (its install with no
/// tap), then writes its settings with `config` so it starts with no
/// settings screen. Returns the folder it converted into, or, inside,
/// why J2ME Loader could not convert it.
pub fn j2ml_convert(
    device: &Device,
    media_id: u64,
    config: &Path,
    scratch: &Path,
) -> Result<Result<String, String>, String> {
    let before = converted(device);
    device.shell_any("logcat -c");
    device.shell(&format!(
        "am start -n {J2ML}/.MainActivity -a android.intent.action.VIEW \
         -t application/java-archive -d {} --grant-read-uri-permission",
        uri(media_id)
    ))?;
    let until = Instant::now() + CONVERT_LIMIT;
    let dir = loop {
        if let Some(dir) = converted(device).into_iter().find(|d| !before.contains(d)) {
            let conf = format!("{J2ML_FILES}/converted/{dir}/converted.dex.conf");
            if device
                .shell(&format!("test -f {}", shell_quote(&conf)))
                .is_ok()
            {
                break dir;
            }
        }
        if let Some(error) = convert_error(device) {
            device.shell(&format!("am force-stop {J2ML}"))?;
            return Ok(Err(error));
        }
        if Instant::now() > until {
            return Err(format!(
                "J2ME Loader did not convert JAR {media_id} in {} s",
                CONVERT_LIMIT.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(500));
    };
    // The app list's row is written just after the folder.
    thread::sleep(Duration::from_secs(2));
    device.shell(&format!("am force-stop {J2ML}"))?;
    let configs = format!("{J2ML_FILES}/configs/{dir}");
    device.shell(&format!("mkdir -p {}", shell_quote(&configs)))?;
    // adb push takes the remote path as one argument, spaces and all.
    let local = scratch.join("config.json");
    std::fs::copy(config, &local).map_err(|e| format!("{}: {e}", config.display()))?;
    device.push(&local, &format!("{configs}/config.json"))?;
    Ok(Ok(dir))
}

/// Stops both apps and clears what a game kept: FastEmu's data (library,
/// copies, saves) and J2ME Loader's saves of `suite`.
pub fn reset(device: &Device, suites: &[&Suite]) -> Result<(), String> {
    device.shell(&format!("am force-stop {FASTEMU}"))?;
    device.shell(&format!("am force-stop {J2ML}"))?;
    device.shell(&format!("pm clear {FASTEMU}"))?;
    for dir in suites.iter().filter_map(|s| s.j2ml_dir.as_ref()) {
        let data = format!("{J2ML_FILES}/data/{dir}");
        device.shell(&format!("rm -rf {}", shell_quote(&data)))?;
    }
    Ok(())
}

/// Starts `suite` in `app` the way the bench can with no tap: FastEmu
/// opens the JAR another app hands it; J2ME Loader starts a game it has
/// installed.
pub fn start(device: &Device, app: App, suite: &Suite) -> Result<(), String> {
    let script = match app {
        App::FastEmu => format!(
            "am start -n {FASTEMU}/.MainActivity -a android.intent.action.VIEW \
             -t application/java-archive -d {} --grant-read-uri-permission",
            uri(suite.media_id)
        ),
        App::J2meLoader => format!(
            "am start -n {J2ML}/.config.ConfigActivity -a android.intent.action.VIEW \
             -d {} --es midletName {}",
            shell_quote(&format!(
                "{J2ML_FILES}/converted/{}",
                suite
                    .j2ml_dir
                    .as_ref()
                    .ok_or_else(|| format!("J2ME Loader has no {} installed", suite.name))?
            )),
            shell_quote(&suite.name)
        ),
    };
    device.shell(&script).map(drop)
}

/// The Android key code a bench key presses: the arrows and fire mean the
/// same on both apps.
pub fn keycode(key: fastemu_core::Key) -> Option<u32> {
    use fastemu_core::Key;
    Some(match key {
        Key::Up => 19,
        Key::Down => 20,
        Key::Left => 21,
        Key::Right => 22,
        Key::Fire => 66,
        _ => return None,
    })
}

/// A shell script that presses the bench's keys at their times after its
/// start: `sleep` between `input keyevent`s.
pub fn key_script(steps: &[crate::bench::keys::KeyStep]) -> Result<String, String> {
    let mut script = Vec::new();
    let mut at = 0;
    for step in steps.iter().filter(|s| s.press) {
        let code = keycode(step.key).ok_or_else(|| format!("no Android key for {:?}", step.key))?;
        let wait = step.at_ms.saturating_sub(at);
        if wait > 0 {
            script.push(format!("sleep {}.{:03}", wait / 1000, wait % 1000));
        }
        script.push(format!("input keyevent {code}"));
        at = step.at_ms;
    }
    Ok(script.join("; "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bench::keys::{BENCH_KEYS, steps};

    #[test]
    fn the_bench_keys_become_one_script() {
        let script = key_script(&steps(BENCH_KEYS).unwrap()).unwrap();
        assert!(
            script.starts_with("sleep 1.500; input keyevent 66; sleep "),
            "{script}"
        );
        assert_eq!(script.matches("input keyevent").count(), 8);
        for code in [19, 20, 21, 22, 66] {
            assert!(script.contains(&format!("keyevent {code}")), "{script}");
        }
    }

    #[test]
    fn only_the_pinned_j2me_loader_passes() {
        let file = std::env::temp_dir().join(format!("fastemu-j2ml-{}.apk", std::process::id()));
        std::fs::write(&file, b"not the apk").unwrap();
        let refused = check_j2ml_apk(&file).unwrap_err();
        assert!(refused.contains("not J2ME Loader 1.9.0"), "{refused}");
        let _ = std::fs::remove_file(&file);
    }
}
