//! The [`ClassSource`] over a suite's JAR: where the virtual machine finds
//! the game's classes and resources.
//!
//! Resource names follow `docs/compat/notes/01-suite-loading.md`. The
//! virtual machine resolves a name relative to the package of the class
//! `getResourceAsStream` is called on; this source then turns `\` into
//! `/`, collapses repeated slashes and `.` segments, strips leading
//! slashes, and tries an exact match, a match that ignores case, and
//! finally the same path with its leading directories dropped one at a
//! time, which rescues games that wrote names relative to the root. Misses
//! are logged once per name, for at most `max_reported` names, and a name
//! longer than [`MAX_MISSED_NAME`] characters is cut first.

use std::collections::HashSet;
use std::fmt;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::{Arc, Mutex, PoisonError};

use fastemu_core::{LogLevel, LogLine};
use fastemu_jvm::{ClassSource, ResourceRead, VmLimits};

use super::zip::{MAX_ENTRY_BYTES, ZipArchive, ZipError};

/// Receives the source's warnings: resources a game asked for that the
/// JAR does not have.
pub type SourceLog = Arc<dyn Fn(LogLine) + Send + Sync>;

/// The most characters of a missing name the source keeps and logs.
const MAX_MISSED_NAME: usize = 255;

/// The longest class file the source reads, inflated. The largest class in
/// the test folder, libraries included, has 200 KB; this is 20 times that,
/// and it is what a refused class can cost while it is parsed.
pub const MAX_CLASS_BYTES: usize = 4 * 1024 * 1024;

/// The names of the resources a game asked for and the JAR lacks.
#[derive(Default)]
struct Missed {
    names: HashSet<String>,
    /// Whether a name was left out for lack of room, which is logged once.
    full: bool,
}

/// Classes and resources from a suite's JAR.
pub struct JarSource {
    archive: Arc<ZipArchive>,
    log: Option<SourceLog>,
    limits: VmLimits,
    missed: Mutex<Missed>,
    /// Held while a resource inflates, so that the threads of one game, up
    /// to the machine's limit, do not each hold a buffer of the size the
    /// caller allows (up to the heap's) at the same time.
    inflating: Mutex<()>,
    /// What the tests see of `inflating`: how many resources inflate now,
    /// and the most that did at once.
    #[cfg(test)]
    busy: (AtomicUsize, AtomicUsize),
}

/// Counts a resource as inflating while it lives.
#[cfg(test)]
struct Busy<'a>(&'a AtomicUsize);

#[cfg(test)]
impl<'a> Busy<'a> {
    fn enter(busy: &'a (AtomicUsize, AtomicUsize)) -> Self {
        let now = busy.0.fetch_add(1, SeqCst) + 1;
        busy.1.fetch_max(now, SeqCst);
        Self(&busy.0)
    }
}

#[cfg(test)]
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, SeqCst);
    }
}

impl fmt::Debug for JarSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JarSource")
            .field("archive", &self.archive)
            .finish_non_exhaustive()
    }
}

impl JarSource {
    /// A source over `archive` that reports misses to `log`, if given, with
    /// the default limits.
    pub fn new(archive: Arc<ZipArchive>, log: Option<SourceLog>) -> Self {
        Self::with_limits(archive, log, VmLimits::default())
    }

    /// A source over `archive` that reports misses to `log`, if given, and
    /// reports at most `limits.max_reported` different ones: the limits of
    /// the virtual machine it serves.
    pub fn with_limits(archive: Arc<ZipArchive>, log: Option<SourceLog>, limits: VmLimits) -> Self {
        Self {
            archive,
            log,
            limits,
            missed: Mutex::new(Missed::default()),
            inflating: Mutex::new(()),
            #[cfg(test)]
            busy: Default::default(),
        }
    }

    /// The archive this source reads.
    pub fn archive(&self) -> &Arc<ZipArchive> {
        &self.archive
    }

    /// Logs a resource the game asked for and the JAR lacks, once for each
    /// name. The name is cut to [`MAX_MISSED_NAME`] characters, and past
    /// `max_reported` different names the source keeps none and says so
    /// once, so a game cannot make it hold names without end.
    fn report_miss(&self, path: &str) {
        self.report_once(path, |name| format!("resource not found: /{name}"));
    }

    /// Logs `text(name)` once for each `path`, under the limits of
    /// [`JarSource::report_miss`], which is the same set of names.
    fn report_once(&self, path: &str, text: impl FnOnce(&str) -> String) {
        let Some(log) = &self.log else { return };
        let name = match path.char_indices().nth(MAX_MISSED_NAME) {
            Some((at, _)) => &path[..at],
            None => path,
        };
        let text = {
            let mut missed = self.missed.lock().unwrap_or_else(PoisonError::into_inner);
            if missed.names.contains(name) {
                return;
            }
            if missed.names.len() >= self.limits.max_reported {
                if std::mem::replace(&mut missed.full, true) {
                    return;
                }
                "further missing resources are not reported: too many different ones".to_owned()
            } else {
                missed.names.insert(name.to_owned());
                text(name)
            }
        };
        log(LogLine {
            level: LogLevel::Warn,
            text,
        });
    }
}

