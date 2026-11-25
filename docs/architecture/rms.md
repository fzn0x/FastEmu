# Record stores

`javax.microedition.rms` is the only storage MIDP guarantees, and nearly
every game keeps its saves, settings and high scores in it. FastEmu
implements it in Rust in `crates/fastemu-midp/src/rms/`. This page
describes where the stores live on disk, the file format, how names
become file names, and what is guaranteed when an instance crashes or is
killed.

The API itself (every class, constant and descriptor) is in
[`docs/compat/api/midp-rms.md`](../compat/api/midp-rms.md); what games
expect from it is in [note 6](../compat/notes/06-rms.md).

## Shape

```text
  Java threads of one instance
        |  RecordStore, RecordEnumeration (natives)
        v
  Rms (a VM extension, made from the InstanceEnv on first use)
    which stores exist, per owner      the open stores and their objects
    the contents of open stores        the quota
    the file lock
        |  disk work runs with the VM lock released
        v
  <data_dir>/rms/                      this suite's stores
      save.rms
      _0053ettings.rms                 "Settings"
      save.bad                         a copy of a damaged file, if any
      suites/<vendor>/<suite>/         other suites' stores
```

| Module | What it holds |
| --- | --- |
| `name` | Store names (UTF-16, 1 to 32 code units) and the file and folder names they map to. Public. |
| `format` | `StoreData` (a store's contents) and its file format, with recovery from damaged files. Public. |
| `disk` | Folder layout, listing, atomic writes, deleting, keeping damaged copies. Public. |
| `state` | The `Rms` extension: the index of stores, the open stores, the quota, the limit of 64 stores, the write budget and the file lock. |
| `budget` | `WriteBudget`, the token bucket of disk writes, and the `Clock` it is read against. |
| `store` | The `RecordStore` class. |
| `enumeration` | `RecordEnumeration`, as the class `fastemu/rms/RecordEnumerationImpl`. |
| `api` | The interfaces (`RecordFilter`, `RecordComparator`, `RecordListener`, `RecordEnumeration`) and the five exceptions. |

`name`, `format` and `disk` are public so that tools (a save manager, an
export, tests) can read and write saves without a virtual machine.

## Where stores live

Each instance has its own `InstanceEnv::data_dir`, so two instances of
one game keep separate saves. The stores of the suite an instance runs
are one file each in `<data_dir>/rms/`. Nothing is read when the virtual
machine starts: the folder is listed on the first record store call, a
store's file is read when the store is opened, and its contents leave
memory again when it is closed for the last time.

`openRecordStore(name, vendor, suite)` names a store of another suite.
When the vendor and suite are this suite's own `MIDlet-Vendor` and
`MIDlet-Name`, it opens this suite's store. Otherwise it looks in
`<data_dir>/rms/suites/<vendor>/<suite>/`, with each folder name encoded
as below (an empty name becomes `_`; a name whose encoding passes 64
characters is cut to a 40-character prefix followed by `.` and a 64-bit
FNV-1a hash of the whole name). The store opens only when it was made
with `AUTHMODE_ANY`, and it can be changed only when it was also made
writable; otherwise the call throws `SecurityException`.

The list of another suite's folder is remembered only when the folder
holds a store, and for 16 suites at most. A folder with no store is read
again at each call, so a game that tries thousands of vendor names leaves
nothing behind in the virtual machine. A 17th suite that has stores
throws `RecordStoreException`: its stores are there, so the call does not
say they are missing.

**Stores are not shared between instances yet.** The folder of another
suite sits inside this instance's own data folder, and nothing puts
another instance's stores there, so in practice such a store is missing
(`RecordStoreNotFoundException`). Episodic games that read an earlier
chapter's save need a place both instances can see: a shared root keyed
by vendor and suite, outside the per-instance folders. That is a decision
for the launcher; the layout above is where it would plug in.

## File names

A store name is never used as a file name as it is. MIDP names are 1 to
32 UTF-16 code units, case-sensitive and may hold any character, while
Windows forbids `< > : " / \ | ? *`, control characters, trailing dots
and spaces and device names such as `con`, and Windows and macOS ignore
case by default.

The file stem keeps `a` to `z`, `0` to `9` and `-` as they are and writes
every other code unit, capital letters included, as `_` and four
lowercase hex digits:

| Store name | File |
| --- | --- |
| `save` | `save.rms` |
| `Save` | `_0053ave.rms` |
| `high scores` | `high_0020scores.rms` |
| `con` | `_0063on.rms` (a device name, so its first letter is escaped too) |
| two CJK characters, U+5B58 U+6863 | `_5b58_6863.rms` |

The stem is ASCII, at most 160 characters, the same on every platform,
and decodes back to the exact name, unpaired surrogates included. Only
the canonical stem of a name is accepted when the folder is listed, so
names and files match one to one; other files in the folder are ignored.
`listRecordStores` returns the names in UTF-16 code unit order (the
order of `String.compareTo`), or `null` when there are none. An empty
name or one of more than 32 code units throws `IllegalArgumentException`,
as MIDP says; J2ME Loader cuts long names to 32 characters instead.

## File format

All numbers are little-endian.

```text
header   "FERS"                magic
         u16  format           1
         u16  flags            bit 0: AUTHMODE_ANY, bit 1: writable by others
         i32  next id          the id the next added record gets
         i32  version
         i64  last modified    milliseconds since 1970, as currentTimeMillis
         u32  record count
         u16  name length      in UTF-16 code units, 1 to 32
         u16  name[length]     the exact store name
         u32  CRC-32           of every header byte before it
record   i32  id               records follow in ascending id order
         u32  length
         u8   data[length]
         u32  CRC-32           of the id, length and data bytes
```

A store's `getSize()` is exactly its file size: 34 bytes plus twice the
name length for the header, and 12 bytes plus the data for each record.

## Records, ids and versions

- Record ids start at 1 and each new record gets the next id stored in
  the file, so an id is never reused, even after the highest record is
  deleted and the instance restarts. `getNextRecordID` returns it.
- A record with no data (`addRecord(null, 0, 0)`) reads back as `null`
  from `getRecord`, as MIDP says.
- The version starts at 0 and grows by one on every `addRecord`,
  `setRecord` and `deleteRecord`, which also set the last-modified time.
  `setMode` changes neither.
- A store opened again while open returns the same object, and it closes
  only after as many `closeRecordStore` calls as opens. Closing it for the
  last time removes its listeners; a later open gives a new object.
- `deleteRecordStore` throws `RecordStoreException` while the store is
  open and `RecordStoreNotFoundException` when it does not exist.

## Sizes and the quota

Each suite's stores share a quota, 1 MiB by default (`rms::DEFAULT_QUOTA`,
what J2ME Loader and MicroEmulator report). `getSizeAvailable()` is the
quota minus the bytes the suite's store files take, never negative. A
change that would make the suite's stores pass the quota throws
`RecordStoreFullException` and changes nothing; a change that does not
grow a store always succeeds. A full disk also throws
`RecordStoreFullException`. The launcher may set another quota with
`rms::set_quota` (a device profile could name one).

