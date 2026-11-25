//! A ZIP reader that opens the JARs real phones accepted.
//!
//! Game JARs are often damaged or written by odd tools, and a phone ran
//! them anyway. This reader reads the central directory when it is sound
//! and otherwise walks the local file headers; it keeps entry names as raw
//! bytes, ignores CRC mismatches, reads stored and deflated entries with
//! or without data descriptors, and takes the first of duplicate names.
//! See `docs/compat/notes/01-suite-loading.md`.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

const LOCAL_HEADER: u32 = 0x0403_4b50;
const CENTRAL_HEADER: u32 = 0x0201_4b50;
const END_OF_CENTRAL: u32 = 0x0605_4b50;
const DATA_DESCRIPTOR: u32 = 0x0807_4b50;

const STORED: u16 = 0;
const DEFLATED: u16 = 8;
/// General purpose flag bit 0: the entry is encrypted.
const ENCRYPTED: u16 = 1;
/// General purpose flag bit 3: sizes follow the data in a descriptor.
const HAS_DESCRIPTOR: u16 = 1 << 3;

/// The largest entry [`ZipArchive::read`] inflates, so a hostile archive
/// cannot exhaust the host. No phone had this much storage for one file.
/// Readers that know what they want use [`ZipArchive::read_limited`] with
/// a smaller limit.
pub const MAX_ENTRY_BYTES: usize = 64 * 1024 * 1024;


/// The most a deflate stream can expand: a 258-byte match can take as
/// little as two bits, so one input byte gives at most 4 * 258 bytes.
const MAX_DEFLATE_RATIO: usize = 1032;

/// Why an archive or an entry could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZipError {
    /// Neither the central directory nor a scan of the local headers found
    /// a single entry.
    NotAnArchive,
    /// The entry uses a compression method other than stored or deflate.
    UnsupportedMethod(u16),
    /// The entry is encrypted.
    Encrypted,
    /// The entry's data is cut short or does not inflate at all.
    Corrupt(String),
    /// The entry holds more than the reader was asked to read, which is
    /// the number of bytes.
    TooLarge(usize),
}

impl fmt::Display for ZipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnArchive => f.write_str("not a ZIP or JAR file"),
            Self::UnsupportedMethod(m) => write!(f, "compression method {m} is not supported"),
            Self::Encrypted => f.write_str("the entry is encrypted"),
            Self::Corrupt(why) => write!(f, "the entry is damaged: {why}"),
            Self::TooLarge(max) => write!(f, "the entry is more than {max} bytes"),
        }
    }
}

impl std::error::Error for ZipError {}

/// One file in the archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The name as stored: raw bytes, often UTF-8, sometimes GBK or
    /// CP1251.
    pub name: Vec<u8>,
    method: u16,
    flags: u16,
    /// Where the entry's data starts in the archive.
    data_start: usize,
    /// The compressed size, clamped to the bytes the archive holds.
    compressed: usize,
    /// The size the entry says it inflates to, for preallocation only.
    uncompressed: usize,
}

impl Entry {
    /// The name decoded for display: UTF-8 when it is valid, else Latin-1.
    pub fn display_name(&self) -> String {
        decode_name(&self.name)
    }
}

/// An open archive: the whole file in memory and an index of its entries.
#[derive(Clone)]
pub struct ZipArchive {
    data: Arc<Vec<u8>>,
    entries: Vec<Entry>,
    exact: HashMap<Vec<u8>, usize>,
    folded: HashMap<Vec<u8>, usize>,
}

