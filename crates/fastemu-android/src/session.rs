//! The game the Android app runs: which instance, where its saves go, and
//! the keys held on it.
//!
//! The app runs one game at a time until it shows fastemu-ui's views, which
//! run many. Nothing here touches GPUI or Android, so the host tests cover it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use fastemu_core::{
    InputEvent, Instance, Key, LaunchError, LaunchRequest, Launcher, Notify, PointerAction,
};
use futures::StreamExt as _;
use futures::channel::mpsc;

use crate::keypad::HeldKeys;

/// The running game, if any, and how to start the next.
pub struct Session {
    launcher: Arc<dyn Launcher>,
    data_root: PathBuf,
    notify: Notify,
    running: Option<Running>,
}

struct Running {
    instance: Arc<dyn Instance>,
    held: HeldKeys,
}

impl Session {
    /// A session with no game yet. Each game keeps its record stores in a
    /// folder of `data_root` named after its file; `notify` is told of every
    /// new frame and status change.
    pub fn new(launcher: Arc<dyn Launcher>, data_root: impl Into<PathBuf>, notify: Notify) -> Self {
        Self {
            launcher,
            data_root: data_root.into(),
            notify,
            running: None,
        }
    }

    /// Starts the first MIDlet of `suite` (a `.jar` or `.jad`) and stops the
    /// game that ran before. When the launch fails, that game keeps running.
    pub fn open(&mut self, suite: &Path) -> Result<(), LaunchError> {
        let data_dir = self.data_dir(suite);
        std::fs::create_dir_all(&data_dir)?;
        let mut request = LaunchRequest::new(suite, data_dir);
        request.notify = Some(Arc::clone(&self.notify));
        let instance = self.launcher.launch(request)?;
        log::info!(
            "session: started {} from {}",
            instance.title(),
            suite.display()
        );
        if let Some(previous) = self.running.replace(Running {
            instance,
            held: HeldKeys::default(),
        }) {
            previous.instance.stop();
        }
        Ok(())
    }

    /// Where the game in `suite` keeps its saves: the same folder each time
    /// it starts, so they survive a restart.
    pub fn data_dir(&self, suite: &Path) -> PathBuf {
        let name = suite
            .file_stem()
            .map_or_else(|| "game".into(), |stem| stem.to_string_lossy().into_owned());
        self.data_root.join(name)
    }

    pub fn instance(&self) -> Option<&Arc<dyn Instance>> {
        self.running.as_ref().map(|running| &running.instance)
    }

    /// Presses `key` on the game, once per stroke.
    pub fn press(&mut self, key: Key) {
        if let Some(running) = &mut self.running
            && let Some(event) = running.held.press(key)
        {
            running.instance.send(event);
        }
    }

    pub fn release(&mut self, key: Key) {
        if let Some(running) = &mut self.running
            && let Some(event) = running.held.release(key)
        {
            running.instance.send(event);
        }
    }

    pub fn is_held(&self, key: Key) -> bool {
        self.running
            .as_ref()
            .is_some_and(|running| running.held.is_held(key))
    }

    /// A touch or click at phone pixel (`x`, `y`).
    pub fn point(&self, action: PointerAction, (x, y): (i32, i32)) {
        if let Some(running) = &self.running {
            running.instance.send(InputEvent::Pointer { action, x, y });
        }
    }

    /// Pauses the game when the app leaves the screen. Held keys are
    /// released first: their releases would never arrive.
    pub fn pause(&mut self) {
        if let Some(running) = &mut self.running {
            for event in running.held.release_all() {
                running.instance.send(event);
            }
            running.instance.pause();
        }
    }

    pub fn resume(&self) {
        if let Some(running) = &self.running {
            running.instance.resume();
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(running) = &self.running {
            running.instance.stop();
        }
    }
}

/// The path of a `file://` URL, as the Activity sends for a game another app
/// opened with FastEmu. Other schemes are not files this app can read.
pub fn file_url_path(url: &str) -> Option<PathBuf> {
    let encoded = url.strip_prefix("file://")?;
    let bytes = encoded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = encoded.get(i + 1..i + 3)?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded)
        .ok()
        .filter(|path| path.starts_with('/'))
        .map(PathBuf::from)
}

