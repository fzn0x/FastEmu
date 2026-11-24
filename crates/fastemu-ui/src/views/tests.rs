//! The root view driven headless through GPUI's test platform, with a
//! launcher whose instances record what they receive.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use fastemu_core::{
    Frame, InputEvent, Instance, InstanceId, InstanceStatus, Key, KeyAction, LaunchError,
    LaunchRequest, Launcher, LogLevel, LogLine, SuiteInfo,
};
use gpui::{Entity, KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext, VisualTestContext};

use super::*;
use crate::settings::LayoutMode;

/// An instance that shows whatever frame the test sets and keeps every
/// input event.
struct FakeInstance {
    id: InstanceId,
    title: String,
    status: Mutex<InstanceStatus>,
    frame: Mutex<Option<Frame>>,
    events: Mutex<Vec<InputEvent>>,
    log: Mutex<Vec<LogLine>>,
    volume: Mutex<f32>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeInstance {
    fn events(&self) -> Vec<InputEvent> {
        lock(&self.events).clone()
    }

    fn show(&self, serial: u64) {
        *lock(&self.frame) = Some(Frame::filled(240, 320, 0xFF00_8040, serial));
    }
}

impl Instance for FakeInstance {
    fn id(&self) -> InstanceId {
        self.id
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn status(&self) -> InstanceStatus {
        lock(&self.status).clone()
    }

    fn frame(&self) -> Option<Frame> {
        lock(&self.frame).clone()
    }

    fn send(&self, event: InputEvent) {
        lock(&self.events).push(event);
    }

    fn pause(&self) {
        *lock(&self.status) = InstanceStatus::Paused;
    }

    fn resume(&self) {
        *lock(&self.status) = InstanceStatus::Running;
    }

    fn stop(&self) {
        *lock(&self.status) = InstanceStatus::Exited;
    }

    fn set_volume(&self, volume: f32) {
        *lock(&self.volume) = volume;
    }

    fn drain_log(&self) -> Vec<LogLine> {
        std::mem::take(&mut lock(&self.log))
    }
}

/// Reads any `.jar` or `.jad` path as a suite named after the file, and
/// keeps every instance it starts.
#[derive(Default)]
struct FakeLauncher {
    started: Mutex<Vec<(LaunchRequest, Arc<FakeInstance>)>>,
}

impl FakeLauncher {
    fn instance(&self, index: usize) -> Arc<FakeInstance> {
        Arc::clone(&lock(&self.started)[index].1)
    }

    fn request(&self, index: usize) -> LaunchRequest {
        lock(&self.started)[index].0.clone()
    }

    fn count(&self) -> usize {
        lock(&self.started).len()
    }
}

impl Launcher for FakeLauncher {
    fn inspect(&self, suite: &Path) -> Result<SuiteInfo, LaunchError> {
        let name = suite
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .ok_or_else(|| LaunchError::InvalidSuite("no file name".into()))?;
        if name.starts_with("broken") {
            return Err(LaunchError::InvalidSuite("no manifest".into()));
        }
        Ok(SuiteInfo {
            name: name.to_uppercase(),
            vendor: "Test".into(),
            ..SuiteInfo::default()
        })
    }

    fn launch(&self, request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
        let info = self.inspect(&request.suite)?;
        let instance = Arc::new(FakeInstance {
            id: InstanceId::next(),
            title: info.name,
            status: Mutex::new(InstanceStatus::Running),
            frame: Mutex::new(None),
            events: Mutex::new(Vec::new()),
            log: Mutex::new(vec![LogLine {
                level: LogLevel::Warn,
                text: "javax.microedition.m3g is not available".into(),
            }]),
            volume: Mutex::new(request.settings.volume),
        });
        lock(&self.started).push((request, Arc::clone(&instance)));
        Ok(instance)
    }
}

struct Harness<'a> {
    view: Entity<FastEmuView>,
    cx: &'a mut VisualTestContext,
    launcher: Arc<FakeLauncher>,
    dir: tempfile::TempDir,
}

fn open(cx: &mut TestAppContext) -> Harness<'_> {
    let dir = tempfile::tempdir().expect("temp dir");
    open_in(cx, dir)
}

fn open_in(cx: &mut TestAppContext, dir: tempfile::TempDir) -> Harness<'_> {
    let launcher = Arc::new(FakeLauncher::default());
    let config = AppConfig {
        launcher: launcher.clone(),
        paths: AppPaths {
            config_dir: dir.path().join("config"),
            data_dir: dir.path().join("data"),
            screenshot_dir: dir.path().join("shots"),
        },
    };
    cx.update(init);
    let (view, cx) = cx.add_window_view(|window, cx| FastEmuView::new(config, window, cx));
    Harness {
        view,
        cx,
        launcher,
        dir,
    }
}

