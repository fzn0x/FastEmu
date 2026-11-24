//! Helpers for the integration tests: compiling the test MIDlets with the
//! installed JDK, packing JARs (whole or broken), and driving an instance
//! while collecting its log.
//!
//! The MIDlets in `tests/java/midlets` compile against the stubs in
//! `tests/java/stubs` with `javac --release 8`; only the MIDlets' own
//! classes go into the JARs, so the runtime's classes serve the stubs'
//! names. Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set (CI sets it).

#![allow(dead_code, reason = "each test binary uses a different part")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use fastemu_core::{
    DeviceProfile, Frame, InputEvent, Instance, InstanceSettings, InstanceStatus, Key, KeyAction,
    LaunchRequest, LogLine, default_profile,
};
use fastemu_midp::{MidpInstance, MidpLauncher};

/// How long a test waits for something before it fails.
pub const TIMEOUT: Duration = Duration::from_secs(15);

/// A scratch folder for this test binary, inside the target folder.
pub fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("midp-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("making a scratch folder");
    dir
}

/// The JDK tool `name`, from `JAVA_HOME` first, then the `PATH`.
fn tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let candidate = Path::new(&home).join("bin").join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let works = Command::new(name)
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success());
    works.then(|| PathBuf::from(name))
}

fn java_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reading the Java sources") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            java_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

/// Compiles the stubs and the test MIDlets once and returns the folder of
/// class files, or why that was impossible.
fn compiled() -> &'static Result<PathBuf, String> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let javac = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let classes = scratch("classes");
        let java = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("java");
        let mut sources = Vec::new();
        java_sources(&java, &mut sources);
        sources.sort();
        let output = Command::new(&javac)
            .args([
                "--release",
                "8",
                "-g",
                "-encoding",
                "UTF-8",
                "-nowarn",
                "-d",
            ])
            .arg(&classes)
            .args(&sources)
            .output()
            .map_err(|e| format!("running javac: {e}"))?;
        assert!(
            output.status.success(),
            "javac failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(classes)
    })
}

/// The compiled test classes, or `None` (after saying why) when no JDK
/// is installed and Java is not required.
pub fn classes() -> Option<&'static Path> {
    match compiled() {
        Ok(dir) => Some(dir),
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "Java is required but unusable: {why}"
            );
            eprintln!("skipping: {why}");
            None
        }
    }
}

/// The manifest of the test suite: one MIDlet per test program.
pub const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Tests\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: Shapes, , t.Shapes\r\n\
MIDlet-2: Keys, , t.Keys\r\n\
MIDlet-3: Threads, , t.Threads\r\n\
MIDlet-4: Serial, , t.Serial\r\n\
MIDlet-5: Looper, , t.Looper\r\n\
MIDlet-6: Commands, , t.Commands\r\n\
MIDlet-7: Many, , t.ManyCommands\r\n\
MIDlet-8: Full, , t.FullCommands\r\n\
MIDlet-9: Lifecycle, , t.Lifecycle\r\n\
MIDlet-10: Crash, , t.Crash\r\n\
Game-Speed: 3\r\n";

/// Which `MIDlet-<n>` (from zero) runs each test program.
pub mod midlet {
    pub const SHAPES: usize = 0;
    pub const KEYS: usize = 1;
    pub const THREADS: usize = 2;
    pub const SERIAL: usize = 3;
    pub const LOOPER: usize = 4;
    pub const COMMANDS: usize = 5;
    pub const MANY_COMMANDS: usize = 6;
    pub const FULL_COMMANDS: usize = 7;
    pub const LIFECYCLE: usize = 8;
    pub const CRASH: usize = 9;
}

/// A 2 by 2 PNG: red, green, blue and half-transparent white.
pub fn test_png() -> Vec<u8> {
    let pixels: [[u8; 4]; 4] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 255, 128],
    ];
    let mut raw = Vec::new();
    for row in pixels.chunks(2) {
        raw.push(0);
        for p in row {
            raw.extend_from_slice(p);
        }
    }
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        png.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        png.extend_from_slice(&body);
        png.extend_from_slice(&crc32fast::hash(&body).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(b"IEND", &[]);
    png
}

/// Writes a ZIP of stored entries; without `central` it has no central
/// directory, as some broken JARs.
pub fn zip(entries: &[(String, Vec<u8>)], central: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let mut directory = Vec::new();
    for (name, data) in entries {
        let crc = crc32fast::hash(data);
        let offset = out.len() as u32;
        let mut header = Vec::new();
        header.extend_from_slice(&20u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        header.extend_from_slice(&[0; 4]);
        header.extend_from_slice(&crc.to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(data.len() as u32).to_le_bytes());
        header.extend_from_slice(&(name.len() as u16).to_le_bytes());
        header.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&header);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&header);
        // Comment length, disk, internal and external attributes.
        directory.extend_from_slice(&[0; 10]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }
    if central {
        let start = out.len() as u32;
        out.extend_from_slice(&directory);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
    }
    out
}

