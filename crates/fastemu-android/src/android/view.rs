//! The minimal emulator view: the running game's screen above an on-screen
//! phone keypad. fastemu-ui's views replace it once they build for Android.
//!
//! Keys respond to a press as a phone's do: a finger or the mouse presses a
//! key when it goes down and releases it when it lifts, so games see held
//! keys. GPUI follows one finger at a time, so the keypad does too.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use fastemu_core::{InstanceStatus, Key, PointerAction};
use gpui::{
    Bounds, Context, Corners, DispatchPhase, FocusHandle, IntoElement, KeyDownEvent, KeyUpEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathPromptOptions, Pixels, Point,
    Render, SharedString, Task, TouchDragEvent, TouchPhase, WeakEntity, Window, canvas, div, point,
    prelude::*, px, relative, rgb, size,
};

use super::frame_image::FrameImage;
use crate::keymap;
use crate::keypad::{self, KeypadKey};
use crate::screen::{self, Placement};
use crate::session::{RedrawSignal, Session};

const BACKGROUND: u32 = 0x0010_1216;
const PANEL: u32 = 0x001c_1f26;
const KEY: u32 = 0x002b_2f3a;
const KEY_HELD: u32 = 0x004c_7df0;
const TEXT: u32 = 0x00e6_e8ee;
const MUTED: u32 = 0x009a_a0ae;
const ERROR: u32 = 0x00ff_8a80;
/// Android's UI font, on every device.
const FONT: &str = "Roboto";
/// How much of the window's height the keypad takes.
const KEYPAD_SHARE: f32 = 0.42;

pub(super) struct EmulatorView {
    session: Session,
    focus: FocusHandle,
    frame_image: Rc<RefCell<FrameImage>>,
    /// The keypad key the finger on the keypad holds.
    touch_key: Option<Key>,
    /// The keypad key the mouse holds.
    mouse_key: Option<Key>,
    /// Where a finger or the mouse holds the game's screen, in phone pixels.
    pointer: Option<(i32, i32)>,
    /// Why the last game didn't open, until one does.
    error: Option<SharedString>,
    /// Holds GPUI's guard that keeps the screen on while the app is in
    /// front, for as long as the view lives.
    _awake: Task<()>,
    _redraws: Task<()>,
}

