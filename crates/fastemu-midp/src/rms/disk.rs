//! Store files on disk: where they live, and writing them so that a crash
//! or a killed instance never leaves half a store.
//!
//! Every function here does blocking I/O; natives call them inside
//! `Ctx::without_lock`.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use super::name::{StoreName, folder_name};

/// The extension of a store file.
const STORE_EXT: &str = "rms";
/// The extension of the file a store is written to before it replaces the
/// old one. A leftover one is an unfinished write and is ignored.
const TEMP_EXT: &str = "tmp";
/// The extension of the copy kept of a damaged store file.
const DAMAGED_EXT: &str = "bad";

/// The folder under an instance's data folder that holds its record stores.
pub fn rms_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("rms")
}

/// The folder that holds the stores of another suite, identified by its
/// `MIDlet-Vendor` and `MIDlet-Name`, inside an instance's data folder.
pub fn suite_dir(data_dir: &Path, vendor: &str, suite: &str) -> PathBuf {
    rms_dir(data_dir)
        .join("suites")
        .join(folder_name(vendor))
        .join(folder_name(suite))
}

/// The file of store `name` in folder `dir`.
pub fn store_path(dir: &Path, name: &StoreName) -> PathBuf {
    dir.join(format!("{}.{STORE_EXT}", name.file_stem()))
}

/// The stores in `dir` and the sizes of their files. A folder that does
/// not exist holds none; files that are not store files are skipped.
pub fn scan(dir: &Path) -> io::Result<Vec<(StoreName, u64)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut stores = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new(STORE_EXT)) {
            continue;
        }
        let Some(name) = path
            .file_stem()
            .and_then(OsStr::to_str)
            .and_then(StoreName::from_file_stem)
        else {
            continue;
        };
        let meta = entry.metadata()?;
        if meta.is_file() {
            stores.push((name, meta.len()));
        }
    }
    stores.sort();
    Ok(stores)
}

/// The contents of a store file, or `None` when it does not exist.
pub fn read(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Replaces the file at `path` with `bytes` in one step: the bytes go to a
/// temporary file beside it, which is flushed to the disk and then renamed
/// over the old file. Whenever the process dies, the file holds either the
/// old contents or the new ones.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension(TEMP_EXT);
    let written = File::create(&temp).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    if let Err(e) = written.and_then(|()| rename_replacing(&temp, path)) {
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    if let Some(dir) = path.parent() {
        sync_dir(dir);
    }
    Ok(())
}

/// Deletes a store file. One that is already gone counts as deleted.
pub fn remove(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

/// Keeps a copy of a damaged store file beside it, so that recovering the
/// store never destroys the only copy of what the file held. Returns the
/// copy's path.
pub fn keep_damaged(path: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let copy = path.with_extension(DAMAGED_EXT);
    write_atomic(&copy, bytes)?;
    Ok(copy)
}

/// `fs::rename`, retried for a moment on Windows, where a virus scanner or
/// the search indexer may hold the old file open just after it changed.
fn rename_replacing(from: &Path, to: &Path) -> io::Result<()> {
    let mut attempt = 0;
    loop {
        match fs::rename(from, to) {
            Err(e)
                if cfg!(windows) && e.kind() == io::ErrorKind::PermissionDenied && attempt < 5 =>
            {
                attempt += 1;
                std::thread::sleep(std::time::Duration::from_millis(10 * attempt));
            }
            result => return result,
        }
    }
}

/// Flushes a folder's entries, so a rename survives a power cut. Only
/// Unix-like systems can open a folder for this; elsewhere the file system
/// journals renames itself.
fn sync_dir(dir: &Path) {
    #[cfg(unix)]
    if let Ok(dir) = File::open(dir) {
        let _ = dir.sync_all();
    }
    #[cfg(not(unix))]
    let _ = dir;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> StoreName {
        StoreName::from_text(text).unwrap()
    }

    #[test]
    fn atomic_writes_replace_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = store_path(&dir.path().join("rms"), &name("Save"));
        write_atomic(&path, b"one").unwrap();
        write_atomic(&path, b"two, longer").unwrap();
        assert_eq!(read(&path).unwrap().as_deref(), Some(&b"two, longer"[..]));
        assert!(!path.with_extension(TEMP_EXT).exists());
    }

    #[test]
    fn missing_files_and_folders_are_not_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nothing");
        assert!(scan(&missing).unwrap().is_empty());
        assert_eq!(read(&store_path(&missing, &name("x"))).unwrap(), None);
        remove(&store_path(&missing, &name("x"))).unwrap();
    }

    #[test]
    fn scan_lists_only_store_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write_atomic(&store_path(root, &name("b")), b"12345").unwrap();
        write_atomic(&store_path(root, &name("A")), b"1").unwrap();
        // An interrupted write, a damaged copy, a foreign file, a stem that
        // is not canonical and a folder are all skipped.
        fs::write(root.join("c.tmp"), b"partial").unwrap();
        keep_damaged(&store_path(root, &name("d")), b"bad").unwrap();
        fs::write(root.join("notes.txt"), b"hello").unwrap();
        fs::write(root.join("Upper.rms"), b"?").unwrap();
        fs::create_dir(root.join("dir.rms")).unwrap();
        let found = scan(root).unwrap();
        assert_eq!(found, vec![(name("A"), 1), (name("b"), 5)]);
    }

    #[test]
    fn a_leftover_temporary_file_does_not_block_writes() {
        let dir = tempfile::tempdir().unwrap();
        let path = store_path(dir.path(), &name("x"));
        fs::write(path.with_extension(TEMP_EXT), b"left by a crash").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn other_suites_get_their_own_folders() {
        let data = Path::new("data");
        let a = suite_dir(data, "Acme", "Game");
        let b = suite_dir(data, "Acme", "Game 2");
        assert_ne!(a, b);
        assert!(a.starts_with(rms_dir(data)));
        assert_eq!(suite_dir(data, "Acme", "Game"), a);
    }
}
