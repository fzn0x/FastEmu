//! The `--keys` script: key taps and holds, pointer presses, drags and
//! releases, and waits, played against the clock from the first frame.

use std::time::{Duration, Instant};

use fastemu_core::{InputEvent, Instance, Key, KeyAction, PointerAction};

/// How long a key or a pointer tap is held when the script gives no time.
pub const TAP: Duration = Duration::from_millis(100);

/// One step of a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Press(Key),
    Release(Key),
    /// A touch or click at phone pixel (`x`, `y`).
    Pointer {
        action: PointerAction,
        x: i32,
        y: i32,
    },
    Wait(Duration),
}

fn key_named(name: &str) -> Option<Key> {
    let key = match name.to_ascii_lowercase().as_str() {
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "star" | "*" => Key::Star,
        "pound" | "#" => Key::Pound,
        "up" => Key::Up,
        "down" => Key::Down,
        "left" => Key::Left,
        "right" => Key::Right,
        "fire" => Key::Fire,
        "softleft" => Key::SoftLeft,
        "softright" => Key::SoftRight,
        "clear" => Key::Clear,
        "send" => Key::Send,
        "end" => Key::End,
        _ => return None,
    };
    Some(key)
}

fn millis(text: &str) -> Result<Duration, String> {
    text.parse::<u64>()
        .map(Duration::from_millis)
        .map_err(|_| format!("{text:?} is not a number of milliseconds"))
}

/// Parses `XxY`, a phone pixel such as `120x160`.
fn pixel(text: &str) -> Result<(i32, i32), String> {
    let bad = || format!("{text:?} is not a screen position such as 120x160");
    let (x, y) = text.split_once(['x', 'X']).ok_or_else(bad)?;
    let coordinate = |c: &str| c.parse::<u16>().map(i32::from).map_err(|_| bad());
    Ok((coordinate(x)?, coordinate(y)?))
}

/// Parses a pointer step, `NAME@XxY` with `tap@XxY:MS` holding the touch
/// for MS milliseconds, into `steps`.
fn pointer_step(name: &str, place: &str, steps: &mut Vec<Step>) -> Result<(), String> {
    let (place, hold) = match place.split_once(':') {
        Some((place, hold)) => (place, Some(millis(hold)?)),
        None => (place, None),
    };
    let (x, y) = pixel(place)?;
    let at = |action| Step::Pointer { action, x, y };
    match (name.to_ascii_lowercase().as_str(), hold) {
        ("tap", hold) => steps.extend([
            at(PointerAction::Press),
            Step::Wait(hold.unwrap_or(TAP)),
            at(PointerAction::Release),
        ]),
        ("press", None) => steps.push(at(PointerAction::Press)),
        ("drag", None) => steps.push(at(PointerAction::Drag)),
        ("release", None) => steps.push(at(PointerAction::Release)),
        ("press" | "drag" | "release", Some(_)) => {
            return Err(format!("only tap takes a time, not {name}@{place}"));
        }
        _ => return Err(format!("unknown pointer step {name:?} in the key script")),
    }
    Ok(())
}

/// Parses a script into steps.
pub fn parse(script: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    let words = script
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .flat_map(|line| line.split([',', ' ', '\t']))
        .filter(|word| !word.is_empty());
    for word in words {
        if let Some((name, place)) = word.split_once('@') {
            pointer_step(name, place, &mut steps)?;
            continue;
        }
        let (name, time) = match word.split_once(':') {
            Some((name, time)) => (name, Some(time)),
            None => (word, None),
        };
        if name.eq_ignore_ascii_case("wait") {
            steps.push(Step::Wait(millis(time.unwrap_or_default())?));
            continue;
        }
        if time.is_none() && name.bytes().all(|b| b.is_ascii_digit()) && name.len() > 1 {
            steps.push(Step::Wait(millis(name)?));
            continue;
        }
        let key =
            key_named(name).ok_or_else(|| format!("unknown key {name:?} in the key script"))?;
        let hold = match time {
            Some(time) => millis(time)?,
            None => TAP,
        };
        steps.extend([Step::Press(key), Step::Wait(hold), Step::Release(key)]);
    }
    Ok(steps)
}

/// Plays a script against the clock.
pub struct Script {
    steps: Vec<Step>,
    next: usize,
    due: Option<Instant>,
}

