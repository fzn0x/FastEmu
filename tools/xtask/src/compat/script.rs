//! Key scripts, written as `fastemu run --keys` takes them: steps
//! separated by commas or spaces; a key name taps the key, `NAME:MS`
//! holds it for MS milliseconds, and `wait:MS` waits.

use std::time::{Duration, Instant};

use fastemu_core::{InputEvent, Instance, Key, KeyAction};

/// How long a tap holds its key.
const TAP: Duration = Duration::from_millis(100);

/// One step of a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Press(Key),
    Release(Key),
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

/// Parses a script into steps.
pub fn parse(script: &str) -> Result<Vec<Step>, String> {
    let mut steps = Vec::new();
    for word in script
        .split([',', ' ', '\t', '\n'])
        .filter(|w| !w.is_empty())
    {
        let (name, time) = match word.split_once(':') {
            Some((name, time)) => (name, Some(time)),
            None => (word, None),
        };
        if name.eq_ignore_ascii_case("wait") {
            steps.push(Step::Wait(millis(time.unwrap_or_default())?));
            continue;
        }
        let key =
            key_named(name).ok_or_else(|| format!("unknown key {name:?} in the key script"))?;
        let hold = time.map_or(Ok(TAP), millis)?;
        steps.extend([Step::Press(key), Step::Wait(hold), Step::Release(key)]);
    }
    Ok(steps)
}

/// Plays a script against the clock.
pub struct Player {
    steps: Vec<Step>,
    next: usize,
    due: Option<Instant>,
}

impl Player {
    pub fn new(steps: Vec<Step>) -> Self {
        Self {
            steps,
            next: 0,
            due: None,
        }
    }

    /// Sends every step that is due by `now`; the first call starts the
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
    fn scripts_tap_hold_and_wait() {
        let steps = parse("wait:500 5, up:300").unwrap();
        assert_eq!(
            steps,
            [
                Step::Wait(Duration::from_millis(500)),
                Step::Press(Key::Num5),
                Step::Wait(TAP),
                Step::Release(Key::Num5),
                Step::Press(Key::Up),
                Step::Wait(Duration::from_millis(300)),
                Step::Release(Key::Up),
            ]
        );
        assert!(parse("jump").is_err());
        assert!(parse("fire:soon").is_err());
    }
}
