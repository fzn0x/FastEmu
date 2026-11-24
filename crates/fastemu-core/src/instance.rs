use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::device::{DeviceProfile, default_profile};
use crate::frame::Frame;
use crate::input::InputEvent;
use crate::suite::SuiteInfo;

/// Identifies one running instance for the life of the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InstanceId(pub u64);

impl InstanceId {
    /// A fresh identifier, never handed out before in this process.
    pub fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstanceStatus {
    /// Loading classes and running the MIDlet's constructor.
    Starting,
    Running,
    /// Paused by the app; the MIDlet got `pauseApp`.
    Paused,
    /// The MIDlet called `notifyDestroyed`, or the app stopped it.
    Exited,
    /// The game hit an error the runtime could not recover from.
    Crashed {
        message: String,
    },
}

impl InstanceStatus {
    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Exited | Self::Crashed { .. })
    }
}

/// Whether a game may open network connections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NetworkPolicy {
    /// Connections fail as if the phone had no signal.
    #[default]
    Deny,
    Allow,
}

/// Per-instance settings an app picks before launching.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstanceSettings {
    /// The `microedition.encoding` system property, the charset of
    /// `new String(bytes)` and every other conversion that names none.
    /// ISO-8859-1 by default, as on Nokia's phones and in J2ME Loader;
    /// games from some regions expect their phones' encoding, such as
    /// `UTF-8` or `windows-1251`.
    pub encoding: String,
    /// The `microedition.locale` system property.
    pub locale: String,
    /// The starting volume, 0.0 (silent) to 1.0.
    pub volume: f32,
    pub network: NetworkPolicy,
    /// An upper bound on presented frames per second, if any.
    pub max_fps: Option<u32>,
    /// Extra system properties, and application properties that override
    /// the suite's own (a user's JAD edits).
    pub properties: Vec<(String, String)>,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        Self {
            encoding: "ISO-8859-1".to_owned(),
            locale: "en-US".to_owned(),
            volume: 1.0,
            network: NetworkPolicy::Deny,
            max_fps: None,
            properties: Vec::new(),
        }
    }
}

/// Called from the instance's threads when a new frame is ready or its
/// status changes, so an app can schedule a redraw. It must return quickly.
pub type Notify = Arc<dyn Fn(InstanceId) + Send + Sync>;

/// Everything a [`Launcher`] needs to start one instance.
#[derive(Clone)]
pub struct LaunchRequest {
    /// The suite's `.jar`, or a `.jad` next to (or pointing at) its JAR.
    pub suite: PathBuf,
    /// Which `MIDlet-<n>` to start, from zero.
    pub midlet: usize,
    pub profile: DeviceProfile,
    pub settings: InstanceSettings,
    /// Where this instance keeps its record stores and files. Two
    /// instances of one game get two folders, so their saves stay apart.
    pub data_dir: PathBuf,
    pub notify: Option<Notify>,
}

impl LaunchRequest {
    /// A request with the default profile and settings.
    pub fn new(suite: impl Into<PathBuf>, data_dir: impl Into<PathBuf>) -> Self {
        Self {
            suite: suite.into(),
            midlet: 0,
            profile: default_profile(),
            settings: InstanceSettings::default(),
            data_dir: data_dir.into(),
            notify: None,
        }
    }
}

impl fmt::Debug for LaunchRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LaunchRequest")
            .field("suite", &self.suite)
            .field("midlet", &self.midlet)
            .field("profile", &self.profile.id)
            .field("settings", &self.settings)
            .field("data_dir", &self.data_dir)
            .field("notify", &self.notify.is_some())
            .finish()
    }
}

#[derive(Debug)]
pub enum LaunchError {
    Io(std::io::Error),
    /// The file is not a MIDlet suite, or its manifest or descriptor is
    /// broken beyond what the runtime tolerates.
    InvalidSuite(String),
    NoSuchMidlet(usize),
    /// The suite needs something FastEmu does not provide.
    Unsupported(String),
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "could not read the suite: {err}"),
            Self::InvalidSuite(why) => write!(f, "not a usable MIDlet suite: {why}"),
            Self::NoSuchMidlet(n) => write!(f, "the suite has no MIDlet number {}", n + 1),
            Self::Unsupported(what) => write!(f, "unsupported: {what}"),
        }
    }
}

impl std::error::Error for LaunchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for LaunchError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Starts instances. The runtime provides the real one; apps take any.
pub trait Launcher: Send + Sync {
    /// Reads a suite's descriptor and manifest without starting it.
    fn inspect(&self, suite: &Path) -> Result<SuiteInfo, LaunchError>;

    /// Starts one MIDlet in a new instance, isolated from every other.
    fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// The game wrote to `System.out`.
    Out,
    /// The game wrote to `System.err`, or an exception went uncaught.
    Err,
    /// Something the runtime noticed, such as an API the game called that
    /// FastEmu does not provide yet.
    Warn,
    Info,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogLine {
    pub level: LogLevel,
    pub text: String,
}

/// One running MIDlet: an isolated phone with its own virtual machine.
///
/// Every method returns quickly and may be called from any thread.
pub trait Instance: Send + Sync {
    fn id(&self) -> InstanceId;

    /// The MIDlet's name, for title bars.
    fn title(&self) -> String;

    fn status(&self) -> InstanceStatus;

    /// The latest presented picture, or `None` before the first.
    fn frame(&self) -> Option<Frame>;

    fn send(&self, event: InputEvent);

    /// Freezes the game's threads and calls `pauseApp`.
    fn pause(&self);

    /// Calls `startApp` again and lets the threads run.
    fn resume(&self);

    /// Calls `destroyApp(true)`, then ends every thread of the instance.
    fn stop(&self);

    /// 0.0 mutes the instance; 1.0 is full volume.
    fn set_volume(&self, volume: f32);

    /// Takes the log lines written since the last call.
    fn drain_log(&self) -> Vec<LogLine>;

    /// Whether the game shows a text box or a focused text field right now,
    /// so an app should send typed characters as [`InputEvent::Text`] (and
    /// show the on-screen keyboard on a phone) instead of mapping keys to
    /// the keypad. Instances without text fields keep the default, `false`.
    fn wants_text(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let a = InstanceId::next();
        let b = InstanceId::next();
        assert_ne!(a, b);
        assert!(b > a);
    }

    #[test]
    fn errors_read_well() {
        assert_eq!(
            LaunchError::NoSuchMidlet(1).to_string(),
            "the suite has no MIDlet number 2"
        );
        assert!(
            InstanceStatus::Crashed {
                message: "x".into()
            }
            .is_finished()
        );
        assert!(!InstanceStatus::Running.is_finished());
    }
}
