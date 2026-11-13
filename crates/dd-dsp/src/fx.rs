//! Effect settings: the reverb space, the echo, the ensemble and tuning.
//!
//! Copied from Ambient MIDI Generator's `song.rs`
//! (https://github.com/catskinroyale/ambient-midi-generator, MIT) so that its reverb,
//! echo and ensemble carry over unchanged. Keep the two in step until the repos merge.

use serde::{Deserialize, Serialize};

/// The reverb's character.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    Room,
    Hall,
    Cathedral,
    Shimmer,
    BlackHole,
    Plate,
    Cloud,
    Noir,
}

impl Space {
    pub const ALL: [Space; 8] =
        [Space::Room, Space::Plate, Space::Hall, Space::Cathedral, Space::Noir, Space::Shimmer, Space::Cloud, Space::BlackHole];

    pub fn label(self) -> &'static str {
        match self {
            Space::Room => "Room",
            Space::Hall => "Hall",
            Space::Cathedral => "Cathedral",
            Space::Shimmer => "Shimmer",
            Space::BlackHole => "Black Hole",
            Space::Plate => "Plate",
            Space::Cloud => "Cloud",
            Space::Noir => "Noir",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Space::Room => "A small, close room with a short tail",
            Space::Hall => "A concert hall, about three seconds long",
            Space::Cathedral => "A large stone space with a long, dark tail",
            Space::Shimmer => "A long hall whose tail rises an octave, like a halo of high strings",
            Space::BlackHole => "Enormous, slowly swirling and almost endless: notes dissolve into a cloud",
            Space::Plate => "A bright, dense studio plate that blooms at once, with no sense of walls",
            Space::Cloud => "Granular: the reverb is fed scattered, slightly detuned fragments of what was just played",
            Space::Noir => "A vast, dark 1980s digital hall: a long pause before it blooms, then a slow, chorused tail",
        }
    }
}

/// Reference pitch for A above middle C.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tuning {
    A440,
    A432,
    A415,
}

impl Tuning {
    pub const ALL: [Tuning; 3] = [Tuning::A440, Tuning::A432, Tuning::A415];

    pub fn label(self) -> &'static str {
        match self {
            Tuning::A440 => "A = 440 Hz (standard)",
            Tuning::A432 => "A = 432 Hz (warmer)",
            Tuning::A415 => "A = 415 Hz (baroque)",
        }
    }

    /// Offset from standard pitch in cents.
    pub fn cents(self) -> f32 {
        match self {
            Tuning::A440 => 0.0,
            Tuning::A432 => 1200.0 * (432f32 / 440.0).log2(),
            Tuning::A415 => 1200.0 * (415f32 / 440.0).log2(),
        }
    }
}

/// Playback effects: the reverb space and tuning. All amounts are 0..1 with
/// 0.5 as the space's natural setting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fx {
    pub space: Space,
    /// Reverb length: 0 = half, 1 = double the space's natural decay.
    pub size: f32,
    /// Reverb brightness: 0 = dark, 1 = bright.
    pub tone: f32,
    /// Reverb modulation: 0 = still, 1 = swirling.
    pub motion: f32,
    /// How far each part and note is pulled off pitch: 0 = in tune.
    pub detune: f32,
    /// Slow tape-like pitch wander: 0 = steady.
    pub drift: f32,
    pub tuning: Tuning,
    /// Shimmer feedback: how much of the octave-up tail is fed back in.
    pub shimmer: f32,
    /// Detunes the shimmer feedback: 0 = a pure octave; higher splits it into
    /// a sharp and a flat copy that beat against each other.
    pub shimmer_detune: f32,
    /// The interval the shimmer feedback is pitched by.
    pub shimmer_interval: Interval,
    /// Extra delay before the reverb starts: 0 = the space's own, 1 = a quarter second more.
    pub predelay: f32,
    /// How much low end is kept out of the reverb: 0 = 40 Hz, 0.5 = 160 Hz, 1 = 640 Hz.
    pub low_cut: f32,
    /// Stereo width of the reverb: 0 = mono, 0.5 = natural, 1 = extra wide.
    pub width: f32,
    /// How far the reverb ducks under the music while it plays, so the tail
    /// blooms in the gaps: 0 = off.
    pub duck: f32,
    /// Hold the reverb's tail forever and let nothing new in.
    pub freeze: bool,
    /// The echo: how much of each part goes into it is set on the mixer.
    pub echo: Echo,
    /// A string-machine ensemble chorus over everything but the bass: 0 = off.
    pub ensemble: f32,
}

