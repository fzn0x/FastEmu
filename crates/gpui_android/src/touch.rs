//! Finger input for gpui.
//!
//! [`TouchFeed`] passes each finger on the touchscreen to gpui as raw
//! `PlatformInput::Touch` events, and gpui's recognizer (`gpui::gestures`) turns
//! them into taps, pans with momentum, long presses and claimed touch drags.
//! That recognizer follows one touch at a time.
//!
//! Modified for FastEmu: the port's two-finger gestures (a secondary click
//! and a pan at the fingers' centroid) are gone. A second finger cancelled
//! the first one's touch there, which would release a held phone key, so
//! here every finger is its own touch and gpui ignores the extra ones.

use std::collections::HashMap;

use android_activity::input::{MotionAction, ToolType};
use gpui::{Pixels, PlatformInput, Point, TouchId, TouchPhase};

use crate::events::{Motion, MotionPointer};

/// A touch gpui has seen start and not yet end.
struct OpenTouch {
    id: TouchId,
    position: Point<Pixels>,
}

/// Feeds the window's finger input to gpui's touch gesture recognizer.
#[derive(Default)]
pub(crate) struct TouchFeed {
    /// gpui forbids reusing a `TouchId`, and Android numbers pointers from
    /// 0 again in every gesture, so ids come from a counter that only grows.
    next_touch_id: u64,
    /// By Android pointer id.
    open_touches: HashMap<i32, OpenTouch>,
}

impl TouchFeed {
    pub(crate) fn on_event(&mut self, event: &Motion) -> Vec<PlatformInput> {
        let mut out = Vec::new();
        match event.action {
            MotionAction::Down => {
                self.cancel_all(&mut out);
                if let Some(pointer) = event.pointers.first()
                    && !is_palm(pointer)
                {
                    self.start(pointer, &mut out);
                }
            }
            MotionAction::PointerDown => {
                self.reject_palms(event, &mut out);
                if let Some(pointer) = event.pointers.get(event.action_index)
                    && !is_palm(pointer)
                {
                    self.start(pointer, &mut out);
                }
            }
            MotionAction::Move => {
                self.reject_palms(event, &mut out);
                self.pointers_moved(event, &mut out);
            }
            MotionAction::PointerUp => {
                self.reject_palms(event, &mut out);
                if let Some(pointer) = event.pointers.get(event.action_index) {
                    self.pointer_up(pointer, event.canceled, &mut out);
                }
            }
            MotionAction::Up => {
                self.reject_palms(event, &mut out);
                if let Some(pointer) = event.pointers.first() {
                    self.pointer_up(pointer, event.canceled, &mut out);
                }
                self.cancel_all(&mut out);
            }
            MotionAction::Cancel => self.cancel_all(&mut out),
            _ => {}
        }
        out
    }

    fn start(&mut self, pointer: &MotionPointer, out: &mut Vec<PlatformInput>) {
        if let Some(touch) = self.open_touches.remove(&pointer.id) {
            out.push(touch_input(touch.id, TouchPhase::Cancelled, touch.position));
        }
        let Some(id) = self.allocate_touch_id() else {
            return;
        };
        self.open_touches.insert(
            pointer.id,
            OpenTouch {
                id,
                position: pointer.position,
            },
        );
        out.push(touch_input(id, TouchPhase::Started, pointer.position));
    }

    fn allocate_touch_id(&mut self) -> Option<TouchId> {
        let id = TouchId(self.next_touch_id);
        let Some(next_touch_id) = self.next_touch_id.checked_add(1) else {
            log::error!("touch: exhausted touch identifiers");
            return None;
        };
        self.next_touch_id = next_touch_id;
        Some(id)
    }

    fn pointers_moved(&mut self, event: &Motion, out: &mut Vec<PlatformInput>) {
        for pointer in &event.pointers {
            let Some(touch) = self.open_touches.get_mut(&pointer.id) else {
                continue;
            };
            if touch.position == pointer.position {
                continue;
            }
            touch.position = pointer.position;
            out.push(touch_input(touch.id, TouchPhase::Moved, pointer.position));
        }
    }

    fn pointer_up(
        &mut self,
        pointer: &MotionPointer,
        canceled: bool,
        out: &mut Vec<PlatformInput>,
    ) {
        if let Some(touch) = self.open_touches.remove(&pointer.id) {
            let phase = if canceled {
                TouchPhase::Cancelled
            } else {
                TouchPhase::Ended
            };
            out.push(touch_input(touch.id, phase, pointer.position));
        }
    }

    /// Palms reach gpui as cancelled touches.
    fn reject_palms(&mut self, event: &Motion, out: &mut Vec<PlatformInput>) {
        for pointer in event.pointers.iter().filter(|pointer| is_palm(pointer)) {
            if let Some(touch) = self.open_touches.remove(&pointer.id) {
                out.push(touch_input(
                    touch.id,
                    TouchPhase::Cancelled,
                    pointer.position,
                ));
            }
        }
    }

    fn cancel_all(&mut self, out: &mut Vec<PlatformInput>) {
        for (_, touch) in self.open_touches.drain() {
            out.push(touch_input(touch.id, TouchPhase::Cancelled, touch.position));
        }
    }
}

/// `MotionEvent.TOOL_TYPE_PALM`: the system decided this contact is a palm resting
/// on the screen.
fn is_palm(pointer: &MotionPointer) -> bool {
    pointer.tool == ToolType::Palm
}

