//! A patch for the polysynth: two layers, each a full voice of its own (oscillators,
//! a high-pass and a low-pass filter with their own envelope, and an amplifier), played
//! together, as on the Yamaha CS-80's two channels. Around them: one LFO, glide, the
//! pitch sag that brass chords fall by as they die away, a ribbon-style bend range,
//! analogue drift, and what touch (aftertouch) and the modulation wheel do.
//!
//! The factory patches are original sounds written for this app, in the spirit of
//! early-1980s polysynth film scores. They are not copies of any instrument's presets.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum Category {
    Brass,
    Pad,
    Strings,
    Lead,
    Keys,
    Bells,
    Pluck,
    Bass,
    Atmosphere,
    Drone,
    Init,
}

impl Category {
    pub const ALL: [Category; 11] = [
        Category::Brass,
        Category::Pad,
        Category::Strings,
        Category::Lead,
        Category::Keys,
        Category::Bells,
        Category::Pluck,
        Category::Bass,
        Category::Atmosphere,
        Category::Drone,
        Category::Init,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Category::Brass => "Brass",
            Category::Pad => "Pads",
            Category::Strings => "Strings",
            Category::Lead => "Leads",
            Category::Keys => "Keys",
            Category::Bells => "Bells",
            Category::Pluck => "Plucks",
            Category::Bass => "Bass",
            Category::Atmosphere => "Atmosphere",
            Category::Drone => "Drones",
            Category::Init => "Init",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoShape {
    Sine,
    Triangle,
    Saw,
    Square,
    /// Sample and hold: a new random value each cycle.
    Random,
}

impl LfoShape {
    pub const ALL: [LfoShape; 5] = [LfoShape::Sine, LfoShape::Triangle, LfoShape::Saw, LfoShape::Square, LfoShape::Random];

    pub fn label(self) -> &'static str {
        match self {
            LfoShape::Sine => "Sine",
            LfoShape::Triangle => "Tri",
            LfoShape::Saw => "Saw",
            LfoShape::Square => "Square",
            LfoShape::Random => "S&H",
        }
    }
}

/// An attack-decay-sustain-release envelope. Times in seconds; decay and release are
/// the time to fall 40 dB.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Env {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for Env {
    fn default() -> Self {
        Env { attack: 0.01, decay: 0.6, sustain: 0.8, release: 0.5 }
    }
}

const fn env(attack: f32, decay: f32, sustain: f32, release: f32) -> Env {
    Env { attack, decay, sustain, release }
}

/// One of the patch's two layers.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Layer {
    pub on: bool,
    pub level: f32,
    /// Octaves up or down (-2..=2), like a synth's 16', 8', 4' feet.
    pub octave: i8,
    /// Fine tuning in cents, -50..50: detune the two layers against each other.
    pub detune: f32,
    /// Oscillator levels, 0..1.
    pub saw: f32,
    pub pulse: f32,
    /// Pulse width, 0.05..0.5 (square).
    pub width: f32,
    /// How far the LFO sweeps the pulse width, 0..1.
    pub pwm: f32,
    pub sine: f32,
    pub noise: f32,
    /// Frequency modulation of the sine: the modulator's ratio to the note, its
    /// depth (0..1), and how quickly the depth dies away (seconds, 0 = held).
    pub fm_ratio: f32,
    pub fm_depth: f32,
    pub fm_decay: f32,
    /// High-pass cutoff (Hz) and resonance (0..1).
    pub hpf: f32,
    pub hpf_res: f32,
    /// Low-pass cutoff (Hz) and resonance (0..1).
    pub lpf: f32,
    pub lpf_res: f32,
    pub filter_env: Env,
    /// How far the filter envelope opens the low-pass, in octaves (may be negative).
    pub env_amount: f32,
    /// How far the low-pass follows the keyboard: 1 = one octave per octave.
    pub key_track: f32,
    /// Octaves the low-pass opens by at full velocity (initial touch).
    pub vel_to_filter: f32,
    pub amp_env: Env,
    /// How much velocity changes the level: 0 = not at all, 1 = fully.
    pub vel_to_amp: f32,
    /// -1 = left, 1 = right.
    pub pan: f32,
    /// Saturation before the filters, 0..1.
    pub drive: f32,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            on: true,
            level: 0.8,
            octave: 0,
            detune: 0.0,
            saw: 1.0,
            pulse: 0.0,
            width: 0.5,
            pwm: 0.0,
            sine: 0.0,
            noise: 0.0,
            fm_ratio: 1.0,
            fm_depth: 0.0,
            fm_decay: 0.0,
            hpf: 20.0,
            hpf_res: 0.0,
            lpf: 4000.0,
            lpf_res: 0.15,
            filter_env: Env::default(),
            env_amount: 1.0,
            key_track: 0.5,
            vel_to_filter: 1.0,
            amp_env: Env::default(),
            vel_to_amp: 0.5,
            pan: 0.0,
            drive: 0.0,
        }
    }
}

