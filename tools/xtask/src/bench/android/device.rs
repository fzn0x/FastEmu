//! The emulator and adb: the SDK's tools, one headless emulator the bench
//! starts on a port of its own, its shell, and its logcat as a stream.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use super::parse::{self, LogcatLine};
use crate::bench::measure::Running;

/// The console port of the bench's emulator, so its serial is fixed and
/// no other emulator or phone gets the bench's commands.
pub const PORT: u16 = 5580;
/// The processors and memory the emulator gets, pinned: the AVD's own
/// files may disagree on its memory.
pub const CORES: u32 = 4;
pub const MEMORY_MIB: u32 = 4096;
/// The longest a boot may take.
const BOOT_LIMIT: Duration = Duration::from_secs(300);

/// The Android SDK's emulator and adb.
pub struct Sdk {
    pub emulator: PathBuf,
    pub adb: PathBuf,
}

impl Sdk {
    /// The SDK at `given`, else at `ANDROID_HOME`, `ANDROID_SDK_ROOT`, or
    /// where Android Studio puts it.
    pub fn find(given: Option<&Path>) -> Result<Sdk, String> {
        let mut places: Vec<PathBuf> = Vec::new();
        if let Some(dir) = given {
            places.push(dir.to_owned());
        } else {
            for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
                if let Some(dir) = std::env::var_os(var) {
                    places.push(dir.into());
                }
            }
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                places.push(Path::new(&local).join("Android").join("Sdk"));
            }
            if let Some(home) = std::env::var_os("HOME") {
                places.push(Path::new(&home).join("Library/Android/sdk"));
                places.push(Path::new(&home).join("Android/Sdk"));
            }
        }
        let exe = |name: &str| format!("{name}{}", std::env::consts::EXE_SUFFIX);
        for root in places {
            let emulator = root.join("emulator").join(exe("emulator"));
            let adb = root.join("platform-tools").join(exe("adb"));
            if emulator.is_file() && adb.is_file() {
                return Ok(Sdk { emulator, adb });
            }
        }
        Err("no Android SDK with emulator and platform-tools found: give --sdk DIR".into())
    }

    /// The emulator's version: the first line of `emulator -version`.
    pub fn emulator_version(&self) -> String {
        Command::new(&self.emulator)
            .arg("-version")
            .output()
            .ok()
            .and_then(|out| {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .find(|l| l.contains("version"))
                    .map(|l| l.trim().to_owned())
            })
            .unwrap_or_else(|| "unknown".into())
    }
}

/// One device, by its serial.
#[derive(Clone)]
pub struct Device {
    adb: PathBuf,
    pub serial: String,
}

impl Device {
    pub fn new(adb: &Path, serial: &str) -> Self {
        Self {
            adb: adb.to_owned(),
            serial: serial.to_owned(),
        }
    }

    /// adb itself.
    pub fn adb(&self) -> &Path {
        &self.adb
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.adb);
        command.arg("-s").arg(&self.serial);
        command
    }

    fn run(mut command: Command, what: &str) -> Result<String, String> {
        let out = command
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("{what}: {e}"))?;
        let text = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
        if out.status.success() {
            Ok(text)
        } else {
            Err(format!(
                "{what} failed ({}): {}{}",
                out.status,
                text.trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    /// Runs `script` in the device's shell and returns what it printed.
    /// A status other than 0 is an error.
    pub fn shell(&self, script: &str) -> Result<String, String> {
        let mut command = self.command();
        command.arg("shell").arg(script);
        Self::run(command, &format!("adb shell {script}"))
    }

    /// Runs `script` and ignores how it ended.
    pub fn shell_any(&self, script: &str) -> String {
        self.shell(script).unwrap_or_default()
    }

    /// Copies `local` to `remote` on the device.
    pub fn push(&self, local: &Path, remote: &str) -> Result<(), String> {
        let mut command = self.command();
        command.arg("push").arg(local).arg(remote);
        Self::run(command, &format!("adb push {}", local.display())).map(drop)
    }

    /// Installs `apk`, granting every runtime permission it asks for.
    pub fn install(&self, apk: &Path) -> Result<(), String> {
        let mut command = self.command();
        command.arg("install").arg("-g").arg(apk);
        Self::run(command, &format!("adb install {}", apk.display())).map(drop)
    }

    /// Saves the screen as a PNG at `path`.
    pub fn screenshot(&self, path: &Path) -> Result<(), String> {
        let out = self
            .command()
            .args(["exec-out", "screencap", "-p"])
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("screencap: {e}"))?;
        if !out.status.success() || out.stdout.is_empty() {
            return Err("screencap printed no picture".into());
        }
        std::fs::write(path, &out.stdout).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The id of the process named `name`, if it runs.
    pub fn pid(&self, name: &str) -> Option<u32> {
        self.shell(&format!("pidof {name}"))
            .ok()?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }

    /// The processor time of process `pid` so far, in milliseconds.
    pub fn cpu_ms(&self, pid: u32) -> Option<f64> {
        let stat = self.shell(&format!("cat /proc/{pid}/stat")).ok()?;
        // Linux counts in USER_HZ, 100 a second on Android.
        parse::cpu_ticks(&stat).map(|ticks| ticks as f64 * 10.0)
    }

    /// The whole guest's busy and total jiffies.
    pub fn guest_jiffies(&self) -> Option<(u64, u64)> {
        parse::guest_jiffies(&self.shell("cat /proc/stat").ok()?)
    }

    /// Clears logcat and streams its lines of the tags the bench reads.
    pub fn logcat(&self, keep: &Path) -> Result<Logcat, String> {
        self.shell_any("logcat -c");
        let mut command = self.command();
        command
            .args(["logcat", "-v", "monotonic", "-b", "main,system"])
            .args([
                "fastemu:I",
                "System.out:I",
                "System.err:W",
                "ActivityTaskManager:I",
                "ActivityManager:I",
                "AndroidRuntime:E",
                "*:S",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|e| format!("adb logcat: {e}"))?;
        let stdout = child.stdout.take().ok_or("adb logcat has no output")?;
        let mut file = File::create(keep).map_err(|e| format!("{}: {e}", keep.display()))?;
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            use std::io::Write as _;
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = writeln!(file, "{line}");
                if let Some(parsed) = parse::logcat_line(&line)
                    && sender.send(parsed).is_err()
                {
                    break;
                }
            }
        });
        Ok(Logcat {
            _running: Running::new(child),
            lines,
        })
    }
}

