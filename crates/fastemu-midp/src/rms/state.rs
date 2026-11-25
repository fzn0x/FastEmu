//! The record stores of one virtual machine: which exist, which are in
//! memory, which are open, and the lock that keeps their files in order.
//!
//! The state is a VM extension ([`Rms`]), made on the first record store
//! call from the instance's [`InstanceEnv`]. Its fields change only while
//! the VM lock is held. File work runs with the VM lock released and the
//! file lock held, so other Java threads keep running while a store is
//! read or written, and every change reaches the disk in the order it was
//! made. A thread takes the file lock only with the VM lock released (or
//! with `try_lock`), so the two locks can never deadlock.

use std::collections::{BTreeMap, HashMap};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, TryLockError};
use std::time::{SystemTime, UNIX_EPOCH};

use fastemu_core::LogLevel;
use fastemu_jvm::{Ctx, GlobalRef, JResult, Throw};

use super::budget::{Clock, SLICE, SystemClock, WRITE_BURST, WRITE_RATE, WriteBudget};
use super::disk;
use super::format::StoreData;
use super::name::StoreName;
use super::{RECORD_STORE_EXCEPTION, RECORD_STORE_FULL_EXCEPTION};
use crate::env::{InstanceEnv, env};

/// The record store space each suite gets unless the launcher sets
/// another with [`set_quota`]: 1 MiB, what J2ME Loader and MicroEmulator
/// report and more than most phones offered.
pub const DEFAULT_QUOTA: u64 = 1 << 20;

/// How many other suites' folders one virtual machine remembers. A game
/// names any vendor and suite it likes, and each one it names that has
/// stores of its own would take a place in the map.
const MAX_OTHER_OWNERS: usize = 16;

/// How many record stores a suite may have. Each is a file, and the quota
/// counts the bytes in them, so without a limit a game that makes empty
/// stores gets tens of thousands of files for its megabyte. Phones allowed
/// a few dozen at most and games make fewer than ten.
pub(crate) const MAX_STORES: usize = 64;

/// Whose stores a store belongs to.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Owner {
    /// The suite this instance runs.
    Own,
    /// Another suite, by `MIDlet-Vendor` and `MIDlet-Name`.
    Other { vendor: String, suite: String },
}

/// One record store: its owner and its name.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct StoreKey {
    pub(crate) owner: Owner,
    pub(crate) name: StoreName,
}

/// A store that exists, in memory or not.
pub(crate) struct Entry {
    /// The bytes its file takes, which count against the quota.
    size: u64,
    /// Its contents, while it is open (or being opened).
    data: Option<StoreData>,
}

/// A store some code holds open: its one `RecordStore` object and how many
/// `openRecordStore` calls are not yet matched by `closeRecordStore`.
pub(crate) struct OpenStore {
    pub(crate) object: GlobalRef,
    pub(crate) opens: u32,
}

/// Every record store of one virtual machine.
pub(crate) struct Rms {
    data_dir: PathBuf,
    own_vendor: String,
    own_suite: String,
    quota: u64,
    files: Arc<Mutex<()>>,
    /// How much the game may still write to the disk.
    budget: WriteBudget,
    /// The time the budget is read against.
    clock: Arc<dyn Clock>,
    /// Whether the log has said that the game was slowed.
    slowed: bool,
    /// The stores of each owner whose folder has been listed.
    suites: HashMap<Owner, BTreeMap<StoreName, Entry>>,
    /// The stores that are open.
    pub(crate) open: HashMap<StoreKey, OpenStore>,
}

impl Rms {
    fn new(env: &InstanceEnv) -> Self {
        let clock: Arc<dyn Clock> = Arc::new(SystemClock::new());
        Self {
            data_dir: env.data_dir.clone(),
            own_vendor: env.suite.vendor.clone(),
            own_suite: env.suite.name.clone(),
            quota: DEFAULT_QUOTA,
            files: Arc::default(),
            budget: WriteBudget::new(WRITE_BURST, WRITE_RATE, clock.now()),
            clock,
            slowed: false,
            suites: HashMap::new(),
            open: HashMap::new(),
        }
    }

