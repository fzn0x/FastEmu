//! Shared by the `io_` and `wma_` tests: compiling the Java test programs
//! in `tests/io_java`, running them on a virtual machine with the MIDP
//! classes, and a small HTTP server on 127.0.0.1.
//!
//! The programs compile with `javac --release 8` against the stubs in
//! `tests/io_java/stubs`, which stand in for the javax.microedition
//! classes; only the programs' own classes go to the VM. Without a JDK the
//! tests print why and pass, unless `FASTEMU_REQUIRE_JAVA=1` is set.

#![allow(
    dead_code,
    reason = "each test file uses a different part of this module"
)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use fastemu_core::{LogLevel, LogLine, NetworkPolicy};
use fastemu_jvm::{Ctx, DirSource, MemorySource, NativeClass, Vm, VmConfig};
use fastemu_midp::env::InstanceEnv;

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

/// Every `.java` file under `dir`.
fn sources(dir: &Path, recurse: bool) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("reading a Java source folder") {
        let path = entry.expect("listing a Java source folder").path();
        if path.is_dir() && recurse {
            found.extend(sources(&path, true));
        } else if path.extension().is_some_and(|x| x == "java") {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn javac(javac: &Path, classpath: Option<&Path>, out: &Path, files: &[PathBuf]) {
    let mut command = Command::new(javac);
    command.args([
        "--release",
        "8",
        "-g",
        "-encoding",
        "UTF-8",
        "-nowarn",
        "-d",
    ]);
    command.arg(out);
    if let Some(cp) = classpath {
        command.arg("-cp").arg(cp);
    }
    let output = command.args(files).output().expect("running javac");
    assert!(
        output.status.success(),
        "javac failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The folder of compiled test programs, compiled once per test binary;
/// `None` when no JDK is installed (and `FASTEMU_REQUIRE_JAVA` is unset).
pub fn classes() -> Option<&'static Path> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    let compiled = CLASSES.get_or_init(|| {
        let javac_path = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let java_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("io_java");
        let out = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{}-java", env!("CARGO_CRATE_NAME")));
        let _ = std::fs::remove_dir_all(&out);
        let (stubs, programs) = (out.join("stubs"), out.join("classes"));
        std::fs::create_dir_all(&stubs).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&programs).map_err(|e| e.to_string())?;
        javac(
            &javac_path,
            None,
            &stubs,
            &sources(&java_dir.join("stubs"), true),
        );
        javac(
            &javac_path,
            Some(&stubs),
            &programs,
            &sources(&java_dir, false),
        );
        Ok(programs)
    });
    match compiled {
        Ok(dir) => Some(dir),
        Err(why) => {
            assert!(
                std::env::var_os("FASTEMU_REQUIRE_JAVA").is_none_or(|v| v != "1"),
                "FASTEMU_REQUIRE_JAVA=1 but the Java tests cannot run: {why}"
            );
            eprintln!("skipping the Java tests: {why}");
            None
        }
    }
}

/// Whether the CLDC library of this build has class `name`. The java.io
/// data streams come from another part of the library; tests that need
/// them check first.
pub fn has_class(name: &str) -> bool {
    let vm = Vm::new(
        VmConfig::default(),
        Arc::new(MemorySource::new()),
        Vec::new(),
    );
    vm.attach("probe", |ctx| ctx.find_class(name).is_ok())
}

/// The methods of `java.io.DataInput` and `DataOutput` in CLDC 1.1
/// (docs/compat/api/cldc-java-io.md).
const DATA_INPUT: &[(&str, &str)] = &[
    ("readFully", "([B)V"),
    ("readFully", "([BII)V"),
    ("skipBytes", "(I)I"),
    ("readBoolean", "()Z"),
    ("readByte", "()B"),
    ("readUnsignedByte", "()I"),
    ("readShort", "()S"),
    ("readUnsignedShort", "()I"),
    ("readChar", "()C"),
    ("readInt", "()I"),
    ("readLong", "()J"),
    ("readFloat", "()F"),
    ("readDouble", "()D"),
    ("readUTF", "()Ljava/lang/String;"),
];
const DATA_OUTPUT: &[(&str, &str)] = &[
    ("write", "(I)V"),
    ("write", "([B)V"),
    ("write", "([BII)V"),
    ("writeBoolean", "(Z)V"),
    ("writeByte", "(I)V"),
    ("writeShort", "(I)V"),
    ("writeChar", "(I)V"),
    ("writeInt", "(I)V"),
    ("writeLong", "(J)V"),
    ("writeFloat", "(F)V"),
    ("writeDouble", "(D)V"),
    ("writeChars", "(Ljava/lang/String;)V"),
    ("writeUTF", "(Ljava/lang/String;)V"),
];

/// The MIDP classes, plus stand-ins for the `java.io.DataInput` and
/// `DataOutput` interfaces when the library does not have them yet, so
/// `Datagram` (which extends both) loads and its methods resolve.
fn natives() -> Vec<NativeClass> {
    let mut natives = fastemu_midp::classes();
    for (name, methods) in [
        ("java/io/DataInput", DATA_INPUT),
        ("java/io/DataOutput", DATA_OUTPUT),
    ] {
        if !has_class(name) {
            let mut stand_in = NativeClass::interface(name);
            for (method, desc) in methods {
                stand_in = stand_in.abstract_method(method, desc);
            }
            natives.push(stand_in);
        }
    }
    natives
}

