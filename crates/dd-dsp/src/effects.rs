//! The shared effects around the reverb: a string-machine ensemble chorus and a
//! tempo-synced tape echo, plus the master soft clipper.
//!
//! Lifted from Ambient MIDI Generator's `mixer.rs` and `synth.rs`
//! (https://github.com/catskinroyale/ambient-midi-generator, MIT); made public, otherwise
//! unchanged.

use std::f32::consts::TAU;

use crate::fx::Echo;

/// Gentle saturation that leaves everything below 0.6 untouched.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x.abs() < 0.6 { x } else { x.signum() * (0.6 + 0.4 * ((x.abs() - 0.6) / 0.4).tanh()) }
}

/// A string-machine ensemble: three delays per side, swept by a slow and a
/// fast wobble a third of a cycle apart, so one voice sounds like several
/// slightly out-of-tune ones. Left and right sweep out of step, for width.
pub struct Ensemble {
    sr: f32,
    lines: [Vec<f32>; 2],
    idx: usize,
    slow: f32,
    fast: f32,
    /// The amount, gliding to `target`.
    pub amount: f32,
    pub target: f32,
    lp: [f32; 2],
    lp_coef: f32,
}

impl Ensemble {
    pub fn new(sr: f32) -> Self {
        let len = (sr * 0.02) as usize;
        Ensemble {
            sr,
            lines: [vec![0.0; len], vec![0.0; len]],
            idx: 0,
            slow: 0.0,
            fast: 0.0,
            amount: 0.0,
            target: 0.0,
            lp: [0.0; 2],
            lp_coef: (-TAU * 7000.0 / sr).exp(),
        }
    }

    /// Off, and not on its way in or out.
    pub fn active(&self) -> bool {
        self.target > 0.0 || self.amount > 1e-4
    }

    pub fn clear(&mut self) {
        for l in self.lines.iter_mut() {
            l.fill(0.0);
        }
        self.lp = [0.0; 2];
        self.amount = self.target;
    }

    #[inline]
    fn read(&self, side: usize, d: f32) -> f32 {
        let line = &self.lines[side];
        let n = line.len();
        let pos = self.idx as f32 + n as f32 - d;
        let i = pos as usize;
        let f = pos - i as f32;
        let a = line[i % n];
        let b = line[(i + 1) % n];
        a + (b - a) * f
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        self.amount += (self.target - self.amount) * 0.0005;
        let n = self.lines[0].len();
        self.lines[0][self.idx] = l;
        self.lines[1][self.idx] = r;
        self.slow = (self.slow + 0.6 / self.sr).fract();
        self.fast = (self.fast + 6.1 / self.sr).fract();
        let a = self.amount;
        let ms = self.sr * 0.001;
        let (base, slow, fast) = (6.0 * ms, (0.6 + 1.6 * a) * ms, (0.1 + 0.15 * a) * ms);
        let mut out = [0.0f32; 2];
        for (s, o) in out.iter_mut().enumerate() {
            let mut sum = 0.0;
            for k in 0..3 {
                let off = k as f32 / 3.0 + s as f32 / 6.0;
                let d = base + slow * ((self.slow + off) * TAU).sin() + fast * ((self.fast + off) * TAU).sin();
                sum += self.read(s, d);
            }
            // the bucket-brigade chips' dull top end
            // (the taps add up mostly out of phase, so scale them by the root of their number)
            self.lp[s] = self.lp_coef * self.lp[s] + (1.0 - self.lp_coef) * sum * 0.577;
            *o = self.lp[s];
        }
        self.idx = (self.idx + 1) % n;
        // the dry level falls as the chorus comes in, keeping the loudness about even
        let (dry, wet) = (1.0 - 0.4 * a, 0.9 * a);
        (l * dry + out[0] * wet, r * dry + out[1] * wet)
    }
}

/// The echo: a stereo, tempo-synced delay with darkening, slightly wobbling
/// repeats, which can bounce from side to side.
pub struct EchoFx {
    sr: f32,
    lines: [Vec<f32>; 2],
    idx: usize,
    /// Delay in samples, and the value it glides to when the tempo or time changes.
    delay: f32,
    pub target: f32,
    beats: f64,
    feedback: f32,
    ping_pong: bool,
    lp: [f32; 2],
    lp_coef: f32,
    hp: [f32; 2],
    hp_coef: f32,
    wow: f32,
    phase: f32,
    pub to_reverb: f32,
}