impl fmt::Debug for ZipArchive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ZipArchive")
            .field("bytes", &self.data.len())
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl ZipArchive {
    /// Indexes the archive in `data`. A `Vec` is kept as it is: turning it
    /// into an `Arc<[u8]>` would copy it, and a big JAR would take twice
    /// its size while that happens.
    pub fn new(data: impl Into<Arc<Vec<u8>>>) -> Result<Self, ZipError> {
        let data: Arc<Vec<u8>> = data.into();
        let mut entries = read_central_directory(&data).unwrap_or_default();
        if entries.is_empty() {
            entries = scan_local_headers(&data);
        }
        if entries.is_empty() {
            return Err(ZipError::NotAnArchive);
        }
        let mut exact = HashMap::new();
        let mut folded = HashMap::new();
        for (i, entry) in entries.iter().enumerate() {
            exact.entry(entry.name.clone()).or_insert(i);
            folded.entry(entry.name.to_ascii_lowercase()).or_insert(i);
        }
        Ok(Self {
            data,
            entries,
            exact,
            folded,
        })
    }

    /// Every entry, in archive order, duplicates included.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The size of the whole archive in bytes.
    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    /// The entry named exactly `name` (the first one, if names repeat).
    pub fn find_exact(&self, name: &[u8]) -> Option<&Entry> {
        self.exact.get(name).map(|&i| &self.entries[i])
    }

    /// The entry named `name`, comparing bytes exactly first and then
    /// ignoring ASCII case, as J2ME Loader does.
    pub fn find(&self, name: &[u8]) -> Option<&Entry> {
        self.find_exact(name).or_else(|| {
            self.folded
                .get(&name.to_ascii_lowercase())
                .map(|&i| &self.entries[i])
        })
    }

    /// The entry for a Java name, trying its UTF-8 bytes and then its
    /// Latin-1 bytes (for archives whose names are in a legacy code page
    /// and were decoded byte for byte).
    pub fn find_str(&self, name: &str) -> Option<&Entry> {
        self.find(name.as_bytes()).or_else(|| {
            let latin1: Option<Vec<u8>> = name
                .chars()
                .map(|c| u8::try_from(u32::from(c)).ok())
                .collect();
            latin1.and_then(|bytes| self.find(&bytes))
        })
    }

    /// The decompressed contents of `entry`, at most [`MAX_ENTRY_BYTES`].
    /// A deflate stream that breaks part way returns what inflated before
    /// the break; the CRC is never checked.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>, ZipError> {
        self.read_limited(entry, MAX_ENTRY_BYTES)
    }

    /// The decompressed contents of `entry`, or [`ZipError::TooLarge`] when
    /// it holds more than `max` bytes. The inflate stops at `max`, so an
    /// entry that expands a thousand times costs `max` bytes of memory and
    /// time, whatever its header says it holds.
    pub fn read_limited(&self, entry: &Entry, max: usize) -> Result<Vec<u8>, ZipError> {
        match self.read_bounded(entry, max)? {
            (bytes, false) => Ok(bytes),
            (_, true) => Err(ZipError::TooLarge(max)),
        }
    }

    /// The first `max` bytes of the decompressed contents of `entry`, or all
    /// of them when there are fewer, for a reader that makes do with the
    /// start of a file.
    pub fn read_prefix(&self, entry: &Entry, max: usize) -> Result<Vec<u8>, ZipError> {
        self.read_bounded(entry, max).map(|(bytes, _)| bytes)
    }

    /// At most `max` bytes of `entry`, and whether it holds more.
    fn read_bounded(&self, entry: &Entry, max: usize) -> Result<(Vec<u8>, bool), ZipError> {
        if entry.flags & ENCRYPTED != 0 {
            return Err(ZipError::Encrypted);
        }
        let end = entry
            .data_start
            .saturating_add(entry.compressed)
            .min(self.data.len());
        let raw = &self.data[entry.data_start.min(end)..end];
        match entry.method {
            STORED => Ok((raw[..raw.len().min(max)].to_vec(), raw.len() > max)),
            DEFLATED => inflate(raw, entry.uncompressed, max),
            other => Err(ZipError::UnsupportedMethod(other)),
        }
    }
}

/// Inflates a raw deflate stream into at most `max` bytes and says whether
/// it had more to give. A stream that breaks part way keeps what inflated
/// before the break, so the rows of a damaged image still show.
fn inflate(raw: &[u8], expected: usize, max: usize) -> Result<(Vec<u8>, bool), ZipError> {
    use miniz_oxide::inflate::TINFLStatus;
    use miniz_oxide::inflate::core::{DecompressorOxide, decompress, inflate_flags};

    let flags = inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    // The size the header claims is only a hint: deflate cannot expand
    // more than about 1032 times, so a larger claim is a lie.
    let start = expected
        .min(raw.len().saturating_mul(MAX_DEFLATE_RATIO))
        .max(raw.len().saturating_mul(2))
        .max(64)
        .min(max);
    let mut out = vec![0u8; start];
    let mut decompressor = Box::<DecompressorOxide>::default();
    let mut input = raw;
    let mut written = 0;
    let mut more = false;
    loop {
        let (status, consumed, produced) =
            decompress(&mut decompressor, input, &mut out, written, flags);
        written += produced;
        input = input.get(consumed..).unwrap_or_default();
        match status {
            TINFLStatus::Done => break,
            TINFLStatus::HasMoreOutput if out.len() < max => {
                let grown = out.len().saturating_mul(2).max(64).min(max);
                out.resize(grown, 0);
            }
            TINFLStatus::HasMoreOutput => {
                more = true;
                break;
            }
            _ if written > 0 => break,
            status => return Err(ZipError::Corrupt(format!("{status:?}"))),
        }
    }
    #[cfg(test)]
    testing::count_inflated(written);
    out.truncate(written);
    out.shrink_to_fit();
    Ok((out, more))
}

