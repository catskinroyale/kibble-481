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
