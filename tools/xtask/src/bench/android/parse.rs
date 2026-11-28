//! What the Android bench reads from the emulator, as text: logcat lines,
//! `/proc` files, `dumpsys` output and `content query` rows. Each parser
//! is a pure function, so it is tested here on the host.

use std::fmt::Write as _;

/// The mark before each line of FastEmu's bench log (see
/// `crates/fastemu-android/src/bench_log.rs`).
pub const BENCH_LOG_MARK: &str = "FastEmuBench: ";

/// One line of `adb logcat -v monotonic`.
#[derive(Clone, Debug, PartialEq)]
pub struct LogcatLine {
    /// Seconds since the device booted: its monotonic clock, the one
    /// SurfaceFlinger times presents with.
    pub time: f64,
    pub pid: u32,
    pub tid: u32,
    /// `V`, `D`, `I`, `W`, `E`, `F` or `A`.
    pub level: char,
    pub tag: String,
    pub msg: String,
}

/// Parses a line of `logcat -v monotonic`:
/// `  1795.205  3052  3238 I fastemu : message`.
pub fn logcat_line(line: &str) -> Option<LogcatLine> {
    let line = line.trim_end_matches(['\r', '\n']);
    let mut rest = line.trim_start();
    let mut field = || {
        let end = rest.find(char::is_whitespace)?;
        let word = &rest[..end];
        rest = rest[end..].trim_start();
        Some(word)
    };
    let time = field()?.parse().ok()?;
    let pid = field()?.parse().ok()?;
    let tid = field()?.parse().ok()?;
    let level = field()?;
    if level.len() != 1 {
        return None;
    }
    let level = level.chars().next()?;
    let (tag, msg) = rest.split_once(": ").or_else(|| {
        // A line with an empty message ends with the colon.
        rest.strip_suffix(':').map(|tag| (tag, ""))
    })?;
    Some(LogcatLine {
        time,
        pid,
        tid,
        level,
        tag: tag.trim().to_owned(),
        msg: msg.to_owned(),
    })
}

/// The line of the desktop bench's format (`super::super::wire`) that a
/// logcat line carries, for the side that wrote it: FastEmu's bench log
/// marks its lines, and J2ME Loader's game prints `BENCH ...` to
/// `System.out`, which Android logs under that tag.
pub fn bench_line(line: &LogcatLine, fastemu: bool) -> Option<String> {
    if fastemu {
        if line.tag != "fastemu" {
            return None;
        }
        let at = line.msg.find(BENCH_LOG_MARK)?;
        Some(line.msg[at + BENCH_LOG_MARK.len()..].to_owned())
    } else if line.tag == "System.out" && line.msg.starts_with("BENCH ") {
        Some(format!("log 0 out {}", line.msg))
    } else {
        None
    }
}

/// Whether `msg`, of tag `ActivityTaskManager`, is the start of an
/// activity of `package`: `START u0 {... cmp=package/...}`.
pub fn is_start_of(msg: &str, package: &str) -> bool {
    msg.starts_with("START ") && msg.contains(&format!("cmp={package}/"))
}

/// The processor time of a process, in clock ticks: `utime` plus `stime`
/// of `/proc/<pid>/stat`.
pub fn cpu_ticks(stat: &str) -> Option<u64> {
    // The name in brackets may hold spaces; the fields after it do not.
    let after = &stat[stat.rfind(')')? + 1..];
    let fields: Vec<&str> = after.split_whitespace().collect();
    // After the name come the state (field 3) and then fields 4 on, so
    // utime (14) and stime (15) are the 12th and 13th words here.
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

/// The busy and the total jiffies of the whole guest, from the `cpu` line
/// of `/proc/stat`. Idle and I/O wait are not busy.
pub fn guest_jiffies(stat: &str) -> Option<(u64, u64)> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(|v| v.parse().ok())
        .collect::<Option<_>>()?;
    // user nice system idle iowait irq softirq steal (guest time is
    // already in user).
    let total: u64 = values.iter().take(8).sum();
    let idle = values.get(3)? + values.get(4).copied().unwrap_or(0);
    Some((total - idle, total))
}

/// The number of processors of the guest: the `cpuN` lines of `/proc/stat`.
pub fn guest_cores(stat: &str) -> usize {
    stat.lines()
        .filter(|l| l.starts_with("cpu") && l.as_bytes().get(3).is_some_and(u8::is_ascii_digit))
        .count()
}