impl Harness<'_> {
    fn add(&mut self, names: &[&str]) {
        let paths: Vec<PathBuf> = names.iter().map(|n| self.dir.path().join(n)).collect();
        self.view.update_in(self.cx, |view, window, cx| {
            view.add_paths(paths, false, window, cx);
        });
        self.cx.run_until_parked();
    }

    fn launch_first(&mut self) {
        self.view.update_in(self.cx, |view, window, cx| {
            let id = view.library.entries()[0].id;
            view.launch(id, window, cx);
        });
        self.cx.run_until_parked();
    }

    fn read<R>(&mut self, f: impl FnOnce(&FastEmuView) -> R) -> R {
        self.view.read_with(self.cx, |view, _| f(view))
    }

    fn key_down(&mut self, source: &'static str, key_char: Option<&str>, held: bool) {
        let mut keystroke = Keystroke::parse(source).expect("parses");
        keystroke.key_char = key_char.map(str::to_owned);
        self.cx.simulate_event(KeyDownEvent {
            keystroke,
            is_held: held,
            prefer_character_input: false,
        });
    }

    fn key_up(&mut self, source: &'static str) {
        let keystroke = Keystroke::parse(source).expect("parses");
        self.cx.simulate_event(KeyUpEvent { keystroke });
    }

    fn redraw(&mut self) {
        self.view.update(self.cx, |_, cx| cx.notify());
        self.cx.run_until_parked();
    }
}

fn key(key: Key, action: KeyAction) -> InputEvent {
    InputEvent::Key { key, action }
}

#[gpui::test]
fn adding_files_fills_and_saves_the_library(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar", "tetris.jad", "notes.txt", "broken.jar"]);
    let (names, notices) = h.read(|v| {
        (
            v.library
                .entries()
                .iter()
                .map(|e| e.name.clone())
                .collect::<Vec<_>>(),
            v.notices
                .iter()
                .map(|n| n.text.to_string())
                .collect::<Vec<_>>(),
        )
    });
    assert_eq!(names, vec!["SNAKE", "TETRIS"]);
    assert!(
        notices
            .iter()
            .any(|n| n.contains("notes.txt is not a .jar")),
        "{notices:?}"
    );
    assert!(
        notices
            .iter()
            .any(|n| n.contains("Could not add broken.jar")),
        "{notices:?}"
    );
    let saved = std::fs::read_to_string(h.dir.path().join("config/library.json")).expect("saved");
    assert!(saved.contains("SNAKE") && saved.contains("TETRIS"));

    h.add(&["snake.jar"]);
    assert_eq!(h.read(|v| v.library.entries().len()), 2, "no duplicates");
}

#[gpui::test]
fn launching_starts_a_game_in_its_own_slot(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.launch_first();
    assert_eq!(h.launcher.count(), 2);
    let (first, second) = (h.launcher.request(0), h.launcher.request(1));
    assert_ne!(first.data_dir, second.data_dir, "copies share no saves");
    assert!(
        first.data_dir.exists(),
        "the data folder is made before launch"
    );
    assert!(first.notify.is_some());
    let (tiles, focused) = h.read(|v| (v.tiles.len(), v.focused));
    assert_eq!(tiles, 2);
    assert_eq!(focused, h.read(|v| v.tiles.last().map(|t| t.id)));
}

#[gpui::test]
fn keys_reach_the_focused_game_once_per_stroke(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);

    h.key_down("up", None, false);
    h.key_down("up", None, true);
    h.key_up("up");
    h.key_down("enter", None, false);
    h.key_down("space", Some(" "), false);
    h.key_up("enter");
    h.key_up("space");
    h.key_down("ctrl-up", None, false);
    assert_eq!(
        game.events(),
        vec![
            key(Key::Up, KeyAction::Press),
            key(Key::Up, KeyAction::Release),
            key(Key::Fire, KeyAction::Press),
            key(Key::Fire, KeyAction::Release),
        ]
    );
}

#[gpui::test]
fn text_mode_sends_typed_characters(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["chat.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    h.view
        .update(h.cx, |v, _| v.tiles[0].session.text_mode = true);
    h.key_down("a", Some("a"), false);
    h.key_down("left", None, false);
    h.key_up("left");
    assert_eq!(
        game.events(),
        vec![
            InputEvent::Text("a".into()),
            key(Key::Left, KeyAction::Press),
            key(Key::Left, KeyAction::Release),
        ]
    );
}

#[gpui::test]
fn switching_games_releases_held_keys(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.launch_first();
    let first = h.launcher.instance(0);
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.focus_tile(id, cx);
    });
    h.key_down("right", None, false);
    h.view.update(h.cx, |v, cx| v.cycle_focus(1, cx));
    assert_eq!(
        first.events(),
        vec![
            key(Key::Right, KeyAction::Press),
            key(Key::Right, KeyAction::Release),
        ]
    );
}