impl Script {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            next: 0,
            due: None,
        }
    }

    /// Sends every step that is due at `now`. The first call starts the
    /// clock.
    pub fn advance(&mut self, instance: &dyn Instance, now: Instant) {
        let mut due = *self.due.get_or_insert(now);
        while self.next < self.steps.len() && now >= due {
            match self.steps[self.next] {
                Step::Press(key) => instance.send(InputEvent::Key {
                    key,
                    action: KeyAction::Press,
                }),
                Step::Release(key) => instance.send(InputEvent::Key {
                    key,
                    action: KeyAction::Release,
                }),
                Step::Pointer { action, x, y } => {
                    instance.send(InputEvent::Pointer { action, x, y });
                }
                Step::Wait(d) => due += d,
            }
            self.next += 1;
        }
        self.due = Some(due);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_tap_hold_and_wait() {
        let steps = parse("fire, wait:500 down:300\n# a comment\n250 5").unwrap();
        assert_eq!(
            steps,
            vec![
                Step::Press(Key::Fire),
                Step::Wait(TAP),
                Step::Release(Key::Fire),
                Step::Wait(Duration::from_millis(500)),
                Step::Press(Key::Down),
                Step::Wait(Duration::from_millis(300)),
                Step::Release(Key::Down),
                Step::Wait(Duration::from_millis(250)),
                Step::Press(Key::Num5),
                Step::Wait(TAP),
                Step::Release(Key::Num5),
            ]
        );
        assert!(parse("jump").is_err());
        assert!(parse("fire:soon").is_err());
    }

    #[test]
    fn pointers_press_drag_and_release() {
        let at = |action, x, y| Step::Pointer { action, x, y };
        let steps = parse("press@10x20 drag@15x20,release@15x25 tap@3X4 tap@0x0:400").unwrap();
        assert_eq!(
            steps,
            vec![
                at(PointerAction::Press, 10, 20),
                at(PointerAction::Drag, 15, 20),
                at(PointerAction::Release, 15, 25),
                at(PointerAction::Press, 3, 4),
                Step::Wait(TAP),
                at(PointerAction::Release, 3, 4),
                at(PointerAction::Press, 0, 0),
                Step::Wait(Duration::from_millis(400)),
                at(PointerAction::Release, 0, 0),
            ]
        );
        for bad in [
            "press@10",
            "press@-1x2",
            "drag@1x2:300",
            "poke@1x2",
            "tap@ax1",
        ] {
            assert!(parse(bad).is_err(), "{bad} parsed");
        }
    }

    #[test]
    fn steps_wait_for_their_time() {
        use std::sync::Mutex;

        use fastemu_core::{Frame, InstanceId, InstanceStatus, LogLine};

        #[derive(Default)]
        struct Recorder(Mutex<Vec<InputEvent>>);

        impl Instance for Recorder {
            fn id(&self) -> InstanceId {
                InstanceId(0)
            }
            fn title(&self) -> String {
                String::new()
            }
            fn status(&self) -> InstanceStatus {
                InstanceStatus::Running
            }
            fn frame(&self) -> Option<Frame> {
                None
            }
            fn send(&self, event: InputEvent) {
                self.0.lock().unwrap().push(event);
            }
            fn pause(&self) {}
            fn resume(&self) {}
            fn stop(&self) {}
            fn set_volume(&self, _: f32) {}
            fn drain_log(&self) -> Vec<LogLine> {
                Vec::new()
            }
        }

        let game = Recorder::default();
        let mut script = Script::new(parse("tap@1x2:50 up").unwrap());
        let start = Instant::now();
        script.advance(&game, start);
        assert_eq!(game.0.lock().unwrap().len(), 1, "only the press is due");
        script.advance(&game, start + Duration::from_millis(49));
        assert_eq!(game.0.lock().unwrap().len(), 1);
        script.advance(&game, start + Duration::from_millis(50));
        assert_eq!(game.0.lock().unwrap().len(), 3, "release, then up");
        script.advance(&game, start + Duration::from_secs(1));
        assert_eq!(
            game.0.lock().unwrap().last(),
            Some(&InputEvent::Key {
                key: Key::Up,
                action: KeyAction::Release
            })
        );
    }
}