/// Decodes an entry name for display.
pub fn decode_name(name: &[u8]) -> String {
    match std::str::from_utf8(name) {
        Ok(s) => s.to_owned(),
        Err(_) => name.iter().map(|&b| char::from(b)).collect(),
    }
}

fn u16_at(data: &[u8], at: usize) -> Option<u16> {
    let bytes = data.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Keeps names that stand for files: not empty, not directories.
fn is_file_name(name: &[u8]) -> bool {
    !name.is_empty() && !name.ends_with(b"/")
}

/// The offset of the end of central directory record, searched backwards
/// over the largest comment a ZIP may carry.
fn find_end_of_central(data: &[u8]) -> Option<usize> {
    if data.len() < 22 {
        return None;
    }
    let lowest = data.len().saturating_sub(22 + 0xFFFF);
    (lowest..=data.len() - 22)
        .rev()
        .find(|&at| u32_at(data, at) == Some(END_OF_CENTRAL))
}

/// Reads the entries the central directory lists, or `None` when it is
/// missing or inconsistent, so the caller falls back to the local headers.
fn read_central_directory(data: &[u8]) -> Option<Vec<Entry>> {
    let end = find_end_of_central(data)?;
    let count = usize::from(u16_at(data, end + 10)?);
    let size = u32_at(data, end + 12)? as usize;
    let mut offset = u32_at(data, end + 16)? as usize;
    // Archives with data prepended (self-extracting stubs, KDDI headers)
    // have offsets relative to the archive's own start.
    let shift = end.checked_sub(size)?.saturating_sub(offset);
    offset += shift;
    let mut entries = Vec::with_capacity(count);
    let mut at = offset;
    for _ in 0..count {
        if u32_at(data, at)? != CENTRAL_HEADER {
            return None;
        }
        let flags = u16_at(data, at + 8)?;
        let method = u16_at(data, at + 10)?;
        let compressed = u32_at(data, at + 20)? as usize;
        let uncompressed = u32_at(data, at + 24)? as usize;
        let name_len = usize::from(u16_at(data, at + 28)?);
        let extra_len = usize::from(u16_at(data, at + 30)?);
        let comment_len = usize::from(u16_at(data, at + 32)?);
        let local = u32_at(data, at + 42)? as usize + shift;
        let name = data.get(at + 46..at + 46 + name_len)?.to_vec();
        at += 46 + name_len + extra_len + comment_len;
        if !is_file_name(&name) {
            continue;
        }
        if u32_at(data, local)? != LOCAL_HEADER {
            return None;
        }
        let local_name = usize::from(u16_at(data, local + 26)?);
        let local_extra = usize::from(u16_at(data, local + 28)?);
        let data_start = local + 30 + local_name + local_extra;
        if data_start > data.len() {
            return None;
        }
        entries.push(Entry {
            name,
            method,
            flags,
            data_start,
            compressed: compressed.min(data.len() - data_start),
            uncompressed,
        });
    }
    Some(entries)
}

/// Finds every local file header by walking the file, for archives whose
/// central directory is missing or broken.
fn scan_local_headers(data: &[u8]) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut at = 0;
    while let Some(found) = find_signature(data, at, LOCAL_HEADER) {
        match read_local_entry(data, found) {
            Some((entry, next)) => {
                if is_file_name(&entry.name) {
                    entries.push(entry);
                }
                at = next.max(found + 4);
            }
            None => at = found + 4,
        }
    }
    entries
}

fn find_signature(data: &[u8], from: usize, signature: u32) -> Option<usize> {
    let needle = signature.to_le_bytes();
    data.get(from..)?
        .windows(4)
        .position(|w| w == needle)
        .map(|p| from + p)
}