/// The shimmer feedback's pitch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interval {
    OctaveUp,
    FifthUp,
    TwoOctavesUp,
    OctaveDown,
}

impl Interval {
    pub const ALL: [Interval; 4] = [Interval::OctaveUp, Interval::FifthUp, Interval::TwoOctavesUp, Interval::OctaveDown];

    pub fn label(self) -> &'static str {
        match self {
            Interval::OctaveUp => "Octave up",
            Interval::FifthUp => "Fifth up",
            Interval::TwoOctavesUp => "Two octaves up",
            Interval::OctaveDown => "Octave down",
        }
    }

    pub fn ratio(self) -> f32 {
        match self {
            Interval::OctaveUp => 2.0,
            Interval::FifthUp => 1.5,
            Interval::TwoOctavesUp => 4.0,
            Interval::OctaveDown => 0.5,
        }
    }
}

/// A tempo-synced echo, fed from the mixer's echo sends.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Echo {
    pub time: EchoTime,
    /// How many repeats: 0 = one, 1 = almost endless.
    pub feedback: f32,
    /// Repeats get darker (0) or stay bright (1).
    pub tone: f32,
    /// Repeats bounce between left and right.
    pub ping_pong: bool,
    /// How much of the echo goes on into the reverb.
    pub to_reverb: f32,
    /// Tape-like wobble on the repeats.
    pub wow: f32,
}

impl Default for Echo {
    fn default() -> Self {
        Echo { time: EchoTime::DottedEighth, feedback: 0.45, tone: 0.4, ping_pong: true, to_reverb: 0.5, wow: 0.2 }
    }
}

/// Echo time in beats (quarter notes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EchoTime {
    Eighth,
    DottedEighth,
    Quarter,
    DottedQuarter,
    Half,
}

impl EchoTime {
    pub const ALL: [EchoTime; 5] = [EchoTime::Eighth, EchoTime::DottedEighth, EchoTime::Quarter, EchoTime::DottedQuarter, EchoTime::Half];

    pub fn label(self) -> &'static str {
        match self {
            EchoTime::Eighth => "1/8",
            EchoTime::DottedEighth => "Dotted 1/8",
            EchoTime::Quarter => "1/4",
            EchoTime::DottedQuarter => "Dotted 1/4",
            EchoTime::Half => "1/2",
        }
    }

    pub fn beats(self) -> f64 {
        match self {
            EchoTime::Eighth => 0.5,
            EchoTime::DottedEighth => 0.75,
            EchoTime::Quarter => 1.0,
            EchoTime::DottedQuarter => 1.5,
            EchoTime::Half => 2.0,
        }
    }
}

impl Default for Fx {
    fn default() -> Self {
        Fx {
            space: Space::Hall,
            size: 0.5,
            tone: 0.5,
            motion: 0.5,
            detune: 0.0,
            drift: 0.0,
            tuning: Tuning::A440,
            shimmer: 0.5,
            shimmer_detune: 0.0,
            shimmer_interval: Interval::OctaveUp,
            predelay: 0.0,
            low_cut: 0.5,
            width: 0.5,
            duck: 0.0,
            freeze: false,
            echo: Echo::default(),
            ensemble: 0.0,
        }
    }
}
