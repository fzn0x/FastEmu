//! A stand-in [`Launcher`] for building and testing apps without the
//! virtual machine. Its instances draw a moving test pattern with a square
//! the arrow keys move and the fire key recolours.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use crate::frame::Frame;
use crate::input::{InputEvent, Key, KeyAction};
use crate::instance::{
    Instance, InstanceId, InstanceStatus, LaunchError, LaunchRequest, Launcher, LogLevel, LogLine,
    Notify,
};
use crate::suite::{MidletInfo, SuiteInfo};

/// Launches [`MockInstance`]s for any path, existing or not.
#[derive(Clone, Copy, Debug, Default)]
pub struct MockLauncher;

impl Launcher for MockLauncher {
    fn inspect(&self, suite: &Path) -> Result<SuiteInfo, LaunchError> {
        let name = suite
            .file_stem()
            .map_or_else(|| "Mock".to_owned(), |s| s.to_string_lossy().into_owned());
        Ok(SuiteInfo {
            name: name.clone(),
            vendor: "FastEmu".to_owned(),
            version: "1.0".to_owned(),
            description: Some("A test pattern instead of a game".to_owned()),
            icon: None,
            midlets: vec![MidletInfo {
                name,
                icon_path: None,
                class_name: "Mock".to_owned(),
            }],
            properties: Vec::new(),
            jar_bytes: 0,
        })
    }

    fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
        let info = self.inspect(&request.suite)?;
        if request.midlet >= info.midlets.len() {
            return Err(LaunchError::NoSuchMidlet(request.midlet));
        }
        let instance = Arc::new(MockInstance::new(
            info.name,
            request.profile.screen_width.max(1),
            request.profile.screen_height.max(1),
            request.notify,
        ));
        MockInstance::spawn(&instance);
        Ok(instance)
    }
}

struct State {
    status: InstanceStatus,
    frame: Option<Frame>,
    square: (i32, i32),
    held: Vec<Key>,
    tint: u32,
    tick: u64,
    log: Vec<LogLine>,
}

/// An instance that animates a test pattern on its own thread.
pub struct MockInstance {
    id: InstanceId,
    title: String,
    width: u32,
    height: u32,
    notify: Option<Notify>,
    stopped: AtomicBool,
    state: Mutex<State>,
}

impl MockInstance {
    fn new(title: String, width: u32, height: u32, notify: Option<Notify>) -> Self {
        Self {
            id: InstanceId::next(),
            title,
            width,
            height,
            notify,
            stopped: AtomicBool::new(false),
            state: Mutex::new(State {
                status: InstanceStatus::Starting,
                frame: None,
                square: (width as i32 / 2 - 8, height as i32 / 2 - 8),
                held: Vec::new(),
                tint: 0x00FF_7A00,
                tick: 0,
                log: vec![LogLine {
                    level: LogLevel::Info,
                    text: "mock instance started".to_owned(),
                }],
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn spawn(this: &Arc<Self>) {
        let me = Arc::clone(this);
        thread::Builder::new()
            .name(format!("mock instance {}", me.id))
            .spawn(move || me.run())
            .expect("spawning the mock instance's thread");
    }

    fn run(&self) {
        self.lock().status = InstanceStatus::Running;
        while !self.stopped.load(Ordering::Acquire) {
            let paused = self.lock().status == InstanceStatus::Paused;
            if !paused {
                self.step();
                if let Some(notify) = &self.notify {
                    notify(self.id);
                }
            }
            thread::sleep(Duration::from_millis(33));
        }
        self.lock().status = InstanceStatus::Exited;
        if let Some(notify) = &self.notify {
            notify(self.id);
        }
    }

    fn step(&self) {
        let mut state = self.lock();
        state.tick += 1;
        let (w, h) = (self.width as i32, self.height as i32);
        for key in state.held.clone() {
            let (dx, dy) = match key {
                Key::Left | Key::Num4 => (-2, 0),
                Key::Right | Key::Num6 => (2, 0),
                Key::Up | Key::Num2 => (0, -2),
                Key::Down | Key::Num8 => (0, 2),
                _ => (0, 0),
            };
            state.square.0 = (state.square.0 + dx).clamp(0, w - 16);
            state.square.1 = (state.square.1 + dy).clamp(0, h - 16);
        }
        let t = state.tick as u32;
        let mut pixels = Vec::with_capacity((w * h) as usize);
        for y in 0..h as u32 {
            for x in 0..w as u32 {
                let r = (x + t) & 0xFF;
                let g = (y + t / 2) & 0xFF;
                let b = ((x ^ y) + t) & 0x7F;
                pixels.push(0xFF00_0000 | (r << 16) | (g << 8) | b);
            }
        }
        let (sx, sy) = state.square;
        for y in sy.max(0)..(sy + 16).min(h) {
            for x in sx.max(0)..(sx + 16).min(w) {
                pixels[(y * w + x) as usize] = 0xFF00_0000 | state.tint;
            }
        }
        let serial = state.tick;
        state.frame = Some(Frame::new(self.width, self.height, pixels.into(), serial));
    }
}

impl Instance for MockInstance {
    fn id(&self) -> InstanceId {
        self.id
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn status(&self) -> InstanceStatus {
        self.lock().status.clone()
    }

    fn frame(&self) -> Option<Frame> {
        self.lock().frame.clone()
    }

    fn send(&self, event: InputEvent) {
        let mut state = self.lock();
        match event {
            InputEvent::Key {
                key: Key::Fire | Key::Num5,
                action: KeyAction::Press,
            } => {
                state.tint = state.tint.rotate_left(8) & 0x00FF_FFFF;
            }
            InputEvent::Key {
                key,
                action: KeyAction::Press,
            } => {
                if !state.held.contains(&key) {
                    state.held.push(key);
                }
            }
            InputEvent::Key {
                key,
                action: KeyAction::Release,
            } => state.held.retain(|&k| k != key),
            InputEvent::Pointer { x, y, .. } => state.square = (x - 8, y - 8),
            InputEvent::Text(text) => state.log.push(LogLine {
                level: LogLevel::Out,
                text: format!("typed {text:?}"),
            }),
            InputEvent::RawKey { .. } => {}
        }
    }

    fn pause(&self) {
        let mut state = self.lock();
        if state.status == InstanceStatus::Running {
            state.status = InstanceStatus::Paused;
        }
    }

    fn resume(&self) {
        let mut state = self.lock();
        if state.status == InstanceStatus::Paused {
            state.status = InstanceStatus::Running;
        }
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    fn set_volume(&self, _volume: f32) {}

    fn drain_log(&self) -> Vec<LogLine> {
        std::mem::take(&mut self.lock().log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::LaunchRequest;
    use std::time::Instant;

    #[test]
    fn mock_instances_draw_and_stop() {
        let instance = MockLauncher
            .launch(LaunchRequest::new("snake.jar", "unused"))
            .expect("mock launches never fail");
        assert_eq!(instance.title(), "snake");
        let deadline = Instant::now() + Duration::from_secs(5);
        while instance.frame().is_none() {
            assert!(Instant::now() < deadline, "no frame within 5 s");
            thread::sleep(Duration::from_millis(10));
        }
        let frame = instance.frame().expect("checked above");
        assert_eq!((frame.width(), frame.height()), (240, 320));
        instance.stop();
        while instance.status() != InstanceStatus::Exited {
            assert!(Instant::now() < deadline, "did not stop within 5 s");
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!instance.drain_log().is_empty());
    }
}
