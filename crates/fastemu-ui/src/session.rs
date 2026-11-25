//! One running game as the workspace sees it: the instance, how it was
//! started, and what the app keeps about it between frames.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fastemu_core::{
    Frame, InputEvent, Instance, InstanceId, InstanceStatus, Key, KeyAction, LaunchError,
    LaunchRequest, LogLine, PointerAction, is_web_link,
};

use crate::bindings::KeyTracker;
use crate::library::EntryId;
use crate::logs::LogBuffer;

/// Where an app keeps its files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    /// `library.json` and `settings.json`.
    pub config_dir: PathBuf,
    /// Each game's record stores and files, under `games/`.
    pub data_dir: PathBuf,
    /// Where screenshots go.
    pub screenshot_dir: PathBuf,
}

impl AppPaths {
    /// The folder of one copy of a game. Every copy running at once gets
    /// its own slot, so two copies never share a save.
    pub fn game_dir(&self, entry: EntryId, slot: u32) -> PathBuf {
        self.data_dir
            .join("games")
            .join(entry.0.to_string())
            .join(slot.to_string())
    }

    /// The numbers of the folders under `games`, one for each game that
    /// ever ran here, whether or not the library still lists it. Nothing
    /// deletes a game's saves, so a new game must not get one of these
    /// numbers. A name that is no number is not a game's folder. Folders
    /// that cannot be listed count as none; the library's own counter then
    /// serves.
    pub fn saved_ids(&self) -> HashSet<u64> {
        let games = self.data_dir.join("games");
        let listing = match std::fs::read_dir(&games) {
            Ok(listing) => listing,
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    log::warn!("could not list the saves in {}: {err}", games.display());
                }
                return HashSet::new();
            }
        };
        listing
            .filter_map(Result::ok)
            .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
            .collect()
    }
}

/// The smallest slot number not in `taken`.
pub fn free_slot(taken: impl IntoIterator<Item = u32>) -> u32 {
    let mut taken: Vec<u32> = taken.into_iter().collect();
    taken.sort_unstable();
    taken.dedup();
    taken
        .iter()
        .enumerate()
        .find(|&(i, &slot)| i as u32 != slot)
        .map_or(taken.len() as u32, |(i, _)| i as u32)
}

/// Stands in for a game that could not start, so its tile shows why and
/// can try again. It reads as crashed with the launch error and ignores
/// everything sent to it.
pub struct NotStarted {
    id: InstanceId,
    title: String,
    message: String,
}

impl NotStarted {
    pub fn new(title: impl Into<String>, error: &LaunchError) -> Self {
        Self {
            id: InstanceId::next(),
            title: title.into(),
            message: error.to_string(),
        }
    }
}

impl Instance for NotStarted {
    fn id(&self) -> InstanceId {
        self.id
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn status(&self) -> InstanceStatus {
        InstanceStatus::Crashed {
            message: self.message.clone(),
        }
    }

    fn frame(&self) -> Option<Frame> {
        None
    }

    fn send(&self, _: InputEvent) {}

    fn pause(&self) {}

    fn resume(&self) {}

    fn stop(&self) {}

    fn set_volume(&self, _: f32) {}

    fn drain_log(&self) -> Vec<LogLine> {
        Vec::new()
    }
}

/// How long a link prompt ignores clicks after it appears or its link
/// changes. A game can ask to open a page at the moment of a tap it just
/// got, and a user who taps fast would answer a question they have not read.
pub const LINK_ARMING: Duration = Duration::from_millis(500);

/// A web page a game asked to open, and when the app picked the request
/// up, which is when its prompt appears.
struct AskedLink {
    url: String,
    asked_at: Instant,
}

/// A running game.
pub struct Session {
    pub entry: EntryId,
    pub slot: u32,
    /// The request it started with, to start it again on restart.
    pub request: LaunchRequest,
    pub instance: Arc<dyn Instance>,
    pub title: String,
    pub status: InstanceStatus,
    /// The latest picture, kept after the game ends so its last screen
    /// stays visible.
    pub frame: Option<Frame>,
    pub log: LogBuffer,
    /// What the keyboard and the on-screen keypad hold down.
    pub keys: KeyTracker,
    pub muted: bool,
    /// Whether the game showed a text box when last polled.
    wants_text: bool,
    /// The user's choice of typing mode, which wins over `wants_text`
    /// until the game opens or closes a text box.
    text_override: Option<bool>,
    /// Where a pointer press on the screen holds it, in phone pixels.
    pointer_at: Option<(i32, i32)>,
    /// The web page the game asked to open, until the user opens or
    /// dismisses it.
    link: Option<AskedLink>,
}

impl Session {
    pub fn new(
        entry: EntryId,
        slot: u32,
        request: LaunchRequest,
        instance: Arc<dyn Instance>,
    ) -> Self {
        let title = instance.title();
        let status = instance.status();
        Self {
            entry,
            slot,
            request,
            instance,
            title,
            status,
            frame: None,
            log: LogBuffer::default(),
            keys: KeyTracker::default(),
            muted: false,
            wants_text: false,
            text_override: None,
            pointer_at: None,
            link: None,
        }
    }

