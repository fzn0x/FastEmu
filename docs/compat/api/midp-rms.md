# javax.microedition.rms (MIDP 2.0)

`javax.microedition.rms`, the Record Management System: the only
persistent storage MIDP guarantees. Nearly every game keeps its saved
progress, settings and high scores here, so this package must keep data
exactly across runs.

Rules from the `RecordStore` class description:

- Record stores belong to a MIDlet suite, which is identified by its
  `MIDlet-Vendor` and `MIDlet-Name`. Names are case-sensitive, 1 to 32
  characters, unique within the suite. FastEmu keeps each suite's stores
  apart, and each instance of the same suite shares them (the spec has no
  notion of two copies of a suite running at once, so FastEmu must
  serialize their writes).
- Record IDs start at 1 and each new record gets the previous ID plus
  one, so IDs of deleted records are never reused. Games often save in
  record 1 and read it back with `getRecord(1)`, so the numbering must be
  exact.
- Every operation is atomic, synchronous and serialized; `addRecord` and
  `setRecord` reach persistent storage before they return.
- `openRecordStore` called again for an open store returns the same
  object; the store closes only after as many `closeRecordStore` calls as
  opens.
- The version starts at a value the implementation picks and grows on
  every add, set and delete; the last-modified time uses
  `System.currentTimeMillis()` format.

Column meanings and tier rules: [README](README.md).

## Sources