/// The proportional set size in KiB from `dumpsys meminfo <pid>`: its
/// `TOTAL PSS:` summary, or else the first number of its `TOTAL` row.
pub fn pss_kib(meminfo: &str) -> Option<u64> {
    for line in meminfo.lines() {
        if let Some(at) = line.find("TOTAL PSS:") {
            return line[at + "TOTAL PSS:".len()..]
                .split_whitespace()
                .next()?
                .parse()
                .ok();
        }
    }
    meminfo.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next()? == "TOTAL").then_some(())?;
        words.next()?.parse().ok()
    })
}

/// A `kB` field of `/proc/<pid>/status`, such as `VmHWM`.
pub fn status_kib(status: &str, field: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        let rest = line.strip_prefix(field)?.strip_prefix(':')?;
        rest.split_whitespace().next()?.parse().ok()
    })
}

/// The frames each layer presented, from
/// `dumpsys SurfaceFlinger --timestats -dump`: `layerName = ` starts a
/// layer, and its `totalFrames = ` follows.
pub fn timestats(dump: &str) -> Vec<(String, u64)> {
    let mut layers = Vec::new();
    let mut name: Option<String> = None;
    for line in dump.lines() {
        let line = line.trim();
        if let Some(n) = line.strip_prefix("layerName = ") {
            name = Some(n.to_owned());
        } else if let Some(n) = line.strip_prefix("totalFrames = ")
            && let Some(layer) = name.take()
            && let Ok(frames) = n.trim().parse()
        {
            layers.push((layer, frames));
        }
    }
    layers
}

/// The frames of the busiest layer of `package`, as a game's surface is
/// the layer that presents the most.
pub fn package_frames(layers: &[(String, u64)], package: &str) -> Option<u64> {
    layers
        .iter()
        .filter(|(name, _)| name.contains(package))
        .map(|&(_, frames)| frames)
        .max()
}

/// The surface layers of `package` in `dumpsys SurfaceFlinger --list`:
/// its `SurfaceView[...](BLAST)#N` layers, where a game is drawn.
pub fn surface_layers(list: &str, package: &str) -> Vec<String> {
    list.lines()
        .filter_map(|line| {
            let line = line.trim();
            let name = line.strip_prefix("RequestedLayerState{").unwrap_or(line);
            let name = name.strip_prefix("SurfaceView[")?;
            let end = name
                .find(|c: char| c.is_whitespace() || c == '}')
                .unwrap_or(name.len());
            let name = format!("SurfaceView[{}", &name[..end]);
            (name.contains(package) && name.contains("(BLAST)")).then_some(name)
        })
        .collect()
}

/// The times a layer's frames were presented, in seconds of the monotonic
/// clock, from `dumpsys SurfaceFlinger --latency <layer>`: its rows after
/// the first are `desired actual ready` in nanoseconds, and a frame not
/// presented yet has 0 or the largest 64-bit number.
pub fn presents(latency: &str) -> Vec<f64> {
    latency
        .lines()
        .skip(1)
        .filter_map(|row| {
            let actual: i64 = row.split_whitespace().nth(1)?.parse().ok()?;
            (actual > 0 && actual < i64::MAX).then(|| actual as f64 / 1e9)
        })
        .collect()
}

/// The `_id` of the first row of `content query --projection _id`:
/// `Row: 0 _id=20`.
pub fn content_id(rows: &str) -> Option<u64> {
    rows.lines().find_map(|line| {
        let at = line.find("_id=")?;
        line[at + 4..]
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    })
}