fn touch_input(id: TouchId, phase: TouchPhase, position: Point<Pixels>) -> PlatformInput {
    PlatformInput::Touch(gpui::TouchEvent {
        id,
        phase,
        position,
        predicted_position: None,
        force: None,
    })
}

#[cfg(test)]
mod tests {
    use android_activity::input::{MetaState, Source};
    use gpui::{point, px};

    use super::*;

    #[derive(Debug, PartialEq)]
    enum Output {
        Touch(u64, TouchPhase, (f32, f32)),
    }

    fn outputs(inputs: Vec<PlatformInput>) -> Vec<Output> {
        let coordinates = |position: Point<Pixels>| (f32::from(position.x), f32::from(position.y));
        inputs
            .into_iter()
            .map(|input| match input {
                PlatformInput::Touch(touch) => {
                    Output::Touch(touch.id.0, touch.phase, coordinates(touch.position))
                }
                other => panic!("unexpected input {other:?}"),
            })
            .collect()
    }

    fn finger(id: i32, x: f32, y: f32) -> MotionPointer {
        MotionPointer {
            id,
            position: point(px(x), px(y)),
            tool: ToolType::Finger,
        }
    }

    fn palm(id: i32, x: f32, y: f32) -> MotionPointer {
        MotionPointer {
            tool: ToolType::Palm,
            ..finger(id, x, y)
        }
    }

    fn event_at(action: MotionAction, action_index: usize, pointers: Vec<MotionPointer>) -> Motion {
        Motion {
            action,
            action_index,
            source: Source::Touchscreen,
            meta_state: MetaState(0),
            button_state: 0,
            canceled: false,
            vscroll: 0.0,
            hscroll: 0.0,
            pointers,
        }
    }

    fn event(action: MotionAction, pointers: Vec<MotionPointer>) -> Motion {
        event_at(action, 0, pointers)
    }

    #[test]
    fn touch_ids_are_never_reused() {
        let mut feed = TouchFeed::default();
        let down = event(MotionAction::Down, vec![finger(0, 10.0, 10.0)]);
        let up = event(MotionAction::Up, vec![finger(0, 10.0, 10.0)]);
        assert_eq!(
            outputs(feed.on_event(&down)),
            [Output::Touch(0, TouchPhase::Started, (10.0, 10.0))]
        );
        assert_eq!(
            outputs(feed.on_event(&up)),
            [Output::Touch(0, TouchPhase::Ended, (10.0, 10.0))]
        );
        assert_eq!(
            outputs(feed.on_event(&down)),
            [Output::Touch(1, TouchPhase::Started, (10.0, 10.0))]
        );
    }

    #[test]
    fn down_cancels_a_touch_left_open() {
        let mut feed = TouchFeed::default();
        feed.on_event(&event(MotionAction::Down, vec![finger(0, 10.0, 10.0)]));
        assert_eq!(
            outputs(feed.on_event(&event(MotionAction::Down, vec![finger(0, 30.0, 30.0)]))),
            [
                Output::Touch(0, TouchPhase::Cancelled, (10.0, 10.0)),
                Output::Touch(1, TouchPhase::Started, (30.0, 30.0)),
            ]
        );
    }

    #[test]
    fn canceled_lift_and_palm_cancel_the_touch() {
        let mut feed = TouchFeed::default();
        feed.on_event(&event(MotionAction::Down, vec![finger(0, 10.0, 10.0)]));
        let mut up = event(MotionAction::Up, vec![finger(0, 10.0, 10.0)]);
        up.canceled = true;
        assert_eq!(
            outputs(feed.on_event(&up)),
            [Output::Touch(0, TouchPhase::Cancelled, (10.0, 10.0))]
        );

        feed.on_event(&event(MotionAction::Down, vec![finger(0, 10.0, 10.0)]));
        let moved = event(MotionAction::Move, vec![palm(0, 12.0, 10.0)]);
        assert_eq!(
            outputs(feed.on_event(&moved)),
            [Output::Touch(1, TouchPhase::Cancelled, (12.0, 10.0))]
        );
        let up = event(MotionAction::Up, vec![palm(0, 12.0, 10.0)]);
        assert!(outputs(feed.on_event(&up)).is_empty());
    }

    #[test]
    fn a_second_finger_leaves_the_first_touch_alone() {
        let mut feed = TouchFeed::default();
        feed.on_event(&event(MotionAction::Down, vec![finger(0, 100.0, 100.0)]));
        let second_down = event_at(
            MotionAction::PointerDown,
            1,
            vec![finger(0, 100.0, 100.0), finger(1, 200.0, 100.0)],
        );
        assert_eq!(
            outputs(feed.on_event(&second_down)),
            [Output::Touch(1, TouchPhase::Started, (200.0, 100.0))]
        );
        let second_up = event_at(
            MotionAction::PointerUp,
            1,
            vec![finger(0, 100.0, 100.0), finger(1, 200.0, 100.0)],
        );
        assert_eq!(
            outputs(feed.on_event(&second_up)),
            [Output::Touch(1, TouchPhase::Ended, (200.0, 100.0))]
        );
        let last_up = event(MotionAction::Up, vec![finger(0, 100.0, 100.0)]);
        assert_eq!(
            outputs(feed.on_event(&last_up)),
            [Output::Touch(0, TouchPhase::Ended, (100.0, 100.0))]
        );
    }
}
