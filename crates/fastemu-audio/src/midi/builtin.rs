//! A small General MIDI synthesizer that needs no sample files.
//!
//! Each of the sixteen GM instrument families gets an oscillator (sine,
//! band-limited saw, square or pulse, a drawbar organ, two-operator FM, a
//! detuned saw pair, or noise), an ADSR envelope and a key-tracked low
//! pass, tuned to sound like the family. Channel 10 plays drums built
//! from pitch-swept tones and filtered noise. It aims at the sound of a
//! phone's MIDI chip, not a sampler: every file plays recognisably, and
//! users who want better sound load a SoundFont.

use std::sync::OnceLock;

use crate::midi::Synth;

/// Voices sounding at once; the quietest released voice, or else the
/// oldest, makes room for a new note.
const VOICES: usize = 32;
/// Scales the sum of voices so a busy arrangement stays clear of
/// clipping before the mixer's limiter.
const OUTPUT_GAIN: f32 = 0.3;
/// A voice below this level is silent and freed.
const SILENT: f32 = 1e-4;
const DRUM_CHANNEL: usize = 9;
const SINE_SIZE: usize = 4096;

/// One cycle of a sine, with the first value repeated at the end so
/// interpolation never wraps.
fn sine_table() -> &'static [f32] {
    static TABLE: OnceLock<Box<[f32]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        (0..=SINE_SIZE)
            .map(|i| (i as f64 / SINE_SIZE as f64 * std::f64::consts::TAU).sin() as f32)
            .collect()
    })
}

/// The sine of `phase` cycles, from a table.
fn sine(phase: f32) -> f32 {
    let table = sine_table();
    let position = (phase - phase.floor()) * SINE_SIZE as f32;
    let index = (position as usize).min(SINE_SIZE - 1);
    let fraction = position - index as f32;
    table[index] + (table[index + 1] - table[index]) * fraction
}

/// The polynomial band-limited step for the saw, square and pulse waves.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Wave {
    Sine,
    Triangle,
    Saw,
    Square,
    /// A pulse high for this fraction of each cycle.
    Pulse(f32),
    /// Sine drawbars at the first four harmonics.
    Organ,
    /// A sine carrier modulated by a sine at `ratio` times its frequency;
    /// the depth follows the envelope, so notes start bright.
    Fm {
        ratio: f32,
        index: f32,
    },
    /// Two saws a few cents apart.
    DetunedSaw,
    Noise,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Patch {
    wave: Wave,
    attack: f32,
    decay: f32,
    sustain: f32,
    release: f32,
    /// The low pass cutoff as a multiple of the note's frequency, or 0
    /// for none.
    brightness: f32,
    gain: f32,
}

const fn patch(
    wave: Wave,
    (attack, decay, sustain, release): (f32, f32, f32, f32),
    brightness: f32,
    gain: f32,
) -> Patch {
    Patch {
        wave,
        attack,
        decay,
        sustain,
        release,
        brightness,
        gain,
    }
}

