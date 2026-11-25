//! The types the FastEmu runtime and the apps that show it share.
//!
//! An app (the desktop window, the Android activity, the headless runner)
//! asks a [`Launcher`] to start a MIDlet suite. Each start makes an
//! [`Instance`]: an isolated Java ME phone with its own virtual machine,
//! threads, heap, record stores and sound. The app reads the instance's
//! latest [`Frame`], sends it [`InputEvent`]s, and pauses, resumes or stops
//! it. Many instances run side by side, each on its own threads.
//!
//! Nothing in this crate depends on how the runtime works inside, so the
//! apps build and test against the `mock` module's launcher (feature `mock`) as
//! well as the real one.

pub mod device;
pub mod frame;
pub mod input;
pub mod instance;
#[cfg(feature = "mock")]
pub mod mock;
pub mod suite;
pub mod text;

pub use device::{DeviceProfile, FontSizes, KeyCodes, builtin_profiles, default_profile};
pub use frame::Frame;
pub use input::{InputEvent, Key, KeyAction, PointerAction};
pub use instance::{
    Instance, InstanceId, InstanceSettings, InstanceStatus, LaunchError, LaunchRequest, Launcher,
    LogLevel, LogLine, NetworkPolicy, Notify,
};
pub use suite::{MidletInfo, SuiteInfo};
pub use text::escape_control;
