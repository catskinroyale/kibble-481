//! The polysynth: sixteen voices of a two-layer patch (see `patch.rs`).
//!
//! Each layer of each voice runs band-limited saw and pulse oscillators (polyBLEP), a
//! sine that can be frequency-modulated, and noise, through a resonant high-pass and
//! low-pass (zero-delay-feedback state-variable filters, 12 dB/octave like the CS-80's)
//! and an amplifier, each with its own envelope. Pitch, cutoffs and levels are worked
//! out at a control rate of one in every `CTRL` samples and glide in between.

use std::f32::consts::{PI, TAU};

use crate::Instrument;
use crate::patch::{Env, LfoShape, Patch};

const VOICES: usize = 16;
/// Samples between control-rate updates.
const CTRL: usize = 16;

#[inline]
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

/// Coefficient for a one-pole fall of 40 dB (×0.01) in `secs`.
#[inline]
fn fall(secs: f32, sr: f32) -> f32 {
    (-4.6 / (secs.max(0.0005) * sr)).exp()
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Off,
}

/// An envelope advanced once per control block.
#[derive(Clone, Copy, Debug)]
struct EnvState {
    stage: Stage,
    level: f32,
}

impl EnvState {
    const OFF: EnvState = EnvState { stage: Stage::Off, level: 0.0 };

    fn start(&mut self) {
        self.stage = Stage::Attack;
    }

    fn release(&mut self) {
        if self.stage != Stage::Off {
            self.stage = Stage::Release;
        }
    }

