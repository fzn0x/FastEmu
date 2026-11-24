//! The sound of every game: one engine, played through the system's
//! default output device.

use fastemu_audio::{AudioOutput, Engine, NullOutput};

/// What keeps the engine rendering. It must live as long as the app:
/// dropping it stops the sound.
pub struct Output {
    /// The system's audio device, when one opened.
    _device: Option<AudioOutput>,
    /// Otherwise the mix is rendered in real time and dropped, so games
    /// that wait for the end of a sound still go on.
    _silent: Option<NullOutput>,
}

/// Opens the default output device for `engine`, or keeps time with a
/// silent output when there is none.
pub fn start(engine: &Engine) -> Output {
    match AudioOutput::open(engine) {
        Ok(output) => {
            let info = output.info();
            log::info!(
                "sound on {} at {} Hz, {} channels, {}",
                info.device,
                info.sample_rate,
                info.channels,
                info.sample_format
            );
            Output {
                _device: Some(output),
                _silent: None,
            }
        }
        Err(err) => {
            log::warn!("no sound: {err}; games play silently");
            Output {
                _device: None,
                _silent: Some(NullOutput::start(engine)),
            }
        }
    }
}