/// The first local header, central directory header or end record at or
/// after `from`, found in one pass. Searching for each signature on its
/// own would scan to the end of the file for every entry whose sizes sit
/// in a data descriptor.
fn find_next_record(data: &[u8], from: usize) -> Option<usize> {
    data.get(from..)?
        .windows(4)
        .position(|w| {
            let signature = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            matches!(signature, LOCAL_HEADER | CENTRAL_HEADER | END_OF_CENTRAL)
        })
        .map(|p| from + p)
}

/// Reads the local header at `at` and returns its entry and where the next
/// record should start.
fn read_local_entry(data: &[u8], at: usize) -> Option<(Entry, usize)> {
    let flags = u16_at(data, at + 6)?;
    let method = u16_at(data, at + 8)?;
    let mut compressed = u32_at(data, at + 18)? as usize;
    let uncompressed = u32_at(data, at + 22)? as usize;
    let name_len = usize::from(u16_at(data, at + 26)?);
    let extra_len = usize::from(u16_at(data, at + 28)?);
    let name = data.get(at + 30..at + 30 + name_len)?.to_vec();
    let data_start = at + 30 + name_len + extra_len;
    if data_start > data.len() {
        return None;
    }
    let next;
    if flags & HAS_DESCRIPTOR != 0 && compressed == 0 {
        // The sizes follow the data. The data ends where the next record
        // starts, less the descriptor in front of it.
        let following = find_next_record(data, data_start).unwrap_or(data.len());
        let mut end = following;
        for len in [16, 12] {
            if let Some(start) = following.checked_sub(len)
                && start >= data_start
                && u32_at(data, start) == Some(DATA_DESCRIPTOR)
            {
                end = start;
                break;
            }
        }
        compressed = end - data_start;
        next = following;
    } else {
        compressed = compressed.min(data.len() - data_start);
        next = data_start + compressed;
    }
    Some((
        Entry {
            name,
            method,
            flags,
            data_start,
            compressed,
            uncompressed,
        },
        next,
    ))
}

#[cfg(test)]
pub(crate) mod testing {
    //! A small ZIP writer for tests: stored or deflated entries, with or
    //! without a central directory, so tests can build broken JARs.

    use std::cell::Cell;

    thread_local! {
        static INFLATED: Cell<u64> = const { Cell::new(0) };
    }

    /// Adds `bytes` to what this thread's inflates have produced.
    pub(super) fn count_inflated(bytes: usize) {
        INFLATED.with(|c| c.set(c.get() + bytes as u64));
    }

    /// The bytes every inflate on this thread has produced since the last
    /// call, and starts the count again.
    pub fn take_inflated() -> u64 {
        INFLATED.with(|c| c.replace(0))
    }