/// `text` quoted for the emulator's shell: in single quotes, with each
/// single quote closed, escaped and reopened.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The digest of a file's bytes, in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// J2ME Loader's per-game settings, `configs/<game>/config.json`: the
/// values its settings screen gives a new game (ProfileModel's defaults in
/// 1.9.0), with `fps_limit` as its frame limit (0: none) and
/// `system_properties` the text of its `assets/defaults/system.props`.
/// Every field is written, since a missing one would read as 0 or false,
/// not as its default.
pub fn j2meloader_config(fps_limit: u32, system_properties: &str) -> String {
    let fields: [(&str, String); 43] = [
        ("Version", "3".into()),
        ("ScreenWidth", "240".into()),
        ("ScreenHeight", "320".into()),
        ("ScreenBackgroundColor", "13684944".into()),
        ("ScreenScaleRatio", "100".into()),
        ("Orientation", "0".into()),
        ("ScreenScaleToFit", "true".into()),
        ("ScreenKeepAspectRatio", "true".into()),
        ("ScreenScaleType", "1".into()),
        ("ScreenGravity", "1".into()),
        ("ScreenFilter", "false".into()),
        ("ImmediateMode", "false".into()),
        ("HwAcceleration", "false".into()),
        ("GraphicsMode", "1".into()),
        ("M3GForceDisableLighting", "false".into()),
        ("ParallelRedrawScreen", "false".into()),
        ("ShowFps", "false".into()),
        ("FpsLimit", fps_limit.to_string()),
        ("ForceFullscreen", "false".into()),
        ("FontSizeSmall", "18".into()),
        ("FontSizeMedium", "22".into()),
        ("FontSizeLarge", "26".into()),
        ("FontApplyDimensions", "false".into()),
        ("FontAntiAlias", "true".into()),
        ("TouchInput", "true".into()),
        ("ShowKeyboard", "true".into()),
        ("VirtualKeyboardType", "0".into()),
        ("ButtonShape", "2".into()),
        ("VirtualKeyboardAlpha", "64".into()),
        ("VirtualKeyboardForceOpacity", "false".into()),
        ("VirtualKeyboardFeedback", "false".into()),
        ("VirtualKeyboardDelay", "0".into()),
        ("VirtualKeyboardColorBackground", "13684944".into()),
        ("VirtualKeyboardColorBackgroundSelected", "128".into()),
        ("VirtualKeyboardColorForeground", "128".into()),
        ("VirtualKeyboardColorForegroundSelected", "16777215".into()),
        ("VirtualKeyboardColorOutline", "16777215".into()),
        ("Layout", "0".into()),
        ("DisablePause", "false".into()),
        // Neither of these is set on a new game.
        ("KeyCodeMap", "null".into()),
        ("KeyMappings", "null".into()),
        (
            "SystemProperties",
            crate::bench::report::string(system_properties.trim_end_matches(['\n', '\r'])),
        ),
        ("Shader", "null".into()),
    ];
    let mut out = String::from("{\n");
    let written: Vec<String> = fields
        .iter()
        .filter(|(_, value)| value != "null")
        .map(|(key, value)| format!("  \"{key}\": {value}"))
        .collect();
    out.push_str(&written.join(",\n"));
    out.push_str("\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logcat_lines_parse() {
        let line = logcat_line(
            "         1795824513.205  3052  3238 I fastemu : fastemu_android::android::app: \
             FastEmuBench: RUN first game=0 ms=683\r\n",
        )
        .unwrap();
        assert_eq!(line.pid, 3052);
        assert_eq!(line.tid, 3238);
        assert_eq!(line.level, 'I');
        assert_eq!(line.tag, "fastemu");
        assert!((line.time - 1_795_824_513.205).abs() < 1e-6);
        assert_eq!(bench_line(&line, true).unwrap(), "RUN first game=0 ms=683");
        assert_eq!(bench_line(&line, false), None);

        let out =
            logcat_line("1795824513.5  9283  9301 I System.out: BENCH done rounds=5").unwrap();
        assert_eq!(out.tag, "System.out");
        assert_eq!(
            bench_line(&out, false).unwrap(),
            "log 0 out BENCH done rounds=5"
        );
        // Another line of System.out is not a bench line.
        let other = logcat_line("1.0 1 1 I System.out: hello").unwrap();
        assert_eq!(bench_line(&other, false), None);
        let empty = logcat_line("1.0 1 1 W Binder:").unwrap();
        assert_eq!(empty.msg, "");
        assert_eq!(logcat_line("--------- beginning of main"), None);
    }

    #[test]
    fn starts_are_told_apart_by_package() {
        let msg = "START u0 {act=android.intent.action.VIEW dat=content://media/... \
                   typ=application/java-archive flg=0x10000001 \
                   cmp=io.github.fzn0x.fastemu/.MainActivity} with LAUNCH_SINGLE_TASK";
        assert!(is_start_of(msg, "io.github.fzn0x.fastemu"));
        assert!(!is_start_of(msg, "ru.playsoftware.j2meloader"));
        assert!(!is_start_of(
            "Displayed io.github.fzn0x.fastemu/.MainActivity",
            "io.github.fzn0x.fastemu"
        ));
    }

    #[test]
    fn proc_files_parse() {
        let stat = "9283 (j2meloader:mid let) S 300 300 0 0 -1 1077952832 9000 0 3 0 \
                    250 41 0 0 10 -10 30 0 6812 1000000 5000 18446744073709551615";
        assert_eq!(cpu_ticks(stat), Some(291));
        assert_eq!(cpu_ticks("garbage"), None);

        let proc_stat = "cpu  100 5 50 1000 20 3 2 0 0 0\ncpu0 50 2 25 500 10 1 1 0 0 0\n\
                         cpu1 50 3 25 500 10 2 1 0 0 0\nintr 1 2 3\n";
        assert_eq!(guest_jiffies(proc_stat), Some((160, 1180)));
        assert_eq!(guest_cores(proc_stat), 2);

        let status =
            "Name:\tfastemu\nVmPeak:\t 900000 kB\nVmHWM:\t  123456 kB\nVmRSS:\t 100000 kB\n";
        assert_eq!(status_kib(status, "VmHWM"), Some(123_456));
        assert_eq!(status_kib(status, "VmSwap"), None);
    }

    #[test]
    fn meminfo_gives_the_total_pss() {
        let new = "Applications Memory Usage (in Kilobytes):\n ...\n           TOTAL PSS:    45678            TOTAL RSS:   99999       TOTAL SWAP PSS:       12\n";
        assert_eq!(pss_kib(new), Some(45_678));
        let old = "  Native Heap     1000\n        TOTAL    51234    40000\n";
        assert_eq!(pss_kib(old), Some(51_234));
        assert_eq!(pss_kib("No process found for: 1"), None);
    }

    #[test]
    fn timestats_give_each_layer_its_frames() {
        let dump = "SurfaceFlinger TimeStats:\ntotalFrames = 101\ndisplayRefreshRate = 60 fps\n\
                    layerName = SurfaceView[ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity](BLAST)#192\n\
                    packageName = \ntotalFrames = 98\nlayerName = none\ntotalFrames = 0\n\
                    layerName = ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity#186\ntotalFrames = 4\n";
        let layers = timestats(dump);
        assert_eq!(layers.len(), 3);
        assert_eq!(layers[0].1, 98);
        assert_eq!(
            package_frames(&layers, "ru.playsoftware.j2meloader"),
            Some(98)
        );
        assert_eq!(package_frames(&layers, "io.github.fzn0x.fastemu"), None);
    }

    #[test]
    fn surface_layers_and_presents_parse() {
        let list = "RequestedLayerState{Bounds for - ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity#190 parentId=187}
                    RequestedLayerState{Background for SurfaceView[ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity]#193 parentId=191}
                    RequestedLayerState{SurfaceView[ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity]#191 parentId=190}
                    RequestedLayerState{SurfaceView[ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity](BLAST)#192 parentId=191}
                    SurfaceView[io.github.fzn0x.fastemu/io.github.fzn0x.fastemu.MainActivity](BLAST)#40
";
        assert_eq!(
            surface_layers(list, "ru.playsoftware.j2meloader"),
            [
                "SurfaceView[ru.playsoftware.j2meloader/javax.microedition.shell.MicroActivity](BLAST)#192"
            ]
        );
        assert_eq!(surface_layers(list, "io.github.fzn0x.fastemu").len(), 1);
        let latency = "16666666
133156563900	133176402700	133173612400
                       0	0	0
133206589200	9223372036854775807	133223219400
";
        assert_eq!(presents(latency), [133.1764027]);
    }

    #[test]
    fn small_helpers() {
        assert_eq!(content_id("Row: 0 _id=20, _display_name=x.jar\n"), Some(20));
        assert_eq!(content_id("No result found.\n"), None);
        assert_eq!(shell_quote("Block Fall"), "'Block Fall'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn the_config_holds_every_default() {
        let config = j2meloader_config(0, "a: 1\nb: \"2\"\n");
        assert!(config.starts_with("{\n  \"Version\": 3,\n"));
        assert!(config.contains("\"FpsLimit\": 0,"));
        assert!(config.contains("\"GraphicsMode\": 1,"));
        assert!(config.contains("\"SystemProperties\": \"a: 1\\nb: \\\"2\\\"\""));
        assert!(!config.contains("null"));
        assert_eq!(config.lines().filter(|l| l.starts_with("  \"")).count(), 40);
        assert!(j2meloader_config(60, "").contains("\"FpsLimit\": 60,"));
    }
}
