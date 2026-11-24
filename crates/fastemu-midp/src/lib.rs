//! MIDP 2.0, MMAPI and the vendor APIs on the FastEmu virtual machine, and
//! the launcher that starts real instances.
//!
//! Every Java class here is implemented in Rust with
//! [`fastemu_jvm::NativeClass`] (see `docs/architecture/jvm.md` for the
//! rules native code follows). Each area lives in its own module and
//! returns its classes from a `classes()` function; [`classes`] gathers
//! them for a new virtual machine. Natives reach the instance they run in
//! through [`env::env`].

pub mod env;
pub mod io;
pub mod lcdui;
pub mod media;
pub mod midlet;
pub mod nokia;
pub mod rms;
pub mod suite;
pub mod vendor;
pub mod wma;

use fastemu_jvm::NativeClass;

/// Every Java class this crate adds to a virtual machine.
pub fn classes() -> Vec<NativeClass> {
    let mut classes = Vec::new();
    classes.extend(midlet::classes());
    classes.extend(lcdui::classes());
    classes.extend(rms::classes());
    classes.extend(media::classes());
    classes.extend(io::classes());
    classes.extend(wma::classes());
    classes.extend(nokia::classes());
    classes.extend(vendor::classes());
    classes
}
