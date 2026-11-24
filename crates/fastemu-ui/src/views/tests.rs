//! The root view driven headless through GPUI's test platform, with a
//! launcher whose instances record what they receive.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use fastemu_core::{
    Frame, InputEvent, Instance, InstanceId, InstanceStatus, Key, KeyAction, LaunchError,
    LaunchRequest, Launcher, LogLevel, LogLine, PointerAction, SuiteInfo,
};
use gpui::{
    Entity, KeyDownEvent, KeyUpEvent, Keystroke, ScrollStrategy, TestAppContext, VisualTestContext,
};

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
    wants_text: Mutex<bool>,
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

    fn wants_text(&self) -> bool {
        *lock(&self.wants_text)
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
        if info.name.starts_with("FAILS") {
            return Err(LaunchError::Unsupported("JSR-184 3D graphics".into()));
        }
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
            wants_text: Mutex::new(false),
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
    open_with(cx, dir, None)
}

fn open_with(
    cx: &mut TestAppContext,
    dir: tempfile::TempDir,
    text_entry: Option<TextEntry>,
) -> Harness<'_> {
    let launcher = Arc::new(FakeLauncher::default());
    let config = AppConfig {
        launcher: launcher.clone(),
        paths: AppPaths {
            config_dir: dir.path().join("config"),
            data_dir: dir.path().join("data"),
            screenshot_dir: dir.path().join("shots"),
        },
        text_entry,
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
fn a_game_keeps_its_save_folder_across_starts(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.view.update_in(h.cx, |v, window, cx| {
        let id = v.tiles[0].id;
        v.close_tile(id, window, cx);
    });
    h.launch_first();
    let (first, again) = (h.launcher.request(0), h.launcher.request(1));
    assert_eq!(first.data_dir, again.data_dir, "the first copy's saves");
    let data = h.dir.path().join("data");
    assert!(
        first.data_dir.starts_with(&data),
        "under the app's data folder"
    );

    // The next time the app opens, the entry and its folder are the same.
    let dir = h.dir;
    let mut reopened = open_in(cx, dir);
    reopened.launch_first();
    assert_eq!(reopened.launcher.request(0).data_dir, first.data_dir);
}

#[gpui::test]
fn starting_a_game_on_a_narrow_window_hides_the_library(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    assert!(h.read(|v| v.sidebar_open), "a wide window keeps it");
    h.cx.simulate_resize(gpui::size(gpui::px(400.0), gpui::px(800.0)));
    h.launch_first();
    assert!(!h.read(|v| v.sidebar_open), "a phone shows the game");
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
        .update(h.cx, |v, _| v.tiles[0].session.toggle_typing());
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
fn typing_follows_the_game_text_box(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["chat.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    *lock(&game.wants_text) = true;
    h.redraw();
    h.key_down("h", Some("h"), false);
    h.key_down("5", Some("5"), false);
    // The user wants keys while the text box shows.
    h.view
        .update(h.cx, |v, _| v.tiles[0].session.toggle_typing());
    h.key_down("5", Some("5"), false);
    h.key_up("5");
    // The text box closes and opens again: typing follows the game again.
    *lock(&game.wants_text) = false;
    h.redraw();
    *lock(&game.wants_text) = true;
    h.redraw();
    h.key_down("i", Some("i"), false);
    assert_eq!(
        game.events(),
        vec![
            InputEvent::Text("h".into()),
            InputEvent::Text("5".into()),
            key(Key::Num5, KeyAction::Press),
            key(Key::Num5, KeyAction::Release),
            InputEvent::Text("i".into()),
        ]
    );
}

#[gpui::test]
fn the_platform_text_entry_types_into_the_game(cx: &mut TestAppContext) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let entry: TextEntry = {
        let asked = Arc::clone(&asked);
        Arc::new(move |title: &str| {
            lock(&asked).push(title.to_owned());
            let (answer, receiver) = oneshot::channel();
            answer.send(Some("hello".to_owned())).ok();
            receiver
        })
    };
    let dir = tempfile::tempdir().expect("temp dir");
    let mut h = open_with(cx, dir, Some(entry));
    h.add(&["chat.jar"]);
    h.launch_first();
    h.redraw();
    assert!(lock(&asked).is_empty(), "no text box, no keyboard");
    let game = h.launcher.instance(0);
    *lock(&game.wants_text) = true;
    h.redraw();
    h.redraw();
    assert_eq!(*lock(&asked), ["CHAT"], "asked once per text box");
    assert_eq!(game.events(), vec![InputEvent::Text("hello".into())]);
    assert!(
        !h.read(|v| v.tiles[0].session.typing_text()),
        "the answer hands the game back to keys"
    );
    h.view
        .update(h.cx, |v, _| v.tiles[0].session.toggle_typing());
    h.redraw();
    assert_eq!(lock(&asked).len(), 2, "one press of Text asks again");
    assert_eq!(
        game.events(),
        vec![
            InputEvent::Text("hello".into()),
            InputEvent::Text("hello".into())
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
fn a_pointer_held_down_is_released_when_the_game_loses_focus(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.launch_first();
    let first = h.launcher.instance(0);
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.focus_tile(id, cx);
        v.tiles[0].session.pointer_press((10, 20));
        v.tiles[0].session.pointer_drag((12, 20));
        v.cycle_focus(1, cx);
    });
    let pointer = |action, x, y| InputEvent::Pointer { action, x, y };
    assert_eq!(
        first.events(),
        vec![
            pointer(PointerAction::Press, 10, 20),
            pointer(PointerAction::Drag, 12, 20),
            pointer(PointerAction::Release, 12, 20),
        ]
    );
    h.view.update(h.cx, |v, _| {
        assert!(!v.tiles[0].session.is_pointer_down());
        v.tiles[1].session.pointer_press((1, 1));
    });
    let second = h.launcher.instance(1);
    h.cx.update(|window, _| window.activate_window());
    h.cx.run_until_parked();
    h.cx.deactivate_window();
    h.cx.run_until_parked();
    assert_eq!(
        second.events().last(),
        Some(&pointer(PointerAction::Release, 1, 1)),
        "the window going to the background lifts the pointer"
    );
}

/// A finger at `phase` on `at`, having started at `start`.
fn finger(
    phase: gpui::TouchPhase,
    start: gpui::Point<gpui::Pixels>,
    at: gpui::Point<gpui::Pixels>,
) -> gpui::TouchDragEvent {
    gpui::TouchDragEvent {
        phase,
        start_position: start,
        position: at,
    }
}

#[gpui::test]
fn fingers_hold_keypad_keys_and_drag_on_screens(cx: &mut TestAppContext) {
    use gpui::{TouchPhase, point, px};

    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    game.show(1);
    h.redraw();
    let key5 = h.cx.debug_bounds("key-Num5").expect("the 5 key is drawn");
    let center = key5.center();
    h.cx.simulate_event(finger(TouchPhase::Started, center, center));
    assert_eq!(
        game.events(),
        vec![key(Key::Num5, KeyAction::Press)],
        "the key goes down with the finger"
    );
    h.cx.simulate_event(finger(TouchPhase::Ended, center, center));
    assert_eq!(game.events().len(), 2);

    let tile = h.read(|v| v.tiles[0].id.0);
    let selector: &'static str = Box::leak(format!("screen-{tile}").into_boxed_str());
    let screen = h.cx.debug_bounds(selector).expect("the screen is drawn");
    let start = screen.origin + point(px(1.0), px(1.0));
    let moved = start + point(px(20.0), px(0.0));
    h.cx.simulate_event(finger(TouchPhase::Started, start, start));
    h.cx.simulate_event(finger(TouchPhase::Moved, start, moved));
    h.cx.simulate_event(finger(TouchPhase::Ended, start, moved));
    let actions: Vec<PointerAction> = game
        .events()
        .into_iter()
        .filter_map(|e| match e {
            InputEvent::Pointer { action, .. } => Some(action),
            _ => None,
        })
        .collect();
    assert_eq!(
        actions,
        [
            PointerAction::Press,
            PointerAction::Drag,
            PointerAction::Release
        ]
    );
}

#[gpui::test]
fn the_keypad_and_the_keyboard_hold_keys_together(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    let game = h.launcher.instance(0);
    h.key_down("left", None, false);
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.keypad_press(id, Key::Left, cx);
        v.keypad_release(id, cx);
    });
    assert_eq!(
        game.events(),
        vec![key(Key::Left, KeyAction::Press)],
        "the keyboard still holds Left"
    );
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.keypad_press(id, Key::Fire, cx);
    });
    h.key_down("enter", None, false);
    h.key_up("enter");
    h.key_up("left");
    h.view.update(h.cx, |v, cx| {
        let id = v.tiles[0].id;
        v.keypad_release(id, cx);
    });
    assert_eq!(
        game.events(),
        vec![
            key(Key::Left, KeyAction::Press),
            key(Key::Fire, KeyAction::Press),
            key(Key::Left, KeyAction::Release),
            key(Key::Fire, KeyAction::Release),
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
fn log_lines_arrive_through_notify_while_nothing_animates(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.view.update(h.cx, |v, cx| {
        v.tiles[0].session.toggle_pause();
        v.toggle_log(cx);
    });
    h.redraw();
    assert!(!h.read(|v| v.animating), "a paused game asks for no frames");
    let game = h.launcher.instance(0);
    lock(&game.log).push(LogLine {
        level: LogLevel::Out,
        text: "saved".into(),
    });
    let notify = h.launcher.request(0).notify.expect("a notify callback");
    notify(game.id());
    h.cx.run_until_parked();
    assert_eq!(
        h.read(|v| v.tiles[0].session.log.len()),
        2,
        "the callback alone brings the line in"
    );
}

#[gpui::test]
fn the_log_follows_new_lines_until_scrolled_up(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.view.update(h.cx, FastEmuView::toggle_log);
    let game = h.launcher.instance(0);
    let write = |h: &mut Harness<'_>, count: usize| {
        lock(&game.log).extend((0..count).map(|i| LogLine {
            level: LogLevel::Out,
            text: format!("line {i}"),
        }));
        h.redraw();
        h.redraw();
    };
    write(&mut h, 50);
    let at_end = |h: &mut Harness<'_>| h.read(|v| v.log_scroll.is_scrolled_to_end());
    assert_eq!(at_end(&mut h), Some(true), "the log follows new lines");
    h.view.update(h.cx, |v, _| {
        v.log_scroll.scroll_to_item(0, ScrollStrategy::Top);
    });
    h.redraw();
    write(&mut h, 5);
    assert_eq!(at_end(&mut h), Some(false), "reading older lines stays put");
    h.view.update(h.cx, |v, _| v.log_scroll.scroll_to_bottom());
    h.redraw();
    write(&mut h, 5);
    assert_eq!(
        at_end(&mut h),
        Some(true),
        "back at the end, it follows again"
    );
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
fn a_game_that_cannot_start_shows_why_in_its_tile(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["fails.jar"]);
    h.launch_first();
    h.redraw();
    let (count, title, status) = h.read(|v| {
        (
            v.tiles.len(),
            v.tiles[0].session.title.clone(),
            v.tiles[0].session.status.clone(),
        )
    });
    assert_eq!(count, 1, "the failure gets a tile");
    assert_eq!(title, "FAILS");
    assert_eq!(
        status,
        InstanceStatus::Crashed {
            message: "unsupported: JSR-184 3D graphics".into()
        }
    );
    // Trying again keeps the tile and its message.
    h.view.update_in(h.cx, |v, window, cx| {
        let id = v.tiles[0].id;
        v.restart_tile(id, window, cx);
    });
    h.cx.run_until_parked();
    assert_eq!(h.read(|v| v.tiles.len()), 1);
    assert!(h.read(|v| matches!(v.tiles[0].session.status, InstanceStatus::Crashed { .. })));
}

#[gpui::test]
fn the_background_pauses_running_games_only(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.launch_first();
    let (playing, paused) = (h.launcher.instance(0), h.launcher.instance(1));
    h.view.update(h.cx, |v, cx| {
        v.tiles[1].session.toggle_pause();
        v.set_background(true, cx);
    });
    assert_eq!(playing.status(), InstanceStatus::Paused);
    h.view.update(h.cx, |v, cx| v.set_background(false, cx));
    assert_eq!(playing.status(), InstanceStatus::Running);
    assert_eq!(
        paused.status(),
        InstanceStatus::Paused,
        "the game the user paused stays paused"
    );
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
fn a_game_that_starts_after_the_window_closed_is_stopped(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.view.update_in(h.cx, |v, window, cx| {
        let id = v.library.entries()[0].id;
        v.launch(id, window, cx);
        window.remove_window();
    });
    h.cx.run_until_parked();
    assert_eq!(h.launcher.count(), 1, "the launch went ahead");
    assert_eq!(
        h.launcher.instance(0).status(),
        InstanceStatus::Exited,
        "nothing shows it, so it must not keep running"
    );
}

#[gpui::test]
fn a_restart_that_ends_after_the_window_closed_is_stopped(cx: &mut TestAppContext) {
    let mut h = open(cx);
    h.add(&["snake.jar"]);
    h.launch_first();
    h.view.update_in(h.cx, |v, window, cx| {
        let id = v.tiles[0].id;
        v.restart_tile(id, window, cx);
        window.remove_window();
    });
    h.cx.run_until_parked();
    assert_eq!(h.launcher.count(), 2, "the restart went ahead");
    assert_eq!(h.launcher.instance(1).status(), InstanceStatus::Exited);
}

#[gpui::test]
fn entries_removed_during_a_library_scan_stay_gone(cx: &mut TestAppContext) {
    let mut h = open(cx);
    let path = h.dir.path().join("broken.jar");
    let id = h.view.update_in(h.cx, |v, window, cx| {
        let id = v.library.add(path, &SuiteInfo::default());
        // The scan reads every entry in the background; the entry goes
        // before its result, a read error, comes back.
        v.refresh_library(window, cx);
        v.remove_entry(id, window, cx);
        id
    });
    h.cx.run_until_parked();
    assert!(h.read(|v| v.library.entries().is_empty()));
    assert!(
        !h.read(|v| v.broken.contains_key(&id) || v.icons.contains_key(&id)),
        "the scan's late result must not bring back state for the entry"
    );
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
            s.touch = true;
            "windows-1251".clone_into(&mut s.encoding);
            s.step_volume(-0.5);
        });
    });
    h.launch_first();
    let request = h.launcher.request(0);
    assert_eq!(request.profile.screen_width, 128);
    assert!(
        request.profile.touch,
        "the touch switch gives a keypad phone a touch screen"
    );
    assert_eq!(request.settings.encoding, "windows-1251");
    assert_eq!(request.settings.network, fastemu_core::NetworkPolicy::Allow);
    assert!((request.settings.volume - 0.5).abs() < 1e-6);
    let saved = Store::new(h.dir.path().join("config")).load_library().value;
    assert_eq!(saved.entries()[0].settings.encoding, "windows-1251");
}