    /// Advance by `dt` seconds (a control block).
    fn step(&mut self, e: &Env, dt: f32, sr: f32) -> f32 {
        let blocks = dt * sr;
        match self.stage {
            Stage::Attack => {
                // a slightly convex attack, like a capacitor charging towards 1.3
                let rate = dt / e.attack.max(0.0005);
                self.level += (1.3 - self.level) * (1.0 - (-rate * 1.2).exp());
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let k = fall(e.decay, sr).powf(blocks);
                self.level = e.sustain + (self.level - e.sustain) * k;
                if (self.level - e.sustain).abs() < 1e-4 {
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = e.sustain,
            Stage::Release => {
                self.level *= fall(e.release, sr).powf(blocks);
                if self.level < 1e-4 {
                    self.level = 0.0;
                    self.stage = Stage::Off;
                }
            }
            Stage::Off => self.level = 0.0,
        }
        self.level
    }
}

/// A zero-delay-feedback state-variable filter (after Andrew Simper's design).
#[derive(Clone, Copy, Default, Debug)]
struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    fn set(&mut self, hz: f32, res: f32, sr: f32) {
        let hz = hz.clamp(16.0, sr * 0.45);
        let g = (PI * hz / sr).tan();
        self.k = 2.0 - 1.96 * res.clamp(0.0, 1.0);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Returns (low, high).
    #[inline]
    fn process(&mut self, x: f32) -> (f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        (v2, x - self.k * v1 - v2)
    }

    fn clear(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }
}

#[derive(Clone, Copy, Debug)]
struct LayerVoice {
    phase: f32,
    sine_phase: f32,
    mod_phase: f32,
    /// FM depth envelope, 1 at the note's start.
    fm_env: f32,
    amp: EnvState,
    filt: EnvState,
    hp: Svf,
    lp: Svf,
    /// Per-sample increments and levels, ramped across a control block.
    inc: f32,
    gain: f32,
    gain_step: f32,
    width: f32,
}

impl LayerVoice {
    fn new() -> Self {
        LayerVoice {
            phase: 0.0,
            sine_phase: 0.0,
            mod_phase: 0.0,
            fm_env: 0.0,
            amp: EnvState::OFF,
            filt: EnvState::OFF,
            hp: Svf::default(),
            lp: Svf::default(),
            inc: 0.0,
            gain: 0.0,
            gain_step: 0.0,
            width: 0.5,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    key: u8,
    vel: f32,
    /// Held down on the keyboard.
    gate: bool,
    /// Released while the sustain pedal is down.
    pedalled: bool,
    /// When it started, for stealing the oldest.
    age: u64,
    /// The pitch now and the pitch it glides to, in (fractional) MIDI keys.
    pitch: f32,
    target: f32,
    /// Seconds since the note started, and since it was released.
    held: f32,
    released: f32,
    /// This voice's fixed detune and drift.
    offset: f32,
    drift_phase: f32,
    drift_rate: f32,
    pan: f32,
    layers: [LayerVoice; 2],
}

impl Voice {
    fn idle() -> Self {
        Voice {
            key: 0,
            vel: 0.0,
            gate: false,
            pedalled: false,
            age: 0,
            pitch: 60.0,
            target: 60.0,
            held: 0.0,
            released: 0.0,
            offset: 0.0,
            drift_phase: 0.0,
            drift_rate: 0.1,
            pan: 0.0,
            layers: [LayerVoice::new(); 2],
        }
    }

    fn active(&self) -> bool {
        self.layers.iter().any(|l| l.amp.stage != Stage::Off)
    }

    fn release(&mut self) {
        self.gate = false;
        self.released = 0.0;
        for l in self.layers.iter_mut() {
            l.amp.release();
            l.filt.release();
        }
    }
}

/// The CS-style polysynth.
pub struct PolySynth {
    sr: f32,
    patch: Patch,
    voices: [Voice; VOICES],
    clock: u64,
    seed: u32,
    lfo_phase: f32,
    lfo_value: f32,
    lfo_hold: f32,
    /// Keys held (mono mode), oldest first.
    held: Vec<(u8, f32)>,
    last_pitch: Option<f32>,
    // controllers
    bend: f32,
    wheel: f32,
    pressure: f32,
    pressure_smooth: f32,
    volume: f32,
    expression: f32,
    brightness: f32,
    pedal: bool,
    tuning: f32,
    out_gain: f32,
    /// Counts down to the next control update.
    until_ctrl: usize,
}

impl PolySynth {
    pub fn new(sample_rate: u32, patch: Patch) -> Self {
        let mut s = PolySynth {
            sr: sample_rate as f32,
            patch,
            voices: [Voice::idle(); VOICES],
            clock: 0,
            seed: 0x9e37_79b9,
            lfo_phase: 0.0,
            lfo_value: 0.0,
            lfo_hold: 0.0,
            held: Vec::new(),
            last_pitch: None,
            bend: 0.0,
            wheel: 0.0,
            pressure: 0.0,
            pressure_smooth: 0.0,
            volume: 1.0,
            expression: 1.0,
            brightness: 0.0,
            pedal: false,
            tuning: 0.0,
            out_gain: 1.0,
            until_ctrl: 0,
        };
        s.update_gain();
        s
    }

    pub fn patch(&self) -> &Patch {
        &self.patch
    }

    fn update_gain(&mut self) {
        self.out_gain = 0.2 * 10f32.powf(self.patch.gain_db / 20.0);
    }

    fn rand(&mut self) -> f32 {
        // xorshift32
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed as f32 / u32::MAX as f32
    }

    fn start_voice(&mut self, v: usize, key: u8, vel: f32, from: Option<f32>) {
        let (spread, drift, glides) = (self.patch.spread, self.patch.drift, self.patch.glide > 0.0);
        let (r1, r2, r3) = (self.rand(), self.rand(), self.rand());
        let target = key as f32;
        let pitch = match (from, glides) {
            (Some(f), true) => f,
            _ => target,
        };
        let voice = &mut self.voices[v];
        let restart = !voice.active();
        *voice = Voice {
            key,
            vel,
            gate: true,
            pedalled: false,
            age: self.clock,
            pitch,
            target,
            held: 0.0,
            released: 0.0,
            offset: (r1 - 0.5) * drift * 0.6,
            drift_phase: r2,
            drift_rate: 0.07 + 0.2 * r3,
            pan: (if v.is_multiple_of(2) { -1.0 } else { 1.0 }) * spread * (0.3 + 0.7 * r3),
            layers: voice.layers,
        };
        for (l, lv) in voice.layers.iter_mut().enumerate() {
            lv.amp.start();
            lv.filt.start();
            lv.fm_env = 1.0;
            if restart {
                lv.hp.clear();
                lv.lp.clear();
                lv.amp.level = 0.0;
                lv.filt.level = 0.0;
                // free-running oscillators: start each at its own phase
                lv.phase = (r1 * 7.0 + l as f32 * 0.37).fract();
                lv.sine_phase = 0.0;
                lv.mod_phase = 0.0;
                lv.gain = 0.0;
            }
        }
        self.last_pitch = Some(target);
    }

    fn free_voice(&self) -> usize {
        if let Some(i) = self.voices.iter().position(|v| !v.active()) {
            return i;
        }
        // steal the oldest released voice, else the oldest
        self.voices.iter().enumerate().min_by_key(|(_, v)| (v.gate || v.pedalled, v.age)).map(|(i, _)| i).unwrap_or(0)
    }

    fn mono_on(&mut self, key: u8, vel: f32) {
        self.held.retain(|(k, _)| *k != key);
        self.held.push((key, vel));
        let v0 = &self.voices[0];
        if v0.gate && v0.active() {
            // legato: glide to the new note without restarting the envelopes
            let v = &mut self.voices[0];
            v.key = key;
            v.target = key as f32;
            if self.patch.glide <= 0.0 {
                v.pitch = v.target;
            }
            self.last_pitch = Some(key as f32);
        } else {
            let from = if v0.active() { Some(v0.pitch) } else { self.last_pitch };
            self.start_voice(0, key, vel, from);
        }
    }

    fn mono_off(&mut self, key: u8) {
        self.held.retain(|(k, _)| *k != key);
        let v = &mut self.voices[0];
        if v.key != key {
            return;
        }
        if let Some(&(prev, _)) = self.held.last() {
            v.key = prev;
            v.target = prev as f32;
            if self.patch.glide <= 0.0 {
                v.pitch = v.target;
            }
        } else if self.pedal {
            v.pedalled = true;
        } else {
            v.release();
        }
    }

    fn lfo_step(&mut self, dt: f32) {
        let p = &self.patch;
        let before = self.lfo_phase;
        self.lfo_phase = (self.lfo_phase + p.lfo_rate.max(0.0) * dt).fract();
        let wrapped = self.lfo_phase < before;
        let ph = self.lfo_phase;
        self.lfo_value = match p.lfo_shape {
            LfoShape::Sine => (ph * TAU).sin(),
            LfoShape::Triangle => 1.0 - 4.0 * (ph - 0.5).abs(),
            LfoShape::Saw => 2.0 * ph - 1.0,
            LfoShape::Square => {
                if ph < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoShape::Random => {
                if wrapped {
                    self.lfo_hold = self.rand() * 2.0 - 1.0;
                }
                self.lfo_hold
            }
        };
    }

    /// Work out every voice's pitch, cutoffs and levels for the next block.
    fn update_controls(&mut self, block: usize) {
        let dt = block as f32 / self.sr;
        self.lfo_step(dt);
        self.pressure_smooth += (self.pressure - self.pressure_smooth) * (1.0 - (-dt * 30.0).exp());
        let sr = self.sr;
        let p = &self.patch;
        let lfo = self.lfo_value;
        let touch = self.pressure_smooth;
        let glide_k = if p.glide > 0.0 { 1.0 - (-dt * 4.0 / p.glide).exp() } else { 1.0 };
        let tremolo = 1.0 - p.tremolo * 0.5 * (1.0 + lfo);
        let master = self.volume * self.expression;
        for v in self.voices.iter_mut() {
            if !v.active() {
                continue;
            }
            v.held += dt;
            if !v.gate {
                v.released += dt;
            }
            v.pitch += (v.target - v.pitch) * glide_k;
            v.drift_phase = (v.drift_phase + v.drift_rate * dt).fract();
            let drift = p.drift * 0.5 * ((v.drift_phase * TAU).sin() + 0.5 * (v.drift_phase * TAU * 2.7 + 1.3).sin());
            // vibrato fades in after the LFO delay
            let fade = if p.lfo_delay > 0.0 { ((v.held - p.lfo_delay) / p.lfo_delay.max(0.2)).clamp(0.0, 1.0) } else { 1.0 };
            let vib_depth = p.vibrato * fade + p.wheel_vibrato * self.wheel + p.touch_vibrato * touch;
            let sag = if v.gate { 0.0 } else { p.sag * (v.released / p.sag_time.max(0.05)).min(1.0) };
            let cents = v.offset + drift + vib_depth * lfo + sag + self.bend * p.bend_range * 100.0 + p.touch_bend * touch + self.tuning;
            let base = v.pitch + cents / 100.0;
            let key_oct = (v.key as f32 - 60.0) / 12.0;
            for (lv, lp) in v.layers.iter_mut().zip(p.layers.iter()) {
                if !lp.on {
                    lv.amp.stage = Stage::Off;
                    lv.gain = 0.0;
                    lv.gain_step = 0.0;
                    continue;
                }
                let semis = base + lp.octave as f32 * 12.0 + lp.detune / 100.0;
                let hz = 440.0 * 2f32.powf((semis - 69.0) / 12.0);
                lv.inc = (hz / sr).min(0.45);
                let fenv = lv.filt.step(&lp.filter_env, dt, sr);
                let aenv = lv.amp.step(&lp.amp_env, dt, sr);
                if lp.fm_decay > 0.0 {
                    lv.fm_env *= fall(lp.fm_decay, sr).powf(block as f32);
                }
                let octaves = fenv * lp.env_amount
                    + lp.key_track * key_oct
                    + lp.vel_to_filter * (v.vel - 0.7)
                    + p.lfo_to_filter * lfo
                    + p.touch_filter * touch
                    + self.brightness;
                lv.lp.set(lp.lpf * 2f32.powf(octaves), lp.lpf_res, sr);
                lv.hp.set(lp.hpf, lp.hpf_res, sr);
                lv.width = (lp.width + lp.pwm * 0.45 * lfo).clamp(0.04, 0.96);
                let vel_gain = 1.0 - lp.vel_to_amp + lp.vel_to_amp * v.vel;
                let target = aenv * vel_gain * lp.level * tremolo * master;
                lv.gain_step = (target - lv.gain) / block as f32;
            }
        }
    }
}

impl Instrument for PolySynth {
    fn note_on(&mut self, key: u8, vel: u8) {
        if vel == 0 {
            return self.note_off(key);
        }
        let vel = vel as f32 / 127.0;
        self.clock += 1;
        if self.patch.mono {
            return self.mono_on(key, vel);
        }
        // retrigger a key that is still sounding rather than stacking it
        let i = self.voices.iter().position(|v| v.active() && v.key == key).unwrap_or_else(|| self.free_voice());
        let from = self.last_pitch;
        self.start_voice(i, key, vel, from);
    }

    fn note_off(&mut self, key: u8) {
        if self.patch.mono {
            return self.mono_off(key);
        }
        let pedal = self.pedal;
        for v in self.voices.iter_mut().filter(|v| v.gate && v.key == key) {
            if pedal {
                v.pedalled = true;
                v.gate = false;
                v.released = 0.0;
            } else {
                v.release();
            }
        }
    }

    fn control(&mut self, cc: u8, value: u8) {
        let x = value as f32 / 127.0;
        match cc {
            1 => self.wheel = x,
            7 => self.volume = x * x,
            11 => self.expression = x,
            64 => {
                self.pedal = value >= 64;
                if !self.pedal {
                    for v in self.voices.iter_mut().filter(|v| v.pedalled) {
                        v.pedalled = false;
                        v.release();
                    }
                }
            }
            // brightness: ±3 octaves on the low-pass
            74 => self.brightness = (x - 0.5) * 6.0,
            120 => self.reset(),
            123 => self.all_notes_off(),
            _ => {}
        }
    }

    fn bend(&mut self, value: i16) {
        self.bend = value as f32 / 8192.0;
    }

    fn pressure(&mut self, value: u8) {
        self.pressure = value as f32 / 127.0;
    }

    fn all_notes_off(&mut self) {
        self.held.clear();
        self.pedal = false;
        for v in self.voices.iter_mut() {
            v.pedalled = false;
            v.release();
        }
    }

    fn reset(&mut self) {
        self.held.clear();
        self.voices = [Voice::idle(); VOICES];
        self.bend = 0.0;
        self.wheel = 0.0;
        self.pressure = 0.0;
        self.pressure_smooth = 0.0;
        self.pedal = false;
        self.brightness = 0.0;
        self.volume = 1.0;
        self.expression = 1.0;
    }

    fn set_patch(&mut self, patch: &Patch) {
        let mono_changed = patch.mono != self.patch.mono;
        self.patch = patch.clone();
        self.update_gain();
        if mono_changed {
            self.all_notes_off();
        }
    }

    fn set_tuning(&mut self, cents: f32) {
        self.tuning = cents;
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
        let n = left.len();
        let mut i = 0;
        while i < n {
            if self.until_ctrl == 0 {
                self.update_controls(CTRL);
                self.until_ctrl = CTRL;
            }
            let run = self.until_ctrl.min(n - i);
            self.until_ctrl -= run;
            self.render_run(&mut left[i..i + run], &mut right[i..i + run]);
            i += run;
        }
    }
}

impl PolySynth {
    fn render_run(&mut self, left: &mut [f32], right: &mut [f32]) {
        let p = &self.patch;
        let ring = p.ring;
        let out = self.out_gain;
        let mut seed = self.seed;
        for v in self.voices.iter_mut() {
            if !v.active() {
                continue;
            }
            let pans: [(f32, f32); 2] = std::array::from_fn(|l| {
                let pan = (p.layers[l].pan + v.pan).clamp(-1.0, 1.0);
                let a = (pan + 1.0) * PI / 4.0;
                (a.cos(), a.sin())
            });
            for s in 0..left.len() {
                let mut outs = [0.0f32; 2];
                for (l, (lv, lp)) in v.layers.iter_mut().zip(p.layers.iter()).enumerate() {
                    if lv.amp.stage == Stage::Off && lv.gain <= 1e-6 {
                        continue;
                    }
                    let dt = lv.inc;
                    let t = lv.phase;
                    let mut x = 0.0;
                    if lp.saw > 0.0 {
                        x += lp.saw * (2.0 * t - 1.0 - poly_blep(t, dt));
                    }
                    if lp.pulse > 0.0 {
                        let w = lv.width;
                        let t2 = (t + 1.0 - w).fract();
                        let sq = (if t < w { 1.0 } else { -1.0 }) + poly_blep(t, dt) - poly_blep(t2, dt);
                        x += lp.pulse * (sq - (2.0 * w - 1.0));
                    }
                    if lp.sine > 0.0 {
                        let m = if lp.fm_depth > 0.0 {
                            lv.mod_phase = (lv.mod_phase + dt * lp.fm_ratio).fract();
                            lp.fm_depth * 6.0 * lv.fm_env * (lv.mod_phase * TAU).sin()
                        } else {
                            0.0
                        };
                        x += lp.sine * (lv.sine_phase * TAU + m).sin();
                        lv.sine_phase = (lv.sine_phase + dt).fract();
                    }
                    if lp.noise > 0.0 {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        x += lp.noise * (seed as f32 / u32::MAX as f32 * 2.0 - 1.0);
                    }
                    lv.phase = (t + dt).fract();
                    if lp.drive > 0.0 {
                        let d = 1.0 + lp.drive * 4.0;
                        x = (x * d).tanh() / d.tanh();
                    }
                    let (_, hi) = lv.hp.process(x);
                    let (lo, _) = lv.lp.process(hi);
                    lv.gain += lv.gain_step;
                    outs[l] = lo * lv.gain.max(0.0);
                }
                if ring > 0.0 {
                    outs[1] = outs[1] * (1.0 - ring) + outs[0] * outs[1] * 6.0 * ring;
                }
                left[s] += (outs[0] * pans[0].0 + outs[1] * pans[1].0) * out;
                right[s] += (outs[0] * pans[0].1 + outs[1] * pans[1].1) * out;
            }
        }
        self.seed = seed;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::factory;

    fn render(s: &mut PolySynth, secs: f32) -> (Vec<f32>, Vec<f32>) {
        let n = (secs * 48000.0) as usize;
        let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            s.render(a, b);
        }
        (l, r)
    }

    fn peak(v: &[f32]) -> f32 {
        v.iter().fold(0.0f32, |m, x| m.max(x.abs()))
    }

    /// Every factory patch makes sound at a sensible level, a chord doesn't clip, and
    /// it dies away after release.
    #[test]
    fn every_patch_sounds_and_stops() {
        for patch in factory() {
            let mut s = PolySynth::new(48000, patch.clone());
            for k in [48, 55, 60, 63, 67] {
                s.note_on(k, 100);
            }
            let (l, r) = render(&mut s, 2.5);
            let pk = peak(&l).max(peak(&r));
            assert!(pk > 0.02, "{} is too quiet: {pk}", patch.name);
            assert!(pk < 1.6, "{} is too loud: {pk}", patch.name);
            assert!(l.iter().chain(&r).all(|x| x.is_finite()), "{} blew up", patch.name);
            s.all_notes_off();
            render(&mut s, 12.0);
            let (l, _) = render(&mut s, 0.2);
            assert!(peak(&l) < 1e-3, "{} keeps sounding", patch.name);
        }
    }

    /// Bells, keys and plucks are in tune: one held A4 is strongly periodic at
    /// exactly 440 Hz (normalised autocorrelation at the period), which inharmonic FM
    /// ratios, detuned layers or drift would all spoil.
    #[test]
    fn tuned_percussion_is_on_pitch() {
        use crate::patch::Category;
        // the old Night Bells' inharmonic ratio, which must fail
        let mut detuned = crate::patch::by_name("Night Bells");
        detuned.name = "inharmonic".into();
        detuned.layers[0].fm_ratio = 3.5;
        let mut failed = false;
        for patch in factory().into_iter().filter(|p| matches!(p.category, Category::Bells | Category::Keys)).chain([detuned]) {
            let mut s = PolySynth::new(48000, patch.clone());
            s.note_on(69, 100);
            let (l, r) = render(&mut s, 0.4);
            let x: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a + b).collect::<Vec<_>>()[4800..14400].to_vec();
            let corr = |lag: usize| {
                let (mut num, mut e0, mut e1) = (0.0f64, 0.0f64, 0.0f64);
                for i in 0..x.len() - lag {
                    num += x[i] as f64 * x[i + lag] as f64;
                    e0 += (x[i] as f64).powi(2);
                    e1 += (x[i + lag] as f64).powi(2);
                }
                num / (e0 * e1).sqrt().max(1e-12)
            };
            // the best lag near one period, refined between samples
            let period = 48000.0 / 440.0;
            let (lo, hi) = ((period * 0.97) as usize, (period * 1.03) as usize + 1);
            let best = (lo..=hi).max_by(|a, b| corr(*a).total_cmp(&corr(*b))).unwrap();
            let (a, b, c) = (corr(best - 1), corr(best), corr(best + 1));
            let lag = best as f64 + 0.5 * (a - c) / (a - 2.0 * b + c);
            let cents = 1200.0 * (period / lag).log2();
            if patch.name == "inharmonic" {
                failed = b <= 0.9 || cents.abs() >= 4.0;
                continue;
            }
            assert!(b > 0.9, "{} isn't periodic at its pitch ({b:.3})", patch.name);
            assert!(cents.abs() < 4.0, "{} is {cents:.1} cents off", patch.name);
        }
        assert!(failed, "the check should catch an inharmonic bell");
    }

    #[test]
    fn mono_patch_glides_without_retriggering() {
        let mut s = PolySynth::new(48000, crate::patch::by_name("Smoky Lead"));
        s.note_on(60, 100);
        render(&mut s, 0.3);
        s.note_on(67, 100);
        render(&mut s, 0.01);
        assert_eq!(s.voices.iter().filter(|v| v.active()).count(), 1);
        let v = s.voices[0];
        assert!(v.pitch > 60.0 && v.pitch < 67.0, "should be gliding: {}", v.pitch);
        render(&mut s, 0.5);
        assert!((s.voices[0].pitch - 67.0).abs() < 0.05);
    }

    #[test]
    fn brass_sags_after_release() {
        let mut s = PolySynth::new(48000, crate::patch::by_name("Neon Brass"));
        s.note_on(60, 100);
        render(&mut s, 0.5);
        s.note_off(60);
        let before = s.voices[0].released;
        render(&mut s, 0.7);
        assert!(s.voices[0].released > before);
        assert!(s.voices[0].active());
    }

    #[test]
    fn sustain_pedal_holds_notes() {
        let mut s = PolySynth::new(48000, crate::patch::by_name("Polysynth Keys"));
        s.control(64, 127);
        s.note_on(60, 100);
        render(&mut s, 0.1);
        s.note_off(60);
        render(&mut s, 0.3);
        assert!(s.voices.iter().any(|v| v.pedalled && v.layers[0].amp.stage != Stage::Release));
        s.control(64, 0);
        assert!(s.voices.iter().all(|v| !v.pedalled));
    }

    #[test]
    fn steals_voices_beyond_sixteen() {
        let mut s = PolySynth::new(48000, crate::patch::by_name("Rain Pad"));
        for k in 30..60 {
            s.note_on(k, 90);
        }
        assert_eq!(s.voices.iter().filter(|v| v.active()).count(), VOICES);
        assert!(s.voices.iter().any(|v| v.key == 59));
    }
}