    /// Picks up the latest frame, status, log lines and the link the game
    /// asks to open. `now` is the time of the poll, which starts the wait
    /// before the link's prompt takes clicks (see [`LINK_ARMING`]).
    /// Returns whether anything changed. Frames are compared by serial, so
    /// an unchanged picture costs nothing.
    pub fn poll(&mut self, now: Instant) -> bool {
        let mut changed = false;
        if let Some(frame) = self.instance.frame()
            && self
                .frame
                .as_ref()
                .is_none_or(|old| old.serial() != frame.serial())
        {
            self.frame = Some(frame);
            changed = true;
        }
        let status = self.instance.status();
        if status != self.status {
            self.status = status;
            changed = true;
        }
        let wants_text = self.instance.wants_text();
        if wants_text != self.wants_text {
            self.wants_text = wants_text;
            self.text_override = None;
            changed = true;
        }
        let lines = self.instance.drain_log();
        if !lines.is_empty() {
            self.log.extend(lines);
            changed = true;
        }
        if let Some(link) = self.instance.take_link_request() {
            // The runtime hands over only web links; this guards against an
            // instance that would hand over more.
            if is_web_link(&link) {
                // The same page asked for again keeps its wait running, so
                // a game cannot hold the prompt shut by asking in a loop
                // and cannot restart it on a user about to click.
                if self.link.as_ref().is_none_or(|asked| asked.url != link) {
                    self.link = Some(AskedLink {
                        url: link,
                        asked_at: now,
                    });
                }
                changed = true;
            } else {
                log::warn!("{} asked to open {link:?}, not a web link", self.title);
            }
        }
        changed
    }

    /// The web page the game asked to open, until the user opens or
    /// dismisses it. A newer request replaces it.
    pub fn link(&self) -> Option<&str> {
        self.link.as_ref().map(|asked| asked.url.as_str())
    }

    /// Whether the prompt for the game's link takes clicks at `now`: it
    /// has been up for [`LINK_ARMING`].
    pub fn link_armed(&self, now: Instant) -> bool {
        self.link.as_ref().is_some_and(|asked| armed(asked, now))
    }

    /// Answers the prompt for `link`, which the user chose to open or to
    /// dismiss: drops the request and returns true. Nothing happens, and
    /// the answer is false, when the prompt is not armed yet (a click that
    /// came too fast to have read it), or when the game asked for another
    /// page since the prompt showed `link`: its prompt follows, and its
    /// own wait.
    pub fn answer_link(&mut self, link: &str, now: Instant) -> bool {
        let answer = self
            .link
            .as_ref()
            .is_some_and(|asked| asked.url == link && armed(asked, now));
        if answer {
            self.link = None;
        }
        answer
    }

    /// Swaps in a fresh instance after a restart, keeping the settings and
    /// slot but dropping the old picture, log, held keys and the link the
    /// old game asked to open.
    pub fn replace_instance(&mut self, instance: Arc<dyn Instance>) {
        self.title = instance.title();
        self.status = instance.status();
        self.instance = instance;
        self.frame = None;
        self.log.clear();
        self.keys = KeyTracker::default();
        self.pointer_at = None;
        self.wants_text = false;
        self.text_override = None;
        self.link = None;
        if self.muted {
            self.instance.set_volume(0.0);
        }
    }

    /// Whether typed characters go to the game as text, for its text
    /// boxes, instead of pressing keypad keys: while the game shows a text
    /// box, unless the user chose otherwise.
    pub fn typing_text(&self) -> bool {
        self.text_override.unwrap_or(self.wants_text)
    }

