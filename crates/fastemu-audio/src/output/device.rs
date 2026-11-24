//! The system audio device through cpal: WASAPI on Windows, CoreAudio on
//! macOS, ALSA on Linux and AAudio on Android.

use std::fmt;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ErrorKind, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};

use crate::engine::Engine;

/// How long to wait before opening the device again after it went away.
const RETRY: Duration = Duration::from_secs(1);
/// Frames of scratch the audio callback starts with; it only grows if
/// the device asks for bigger buffers.
const INITIAL_FRAMES: usize = 4096;

/// What the output opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputInfo {
    pub device: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
}

/// Why the audio device could not be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputError {
    /// The system has no default output device.
    NoDevice,
    /// The device refused the stream.
    Device(String),
}

impl fmt::Display for OutputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => f.write_str("no audio output device"),
            Self::Device(why) => write!(f, "the audio device failed: {why}"),
        }
    }
}

impl std::error::Error for OutputError {}

enum Control {
    /// The stream broke (the device went away); open it again.
    Rebuild,
    Stop,
}

/// Plays the engine's mix on the default output device.
///
/// A thread owns the stream, since cpal streams cannot move between
/// threads on every platform. If the device goes away, the output opens
/// the new default device; meanwhile nothing renders, so apps that need
/// time to keep moving can fall back to [`NullOutput`](crate::NullOutput)
/// when [`AudioOutput::open`] fails. Dropping it closes the stream.
pub struct AudioOutput {
    control: mpsc::Sender<Control>,
    thread: Option<JoinHandle<()>>,
    info: OutputInfo,
    last_error: Arc<Mutex<Option<String>>>,
}

impl AudioOutput {
    /// Opens the default output device at its preferred rate, sets the
    /// engine to that rate, and starts playing.
    pub fn open(engine: &Engine) -> Result<Self, OutputError> {
        let (control, commands) = mpsc::channel();
        let (ready, opened) = mpsc::channel();
        let last_error = Arc::new(Mutex::new(None));
        let thread = {
            let engine = engine.clone();
            let control = control.clone();
            let last_error = Arc::clone(&last_error);
            thread::Builder::new()
                .name("fastemu-audio-output".into())
                .spawn(move || run(&engine, &control, &commands, &ready, &last_error))
                .map_err(|err| OutputError::Device(err.to_string()))?
        };
        match opened.recv() {
            Ok(Ok(info)) => Ok(Self {
                control,
                thread: Some(thread),
                info,
                last_error,
            }),
            Ok(Err(err)) => {
                let _ = thread.join();
                Err(err)
            }
            Err(_) => {
                let _ = thread.join();
                Err(OutputError::Device("the output thread stopped".into()))
            }
        }
    }

    /// The device and format first opened.
    pub fn info(&self) -> &OutputInfo {
        &self.info
    }