/// The entries of the test suite's JAR: the manifest, every class under
/// `t/` and the resources.
pub fn suite_entries(classes: &Path, manifest: &str) -> Vec<(String, Vec<u8>)> {
    let mut entries = vec![(
        "META-INF/MANIFEST.MF".to_owned(),
        manifest.as_bytes().to_vec(),
    )];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("t"))
        .expect("the compiled test classes")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let name = format!("t/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((name, std::fs::read(&file).expect("a class file")));
    }
    entries.push(("res/dot.png".to_owned(), test_png()));
    entries
}

/// Writes the test suite's JAR to `dir/name` and returns its path.
pub fn write_suite(classes: &Path, dir: &Path, name: &str, central: bool) -> PathBuf {
    let path = dir.join(name);
    let entries = suite_entries(classes, MANIFEST);
    std::fs::write(&path, zip(&entries, central)).expect("writing the JAR");
    path
}

/// A launched instance and everything it has logged so far.
pub struct Running {
    pub instance: Arc<MidpInstance>,
    log: Vec<LogLine>,
}

impl Running {
    /// Starts MIDlet `midlet` of `suite` on `profile`.
    pub fn launch_on(suite: &Path, midlet: usize, profile: DeviceProfile) -> Self {
        let data = suite.with_extension("data");
        let request = LaunchRequest {
            midlet,
            profile,
            settings: InstanceSettings::default(),
            ..LaunchRequest::new(suite, data)
        };
        let instance = MidpLauncher::new(None)
            .launch_midp(request)
            .expect("launching the test MIDlet");
        Self {
            instance,
            log: Vec::new(),
        }
    }

    /// Starts MIDlet `midlet` of `suite` on the default profile.
    pub fn launch(suite: &Path, midlet: usize) -> Self {
        Self::launch_on(suite, midlet, default_profile())
    }

    fn collect(&mut self) {
        self.log.extend(self.instance.drain_log());
    }

    /// Every line logged so far.
    pub fn lines(&mut self) -> Vec<String> {
        self.collect();
        self.log.iter().map(|l| l.text.clone()).collect()
    }

    /// The log so far, for failure messages.
    pub fn dump(&mut self) -> String {
        self.lines().join("\n")
    }

    /// Whether a line containing `needle` was logged.
    pub fn has(&mut self, needle: &str) -> bool {
        self.lines().iter().any(|l| l.contains(needle))
    }

    /// Waits for a line containing `needle` and returns it.
    pub fn wait_log(&mut self, needle: &str) -> String {
        let start = Instant::now();
        loop {
            if let Some(line) = self.lines().into_iter().find(|l| l.contains(needle)) {
                return line;
            }
            assert!(
                start.elapsed() <= TIMEOUT,
                "no log line with {needle:?}; the log:\n{}",
                self.dump()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits until `test` holds for the status.
    pub fn wait_status(&mut self, test: impl Fn(&InstanceStatus) -> bool) -> InstanceStatus {
        let start = Instant::now();
        loop {
            let status = self.instance.status();
            if test(&status) {
                return status;
            }
            assert!(
                start.elapsed() <= TIMEOUT,
                "status stayed {status:?}; the log:\n{}",
                self.dump()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Waits for a frame `test` accepts.
    pub fn wait_frame(&mut self, test: impl Fn(&Frame) -> bool) -> Frame {
        let start = Instant::now();
        loop {
            if let Some(frame) = self.instance.frame()
                && test(&frame)
            {
                return frame;
            }
            assert!(
                start.elapsed() <= TIMEOUT,
                "no matching frame (last serial {:?}); the log:\n{}",
                self.instance.frame().map(|f| f.serial()),
                self.dump()
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Presses or releases `key`.
    pub fn key(&self, key: Key, action: KeyAction) {
        self.instance.send(InputEvent::Key { key, action });
    }

    /// Presses and releases `key`.
    pub fn tap(&self, key: Key) {
        self.key(key, KeyAction::Press);
        self.key(key, KeyAction::Release);
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.instance.stop();
    }
}

/// The pixel at `(x, y)` without its alpha.
pub fn rgb(frame: &Frame, x: u32, y: u32) -> u32 {
    frame.pixel(x, y).expect("a pixel inside the frame") & 0x00FF_FFFF
}