impl EmulatorView {
    pub(super) fn new(
        session: Session,
        mut redraws: RedrawSignal,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        let redraw_task = cx.spawn(async move |this, cx| {
            while redraws.changed().await {
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        });
        let awake = cx.prevent_idle_sleep("a game is on screen");
        let awake_task = cx.spawn(async move |_, _| match awake.await {
            Ok(guard) => {
                // Never finishes: the guard goes when the view drops the task.
                futures::future::pending::<()>().await;
                drop(guard);
            }
            Err(error) => log::warn!("view: the screen may turn off: {error:#}"),
        });
        Self {
            session,
            focus,
            frame_image: Rc::default(),
            touch_key: None,
            mouse_key: None,
            pointer: None,
            error: None,
            _awake: awake_task,
            _redraws: redraw_task,
        }
    }

    /// Starts the game in `suite`, replacing the one that runs.
    pub(super) fn open(&mut self, suite: &Path, cx: &mut Context<'_, Self>) {
        self.forget_input();
        self.error = match self.session.open(suite) {
            Ok(()) => None,
            Err(error) => {
                log::error!("view: could not open {}: {error}", suite.display());
                let name = suite.file_name().unwrap_or(suite.as_os_str());
                Some(format!("Could not open {}: {error}", name.to_string_lossy()).into())
            }
        };
        cx.notify();
    }

    pub(super) fn pause(&mut self, cx: &mut Context<'_, Self>) {
        self.forget_input();
        self.session.pause();
        cx.notify();
    }

    pub(super) fn resume(&mut self, cx: &mut Context<'_, Self>) {
        self.session.resume();
        cx.notify();
    }

    /// Drops the presses whose releases won't arrive; the session releases
    /// the keys themselves.
    fn forget_input(&mut self) {
        self.touch_key = None;
        self.mouse_key = None;
        self.pointer = None;
    }

    fn choose_game(cx: &mut Context<'_, Self>) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open a game".into()),
        });
        cx.spawn(async move |this, cx| {
            let picked = match paths.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(error)) => {
                    log::error!("view: the file picker failed: {error:#}");
                    None
                }
            };
            if let Some(path) = picked {
                this.update(cx, |view, cx| view.open(&path, cx)).ok();
            }
        })
        .detach();
    }

    fn press(&mut self, key: Key, cx: &mut Context<'_, Self>) {
        self.session.press(key);
        cx.notify();
    }

    fn release(&mut self, key: Key, cx: &mut Context<'_, Self>) {
        self.session.release(key);
        cx.notify();
    }

    fn press_with_touch(&mut self, key: Key, cx: &mut Context<'_, Self>) {
        if let Some(previous) = self.touch_key.replace(key) {
            self.release(previous, cx);
        }
        self.press(key, cx);
    }

    fn press_with_mouse(&mut self, key: Key, cx: &mut Context<'_, Self>) {
        if let Some(previous) = self.mouse_key.replace(key) {
            self.release(previous, cx);
        }
        self.press(key, cx);
    }

    fn pointer_pressed(&mut self, at: (i32, i32)) {
        self.pointer = Some(at);
        self.session.point(PointerAction::Press, at);
    }

    fn pointer_dragged(&mut self, at: (i32, i32)) {
        if self.pointer.is_some() && self.pointer != Some(at) {
            self.pointer = Some(at);
            self.session.point(PointerAction::Drag, at);
        }
    }

    fn pointer_released(&mut self) {
        if let Some(at) = self.pointer.take() {
            self.session.point(PointerAction::Release, at);
        }
    }

    /// The finger lifted: whatever it held lets go.
    fn touch_ended(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(key) = self.touch_key.take() {
            self.release(key, cx);
        }
        self.pointer_released();
    }

    fn mouse_released(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(key) = self.mouse_key.take() {
            self.release(key, cx);
        }
        self.pointer_released();
    }

    fn header(&self, cx: &Context<'_, Self>) -> impl IntoElement {
        let (title, status) = match self.session.instance() {
            Some(instance) => (
                SharedString::from(instance.title()),
                describe(&instance.status()),
            ),
            None => ("No game".into(), String::new()),
        };
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .bg(rgb(PANEL))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(16.)).child(title))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(MUTED))
                            .child(status),
                    ),
            )
            .child(
                div()
                    .id("open-game")
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(KEY))
                    .text_size(px(14.))
                    .child("Open game")
                    .on_click(cx.listener(|_, _, _, cx| Self::choose_game(cx))),
            )
    }

    /// The game's screen at the largest whole zoom that fits. Touching or
    /// clicking it sends pointer events in phone pixels.
    fn screen(&self, cx: &Context<'_, Self>) -> impl IntoElement {
        let view = cx.entity().downgrade();
        let instance = self.session.instance().cloned();
        let frame_image = Rc::clone(&self.frame_image);
        canvas(
            |_, _, _| {},
            move |bounds, (), window, _| {
                let shown = instance
                    .as_ref()
                    .and_then(|instance| Some((instance.id(), instance.frame()?)));
                let Some((id, frame)) = shown else {
                    frame_image.borrow_mut().clear(window);
                    return;
                };
                let scale = window.scale_factor();
                let placed = screen::place(
                    (frame.width(), frame.height()),
                    (
                        f32::from(bounds.origin.x) * scale,
                        f32::from(bounds.origin.y) * scale,
                    ),
                    (
                        f32::from(bounds.size.width) * scale,
                        f32::from(bounds.size.height) * scale,
                    ),
                );
                let image = frame_image
                    .borrow_mut()
                    .image(id, &frame, placed.upscale(), window);
                if let Some(image) = image {
                    let image_bounds = logical_bounds(&placed, scale);
                    let painted = window.paint_image(
                        image_bounds,
                        image_bounds,
                        Corners::default(),
                        image,
                        0,
                        false,
                    );
                    if let Err(error) = painted {
                        log::error!("view: painting the frame: {error:#}");
                    }
                }
                listen_for_pointer(window, &view, placed, scale);
            },
        )
        .flex_1()
        .w_full()
    }

    fn keypad(&self, cx: &Context<'_, Self>) -> impl IntoElement {
        let rows: Vec<_> = keypad::ROWS
            .iter()
            .map(|row| {
                let buttons: Vec<_> = row
                    .iter()
                    .map(|button| self.key_button(*button, cx))
                    .collect();
                div().flex().flex_row().flex_1().gap_1().children(buttons)
            })
            .collect();
        div()
            .w_full()
            .h(relative(KEYPAD_SHARE))
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .bg(rgb(PANEL))
            .children(rows)
    }

    fn key_button(&self, button: KeypadKey, cx: &Context<'_, Self>) -> impl IntoElement {
        let key = button.key;
        let color = if self.session.is_held(key) {
            KEY_HELD
        } else {
            KEY
        };
        div()
            .relative()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .bg(rgb(color))
            .text_size(px(18.))
            .child(button.label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |view, _: &MouseDownEvent, _, cx| {
                    view.press_with_mouse(key, cx);
                }),
            )
            .child(touch_target(key, cx.entity().downgrade()))
    }
}

