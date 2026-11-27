//! `fastemu.com`, the console program beside `fastemu.exe` on Windows.
//!
//! `fastemu.exe` is a GUI program, so a terminal neither waits for it nor
//! sees its exit code. Typed in Command Prompt or PowerShell, `fastemu`
//! runs this program instead (`.COM` comes before `.EXE` in `PATHEXT`): it
//! starts `fastemu.exe` from its own folder with the same arguments, which
//! prints into this console (see `src/console.rs`), waits for it, and ends
//! with its exit code. The release renames it from `fastemu-console.exe`.

#[cfg(windows)]
// Ctrl+C is handled through the Win32 API only.
#[allow(unsafe_code)]
fn main() {
    use windows_sys::Win32::Foundation::TRUE;
    use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
    use windows_sys::core::BOOL;

    /// Ctrl+C and Ctrl+Break reach every program on the console:
    /// `fastemu.exe` stops on them, and this one waits for it to end rather
    /// than return to the prompt first. A handler, unlike ignoring Ctrl+C
    /// outright, is not inherited by `fastemu.exe`.
    unsafe extern "system" fn wait_for_fastemu(_event: u32) -> BOOL {
        TRUE
    }

    // SAFETY: the handler is a plain function that lives as long as the
    // program.
    unsafe {
        SetConsoleCtrlHandler(Some(wait_for_fastemu), TRUE);
    }

    let exe = match std::env::current_exe() {
        Ok(me) => me.with_file_name("fastemu.exe"),
        Err(err) => {
            eprintln!("fastemu: cannot find this program's folder: {err}");
            std::process::exit(1);
        }
    };
    match std::process::Command::new(&exe)
        .args(std::env::args_os().skip(1))
        .status()
    {
        Ok(status) => std::process::exit(status.code().unwrap_or(1)),
        Err(err) => {
            eprintln!("fastemu: cannot start {}: {err}", exe.display());
            std::process::exit(1);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("fastemu-console is the Windows command line of FastEmu: run fastemu instead");
    std::process::exit(2);
}
