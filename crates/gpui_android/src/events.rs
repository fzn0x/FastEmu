//! Android input to gpui `PlatformInput` translation.
//!
//! Modified for FastEmu: trackpad gestures, which the port's Activity sent
//! over JNI with their classification, are gone; a trackpad reaches gpui as
//! a mouse.
//!
//! Android's input stack does the gesture work, so this only passes its events
//! on. A finger on the touchscreen goes to gpui's touch recognizer through
//! [`crate::touch`]. A mouse and a stylus, which Android turns into a pointer
//! with button presses, go through [`mouse`]. Hardware keys are in
//! [`keyboard`].

pub(crate) mod keyboard;
pub(crate) mod mouse;

use android_activity::input::{Axis, MetaState, MotionAction, MotionEvent, Source, ToolType};
use gpui::{Pixels, PlatformInput, Point, point, px};

pub(crate) use keyboard::translate_key_event;

use crate::window::AndroidWindowState;

/// `MotionEvent.FLAG_CANCELED`: the pointer going up was cancelled, such as a
/// palm the system rejected, so it must not act as a release.
pub(crate) const FLAG_CANCELED: u32 = 0x20;

/// `InputDevice.SOURCE_TOUCHSCREEN`: a class bit set plus a device bit. An
/// event's source is a union of these (an S Pen on the screen reports
/// `SOURCE_TOUCHSCREEN | SOURCE_STYLUS`), so a source is tested by its bits,
/// never by equality.
const SOURCE_TOUCHSCREEN: u32 = 0x1002;

/// One Android `MotionEvent` from android-activity's input queue.
pub(crate) struct Motion {
    pub action: MotionAction,
    /// In `pointers`, the pointer that `PointerDown` or `PointerUp` is about.
    pub action_index: usize,
    pub source: Source,
    pub meta_state: MetaState,
    /// `MotionEvent.getButtonState()`.
    pub button_state: u32,
    /// `MotionEvent.FLAG_CANCELED`.
    pub canceled: bool,
    /// `AXIS_VSCROLL` and `AXIS_HSCROLL`, for `ACTION_SCROLL`.
    pub vscroll: f32,
    pub hscroll: f32,
    pub pointers: Vec<MotionPointer>,
}

pub(crate) struct MotionPointer {
    pub id: i32,
    pub position: Point<Pixels>,
    pub tool: ToolType,
}

impl Motion {
    pub(crate) fn from_event(event: &MotionEvent<'_>) -> Self {
        let pointers = (0..event.pointer_count())
            .map(|index| {
                let pointer = event.pointer_at_index(index);
                MotionPointer {
                    id: pointer.pointer_id(),
                    position: point(px(pointer.x()), px(pointer.y())),
                    tool: pointer.tool_type(),
                }
            })
            .collect();
        let (vscroll, hscroll) = if event.pointer_count() > 0 {
            let pointer = event.pointer_at_index(0);
            (
                pointer.axis_value(Axis::Vscroll),
                pointer.axis_value(Axis::Hscroll),
            )
        } else {
            (0.0, 0.0)
        };
        Self {
            action: event.action(),
            action_index: event.pointer_index(),
            source: event.source(),
            meta_state: event.meta_state(),
            button_state: event.button_state().0,
            canceled: event.flags().0 & FLAG_CANCELED != 0,
            vscroll,
            hscroll,
            pointers,
        }
    }

    fn has_source(&self, source: u32) -> bool {
        u32::from(self.source) & source == source
    }

    /// Whether this is a finger on the touchscreen. A trackpad's presses report a
    /// finger as their tool too, with the mouse's source.
    fn is_touchscreen_finger(&self) -> bool {
        self.has_source(SOURCE_TOUCHSCREEN)
            && self.pointers.first().is_some_and(|pointer| {
                !matches!(
                    pointer.tool,
                    ToolType::Mouse | ToolType::Stylus | ToolType::Eraser
                )
            })
    }
}

/// What [`translate_motion`] made of an event.
#[derive(Default)]
pub(crate) struct TranslatedMotion {
    pub(crate) inputs: Vec<PlatformInput>,
    /// Whether the event is consumed. Events fed to gpui's touch recognizer
    /// are, even when they produce no input yet.
    pub(crate) handled: bool,
}

/// Translates `motion`, whose positions Android reports in physical pixels, into
/// the logical pixels gpui expects.
pub(crate) fn translate_motion(
    mut motion: Motion,
    scale_factor: f32,
    state: &mut AndroidWindowState,
) -> TranslatedMotion {
    for pointer in &mut motion.pointers {
        pointer.position = point(
            pointer.position.x / scale_factor,
            pointer.position.y / scale_factor,
        );
    }
    if motion.is_touchscreen_finger() {
        return TranslatedMotion {
            inputs: state.touch_feed.on_event(&motion),
            handled: true,
        };
    }
    let inputs = Vec::from_iter(mouse::translate(&motion, &mut state.mouse));
    TranslatedMotion {
        handled: !inputs.is_empty(),
        inputs,
    }
}
