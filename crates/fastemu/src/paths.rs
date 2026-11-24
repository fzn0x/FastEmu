//! Where the desktop app keeps its files, following each platform's
//! conventions through the `dirs` crate.

use std::path::{Path, PathBuf};

use fastemu_ui::AppPaths;

/// The app's folder name: capitalised where folders usually are (Windows
/// and macOS), lower case on Linux and the BSDs.
fn app_dir_name() -> &'static str {
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        "FastEmu"
    } else {
        "fastemu"
    }
}

/// The folders for this user:
///
/// - settings and library in the config folder (`%APPDATA%\FastEmu`,
///   `~/Library/Application Support/FastEmu`, `~/.config/fastemu`);
/// - game saves in the data folder (the same on Windows and macOS,
///   `~/.local/share/fastemu` on Linux);
/// - screenshots in `FastEmu` under the user's Pictures folder.
///
/// A folder the platform does not name falls back to `.fastemu` in the
/// home folder, or in the working folder when there is no home either.
pub fn desktop() -> AppPaths {
    let fallback = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".fastemu");
    resolve(
        dirs::config_dir(),
        dirs::data_dir(),
        dirs::picture_dir(),
        &fallback,
    )
}

fn resolve(
    config: Option<PathBuf>,
    data: Option<PathBuf>,
    pictures: Option<PathBuf>,
    fallback: &Path,
) -> AppPaths {
    let name = app_dir_name();
    let data_dir = data.map_or_else(|| fallback.join("data"), |d| d.join(name));
    AppPaths {
        config_dir: config.map_or_else(|| fallback.join("config"), |d| d.join(name)),
        screenshot_dir: pictures
            .map_or_else(|| data_dir.join("screenshots"), |d| d.join("FastEmu")),
        data_dir,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_folders_get_the_app_name() {
        let paths = resolve(
            Some("cfg".into()),
            Some("data".into()),
            Some("pics".into()),
            Path::new("home/.fastemu"),
        );
        assert_eq!(paths.config_dir, Path::new("cfg").join(app_dir_name()));
        assert_eq!(paths.data_dir, Path::new("data").join(app_dir_name()));
        assert_eq!(paths.screenshot_dir, Path::new("pics/FastEmu"));
    }

    #[test]
    fn missing_folders_fall_back_to_the_home_folder() {
        let paths = resolve(None, None, None, Path::new("home/.fastemu"));
        assert_eq!(paths.config_dir, Path::new("home/.fastemu/config"));
        assert_eq!(paths.data_dir, Path::new("home/.fastemu/data"));
        assert_eq!(
            paths.screenshot_dir,
            Path::new("home/.fastemu/data/screenshots")
        );
    }
}