    /// One file to write.
    pub struct TestEntry<'a> {
        pub name: &'a [u8],
        pub data: &'a [u8],
        pub deflate: bool,
        /// Write sizes in a data descriptor after the data instead of the
        /// local header.
        pub descriptor: bool,
    }

    impl<'a> TestEntry<'a> {
        pub fn stored(name: &'a str, data: &'a [u8]) -> Self {
            Self {
                name: name.as_bytes(),
                data,
                deflate: false,
                descriptor: false,
            }
        }

        pub fn deflated(name: &'a str, data: &'a [u8]) -> Self {
            Self {
                deflate: true,
                ..Self::stored(name, data)
            }
        }
    }

    /// A deflated file `data` that the central directory lists once for
    /// each of `names`: every entry points at the same local header and
    /// the same compressed bytes, as in a crafted archive.
    pub fn write_overlapping(names: &[String], data: &[u8]) -> Vec<u8> {
        let one = write(&[TestEntry::deflated("x", data)], true);
        let central_at = one.len() - 22 - 46 - 1;
        let mut out = one[..central_at].to_vec();
        let template = one[central_at..central_at + 46].to_vec();
        let start = out.len() as u32;
        let mut directory = Vec::new();
        for name in names {
            let mut header = template.clone();
            header[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&header);
            directory.extend_from_slice(name.as_bytes());
        }
        out.extend_from_slice(&directory);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(names.len() as u16).to_le_bytes());
        out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
        out.extend_from_slice(&start.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out
    }

    /// Writes the archive; `central` false leaves out the central
    /// directory and its end record.
    pub fn write(entries: &[TestEntry<'_>], central: bool) -> Vec<u8> {
        let mut out = Vec::new();
        let mut directory = Vec::new();
        for e in entries {
            let crc = crc32fast::hash(e.data);
            let body = if e.deflate {
                miniz_oxide::deflate::compress_to_vec(e.data, 6)
            } else {
                e.data.to_vec()
            };
            let method: u16 = if e.deflate { 8 } else { 0 };
            let flags: u16 = if e.descriptor { 1 << 3 } else { 0 };
            let offset = out.len() as u32;
            let (c, u, k) = if e.descriptor {
                (0, 0, 0)
            } else {
                (body.len() as u32, e.data.len() as u32, crc)
            };
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&k.to_le_bytes());
            out.extend_from_slice(&c.to_le_bytes());
            out.extend_from_slice(&u.to_le_bytes());
            out.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(e.name);
            out.extend_from_slice(&body);
            if e.descriptor {
                out.extend_from_slice(&0x0807_4b50u32.to_le_bytes());
                out.extend_from_slice(&crc.to_le_bytes());
                out.extend_from_slice(&(body.len() as u32).to_le_bytes());
                out.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
            }
            directory.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&20u16.to_le_bytes());
            directory.extend_from_slice(&flags.to_le_bytes());
            directory.extend_from_slice(&method.to_le_bytes());
            directory.extend_from_slice(&[0; 4]);
            directory.extend_from_slice(&crc.to_le_bytes());
            directory.extend_from_slice(&(body.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(e.data.len() as u32).to_le_bytes());
            directory.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
            directory.extend_from_slice(&[0; 12]);
            directory.extend_from_slice(&offset.to_le_bytes());
            directory.extend_from_slice(e.name);
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
}

#[cfg(test)]
mod tests {
    use super::testing::{TestEntry, write};
    use super::*;

    fn read_all(archive: &ZipArchive) -> Vec<(String, Vec<u8>)> {
        archive
            .entries()
            .iter()
            .map(|e| (e.display_name(), archive.read(e).unwrap()))
            .collect()
    }

    #[test]
    fn reads_stored_and_deflated_entries() {
        let text = b"hello hello hello hello".repeat(20);
        let bytes = write(
            &[
                TestEntry::stored("a.txt", b"plain"),
                TestEntry::deflated("dir/b.txt", &text),
            ],
            true,
        );
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            read_all(&archive),
            vec![
                ("a.txt".to_owned(), b"plain".to_vec()),
                ("dir/b.txt".to_owned(), text)
            ]
        );
    }

    #[test]
    fn falls_back_to_local_headers_without_a_central_directory() {
        let bytes = write(
            &[
                TestEntry::stored("a.txt", b"one"),
                TestEntry::deflated("b.txt", b"two two two"),
            ],
            false,
        );
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(archive.entries().len(), 2);
        assert_eq!(
            archive.read(archive.find(b"b.txt").unwrap()).unwrap(),
            b"two two two"
        );
    }

    #[test]
    fn a_damaged_central_directory_falls_back_too() {
        let mut bytes = write(&[TestEntry::stored("a.txt", b"one")], true);
        // Point the end record's directory offset at garbage.
        let end = bytes.len() - 22;
        bytes[end + 16..end + 20].copy_from_slice(&3u32.to_le_bytes());
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            archive.read(archive.find(b"a.txt").unwrap()).unwrap(),
            b"one"
        );
    }

    #[test]
    fn data_descriptors_work_without_a_central_directory() {
        let data = b"described data, described data".to_vec();
        for deflate in [false, true] {
            let entries = [
                TestEntry {
                    name: b"x.bin",
                    data: &data,
                    deflate,
                    descriptor: true,
                },
                TestEntry::stored("y.bin", b"after"),
            ];
            let archive = ZipArchive::new(write(&entries, false)).unwrap();
            assert_eq!(archive.read(archive.find(b"x.bin").unwrap()).unwrap(), data);
            assert_eq!(
                archive.read(archive.find(b"y.bin").unwrap()).unwrap(),
                b"after"
            );
        }
    }

    /// A JAR the Java tools wrote (sizes in data descriptors) that lost its
    /// central directory is scanned in one pass, not one pass per entry.
    #[test]
    fn many_described_entries_scan_quickly() {
        let names: Vec<String> = (0..4000).map(|i| format!("r/{i}.bin")).collect();
        let data = [7u8; 256];
        let entries: Vec<TestEntry<'_>> = names
            .iter()
            .map(|name| TestEntry {
                name: name.as_bytes(),
                data: &data,
                deflate: false,
                descriptor: true,
            })
            .collect();
        let bytes = write(&entries, false);
        let start = std::time::Instant::now();
        let archive = ZipArchive::new(bytes).unwrap();
        let took = start.elapsed();
        assert_eq!(archive.entries().len(), 4000);
        assert_eq!(
            archive.read(archive.find(b"r/3999.bin").unwrap()).unwrap(),
            data
        );
        assert!(took < std::time::Duration::from_secs(2), "took {took:?}");
    }

    /// A size field that lies costs no memory beyond the real contents.
    #[test]
    fn a_huge_claimed_size_is_not_kept() {
        let text = b"small text, small text, small text".to_vec();
        let mut bytes = write(&[TestEntry::deflated("a.txt", &text)], true);
        let cd = bytes.len() - 22 - 46 - 5;
        bytes[cd + 24..cd + 28].copy_from_slice(&u32::MAX.to_le_bytes());
        let archive = ZipArchive::new(bytes).unwrap();
        let read = archive.read(&archive.entries()[0]).unwrap();
        assert_eq!(read, text);
        assert!(read.capacity() < 4096, "kept {} bytes", read.capacity());
    }

    #[test]
    fn crc_mismatches_are_ignored() {
        let mut bytes = write(&[TestEntry::stored("a.txt", b"data")], true);
        bytes[14] ^= 0xFF; // the local header's CRC
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(archive.read(&archive.entries()[0]).unwrap(), b"data");
    }

    #[test]
    fn names_match_exactly_then_without_case() {
        let bytes = write(
            &[
                TestEntry::stored("Title.PNG", b"1"),
                TestEntry::stored("title.png", b"2"),
            ],
            true,
        );
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            archive.read(archive.find(b"title.png").unwrap()).unwrap(),
            b"2"
        );
        assert_eq!(
            archive.read(archive.find(b"TITLE.png").unwrap()).unwrap(),
            b"1"
        );
        assert!(archive.find_exact(b"TITLE.png").is_none());
    }

    #[test]
    fn duplicates_resolve_to_the_first_entry() {
        let bytes = write(
            &[
                TestEntry::stored("a.txt", b"first"),
                TestEntry::stored("a.txt", b"second"),
            ],
            true,
        );
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            archive.read(archive.find(b"a.txt").unwrap()).unwrap(),
            b"first"
        );
    }

    #[test]
    fn names_stay_bytes() {
        // "Привет.txt" in CP1251.
        let name = [0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2, b'.', b't', b'x', b't'];
        let entries = [TestEntry {
            name: &name,
            data: b"hi",
            deflate: false,
            descriptor: false,
        }];
        let archive = ZipArchive::new(write(&entries, true)).unwrap();
        let latin1: String = name.iter().map(|&b| char::from(b)).collect();
        assert_eq!(archive.entries()[0].display_name(), latin1);
        assert!(archive.find_str(&latin1).is_some());
    }

    #[test]
    fn a_truncated_deflate_stream_keeps_what_inflated() {
        let text: Vec<u8> = (0..20_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut bytes = write(&[TestEntry::deflated("big.bin", &text)], false);
        bytes.truncate(bytes.len() / 2);
        let archive = ZipArchive::new(bytes).unwrap();
        let read = archive.read(&archive.entries()[0]).unwrap();
        assert!(!read.is_empty() && read.len() < text.len());
        assert_eq!(&text[..read.len()], &read[..]);
    }

    #[test]
    fn the_bytes_are_kept_where_they_were_read() {
        let bytes = write(&[TestEntry::stored("a.txt", b"data")], true);
        let at = bytes.as_ptr();
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(archive.data.as_ptr(), at, "the archive copied its bytes");
    }

    #[test]
    fn rejects_files_that_are_not_archives() {
        assert_eq!(
            ZipArchive::new(b"not a zip at all".to_vec()).unwrap_err(),
            ZipError::NotAnArchive
        );
        assert_eq!(
            ZipArchive::new(Vec::new()).unwrap_err(),
            ZipError::NotAnArchive
        );
    }

    #[test]
    fn unknown_methods_and_encryption_fail_per_entry() {
        let mut bytes = write(&[TestEntry::stored("a.txt", b"data")], true);
        bytes[8] = 14; // LZMA in the local header
        let cd = bytes.len() - 22 - 46 - 5;
        bytes[cd + 10] = 14; // and in the central directory
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            archive.read(&archive.entries()[0]),
            Err(ZipError::UnsupportedMethod(14))
        );
    }

    #[test]
    fn prepended_data_shifts_offsets() {
        let mut bytes = b"KJX header bytes".to_vec();
        bytes.extend(write(&[TestEntry::stored("a.txt", b"data")], true));
        let archive = ZipArchive::new(bytes).unwrap();
        assert_eq!(
            archive.read(archive.find(b"a.txt").unwrap()).unwrap(),
            b"data"
        );
    }

    #[test]
    fn read_limited_stops_at_the_limit_however_much_the_stream_would_give() {
        let zeros = vec![0u8; 1 << 20];
        let bytes = write(
            &[
                TestEntry::deflated("z.bin", &zeros),
                TestEntry::stored("s.bin", &[7; 100]),
            ],
            true,
        );
        let archive = ZipArchive::new(bytes).unwrap();
        let z = archive.find(b"z.bin").unwrap();
        testing::take_inflated();
        assert_eq!(archive.read_limited(z, 1000), Err(ZipError::TooLarge(1000)));
        assert_eq!(testing::take_inflated(), 1000, "nothing past the limit");
        assert_eq!(archive.read_limited(z, 0), Err(ZipError::TooLarge(0)));
        assert_eq!(
            archive.read_limited(z, (1 << 20) - 1),
            Err(ZipError::TooLarge((1 << 20) - 1))
        );
        // An exact fit is not too large.
        assert_eq!(archive.read_limited(z, 1 << 20).unwrap(), zeros);
        assert_eq!(archive.read_prefix(z, 10).unwrap(), vec![0; 10]);
        assert_eq!(archive.read(z).unwrap().len(), 1 << 20);

        let s = archive.find(b"s.bin").unwrap();
        assert_eq!(archive.read_limited(s, 99), Err(ZipError::TooLarge(99)));
        assert_eq!(archive.read_limited(s, 0), Err(ZipError::TooLarge(0)));
        assert_eq!(archive.read_limited(s, 100).unwrap(), vec![7; 100]);
        assert_eq!(archive.read_prefix(s, 50).unwrap(), vec![7; 50]);
        assert_eq!(archive.read_prefix(s, 500).unwrap(), vec![7; 100]);
    }

    #[test]
    fn overlapping_entries_read_the_same_bytes_and_each_read_stops_at_its_limit() {
        let data = vec![5u8; 100_000];
        let names: Vec<String> = (0..50).map(|i| format!("a{i}.bin")).collect();
        let archive = ZipArchive::new(testing::write_overlapping(&names, &data)).unwrap();
        assert_eq!(archive.entries().len(), 50);
        testing::take_inflated();
        for entry in archive.entries() {
            assert_eq!(archive.read(entry).unwrap(), data);
        }
        assert_eq!(testing::take_inflated(), 50 * 100_000);
        for entry in archive.entries() {
            assert_eq!(
                archive.read_limited(entry, 1000),
                Err(ZipError::TooLarge(1000))
            );
        }
        assert_eq!(testing::take_inflated(), 50 * 1000);
    }

    /// Every single-byte corruption and every truncation of a small
    /// archive opens or fails cleanly, and every entry it finds reads or
    /// fails cleanly: no panic, whatever the damage.
    #[test]
    fn damage_never_panics() {
        let text = b"compressible text, compressible text, compressible".to_vec();
        let entries = [
            TestEntry::stored("META-INF/MANIFEST.MF", b"MIDlet-Name: A\n"),
            TestEntry::deflated("a/B.class", &text),
            TestEntry {
                name: b"c.bin",
                data: &text,
                deflate: true,
                descriptor: true,
            },
        ];
        let good = write(&entries, true);
        let try_open = |bytes: Vec<u8>| {
            if let Ok(archive) = ZipArchive::new(bytes) {
                for entry in archive.entries() {
                    let _ = archive.read(entry);
                }
            }
        };
        for at in 0..good.len() {
            for flip in [0xFF, 0x80, 0x01] {
                let mut bad = good.clone();
                bad[at] ^= flip;
                try_open(bad);
            }
            try_open(good[..at].to_vec());
            try_open(good[at..].to_vec());
        }
    }
}
