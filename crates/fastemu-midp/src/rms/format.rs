//! The contents of one record store and the file format it is saved in.
//!
//! A store file is a header and the records in ascending id order, all
//! numbers little-endian:
//!
//! ```text
//! header   "FERS"                magic
//!          u16  format           1
//!          u16  flags            bit 0: AUTHMODE_ANY, bit 1: writable by others
//!          i32  next id          the id the next added record gets
//!          i32  version
//!          i64  last modified    milliseconds since 1970, as currentTimeMillis
//!          u32  record count
//!          u16  name length      in UTF-16 code units, 1 to 32
//!          u16  name[length]     the exact store name
//!          u32  CRC-32           of every header byte before it
//! record   i32  id
//!          u32  length
//!          u8   data[length]
//!          u32  CRC-32           of the id, length and data bytes
//! ```
//!
//! Reading never fails. A damaged file gives back every record whose
//! checksum holds; a record whose checksum fails keeps its id with no data,
//! and a header that cannot be read gives an empty store. Each problem is
//! reported so the caller can log it.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::sync::Arc;

use super::name::{MAX_NAME_UNITS, StoreName};

const MAGIC: &[u8; 4] = b"FERS";
const FORMAT: u16 = 1;
const FLAG_SHARED: u16 = 1;
const FLAG_WRITABLE: u16 = 2;
/// Header bytes before the name.
const FIXED_HEADER: usize = 4 + 2 + 2 + 4 + 4 + 8 + 4 + 2;
/// Bytes a record takes beyond its data.
const RECORD_OVERHEAD: usize = 4 + 4 + 4;

/// Everything a record store holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreData {
    pub name: StoreName,
    /// `AUTHMODE_ANY`: other suites may open the store.
    pub shared: bool,
    /// Whether other suites that may open the store may also change it.
    pub writable: bool,
    /// The id the next added record gets. Ids are never reused, so this
    /// only grows.
    pub next_id: i32,
    /// Grows on every add, set and delete.
    pub version: i32,
    /// When the store last changed, in milliseconds since 1970.
    pub last_modified: i64,
    /// The records by id. An empty record is one with no data.
    pub records: BTreeMap<i32, Arc<[u8]>>,
}

/// A store read from a file, and what was wrong with the file.
#[derive(Debug)]
pub struct Loaded {
    pub data: StoreData,
    /// One line per problem; empty for a sound file.
    pub problems: Vec<String>,
}

impl StoreData {
    /// A new, empty, private store created at `now`.
    pub fn new(name: StoreName, now: i64) -> Self {
        Self {
            name,
            shared: false,
            writable: false,
            next_id: 1,
            version: 0,
            last_modified: now,
            records: BTreeMap::new(),
        }
    }

    /// The bytes the store takes on disk, which is what `getSize` reports.
    pub fn size(&self) -> u64 {
        let header = FIXED_HEADER + 2 * self.name.units().len() + 4;
        let records: usize = self
            .records
            .values()
            .map(|data| RECORD_OVERHEAD + data.len())
            .sum();
        (header + records) as u64
    }

    /// Adds a record and returns its id, or `None` when every id is used.
    pub fn add(&mut self, data: Vec<u8>, now: i64) -> Option<i32> {
        let id = self.next_id;
        self.next_id = id.checked_add(1)?;
        self.records.insert(id, data.into());
        self.touch(now);
        Some(id)
    }

    /// Replaces record `id`'s data; `false` when there is no such record.
    pub fn set(&mut self, id: i32, data: Vec<u8>, now: i64) -> bool {
        match self.records.get_mut(&id) {
            Some(slot) => {
                *slot = data.into();
                self.touch(now);
                true
            }
            None => false,
        }
    }

    /// Deletes record `id`; `false` when there is no such record.
    pub fn delete(&mut self, id: i32, now: i64) -> bool {
        let found = self.records.remove(&id).is_some();
        if found {
            self.touch(now);
        }
        found
    }