A suite may have 64 record stores. Creating the 65th throws
`RecordStoreFullException`, the exception that games already catch for a
full store. The quota counts the bytes in the files and not the files, so
without this limit a game could make tens of thousands of empty stores,
each a file of its own, before the quota stopped it. Phones allowed a few
dozen stores at most and the games of the test corpus make one or two. A
store that exists still opens and changes when its suite has more than 64,
as an older version could make them; the suite makes a new one once it is
under the limit. Another suite's stores are never created by this
instance, so the limit is only for this suite's own.

## The write budget

The quota limits the bytes the stores hold. It does not limit the bytes
they write, and every change writes the whole store file and flushes it,
so a game that rewrites a record of 900 KiB in a loop writes about 80 MiB
a second for as long as it runs. That wears out an SSD and slows every
other program on the disk. Each instance therefore has a write budget, a
token bucket in `budget.rs`:

- It holds 32 MiB at the start and refills at 4 MiB a second. A write
  takes the size of the file it wrote from it, and at least 4 KiB, which
  is what a small file costs a file system.
- A save waits before it writes, when the writes before it have spent
  the budget. It waits for as long as the refill takes to pay the debt, in
  slices of 50 ms with the VM lock released, so the game's other threads
  keep running. After each slice it looks at the stop flag, and a stopping
  machine ends the wait with `Throw::Stop` and writes nothing. The first
  wait of an instance is logged once, as a warning. The save holds the
  file lock while it waits, so another thread of the game that opens,
  deletes or lists a store waits with it. Reading an open store does not.
- Saves stay synchronous and durable. A save is never skipped, merged
  with another or left for later: the call returns after the file is
  flushed and renamed, as it always did. Only the time it starts changes.
- The copy of a damaged file (`<stem>.bad`) and a write that fails are
  paid for like a save, since both wrote the file.
- Deleting a store and reading one are not charged. A game that makes and
  deletes a store in a loop is held by the 4 KiB it costs to make it and
  by the speed of the file system. A game that opens and closes a big
  store in a loop reads its file each time, mostly from the cache of the
  system, and costs processor time and no disk wear.

A game that writes less than 4 MiB a second on average is never slowed,
and the burst covers a first run that saves a lot at once. A game that
adds 1000 records of 100 bytes one at a time writes a file that grows to
112 KB each time, 56 MB in all, and waits about 5 seconds in the whole
run. The loop above falls from 80 MiB a second to 4 MiB a second.

Every save still rewrites the whole file, so the cost of filling a store
one record at a time grows with the square of its size. That is a known
limit. Writing only the change would end it, and is a later change. The
launcher may set another burst and rate with `rms::set_write_limit`, as
it sets the quota; the defaults are `rms::WRITE_BURST` and
`rms::WRITE_RATE`.