impl ClassSource for JarSource {
    fn class_bytes(&self, name: &str) -> Option<Vec<u8>> {
        // A game must not replace the runtime's own classes.
        if name.starts_with("java/") || name.starts_with("javax/") {
            return None;
        }
        let file = format!("{name}.class");
        let entry = self.archive.find_str(&file)?;
        match self.archive.read_limited(entry, MAX_CLASS_BYTES) {
            Ok(bytes) => Some(bytes),
            Err(ZipError::TooLarge(max)) => {
                self.report_once(&file, |file| {
                    format!("{file} is more than {} MiB, so it is not loaded", max >> 20)
                });
                None
            }
            Err(_) => None,
        }
    }

    fn resource(&self, path: &str) -> Option<Vec<u8>> {
        match self.resource_within(path, MAX_ENTRY_BYTES) {
            ResourceRead::Found(bytes) => Some(bytes),
            ResourceRead::Missing | ResourceRead::TooLarge => None,
        }
    }

    fn resource_within(&self, path: &str, max: usize) -> ResourceRead {
        let path = normalize(path);
        if path.is_empty() {
            return ResourceRead::Missing;
        }
        let found = {
            // One resource inflates at a time, so the buffers of the
            // threads of a game never add up. Nothing here waits on
            // anything else, so no lock can be held the other way round.
            let _inflating = self
                .inflating
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            #[cfg(test)]
            let _busy = Busy::enter(&self.busy);
            let mut candidate = path.as_str();
            loop {
                if let Some(entry) = self.archive.find_str(candidate) {
                    match self.archive.read_limited(entry, max) {
                        Ok(bytes) => break Some(ResourceRead::Found(bytes)),
                        Err(ZipError::TooLarge(_)) => break Some(ResourceRead::TooLarge),
                        Err(_) => {}
                    }
                }
                match candidate.split_once('/') {
                    Some((_, rest)) if !rest.is_empty() => candidate = rest,
                    _ => break None,
                }
            }
        };
        found.unwrap_or_else(|| {
            self.report_miss(&path);
            ResourceRead::Missing
        })
    }
}