    fn touch(&mut self, now: i64) {
        self.version = self.version.wrapping_add(1);
        self.last_modified = now;
    }

    /// The file contents for this store.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.size() as usize);
        let mut flags = 0;
        if self.shared {
            flags |= FLAG_SHARED;
        }
        if self.writable {
            flags |= FLAG_WRITABLE;
        }
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&self.next_id.to_le_bytes());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.last_modified.to_le_bytes());
        out.extend_from_slice(&(self.records.len() as u32).to_le_bytes());
        let name = self.name.units();
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        for unit in name {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        let crc = crc32fast::hash(&out);
        out.extend_from_slice(&crc.to_le_bytes());
        for (id, data) in &self.records {
            let start = out.len();
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(data);
            let crc = crc32fast::hash(&out[start..]);
            out.extend_from_slice(&crc.to_le_bytes());
        }
        out
    }

    /// Reads a store file saved as `name`, recovering what it can. `now`
    /// stands in for the modification time when the header is unreadable.
    pub fn decode(bytes: &[u8], name: &StoreName, now: i64) -> Loaded {
        let mut problems = Vec::new();
        let mut reader = Reader { bytes, pos: 0 };
        let header = match Header::read(&mut reader) {
            Ok(header) => header,
            Err(why) => {
                problems.push(format!("{why}; the store opens empty"));
                return Loaded {
                    data: Self::new(name.clone(), now),
                    problems,
                };
            }
        };
        if !header.sound {
            problems.push(
                "the header failed its checksum; the next id, version and mode are rebuilt"
                    .to_owned(),
            );
        } else if header.name != name.units() {
            problems.push(format!(
                "the file names the store {:?}",
                String::from_utf16_lossy(&header.name)
            ));
        }
        let records = read_records(&mut reader, &header, &mut problems);
        let after_last = records
            .keys()
            .next_back()
            .map_or(1, |&id| id.saturating_add(1));
        let data = if header.sound {
            Self {
                name: name.clone(),
                shared: header.flags & FLAG_SHARED != 0,
                writable: header.flags & FLAG_WRITABLE != 0,
                next_id: header.next_id.max(after_last).max(1),
                version: header.version,
                last_modified: header.last_modified,
                records,
            }
        } else {
            Self {
                next_id: after_last,
                records,
                ..Self::new(name.clone(), now)
            }
        };
        Loaded { data, problems }
    }
}

/// A header as read, and whether its checksum held.
struct Header {
    flags: u16,
    next_id: i32,
    version: i32,
    last_modified: i64,
    count: u32,
    name: Vec<u16>,
    sound: bool,
}

impl Header {
    fn read(r: &mut Reader<'_>) -> Result<Self, String> {
        const SHORT: &str = "the file is too short to be a record store";
        if r.take(4).ok_or(SHORT)? != MAGIC {
            return Err("the file is not a FastEmu record store".to_owned());
        }
        let format = r.u16().ok_or(SHORT)?;
        if format != FORMAT {
            return Err(format!(
                "the file has format {format}, which this version cannot read"
            ));
        }
        let flags = r.u16().ok_or(SHORT)?;
        let next_id = r.i32().ok_or(SHORT)?;
        let version = r.i32().ok_or(SHORT)?;
        let last_modified = r.i64().ok_or(SHORT)?;
        let count = r.u32().ok_or(SHORT)?;
        let len = usize::from(r.u16().ok_or(SHORT)?);
        if len == 0 || len > MAX_NAME_UNITS {
            return Err("the header is damaged".to_owned());
        }
        let mut name = Vec::with_capacity(len);
        for _ in 0..len {
            name.push(r.u16().ok_or(SHORT)?);
        }
        let end = r.pos;
        let crc = r.u32().ok_or(SHORT)?;
        Ok(Self {
            flags,
            next_id,
            version,
            last_modified,
            count,
            name,
            sound: crc == crc32fast::hash(&r.bytes[..end]),
        })
    }
}

