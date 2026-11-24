//! One running game as the workspace sees it: the instance, how it was
//! started, and what the app keeps about it between frames.

use std::path::PathBuf;
use std::sync::Arc;

use fastemu_core::{
    Frame, InputEvent, Instance, InstanceStatus, Key, KeyAction, LaunchRequest, PointerAction,
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
    /// Typed characters go to the game as text instead of keypad keys,
    /// for its text boxes.
    pub text_mode: bool,
    /// Whether a pointer press on the screen is in progress.
    pub pointer_down: bool,
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
            text_mode: false,
            pointer_down: false,
        }
    }

    /// Picks up the latest frame, status and log lines. Returns whether
    /// anything changed. Frames are compared by serial, so an unchanged
    /// picture costs nothing.
    pub fn poll(&mut self) -> bool {
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
        let lines = self.instance.drain_log();
        if !lines.is_empty() {
            self.log.extend(lines);
            changed = true;
        }
        changed
    }

    /// Swaps in a fresh instance after a restart, keeping the settings and
    /// slot but dropping the old picture, log and held keys.
    pub fn replace_instance(&mut self, instance: Arc<dyn Instance>) {
        self.title = instance.title();
        self.status = instance.status();
        self.instance = instance;
        self.frame = None;
        self.log.clear();
        self.keys = KeyTracker::default();
        self.pointer_down = false;
        if self.muted {
            self.instance.set_volume(0.0);
        }
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

    pub fn pointer(&self, action: PointerAction, (x, y): (i32, i32)) {
        self.instance.send(InputEvent::Pointer { action, x, y });
    }

    /// Releases every key the keyboard or keypad holds, as when the game
    /// loses focus, so none stays pressed inside the game.
    pub fn release_everything(&mut self) {
        for key in self.keys.release_all() {
            self.release(key);
        }
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
    use fastemu_core::mock::MockLauncher;
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
    fn sessions_follow_their_instance() {
        let request = LaunchRequest::new("snake.jar", "unused");
        let instance = MockLauncher.launch(request.clone()).expect("mock");
        let mut session = Session::new(EntryId(1), 0, request, instance);
        assert_eq!(session.title, "snake");
        let deadline = Instant::now() + Duration::from_secs(5);
        while session.frame.is_none() {
            assert!(Instant::now() < deadline, "no frame within 5 s");
            session.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.screen_size(), (240, 320));
        assert!(!session.log.is_empty());
        while session.status != InstanceStatus::Running {
            assert!(Instant::now() < deadline, "not running within 5 s");
            session.poll();
        }
        session.toggle_pause();
        assert_eq!(session.status, InstanceStatus::Paused);
        session.toggle_pause();
        assert_eq!(session.status, InstanceStatus::Running);
        session.instance.stop();
        while session.status != InstanceStatus::Exited {
            assert!(Instant::now() < deadline, "did not stop within 5 s");
            session.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(session.status_label(), "Exited");
    }
}
