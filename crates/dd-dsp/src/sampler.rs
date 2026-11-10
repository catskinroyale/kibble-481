//! Plays the city kit (`kit.rs`): the drum kit, or the effects (rain, thunder,
//! machinery hum, neon buzz, distant metal, sub booms, braams, risers, drones,
//! glass grains and timpani). Sustained effects such as rain loop for as long as
//! the key is held, swelling in and dying away. The rains are real recordings
//! (`rain.rs`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::Instrument;
use crate::kit::{self, Sound};
use crate::rain::{RainBed, RainKind, level_of};

/// The city FX key for rain on a wet street (beside the kit's own keys).
pub const WET_STREET: u8 = crate::rain::keys::WET_STREET;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KitKind {
    Drums,
    CityFx,
}

impl KitKind {
    pub fn label(self) -> &'static str {
        match self {
            KitKind::Drums => "Noir Drum Kit",
            KitKind::CityFx => "City FX",
        }
    }

    /// The name of each key that plays something, lowest first.
    pub fn key_names(self) -> Vec<(u8, u8, &'static str)> {
        let mut v: Vec<(u8, u8, &'static str)> = sounds(self, 8000)
            .iter()
            .filter(|s| self != KitKind::CityFx || s.keys.0 != kit::fx::RAIN)
            .map(|s| (s.keys.0, s.keys.1, s.name))
            .collect();
        if self == KitKind::CityFx {
            v.extend(RainKind::ALL.iter().map(|k| (k.key(), k.key(), k.label())));
            v.sort_by_key(|x| x.0);
        }
        v
    }

    /// The recorded rain a key plays, if it is one.
    pub fn rain_of(self, key: u8) -> Option<RainKind> {
        if self == KitKind::CityFx { RainKind::of_key(key) } else { None }
    }

    /// Whether a key keeps sounding for as long as it's held (rain, hum, drones).
    pub fn sustains(self, key: u8) -> bool {
        self.rain_of(key).is_some() || sounds(self, 8000).iter().any(|s| (s.keys.0..=s.keys.1).contains(&key) && s.looped.is_some())
    }
}

/// The kit's sounds at a sample rate, made once and shared.
pub fn sounds(kind: KitKind, sr: u32) -> Arc<Vec<Sound>> {
    type Cache = Mutex<HashMap<(KitKind, u32), Arc<Vec<Sound>>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(s) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&(kind, sr)) {
        return s.clone();
    }
    let made = Arc::new(match kind {
        KitKind::Drums => kit::drums(sr as f64),
        KitKind::CityFx => kit::effects(sr as f64),
    });
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert((kind, sr), made.clone());
    made
}

struct Voice {
    sound: usize,
    key: u8,
    pos: f64,
    rate: f64,
    gain: f32,
    env: f32,
    /// Per-sample change of `env`: up while swelling in, a factor while dying away.
    rise: f32,
    fall: f32,
    releasing: bool,
    done: bool,
}

pub struct KitPlayer {
    kind: KitKind,
    sounds: Arc<Vec<Sound>>,
    sr: f32,
    voices: Vec<Voice>,
    clouds: Vec<RainBed>,
    /// Notes started, to seed each cloud differently but repeatably.
    started: u64,
    /// The modulation wheel, once moved: how hard it rains.
    wheel: Option<f32>,
    bright: f32,
    volume: f32,
    expression: f32,
}

const MAX_VOICES: usize = 48;

impl KitPlayer {
    pub fn new(kind: KitKind, sample_rate: u32) -> Self {
        if kind == KitKind::CityFx {
            crate::rain::warm_up();
        }
        KitPlayer {
            kind,
            sounds: sounds(kind, sample_rate),
            sr: sample_rate as f32,
            voices: Vec::new(),
            clouds: Vec::new(),
            started: 0,
            wheel: None,
            bright: 14_000.0,
            volume: 1.0,
            expression: 1.0,
        }
    }
}

impl Instrument for KitPlayer {
    fn note_on(&mut self, key: u8, vel: u8) {
        if vel == 0 {
            return self.note_off(key);
        }
        self.started += 1;
        if let Some(kind) = self.kind.rain_of(key) {
            // a new rain crossfades over one already falling on this key
            self.note_off(key);
            let mut c = RainBed::new(kind, vel, self.sr as u32, key as u64 * 7919 + self.started);
            if let Some(w) = self.wheel {
                c.level = level_of(w);
            }
            c.bright_target = self.bright;
            self.clouds.push(c);
            return;
        }
        let Some(i) = self.sounds.iter().position(|s| (s.keys.0..=s.keys.1).contains(&key)) else { return };
        let s = &self.sounds[i];
        let rate = if s.pitched() { 2f64.powf((key as f64 - s.root as f64) / 12.0) } else { 1.0 };
        let looped = s.looped.is_some();
        // a sustained sound swells in; a one-shot starts at once
        let rise = if looped { 1.0 / (s.fades.0 as f32 * self.sr).max(1.0) } else { 1.0 };
        let fall = (-4.6 / (s.fades.1 as f32 * self.sr).max(1.0)).exp();
        if self.voices.len() >= MAX_VOICES {
            self.voices.remove(0);
        }
        let v = vel as f32 / 127.0;
        self.voices.push(Voice {
            sound: i,
            key,
            pos: 0.0,
            rate,
            gain: v * v.sqrt(),
            env: if looped { 0.0 } else { 1.0 },
            rise,
            fall,
            releasing: false,
            done: false,
        });
    }