impl Render for EmulatorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let safe_area = window.insets().safe_area;
        let view = cx.entity().downgrade();
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .pt(safe_area.top)
            .pb(safe_area.bottom)
            .pl(safe_area.left)
            .pr(safe_area.right)
            .bg(rgb(BACKGROUND))
            .text_color(rgb(TEXT))
            .font_family(FONT)
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, _, cx| {
                if let Some(key) = keymap::phone_key(&event.keystroke.key) {
                    if !event.is_held {
                        view.press(key, cx);
                    }
                    cx.stop_propagation();
                }
            }))
            .on_key_up(cx.listener(|view, event: &KeyUpEvent, _, cx| {
                if let Some(key) = keymap::phone_key(&event.keystroke.key) {
                    view.release(key, cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, _: &MouseUpEvent, _, cx| view.mouse_released(cx)),
            )
            .child(self.header(cx))
            .child(self.screen(cx))
            .children(self.error.clone().map(|error| {
                div()
                    .px_3()
                    .py_1()
                    .text_size(px(13.))
                    .text_color(rgb(ERROR))
                    .child(error)
            }))
            .child(self.keypad(cx))
            .child(touch_end_listener(view))
    }
}

/// A status line for the header.
fn describe(status: &InstanceStatus) -> String {
    match status {
        InstanceStatus::Starting => "Starting".into(),
        InstanceStatus::Running => "Running".into(),
        InstanceStatus::Paused => "Paused".into(),
        InstanceStatus::Exited => "Exited".into(),
        InstanceStatus::Crashed { message } => format!("Crashed: {message}"),
    }
}

/// The placement's device pixels in GPUI's logical pixels.
fn logical_bounds(placed: &Placement, scale: f32) -> Bounds<Pixels> {
    Bounds::new(
        point(px(placed.x / scale), px(placed.y / scale)),
        size(px(placed.width / scale), px(placed.height / scale)),
    )
}

/// Turns clicks and touches on the game's screen into pointer events.
fn listen_for_pointer(
    window: &mut Window,
    view: &WeakEntity<EmulatorView>,
    placed: Placement,
    scale: f32,
) {
    let to_phone = move |position: Point<Pixels>| {
        placed.to_phone(f32::from(position.x) * scale, f32::from(position.y) * scale)
    };
    window.on_mouse_event({
        let view = view.clone();
        move |event: &MouseDownEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble
                && event.button == MouseButton::Left
                && let Some(at) = to_phone(event.position)
            {
                view.update(cx, |view, _| view.pointer_pressed(at)).ok();
            }
        }
    });
    window.on_mouse_event({
        let view = view.clone();
        move |event: &MouseMoveEvent, phase, _, cx| {
            if phase == DispatchPhase::Bubble
                && event.pressed_button == Some(MouseButton::Left)
                && let Some(at) = to_phone(event.position)
            {
                view.update(cx, |view, _| view.pointer_dragged(at)).ok();
            }
        }
    });
    window.on_mouse_event({
        let view = view.clone();
        move |event: &TouchDragEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble {
                return;
            }
            match event.phase {
                TouchPhase::Started => {
                    if let Some(at) = to_phone(event.start_position) {
                        window.prevent_default();
                        view.update(cx, |view, _| view.pointer_pressed(at)).ok();
                    }
                }
                TouchPhase::Moved => {
                    if let Some(at) = to_phone(event.position) {
                        view.update(cx, |view, _| view.pointer_dragged(at)).ok();
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {}
            }
        }
    });
}

/// Claims a touch that starts on a keypad key, so GPUI reports its release
/// instead of turning it into a tap, and presses the key.
fn touch_target(key: Key, view: WeakEntity<EmulatorView>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            window.on_mouse_event(move |event: &TouchDragEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && event.phase == TouchPhase::Started
                    && bounds.contains(&event.start_position)
                {
                    window.prevent_default();
                    view.update(cx, |view, cx| view.press_with_touch(key, cx))
                        .ok();
                }
            });
        },
    )
    .absolute()
    .size_full()
}

/// Lets go of what a finger held, on the keypad or the screen, when it lifts.
fn touch_end_listener(view: WeakEntity<EmulatorView>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            window.on_mouse_event(move |event: &TouchDragEvent, phase, _, cx| {
                if phase == DispatchPhase::Bubble
                    && matches!(event.phase, TouchPhase::Ended | TouchPhase::Cancelled)
                {
                    view.update(cx, EmulatorView::touch_ended).ok();
                }
            });
        },
    )
    .absolute()
    .size_full()
}