/// The longest echo, in seconds.
const MAX_ECHO_SECS: f32 = 3.0;

impl EchoFx {
    pub fn new(sr: f32) -> Self {
        let len = (sr * (MAX_ECHO_SECS + 0.1)) as usize;
        let mut e = EchoFx {
            sr,
            lines: [vec![0.0; len], vec![0.0; len]],
            idx: 0,
            delay: sr * 0.75,
            target: sr * 0.75,
            beats: 0.75,
            feedback: 0.4,
            ping_pong: true,
            lp: [0.0; 2],
            lp_coef: 0.0,
            hp: [0.0; 2],
            hp_coef: (-TAU * 140.0 / sr).exp(),
            wow: 0.0,
            phase: 0.0,
            to_reverb: 0.5,
        };
        e.set(&Echo::default(), 60.0);
        e.delay = e.target;
        e
    }

    pub fn set(&mut self, echo: &Echo, bpm: f64) {
        self.beats = echo.time.beats();
        self.feedback = echo.feedback.clamp(0.0, 1.0) * 0.88;
        self.ping_pong = echo.ping_pong;
        self.lp_coef = (-TAU * 1500.0 * 8f32.powf(echo.tone.clamp(0.0, 1.0)) / self.sr).exp();
        self.wow = echo.wow.clamp(0.0, 1.0);
        self.to_reverb = echo.to_reverb.clamp(0.0, 1.0);
        self.set_tempo(bpm);
    }

    pub fn set_tempo(&mut self, bpm: f64) {
        let secs = (self.beats * 60.0 / bpm.max(1.0)) as f32;
        self.target = secs.min(MAX_ECHO_SECS) * self.sr;
    }

    pub fn clear(&mut self) {
        for l in self.lines.iter_mut() {
            l.fill(0.0);
        }
        self.lp = [0.0; 2];
        self.hp = [0.0; 2];
        self.delay = self.target;
    }

    #[inline]
    fn read(&self, side: usize, d: f32) -> f32 {
        let line = &self.lines[side];
        let n = line.len();
        let pos = self.idx as f32 + n as f32 - d;
        let i = pos as usize;
        let f = pos - i as f32;
        let a = line[i % n];
        let b = line[(i + 1) % n];
        a + (b - a) * f
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // glide to a new time slowly, like turning a tape echo's speed knob
        self.delay += (self.target - self.delay) * 0.0002;
        self.phase = (self.phase + 0.55 / self.sr).fract();
        let wobble = (self.phase * TAU).sin() * self.wow * 0.004 * self.sr;
        let d = (self.delay + wobble).clamp(1.0, self.lines[0].len() as f32 - 2.0);
        let out = [self.read(0, d), self.read(1, d)];
        let mut fb = [0.0f32; 2];
        for s in 0..2 {
            self.lp[s] = self.lp_coef * self.lp[s] + (1.0 - self.lp_coef) * out[s];
            self.hp[s] = self.hp_coef * self.hp[s] + (1.0 - self.hp_coef) * self.lp[s];
            fb[s] = (self.lp[s] - self.hp[s]) * self.feedback;
        }
        let (in_l, in_r) = if self.ping_pong {
            // a mono input starts on the left; each repeat crosses over
            ((l + r) * 0.5 + fb[1], fb[0])
        } else {
            (l + fb[0], r + fb[1])
        };
        let n = self.lines[0].len();
        self.lines[0][self.idx] = in_l.tanh();
        self.lines[1][self.idx] = in_r.tanh();
        self.idx = (self.idx + 1) % n;
        (out[0], out[1])
    }
}

/// Below this a low cut is off.
pub const LOW_CUT_OFF: f32 = 20.0;
/// At or above this a high cut is off.
pub const HIGH_CUT_OFF: f32 = 20_000.0;

/// One channel of a 12 dB/octave Butterworth state-variable filter (Andrew Simper's
/// zero-delay-feedback design), giving its low- and high-pass outputs.
#[derive(Clone, Copy, Default, Debug)]
struct Pole2 {
    ic1: f32,
    ic2: f32,
}

#[derive(Clone, Copy, Default, Debug)]
struct Coefs {
    a1: f32,
    a2: f32,
    a3: f32,
}

const BUTTERWORTH_K: f32 = std::f32::consts::SQRT_2;

