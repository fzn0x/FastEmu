# 6. Record stores (RMS)

Record ids, size limits, store naming and storage, sharing between
suites, corruption, and what games expect from `getSizeAvailable`. Link
bases are as in [note 1](01-suite-loading.md).

## Record ids

- Ids start at 1 and are never reused: the next id is the highest id
  ever handed out plus one, stored with the store so it survives
  restarts. JSR 118: "The first record created in a record store will
  have recordId equal to one (1)", and after `deleteRecord` "The
  recordId for this record is NOT reused." [spec] J2ME Loader keeps `lastRecordId` in the header file
  ([getNextRecordID](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/impl/RecordStoreImpl.java#L253));
  freej2me-plus keeps a padding entry at index 0 so ids start at 1. [code]
- Pinball 3D Timeshock failed to boot in freej2me because the next record
  id computation could go negative
  ([freej2me issue 152](https://github.com/hex007/freej2me/issues/152));
  J2ME Loader clamps a negative next id to 0. [tracker] [code]
- `getRecord` of a record with no data returns `null` in both J2ME
  Loader and freej2me-plus, as JSR 118 says ("if the record has no data,
  this method will return null" [spec]), and an unknown id throws
  `InvalidRecordIDException`
  ([JL getRecord](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/impl/RecordStoreImpl.java#L353),
  [FP getRecord](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStore.java#L410)). [code]
- Enumeration with no comparator: J2ME Loader iterates a `HashMap` of
  ids, which for small ids comes out in ascending order; freej2me-plus
  keeps insertion order, also ascending. [code] The order on real
  phones is not specified and not recorded in the sources.
  [unverified] Ascending id order is the safe default.
- freej2me-plus had an enumeration that looped forever
  ([commit d5bbd5a](https://github.com/TASEmulators/freej2me-plus/commit/d5bbd5a))
  and one that indexed by position instead of id
  ([commit 6c10634](https://github.com/TASEmulators/freej2me-plus/commit/6c10634)). [code]

## Sizes and getSizeAvailable

| | `getSizeAvailable()` | `getSize()` |
| --- | --- | --- |
| J2ME Loader | always 1 MiB (1048576) | sum of record lengths |
| freej2me-plus | 16 MiB minus the store's data | sum of record lengths |
| MicroEmulator (file storage) | always 1 MiB | |
| MicroEmulator (memory storage) | free Java heap ("returns too much") | |
| wie | 1000000 | |

Sources: [JL AndroidRecordStoreManager.getSizeAvailable](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/impl/AndroidRecordStoreManager.java#L255),
[FP getSizeAvailable](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStore.java#L465),
[ME FileRecordStoreManager](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-javase/src/main/java/org/microemu/app/util/FileRecordStoreManager.java#L322). [code]

J2ME Loader throws `RecordStoreFullException` when one record is larger
than the available size. [code] Games check `getSizeAvailable()`
before saving and skip the save when it looks too small; a value near
`Integer.MAX_VALUE` risks overflow in game arithmetic. [unverified as
a tracker report; it is why every emulator picks a modest constant]

**For FastEmu:** a per-profile RMS quota (1 MiB is a safe default)
minus the space the suite already uses, never negative; throw
`RecordStoreFullException` only when the quota is really exceeded.
Count `getSize()` as the sum of record lengths plus a small fixed
overhead per record.

## Store names and storage

- JSR 118: "Record store names are case sensitive and may consist of
  any combination of between one and 32 Unicode characters inclusive."
  [spec]
  J2ME Loader removes the characters `/ \ : * ? " < > |` and cuts names
  to 32 characters, so two names that differ only in those characters
  share a store
  ([openRecordStore](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/impl/AndroidRecordStoreManager.java#L112),
  [RecordStoreImpl](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/impl/RecordStoreImpl.java#L67)). [code]
- J2ME Loader writes one header file (`name.rsh`) and one file per record
  (`name.<id>.rsr`) in the app's data folder, on every change. [code]
- freej2me-plus writes a JSON index (`basename.rms`) and one file per
  record (`basename.<id>`) under `rms/<suite name>/`, where the base name
  is derived from the vendor and the store name. [code]
- Non-ASCII names broke saves: games with Chinese names could not save
  ([freej2me-plus issue 167](https://github.com/TASEmulators/freej2me-plus/issues/167)),
  RMS paths came out garbled ([issue 140](https://github.com/TASEmulators/freej2me-plus/issues/140)),
  and a Battleship/Connect 4 version could not create its save on
  Windows ([issue 314](https://github.com/TASEmulators/freej2me-plus/issues/314)). [tracker]

**For FastEmu:** never use a store name as a file name. Keep one file
per suite (or per store) whose index maps the exact Unicode name to its
records, inside `LaunchRequest::data_dir`. Write with a temporary file
and an atomic rename so a killed instance cannot leave half a store.

## Opening and closing

- J2ME Loader returns the same object when an open store is opened again
  and keeps an open count; `closeRecordStore` releases it when the count
  reaches 0, and closing a closed store throws
  `RecordStoreNotOpenException`. [code] This matches JSR 118: "the
  record store will not actually be closed until closeRecordStore() is
  called as many times as openRecordStore() was called." [spec]
- freej2me-plus creates a new object for every open. [code]
- `openRecordStore(name, false)` for a missing store must throw
  `RecordStoreNotFoundException`; freej2me threw the wrong thing once
  ([commit 4a338a3](https://github.com/TASEmulators/freej2me-plus/commit/4a338a3)). [code]

## Corrupted stores

J2ME Loader opens a store whose header cannot be read as a new empty
store, and replaces each unreadable record with an empty one, instead of
throwing `RecordStoreException`
([commit 9d308f83](https://github.com/nikita36078/J2ME-Loader/commit/9d308f83)). [code]
A game that hits an exception while loading its save often hangs on a
black screen at every start; J2ME Loader's issue 536 describes that
until the user cleared the data. [tracker]

**For FastEmu:** recover what can be read, log the rest, and never fail
`openRecordStore` because of a damaged file.

## Sharing between suites

`openRecordStore(name, vendor, suite)` opens another suite's store when
that store was created with `AUTHMODE_ANY`. freej2me-plus keys stores by
vendor and suite and implements it; J2ME Loader ignores the vendor and
suite and opens the caller's own store of that name. [code] Episodic
games (chapter 2 reading chapter 1's save) use it; J2ME Loader users
asked for it ([issue 306](https://github.com/nikita36078/J2ME-Loader/issues/306)). [tracker]

`fastemu-core` gives each instance its own `data_dir` so two instances of
one game keep separate saves. Sharing needs a place that both suites can
see. A shared root keyed by vendor and suite name (outside the
per-instance folder) would serve it; that is a decision for the lead.
