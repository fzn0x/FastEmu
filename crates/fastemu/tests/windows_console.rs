//! fastemu.exe opens no console window, and the command line still prints
//! in a terminal through fastemu.com (#83).
#![cfg(windows)]

use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

const APP: &str = env!("CARGO_BIN_EXE_fastemu");
const LAUNCHER: &str = env!("CARGO_BIN_EXE_fastemu-console");

/// The subsystem a program's PE header names: 2 for a Windows (GUI)
/// program, which gets no console, 3 for a console program.
fn subsystem(path: &str) -> u16 {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let pe = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    assert_eq!(&bytes[pe..pe + 4], b"PE\0\0", "{path} is not a PE file");
    // The signature, the 20-byte file header, then the subsystem at offset
    // 68 of the optional header, the same for 32 and 64 bits.
    u16::from_le_bytes([bytes[pe + 92], bytes[pe + 93]])
}

#[test]
fn the_app_is_a_gui_program_and_the_launcher_a_console_one() {
    assert_eq!(subsystem(APP), 2, "fastemu.exe would open a console window");
    assert_eq!(
        subsystem(LAUNCHER),
        3,
        "fastemu.com would not hold the terminal"
    );
}

#[test]
fn the_launcher_passes_arguments_output_and_exit_codes() {
    let version = Command::new(LAUNCHER).arg("--version").output().unwrap();
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        format!("fastemu {}", env!("CARGO_PKG_VERSION"))
    );

    let bad = Command::new(LAUNCHER)
        .arg("--no-such-option")
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("unknown option --no-such-option"));
}

#[test]
fn the_app_writes_into_a_pipe_without_a_console() {
    // `cargo xtask compat` and `bench` read the app's output through pipes.
    let out = Command::new(APP).arg("--version").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("fastemu "));
}

/// A terminal hands its console to the programs it starts. This test runs
/// itself again in a console of its own, hidden, and there starts
/// fastemu.com as a terminal would; then it reads back what the console
/// shows.
#[test]
fn the_command_line_prints_into_the_terminal() {
    if let Some(report) = std::env::var_os("FASTEMU_CONSOLE_PROBE") {
        probe(&report);
        return;
    }
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("console.txt");
    let errors = fs::File::create(dir.path().join("probe.txt")).unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args([
            "the_command_line_prints_into_the_terminal",
            "--exact",
            "--nocapture",
        ])
        .env("FASTEMU_CONSOLE_PROBE", &report)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(errors)
        .status()
        .unwrap();
    let probe_log = fs::read_to_string(dir.path().join("probe.txt")).unwrap_or_default();
    assert!(status.success(), "the probe failed:\n{probe_log}");
    let console = fs::read_to_string(&report).unwrap();
    let version = format!("fastemu {}", env!("CARGO_PKG_VERSION"));
    assert!(console.contains(&version), "the console shows:\n{console}");
    assert!(
        console.contains("unknown option --no-such-option"),
        "the console shows:\n{console}"
    );
    assert!(
        console.contains("exit codes: 0 2"),
        "the console shows:\n{console}"
    );
}

/// In the hidden console: fastemu.com with the console as its standard
/// handles, as Command Prompt starts it, then the console's text.
fn probe(report: &std::ffi::OsStr) {
    let open = |name: &str| {
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"))
    };
    let (screen, keys) = (open("CONOUT$"), open("CONIN$"));
    let mut codes = Vec::new();
    for arg in ["--version", "--no-such-option"] {
        let status = Command::new(LAUNCHER)
            .arg(arg)
            .stdin(keys.try_clone().unwrap())
            .stdout(screen.try_clone().unwrap())
            .stderr(screen.try_clone().unwrap())
            .status()
            .unwrap();
        codes.push(status.code().unwrap_or(-1).to_string());
    }
    let text = format!(
        "{}\nexit codes: {}\n",
        console_text(&screen),
        codes.join(" ")
    );
    fs::write(Path::new(&OsString::from(report)), text).unwrap();
}

/// The characters of a console screen buffer, line by line, up to the
/// cursor.
// The console is read through the Win32 API only.
#[allow(unsafe_code)]
fn console_text(screen: &fs::File) -> String {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::{
        CONSOLE_SCREEN_BUFFER_INFO, COORD, GetConsoleScreenBufferInfo, ReadConsoleOutputCharacterW,
    };

    let handle = screen.as_raw_handle();
    // SAFETY: `handle` is the open console buffer `screen` owns; the info
    // and the text are written into buffers of the sizes given.
    unsafe {
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
        assert_ne!(
            GetConsoleScreenBufferInfo(handle, &raw mut info),
            0,
            "not a console"
        );
        let width = usize::try_from(info.dwSize.X).unwrap().max(1);
        let rows = usize::try_from(info.dwCursorPosition.Y).unwrap() + 1;
        let mut text = vec![0u16; width * rows];
        let mut read = 0u32;
        ReadConsoleOutputCharacterW(
            handle,
            text.as_mut_ptr(),
            u32::try_from(text.len()).unwrap(),
            COORD { X: 0, Y: 0 },
            &raw mut read,
        );
        text.truncate(read as usize);
        String::from_utf16_lossy(&text)
            .chars()
            .collect::<Vec<_>>()
            .chunks(width)
            .map(|line| line.iter().collect::<String>().trim_end().to_owned())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