#[gpui::test]
fn the_keypad_presses_and_releases(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.keypad_press(id, Key::Num5, cx);
        v.keypad_press(id, Key::Star, cx);
        v.keypad_release(id, cx);
        v.keypad_release(id, cx);
    });
    assert_eq!(
        game.events(),
        vec![
            key(Key::Num5, KeyAction::Press),
            key(Key::Num5, KeyAction::Release),
            key(Key::Star, KeyAction::Press),
            key(Key::Star, KeyAction::Release),
        ]
    );
}

#[gpui::test]
fn new_frames_replace_the_uploaded_image(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    let screen = h.read(|v| v.tiles[0].screen.clone());

    game.show(1);
    h.redraw();
    let first = screen
        .borrow()
        .image()
        .cloned()
        .expect("the first frame is uploaded");
    assert_eq!(screen.borrow().uploads(), 1);

    h.redraw();
    assert_eq!(
        screen.borrow().uploads(),
        1,
        "an unchanged frame is not uploaded again"
    );

    for serial in 2..10 {
        game.show(serial);
        h.redraw();
    }
    assert_eq!(screen.borrow().uploads(), 9);
    let last = screen.borrow().image().cloned().expect("an image");
    let (old_kept, new_kept) = h.cx.update(|window, _| {
        (
            window.has_image_atlas_entry(&first),
            window.has_image_atlas_entry(&last),
        )
    });
    assert!(!old_kept, "the first frame left the atlas");
    assert!(new_kept);
    assert!(screen.borrow().drawn().is_some());
}

#[gpui::test]
fn closing_a_game_stops_it_and_frees_its_image(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    game.show(1);
    h.redraw();
    let image = h
        .read(|v| v.tiles[0].screen.borrow().image().cloned())
        .expect("an image");
    h.view.update_in(h.cx, |v, window, cx| {
        let id = v.tiles[0].id;
        v.close_tile(id, window, cx);
    });
    assert_eq!(game.status(), InstanceStatus::Exited);
    assert_eq!(h.read(|v| v.tiles.len()), 0);
    assert!(
        !h.cx
            .update(|window, _| window.has_image_atlas_entry(&image))
    );
}

#[gpui::test]
fn logs_collect_and_flag_problems(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.redraw();
    let unseen = |h: &mut Harness<'_>| {
        h.read(|v| {
            (
                v.tiles[0].session.log.len(),
                v.tiles[0].session.log.unseen_problems(),
            )
        })
    };
    assert_eq!(
        unseen(&mut h),
        (1, 1),
        "a warning waits for the log to open"
    );
    h.view.update(h.cx, FastEmuView::toggle_log);
    h.redraw();
    assert_eq!(unseen(&mut h), (1, 0), "opening the log marks it seen");
    let game = h.launcher.instance(0);
    lock(&game.log).push(LogLine {
        level: LogLevel::Err,
        text: "java.lang.NullPointerException".into(),
    });
    h.redraw();
    assert_eq!(unseen(&mut h), (2, 0), "new lines in an open log are seen");
}

#[gpui::test]
fn settings_changes_are_saved(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.view.update(h.cx, |v, cx| {
        v.toggle_layout(cx);
        v.update_settings(cx, |s| {
            s.bindings.bind(Key::Fire, "f");
            s.theme = crate::settings::ThemeMode::Light;
        });
    });
    assert_eq!(h.read(|v| v.settings.layout), LayoutMode::Focus);
    let saved = Store::new(h.dir.path().join("config"))
        .load_settings()
        .value;
    assert_eq!(saved.layout, LayoutMode::Focus);
    assert_eq!(saved.bindings.key_for("f"), Some(Key::Fire));
    let mode = h.cx.update(|_, cx| Theme::get(cx).mode);
    assert_eq!(
        mode,
        crate::settings::ThemeMode::Light,
        "the theme switches at once"
    );
}

#[gpui::test]
fn a_damaged_settings_file_is_reported(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir_all(dir.path().join("config")).expect("mkdir");
    std::fs::write(dir.path().join("config/settings.json"), "{oops").expect("write");
    let mut h = open_in(cx, dir);
    let notices = h.read(|v| {
        v.notices
            .iter()
            .map(|n| n.text.to_string())
            .collect::<Vec<_>>()
    });
    assert!(
        notices
            .iter()
            .any(|n| n.contains("settings.json is damaged")),
        "{notices:?}"
    );
    assert_eq!(h.read(|v| v.settings.clone()), AppSettings::default());
}