impl Layer {
    fn off() -> Layer {
        Layer { on: false, ..Layer::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Patch {
    pub name: String,
    pub category: Category,
    pub layers: [Layer; 2],
    pub lfo_shape: LfoShape,
    /// Hz.
    pub lfo_rate: f32,
    /// Seconds after each note before its vibrato fades in.
    pub lfo_delay: f32,
    /// Constant vibrato depth in cents.
    pub vibrato: f32,
    /// LFO to the low-pass, in octaves.
    pub lfo_to_filter: f32,
    /// LFO to the level (tremolo), 0..1.
    pub tremolo: f32,
    /// Extra vibrato at full modulation wheel, in cents.
    pub wheel_vibrato: f32,
    /// What pressing harder on a held key does (aftertouch, the CS-80's second touch):
    /// opens the low-pass by this many octaves...
    pub touch_filter: f32,
    /// ...and adds this much vibrato, in cents...
    pub touch_vibrato: f32,
    /// ...and bends up by this many cents.
    pub touch_bend: f32,
    /// Portamento: seconds to glide from one note to the next (0 = off).
    pub glide: f32,
    /// One note at a time; a new note while one is held glides without restarting.
    pub mono: bool,
    /// Cents the pitch falls by as a released note dies away (negative falls).
    pub sag: f32,
    /// Seconds the sag takes.
    pub sag_time: f32,
    /// Pitch bend (ribbon) range, in semitones.
    pub bend_range: f32,
    /// Slow random pitch wander of each voice, in cents.
    pub drift: f32,
    /// Ring modulation of layer II by layer I, 0..1.
    pub ring: f32,
    /// Spread the voices across the stereo field, 0..1.
    pub spread: f32,
    pub gain_db: f32,
}

impl Default for Patch {
    fn default() -> Self {
        Patch {
            name: "Init".into(),
            category: Category::Init,
            layers: [Layer::default(), Layer::off()],
            lfo_shape: LfoShape::Sine,
            lfo_rate: 5.5,
            lfo_delay: 0.4,
            vibrato: 0.0,
            lfo_to_filter: 0.0,
            tremolo: 0.0,
            wheel_vibrato: 30.0,
            touch_filter: 1.0,
            touch_vibrato: 20.0,
            touch_bend: 0.0,
            glide: 0.0,
            mono: false,
            sag: 0.0,
            sag_time: 1.0,
            bend_range: 2.0,
            drift: 3.0,
            ring: 0.0,
            spread: 0.3,
            gain_db: 0.0,
        }
    }
}

fn named(name: &str, category: Category) -> Patch {
    Patch { name: name.into(), category, ..Patch::default() }
}

/// A bell patch: in tune and still.
fn bell(name: &str) -> Patch {
    Patch { drift: 0.0, wheel_vibrato: 0.0, touch_vibrato: 0.0, spread: 0.6, ..named(name, Category::Bells) }
}

/// The factory patches, in display order.
pub fn factory() -> Vec<Patch> {
    let saw = Layer { saw: 1.0, ..Layer::default() };
    vec![
        // ------------------------------------------------------------------ brass
        Patch {
            layers: [
                Layer {
                    lpf: 380.0,
                    lpf_res: 0.22,
                    filter_env: env(0.32, 1.4, 0.55, 1.6),
                    env_amount: 3.4,
                    key_track: 0.6,
                    vel_to_filter: 1.4,
                    amp_env: env(0.09, 2.0, 0.85, 1.7),
                    detune: -6.0,
                    pan: -0.35,
                    drive: 0.15,
                    ..saw
                },
                Layer {
                    saw: 0.6,
                    pulse: 0.5,
                    width: 0.42,
                    pwm: 0.15,
                    lpf: 420.0,
                    lpf_res: 0.18,
                    filter_env: env(0.42, 1.6, 0.5, 1.8),
                    env_amount: 3.1,
                    key_track: 0.6,
                    vel_to_filter: 1.2,
                    amp_env: env(0.12, 2.0, 0.85, 1.9),
                    detune: 7.0,
                    pan: 0.35,
                    ..saw
                },
            ],
            vibrato: 4.0,
            lfo_rate: 5.2,
            lfo_delay: 0.7,
            sag: -55.0,
            sag_time: 1.4,
            touch_filter: 1.5,
            touch_vibrato: 22.0,
            drift: 4.0,
            ..named("Neon Brass", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 300.0,
                    lpf_res: 0.3,
                    filter_env: env(0.7, 2.4, 0.6, 2.4),
                    env_amount: 3.8,
                    vel_to_filter: 1.6,
                    amp_env: env(0.25, 3.0, 0.9, 2.6),
                    detune: -9.0,
                    pan: -0.5,
                    drive: 0.25,
                    ..saw
                },
                Layer {
                    octave: -1,
                    lpf: 260.0,
                    lpf_res: 0.25,
                    filter_env: env(0.9, 2.6, 0.6, 2.6),
                    env_amount: 3.6,
                    vel_to_filter: 1.4,
                    amp_env: env(0.3, 3.0, 0.9, 2.8),
                    detune: 8.0,
                    pan: 0.5,
                    level: 0.7,
                    ..saw
                },
            ],
            vibrato: 3.0,
            lfo_rate: 4.6,
            lfo_delay: 1.0,
            sag: -80.0,
            sag_time: 2.2,
            touch_filter: 1.8,
            touch_vibrato: 28.0,
            drift: 6.0,
            spread: 0.6,
            ..named("Skyline Swell", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 220.0,
                    lpf_res: 0.35,
                    filter_env: env(0.004, 0.35, 0.05, 0.3),
                    env_amount: 4.2,
                    vel_to_filter: 1.5,
                    amp_env: env(0.003, 0.5, 0.0, 0.35),
                    detune: -8.0,
                    drive: 0.35,
                    ..saw
                },
                Layer {
                    octave: -1,
                    lpf: 200.0,
                    lpf_res: 0.3,
                    filter_env: env(0.004, 0.3, 0.05, 0.3),
                    env_amount: 4.0,
                    amp_env: env(0.003, 0.45, 0.0, 0.35),
                    detune: 8.0,
                    level: 0.6,
                    ..saw
                },
            ],
            drift: 3.0,
            ..named("Dark Stab", Category::Brass)
        },
        // ------------------------------------------------------------------ pads
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.3,
                    pwm: 0.55,
                    lpf: 1400.0,
                    lpf_res: 0.12,
                    filter_env: env(2.2, 3.0, 0.7, 4.0),
                    env_amount: 0.8,
                    amp_env: env(1.8, 3.0, 0.9, 4.5),
                    pan: -0.4,
                    vel_to_amp: 0.3,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.4,
                    sine: 0.5,
                    lpf: 1800.0,
                    filter_env: env(2.8, 3.0, 0.7, 4.0),
                    env_amount: 0.6,
                    amp_env: env(2.4, 3.0, 0.9, 5.0),
                    detune: 9.0,
                    pan: 0.4,
                    vel_to_amp: 0.3,
                    ..Layer::default()
                },
            ],
            lfo_rate: 0.35,
            vibrato: 2.0,
            lfo_delay: 0.0,
            drift: 5.0,
            spread: 0.7,
            ..named("Rain Pad", Category::Pad)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 900.0,
                    lpf_res: 0.2,
                    filter_env: env(3.5, 5.0, 0.8, 6.0),
                    env_amount: 1.2,
                    amp_env: env(3.0, 4.0, 1.0, 7.0),
                    detune: -12.0,
                    pan: -0.6,
                    hpf: 120.0,
                    vel_to_amp: 0.2,
                    ..saw
                },
                Layer {
                    octave: 1,
                    saw: 0.0,
                    pulse: 0.7,
                    width: 0.25,
                    pwm: 0.4,
                    lpf: 2400.0,
                    filter_env: env(4.0, 5.0, 0.8, 6.0),
                    env_amount: 0.8,
                    amp_env: env(4.0, 4.0, 1.0, 7.0),
                    detune: 11.0,
                    pan: 0.6,
                    level: 0.45,
                    hpf: 300.0,
                    vel_to_amp: 0.2,
                    ..saw
                },
            ],
            lfo_rate: 0.12,
            lfo_to_filter: 0.4,
            lfo_shape: LfoShape::Triangle,
            drift: 7.0,
            spread: 0.9,
            ..named("Orbital Haze", Category::Pad)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 0.8,
                    width: 0.18,
                    pwm: 0.2,
                    noise: 0.04,
                    hpf: 500.0,
                    hpf_res: 0.3,
                    lpf: 1600.0,
                    lpf_res: 0.55,
                    filter_env: env(1.2, 2.0, 0.8, 3.0),
                    env_amount: 0.5,
                    amp_env: env(1.0, 2.0, 0.9, 3.5),
                    pan: -0.3,
                    ..Layer::default()
                },
                Layer {
                    hpf: 700.0,
                    hpf_res: 0.25,
                    lpf: 2600.0,
                    lpf_res: 0.5,
                    filter_env: env(1.4, 2.0, 0.8, 3.0),
                    env_amount: 0.3,
                    amp_env: env(1.2, 2.0, 0.9, 3.5),
                    detune: 10.0,
                    pan: 0.3,
                    level: 0.5,
                    ..saw
                },
            ],
            vibrato: 9.0,
            lfo_rate: 4.8,
            lfo_delay: 0.6,
            drift: 4.0,
            spread: 0.6,
            ..named("Vox Haze", Category::Pad)
        },
        // ------------------------------------------------------------------ strings
        Patch {
            layers: [
                Layer {
                    hpf: 180.0,
                    lpf: 3200.0,
                    lpf_res: 0.1,
                    filter_env: env(0.9, 2.0, 0.85, 2.5),
                    env_amount: 0.6,
                    amp_env: env(0.8, 2.0, 0.95, 2.8),
                    detune: -13.0,
                    pan: -0.5,
                    vel_to_amp: 0.35,
                    ..saw
                },
                Layer {
                    hpf: 220.0,
                    lpf: 3600.0,
                    filter_env: env(1.0, 2.0, 0.85, 2.5),
                    env_amount: 0.5,
                    amp_env: env(0.9, 2.0, 0.95, 3.0),
                    detune: 12.0,
                    pan: 0.5,
                    vel_to_amp: 0.35,
                    ..saw
                },
            ],
            vibrato: 5.0,
            lfo_rate: 5.6,
            lfo_delay: 0.8,
            drift: 4.0,
            spread: 0.8,
            ..named("Memory Strings", Category::Strings)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    hpf: 60.0,
                    lpf: 1200.0,
                    filter_env: env(1.6, 3.0, 0.8, 3.5),
                    env_amount: 1.0,
                    amp_env: env(1.4, 3.0, 1.0, 4.0),
                    detune: -7.0,
                    pan: -0.4,
                    ..saw
                },
                Layer {
                    hpf: 300.0,
                    lpf: 2600.0,
                    filter_env: env(2.0, 3.0, 0.8, 3.5),
                    env_amount: 0.8,
                    amp_env: env(1.8, 3.0, 1.0, 4.5),
                    detune: 7.0,
                    pan: 0.4,
                    level: 0.6,
                    ..saw
                },
            ],
            vibrato: 3.0,
            lfo_rate: 5.0,
            lfo_delay: 1.2,
            sag: -25.0,
            sag_time: 2.5,
            drift: 5.0,
            spread: 0.7,
            ..named("Low Cloud Strings", Category::Strings)
        },
        // ------------------------------------------------------------------ leads
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.32,
                    pwm: 0.1,
                    noise: 0.08,
                    lpf: 900.0,
                    lpf_res: 0.2,
                    filter_env: env(0.12, 0.8, 0.6, 0.8),
                    env_amount: 1.6,
                    key_track: 0.8,
                    vel_to_filter: 1.2,
                    amp_env: env(0.06, 1.0, 0.85, 0.9),
                    drive: 0.3,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.5,
                    lpf: 1400.0,
                    filter_env: env(0.18, 0.8, 0.6, 0.8),
                    env_amount: 1.2,
                    key_track: 0.8,
                    amp_env: env(0.1, 1.0, 0.85, 1.0),
                    detune: 5.0,
                    level: 0.35,
                    ..saw
                },
            ],
            mono: true,
            glide: 0.09,
            vibrato: 14.0,
            lfo_rate: 5.4,
            lfo_delay: 0.45,
            sag: -40.0,
            sag_time: 0.7,
            touch_vibrato: 30.0,
            touch_filter: 1.2,
            drift: 3.0,
            spread: 0.0,
            ..named("Smoky Lead", Category::Lead)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 1600.0,
                    lpf_res: 0.3,
                    filter_env: env(0.05, 1.0, 0.7, 1.2),
                    env_amount: 1.0,
                    key_track: 0.9,
                    amp_env: env(0.04, 1.0, 0.9, 1.4),
                    detune: -4.0,
                    ..saw
                },
                Layer { sine: 0.6, saw: 0.0, octave: 1, lpf: 6000.0, amp_env: env(0.1, 1.0, 0.9, 1.4), level: 0.3, ..saw },
            ],
            mono: true,
            glide: 0.28,
            bend_range: 12.0,
            vibrato: 8.0,
            lfo_rate: 5.8,
            lfo_delay: 0.3,
            sag: -30.0,
            sag_time: 1.0,
            touch_vibrato: 40.0,
            touch_bend: 50.0,
            spread: 0.0,
            ..named("Ribbon Lead", Category::Lead)
        },
        Patch {
            layers: [
                Layer { saw: 0.0, sine: 1.0, lpf: 8000.0, amp_env: env(0.4, 1.0, 1.0, 1.5), env_amount: 0.0, ..Layer::default() },
                Layer::off(),
            ],
            mono: true,
            glide: 0.6,
            lfo_rate: 0.25,
            vibrato: 350.0,
            lfo_delay: 0.0,
            lfo_shape: LfoShape::Triangle,
            bend_range: 12.0,
            spread: 0.0,
            gain_db: -6.0,
            ..named("Police Siren", Category::Lead)
        },
        // ------------------------------------------------------------------ keys
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 1.0,
                    fm_depth: 0.55,
                    fm_decay: 1.2,
                    lpf: 9000.0,
                    env_amount: 0.0,
                    amp_env: env(0.002, 3.2, 0.0, 0.6),
                    vel_to_amp: 0.7,
                    pan: -0.15,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.35,
                    fm_ratio: 14.0,
                    fm_depth: 0.35,
                    fm_decay: 0.15,
                    lpf: 12000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 0.5, 0.0, 0.3),
                    vel_to_amp: 0.9,
                    pan: 0.15,
                    ..Layer::default()
                },
            ],
            tremolo: 0.15,
            lfo_rate: 4.2,
            lfo_delay: 0.0,
            spread: 0.4,
            drift: 0.5,
            ..named("Neon Tine Piano", Category::Keys)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.6,
                    pulse: 0.4,
                    width: 0.4,
                    lpf: 700.0,
                    lpf_res: 0.15,
                    filter_env: env(0.003, 1.8, 0.2, 1.0),
                    env_amount: 2.6,
                    vel_to_filter: 1.6,
                    amp_env: env(0.004, 3.5, 0.25, 1.2),
                    detune: -5.0,
                    pan: -0.3,
                    ..Layer::default()
                },
                Layer {
                    lpf: 900.0,
                    filter_env: env(0.003, 1.6, 0.2, 1.0),
                    env_amount: 2.2,
                    amp_env: env(0.004, 3.5, 0.25, 1.2),
                    detune: 6.0,
                    pan: 0.3,
                    level: 0.6,
                    ..saw
                },
            ],
            sag: -15.0,
            sag_time: 1.0,
            spread: 0.5,
            ..named("Polysynth Keys", Category::Keys)
        },
        // ------------------------------------------------------------------ bells
        // Bells keep to whole-number FM ratios, so every partial sits on the note's
        // harmonic series and the pitch is unambiguous; no detune, drift or vibrato.
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 4.0,
                    fm_depth: 0.5,
                    fm_decay: 1.2,
                    lpf: 12000.0,
                    env_amount: 0.0,
                    amp_env: env(0.002, 5.0, 0.0, 4.0),
                    vel_to_amp: 0.6,
                    pan: -0.2,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.5,
                    octave: 1,
                    fm_ratio: 2.0,
                    fm_depth: 0.2,
                    fm_decay: 2.0,
                    lpf: 12000.0,
                    env_amount: 0.0,
                    amp_env: env(0.002, 3.0, 0.0, 3.0),
                    pan: 0.25,
                    level: 0.4,
                    ..Layer::default()
                },
            ],
            ..bell("Night Bells")
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    octave: 1,
                    fm_ratio: 3.0,
                    fm_depth: 0.35,
                    fm_decay: 0.5,
                    lpf: 14000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 2.5, 0.0, 2.0),
                    vel_to_amp: 0.7,
                    ..Layer::default()
                },
                Layer::off(),
            ],
            gain_db: -2.0,
            ..bell("Glass Chimes")
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 1.0,
                    fm_depth: 0.3,
                    fm_decay: 0.4,
                    lpf: 12000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 2.2, 0.0, 1.2),
                    vel_to_amp: 0.6,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.25,
                    octave: 2,
                    lpf: 14000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 0.6, 0.0, 0.4),
                    level: 0.5,
                    ..Layer::default()
                },
            ],
            ..bell("Celesta Glow")
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    octave: 1,
                    fm_ratio: 2.0,
                    fm_depth: 0.4,
                    fm_decay: 0.2,
                    lpf: 12000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 1.8, 0.0, 1.0),
                    vel_to_amp: 0.7,
                    ..Layer::default()
                },
                Layer::off(),
            ],
            gain_db: -1.0,
            ..bell("Music Box")
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 4.0,
                    fm_depth: 0.15,
                    fm_decay: 0.3,
                    lpf: 10000.0,
                    env_amount: 0.0,
                    amp_env: env(0.002, 3.5, 0.0, 1.5),
                    vel_to_amp: 0.6,
                    ..Layer::default()
                },
                Layer::off(),
            ],
            // vibraphone motor: tremolo only, which moves the level, never the pitch
            tremolo: 0.35,
            lfo_rate: 5.0,
            lfo_delay: 0.0,
            ..bell("Rain Vibes")
        },
        // ------------------------------------------------------------------ plucks
        Patch {
            layers: [
                Layer {
                    saw: 0.3,
                    pulse: 1.0,
                    width: 0.15,
                    lpf: 900.0,
                    lpf_res: 0.3,
                    filter_env: env(0.001, 0.35, 0.0, 0.4),
                    env_amount: 3.2,
                    key_track: 1.0,
                    amp_env: env(0.001, 1.6, 0.0, 0.6),
                    vel_to_amp: 0.7,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.5,
                    octave: 1,
                    lpf: 6000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 0.4, 0.0, 0.3),
                    level: 0.3,
                    ..Layer::default()
                },
            ],
            sag: -8.0,
            sag_time: 0.4,
            spread: 0.4,
            ..named("Lantern Pluck", Category::Pluck)
        },
        // ------------------------------------------------------------------ bass
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    saw: 0.0,
                    sine: 1.0,
                    lpf: 600.0,
                    env_amount: 0.0,
                    amp_env: env(0.01, 1.0, 1.0, 0.4),
                    vel_to_amp: 0.4,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    lpf: 220.0,
                    lpf_res: 0.25,
                    filter_env: env(0.01, 0.6, 0.3, 0.4),
                    env_amount: 2.0,
                    amp_env: env(0.01, 1.0, 0.9, 0.4),
                    level: 0.5,
                    ..saw
                },
            ],
            mono: true,
            glide: 0.03,
            spread: 0.0,
            drift: 2.0,
            ..named("Sub Pulse Bass", Category::Bass)
        },
        Patch {
            layers: [
                Layer {
                    octave: -2,
                    lpf: 160.0,
                    lpf_res: 0.4,
                    filter_env: env(1.2, 3.0, 0.5, 2.0),
                    env_amount: 2.2,
                    amp_env: env(0.6, 2.0, 1.0, 2.5),
                    detune: -5.0,
                    drive: 0.4,
                    ..saw
                },
                Layer {
                    octave: -1,
                    lpf: 240.0,
                    lpf_res: 0.3,
                    filter_env: env(1.6, 3.0, 0.5, 2.0),
                    env_amount: 1.8,
                    amp_env: env(0.8, 2.0, 1.0, 2.5),
                    detune: 5.0,
                    level: 0.5,
                    ..saw
                },
            ],
            sag: -30.0,
            sag_time: 2.0,
            lfo_rate: 0.15,
            lfo_to_filter: 0.5,
            spread: 0.2,
            ..named("Undercity Bass", Category::Bass)
        },
        // ------------------------------------------------------------------ atmosphere
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    noise: 1.0,
                    lpf: 600.0,
                    lpf_res: 0.75,
                    filter_env: env(2.0, 4.0, 0.6, 4.0),
                    env_amount: 2.0,
                    key_track: 1.0,
                    amp_env: env(1.5, 3.0, 0.9, 4.0),
                    pan: -0.5,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    noise: 1.0,
                    hpf: 2000.0,
                    lpf: 5000.0,
                    lpf_res: 0.6,
                    filter_env: env(3.0, 4.0, 0.6, 4.0),
                    env_amount: -1.5,
                    key_track: 1.0,
                    amp_env: env(2.0, 3.0, 0.9, 4.0),
                    pan: 0.5,
                    level: 0.4,
                    ..Layer::default()
                },
            ],
            lfo_rate: 0.08,
            lfo_to_filter: 1.2,
            lfo_shape: LfoShape::Triangle,
            spread: 0.9,
            ..named("Hover Sweep", Category::Atmosphere)
        },
        Patch {
            layers: [
                Layer {
                    octave: -2,
                    saw: 0.8,
                    pulse: 0.6,
                    width: 0.2,
                    pwm: 0.6,
                    lpf: 300.0,
                    lpf_res: 0.45,
                    filter_env: env(4.0, 6.0, 0.8, 6.0),
                    env_amount: 1.5,
                    amp_env: env(4.0, 6.0, 1.0, 8.0),
                    detune: -9.0,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    lpf: 500.0,
                    lpf_res: 0.35,
                    filter_env: env(5.0, 6.0, 0.8, 6.0),
                    env_amount: 1.2,
                    amp_env: env(5.0, 6.0, 1.0, 8.0),
                    detune: 9.0,
                    level: 0.6,
                    ..saw
                },
            ],
            ring: 0.2,
            lfo_rate: 0.06,
            lfo_to_filter: 0.9,
            lfo_shape: LfoShape::Sine,
            drift: 9.0,
            spread: 0.7,
            ..named("Ozone Drone", Category::Drone)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 2.76,
                    fm_depth: 0.9,
                    lpf: 3000.0,
                    lpf_res: 0.4,
                    env_amount: 1.0,
                    filter_env: env(0.01, 0.3, 0.2, 2.0),
                    amp_env: env(0.01, 2.0, 0.6, 3.0),
                    ..Layer::default()
                },
                Layer { saw: 0.0, pulse: 1.0, width: 0.1, octave: -1, lpf: 2000.0, level: 0.6, ..Layer::default() },
            ],
            ring: 0.8,
            lfo_shape: LfoShape::Random,
            lfo_rate: 9.0,
            lfo_to_filter: 0.8,
            spread: 0.5,
            gain_db: -4.0,
            ..named("Data Static", Category::Atmosphere)
        },
        // ------------------------------------------------------------------ alternates
        Patch {
            layers: [
                Layer {
                    saw: 0.5,
                    sine: 0.6,
                    lpf: 500.0,
                    lpf_res: 0.1,
                    filter_env: env(0.6, 2.0, 0.6, 2.0),
                    env_amount: 2.2,
                    vel_to_filter: 1.0,
                    amp_env: env(0.35, 2.0, 0.9, 2.2),
                    detune: -4.0,
                    pan: -0.3,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.7,
                    lpf: 600.0,
                    filter_env: env(0.8, 2.0, 0.6, 2.2),
                    env_amount: 2.0,
                    amp_env: env(0.45, 2.0, 0.9, 2.4),
                    detune: 4.0,
                    pan: 0.3,
                    level: 0.6,
                    ..Layer::default()
                },
            ],
            vibrato: 3.0,
            lfo_rate: 5.0,
            lfo_delay: 0.9,
            sag: -35.0,
            sag_time: 1.6,
            drift: 3.0,
            spread: 0.5,
            ..named("Velvet Horns", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 260.0,
                    lpf_res: 0.25,
                    filter_env: env(1.4, 3.0, 0.7, 3.0),
                    env_amount: 4.0,
                    vel_to_filter: 1.6,
                    amp_env: env(0.6, 3.0, 1.0, 3.2),
                    detune: -10.0,
                    pan: -0.6,
                    drive: 0.2,
                    ..Layer::default()
                },
                Layer {
                    octave: 1,
                    lpf: 320.0,
                    lpf_res: 0.2,
                    filter_env: env(1.8, 3.0, 0.7, 3.0),
                    env_amount: 3.6,
                    amp_env: env(0.9, 3.0, 1.0, 3.4),
                    detune: 9.0,
                    pan: 0.6,
                    level: 0.55,
                    ..Layer::default()
                },
            ],
            vibrato: 4.0,
            lfo_rate: 4.4,
            lfo_delay: 1.4,
            sag: -110.0,
            sag_time: 2.8,
            touch_filter: 2.0,
            touch_vibrato: 30.0,
            drift: 6.0,
            spread: 0.8,
            ..named("Tidal Brass", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.3,
                    lpf: 180.0,
                    lpf_res: 0.45,
                    filter_env: env(0.002, 0.35, 0.15, 0.2),
                    env_amount: 3.5,
                    key_track: 0.5,
                    amp_env: env(0.002, 0.9, 0.6, 0.2),
                    vel_to_amp: 0.5,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    saw: 0.0,
                    sine: 0.6,
                    env_amount: 0.0,
                    lpf: 400.0,
                    amp_env: env(0.002, 1.0, 0.8, 0.2),
                    level: 0.5,
                    ..Layer::default()
                },
            ],
            mono: true,
            glide: 0.05,
            drift: 1.0,
            spread: 0.0,
            ..named("Rubber Bass", Category::Bass)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    saw: 0.0,
                    sine: 1.0,
                    lpf: 300.0,
                    env_amount: 0.0,
                    amp_env: env(0.02, 1.0, 1.0, 0.6),
                    vel_to_amp: 0.3,
                    drive: 0.15,
                    ..Layer::default()
                },
                Layer::off(),
            ],
            mono: true,
            glide: 0.04,
            drift: 0.0,
            spread: 0.0,
            gain_db: 2.0,
            ..named("Pure Sub", Category::Bass)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    lpf: 240.0,
                    lpf_res: 0.55,
                    filter_env: env(0.005, 0.6, 0.25, 0.3),
                    env_amount: 3.0,
                    key_track: 0.6,
                    vel_to_filter: 1.4,
                    amp_env: env(0.004, 1.2, 0.8, 0.3),
                    detune: -6.0,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    lpf: 300.0,
                    lpf_res: 0.4,
                    filter_env: env(0.005, 0.6, 0.25, 0.3),
                    env_amount: 2.6,
                    amp_env: env(0.004, 1.2, 0.8, 0.3),
                    detune: 6.0,
                    level: 0.6,
                    ..Layer::default()
                },
            ],
            mono: true,
            glide: 0.03,
            drift: 2.0,
            spread: 0.2,
            ..named("Night Drive Bass", Category::Bass)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 2.0,
                    fm_depth: 0.25,
                    lpf: 6000.0,
                    env_amount: 0.0,
                    amp_env: env(0.05, 1.0, 0.9, 1.0),
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    pulse: 0.3,
                    width: 0.12,
                    octave: 1,
                    lpf: 3000.0,
                    env_amount: 0.0,
                    amp_env: env(0.1, 1.0, 0.8, 1.0),
                    level: 0.25,
                    ..Layer::default()
                },
            ],
            mono: true,
            glide: 0.12,
            vibrato: 10.0,
            lfo_rate: 5.6,
            lfo_delay: 0.5,
            sag: -25.0,
            sag_time: 0.8,
            touch_vibrato: 30.0,
            spread: 0.0,
            drift: 1.0,
            ..named("Glass Lead", Category::Lead)
        },
        Patch {
            layers: [
                Layer {
                    lpf: 2200.0,
                    lpf_res: 0.2,
                    filter_env: env(0.05, 0.8, 0.7, 0.8),
                    env_amount: 1.2,
                    key_track: 0.8,
                    amp_env: env(0.03, 1.0, 0.9, 0.9),
                    detune: -9.0,
                    pan: -0.5,
                    ..Layer::default()
                },
                Layer {
                    lpf: 2200.0,
                    filter_env: env(0.05, 0.8, 0.7, 0.8),
                    env_amount: 1.2,
                    key_track: 0.8,
                    amp_env: env(0.03, 1.0, 0.9, 0.9),
                    detune: 9.0,
                    pan: 0.5,
                    ..Layer::default()
                },
            ],
            glide: 0.08,
            mono: true,
            vibrato: 7.0,
            lfo_delay: 0.4,
            sag: -40.0,
            sag_time: 0.9,
            spread: 0.0,
            gain_db: -3.0,
            ..named("Wide Saw Lead", Category::Lead)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.2,
                    pwm: 0.8,
                    lpf: 2000.0,
                    filter_env: env(2.5, 4.0, 0.8, 5.0),
                    env_amount: 0.6,
                    amp_env: env(2.5, 4.0, 1.0, 5.0),
                    pan: -0.5,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.35,
                    pwm: 0.6,
                    octave: 1,
                    lpf: 3000.0,
                    hpf: 400.0,
                    filter_env: env(3.0, 4.0, 0.8, 5.0),
                    env_amount: 0.4,
                    amp_env: env(3.0, 4.0, 1.0, 5.0),
                    detune: 7.0,
                    pan: 0.5,
                    level: 0.45,
                    ..Layer::default()
                },
            ],
            lfo_rate: 0.22,
            lfo_shape: LfoShape::Triangle,
            drift: 4.0,
            spread: 0.9,
            ..named("Neon Mist", Category::Pad)
        },
        // ------------------------------------------------------------------ drones
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    saw: 0.0,
                    sine: 1.0,
                    lpf: 800.0,
                    env_amount: 0.0,
                    amp_env: env(4.0, 6.0, 1.0, 8.0),
                    pan: -0.2,
                    ..Layer::default()
                },
                Layer {
                    lpf: 380.0,
                    lpf_res: 0.3,
                    env_amount: 0.0,
                    amp_env: env(6.0, 6.0, 1.0, 8.0),
                    detune: 6.0,
                    pan: 0.2,
                    level: 0.4,
                    ..Layer::default()
                },
            ],
            lfo_rate: 0.05,
            lfo_to_filter: 0.7,
            lfo_shape: LfoShape::Sine,
            drift: 3.0,
            spread: 0.6,
            ..named("Abyss Drone", Category::Drone)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 0.9,
                    width: 0.15,
                    pwm: 0.2,
                    noise: 0.05,
                    hpf: 350.0,
                    hpf_res: 0.35,
                    lpf: 1200.0,
                    lpf_res: 0.6,
                    env_amount: 0.0,
                    amp_env: env(5.0, 6.0, 1.0, 8.0),
                    pan: -0.4,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    hpf: 200.0,
                    lpf: 900.0,
                    lpf_res: 0.5,
                    env_amount: 0.0,
                    amp_env: env(6.0, 6.0, 1.0, 8.0),
                    detune: 8.0,
                    pan: 0.4,
                    level: 0.5,
                    ..Layer::default()
                },
            ],
            vibrato: 4.0,
            lfo_rate: 4.6,
            lfo_delay: 2.0,
            drift: 4.0,
            spread: 0.7,
            ..named("Choir Drone", Category::Drone)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    fm_ratio: 2.0,
                    fm_depth: 0.3,
                    lpf: 6000.0,
                    env_amount: 0.0,
                    amp_env: env(5.0, 6.0, 1.0, 8.0),
                    pan: -0.4,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.6,
                    octave: 1,
                    lpf: 8000.0,
                    env_amount: 0.0,
                    amp_env: env(7.0, 6.0, 1.0, 8.0),
                    pan: 0.4,
                    level: 0.4,
                    ..Layer::default()
                },
            ],
            tremolo: 0.25,
            lfo_rate: 0.15,
            drift: 0.0,
            spread: 0.8,
            gain_db: -3.0,
            ..named("Glass Drone", Category::Drone)
        },
        // ------------------------------------------------- more of the two-layer classics
        // Sounds the CS-80's architecture is known for: brass that blooms and sags,
        // strings with a delayed vibrato, resonant high-pass "voices", a solo horn played
        // with touch, ring-modulated bells, a clavinet, a drawbar-like organ.
        Patch {
            layers: [
                Layer {
                    lpf: 520.0,
                    lpf_res: 0.32,
                    filter_env: env(0.12, 0.9, 0.6, 1.2),
                    env_amount: 3.6,
                    key_track: 0.7,
                    vel_to_filter: 1.6,
                    amp_env: env(0.05, 1.5, 0.9, 1.2),
                    detune: -5.0,
                    pan: -0.3,
                    drive: 0.25,
                    ..saw
                },
                Layer {
                    octave: 1,
                    saw: 0.5,
                    pulse: 0.5,
                    width: 0.35,
                    lpf: 700.0,
                    lpf_res: 0.25,
                    filter_env: env(0.16, 1.0, 0.55, 1.2),
                    env_amount: 3.2,
                    key_track: 0.7,
                    vel_to_filter: 1.4,
                    amp_env: env(0.08, 1.5, 0.9, 1.3),
                    detune: 5.0,
                    pan: 0.3,
                    level: 0.45,
                    ..saw
                },
            ],
            vibrato: 5.0,
            lfo_rate: 5.6,
            lfo_delay: 0.5,
            sag: -40.0,
            sag_time: 1.0,
            touch_filter: 2.0,
            touch_vibrato: 25.0,
            ..named("Chrome Fanfare", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.7,
                    sine: 0.4,
                    lpf: 240.0,
                    lpf_res: 0.1,
                    filter_env: env(0.5, 2.0, 0.7, 2.2),
                    env_amount: 2.6,
                    key_track: 0.6,
                    vel_to_filter: 1.2,
                    amp_env: env(0.22, 2.5, 0.9, 2.2),
                    detune: -6.0,
                    pan: -0.4,
                    ..Layer::default()
                },
                Layer {
                    octave: -1,
                    saw: 0.0,
                    pulse: 0.8,
                    width: 0.45,
                    pwm: 0.1,
                    lpf: 200.0,
                    filter_env: env(0.6, 2.0, 0.7, 2.4),
                    env_amount: 2.4,
                    amp_env: env(0.28, 2.5, 0.9, 2.4),
                    detune: 6.0,
                    pan: 0.4,
                    level: 0.6,
                    ..Layer::default()
                },
            ],
            vibrato: 3.0,
            lfo_rate: 4.8,
            lfo_delay: 0.9,
            sag: -60.0,
            sag_time: 1.8,
            touch_filter: 1.6,
            touch_vibrato: 20.0,
            drift: 5.0,
            spread: 0.5,
            ..named("Ember Horns", Category::Brass)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.8,
                    pulse: 0.4,
                    width: 0.3,
                    pwm: 0.35,
                    hpf: 260.0,
                    hpf_res: 0.2,
                    lpf: 2600.0,
                    lpf_res: 0.12,
                    filter_env: env(1.4, 2.5, 0.8, 3.0),
                    env_amount: 0.7,
                    amp_env: env(1.2, 2.5, 0.95, 3.2),
                    detune: -10.0,
                    pan: -0.6,
                    vel_to_amp: 0.3,
                    ..Layer::default()
                },
                Layer {
                    octave: 1,
                    hpf: 600.0,
                    lpf: 4200.0,
                    filter_env: env(1.8, 2.5, 0.8, 3.0),
                    env_amount: 0.5,
                    amp_env: env(1.6, 2.5, 0.95, 3.6),
                    detune: 9.0,
                    pan: 0.6,
                    level: 0.35,
                    vel_to_amp: 0.3,
                    ..saw
                },
            ],
            vibrato: 6.0,
            lfo_rate: 5.3,
            lfo_delay: 1.2,
            touch_vibrato: 18.0,
            drift: 5.0,
            spread: 0.9,
            gain_db: 5.0,
            ..named("Silk Strings", Category::Strings)
        },
        Patch {
            layers: [
                Layer {
                    hpf: 140.0,
                    lpf: 300.0,
                    lpf_res: 0.45,
                    filter_env: env(4.0, 6.0, 0.6, 5.0),
                    env_amount: 3.4,
                    amp_env: env(2.5, 4.0, 1.0, 6.0),
                    detune: -8.0,
                    pan: -0.5,
                    vel_to_amp: 0.2,
                    ..saw
                },
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.2,
                    pwm: 0.5,
                    hpf: 400.0,
                    hpf_res: 0.45,
                    lpf: 3200.0,
                    filter_env: env(5.0, 6.0, 0.6, 5.0),
                    env_amount: 0.6,
                    amp_env: env(3.5, 4.0, 1.0, 6.5),
                    detune: 8.0,
                    pan: 0.5,
                    level: 0.5,
                    vel_to_amp: 0.2,
                    ..Layer::default()
                },
            ],
            lfo_shape: LfoShape::Triangle,
            lfo_rate: 0.08,
            lfo_to_filter: 0.6,
            lfo_delay: 0.0,
            drift: 6.0,
            spread: 0.9,
            ..named("Cathedral Sweep", Category::Pad)
        },
        Patch {
            layers: [
                // two resonant band-passes, high- and low-pass together, give vowel-like formants
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.4,
                    pwm: 0.25,
                    hpf: 450.0,
                    hpf_res: 0.5,
                    lpf: 900.0,
                    lpf_res: 0.5,
                    filter_env: env(0.8, 2.0, 0.9, 2.5),
                    env_amount: 0.2,
                    key_track: 0.2,
                    amp_env: env(0.7, 2.0, 1.0, 2.8),
                    pan: -0.35,
                    vel_to_amp: 0.3,
                    ..Layer::default()
                },
                Layer {
                    hpf: 1100.0,
                    hpf_res: 0.5,
                    lpf: 2400.0,
                    lpf_res: 0.45,
                    filter_env: env(0.9, 2.0, 0.9, 2.5),
                    env_amount: 0.1,
                    key_track: 0.2,
                    amp_env: env(0.8, 2.0, 1.0, 3.0),
                    detune: 7.0,
                    pan: 0.35,
                    level: 0.6,
                    vel_to_amp: 0.3,
                    ..saw
                },
            ],
            vibrato: 8.0,
            lfo_rate: 5.0,
            lfo_delay: 0.5,
            touch_vibrato: 15.0,
            drift: 4.0,
            spread: 0.7,
            ..named("Night Choir", Category::Pad)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.7,
                    pulse: 0.4,
                    width: 0.38,
                    lpf: 650.0,
                    lpf_res: 0.2,
                    filter_env: env(0.07, 0.8, 0.65, 0.9),
                    env_amount: 2.2,
                    key_track: 0.8,
                    vel_to_filter: 1.5,
                    amp_env: env(0.04, 1.0, 0.9, 0.9),
                    drive: 0.2,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.5,
                    lpf: 3000.0,
                    env_amount: 0.0,
                    amp_env: env(0.12, 1.0, 0.9, 1.0),
                    detune: 4.0,
                    level: 0.3,
                    ..Layer::default()
                },
            ],
            mono: true,
            glide: 0.06,
            bend_range: 7.0,
            vibrato: 18.0,
            lfo_rate: 5.0,
            lfo_delay: 1.1,
            sag: -25.0,
            sag_time: 0.8,
            touch_filter: 1.8,
            touch_vibrato: 35.0,
            touch_bend: 30.0,
            spread: 0.0,
            gain_db: 5.0,
            ..named("Lonely Horn", Category::Lead)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.14,
                    hpf: 220.0,
                    lpf: 900.0,
                    lpf_res: 0.35,
                    filter_env: env(0.001, 0.25, 0.1, 0.2),
                    env_amount: 3.4,
                    key_track: 1.0,
                    vel_to_filter: 1.4,
                    amp_env: env(0.002, 1.2, 0.3, 0.15),
                    vel_to_amp: 0.7,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.3,
                    octave: 1,
                    hpf: 600.0,
                    lpf: 2400.0,
                    filter_env: env(0.001, 0.15, 0.1, 0.15),
                    env_amount: 2.0,
                    key_track: 1.0,
                    amp_env: env(0.002, 0.6, 0.2, 0.15),
                    level: 0.35,
                    ..Layer::default()
                },
            ],
            drift: 0.0,
            spread: 0.3,
            ..named("Neon Clav", Category::Keys)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    pulse: 0.25,
                    width: 0.5,
                    lpf: 3500.0,
                    env_amount: 0.0,
                    amp_env: env(0.008, 1.0, 1.0, 0.25),
                    vel_to_amp: 0.2,
                    pan: -0.2,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.8,
                    octave: 1,
                    lpf: 6000.0,
                    env_amount: 0.0,
                    amp_env: env(0.008, 1.0, 1.0, 0.25),
                    level: 0.45,
                    vel_to_amp: 0.2,
                    pan: 0.2,
                    ..Layer::default()
                },
            ],
            tremolo: 0.12,
            lfo_rate: 6.2,
            lfo_delay: 0.0,
            drift: 0.0,
            touch_vibrato: 0.0,
            wheel_vibrato: 0.0,
            spread: 0.3,
            gain_db: -5.0,
            ..named("Velour Organ", Category::Keys)
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    lpf: 9000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 3.5, 0.0, 2.5),
                    vel_to_amp: 0.6,
                    ..Layer::default()
                },
                // ring-modulated against its octave: sum and difference land on the
                // 1st and 3rd harmonics, so the clang stays in tune
                Layer {
                    saw: 0.0,
                    sine: 1.0,
                    octave: 1,
                    lpf: 9000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 1.8, 0.0, 1.5),
                    level: 0.7,
                    vel_to_amp: 0.6,
                    ..Layer::default()
                },
            ],
            ring: 0.85,
            gain_db: -6.0,
            ..bell("Ring Bells")
        },
        Patch {
            layers: [
                Layer {
                    saw: 0.5,
                    pulse: 0.6,
                    width: 0.25,
                    lpf: 1200.0,
                    lpf_res: 0.15,
                    filter_env: env(0.001, 0.6, 0.15, 0.8),
                    env_amount: 2.4,
                    key_track: 1.0,
                    amp_env: env(0.001, 2.5, 0.0, 1.5),
                    vel_to_amp: 0.6,
                    pan: -0.25,
                    ..Layer::default()
                },
                Layer {
                    saw: 0.0,
                    sine: 0.7,
                    octave: 1,
                    lpf: 5000.0,
                    env_amount: 0.0,
                    amp_env: env(0.001, 1.4, 0.0, 1.2),
                    level: 0.35,
                    pan: 0.25,
                    ..Layer::default()
                },
            ],
            drift: 1.0,
            spread: 0.6,
            ..named("Rain Harp", Category::Pluck)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    lpf: 160.0,
                    lpf_res: 0.35,
                    filter_env: env(0.03, 0.5, 0.35, 0.4),
                    env_amount: 3.4,
                    vel_to_filter: 1.4,
                    amp_env: env(0.02, 1.2, 0.85, 0.4),
                    drive: 0.3,
                    ..saw
                },
                Layer {
                    octave: -1,
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.4,
                    lpf: 140.0,
                    lpf_res: 0.25,
                    filter_env: env(0.05, 0.6, 0.35, 0.4),
                    env_amount: 3.0,
                    amp_env: env(0.03, 1.2, 0.85, 0.4),
                    detune: 6.0,
                    level: 0.6,
                    ..Layer::default()
                },
            ],
            mono: true,
            glide: 0.04,
            sag: -20.0,
            sag_time: 0.5,
            spread: 0.0,
            drift: 2.0,
            ..named("Brass Bass", Category::Bass)
        },
        Patch {
            layers: [
                Layer {
                    octave: -1,
                    lpf: 180.0,
                    lpf_res: 0.55,
                    env_amount: 0.0,
                    amp_env: env(4.0, 6.0, 1.0, 8.0),
                    detune: -7.0,
                    pan: -0.5,
                    ..saw
                },
                Layer {
                    saw: 0.0,
                    pulse: 1.0,
                    width: 0.3,
                    pwm: 0.4,
                    lpf: 420.0,
                    lpf_res: 0.4,
                    env_amount: 0.0,
                    amp_env: env(6.0, 6.0, 1.0, 8.0),
                    detune: 7.0,
                    pan: 0.5,
                    level: 0.6,
                    ..Layer::default()
                },
            ],
            lfo_shape: LfoShape::Sine,
            lfo_rate: 0.06,
            lfo_to_filter: 1.2,
            lfo_delay: 0.0,
            drift: 5.0,
            spread: 0.8,
            ..named("Sodium Drone", Category::Drone)
        },
        Patch::default(),
    ]
}

/// A factory patch by name, or the init patch.
pub fn by_name(name: &str) -> Patch {
    factory().into_iter().find(|p| p.name == name).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_names_are_unique() {
        let f = factory();
        let mut names: Vec<&str> = f.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), f.len());
    }

    #[test]
    fn patches_round_trip_through_json() {
        for p in factory() {
            let s = serde_json::to_string(&p).unwrap();
            assert_eq!(serde_json::from_str::<Patch>(&s).unwrap(), p);
        }
    }
}
