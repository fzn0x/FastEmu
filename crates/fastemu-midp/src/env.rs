//! What the natives of one instance know about that instance.

use std::path::PathBuf;
use std::sync::Arc;

use fastemu_core::{DeviceProfile, InstanceId, InstanceSettings, SuiteInfo, default_profile};
use fastemu_jvm::Ctx;

/// The instance a virtual machine runs for: its phone, its settings, its
/// suite, its folder and its sound. The launcher stores one in the VM as an
/// extension (`Ctx::set_extension`) before any MIDP class runs, and natives
/// read it with [`env`]. It holds no Java references, so it may live in the
/// VM for its whole life.
pub struct InstanceEnv {
    pub id: InstanceId,
    pub profile: DeviceProfile,
    pub settings: InstanceSettings,
    /// The suite's attributes, for `MIDlet.getAppProperty` and the record
    /// stores' names.
    pub suite: Arc<SuiteInfo>,
    /// Where this instance keeps its record stores and files. Two instances
    /// of one game get two folders.
    pub data_dir: PathBuf,
    /// The instance's sound, when the app runs an audio engine. `None` means
    /// silence: players still work and report their state, but play nothing.
    pub audio: Option<fastemu_audio::Group>,
}

impl InstanceEnv {
    /// An environment for tests: the default profile and settings, an
    /// empty suite, the given folder and no sound.
    pub fn for_tests(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            id: InstanceId::next(),
            profile: default_profile(),
            settings: InstanceSettings::default(),
            suite: Arc::new(SuiteInfo::default()),
            data_dir: data_dir.into(),
            audio: None,
        }
    }
}

/// The environment of the instance `ctx` runs in.
///
/// # Panics
///
/// When the virtual machine has none: the launcher installs it before any
/// MIDP class runs, so only a test that forgot to install it gets here.
pub fn env<'a>(ctx: &'a Ctx<'_>) -> &'a InstanceEnv {
    try_env(ctx).expect("the launcher installs an InstanceEnv before MIDP code runs")
}

/// The environment of the instance `ctx` runs in, if it has one.
pub fn try_env<'a>(ctx: &'a Ctx<'_>) -> Option<&'a InstanceEnv> {
    ctx.extension::<InstanceEnv>()
}