    /// Goes back to pressing keys until the game opens or closes a text
    /// box, or the user switches typing on again.
    pub fn stop_typing(&mut self) {
        if self.typing_text() {
            self.toggle_typing();
        }
    }

    /// Switches between typing text and pressing keys, for as long as the
    /// game keeps its text box open or closed. Held keys are let go.
    pub fn toggle_typing(&mut self) {
        self.release_everything();
        self.text_override = Some(!self.typing_text());
    }

    pub fn is_running(&self) -> bool {
        matches!(
            self.status,
            InstanceStatus::Starting | InstanceStatus::Running
        )
    }

    pub fn press(&self, key: Key) {
        self.instance.send(InputEvent::Key {
            key,
            action: KeyAction::Press,
        });
    }

    pub fn release(&self, key: Key) {
        self.instance.send(InputEvent::Key {
            key,
            action: KeyAction::Release,
        });
    }

    fn pointer(&self, action: PointerAction, (x, y): (i32, i32)) {
        self.instance.send(InputEvent::Pointer { action, x, y });
    }

    /// Puts a finger or the mouse button down on the screen at `at`.
    pub fn pointer_press(&mut self, at: (i32, i32)) {
        self.pointer_at = Some(at);
        self.pointer(PointerAction::Press, at);
    }

    /// Moves a pointer that is down to `at`; does nothing when none is.
    pub fn pointer_drag(&mut self, at: (i32, i32)) {
        if self.pointer_at.is_some_and(|last| last != at) {
            self.pointer_at = Some(at);
            self.pointer(PointerAction::Drag, at);
        }
    }

    /// Lifts a pointer that is down, at `at` or where it last was.
    pub fn pointer_release(&mut self, at: Option<(i32, i32)>) {
        if let Some(last) = self.pointer_at.take() {
            self.pointer(PointerAction::Release, at.unwrap_or(last));
        }
    }

    pub fn is_pointer_down(&self) -> bool {
        self.pointer_at.is_some()
    }

    /// Releases every key the keyboard or keypad holds and a pointer that
    /// is down, as when the game or the window loses focus, so nothing
    /// stays pressed inside the game.
    pub fn release_everything(&mut self) {
        for key in self.keys.release_all() {
            self.release(key);
        }
        self.pointer_release(None);
    }

    pub fn toggle_pause(&mut self) {
        match self.status {
            InstanceStatus::Paused => self.instance.resume(),
            InstanceStatus::Starting | InstanceStatus::Running => {
                self.release_everything();
                self.instance.pause();
            }
            InstanceStatus::Exited | InstanceStatus::Crashed { .. } => {}
        }
        self.status = self.instance.status();
    }

    /// Mutes or unmutes; unmuting goes back to `volume`, the game's
    /// setting.
    pub fn set_muted(&mut self, muted: bool, volume: f32) {
        self.muted = muted;
        self.instance.set_volume(if muted { 0.0 } else { volume });
    }

    pub fn status_label(&self) -> String {
        status_label(&self.status)
    }