/// Reads records until the count the header gives (or the end of the file
/// when the header is not sound), noting every problem.
fn read_records(
    r: &mut Reader<'_>,
    header: &Header,
    problems: &mut Vec<String>,
) -> BTreeMap<i32, Arc<[u8]>> {
    let mut records = BTreeMap::new();
    let mut seen: u64 = 0;
    loop {
        if header.sound && seen == u64::from(header.count) {
            if r.remaining() > 0 {
                problems.push(format!(
                    "{} bytes after the last record are ignored",
                    r.remaining()
                ));
            }
            break;
        }
        if r.remaining() == 0 {
            if header.sound {
                problems.push(format!(
                    "the file ends after {seen} of its {} records",
                    header.count
                ));
            }
            break;
        }
        let start = r.pos;
        let Some((id, data, sound)) = read_record(r) else {
            problems.push(format!(
                "the record at byte {start} is cut short; the {} bytes from there are lost",
                r.bytes.len() - start
            ));
            break;
        };
        seen += 1;
        let plausible = id > 0 && (!header.sound || id < header.next_id);
        let Entry::Vacant(slot) = records.entry(id) else {
            problems.push(format!("record {id} appears twice; the first copy is kept"));
            continue;
        };
        if sound && id > 0 {
            slot.insert(Arc::from(data));
        } else if plausible {
            problems.push(format!(
                "record {id} failed its checksum and is kept with no data"
            ));
            slot.insert(Arc::from(&[][..]));
        } else {
            problems.push(format!("a damaged record at byte {start} is dropped"));
        }
    }
    records
}

