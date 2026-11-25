//! `javax.microedition.rms`: record stores, saved in the instance's
//! folder.
//!
//! Each store is one file under `<data_dir>/rms/`, written atomically on
//! every change, so a game's saves survive a crash and two instances (two
//! data folders) never share them. `docs/architecture/rms.md` describes
//! the files, the names and what is guaranteed.
//!
//! The modules [`name`], [`format`] and [`disk`] are public so tools can
//! read, back up and move saves outside a running game.

mod api;
mod budget;
pub mod disk;
mod enumeration;
pub mod format;
pub mod name;
mod state;
mod store;

use fastemu_jvm::NativeClass;

pub use budget::{WRITE_BURST, WRITE_RATE};
pub use state::{DEFAULT_QUOTA, set_quota, set_write_limit};

const RECORD_STORE: &str = "javax/microedition/rms/RecordStore";
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
    let mut classes = api::classes();
    classes.push(store::class());
    classes.push(enumeration::class());
    classes
}