    /// The last error the device reported, such as an underrun or the
    /// device going away.
    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        let _ = self.control.send(Control::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    engine: &Engine,
    control: &mpsc::Sender<Control>,
    commands: &mpsc::Receiver<Control>,
    ready: &mpsc::Sender<Result<OutputInfo, OutputError>>,
    last_error: &Arc<Mutex<Option<String>>>,
) {
    let mut stream = match open_stream(engine, control, last_error) {
        Ok((stream, info)) => {
            let _ = ready.send(Ok(info));
            Some(stream)
        }
        Err(err) => {
            let _ = ready.send(Err(err));
            return;
        }
    };
    loop {
        // With no stream, wake up now and then to try again.
        let command = if stream.is_some() {
            commands.recv().map_err(|_| RecvTimeoutError::Disconnected)
        } else {
            commands.recv_timeout(RETRY)
        };
        match command {
            Ok(Control::Stop) | Err(RecvTimeoutError::Disconnected) => return,
            Ok(Control::Rebuild) | Err(RecvTimeoutError::Timeout) => {
                stream = None;
                match open_stream(engine, control, last_error) {
                    Ok((new, _)) => stream = Some(new),
                    Err(err) => record(last_error, err.to_string()),
                }
            }
        }
    }
}

fn record(last_error: &Mutex<Option<String>>, message: String) {
    *last_error.lock().unwrap_or_else(PoisonError::into_inner) = Some(message);
}

fn open_stream(
    engine: &Engine,
    control: &mpsc::Sender<Control>,
    last_error: &Arc<Mutex<Option<String>>>,
) -> Result<(Stream, OutputInfo), OutputError> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or(OutputError::NoDevice)?;
    let supported = device
        .default_output_config()
        .map_err(|err| OutputError::Device(err.to_string()))?;
    let config = supported.config();
    engine.set_sample_rate(config.sample_rate);
    let format = supported.sample_format();
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, engine, control, last_error),
        SampleFormat::F64 => build::<f64>(&device, &config, engine, control, last_error),
        SampleFormat::I16 => build::<i16>(&device, &config, engine, control, last_error),
        SampleFormat::I32 => build::<i32>(&device, &config, engine, control, last_error),
        SampleFormat::U16 => build::<u16>(&device, &config, engine, control, last_error),
        SampleFormat::U8 => build::<u8>(&device, &config, engine, control, last_error),
        SampleFormat::I8 => build::<i8>(&device, &config, engine, control, last_error),
        other => Err(OutputError::Device(format!(
            "unsupported sample format {other}"
        ))),
    }?;
    stream
        .play()
        .map_err(|err| OutputError::Device(err.to_string()))?;
    let info = OutputInfo {
        device: device.to_string(),
        sample_rate: config.sample_rate,
        channels: config.channels,
        sample_format: format.to_string(),
    };
    Ok((stream, info))
}

fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    engine: &Engine,
    control: &mpsc::Sender<Control>,
    last_error: &Arc<Mutex<Option<String>>>,
) -> Result<Stream, OutputError>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels.max(1));
    let engine = engine.clone();
    let mut mix = vec![0.0f32; INITIAL_FRAMES * 2];
    let control = control.clone();
    let last_error = Arc::clone(last_error);
    device
        .build_output_stream::<T, _, _>(
            *config,
            move |data: &mut [T], _| {
                let frames = data.len() / channels;
                if mix.len() < frames * 2 {
                    mix.resize(frames * 2, 0.0);
                }
                let mix = &mut mix[..frames * 2];
                engine.render(mix);
                for (out, pair) in data.chunks_mut(channels).zip(mix.as_chunks::<2>().0) {
                    write_frame(out, *pair);
                }
            },
            move |err| {
                let broken = matches!(
                    err.kind(),
                    ErrorKind::StreamInvalidated | ErrorKind::DeviceNotAvailable
                );
                record(&last_error, err.to_string());
                if broken {
                    let _ = control.send(Control::Rebuild);
                }
            },
            None,
        )
        .map_err(|err| OutputError::Device(err.to_string()))
}

/// Writes one stereo frame to a device frame: mono devices get the
/// average, wider ones get silence past the front pair.
fn write_frame<T>(out: &mut [T], [left, right]: [f32; 2])
where
    T: Sample + FromSample<f32>,
{
    match out {
        [mono] => *mono = T::from_sample(f32::midpoint(left, right)),
        [l, r, rest @ ..] => {
            *l = T::from_sample(left);
            *r = T::from_sample(right);
            rest.fill(T::EQUILIBRIUM);
        }
        [] => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_fit_any_channel_count() {
        let mut mono = [0i16; 1];
        write_frame(&mut mono, [0.5, -0.5]);
        assert_eq!(mono, [0]);
        let mut quad = [7u8; 4];
        write_frame(&mut quad, [1.0, -1.0]);
        assert_eq!(quad, [255, 0, 128, 128]);
        let mut stereo = [0.0f32; 2];
        write_frame(&mut stereo, [0.25, 0.75]);
        assert_eq!(stereo, [0.25, 0.75]);
    }
}
