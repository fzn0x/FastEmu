//! `javax.microedition.rms`: record stores, saved in the instance's
//! folder.

mod api;
pub mod disk;
pub mod format;
pub mod name;

use fastemu_jvm::NativeClass;

const RECORD_ENUMERATION: &str = "javax/microedition/rms/RecordEnumeration";
const RECORD_COMPARATOR: &str = "javax/microedition/rms/RecordComparator";
const RECORD_FILTER: &str = "javax/microedition/rms/RecordFilter";
const RECORD_LISTENER: &str = "javax/microedition/rms/RecordListener";
const RECORD_STORE_EXCEPTION: &str = "javax/microedition/rms/RecordStoreException";
const INVALID_RECORD_ID_EXCEPTION: &str = "javax/microedition/rms/InvalidRecordIDException";
const RECORD_STORE_FULL_EXCEPTION: &str = "javax/microedition/rms/RecordStoreFullException";
const RECORD_STORE_NOT_FOUND_EXCEPTION: &str =
    "javax/microedition/rms/RecordStoreNotFoundException";
const RECORD_STORE_NOT_OPEN_EXCEPTION: &str = "javax/microedition/rms/RecordStoreNotOpenException";

/// The classes of this module.
pub fn classes() -> Vec<NativeClass> {
    api::classes()
}