/// A running `adb logcat`, stopped when dropped.
pub struct Logcat {
    _running: Running,
    lines: Receiver<LogcatLine>,
}

impl Logcat {
    /// The next line, or `None` when `until` passed first.
    pub fn next_until(&self, until: Instant) -> Option<LogcatLine> {
        let wait = until.saturating_duration_since(Instant::now());
        self.lines.recv_timeout(wait).ok()
    }
}

/// The bench's emulator, killed when dropped.
pub struct Emulator {
    running: Running,
    pub device: Device,
    log: PathBuf,
}

impl Emulator {
    /// Starts `avd` with no window, from a cold boot that saves nothing,
    /// and waits until Android has booted. `gpu` is the emulator's
    /// `-gpu` mode; its output goes to `log`.
    pub fn start(sdk: &Sdk, avd: &str, gpu: &str, log: &Path) -> Result<Emulator, String> {
        let device = Device::new(&sdk.adb, &format!("emulator-{PORT}"));
        let listed = Command::new(&sdk.adb)
            .arg("devices")
            .output()
            .map_err(|e| format!("adb devices: {e}"))?;
        if String::from_utf8_lossy(&listed.stdout).contains(&device.serial) {
            return Err(format!(
                "{} runs already: stop it first (adb -s {} emu kill)",
                device.serial, device.serial
            ));
        }
        let out = File::create(log).map_err(|e| format!("{}: {e}", log.display()))?;
        let err = out.try_clone().map_err(|e| e.to_string())?;
        let child = Command::new(&sdk.emulator)
            .args(["-avd", avd, "-port", &PORT.to_string()])
            .args(emulator_flags(gpu))
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .spawn()
            .map_err(|e| format!("{}: {e}", sdk.emulator.display()))?;
        let mut emulator = Emulator {
            running: Running::new(child),
            device,
            log: log.to_owned(),
        };
        emulator.wait_for_boot()?;
        Ok(emulator)
    }

    fn wait_for_boot(&mut self) -> Result<(), String> {
        let until = Instant::now() + BOOT_LIMIT;
        while Instant::now() < until {
            if self.running.has_ended() {
                return Err(format!(
                    "the emulator ended before Android booted: see {}",
                    self.log.display()
                ));
            }
            if self
                .device
                .shell("getprop sys.boot_completed")
                .ok()
                .as_deref()
                == Some("1\n")
            {
                return Ok(());
            }
            thread::sleep(Duration::from_secs(2));
        }
        Err(format!(
            "Android did not boot in {} s: see {}",
            BOOT_LIMIT.as_secs(),
            self.log.display()
        ))
    }

    /// The lines of the emulator's log that name the renderers it chose.
    pub fn renderers(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .filter(|l| {
                l.contains("Selecting Vulkan device")
                    || l.contains("gpu mode")
                    || l.contains("GL_RENDERER")
                    || l.contains("OpenGL Renderer")
            })
            .map(|l| l.trim().to_owned())
            .collect()
    }

    /// Keeps the screen on, unlocked and still, and stops the checks
    /// and notices that would ask for a tap.
    pub fn settle(&self) -> Result<(), String> {
        for script in [
            "svc power stayon true",
            "settings put system screen_off_timeout 2147483647",
            "input keyevent KEYCODE_WAKEUP",
            "wm dismiss-keyguard",
            "settings put global window_animation_scale 0",
            "settings put global transition_animation_scale 0",
            "settings put global animator_duration_scale 0",
            "settings put secure immersive_mode_confirmations confirmed",
            "settings put global verifier_verify_adb_installs 0",
            "settings put global package_verifier_user_consent -1",
        ] {
            self.device.shell(script)?;
        }
        Ok(())
    }

    /// Asks the emulator to end, and kills it if it does not.
    pub fn stop(mut self) {
        let mut command = Command::new(&self.device.adb);
        command
            .args(["-s", &self.device.serial, "emu", "kill"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = command.status();
        let until = Instant::now() + Duration::from_secs(30);
        while Instant::now() < until && !self.running.has_ended() {
            thread::sleep(Duration::from_millis(500));
        }
        // Dropping `running` kills it if it still runs.
    }
}

/// The emulator's flags after `-avd` and `-port`: headless, cold boot,
/// nothing saved, processors and memory pinned, GPU mode `gpu`.
pub fn emulator_flags(gpu: &str) -> Vec<String> {
    let mut flags: Vec<String> = [
        "-no-window",
        "-no-boot-anim",
        "-no-snapshot",
        "-read-only",
        "-no-metrics",
        "-camera-back",
        "none",
        "-camera-front",
        "none",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    flags.extend([
        "-cores".to_owned(),
        CORES.to_string(),
        "-memory".to_owned(),
        MEMORY_MIB.to_string(),
        "-gpu".to_owned(),
        gpu.to_owned(),
    ]);
    flags
}