impl Coefs {
    fn new(hz: f32, sr: f32) -> Coefs {
        let g = (std::f32::consts::PI * hz.clamp(10.0, sr * 0.45) / sr).tan();
        let a1 = 1.0 / (1.0 + g * (g + BUTTERWORTH_K));
        let a2 = g * a1;
        Coefs { a1, a2, a3: g * a2 }
    }
}

impl Pole2 {
    /// (low, high)
    #[inline]
    fn process(&mut self, c: &Coefs, x: f32) -> (f32, f32) {
        let v3 = x - self.ic2;
        let v1 = c.a1 * self.ic1 + c.a2 * v3;
        let v2 = self.ic2 + c.a2 * self.ic1 + c.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, x - BUTTERWORTH_K * v1 - v2)
    }
}

/// A track's low cut and high cut, on its output, so it works on any sound: the
/// synth, the kits and SoundFonts alike. Each is 12 dB/octave and off at its end of
/// the range ([`LOW_CUT_OFF`], [`HIGH_CUT_OFF`]). Allocation-free.
#[derive(Clone, Copy, Debug)]
pub struct CutFilter {
    sr: f32,
    low_hz: f32,
    high_hz: f32,
    low: Coefs,
    high: Coefs,
    hp: [Pole2; 2],
    lp: [Pole2; 2],
}

impl CutFilter {
    pub fn new(sr: f32) -> CutFilter {
        CutFilter {
            sr,
            low_hz: LOW_CUT_OFF,
            high_hz: HIGH_CUT_OFF,
            low: Coefs::default(),
            high: Coefs::default(),
            hp: Default::default(),
            lp: Default::default(),
        }
    }

    /// Set the cut-offs in Hz (cheap when they haven't changed).
    pub fn set(&mut self, low_hz: f32, high_hz: f32) {
        if low_hz != self.low_hz {
            if self.low_hz <= LOW_CUT_OFF {
                self.hp = Default::default();
            }
            self.low_hz = low_hz;
            self.low = Coefs::new(low_hz, self.sr);
        }
        if high_hz != self.high_hz {
            if self.high_hz >= HIGH_CUT_OFF {
                self.lp = Default::default();
            }
            self.high_hz = high_hz;
            self.high = Coefs::new(high_hz, self.sr);
        }
    }

    pub fn active(&self) -> bool {
        self.low_hz > LOW_CUT_OFF || self.high_hz < HIGH_CUT_OFF
    }

    /// Filter a block in place.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let (lo, hi) = (self.low_hz > LOW_CUT_OFF, self.high_hz < HIGH_CUT_OFF);
        for (ch, buf) in [l, r].into_iter().enumerate() {
            for x in buf.iter_mut() {
                if lo {
                    *x = self.hp[ch].process(&self.low, *x).1;
                }
                if hi {
                    *x = self.lp[ch].process(&self.high, *x).0;
                }
            }
        }
    }
}

#[cfg(test)]
mod cut_tests {
    use super::*;

    fn level(hz: f32, low: f32, high: f32) -> f32 {
        let sr = 48_000.0;
        let mut f = CutFilter::new(sr);
        f.set(low, high);
        let n = 48_000;
        let mut l: Vec<f32> = (0..n).map(|i| (TAU * hz * i as f32 / sr).sin()).collect();
        let mut r = l.clone();
        f.process(&mut l, &mut r);
        l[n / 2..].iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    #[test]
    fn cuts_where_asked_and_leaves_the_rest() {
        // off: untouched
        assert!((level(50.0, LOW_CUT_OFF, HIGH_CUT_OFF) - 1.0).abs() < 1e-3);
        // a low cut at 200 Hz: -3 dB there, about -24 dB two octaves below, the top kept
        assert!((level(200.0, 200.0, HIGH_CUT_OFF) - 0.707).abs() < 0.03);
        assert!(level(50.0, 200.0, HIGH_CUT_OFF) < 0.07);
        assert!(level(4000.0, 200.0, HIGH_CUT_OFF) > 0.99);
        // a high cut at 2 kHz
        assert!((level(2000.0, LOW_CUT_OFF, 2000.0) - 0.707).abs() < 0.03);
        assert!(level(8000.0, LOW_CUT_OFF, 2000.0) < 0.07);
        assert!(level(100.0, LOW_CUT_OFF, 2000.0) > 0.99);
    }
}