/// A resource path as the JAR stores it: `/` separators, no leading
/// slash, no empty or `.` segments, `..` applied.
pub fn normalize(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::super::zip::testing::{TestEntry, write};
    use super::*;

    fn source(entries: &[TestEntry<'_>]) -> (JarSource, Arc<Mutex<Vec<String>>>) {
        let archive = Arc::new(ZipArchive::new(write(entries, true)).unwrap());
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&lines);
        let log: SourceLog = Arc::new(move |line: LogLine| sink.lock().unwrap().push(line.text));
        (JarSource::new(archive, Some(log)), lines)
    }

    #[test]
    fn normalizes_names() {
        assert_eq!(normalize("//img//a.png"), "img/a.png");
        assert_eq!(normalize("\\img\\a.png"), "img/a.png");
        assert_eq!(normalize("a/./b/../c.png"), "a/c.png");
        assert_eq!(normalize("/"), "");
    }

    #[test]
    fn resources_match_exactly_then_without_case() {
        let (src, _) = source(&[
            TestEntry::stored("img/Title.png", b"T"),
            TestEntry::deflated("data/level1.bin", b"L1 L1 L1"),
        ]);
        assert_eq!(src.resource("img/Title.png").as_deref(), Some(&b"T"[..]));
        assert_eq!(src.resource("IMG/title.PNG").as_deref(), Some(&b"T"[..]));
        assert_eq!(
            src.resource("//data\\level1.bin").as_deref(),
            Some(&b"L1 L1 L1"[..])
        );
    }

    #[test]
    fn paths_fall_back_toward_the_root_and_misses_log_once() {
        let (src, lines) = source(&[TestEntry::stored("img/a.png", b"A")]);
        // A relative name the VM resolved against the class's package.
        assert_eq!(
            src.resource("com/game/img/a.png").as_deref(),
            Some(&b"A"[..])
        );
        assert_eq!(src.resource("com/game/missing.png"), None);
        assert_eq!(src.resource("com/game/missing.png"), None);
        assert_eq!(
            *lines.lock().unwrap(),
            ["resource not found: /com/game/missing.png"]
        );
    }

    #[test]
    fn a_class_file_over_4_mib_is_not_loaded_and_says_so_once() {
        let fits = vec![0u8; MAX_CLASS_BYTES];
        let over = vec![0u8; MAX_CLASS_BYTES + 1];
        let (src, lines) = source(&[
            TestEntry::deflated("a/Over.class", &over),
            TestEntry::deflated("a/Fits.class", &fits),
        ]);
        assert_eq!(src.class_bytes("a/Over"), None);
        assert_eq!(src.class_bytes("a/Over"), None);
        assert_eq!(
            src.class_bytes("a/Fits").map(|bytes| bytes.len()),
            Some(MAX_CLASS_BYTES)
        );
        assert_eq!(
            *lines.lock().unwrap(),
            ["a/Over.class is more than 4 MiB, so it is not loaded"]
        );
    }

    #[test]
    fn a_resource_is_read_up_to_the_limit_it_is_asked_for() {
        let zeros = vec![0u8; 1 << 20];
        let (src, lines) = source(&[
            TestEntry::deflated("data/z.bin", &zeros),
            TestEntry::stored("data/s.bin", b"small"),
        ]);
        super::super::zip::testing::take_inflated();
        assert_eq!(
            src.resource_within("/data/z.bin", 1000),
            ResourceRead::TooLarge
        );
        assert_eq!(super::super::zip::testing::take_inflated(), 1000);
        assert_eq!(
            src.resource_within("data/z.bin", 1 << 20),
            ResourceRead::Found(zeros)
        );
        assert_eq!(
            src.resource_within("data/s.bin", 5),
            ResourceRead::Found(b"small".to_vec())
        );
        assert_eq!(src.resource_within("data/s.bin", 4), ResourceRead::TooLarge);
        assert_eq!(
            src.resource_within("data/none", 1 << 20),
            ResourceRead::Missing
        );
        // Only the missing name is a miss.
        assert_eq!(*lines.lock().unwrap(), ["resource not found: /data/none"]);
    }

    #[test]
    fn the_threads_of_a_game_read_one_resource_at_a_time() {
        let zeros = vec![0u8; 1 << 20];
        let (src, _) = source(&[TestEntry::deflated("z.bin", &zeros)]);
        let src = Arc::new(src);
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let src = Arc::clone(&src);
                std::thread::spawn(move || {
                    for _ in 0..3 {
                        let read = src.resource_within("z.bin", 1 << 20);
                        assert!(matches!(read, ResourceRead::Found(_)));
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(src.busy.1.load(SeqCst), 1, "two resources inflated at once");
        assert_eq!(src.busy.0.load(SeqCst), 0);
    }

    #[test]
    fn a_long_missing_name_is_logged_cut() {
        let (src, lines) = source(&[TestEntry::stored("a.png", b"A")]);
        assert_eq!(src.resource(&"n".repeat(5000)), None);
        // Two names that differ after the cut are one name.
        assert_eq!(src.resource(&format!("{}x", "n".repeat(5000))), None);
        let lines = lines.lock().unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            format!("resource not found: /{}", "n".repeat(255))
        );
    }

    #[test]
    fn the_source_logs_256_different_misses_and_then_says_it_stops() {
        let (src, lines) = source(&[TestEntry::stored("a.png", b"A")]);
        let max = VmLimits::default().max_reported;
        for i in 0..4 * max {
            assert_eq!(src.resource(&format!("missing/{i}.png")), None);
        }
        // A name already logged is no news, whether the set is full or not.
        assert_eq!(src.resource("missing/0.png"), None);
        let lines = lines.lock().unwrap();
        assert_eq!(lines.len(), max + 1);
        assert!(
            lines[..max]
                .iter()
                .all(|l| l.starts_with("resource not found: /missing/"))
        );
        assert!(lines[max].starts_with("further missing resources are not reported"));
        // The ones it kept still read: the cap is on the record of misses.
        drop(lines);
        assert_eq!(src.resource("a.png").as_deref(), Some(&b"A"[..]));
    }

    #[test]
    fn the_limits_of_a_source_set_how_many_misses_it_reports() {
        let archive =
            Arc::new(ZipArchive::new(write(&[TestEntry::stored("a.png", b"A")], true)).unwrap());
        let lines = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&lines);
        let log: SourceLog = Arc::new(move |line: LogLine| sink.lock().unwrap().push(line.text));
        let limits = VmLimits {
            max_reported: 2,
            ..VmLimits::default()
        };
        let src = JarSource::with_limits(archive, Some(log), limits);
        for name in ["a", "b", "c", "d"] {
            assert_eq!(src.resource(name), None);
        }
        assert_eq!(lines.lock().unwrap().len(), 3);
    }

    #[test]
    fn classes_load_by_name_but_never_replace_the_runtime() {
        let (src, _) = source(&[
            TestEntry::stored("a/Game.class", b"G"),
            TestEntry::stored("B.CLASS", b"B"),
            TestEntry::stored("java/lang/String.class", b"S"),
        ]);
        assert_eq!(src.class_bytes("a/Game").as_deref(), Some(&b"G"[..]));
        assert_eq!(src.class_bytes("B").as_deref(), Some(&b"B"[..]));
        assert_eq!(src.class_bytes("java/lang/String"), None);
        assert_eq!(src.class_bytes("a/Missing"), None);
    }
}
