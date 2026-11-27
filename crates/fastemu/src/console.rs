//! The terminal on Windows.
//!
//! `fastemu.exe` is a Windows (GUI) program, so opening it from Explorer, a
//! shortcut or the Start menu opens no console window beside FastEmu. A GUI
//! program gets no console from the terminal that starts it, though: its
//! standard output and error are unset unless they were redirected. So
//! [`attach`] joins the console of the program that started it, a terminal,
//! when it has nowhere else to write, and `fastemu run`, `--help`,
//! `--version` and the window's warnings print there. Output already going
//! into a pipe or a file stays there, as `cargo xtask compat` reads it.
//!
//! A terminal does not wait for a GUI program, so `fastemu.com`, the
//! console program beside `fastemu.exe` (`src/bin/fastemu-console.rs`),
//! starts it, waits, and ends with its exit code. Command Prompt and
//! PowerShell run `fastemu.com` for `fastemu`, since `.COM` comes before
//! `.EXE` in `PATHEXT`.

/// Joins the console of the program that started this one, when standard
/// output or error is unset. Does nothing when there is no such console, as
/// for Explorer, or on other systems.
#[cfg(windows)]
// The console is reached through the Win32 API only.
#[allow(unsafe_code)]
pub fn attach() {
    use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        SetStdHandle,
    };

    // SAFETY: the standard handles are read and set by value; nothing is
    // closed or freed.
    unsafe {
        let out = GetStdHandle(STD_OUTPUT_HANDLE);
        let err = GetStdHandle(STD_ERROR_HANDLE);
        let unset = |handle: HANDLE| handle.is_null() || handle == INVALID_HANDLE_VALUE;
        let (out_unset, err_unset) = (unset(out), unset(err));
        if !out_unset && !err_unset {
            return;
        }
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return;
        }
        // Attaching points the standard handles at the console. The ones
        // that were redirected, as `fastemu run game.jar > log.txt`, keep
        // their file or pipe.
        if !out_unset {
            SetStdHandle(STD_OUTPUT_HANDLE, out);
        }
        if !err_unset {
            SetStdHandle(STD_ERROR_HANDLE, err);
        }
    }
}

#[cfg(not(windows))]
pub fn attach() {}