/// One record: its id, its data and whether its checksum held. `None` when
/// the file ends inside it.
fn read_record<'a>(r: &mut Reader<'a>) -> Option<(i32, &'a [u8], bool)> {
    let start = r.pos;
    let id = r.i32()?;
    let len = r.u32()? as usize;
    if len > r.remaining().saturating_sub(4) {
        return None;
    }
    let data = r.take(len)?;
    let end = r.pos;
    let crc = r.u32()?;
    Some((id, data, crc == crc32fast::hash(&r.bytes[start..end])))
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let slice = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(slice)
    }

    fn array<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }

    fn u16(&mut self) -> Option<u16> {
        self.array().map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.array().map(u32::from_le_bytes)
    }

    fn i32(&mut self) -> Option<i32> {
        self.array().map(i32::from_le_bytes)
    }

    fn i64(&mut self) -> Option<i64> {
        self.array().map(i64::from_le_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000_000;

    fn name(text: &str) -> StoreName {
        StoreName::from_text(text).unwrap()
    }

    fn sample() -> StoreData {
        let mut data = StoreData::new(name("Save \u{5b58}"), NOW);
        data.add(b"first".to_vec(), NOW + 1).unwrap();
        data.add(Vec::new(), NOW + 2).unwrap();
        data.add(vec![0xff; 300], NOW + 3).unwrap();
        data.delete(2, NOW + 4);
        data.shared = true;
        data.writable = true;
        data
    }

    #[test]
    fn a_store_round_trips() {
        let data = sample();
        let bytes = data.encode();
        assert_eq!(bytes.len() as u64, data.size());
        let loaded = StoreData::decode(&bytes, &data.name, 0);
        assert!(loaded.problems.is_empty(), "{:?}", loaded.problems);
        assert_eq!(loaded.data, data);
        assert_eq!(loaded.data.next_id, 4);
        assert_eq!(loaded.data.version, 4);
        assert_eq!(loaded.data.last_modified, NOW + 4);
    }

    #[test]
    fn an_empty_store_round_trips() {
        let data = StoreData::new(name("x"), NOW);
        let loaded = StoreData::decode(&data.encode(), &data.name, 0);
        assert!(loaded.problems.is_empty());
        assert_eq!(loaded.data, data);
    }

    #[test]
    fn ids_are_never_reused() {
        let mut data = StoreData::new(name("x"), NOW);
        assert_eq!(data.add(vec![1], NOW), Some(1));
        assert_eq!(data.add(vec![2], NOW), Some(2));
        assert!(data.delete(2, NOW));
        assert_eq!(data.add(vec![3], NOW), Some(3));
        assert!(!data.delete(2, NOW));
        assert!(!data.set(2, vec![], NOW));
        data.next_id = i32::MAX;
        assert_eq!(data.add(vec![4], NOW), None);
    }

    #[test]
    fn garbage_opens_as_an_empty_store() {
        for bytes in [&b""[..], b"FER", b"PK\x03\x04 a zip file", b"FERS\x02\x00"] {
            let loaded = StoreData::decode(bytes, &name("x"), NOW);
            assert_eq!(loaded.data, StoreData::new(name("x"), NOW));
            assert_eq!(loaded.problems.len(), 1, "{bytes:?}");
        }
    }

    #[test]
    fn a_cut_file_keeps_the_records_before_the_cut() {
        let data = sample();
        let bytes = data.encode();
        let cut = &bytes[..bytes.len() - 10];
        let loaded = StoreData::decode(cut, &data.name, 0);
        assert_eq!(loaded.data.records.len(), 1);
        assert_eq!(&*loaded.data.records[&1], b"first");
        // The header still knows record 3 existed, so its id stays used.
        assert_eq!(loaded.data.next_id, 4);
        assert_eq!(loaded.problems.len(), 1, "{:?}", loaded.problems);
    }

    #[test]
    fn a_damaged_record_keeps_its_id_with_no_data() {
        let data = sample();
        let mut bytes = data.encode();
        // The last byte of record 3's data.
        let at = bytes.len() - 5;
        bytes[at] ^= 0x55;
        let loaded = StoreData::decode(&bytes, &data.name, 0);
        assert_eq!(&*loaded.data.records[&1], b"first");
        assert!(loaded.data.records[&3].is_empty());
        assert_eq!(loaded.problems.len(), 1);
    }

    #[test]
    fn a_damaged_header_keeps_the_records() {
        let data = sample();
        let mut bytes = data.encode();
        // A byte of the version field.
        bytes[12] ^= 0x01;
        let loaded = StoreData::decode(&bytes, &data.name, NOW + 9);
        assert_eq!(loaded.data.records, data.records);
        assert_eq!(loaded.data.next_id, 4);
        assert_eq!(loaded.data.version, 0);
        assert_eq!(loaded.data.last_modified, NOW + 9);
        assert!(!loaded.data.shared);
        assert_eq!(loaded.problems.len(), 1);
    }

    #[test]
    fn trailing_bytes_are_reported_and_ignored() {
        let data = sample();
        let mut bytes = data.encode();
        bytes.extend_from_slice(b"junk");
        let loaded = StoreData::decode(&bytes, &data.name, 0);
        assert_eq!(loaded.data, data);
        assert_eq!(loaded.problems.len(), 1);
    }

    #[test]
    fn the_file_name_wins_over_the_stored_name() {
        let data = sample();
        let loaded = StoreData::decode(&data.encode(), &name("other"), 0);
        assert_eq!(loaded.data.name, name("other"));
        assert_eq!(loaded.data.records, data.records);
        assert_eq!(loaded.problems.len(), 1);
    }

    #[test]
    fn a_huge_length_does_not_allocate() {
        let mut bytes = StoreData::new(name("x"), NOW).encode();
        // Claim one record, then give it a length of 4 GiB.
        bytes[24] = 1;
        let crc_at = bytes.len() - 4;
        let crc = crc32fast::hash(&bytes[..crc_at]);
        bytes[crc_at..].copy_from_slice(&crc.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        let loaded = StoreData::decode(&bytes, &name("x"), 0);
        assert!(loaded.data.records.is_empty());
        assert_eq!(loaded.problems.len(), 1);
    }
}