    /// The screen size in phone pixels: the latest frame's, or the
    /// profile's before the first frame.
    pub fn screen_size(&self) -> (u32, u32) {
        self.frame.as_ref().map_or(
            (
                self.request.profile.screen_width,
                self.request.profile.screen_height,
            ),
            |f| (f.width(), f.height()),
        )
    }
}

/// Whether the prompt for `asked` takes clicks at `now`.
fn armed(asked: &AskedLink, now: Instant) -> bool {
    now.saturating_duration_since(asked.asked_at) >= LINK_ARMING
}

/// A word or two for a title bar.
pub fn status_label(status: &InstanceStatus) -> String {
    match status {
        InstanceStatus::Starting => "Starting".to_owned(),
        InstanceStatus::Running => "Running".to_owned(),
        InstanceStatus::Paused => "Paused".to_owned(),
        InstanceStatus::Exited => "Exited".to_owned(),
        InstanceStatus::Crashed { .. } => "Crashed".to_owned(),
    }
}

/// Maps a point on the drawn screen to phone pixels. `origin` and `size`
/// are where the screen was drawn, `screen` its size in phone pixels.
/// Returns the pixel, clamped onto the screen, and whether the point was
/// inside it.
pub fn phone_pixel(
    point: (f32, f32),
    origin: (f32, f32),
    size: (f32, f32),
    screen: (u32, u32),
) -> ((i32, i32), bool) {
    if size.0 <= 0.0 || size.1 <= 0.0 || screen.0 == 0 || screen.1 == 0 {
        return ((0, 0), false);
    }
    let fx = (point.0 - origin.0) / size.0;
    let fy = (point.1 - origin.1) / size.1;
    let inside = (0.0..1.0).contains(&fx) && (0.0..1.0).contains(&fy);
    let x = (fx * screen.0 as f32)
        .floor()
        .clamp(0.0, (screen.0 - 1) as f32) as i32;
    let y = (fy * screen.1 as f32)
        .floor()
        .clamp(0.0, (screen.1 - 1) as f32) as i32;
    ((x, y), inside)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastemu_core::Launcher;
    use fastemu_core::mock::{MOCK_LINK, MockLauncher};
    use std::time::{Duration, Instant};

    #[test]
    fn slots_fill_the_first_gap() {
        assert_eq!(free_slot([]), 0);
        assert_eq!(free_slot([0, 1, 2]), 3);
        assert_eq!(free_slot([1, 2]), 0);
        assert_eq!(free_slot([0, 2, 2]), 1);
    }

    #[test]
    fn copies_of_a_game_get_their_own_folders() {
        let paths = AppPaths {
            config_dir: "c".into(),
            data_dir: "d".into(),
            screenshot_dir: "s".into(),
        };
        assert_ne!(paths.game_dir(EntryId(1), 0), paths.game_dir(EntryId(1), 1));
        assert!(paths.game_dir(EntryId(7), 2).ends_with("games/7/2"));
    }

    #[test]
    fn the_saves_folders_are_found_by_their_numbers() {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths {
            config_dir: dir.path().join("config"),
            data_dir: dir.path().join("data"),
            screenshot_dir: dir.path().join("shots"),
        };
        assert!(paths.saved_ids().is_empty(), "no games folder yet");
        for name in ["1/0", "7/0/rms", "12", "notes", "3x", "-4"] {
            std::fs::create_dir_all(paths.data_dir.join("games").join(name)).expect("mkdir");
        }
        let found = paths.saved_ids();
        assert_eq!(found, HashSet::from([1, 7, 12]));
    }

    #[test]
    fn pointer_positions_map_to_phone_pixels() {
        let map = |p| phone_pixel(p, (10.0, 20.0), (480.0, 640.0), (240, 320));
        assert_eq!(map((10.0, 20.0)), ((0, 0), true));
        assert_eq!(map((11.9, 21.9)), ((0, 0), true));
        assert_eq!(map((12.0, 22.0)), ((1, 1), true));
        assert_eq!(map((489.9, 659.9)), ((239, 319), true));
        assert_eq!(map((500.0, 0.0)), ((239, 0), false));
        assert_eq!(
            phone_pixel((1.0, 1.0), (0.0, 0.0), (0.0, 0.0), (240, 320)),
            ((0, 0), false)
        );
    }

    #[test]
    fn only_web_links_are_offered() {
        assert!(is_web_link("http://example.com"));
        assert!(is_web_link("HTTPS://example.com/a"));
        for other in ["https://", "tel:123", "file:///x", "javascript:x", "", "h"] {
            assert!(!is_web_link(other), "{other}");
        }
        // What the user reads is what opens: no spaces, line breaks or
        // characters that reorder the text.
        for hidden in [
            "https://example.com/\u{202e}moc.elpmaxe",
            "https://example.com/\nhttps://other.example/",
            "https://example.com/a b",
            "https://ex\u{0430}mple.com/",
            "http://good.com\\.evil.com/",
        ] {
            assert!(!is_web_link(hidden), "{hidden:?}");
        }
        // The prompt shows the whole link, so a link has its length limit.
        let long = format!("http://example.com/{}", "a".repeat(4096));
        assert!(!is_web_link(&long), "4097 bytes");
        // A port that no server listens on is no page either.
        assert!(is_web_link("http://example.com:8080/"));
        for port in ["99999", "0", "abc", ""] {
            let link = format!("http://example.com:{port}/");
            assert!(!is_web_link(&link), "{link}");
        }
    }

    #[test]
    fn the_link_a_game_asks_for_waits_for_the_user() {
        let request = LaunchRequest::new("snake.jar", "unused");
        let instance = MockLauncher.launch(request.clone()).expect("mock");
        let mut session = Session::new(EntryId(1), 0, request.clone(), instance);
        let (asked, armed) = (Instant::now(), Instant::now() + LINK_ARMING);
        session.poll(asked);
        assert_eq!(session.link(), None);
        session.press(Key::Pound);
        assert!(session.poll(asked));
        assert_eq!(session.link(), Some(MOCK_LINK));
        session.poll(armed);
        assert_eq!(session.link(), Some(MOCK_LINK), "it waits for an answer");
        assert!(!session.answer_link("https://other.example/", armed));
        assert_eq!(session.link(), Some(MOCK_LINK), "only the shown link");
        assert!(session.answer_link(MOCK_LINK, armed));
        assert_eq!(session.link(), None);
        assert!(!session.answer_link(MOCK_LINK, armed), "answered once");

        session.press(Key::Pound);
        session.poll(armed);
        assert!(session.link().is_some());
        let again = MockLauncher.launch(request).expect("mock");
        session.instance.stop();
        session.replace_instance(again);
        assert_eq!(session.link(), None, "a restart drops it");
        session.instance.stop();
    }

    #[test]
    fn a_link_prompt_takes_no_answer_in_its_first_half_second() {
        let request = LaunchRequest::new("snake.jar", "unused");
        let instance = MockLauncher.launch(request.clone()).expect("mock");
        let mut session = Session::new(EntryId(1), 0, request, instance);
        let asked = Instant::now();
        let almost = asked + Duration::from_millis(499);
        assert!(!session.link_armed(asked), "no link, no prompt");
        session.press(Key::Pound);
        session.poll(asked);
        for now in [asked, almost] {
            assert!(!session.link_armed(now));
            assert!(!session.answer_link(MOCK_LINK, now), "too soon: {now:?}");
            assert_eq!(session.link(), Some(MOCK_LINK), "the prompt stays");
        }
        assert!(session.link_armed(asked + LINK_ARMING));
        // The game asking for the same page again does not restart the wait.
        session.press(Key::Pound);
        session.poll(almost);
        assert!(session.link_armed(asked + LINK_ARMING));
        assert!(session.answer_link(MOCK_LINK, asked + LINK_ARMING));
        // A clock that reads earlier than the poll is never armed.
        session.press(Key::Pound);
        session.poll(asked + LINK_ARMING);
        assert!(!session.link_armed(asked));
        session.instance.stop();
    }

    #[test]
    fn a_game_that_did_not_start_says_why() {
        let error = LaunchError::NoSuchMidlet(2);
        let instance: Arc<dyn Instance> = Arc::new(NotStarted::new("Snake", &error));
        let session = Session::new(
            EntryId(1),
            0,
            LaunchRequest::new("snake.jar", "unused"),
            instance,
        );
        assert_eq!(session.title, "Snake");
        assert_eq!(
            session.status,
            InstanceStatus::Crashed {
                message: "the suite has no MIDlet number 3".into()
            }
        );
        assert!(!session.is_running());
    }

    #[test]
    fn sessions_follow_their_instance() {
        let request = LaunchRequest::new("snake.jar", "unused");
        let instance = MockLauncher.launch(request.clone()).expect("mock");
        let mut session = Session::new(EntryId(1), 0, request, instance);
        assert_eq!(session.title, "snake");
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.frame.is_none() {
            assert!(Instant::now() < deadline, "no frame within 5 s");
            session.poll(Instant::now());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.screen_size(), (240, 320));
        assert!(!session.log.is_empty());
        while session.status != InstanceStatus::Running {
            assert!(Instant::now() < deadline, "not running within 5 s");
            session.poll(Instant::now());
        }
        session.toggle_pause();
        assert_eq!(session.status, InstanceStatus::Paused);
        session.toggle_pause();
        assert_eq!(session.status, InstanceStatus::Running);
        session.instance.stop();
        while session.status != InstanceStatus::Exited {
            assert!(Instant::now() < deadline, "did not stop within 5 s");
            session.poll(Instant::now());
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.status_label(), "Exited");
    }
}