#[gpui::test]
fn the_bindings_editor_captures_the_next_key(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    h.view.update(h.cx, |v, cx| {
        v.open_dialog(Dialog::Settings(SettingsTab::Keys), cx);
        v.capturing = Some(Key::Fire);
    });
    h.key_down("x", Some("x"), false);
    assert_eq!(
        h.read(|v| v.settings.bindings.key_for("x")),
        Some(Key::Fire)
    );
    assert_eq!(h.read(|v| v.capturing), None);
    h.key_down("escape", None, false);
    assert_eq!(h.read(|v| v.dialog), None);
    h.key_down("x", Some("x"), false);
    assert_eq!(game.events(), vec![key(Key::Fire, KeyAction::Press)]);
}

#[gpui::test]
fn pausing_and_muting_reach_the_instance(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    h.view.update(h.cx, |v, cx| {
        v.tiles[0].session.toggle_pause();
        let id = v.tiles[0].id;
        v.toggle_mute(id, cx);
    });
    assert_eq!(game.status(), InstanceStatus::Paused);
    assert!(lock(&game.volume).abs() < f32::EPSILON);
    h.key_down("up", None, false);
    assert!(game.events().is_empty(), "a paused game gets no keys");
}

#[gpui::test]
fn opened_files_are_added_and_started(cx: &mut TestAppContext) {
    let mut h = open(cx);
    let paths = vec![h.dir.path().join("snake.jar")];
    for _ in 0..2 {
        let paths = paths.clone();
        h.view
            .update_in(h.cx, |v, window, cx| v.add_paths(paths, true, window, cx));
        h.cx.run_until_parked();
    }
    assert_eq!(h.launcher.count(), 2, "each open starts a copy");
    assert_eq!(h.read(|v| v.library.entries().len()), 1, "and adds it once");
}

#[gpui::test]
fn relative_paths_are_stored_absolute(cx: &mut TestAppContext) {
    let mut h = open(cx);
    // As `fastemu snake.jar` passes it from the shell's folder.
    h.view.update_in(h.cx, |v, window, cx| {
        v.add_paths(vec![PathBuf::from("snake.jar")], false, window, cx);
    });
    h.cx.run_until_parked();
    let path = h.read(|v| v.library.entries()[0].path.clone());
    assert!(path.is_absolute(), "{}", path.display());
    assert_eq!(
        path,
        std::env::current_dir().expect("cwd").join("snake.jar")
    );
}

#[gpui::test]
fn every_layout_and_dialog_renders(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar", "tetris.jar"]);
    for _ in 0..3 {
        h.launch_first();
    }
    for i in 0..3 {
        h.launcher.instance(i).show(1);
    }
    let entry = h.read(|v| v.library.entries()[1].id);
    type Step = Box<dyn Fn(&mut FastEmuView, &mut Context<'_, FastEmuView>)>;
    let steps: Vec<Step> = vec![
        Box::new(FastEmuView::toggle_layout),
        Box::new(|v, cx| v.update_settings(cx, |s| s.keypad = false)),
        Box::new(FastEmuView::toggle_log),
        Box::new(|v, cx| v.open_dialog(Dialog::Settings(SettingsTab::General), cx)),
        Box::new(|v, cx| v.open_dialog(Dialog::Settings(SettingsTab::Keys), cx)),
        Box::new(move |v, cx| v.open_dialog(Dialog::Game(entry), cx)),
        Box::new(|v, cx| {
            v.close_dialog(cx);
            v.toggle_layout(cx);
            v.sidebar_open = false;
            v.confirm_remove = Some(v.library.entries()[0].id);
        }),
    ];
    for step in steps {
        h.view.update(h.cx, |v, cx| step(v, cx));
        h.redraw();
    }
    let drawn = h.read(|v| {
        v.tiles
            .iter()
            .filter(|t| t.screen.borrow().drawn().is_some())
            .count()
    });
    assert_eq!(drawn, 3, "every tile drew its screen");
}

#[gpui::test]
fn game_settings_apply_to_the_next_start(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    let id = h.read(|v| v.library.entries()[0].id);
    h.view.update(h.cx, |v, cx| {
        v.update_game_settings(id, cx, |s| {
            s.profile = Some("nokia-s40-128x160".into());
            s.network = true;
            "windows-1251".clone_into(&mut s.encoding);
            s.step_volume(-0.5);
        });
    });
    h.launch_first();
    let request = h.launcher.request(0);
    assert_eq!(request.profile.screen_width, 128);
    assert_eq!(request.settings.encoding, "windows-1251");
    assert_eq!(request.settings.network, fastemu_core::NetworkPolicy::Allow);
    assert!((request.settings.volume - 0.5).abs() < 1e-6);
    let saved = Store::new(h.dir.path().join("config")).load_library().value;
    assert_eq!(saved.entries()[0].settings.encoding, "windows-1251");
}
