//! Ways to keep the engine running: the system audio device (feature
//! `output`), or a clock that renders in real time and keeps nothing.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::engine::Engine;

/// How often the null output wakes to render.
const NULL_PERIOD: Duration = Duration::from_millis(10);

/// Renders the mix in real time and throws it away.
///
/// Games wait for sounds to end (an intro jingle, a voice line) through
/// `END_OF_MEDIA` events, which only come while something renders. When
/// there is no audio device, because the headless runner needs none or
/// the device failed to open, this keeps time moving so those games do
/// not hang. Dropping it stops its thread.
pub struct NullOutput {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NullOutput {
    pub fn start(engine: &Engine) -> Self {
        let engine = engine.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("fastemu-audio-null".into())
            .spawn(move || {
                let mut last = Instant::now();
                let mut carry = 0.0f64;
                while !flag.load(Ordering::Acquire) {
                    thread::park_timeout(NULL_PERIOD);
                    let now = Instant::now();
                    let due = now.duration_since(last).as_secs_f64()
                        * f64::from(engine.sample_rate())
                        + carry;
                    last = now;
                    // A long stall (a suspended laptop) is not made up.
                    let frames = due.min(f64::from(engine.sample_rate()) / 2.0);
                    carry = frames.fract();
                    engine.advance(frames as usize);
                }
            })
            .ok();
        Self { stop, thread }
    }
}

impl Drop for NullOutput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineConfig;

    #[test]
    fn keeps_time_moving_without_a_device() {
        let engine = Engine::new(EngineConfig { sample_rate: 8000 });
        let group = engine.create_group();
        let events = group.subscribe();
        let player = group
            .create_player(&crate::wav::tests::pcm16_wav(8000, 1, &[100; 400]), None)
            .unwrap();
        player.start().unwrap();
        let output = NullOutput::start(&engine);
        // 50 ms of sound ends in real time.
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut ended = false;
        while let Ok(event) =
            events.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            if matches!(event.kind, crate::PlayerEventKind::EndOfMedia { .. }) {
                ended = true;
                break;
            }
        }
        assert!(ended);
        drop(output);
    }
}