    /// The owner named by a vendor and a suite name: this suite when they
    /// are its own.
    pub(crate) fn owner(&self, vendor: &str, suite: &str) -> Owner {
        if vendor == self.own_vendor && suite == self.own_suite {
            Owner::Own
        } else {
            Owner::Other {
                vendor: vendor.to_owned(),
                suite: suite.to_owned(),
            }
        }
    }

    /// The lock every file operation holds; take it with [`lock_files`].
    pub(crate) fn files(&self) -> Arc<Mutex<()>> {
        Arc::clone(&self.files)
    }

    fn dir(&self, owner: &Owner) -> PathBuf {
        match owner {
            Owner::Own => disk::rms_dir(&self.data_dir),
            Owner::Other { vendor, suite } => disk::suite_dir(&self.data_dir, vendor, suite),
        }
    }

    fn path(&self, key: &StoreKey) -> PathBuf {
        disk::store_path(&self.dir(&key.owner), &key.name)
    }

    fn entry_mut(&mut self, key: &StoreKey) -> Option<&mut Entry> {
        self.suites.get_mut(&key.owner)?.get_mut(&key.name)
    }

    /// The names of an owner's stores, once its folder has been listed.
    pub(crate) fn names(&self, owner: &Owner) -> Vec<StoreName> {
        self.suites
            .get(owner)
            .map(|stores| stores.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// The contents of a store that is in memory.
    pub(crate) fn data(&self, key: &StoreKey) -> Option<&StoreData> {
        self.suites.get(&key.owner)?.get(&key.name)?.data.as_ref()
    }

    /// Whether `owner` may make one more store than it has now.
    pub(crate) fn may_add_store(&self, owner: &Owner) -> bool {
        self.suites.get(owner).map_or(0, BTreeMap::len) < MAX_STORES
    }

    /// The bytes an owner's stores take.
    fn usage(&self, owner: &Owner) -> u64 {
        self.suites
            .get(owner)
            .map_or(0, |stores| stores.values().map(|e| e.size).sum())
    }

    /// The room an owner's stores have left to grow, never negative.
    pub(crate) fn available(&self, owner: &Owner) -> u64 {
        self.quota.saturating_sub(self.usage(owner))
    }

    /// Whether a store of `owner` may go from `old` to `new` bytes. A store
    /// that does not grow always may, even past the quota.
    pub(crate) fn fits(&self, owner: &Owner, old: u64, new: u64) -> bool {
        new <= old || self.usage(owner).saturating_sub(old) + new <= self.quota
    }

    /// Drops a closed store's contents from memory; the next open reads
    /// its file again.
    pub(crate) fn unload(&mut self, key: &StoreKey) {
        if let Some(entry) = self.entry_mut(key) {
            entry.data = None;
        }
    }
}

/// The record store state of the virtual machine `ctx` runs in, made from
/// its [`InstanceEnv`] on first use.
pub(crate) fn rms<'a>(ctx: &'a mut Ctx<'_>) -> &'a mut Rms {
    if ctx.extension::<Rms>().is_none() {
        let rms = Rms::new(env(ctx));
        ctx.set_extension(rms);
    }
    ctx.extension_mut::<Rms>().expect("installed above")
}

/// Sets how many bytes the stores of each suite may take in this virtual
/// machine (the default is [`DEFAULT_QUOTA`]). `getSizeAvailable` reports
/// what is left of it, and a change that would pass it throws
/// `RecordStoreFullException`. For the launcher, when a device profile
/// names its own size, and for tests. Call it after the [`InstanceEnv`] is
/// installed.
pub fn set_quota(ctx: &mut Ctx<'_>, bytes: u64) {
    rms(ctx).quota = bytes;
}

/// Sets how much this virtual machine's record stores may write to the
/// disk: `burst` bytes at once, then `bytes_per_second` (the defaults are
/// [`WRITE_BURST`] and [`WRITE_RATE`]). A save that would pass the limit
/// waits for the budget to refill, and still returns only once the store
/// is on the disk. For the launcher, when a device profile or a setting
/// names its own limit, and for tests. Call it after the [`InstanceEnv`]
/// is installed.
pub fn set_write_limit(ctx: &mut Ctx<'_>, burst: u64, bytes_per_second: u64) {
    let rms = rms(ctx);
    rms.budget = WriteBudget::new(burst, bytes_per_second, rms.clock.now());
}

/// Waits, with the VM lock released, until the game's earlier writes are
/// paid for: not at all while its write budget is not in debt. The wait
/// sleeps in slices of [`SLICE`] and looks at the stop flag after each, so
/// a stopping machine is not held up by a game that was slowed; it gives
/// `Err(Throw::Stop)`. The first wait of a machine is logged once, since
/// a player may wonder why a game is slow to save.
///
/// A write waits before it is made, so the save that spends the last of
/// the budget is not itself made to wait for it, and the last save of a
/// game that is closing goes through.
pub(crate) fn pace(ctx: &mut Ctx<'_>) -> JResult<()> {
    loop {
        let (wait, clock, say) = {
            let rms = rms(ctx);
            let wait = rms.budget.wait(rms.clock.now());
            let say = !wait.is_zero() && !rms.slowed;
            rms.slowed |= say;
            (wait, Arc::clone(&rms.clock), say)
        };
        if wait.is_zero() {
            return Ok(());
        }
        if ctx.is_stopping() {
            return Err(Throw::Stop);
        }
        if say {
            let rate = rms(ctx).budget.rate();
            ctx.log(
                LogLevel::Warn,
                format!(
                    "RMS: the game writes record stores faster than {} KiB a second; \
                     its saves wait for the disk",
                    rate >> 10
                ),
            );
        }
        ctx.without_lock(|| clock.sleep(wait.min(SLICE)));
    }
}

/// Takes the bytes of a write that was made from the write budget.
fn charge(ctx: &mut Ctx<'_>, bytes: u64) {
    let rms = rms(ctx);
    rms.budget.spend(bytes, rms.clock.now());
}

/// Takes the file lock, releasing the VM lock while it waits for another
/// thread's file work to end.
pub(crate) fn lock_files<'m>(ctx: &mut Ctx<'_>, files: &'m Mutex<()>) -> MutexGuard<'m, ()> {
    match files.try_lock() {
        Ok(guard) => guard,
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
        Err(TryLockError::WouldBlock) => {
            ctx.without_lock(|| files.lock().unwrap_or_else(PoisonError::into_inner))
        }
    }
}

/// The time now, as `System.currentTimeMillis` gives it.
pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

// The functions below do file work: callers hold the file lock.

/// Lists an owner's stores, once per virtual machine. The list of another
/// suite is kept only when it holds a store, and for [`MAX_OTHER_OWNERS`]
/// suites at most. A folder with no store is read again at the next call,
/// which is one directory read, so a game that asks for the stores of
/// thousands of suites that are not there does not grow the map. A suite
/// with stores that would be one too many throws `RecordStoreException`:
/// its stores are there, so the game must not be told they are missing.
pub(crate) fn scan(ctx: &mut Ctx<'_>, owner: &Owner) -> JResult<()> {
    let dir = {
        let rms = rms(ctx);
        if rms.suites.contains_key(owner) {
            return Ok(());
        }
        rms.dir(owner)
    };
    match ctx.without_lock(|| disk::scan(&dir)) {
        Ok(found) => {
            if matches!(owner, Owner::Other { .. }) {
                if found.is_empty() {
                    return Ok(());
                }
                let kept = rms(ctx)
                    .suites
                    .keys()
                    .filter(|o| matches!(o, Owner::Other { .. }))
                    .count();
                if kept >= MAX_OTHER_OWNERS {
                    return ctx.throw(
                        RECORD_STORE_EXCEPTION,
                        "too many other suites' record stores are in use",
                    );
                }
            }
            let stores = found
                .into_iter()
                .map(|(name, size)| (name, Entry { size, data: None }))
                .collect();
            rms(ctx).suites.insert(owner.clone(), stores);
            Ok(())
        }
        Err(e) => Err(io_failure(ctx, "list the record stores in", &dir, &e)),
    }
}

/// Brings a store into memory. `Ok(false)` when it does not exist.
pub(crate) fn load(ctx: &mut Ctx<'_>, key: &StoreKey) -> JResult<bool> {
    scan(ctx, &key.owner)?;
    let path = {
        let rms = rms(ctx);
        match rms.entry_mut(key) {
            None => return Ok(false),
            Some(entry) if entry.data.is_some() => return Ok(true),
            Some(_) => rms.path(key),
        }
    };
    let name = key.name.clone();
    let now = now_millis();
    let read = ctx.without_lock(|| {
        let Some(bytes) = disk::read(&path)? else {
            return Ok(None);
        };
        let loaded = StoreData::decode(&bytes, &name, now);
        let damaged = (!loaded.problems.is_empty()).then_some(bytes);
        io::Result::Ok(Some((loaded, damaged)))
    });
    match read {
        Ok(Some((loaded, damaged))) => {
            for problem in &loaded.problems {
                ctx.log(
                    LogLevel::Warn,
                    format!("RMS: record store {name:?} ({}): {problem}", path.display()),
                );
            }
            if let Some(bytes) = damaged {
                keep_copy(ctx, &path, &bytes)?;
            }
            if let Some(entry) = rms(ctx).entry_mut(key) {
                entry.size = loaded.data.size();
                entry.data = Some(loaded.data);
            }
            Ok(true)
        }
        Ok(None) => {
            // Deleted by something outside the game since the listing.
            if let Some(stores) = rms(ctx).suites.get_mut(&key.owner) {
                stores.remove(&key.name);
            }
            Ok(false)
        }
        Err(e) => Err(io_failure(ctx, "read", &path, &e)),
    }
}

/// Keeps a copy of a damaged store file, which is another write of the
/// whole file: a game that opens a damaged store again and again, as it
/// can before it changes the store, writes the copy each time. So the
/// copy waits for the write budget and is paid from it like a save.
fn keep_copy(ctx: &mut Ctx<'_>, path: &Path, bytes: &[u8]) -> JResult<()> {
    pace(ctx)?;
    let copy = ctx.without_lock(|| disk::keep_damaged(path, bytes));
    charge(ctx, bytes.len() as u64);
    match copy {
        Ok(copy) => ctx.log(
            LogLevel::Warn,
            format!("RMS: the damaged file is kept as {}", copy.display()),
        ),
        Err(e) => ctx.log(
            LogLevel::Warn,
            format!("RMS: could not keep a copy of the damaged file: {e}"),
        ),
    }
    Ok(())
}

/// Writes a store's new contents to its file and, once they are there,
/// makes them the store's contents in memory. The write waits first, when
/// the game's earlier writes have spent its write budget ([`pace`]), and
/// is paid for from the budget whether it works or not: the whole file is
/// written, and a failed write may have written most of it.
pub(crate) fn save(ctx: &mut Ctx<'_>, key: &StoreKey, data: StoreData) -> JResult<()> {
    pace(ctx)?;
    let path = rms(ctx).path(key);
    let (bytes, written) = ctx.without_lock(|| {
        let bytes = data.encode();
        let written = disk::write_atomic(&path, &bytes);
        (bytes.len() as u64, written)
    });
    charge(ctx, bytes);
    match written.map(|()| bytes) {
        Ok(size) => {
            let stores = rms(ctx).suites.entry(key.owner.clone()).or_default();
            stores.insert(
                key.name.clone(),
                Entry {
                    size,
                    data: Some(data),
                },
            );
            Ok(())
        }
        Err(e) => Err(io_failure(ctx, "save", &path, &e)),
    }
}

/// Deletes a store's file. `Ok(false)` when it does not exist.
pub(crate) fn delete(ctx: &mut Ctx<'_>, key: &StoreKey) -> JResult<bool> {
    scan(ctx, &key.owner)?;
    let path = {
        let rms = rms(ctx);
        if rms.entry_mut(key).is_none() {
            return Ok(false);
        }
        rms.path(key)
    };
    match ctx.without_lock(|| disk::remove(&path)) {
        Ok(()) => {
            if let Some(stores) = rms(ctx).suites.get_mut(&key.owner) {
                stores.remove(&key.name);
            }
            Ok(true)
        }
        Err(e) => Err(io_failure(ctx, "delete", &path, &e)),
    }
}

/// Logs a failed file operation with its path and the system's text, and
/// returns the exception for the game, which reads a fixed text for the
/// kind of error and no host path or system text:
/// `RecordStoreFullException` for a full disk, `RecordStoreException`
/// otherwise.
fn io_failure(ctx: &mut Ctx<'_>, what: &str, path: &Path, e: &io::Error) -> Throw {
    ctx.log(
        LogLevel::Warn,
        format!("RMS: could not {what} {}: {e}", path.display()),
    );
    let (class, reason) = match e.kind() {
        io::ErrorKind::StorageFull => (RECORD_STORE_FULL_EXCEPTION, "the disk is full"),
        io::ErrorKind::PermissionDenied => (RECORD_STORE_EXCEPTION, "permission denied"),
        io::ErrorKind::NotFound => (RECORD_STORE_EXCEPTION, "the file is missing"),
        io::ErrorKind::ReadOnlyFilesystem => (RECORD_STORE_EXCEPTION, "the disk is read only"),
        _ => (RECORD_STORE_EXCEPTION, "I/O error"),
    };
    ctx.new_exception(
        class,
        Some(&format!(
            "the record store file could not be used: {reason}"
        )),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use fastemu_jvm::{MemorySource, Vm, VmConfig};

    use super::*;

    /// Runs `body` on a virtual machine whose record stores are in `dir`.
    fn with_ctx(dir: &Path, body: impl FnOnce(&mut Ctx<'_>)) {
        let env = InstanceEnv::for_tests(dir);
        let config = VmConfig {
            log: Arc::new(|_| {}),
            ..VmConfig::default()
        };
        let vm = Vm::new(config, Arc::new(MemorySource::new()), Vec::new());
        vm.attach("test", move |ctx| {
            ctx.set_extension(env);
            body(ctx);
        });
    }

    fn other(vendor: &str, suite: &str) -> Owner {
        Owner::Other {
            vendor: vendor.to_owned(),
            suite: suite.to_owned(),
        }
    }

    fn others(ctx: &mut Ctx<'_>) -> usize {
        rms(ctx)
            .suites
            .keys()
            .filter(|o| matches!(o, Owner::Other { .. }))
            .count()
    }

    #[test]
    fn a_failed_file_operation_gives_the_game_a_fixed_text_and_the_log_the_system_text() {
        let lines: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
        let sink = Arc::clone(&lines);
        let config = VmConfig {
            log: Arc::new(move |line| sink.lock().unwrap().push(line.text)),
            ..VmConfig::default()
        };
        let vm = Vm::new(
            config,
            Arc::new(MemorySource::new()),
            crate::classes(&fastemu_core::default_profile()),
        );
        vm.attach("test", |ctx| {
            let path = Path::new("secret-folder/store.rms");
            let fixed = [
                (io::ErrorKind::StorageFull, "RecordStoreFullException: the record store file could not be used: the disk is full"),
                (io::ErrorKind::PermissionDenied, "RecordStoreException: the record store file could not be used: permission denied"),
                (io::ErrorKind::InvalidInput, "RecordStoreException: the record store file could not be used: I/O error"),
            ];
            for (kind, expected) in fixed {
                let e = io::Error::from(kind);
                let Throw::Exception(exception) = io_failure(ctx, "read", path, &e) else {
                    panic!("an exception");
                };
                let text = ctx.describe_exception(exception);
                assert!(text.ends_with(expected), "{text}");
            }
            // A system error: its text and the path are for the log only.
            let e = io::Error::from_raw_os_error(5);
            let Throw::Exception(exception) = io_failure(ctx, "save", path, &e) else {
                panic!("an exception");
            };
            let text = ctx.describe_exception(exception);
            assert!(!text.contains("os error") && !text.contains("secret-folder"), "{text}");
        });
        let lines = lines.lock().unwrap();
        let last = lines.last().expect("a log line");
        assert!(
            last.contains("secret-folder") && last.contains("os error"),
            "{last}"
        );
    }

    #[test]
    fn asking_for_suites_that_are_not_there_keeps_nothing() {
        let dir = tempfile::tempdir().unwrap();
        with_ctx(dir.path(), |ctx| {
            // This suite's own list is kept, even when it is empty.
            scan(ctx, &Owner::Own).unwrap();
            for i in 0..1000 {
                scan(ctx, &other(&format!("vendor {i}"), "game")).unwrap();
            }
            assert_eq!(others(ctx), 0);
            assert!(rms(ctx).suites.contains_key(&Owner::Own));
            // Their stores are not there, and asking again is the same.
            let key = StoreKey {
                owner: other("vendor 1", "game"),
                name: StoreName::from_text("save").unwrap(),
            };
            assert!(!load(ctx, &key).unwrap());
            assert_eq!(others(ctx), 0);
        });
    }

    #[test]
    fn only_16_other_suites_with_stores_are_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let name = StoreName::from_text("save").unwrap();
        for i in 0..40 {
            let folder = disk::suite_dir(dir.path(), &format!("vendor {i}"), "game");
            disk::write_atomic(&disk::store_path(&folder, &name), b"stored").unwrap();
        }
        with_ctx(dir.path(), |ctx| {
            for i in 0..40 {
                let result = scan(ctx, &other(&format!("vendor {i}"), "game"));
                assert_eq!(result.is_ok(), i < MAX_OTHER_OWNERS, "vendor {i}");
            }
            assert_eq!(others(ctx), MAX_OTHER_OWNERS);
            // One that is remembered stays; one that is not is not kept.
            assert!(rms(ctx).suites.contains_key(&other("vendor 0", "game")));
            assert!(!rms(ctx).suites.contains_key(&other("vendor 39", "game")));
        });
    }

    #[test]
    fn a_suite_past_the_16_remembered_is_an_error_and_not_a_missing_store() {
        let dir = tempfile::tempdir().unwrap();
        let name = StoreName::from_text("save").unwrap();
        let file = StoreData::new(name.clone(), 0).encode();
        for i in 0..20 {
            let folder = disk::suite_dir(dir.path(), &format!("vendor {i}"), "game");
            disk::write_atomic(&disk::store_path(&folder, &name), &file).unwrap();
        }
        let key = |vendor: &str| StoreKey {
            owner: other(vendor, "game"),
            name: StoreName::from_text("save").unwrap(),
        };
        with_ctx(dir.path(), |ctx| {
            for i in 0..MAX_OTHER_OWNERS {
                assert!(load(ctx, &key(&format!("vendor {i}"))).unwrap());
            }
            // The store is there, so the game is not told it is missing.
            for i in MAX_OTHER_OWNERS..20 {
                let result = load(ctx, &key(&format!("vendor {i}")));
                assert!(result.is_err(), "vendor {i}: {result:?}");
                assert!(delete(ctx, &key(&format!("vendor {i}"))).is_err());
            }
            assert_eq!(others(ctx), MAX_OTHER_OWNERS);
            // A suite that has no store is still just missing, and the
            // ones kept still load.
            assert!(!load(ctx, &key("nobody")).unwrap());
            assert!(load(ctx, &key("vendor 0")).unwrap());
        });
    }

    #[test]
    fn a_suite_with_64_stores_makes_no_more_and_keeps_the_ones_it_has() {
        let dir = tempfile::tempdir().unwrap();
        let key = |i: usize| StoreKey {
            owner: Owner::Own,
            name: StoreName::from_text(&format!("s{i}")).unwrap(),
        };
        // 70 stores, made before there was a limit.
        let folder = disk::rms_dir(dir.path());
        for i in 0..70 {
            let file = StoreData::new(key(i).name, 0).encode();
            disk::write_atomic(&disk::store_path(&folder, &key(i).name), &file).unwrap();
        }
        with_ctx(dir.path(), |ctx| {
            scan(ctx, &Owner::Own).unwrap();
            assert!(!rms(ctx).may_add_store(&Owner::Own));
            for i in 0..70 {
                assert!(load(ctx, &key(i)).unwrap(), "store {i}");
            }
            // Deleting makes room only once the suite is under the limit.
            for i in 0..6 {
                assert!(delete(ctx, &key(i)).unwrap());
                assert!(!rms(ctx).may_add_store(&Owner::Own), "{} left", 69 - i);
            }
            assert!(delete(ctx, &key(6)).unwrap());
            assert!(rms(ctx).may_add_store(&Owner::Own));
            // The limit is each suite's own, and a suite with no store
            // is under it.
            assert!(rms(ctx).may_add_store(&other("vendor", "game")));
        });
    }
}
