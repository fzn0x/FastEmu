//! MIDP 2.0, MMAPI and the vendor APIs on the FastEmu virtual machine, and
//! the launcher that starts real instances.
//!
//! [`MidpLauncher`] opens a suite ([`suite`]), makes one virtual machine
//! per instance with these classes, and returns a [`MidpInstance`] that
//! runs the MIDlet's lifecycle on its own threads, its UI callbacks on an
//! event thread, and presents frames to the app.
//!
//! Every Java class here is implemented in Rust with
//! [`fastemu_jvm::NativeClass`] (see `docs/architecture/jvm.md` for the
//! rules native code follows). Each area lives in its own module and
//! returns its classes from a `classes()` function; [`classes`] gathers
//! them for a new virtual machine, with the vendor APIs of the device
//! profile's maker only ([`vendor::Vendor`]). Natives reach the instance
//! they run in through [`env::env`]. `docs/architecture/midp.md` describes
//! the design, `docs/architecture/vendors.md` the vendor APIs.

pub mod env;
pub mod host;
pub mod instance;
pub mod io;
pub mod launcher;
pub mod lcdui;
pub mod media;
pub mod midlet;
pub mod nokia;
pub mod rms;
mod stream_bytes;
pub mod suite;
pub mod vendor;
pub mod wma;

use fastemu_core::DeviceProfile;
use fastemu_jvm::NativeClass;

pub use instance::{MidpInstance, Ping};
pub use launcher::MidpLauncher;

/// Every Java class this crate adds to the virtual machine of a game
/// running on `profile`: MIDP and the optional APIs on every profile, and
/// the vendor APIs only on profiles of their maker, so a game probing for
/// them with `Class.forName` finds the phone the profile imitates.
pub fn classes(profile: &DeviceProfile) -> Vec<NativeClass> {
    let vendor = vendor::Vendor::of(profile);
    let mut classes = Vec::new();
    classes.extend(midlet::classes());
    classes.extend(lcdui::classes());
    classes.extend(rms::classes());
    classes.extend(media::classes());
    classes.extend(io::classes());
    classes.extend(wma::classes());
    if vendor.has_nokia_ui() {
        // Nokia's DirectGraphics is MIDP's Graphics with more methods.
        classes = classes
            .into_iter()
            .map(|class| {
                if class.name() == nokia::direct_graphics::GRAPHICS {
                    nokia::direct_graphics::implement(class)
                } else {
                    class
                }
            })
            .collect();
        classes.extend(nokia::classes());
    }
    classes.extend(vendor::classes(vendor));
    classes
}