/// The patch for a GM program number.
fn melodic_patch(program: u8) -> Patch {
    match program {
        // A few programs sound nothing like the rest of their family.
        45 | 46 => patch(Wave::Saw, (0.002, 0.4, 0.0, 0.15), 3.0, 0.7),
        47 => patch(
            Wave::Fm {
                ratio: 1.5,
                index: 2.0,
            },
            (0.002, 0.5, 0.0, 0.3),
            0.0,
            1.0,
        ),
        _ => match program / 8 {
            // Piano.
            0 => patch(
                Wave::Fm {
                    ratio: 1.0,
                    index: 1.8,
                },
                (0.002, 1.5, 0.0, 0.3),
                0.0,
                0.9,
            ),
            // Chromatic percussion: bells and mallets.
            1 => patch(
                Wave::Fm {
                    ratio: 3.5,
                    index: 1.2,
                },
                (0.001, 0.9, 0.0, 0.4),
                0.0,
                0.8,
            ),
            2 => patch(Wave::Organ, (0.005, 0.1, 1.0, 0.06), 0.0, 0.8),
            // Guitar: a plucked saw with a closing filter feel.
            3 => patch(Wave::Saw, (0.002, 1.0, 0.0, 0.15), 4.0, 0.7),
            4 => patch(Wave::Triangle, (0.003, 0.6, 0.5, 0.1), 0.0, 1.0),
            5 => patch(Wave::Saw, (0.08, 0.2, 0.85, 0.25), 6.0, 0.5),
            6 => patch(Wave::DetunedSaw, (0.12, 0.2, 0.9, 0.35), 5.0, 0.5),
            7 => patch(Wave::Saw, (0.03, 0.1, 0.8, 0.1), 5.0, 0.55),
            8 => patch(Wave::Square, (0.02, 0.1, 0.9, 0.08), 6.0, 0.45),
            // Pipe: flutes and whistles.
            9 => patch(Wave::Sine, (0.03, 0.1, 0.9, 0.1), 0.0, 0.9),
            10 => patch(Wave::Pulse(0.3), (0.005, 0.1, 0.9, 0.1), 8.0, 0.45),
            11 => patch(Wave::DetunedSaw, (0.3, 0.5, 0.9, 0.6), 4.0, 0.45),
            12 => patch(Wave::Triangle, (0.1, 0.5, 0.8, 0.5), 0.0, 0.8),
            // Ethnic: sitar, banjo, koto and friends.
            13 => patch(Wave::Pulse(0.25), (0.002, 0.6, 0.1, 0.2), 5.0, 0.55),
            14 => patch(
                Wave::Fm {
                    ratio: 1.4,
                    index: 1.5,
                },
                (0.001, 0.3, 0.0, 0.1),
                0.0,
                0.9,
            ),
            _ => patch(Wave::Noise, (0.01, 0.5, 0.3, 0.3), 2.0, 0.4),
        },
    }
}

/// A drum: a tone that sweeps down in pitch, and a burst of noise.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Drum {
    tone_start: f32,
    tone_end: f32,
    sweep: f32,
    tone_level: f32,
    tone_decay: f32,
    noise_level: f32,
    noise_decay: f32,
    /// High-passed noise for cymbals and hats, else low-passed.
    bright: bool,
}

const fn drum(
    (tone_start, tone_end, sweep): (f32, f32, f32),
    (tone_level, tone_decay): (f32, f32),
    (noise_level, noise_decay): (f32, f32),
    bright: bool,
) -> Drum {
    Drum {
        tone_start,
        tone_end,
        sweep,
        tone_level,
        tone_decay,
        noise_level,
        noise_decay,
        bright,
    }
}