[SECURITY.md](../../SECURITY.md#limits-a-game-cannot-pass) lists these
limits with every other limit a game meets, and what the budget leaves
among its [known limits](../../SECURITY.md#known-limits).

## What is guaranteed

- **Every change is on disk before the call returns.** `addRecord`,
  `setRecord`, `deleteRecord`, `setMode` and creating a store write the
  whole store to `<stem>.tmp`, flush it to the disk (`fsync`), rename it
  over `<stem>.rms` (an atomic replace on every platform; on Windows the
  rename is retried for a moment when a scanner holds the old file), and
  on Unix flush the folder so the rename survives a power cut. A game that
  has spent its write budget waits before the write, as above.
- **A killed instance never leaves half a store.** Whenever the process
  dies, the store file holds the old contents or the new ones. A leftover
  `.tmp` file is an unfinished write; it is ignored and the next write
  replaces it.
- **A failed write changes nothing.** The change is made to a copy of the
  store, and the copy becomes the store only once it is on disk. The game
  gets `RecordStoreException` (or `RecordStoreFullException` for a full
  disk), and the store keeps its old contents in memory and on disk.
- **Opening never fails because of a damaged file.** A game that gets an
  exception while loading its save often hangs at every start, so a
  damaged file opens with whatever can be read:
  - a file that is not a store, has another format version or has an
    unreadable header opens as an empty store;
  - a header whose checksum fails keeps every record whose checksum holds,
    drops the others (their ids cannot be trusted), and takes the next id
    from the records;
  - under a sound header, a record whose checksum fails keeps its id with
    no data (as J2ME Loader does), so games that read record 1 still find
    it;
  - a file cut short keeps the records before the cut.

  Each problem is logged as a warning, and the damaged file is first
  copied to `<stem>.bad` so recovery never destroys the only copy, and the
  copy counts against the write budget. The next change writes a sound
  file.
- **Operations are serialized.** Natives change a store's state only with
  the VM lock held, and all file work for an instance holds one file lock
  (taken only with the VM lock released, so the two cannot deadlock).
  Readers on other threads see the old contents until the new ones are on
  disk. Disk work runs with the VM lock released (`Ctx::without_lock`),
  so the instance's other threads keep running meanwhile.

## Listeners and enumerations

`RecordListener`s run synchronously, on the thread that changed the
store, after the change is on disk, in the order they were added. A
listener added twice runs once, and an exception it throws reaches the
caller of `addRecord` (the record stays added), as in the MIDP reference
implementation.

A `RecordEnumeration` holds the matching ids in order and a cursor:

- Without a comparator the ids ascend. MIDP leaves that order open;
  J2ME Loader and freej2me-plus both ascend, and games that read records
  in a loop expect it.
- With a comparator the records are merge-sorted, which is stable (records
  the comparator calls equivalent keep ascending ids) and accepts any
  answers, even ones that are not a consistent order. Only the sign of the
  answer counts, since games return differences rather than `PRECEDES`
  and `FOLLOWS`.
- Filters and comparators get a fresh copy of each record, `null` for a
  record with no data.
- After `reset`, `nextRecordId` gives the first id and `previousRecordId`
  the last. At either end the methods throw `InvalidRecordIDException`
  and keep throwing until `reset`. `nextRecord` on a deleted record of an
  enumeration that is not kept updated throws `InvalidRecordIDException`
  and moves on.
- An enumeration kept updated listens to its store and updates itself on
  the changing thread: a new or changed record goes where the filter and
  comparator put it, a deleted one goes away, and the cursor stays on the
  same record (or between its neighbours, when the record it was on is
  deleted).
- On a closed store `hasNextElement` is `false` and `nextRecord` throws
  `RecordStoreNotOpenException`. After `destroy` every method throws
  `IllegalStateException`, except a second `destroy`, which does nothing.

Filters and comparators are game code, so every reference the natives
hold across a call is rooted, and the methods that call them (`rebuild`,
`keepUpdated`, `destroy` and the listener callbacks) are synchronized on
the enumeration, as in the reference implementation.

## Testing

- Unit tests next to the code cover names (round trips, device names,
  case, lengths), the file format (round trips, garbage, cut files,
  damaged records and headers, huge lengths), atomic writes and listing,
  the enumeration cursor and the sort. The write budget is tested on a
  `Clock` that sleeps no time and writes its sleeps down, so no test
  waits for real: the burst, the debt and its wait, the refill, a stop
  that ends the wait, a failed write and a damaged file opened again and
  again.
- `crates/fastemu-midp/tests/rms.rs` compiles the programs in
  `tests/rms_java/` with `javac --release 8` against stubs of the package
  and runs them on FastEmu with an `InstanceEnv` in a temporary folder:
  ids and exceptions, enumeration orders and keepUpdated, listeners,
  stores that survive a restart of the virtual machine, records added from
  several threads at once, other suites' stores, damaged files, the
  quota, the limit of 64 stores and a game that is made to wait for the
  write budget, with a lower bound on the time and none above it.
  Without a JDK they pass with a note, unless `FASTEMU_REQUIRE_JAVA=1`
  is set.
