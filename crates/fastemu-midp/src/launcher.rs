//! [`MidpLauncher`]: the real [`Launcher`], which opens suites and starts
//! each MIDlet in its own virtual machine.

use std::path::Path;
use std::sync::Arc;

use fastemu_audio::Engine;
use fastemu_core::{
    DeviceProfile, Instance, InstanceId, InstanceSettings, LaunchError, LaunchRequest, Launcher,
    LogLevel, LogLine, SuiteInfo,
};
use fastemu_jvm::{Vm, VmConfig};

use crate::env::InstanceEnv;
use crate::host::HostLink;
use crate::instance::MidpInstance;
use crate::lcdui;
use crate::suite::{JarSource, Suite};

/// A fake IMEI for the vendor keys games read for licence checks; never
/// anything from the host.
const FAKE_IMEI: &str = "000000000000000";

/// Starts MIDlets. Each launch gets its own VM, threads, heap and sound
/// group, so instances share nothing.
#[derive(Clone, Default)]
pub struct MidpLauncher {
    audio: Option<Engine>,
}

impl std::fmt::Debug for MidpLauncher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidpLauncher")
            .field("audio", &self.audio.is_some())
            .finish()
    }
}

impl MidpLauncher {
    /// A launcher whose instances play through `audio`: the apps pass an
    /// engine with an output; the headless runner and tests pass none (the
    /// games are silent but their players still work) or an offline one.
    pub fn new(audio: Option<Engine>) -> Self {
        Self { audio }
    }

    /// Starts `request` and returns the instance with its concrete type,
    /// for tools that want more than the [`Instance`] trait offers.
    pub fn launch_midp(&self, request: LaunchRequest) -> Result<Arc<MidpInstance>, LaunchError> {
        let suite = Suite::open(&request.suite)?;
        let midlet = suite
            .info
            .midlets
            .get(request.midlet)
            .cloned()
            .ok_or(LaunchError::NoSuchMidlet(request.midlet))?;
        let id = InstanceId::next();
        let host = HostLink::new(id, request.notify.clone());
        for note in &suite.notes {
            host.log(LogLine {
                level: LogLevel::Warn,
                text: note.clone(),
            });
        }
        let source = Arc::new(JarSource::new(
            Arc::clone(&suite.jar),
            Some(host.log_sink()),
        ));
        let group = self.audio.as_ref().map(|engine| {
            let group = engine.create_group();
            group.set_volume(request.settings.volume.clamp(0.0, 1.0));
            group
        });
        let config = VmConfig {
            heap_bytes: usize::try_from(request.profile.heap_bytes).unwrap_or(usize::MAX),
            properties: system_properties(&request.profile, &request.settings),
            log: host.log_sink(),
            diagnostics: None,
            allow_exit: false,
            ..VmConfig::default()
        };
        let vm = Vm::new(config, source.clone(), crate::classes());
        let env = InstanceEnv {
            id,
            profile: request.profile,
            settings: request.settings,
            suite: Arc::new(suite.info),
            data_dir: request.data_dir,
            audio: group.clone(),
            source,
            host: host.clone(),
        };
        vm.attach("launcher", move |ctx| ctx.set_extension(env));
        vm.spawn("event", lcdui::event::run)?;
        let instance = MidpInstance::start(vm, host, group, midlet.name, midlet.class_name)?;
        Ok(instance)
    }
}

impl Launcher for MidpLauncher {
    fn inspect(&self, suite: &Path) -> Result<SuiteInfo, LaunchError> {
        Ok(Suite::open(suite)?.info)
    }

    fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
        Ok(self.launch_midp(request)?)
    }
}

/// The system properties of an instance, after
/// `docs/compat/api/system-properties.md` and
/// `docs/compat/notes/08-system-properties.md`: the phone's identity from
/// the profile, the encoding and locale from the settings, only what
/// FastEmu implements, a fake IMEI under the keys of the profile's vendor,
/// and the settings' own properties last so they win. Nothing comes from
/// the host.
pub fn system_properties(
    profile: &DeviceProfile,
    settings: &InstanceSettings,
) -> Vec<(String, String)> {
    let platform = profile.platform.as_str();
    let configuration = if platform.contains("CLDC-1.0") {
        "CLDC-1.0"
    } else {
        "CLDC-1.1"
    };
    let profiles = if platform.contains("MIDP-2.1") {
        "MIDP-2.1"
    } else {
        "MIDP-2.0"
    };
    let mut properties: Vec<(&str, &str)> = vec![
        ("microedition.platform", platform),
        ("microedition.configuration", configuration),
        ("microedition.profiles", profiles),
        ("microedition.encoding", &settings.encoding),
        ("microedition.locale", &settings.locale),
        ("supports.mixing", "true"),
        ("supports.audio.capture", "false"),
        ("supports.video.capture", "false"),
        ("supports.recording", "false"),
    ];
    let vendor = platform.to_ascii_lowercase();
    if vendor.starts_with("siemens") {
        properties.push(("com.siemens.IMEI", FAKE_IMEI));
        properties.push(("com.siemens.OSVersion", "11"));
    } else if vendor.starts_with("sonyericsson") {
        properties.push(("com.sonyericsson.imei", "IMEI 00460101-501594-5-00"));
    }
    let mut properties: Vec<(String, String)> = properties
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();
    properties.extend(settings.properties.iter().cloned());
    properties
}

#[cfg(test)]
mod tests {
    use fastemu_core::{builtin_profiles, default_profile};

    use super::*;

    fn lookup<'a>(props: &'a [(String, String)], key: &str) -> Option<&'a str> {
        props
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn properties_come_from_the_profile_and_settings() {
        let settings = InstanceSettings {
            encoding: "windows-1251".into(),
            properties: vec![("microedition.locale".into(), "ru-RU".into())],
            ..InstanceSettings::default()
        };
        let props = system_properties(&default_profile(), &settings);
        assert_eq!(
            lookup(&props, "microedition.platform"),
            Some(default_profile().platform.as_str())
        );
        assert_eq!(
            lookup(&props, "microedition.encoding"),
            Some("windows-1251")
        );
        assert_eq!(lookup(&props, "microedition.locale"), Some("ru-RU"));
        assert_eq!(lookup(&props, "microedition.profiles"), Some("MIDP-2.0"));
        assert_eq!(lookup(&props, "os.name"), None);
        assert_eq!(lookup(&props, "com.siemens.IMEI"), None);
    }

    #[test]
    fn vendor_keys_follow_the_profile() {
        let profiles = builtin_profiles();
        let siemens = profiles
            .iter()
            .find(|p| p.id.starts_with("siemens"))
            .unwrap();
        let props = system_properties(siemens, &InstanceSettings::default());
        assert_eq!(lookup(&props, "com.siemens.IMEI"), Some(FAKE_IMEI));
        let s60 = profiles
            .iter()
            .find(|p| p.id == "nokia-s60-176x208")
            .unwrap();
        let props = system_properties(s60, &InstanceSettings::default());
        assert_eq!(
            lookup(&props, "microedition.configuration"),
            Some("CLDC-1.0")
        );
    }
}