/// The drum for a key on the GM percussion map.
fn drum_for(key: u8) -> Drum {
    match key {
        35 | 36 => drum((130.0, 45.0, 0.04), (1.0, 0.3), (0.15, 0.02), false),
        37 => drum((800.0, 600.0, 0.01), (0.3, 0.03), (0.4, 0.04), true),
        38 | 40 => drum((220.0, 180.0, 0.02), (0.5, 0.08), (0.7, 0.15), true),
        39 => drum((0.0, 0.0, 1.0), (0.0, 0.1), (0.8, 0.12), true),
        // Toms, low to high.
        41 | 43 | 45 | 47 | 48 | 50 => {
            let pitch = 70.0 + f32::from(key - 41) * 14.0;
            drum((pitch * 1.6, pitch, 0.08), (0.9, 0.3), (0.1, 0.05), false)
        }
        42 | 44 => drum((0.0, 0.0, 1.0), (0.0, 0.1), (0.5, 0.04), true),
        46 => drum((0.0, 0.0, 1.0), (0.0, 0.1), (0.5, 0.3), true),
        49 | 52 | 55 | 57 => drum((0.0, 0.0, 1.0), (0.0, 0.1), (0.5, 1.2), true),
        51 | 53 | 59 => drum((620.0, 600.0, 0.5), (0.1, 0.5), (0.35, 0.6), true),
        54 | 69 | 70 => drum((0.0, 0.0, 1.0), (0.0, 0.1), (0.4, 0.08), true),
        56 => drum((560.0, 560.0, 1.0), (0.5, 0.15), (0.0, 0.1), false),
        // Bongos, congas, timbales, agogos: pitched hand drums.
        60..=68 => {
            let pitch = 180.0 + f32::from(key - 60) * 45.0;
            drum((pitch * 1.3, pitch, 0.03), (0.8, 0.15), (0.1, 0.03), false)
        }
        _ => drum((400.0, 300.0, 0.02), (0.3, 0.05), (0.3, 0.05), true),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

#[derive(Clone, Copy, Debug)]
struct Envelope {
    stage: Stage,
    level: f32,
    attack_step: f32,
    decay_coef: f32,
    sustain: f32,
    release_coef: f32,
}

/// The per-sample factor for an exponential fall to 1% in `seconds`.
fn fall_coef(seconds: f32, rate: f32) -> f32 {
    (-4.6 / (seconds.max(0.001) * rate)).exp()
}

impl Envelope {
    fn new(patch: &Patch, rate: f32) -> Self {
        Self {
            stage: Stage::Attack,
            level: 0.0,
            attack_step: 1.0 / (patch.attack.max(0.0005) * rate),
            decay_coef: fall_coef(patch.decay, rate),
            sustain: patch.sustain,
            release_coef: fall_coef(patch.release, rate),
        }
    }

    fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Attack => {
                self.level += self.attack_step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                self.level = self.sustain + (self.level - self.sustain) * self.decay_coef;
                if self.sustain <= 0.0 && self.level < SILENT {
                    self.stage = Stage::Off;
                } else if (self.level - self.sustain).abs() < 1e-3 {
                    self.level = self.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => {}
            Stage::Release => {
                self.level *= self.release_coef;
                if self.level < SILENT {
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => self.level = 0.0,
        }
        self.level
    }

    fn release(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Kind {
    Melodic(Patch),
    Drum {
        drum: Drum,
        tone_env: f32,
        noise_env: f32,
        tone_coef: f32,
        noise_coef: f32,
        sweep_coef: f32,
        sweep: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    channel: usize,
    key: u8,
    /// The key is down.
    held: bool,
    /// The key came up while the sustain pedal was down.
    sustained: bool,
    age: u64,
    gain: f32,
    kind: Kind,
    env: Envelope,
    frequency: f32,
    phase: f32,
    phase2: f32,
    filter: f32,
    filter_coef: f32,
}

impl Voice {
    fn is_off(&self) -> bool {
        match self.kind {
            Kind::Melodic(_) => self.env.stage == Stage::Off,
            Kind::Drum {
                tone_env,
                noise_env,
                ..
            } => tone_env < SILENT && noise_env < SILENT,
        }
    }

    /// The level used to pick a voice to steal.
    fn loudness(&self) -> f32 {
        match self.kind {
            Kind::Melodic(_) => self.env.level,
            Kind::Drum {
                tone_env,
                noise_env,
                ..
            } => tone_env.max(noise_env),
        }
    }

    fn release(&mut self) {
        self.held = false;
        self.sustained = false;
        self.env.release();
    }

    fn sample(&mut self, bend: f32, rate: f32, noise: &mut Noise) -> f32 {
        match &mut self.kind {
            Kind::Melodic(patch) => {
                let level = self.env.next();
                let dt = (self.frequency * bend / rate).min(0.5);
                let raw = match patch.wave {
                    Wave::Sine => sine(self.phase),
                    Wave::Triangle => 4.0 * (self.phase - 0.5).abs() - 1.0,
                    Wave::Saw => 2.0 * self.phase - 1.0 - poly_blep(self.phase, dt),
                    Wave::Square => {
                        let naive = if self.phase < 0.5 { 1.0 } else { -1.0 };
                        naive + poly_blep(self.phase, dt)
                            - poly_blep((self.phase + 0.5).fract(), dt)
                    }
                    Wave::Pulse(width) => {
                        let naive = if self.phase < width { 1.0 } else { -1.0 };
                        naive + poly_blep(self.phase, dt)
                            - poly_blep((self.phase + 1.0 - width).fract(), dt)
                    }
                    Wave::Organ => {
                        let p = self.phase;
                        (sine(p) + 0.5 * sine(2.0 * p) + 0.3 * sine(3.0 * p) + 0.2 * sine(4.0 * p))
                            * 0.5
                    }
                    Wave::Fm { ratio, index } => {
                        self.phase2 = (self.phase2 + dt * ratio).fract();
                        let depth = index * (0.3 + 0.7 * level) / std::f32::consts::TAU;
                        sine(self.phase + depth * sine(self.phase2))
                    }
                    Wave::DetunedSaw => {
                        let dt2 = dt * 1.007;
                        self.phase2 = (self.phase2 + dt2).fract();
                        let a = 2.0 * self.phase - 1.0 - poly_blep(self.phase, dt);
                        let b = 2.0 * self.phase2 - 1.0 - poly_blep(self.phase2, dt2);
                        f32::midpoint(a, b)
                    }
                    Wave::Noise => noise.next(),
                };
                self.phase = (self.phase + dt).fract();
                let filtered = if self.filter_coef > 0.0 {
                    self.filter += (raw - self.filter) * self.filter_coef;
                    self.filter
                } else {
                    raw
                };
                filtered * level * self.gain
            }
            Kind::Drum {
                drum,
                tone_env,
                noise_env,
                tone_coef,
                noise_coef,
                sweep_coef,
                sweep,
            } => {
                let pitch = drum.tone_end + (drum.tone_start - drum.tone_end) * *sweep;
                *sweep *= *sweep_coef;
                let tone = if drum.tone_level > 0.0 {
                    let value = sine(self.phase);
                    self.phase = (self.phase + pitch / rate).fract();
                    value * drum.tone_level * *tone_env
                } else {
                    0.0
                };
                let white = noise.next();
                self.filter += (white - self.filter) * self.filter_coef;
                let shaped = if drum.bright {
                    white - self.filter
                } else {
                    self.filter
                };
                let value = tone + shaped * drum.noise_level * *noise_env;
                *tone_env *= *tone_coef;
                *noise_env *= *noise_coef;
                value * self.gain
            }
        }
    }
}

/// A small, fast pseudo-random source for noise.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        (x as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

#[derive(Clone, Copy, Debug)]
struct Channel {
    program: u8,
    volume: u8,
    expression: u8,
    pan: u8,
    /// The pitch wheel, -8192 to 8191.
    bend: i32,
    bend_range: f32,
    sustain: bool,
    /// The registered parameter data entry changes, or `None` after an
    /// NRPN or a null RPN.
    rpn: Option<(u8, u8)>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            program: 0,
            volume: 100,
            expression: 127,
            pan: 64,
            bend: 0,
            bend_range: 2.0,
            sustain: false,
            rpn: None,
        }
    }
}

impl Channel {
    fn bend_factor(&self) -> f32 {
        let semitones = self.bend as f32 / 8192.0 * self.bend_range;
        2f32.powf(semitones / 12.0)
    }

    /// The left and right gains: GM's squared volume and expression
    /// curves, and an equal-power pan.
    fn gains(&self) -> (f32, f32) {
        let curve = |v: u8| (f32::from(v) / 127.0).powi(2);
        let level = curve(self.volume) * curve(self.expression);
        // 64 is the centre; 0 and 1 are both hard left.
        let angle = f32::from(self.pan.max(1) - 1) / 126.0 * std::f32::consts::FRAC_PI_2;
        (level * angle.cos(), level * angle.sin())
    }

    fn reset_controllers(&mut self) {
        *self = Self {
            program: self.program,
            volume: self.volume,
            pan: self.pan,
            ..Self::default()
        };
    }
}

/// The built-in General MIDI synthesizer.
pub(crate) struct BuiltinSynth {
    rate: f32,
    channels: [Channel; 16],
    voices: Vec<Voice>,
    clock: u64,
    noise: Noise,
}

impl BuiltinSynth {
    pub fn new(rate: u32) -> Self {
        Self {
            rate: rate as f32,
            channels: [Channel::default(); 16],
            voices: Vec::with_capacity(VOICES),
            clock: 0,
            noise: Noise(0x1234_5678),
        }
    }

    fn note_on(&mut self, channel: usize, key: u8, velocity: u8) {
        self.clock += 1;
        let frequency = 440.0 * 2f32.powf((f32::from(key) - 69.0) / 12.0);
        let velocity_gain = (f32::from(velocity) / 127.0).powi(2);
        let (kind, env, gain, filter_coef) = if channel == DRUM_CHANNEL {
            // A closed hi-hat chokes an open one.
            if matches!(key, 42 | 44) {
                self.voices
                    .retain(|v| !(v.channel == channel && v.key == 46));
            }
            let drum = drum_for(key);
            let cutoff = if drum.bright { 7000.0 } else { 1500.0 };
            let kind = Kind::Drum {
                drum,
                tone_env: 1.0,
                noise_env: 1.0,
                tone_coef: fall_coef(drum.tone_decay, self.rate),
                noise_coef: fall_coef(drum.noise_decay, self.rate),
                sweep_coef: fall_coef(drum.sweep, self.rate),
                sweep: 1.0,
            };
            let env = Envelope::new(&melodic_patch(0), self.rate);
            (kind, env, velocity_gain, self.filter_coef(cutoff))
        } else {
            let patch = melodic_patch(self.channels[channel].program);
            let cutoff = if patch.brightness > 0.0 {
                // Louder notes open the filter a little.
                frequency * patch.brightness * (0.6 + 0.6 * velocity_gain)
            } else {
                0.0
            };
            let env = Envelope::new(&patch, self.rate);
            let coef = if cutoff > 0.0 {
                self.filter_coef(cutoff.max(200.0))
            } else {
                0.0
            };
            (Kind::Melodic(patch), env, velocity_gain * patch.gain, coef)
        };
        let voice = Voice {
            channel,
            key,
            held: true,
            sustained: false,
            age: self.clock,
            gain,
            kind,
            env,
            frequency,
            phase: 0.0,
            phase2: 0.0,
            filter: 0.0,
            filter_coef,
        };
        if self.voices.len() < VOICES {
            self.voices.push(voice);
        } else {
            let steal = self
                .voices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    // Released voices go first, the quietest of them;
                    // then the oldest held voice.
                    let rank =
                        |v: &Voice| (v.held, if v.held { v.age as f32 } else { v.loudness() });
                    rank(a)
                        .partial_cmp(&rank(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map_or(0, |(i, _)| i);
            self.voices[steal] = voice;
        }
    }

    fn filter_coef(&self, cutoff: f32) -> f32 {
        let cutoff = cutoff.min(self.rate * 0.45);
        1.0 - (-std::f32::consts::TAU * cutoff / self.rate).exp()
    }

    fn note_off(&mut self, channel: usize, key: u8) {
        // Drums ring out on their own.
        if channel == DRUM_CHANNEL {
            return;
        }
        let pedal = self.channels[channel].sustain;
        for voice in &mut self.voices {
            if voice.channel == channel && voice.key == key && voice.held {
                if pedal {
                    voice.held = false;
                    voice.sustained = true;
                } else {
                    voice.release();
                }
            }
        }
    }

    fn control(&mut self, channel: usize, controller: u8, value: u8) {
        let state = &mut self.channels[channel];
        match controller {
            6 => {
                if state.rpn == Some((0, 0)) {
                    state.bend_range = f32::from(value.min(24));
                }
            }
            7 => state.volume = value,
            10 => state.pan = value,
            11 => state.expression = value,
            64 => {
                state.sustain = value >= 64;
                if !state.sustain {
                    for voice in &mut self.voices {
                        if voice.channel == channel && voice.sustained {
                            voice.release();
                        }
                    }
                }
            }
            98 | 99 => state.rpn = None,
            100 => state.rpn = Some((state.rpn.map_or(0, |(msb, _)| msb), value)),
            101 => state.rpn = Some((value, state.rpn.map_or(0, |(_, lsb)| lsb))),
            120 => self.voices.retain(|v| v.channel != channel),
            121 => state.reset_controllers(),
            123..=127 => {
                for voice in &mut self.voices {
                    if voice.channel == channel {
                        voice.release();
                    }
                }
            }
            _ => {}
        }
    }
}

impl Synth for BuiltinSynth {
    fn message(&mut self, status: u8, data1: u8, data2: u8) {
        let channel = usize::from(status & 0x0F);
        match status & 0xF0 {
            0x80 => self.note_off(channel, data1),
            0x90 if data2 == 0 => self.note_off(channel, data1),
            0x90 => self.note_on(channel, data1, data2),
            0xB0 => self.control(channel, data1, data2),
            0xC0 => self.channels[channel].program = data1 & 0x7F,
            0xE0 => {
                self.channels[channel].bend = ((i32::from(data2) << 7) | i32::from(data1)) - 8192;
            }
            _ => {}
        }
    }

    fn reset(&mut self) {
        self.voices.clear();
        self.channels = [Channel::default(); 16];
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
        let rate = self.rate;
        for voice in &mut self.voices {
            let channel = &self.channels[voice.channel];
            let bend = channel.bend_factor();
            let (gain_left, gain_right) = channel.gains();
            let gain_left = gain_left * OUTPUT_GAIN;
            let gain_right = gain_right * OUTPUT_GAIN;
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                let value = voice.sample(bend, rate, &mut self.noise);
                *l += value * gain_left;
                *r += value * gain_right;
            }
        }
        self.voices.retain(|v| !v.is_off());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 22050;

    fn render(synth: &mut BuiltinSynth, frames: usize) -> (Vec<f32>, Vec<f32>) {
        let mut left = vec![0.0; frames];
        let mut right = vec![0.0; frames];
        synth.render(&mut left, &mut right);
        (left, right)
    }

    fn energy(samples: &[f32]) -> f32 {
        samples.iter().map(|v| v * v).sum::<f32>() / samples.len().max(1) as f32
    }

    fn rising_crossings(samples: &[f32]) -> usize {
        samples
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count()
    }

    #[test]
    fn the_sine_table_is_accurate() {
        for i in 0..100 {
            let phase = i as f32 / 37.0 - 1.0;
            let want = (phase * std::f32::consts::TAU).sin();
            assert!((sine(phase) - want).abs() < 1e-4, "{phase}");
        }
    }

    #[test]
    fn a_flute_plays_at_the_note_pitch() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0xC0, 73, 0);
        synth.message(0x90, 69, 100);
        let (left, right) = render(&mut synth, RATE as usize);
        // One second of A440 rises through zero 440 times.
        let crossings = rising_crossings(&left);
        assert!((438..=442).contains(&crossings), "{crossings}");
        assert!(energy(&left) > 1e-3);
        // Centred by default.
        assert!((energy(&left) - energy(&right)).abs() < 1e-6);
    }

    #[test]
    fn note_off_releases_to_silence() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0xC0, 16, 0);
        synth.message(0x90, 60, 100);
        render(&mut synth, 2205);
        synth.message(0x80, 60, 0);
        let (left, _) = render(&mut synth, RATE as usize);
        assert!(energy(&left[..500]) > 1e-4);
        assert!(energy(&left[left.len() - 500..]) < 1e-9);
        assert!(synth.voices.is_empty());
    }

    #[test]
    fn the_sustain_pedal_holds_notes() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0xC0, 16, 0);
        synth.message(0xB0, 64, 127);
        synth.message(0x90, 60, 100);
        synth.message(0x80, 60, 0);
        let (left, _) = render(&mut synth, RATE as usize);
        assert!(energy(&left[left.len() - 500..]) > 1e-4);
        synth.message(0xB0, 64, 0);
        render(&mut synth, RATE as usize);
        assert!(synth.voices.is_empty());
    }

    #[test]
    fn drums_play_on_channel_ten_and_ring_out() {
        let mut synth = BuiltinSynth::new(RATE);
        for key in [36, 38, 42, 46, 49, 56, 62, 80] {
            synth.message(0x99, key, 120);
        }
        synth.message(0x89, 36, 0);
        let (left, _) = render(&mut synth, 1000);
        assert!(energy(&left) > 1e-4);
        render(&mut synth, RATE as usize * 3);
        assert!(synth.voices.is_empty());
    }

    #[test]
    fn a_closed_hat_chokes_an_open_one() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0x99, 46, 100);
        synth.message(0x99, 42, 100);
        assert_eq!(synth.voices.len(), 1);
        assert_eq!(synth.voices[0].key, 42);
    }

    #[test]
    fn pitch_bend_raises_the_pitch() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0xC0, 73, 0);
        // Bend range 12 semitones through RPN 0, then full bend up.
        synth.message(0xB0, 101, 0);
        synth.message(0xB0, 100, 0);
        synth.message(0xB0, 6, 12);
        synth.message(0xE0, 0x7F, 0x7F);
        synth.message(0x90, 57, 100);
        let (left, _) = render(&mut synth, RATE as usize);
        let crossings = rising_crossings(&left);
        assert!((437..=442).contains(&crossings), "{crossings}");
    }

    #[test]
    fn volume_and_pan_shape_the_output() {
        let loudness = |volume: u8, pan: u8| {
            let mut synth = BuiltinSynth::new(RATE);
            synth.message(0xC0, 73, 0);
            synth.message(0xB0, 7, volume);
            synth.message(0xB0, 10, pan);
            synth.message(0x90, 69, 100);
            let (left, right) = render(&mut synth, 4410);
            (energy(&left), energy(&right))
        };
        let (full, _) = loudness(127, 64);
        let (half, _) = loudness(64, 64);
        assert!(half < full * 0.1);
        let (left, right) = loudness(127, 0);
        assert!(right < left * 1e-3);
    }

    #[test]
    fn programs_change_the_sound() {
        let tone = |program: u8| {
            let mut synth = BuiltinSynth::new(RATE);
            synth.message(0xC0, program, 0);
            synth.message(0x90, 60, 100);
            render(&mut synth, 2000).0
        };
        let sounds: Vec<Vec<f32>> = (0..16).map(|family| tone(family * 8)).collect();
        for (i, a) in sounds.iter().enumerate() {
            assert!(energy(a) > 1e-6, "family {i} is silent");
            for b in &sounds[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn many_notes_steal_voices_instead_of_growing() {
        let mut synth = BuiltinSynth::new(RATE);
        for key in 0..128 {
            synth.message(0x90, key, 100);
        }
        assert_eq!(synth.voices.len(), VOICES);
        let (left, _) = render(&mut synth, 256);
        assert!(left.iter().all(|v| v.is_finite() && v.abs() < 4.0));
    }

    #[test]
    fn reset_and_all_sound_off_silence_everything() {
        let mut synth = BuiltinSynth::new(RATE);
        synth.message(0x90, 60, 100);
        synth.message(0x91, 64, 100);
        synth.message(0xB0, 120, 0);
        assert_eq!(synth.voices.len(), 1);
        synth.message(0xC1, 40, 0);
        synth.reset();
        assert!(synth.voices.is_empty());
        assert_eq!(synth.channels[1].program, 0);
        let (left, _) = render(&mut synth, 100);
        assert!(left.iter().all(|&v| v == 0.0));
    }
}