    fn note_off(&mut self, key: u8) {
        for c in self.clouds.iter_mut().filter(|c| c.key == key) {
            c.release();
        }
        for v in self.voices.iter_mut().filter(|v| v.key == key) {
            // only sustained sounds stop when let go; hits ring out
            if self.sounds[v.sound].looped.is_some() {
                v.releasing = true;
            }
        }
    }

    fn control(&mut self, cc: u8, value: u8) {
        let x = value as f32 / 127.0;
        match cc {
            7 => self.volume = x * x,
            // how hard it rains
            1 => {
                self.wheel = Some(x);
                for c in self.clouds.iter_mut() {
                    c.level = level_of(x);
                }
            }
            // brightness: below the middle the rain darkens, as if behind glass or far off
            74 => {
                self.bright = if x < 0.5 { (14_000.0 * (x / 0.5).powi(2)).max(350.0) } else { 14_000.0 + 4_000.0 * (x - 0.5) * 2.0 };
                for c in self.clouds.iter_mut() {
                    c.bright_target = self.bright;
                }
            }
            11 => self.expression = x,
            120 => self.reset(),
            123 => self.all_notes_off(),
            _ => {}
        }
    }

    fn all_notes_off(&mut self) {
        for c in self.clouds.iter_mut() {
            c.release();
        }
        for v in self.voices.iter_mut() {
            v.releasing = true;
            if self.sounds[v.sound].looped.is_none() {
                // let hits finish, but quickly
                v.fall = (-4.6 / (0.3 * self.sr)).exp();
            }
        }
    }

    fn reset(&mut self) {
        self.voices.clear();
        self.clouds.clear();
        self.wheel = None;
        self.bright = 14_000.0;
    }

    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
        let g = self.volume * self.expression * 0.9;
        for v in self.voices.iter_mut() {
            let s = &self.sounds[v.sound];
            let data = &s.data;
            let len = data.len();
            for i in 0..left.len() {
                if v.releasing {
                    v.env *= v.fall;
                } else if v.env < 1.0 {
                    v.env = (v.env + v.rise).min(1.0);
                }
                let idx = v.pos as usize;
                if idx + 1 >= len {
                    v.done = true;
                    break;
                }
                let f = (v.pos - idx as f64) as f32;
                let x = data[idx] + (data[idx + 1] - data[idx]) * f;
                let y = x * v.gain * v.env * g;
                left[i] += y;
                right[i] += y;
                v.pos += v.rate;
                if let Some((a, b)) = s.looped
                    && v.pos >= b as f64
                {
                    v.pos -= (b - a) as f64;
                }
            }
        }
        self.voices.retain(|v| !v.done && !(v.releasing && v.env < 1e-4));
        for c in self.clouds.iter_mut() {
            c.render(left, right, g);
        }
        self.clouds.retain(|c| !c.finished());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_loops_until_released() {
        let mut k = KitPlayer::new(KitKind::CityFx, 22050);
        k.note_on(kit::fx::RAIN, 100);
        let (mut l, mut r) = (vec![0.0; 22050], vec![0.0; 22050]);
        for _ in 0..30 {
            k.render(&mut l, &mut r);
        }
        assert_eq!(k.clouds.len(), 1, "rain should still be falling after 30 s");
        assert!(l.iter().any(|x| x.abs() > 0.01));
        k.note_off(kit::fx::RAIN);
        for _ in 0..16 {
            k.render(&mut l, &mut r);
        }
        assert!(k.clouds.is_empty());
    }

    #[test]
    fn drum_hits_ring_out_and_end() {
        let mut k = KitPlayer::new(KitKind::Drums, 22050);
        k.note_on(kit::drum::KICK, 120);
        k.note_off(kit::drum::KICK);
        let (mut l, mut r) = (vec![0.0; 2205], vec![0.0; 2205]);
        k.render(&mut l, &mut r);
        assert!(l.iter().any(|x| x.abs() > 0.1));
        for _ in 0..60 {
            k.render(&mut l, &mut r);
        }
        assert!(k.voices.is_empty());
    }
}