- [MIDP 2.0 javadoc (JSR-118)](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/index.html), hosted by Oracle: the spec, and the source of every member, throws entry and since tag.
- MicroEmulator 2.0.4 API stub jar [midpapi20](https://repo1.maven.org/maven2/org/microemu/midpapi20/2.0.4/midpapi20-2.0.4.jar) (Maven Central): compiled class files, read for descriptors, flags and `ConstantValue` attributes.
- [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/tree/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java) at commit `e270ed91e67b5cafc18c1991b92ba5249cf41c50` (Apache-2.0): implementation source, checked for constant values and member lists.
- [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/tree/aab3350832c0d82aff1771394d28bff6f66b0aa4/src) at commit `aab3350832c0d82aff1771394d28bff6f66b0aa4` (GPL-3.0): implementation source, checked for constant values and member lists.
- [MicroEmulator](https://github.com/barteo/microemu/tree/e47b656d983deaa67810c776662444483bdbf134/microemulator) at commit `e47b656d983deaa67810c776662444483bdbf134` (LGPL-2.1 or Apache-2.0): implementation source, checked for constant values and member lists.

## Classes

| Class | Kind | Tier | Since | Members |
|---|---|---|---|---|
| [InvalidRecordIDException](#invalidrecordidexception) | class | 2 | MIDP 1.0 | 2 |
| [RecordComparator](#recordcomparator) | interface | 3 | MIDP 1.0 | 4 |
| [RecordEnumeration](#recordenumeration) | interface | 2 | MIDP 1.0 | 12 |
| [RecordFilter](#recordfilter) | interface | 3 | MIDP 1.0 | 1 |
| [RecordListener](#recordlistener) | interface | 3 | MIDP 1.0 | 3 |
| [RecordStore](#recordstore) | class | 1 | MIDP 1.0 | 25 |
| [RecordStoreException](#recordstoreexception) | class | 1 | MIDP 1.0 | 2 |
| [RecordStoreFullException](#recordstorefullexception) | class | 2 | MIDP 1.0 | 2 |
| [RecordStoreNotFoundException](#recordstorenotfoundexception) | class | 1 | MIDP 1.0 | 2 |
| [RecordStoreNotOpenException](#recordstorenotopenexception) | class | 2 | MIDP 1.0 | 2 |

## javax.microedition.rms

### InvalidRecordIDException

`public class javax.microedition.rms.InvalidRecordIDException extends javax.microedition.rms.RecordStoreException`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/InvalidRecordIDException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/InvalidRecordIDException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/InvalidRecordIDException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/InvalidRecordIDException.java).

Thrown for a record ID that does not exist or was deleted.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |

### RecordComparator

`public interface javax.microedition.rms.RecordComparator`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordComparator.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordComparator.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordComparator.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordComparator.java).

Orders records for `enumerateRecords`.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `EQUIVALENT` | `I` | `0` | static | 1.0 |  |
| 3 | `FOLLOWS` | `I` | `1` | static | 1.0 |  |
| 3 | `PRECEDES` | `I` | `-1` | static | 1.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `int compare(byte[] rec1, byte[] rec2)` | `compare([B[B)I` |  | 1.0 |  |  |

### RecordEnumeration

`public interface javax.microedition.rms.RecordEnumeration`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordEnumeration.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordEnumeration.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordEnumeration.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordEnumeration.java).

Games walk all records with `hasNextElement` and `nextRecordId` or `nextRecord`. Without a comparator the order is undefined; J2ME Loader and freej2me-plus both return ascending record IDs, and FastEmu should too.

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `int numRecords()` | `numRecords()I` |  | 1.0 |  |  |
| 2 | `byte[] nextRecord()` | `nextRecord()[B` |  | 1.0 | `InvalidRecordIDException` when no more records are available. Subsequent calls to this method will continue to throw this exception until reset() has been called to reset the enumeration; `RecordStoreNotOpenException` if the record store is not open; `RecordStoreException` if a general record store exception occurs |  |
| 2 | `int nextRecordId()` | `nextRecordId()I` |  | 1.0 | `InvalidRecordIDException` when no more records are available. Subsequent calls to this method will continue to throw this exception until reset() has been called to reset the enumeration |  |
| 3 | `byte[] previousRecord()` | `previousRecord()[B` |  | 1.0 | `InvalidRecordIDException` when no more records are available. Subsequent calls to this method will continue to throw this exception until reset() has been called to reset the enumeration; `RecordStoreNotOpenException` if the record store is not open; `RecordStoreException` if a general record store exception occurs |  |
| 3 | `int previousRecordId()` | `previousRecordId()I` |  | 1.0 | `InvalidRecordIDException` when no more records are available. Subsequent calls to this method will continue to throw this exception until reset() has been called to reset the enumeration |  |
| 2 | `boolean hasNextElement()` | `hasNextElement()Z` |  | 1.0 |  |  |
| 3 | `boolean hasPreviousElement()` | `hasPreviousElement()Z` |  | 1.0 |  |  |
| 3 | `void reset()` | `reset()V` |  | 1.0 |  |  |
| 3 | `void rebuild()` | `rebuild()V` |  | 1.0 |  |  |
| 3 | `void keepUpdated(boolean keepUpdated)` | `keepUpdated(Z)V` |  | 1.0 |  |  |
| 3 | `boolean isKeptUpdated()` | `isKeptUpdated()Z` |  | 1.0 |  |  |
| 2 | `void destroy()` | `destroy()V` |  | 1.0 |  |  |

### RecordFilter

`public interface javax.microedition.rms.RecordFilter`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordFilter.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordFilter.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordFilter.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordFilter.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `boolean matches(byte[] candidate)` | `matches([B)Z` |  | 1.0 |  |  |

### RecordListener

`public interface javax.microedition.rms.RecordListener`

Tier 3. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordListener.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordListener.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordListener.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordListener.java).

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 3 | `void recordAdded(RecordStore recordStore, int recordId)` | `recordAdded(Ljavax/microedition/rms/RecordStore;I)V` |  | 1.0 |  |  |
| 3 | `void recordChanged(RecordStore recordStore, int recordId)` | `recordChanged(Ljavax/microedition/rms/RecordStore;I)V` |  | 1.0 |  |  |
| 3 | `void recordDeleted(RecordStore recordStore, int recordId)` | `recordDeleted(Ljavax/microedition/rms/RecordStore;I)V` |  | 1.0 |  |  |

### RecordStore

`public class javax.microedition.rms.RecordStore extends java.lang.Object`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStore.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordStore.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStore.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordStore.java).

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 3 | `AUTHMODE_PRIVATE` | `I` | `0` | static | 1.0 | MIDP 2.0: only the owning suite may open the store. |
| 3 | `AUTHMODE_ANY` | `I` | `1` | static | 1.0 | MIDP 2.0: any suite may open the store by vendor and suite name. |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `void deleteRecordStore(String recordStoreName)` | `deleteRecordStore(Ljava/lang/String;)V` | static | 1.0 | `RecordStoreException` if a record store-related exception occurred; `RecordStoreNotFoundException` if the record store could not be found | Throws `RecordStoreException` while the store is open, and `RecordStoreNotFoundException` when it is missing. |
| 1 | `RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary)` | `openRecordStore(Ljava/lang/String;Z)Ljavax/microedition/rms/RecordStore;` | static | 1.0 | `RecordStoreException` if a record store-related exception occurred; `RecordStoreNotFoundException` if the record store could not be found; `RecordStoreFullException` if the operation cannot be completed because the record store is full; `IllegalArgumentException` if recordStoreName is invalid | Throws `RecordStoreNotFoundException` when the store is missing and `createIfNecessary` is false; games use that to detect a first run. |
| 3 | `RecordStore openRecordStore(String recordStoreName, boolean createIfNecessary, int authmode, boolean writable)` | `openRecordStore(Ljava/lang/String;ZIZ)Ljavax/microedition/rms/RecordStore;` | static | 2.0 | `RecordStoreException` if a record store-related exception occurred; `RecordStoreNotFoundException` if the record store could not be found; `RecordStoreFullException` if the operation cannot be completed because the record store is full; `IllegalArgumentException` if authmode or recordStoreName is invalid |  |
| 3 | `RecordStore openRecordStore(String recordStoreName, String vendorName, String suiteName)` | `openRecordStore(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljavax/microedition/rms/RecordStore;` | static | 2.0 | `RecordStoreException` if a record store-related exception occurred; `RecordStoreNotFoundException` if the record store could not be found; `SecurityException` if this MIDlet Suite is not allowed to open the specified RecordStore; `IllegalArgumentException` if recordStoreName is invalid | Opens another suite's shared store by name, vendor and suite name. |
| 3 | `void setMode(int authmode, boolean writable)` | `setMode(IZ)V` |  | 2.0 | `RecordStoreException` if a record store-related exception occurred; `SecurityException` if this MIDlet Suite is not allowed to change the mode of the RecordStore; `IllegalArgumentException` if authmode is invalid |  |
| 1 | `void closeRecordStore()` | `closeRecordStore()V` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `RecordStoreException` if a different record store-related exception occurred |  |
| 2 | `String[] listRecordStores()` | `listRecordStores()[Ljava/lang/String;` | static | 1.0 |  | The names of this suite's stores, or `null` when it has none (not an empty array). |
| 3 | `String getName()` | `getName()Ljava/lang/String;` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |
| 3 | `int getVersion()` | `getVersion()I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |
| 1 | `int getNumRecords()` | `getNumRecords()I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |
| 3 | `int getSize()` | `getSize()I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |
| 2 | `int getSizeAvailable()` | `getSizeAvailable()I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open | Some games refuse to save when this looks too small; report a generous number. |
| 3 | `long getLastModified()` | `getLastModified()J` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |
| 3 | `void addRecordListener(RecordListener listener)` | `addRecordListener(Ljavax/microedition/rms/RecordListener;)V` |  | 1.0 |  |  |
| 3 | `void removeRecordListener(RecordListener listener)` | `removeRecordListener(Ljavax/microedition/rms/RecordListener;)V` |  | 1.0 |  |  |
| 2 | `int getNextRecordID()` | `getNextRecordID()I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `RecordStoreException` if a different record store-related exception occurred |  |
| 1 | `int addRecord(byte[] data, int offset, int numBytes)` | `addRecord([BII)I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `RecordStoreException` if a different record store-related exception occurred; `RecordStoreFullException` if the operation cannot be completed because the record store has no more room; `SecurityException` if the MIDlet has read-only access to the RecordStore | `data` may be null when `numBytes` is 0. |
| 2 | `void deleteRecord(int recordId)` | `deleteRecord(I)V` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `InvalidRecordIDException` if the recordId is invalid; `RecordStoreException` if a general record store exception occurs; `SecurityException` if the MIDlet has read-only access to the RecordStore | Throws `InvalidRecordIDException` for an unknown ID. |
| 2 | `int getRecordSize(int recordId)` | `getRecordSize(I)I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `InvalidRecordIDException` if the recordId is invalid; `RecordStoreException` if a general record store exception occurs |  |
| 2 | `int getRecord(int recordId, byte[] buffer, int offset)` | `getRecord(I[BI)I` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `InvalidRecordIDException` if the recordId is invalid; `RecordStoreException` if a general record store exception occurs; `ArrayIndexOutOfBoundsException` if the record is larger than the buffer supplied | Copies into the caller's buffer and returns the length. Throws `ArrayIndexOutOfBoundsException` when the buffer is too small. |
| 1 | `byte[] getRecord(int recordId)` | `getRecord(I)[B` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `InvalidRecordIDException` if the recordId is invalid; `RecordStoreException` if a general record store exception occurs | Returns a copy, or `null` for a record with no data. |
| 1 | `void setRecord(int recordId, byte[] newData, int offset, int numBytes)` | `setRecord(I[BII)V` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open; `InvalidRecordIDException` if the recordId is invalid; `RecordStoreException` if a general record store exception occurs; `RecordStoreFullException` if the operation cannot be completed because the record store has no more room; `SecurityException` if the MIDlet has read-only access to the RecordStore | Replaces the whole record; the new data may be longer or shorter. |
| 1 | `RecordEnumeration enumerateRecords(RecordFilter filter, RecordComparator comparator, boolean keepUpdated)` | `enumerateRecords(Ljavax/microedition/rms/RecordFilter;Ljavax/microedition/rms/RecordComparator;Z)Ljavax/microedition/rms/RecordEnumeration;` |  | 1.0 | `RecordStoreNotOpenException` if the record store is not open |  |

### RecordStoreException

`public class javax.microedition.rms.RecordStoreException extends java.lang.Exception`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStoreException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordStoreException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStoreException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordStoreException.java).

Base of the other record store exceptions; declared by most methods.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

### RecordStoreFullException

`public class javax.microedition.rms.RecordStoreFullException extends javax.microedition.rms.RecordStoreException`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStoreFullException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordStoreFullException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStoreFullException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordStoreFullException.java).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |

### RecordStoreNotFoundException

`public class javax.microedition.rms.RecordStoreNotFoundException extends javax.microedition.rms.RecordStoreException`

Tier 1. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStoreNotFoundException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordStoreNotFoundException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStoreNotFoundException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordStoreNotFoundException.java).

Games catch it on first start, then create the store.

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 1 | `()` | `<init>()V` |  | 1.0 |  |  |

### RecordStoreNotOpenException

`public class javax.microedition.rms.RecordStoreNotOpenException extends javax.microedition.rms.RecordStoreException`

Tier 2. Since MIDP 1.0. Spec: [midp20](https://docs.oracle.com/javame/config/cldc/ref-impl/midp2.0/jsr118/javax/microedition/rms/RecordStoreNotOpenException.html). Implementations: [J2ME Loader](https://github.com/nikita36078/J2ME-Loader/blob/e270ed91e67b5cafc18c1991b92ba5249cf41c50/app/src/main/java/javax/microedition/rms/RecordStoreNotOpenException.java), [freej2me-plus](https://github.com/TASEmulators/freej2me-plus/blob/aab3350832c0d82aff1771394d28bff6f66b0aa4/src/javax/microedition/rms/RecordStoreNotOpenException.java), [MicroEmulator](https://github.com/barteo/microemu/blob/e47b656d983deaa67810c776662444483bdbf134/microemulator/microemu-midp/src/main/java/javax/microedition/rms/RecordStoreNotOpenException.java).

| T | Constructor | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 2 | `(String message)` | `<init>(Ljava/lang/String;)V` |  | 1.0 |  |  |
| 2 | `()` | `<init>()V` |  | 1.0 |  |  |

## Cross-check

Every member above comes from the spec javadoc. The script that writes this file compared it with the stub and implementation sources listed under Sources: constant values, member lists and `static` flags. Member lists are compared only where the parsed source holds the implementation's whole class hierarchy. What differs is listed here; a class not listed matched everywhere it was compared.

No differences.