/// A [`Notify`] for instances and the receiver the UI waits on, which
/// coalesces: however many frames arrive while the UI is busy, it wakes
/// once, so a game drawing faster than the screen can't flood it.
pub fn redraw_signal() -> (Notify, RedrawSignal) {
    let pending = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::unbounded();
    let notify: Notify = {
        let pending = Arc::clone(&pending);
        Arc::new(move |_| {
            if !pending.swap(true, Ordering::AcqRel) {
                // The receiver is gone only when the app is shutting down.
                sender.unbounded_send(()).ok();
            }
        })
    };
    (notify, RedrawSignal { pending, receiver })
}

/// The receiving end of [`redraw_signal`].
pub struct RedrawSignal {
    pending: Arc<AtomicBool>,
    receiver: mpsc::UnboundedReceiver<()>,
}

impl RedrawSignal {
    /// Waits until an instance has something new to show. `false` once every
    /// notifier is gone.
    pub async fn changed(&mut self) -> bool {
        let woke = self.receiver.next().await.is_some();
        // Cleared before the UI draws, so a frame that lands while it draws
        // wakes it again.
        self.pending.store(false, Ordering::Release);
        woke
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use fastemu_core::{
        Frame, InstanceId, InstanceStatus, KeyAction, LogLine, MidletInfo, SuiteInfo,
    };
    use futures::FutureExt as _;

    use super::*;

    /// An instance that records what it was sent.
    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<InputEvent>>,
        status: Mutex<Vec<&'static str>>,
    }

    impl Instance for Recorder {
        fn id(&self) -> InstanceId {
            InstanceId(0)
        }
        fn title(&self) -> String {
            "recorder".into()
        }
        fn status(&self) -> InstanceStatus {
            InstanceStatus::Running
        }
        fn frame(&self) -> Option<Frame> {
            None
        }
        fn send(&self, event: InputEvent) {
            self.events.lock().unwrap().push(event);
        }
        fn pause(&self) {
            self.status.lock().unwrap().push("pause");
        }
        fn resume(&self) {
            self.status.lock().unwrap().push("resume");
        }
        fn stop(&self) {
            self.status.lock().unwrap().push("stop");
        }
        fn set_volume(&self, _volume: f32) {}
        fn drain_log(&self) -> Vec<LogLine> {
            Vec::new()
        }
    }

    /// Hands out recorders, or fails for suites named `broken`.
    #[derive(Default)]
    struct RecordingLauncher {
        launched: Mutex<Vec<(Arc<Recorder>, LaunchRequest)>>,
    }

    impl Launcher for RecordingLauncher {
        fn inspect(&self, _suite: &Path) -> Result<SuiteInfo, LaunchError> {
            Ok(SuiteInfo {
                midlets: vec![MidletInfo::default()],
                ..SuiteInfo::default()
            })
        }

        fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
            if request
                .suite
                .file_stem()
                .is_some_and(|stem| stem == "broken")
            {
                return Err(LaunchError::InvalidSuite("no manifest".into()));
            }
            let recorder = Arc::new(Recorder::default());
            self.launched
                .lock()
                .unwrap()
                .push((Arc::clone(&recorder), request));
            Ok(recorder)
        }
    }

    impl RecordingLauncher {
        fn recorder(&self, index: usize) -> Arc<Recorder> {
            Arc::clone(&self.launched.lock().unwrap()[index].0)
        }
    }

    fn session(launcher: &Arc<RecordingLauncher>, root: &Path) -> Session {
        let launcher: Arc<dyn Launcher> = launcher.clone();
        Session::new(launcher, root, Arc::new(|_| {}))
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fastemu-android-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    fn key(key: Key, action: KeyAction) -> InputEvent {
        InputEvent::Key { key, action }
    }

    #[test]
    fn opening_a_game_makes_its_save_folder_and_replaces_the_last_game() {
        let root = scratch("open");
        let launcher = Arc::new(RecordingLauncher::default());
        let mut session = session(&launcher, &root);
        session.open(Path::new("/games/snake.jar")).unwrap();
        assert!(root.join("snake").is_dir());
        {
            let launched = launcher.launched.lock().unwrap();
            assert_eq!(launched[0].1.data_dir, root.join("snake"));
            assert!(launched[0].1.notify.is_some());
        }
        session.open(Path::new("/games/tetris.jad")).unwrap();
        assert_eq!(*launcher.recorder(0).status.lock().unwrap(), ["stop"]);
        assert_eq!(session.instance().unwrap().title(), "recorder");
        drop(session);
        assert_eq!(*launcher.recorder(1).status.lock().unwrap(), ["stop"]);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_failed_launch_keeps_the_running_game() {
        let root = scratch("failed");
        let launcher = Arc::new(RecordingLauncher::default());
        let mut session = session(&launcher, &root);
        session.open(Path::new("/games/snake.jar")).unwrap();
        let error = session.open(Path::new("/games/broken.jar")).unwrap_err();
        assert!(matches!(error, LaunchError::InvalidSuite(_)));
        assert!(launcher.recorder(0).status.lock().unwrap().is_empty());
        assert!(session.instance().is_some());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn keys_reach_the_game_once_per_stroke() {
        let root = scratch("keys");
        let launcher = Arc::new(RecordingLauncher::default());
        let mut session = session(&launcher, &root);
        // Before any game, keys go nowhere.
        session.press(Key::Fire);
        assert!(!session.is_held(Key::Fire));
        session.open(Path::new("/games/snake.jar")).unwrap();
        session.press(Key::Fire);
        session.press(Key::Fire);
        assert!(session.is_held(Key::Fire));
        session.release(Key::Fire);
        session.point(PointerAction::Press, (3, 4));
        assert_eq!(
            *launcher.recorder(0).events.lock().unwrap(),
            [
                key(Key::Fire, KeyAction::Press),
                key(Key::Fire, KeyAction::Release),
                InputEvent::Pointer {
                    action: PointerAction::Press,
                    x: 3,
                    y: 4
                },
            ]
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn pausing_lets_go_of_held_keys_first() {
        let root = scratch("pause");
        let launcher = Arc::new(RecordingLauncher::default());
        let mut session = session(&launcher, &root);
        session.open(Path::new("/games/snake.jar")).unwrap();
        session.press(Key::Up);
        session.pause();
        session.resume();
        let recorder = launcher.recorder(0);
        assert_eq!(
            *recorder.events.lock().unwrap(),
            [
                key(Key::Up, KeyAction::Press),
                key(Key::Up, KeyAction::Release)
            ]
        );
        assert_eq!(*recorder.status.lock().unwrap(), ["pause", "resume"]);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn file_urls_decode_to_paths() {
        assert_eq!(
            file_url_path("file:///data/user/0/io.github.fzn0x.fastemu/files/games/My%20Game.jar"),
            Some(PathBuf::from(
                "/data/user/0/io.github.fzn0x.fastemu/files/games/My Game.jar"
            ))
        );
        assert_eq!(
            file_url_path("file:///sdcard/%E3%82%B2%E3%83%BC%E3%83%A0.jad"),
            Some(PathBuf::from("/sdcard/\u{30b2}\u{30fc}\u{30e0}.jad"))
        );
        assert_eq!(file_url_path("content://media/1"), None);
        assert_eq!(file_url_path("file://relative.jar"), None);
        assert_eq!(file_url_path("file:///bad%2"), None);
        assert_eq!(file_url_path("file:///bad%zz"), None);
    }

    #[test]
    fn the_redraw_signal_wakes_once_for_many_frames() {
        let (notify, mut signal) = redraw_signal();
        notify(InstanceId(1));
        notify(InstanceId(1));
        notify(InstanceId(2));
        assert_eq!(signal.changed().now_or_never(), Some(true));
        // The three frames above woke it once.
        assert_eq!(signal.changed().now_or_never(), None);
        notify(InstanceId(1));
        assert_eq!(signal.changed().now_or_never(), Some(true));
        drop(notify);
        assert!(!futures::executor::block_on(signal.changed()));
    }
}