/// What a test program printed.
pub struct Output {
    pub out: String,
    pub err: String,
}

/// A test program running on its own VM.
pub struct Program {
    pub vm: Vm,
    class: String,
    lines: Arc<Mutex<Vec<LogLine>>>,
}

impl Program {
    /// Starts `class`'s `main` with `args` on a VM whose instance has the
    /// network policy `network`, after `setup` (which may install more
    /// extensions).
    pub fn start(
        classes: &Path,
        class: &str,
        args: &[&str],
        network: NetworkPolicy,
        setup: impl FnOnce(&mut Ctx<'_>),
    ) -> Self {
        let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
        let sink = Arc::clone(&lines);
        let config = VmConfig {
            log: Arc::new(move |line| sink.lock().unwrap().push(line)),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(DirSource::new(classes)), natives());
        let data_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("io-data");
        vm.attach("setup", |ctx| {
            let mut env = InstanceEnv::for_tests(&data_dir);
            env.settings.network = network;
            ctx.set_extension(env);
            setup(ctx);
        });
        vm.run_main(class, args).expect("starting the main thread");
        Self {
            vm,
            class: class.to_owned(),
            lines,
        }
    }

    /// What the program printed so far.
    pub fn output(&self) -> Output {
        self.vm.flush_output();
        let lines = self.lines.lock().unwrap();
        let pick = |level: LogLevel| {
            lines
                .iter()
                .filter(|l| l.level == level)
                .map(|l| l.text.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };
        Output {
            out: pick(LogLevel::Out),
            err: pick(LogLevel::Err),
        }
    }

    /// Waits up to `timeout` for the program to print a line equal to
    /// `line`.
    pub fn wait_for_line(&self, line: &str, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            if self.output().out.lines().any(|l| l == line) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Waits for the program to end and returns what it printed.
    pub fn finish(self) -> Output {
        let finished = self.vm.wait_finished(Some(Duration::from_secs(90)));
        if !finished {
            self.vm.stop();
            self.vm.wait_finished(Some(Duration::from_secs(5)));
        }
        let output = self.output();
        assert!(
            finished,
            "{} did not finish; it printed:\n{}\n{}",
            self.class, output.out, output.err
        );
        output
    }
}

/// Runs a program to its end ([`Program::start`], [`Program::finish`]).
pub fn run_with(
    classes: &Path,
    class: &str,
    args: &[&str],
    network: NetworkPolicy,
    setup: impl FnOnce(&mut Ctx<'_>),
) -> Output {
    Program::start(classes, class, args, network, setup).finish()
}

/// [`run_with`] without extra setup.
pub fn run(classes: &Path, class: &str, args: &[&str], network: NetworkPolicy) -> Output {
    run_with(classes, class, args, network, |_| {})
}

/// Checks that a program printed `expected` and nothing on `System.err`.
pub fn assert_output(output: &Output, expected: &str) {
    assert_eq!(output.err, "", "the program wrote to System.err");
    let actual = output.out.replace("\r\n", "\n");
    assert_eq!(actual.trim_end(), expected.trim_end());
}

/// A port nothing listens on.
pub fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// A request the test server received.
#[derive(Clone, Debug)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The value of header `name`, ignoring case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// The path of the target, without the query.
    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or_default()
    }
}

/// An HTTP server on 127.0.0.1 that records every request and answers
/// with `respond`, which writes the whole reply to the socket. It runs
/// until the test binary ends.
pub struct HttpServer {
    pub port: u16,
    pub requests: Arc<Mutex<Vec<Request>>>,
}

impl HttpServer {
    pub fn start(respond: fn(&Request, &mut TcpStream)) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests: Arc<Mutex<Vec<Request>>> = Arc::default();
        let log = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let log = Arc::clone(&log);
                std::thread::spawn(move || {
                    if let Some(request) = read_request(&mut stream) {
                        log.lock().unwrap().push(request.clone());
                        respond(&request, &mut stream);
                    }
                });
            }
        });
        Self { port, requests }
    }

    /// The requests received so far for `path`.
    pub fn requests_for(&self, path: &str) -> Vec<Request> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.path() == path)
            .cloned()
            .collect()
    }
}

/// Reads one request: its head, then a `Content-Length` body.
fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut raw = Vec::new();
    let mut byte = [0];
    while !raw.ends_with(b"\r\n\r\n") {
        if stream.read(&mut byte).ok()? == 0 {
            return None;
        }
        raw.push(byte[0]);
    }
    let text = String::from_utf8_lossy(&raw).into_owned();
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, target) = (first.next()?.to_owned(), first.next()?.to_owned());
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect();
    let length: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).ok()?;
    Some(Request {
        method,
        target,
        headers,
        body,
    })
}

/// Writes `parts` to `stream` one by one with `pause` between them, so the
/// reader sees them arrive over several of its read slices.
pub fn write_slowly(stream: &mut TcpStream, parts: &[&[u8]], pause: Duration) {
    for part in parts {
        stream.write_all(part).unwrap();
        stream.flush().unwrap();
        std::thread::sleep(pause);
    }
}
