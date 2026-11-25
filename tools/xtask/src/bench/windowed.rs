//! `bench --windows`: the desktop app with every game in its window,
//! against one MicroEmulator window per game. It opens windows, so it
//! runs only when asked, with the owner's consent.
//!
//! The app keeps its library and settings in `%APPDATA%\FastEmu`, the
//! folder of the owner's own app, and starting it with games adds them
//! to that library. So the bench refuses to run while any `fastemu.exe`
//! runs, backs `library.json` and `settings.json` up first, and puts
//! them back after, deleting a file that was not there before. A backup
//! left by a run that was stopped part way makes the next run refuse to
//! start, and says how to put it back. The save folders the app made stay
//! (their numbers are never used again) and are printed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::Options;
use super::java::MICROEMU_JARS;

/// The app's files the bench backs up and puts back.
pub const APP_FILES: [&str; 2] = ["library.json", "settings.json"];

/// The folder name of a backup.
pub const BACKUP: &str = "app-config-backup";

/// The desktop app's config folder on Windows: `%APPDATA%\FastEmu`, as
/// the app finds it (`dirs::config_dir`, the roaming application data
/// folder).
pub fn app_config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("FastEmu"))
}

/// The image names of the running processes, from `tasklist`. Fails
/// rather than return an empty list, so an unknown state never passes for
/// "FastEmu is closed".
pub fn running_processes() -> Result<Vec<String>, String> {
    let output = Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output()
        .map_err(|e| format!("listing the running processes: {e}"))?;
    if !output.status.success() {
        return Err("tasklist could not list the running processes".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.split(',').next())
        .map(|name| name.trim().trim_matches('"').to_owned())
        .filter(|name| !name.is_empty())
        .collect())
}

/// Refuses while the app runs: it shares the library the bench changes.
pub fn refuse_running(processes: &[String]) -> Result<(), String> {
    if processes
        .iter()
        .any(|p| p.eq_ignore_ascii_case("fastemu.exe"))
    {
        return Err(
            "fastemu.exe is running: close FastEmu first, since --windows changes the \
             library it shares"
                .into(),
        );
    }
    Ok(())
}

/// Refuses when `backup` is left from a run that did not finish, and
/// says how to put the app's files back.
pub fn refuse_stale(backup: &Path) -> Result<(), String> {
    if backup.exists() {
        let config = app_config_dir().map_or_else(
            || "%APPDATA%\\FastEmu".to_owned(),
            |d| d.display().to_string(),
        );
        return Err(format!(
            "{} is left from a bench that did not finish. Copy the files in it back \
             into {config} (and delete the app's NAME there for each NAME.absent), \
             then delete the folder and run again",
            backup.display()
        ));
    }
    Ok(())
}

/// Copies the app's files from `config` into `backup`, marking a file
/// that does not exist with `NAME.absent`. Refuses when `backup` is left
/// from a run that did not finish.
pub fn back_up(config: &Path, backup: &Path) -> Result<(), String> {
    refuse_stale(backup)?;
    fs::create_dir_all(backup).map_err(|e| format!("{}: {e}", backup.display()))?;
    for name in APP_FILES {
        let from = config.join(name);
        let result = if from.is_file() {
            fs::copy(&from, backup.join(name)).map(drop)
        } else {
            fs::write(backup.join(format!("{name}.absent")), "")
        };
        result.map_err(|e| format!("backing up {}: {e}", from.display()))?;
    }
    Ok(())
}

/// Puts the app's files back from `backup` and deletes it.
pub fn restore(config: &Path, backup: &Path) -> Result<(), String> {
    for name in APP_FILES {
        let to = config.join(name);
        let saved = backup.join(name);
        if saved.is_file() {
            fs::copy(&saved, &to).map_err(|e| format!("restoring {}: {e}", to.display()))?;
        } else if backup.join(format!("{name}.absent")).is_file() && to.exists() {
            fs::remove_file(&to).map_err(|e| format!("removing {}: {e}", to.display()))?;
        }
    }
    fs::remove_dir_all(backup).map_err(|e| format!("{}: {e}", backup.display()))
}

/// The save folders under `config/games`.
pub fn save_folders(config: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(config.join("games"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// MicroEmulator's own window for `jar`, as a player starts it, with its
/// own home folder.
pub fn microemu_window(java: &Path, options: &Options, home: &Path, jar: &Path) -> Command {
    let mut command = Command::new(java);
    command.args(&options.me_jvm_args);
    let mut user_home = std::ffi::OsString::from("-Duser.home=");
    user_home.push(home);
    command.arg(user_home);
    if let Some(dir) = &options.microemu {
        let jars: Vec<PathBuf> = MICROEMU_JARS.iter().map(|j| dir.join(j)).collect();
        if let Ok(classpath) = std::env::join_paths(jars) {
            command.arg("-cp").arg(classpath);
        }
    }
    command
        .arg("org.microemu.app.Main")
        .args(["--resizableDevice", "240", "320", "--rms", "file"])
        .arg(jar);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_files_come_back() {
        let dir =
            std::env::temp_dir().join(format!("fastemu-bench-windows-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let config = dir.join("FastEmu");
        let backup = dir.join("out").join(BACKUP);
        fs::create_dir_all(&config).unwrap();
        fs::write(config.join("library.json"), "old").unwrap();
        back_up(&config, &backup).unwrap();
        assert!(backup.join("settings.json.absent").is_file());
        // What the app does with the games it is given.
        fs::write(config.join("library.json"), "new").unwrap();
        fs::write(config.join("settings.json"), "made").unwrap();
        fs::create_dir_all(config.join("games").join("7")).unwrap();
        restore(&config, &backup).unwrap();
        assert_eq!(
            fs::read_to_string(config.join("library.json")).unwrap(),
            "old"
        );
        assert!(
            !config.join("settings.json").exists(),
            "a file absent before is deleted"
        );
        assert!(!backup.exists());
        assert_eq!(save_folders(&config), ["7"]);
        // A backup left by a stopped run refuses the next one.
        fs::create_dir_all(&backup).unwrap();
        let refused = back_up(&config, &backup).unwrap_err();
        assert!(refused.contains("did not finish"), "{refused}");
        // So does a running app.
        assert!(refuse_running(&["explorer.exe".into(), "FastEmu.exe".into()]).is_err());
        assert!(refuse_running(&["explorer.exe".into(), "java.exe".into()]).is_ok());
        // The real list holds at least this test.
        if cfg!(windows) {
            assert!(!running_processes().unwrap().is_empty());
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
