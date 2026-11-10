//! Rain from real recordings. Each kind of rain is a field recording played as it was
//! recorded (no re-pitching), kept going for as long as the key is held by moving
//! between random stretches of it with slow equal-power crossfades, so it never
//! repeats audibly. A mono recording plays a different stretch on each side, which
//! makes a natural stereo field (rain is incoherent). Velocity and the modulation
//! wheel set how loud it is; brightness (CC74) closes a low-pass, as if heard through
//! glass or far off.
//!
//! The recordings are CC0; who made them and where they come from is in
//! `assets/samples/README.md`.

use std::f32::consts::{FRAC_PI_2, TAU};
use std::sync::{Arc, OnceLock};

/// The City FX keys of the recorded rains (beside the kit's own keys).
pub mod keys {
    pub const CITY: u8 = crate::kit::fx::RAIN;
    pub const WET_STREET: u8 = 108;
    pub const SILL: u8 = 109;
    pub const WINDOW: u8 = 110;
    pub const DRIPS: u8 = 111;
    pub const UMBRELLA: u8 = 112;
    pub const ROOF: u8 = 113;
    pub const LIGHT: u8 = 114;
    pub const BACKSTREET: u8 = 115;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RainKind {
    /// Sparse: drops you can count.
    Sill,
    Drips,
    Umbrella,
    Roof,
    Backstreet,
    Light,
    /// Dense: a wash of rain.
    Window,
    WetStreet,
    City,
}

impl RainKind {
    pub const ALL: [RainKind; 9] = [
        RainKind::Sill,
        RainKind::Drips,
        RainKind::Umbrella,
        RainKind::Roof,
        RainKind::Backstreet,
        RainKind::Light,
        RainKind::Window,
        RainKind::WetStreet,
        RainKind::City,
    ];

    /// Pitter-patter, rather than a wash.
    pub fn sparse(self) -> bool {
        matches!(self, RainKind::Sill | RainKind::Drips | RainKind::Umbrella | RainKind::Roof | RainKind::Backstreet)
    }

    /// The dense recordings were normalised 6 dB louder than the sparse ones; bring
    /// them down to sit as quietly.
    pub fn trim(self) -> f32 {
        if self.sparse() { 1.0 } else { 0.5 }
    }

    pub fn key(self) -> u8 {
        match self {
            RainKind::City => keys::CITY,
            RainKind::WetStreet => keys::WET_STREET,
            RainKind::Sill => keys::SILL,
            RainKind::Window => keys::WINDOW,
            RainKind::Drips => keys::DRIPS,
            RainKind::Umbrella => keys::UMBRELLA,
            RainKind::Roof => keys::ROOF,
            RainKind::Light => keys::LIGHT,
            RainKind::Backstreet => keys::BACKSTREET,
        }
    }

    pub fn of_key(key: u8) -> Option<RainKind> {
        RainKind::ALL.into_iter().find(|k| k.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            RainKind::Sill => "Rain: pitter-pat on a sill",
            RainKind::Drips => "Rain: drips",
            RainKind::Umbrella => "Rain: drops on an umbrella",
            RainKind::Roof => "Rain: soft, on a roof",
            RainKind::Backstreet => "Rain: drops in a back street",
            RainKind::Light => "Rain: light, steady",
            RainKind::Window => "Rain: heavy, on a window",
            RainKind::WetStreet => "Rain: wet street, traffic",
            RainKind::City => "Rain: heavy, city",
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            RainKind::City => include_bytes!("../../../assets/samples/city-rain.ogg"),
            RainKind::WetStreet => include_bytes!("../../../assets/samples/wet-street.ogg"),
            RainKind::Sill => include_bytes!("../../../assets/samples/sill-rain.ogg"),
            RainKind::Window => include_bytes!("../../../assets/samples/window-rain.ogg"),
            RainKind::Drips => include_bytes!("../../../assets/samples/drips.ogg"),
            RainKind::Umbrella => include_bytes!("../../../assets/samples/umbrella-drops.ogg"),
            RainKind::Roof => include_bytes!("../../../assets/samples/roof-rain.ogg"),
            RainKind::Light => include_bytes!("../../../assets/samples/light-rain.ogg"),
            RainKind::Backstreet => include_bytes!("../../../assets/samples/backstreet-rain.ogg"),
        }
    }
}

/// A decoded recording: 16-bit frames, one or two channels.
pub struct Recording {
    pub channels: usize,
    pub sr: u32,
    pub data: Vec<i16>,
}

impl Recording {
    pub fn frames(&self) -> usize {
        self.data.len() / self.channels.max(1)
    }

    pub fn secs(&self) -> f64 {
        self.frames() as f64 / self.sr as f64
    }

    #[inline]
    fn at(&self, frame: usize, ch: usize) -> f32 {
        self.data[frame * self.channels + ch.min(self.channels - 1)] as f32 * (1.0 / 32768.0)
    }

    fn decode(bytes: &[u8]) -> Recording {
        let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).expect("a valid embedded Ogg Vorbis file");
        let channels = r.ident_hdr.audio_channels.max(1) as usize;
        let sr = r.ident_hdr.audio_sample_rate;
        let mut data = Vec::new();
        while let Ok(Some(p)) = r.read_dec_packet_itl() {
            data.extend_from_slice(&p);
        }
        if channels == 2 {
            // a stereo file whose sides barely differ is mono in all but name: fold it, so
            // it gets a different stretch on each side like any mono recording
            let (mut mid, mut side) = (0.0f64, 0.0f64);
            for f in data.as_chunks::<2>().0 {
                let (l, r) = (f[0] as f64, f[1] as f64);
                mid += ((l + r) * 0.5).powi(2);
                side += ((l - r) * 0.5).powi(2);
            }
            if side < mid * 0.01 {
                let mono = data.as_chunks::<2>().0.iter().map(|f| ((f[0] as i32 + f[1] as i32) / 2) as i16).collect();
                return Recording { channels: 1, sr, data: mono };
            }
        }
        Recording { channels, sr, data }
    }
}

/// A recording, decoded once and shared.
pub fn recording(kind: RainKind) -> Arc<Recording> {
    static CACHE: [OnceLock<Arc<Recording>>; 9] = [const { OnceLock::new() }; 9];
    let i = RainKind::ALL.iter().position(|k| *k == kind).unwrap_or(0);
    CACHE[i].get_or_init(|| Arc::new(Recording::decode(kind.bytes()))).clone()
}

/// Decode every recording in the background, so the first note doesn't wait.
pub fn warm_up() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        std::thread::spawn(|| {
            for k in RainKind::ALL {
                recording(k);
            }
        });
    });
}

/// Seconds of each stretch, shortest and longest, and of the crossfades between them.
const STRETCH: (f64, f64) = (6.0, 14.0);
const XFADE: f64 = 1.5;

/// One reader moving from stretch to stretch of a recording.
struct Stream {
    pos: f64,
    /// The stretch faded in, while a crossfade runs.
    next: Option<f64>,
    /// Crossfade progress, 0..1.
    fade: f32,
    /// Output samples until the next crossfade starts.
    until: f64,
}

pub struct RainBed {
    pub key: u8,
    src: Arc<Recording>,
    sr: f64,
    step: f64,
    streams: Vec<Stream>,
    seed: u64,
    /// The level the velocity or wheel asks for, and the one gliding to it.
    pub level: f32,
    gain: f32,
    pub bright_target: f32,
    bright: f32,
    lp: [f32; 2],
    env: f32,
    rise: f32,
    fall: f32,
    releasing: bool,
    trim: f32,
}

/// Velocity or wheel to level: from 18 dB down to full.
pub fn level_of(x: f32) -> f32 {
    10f32.powf(-18.0 * (1.0 - x.clamp(0.0, 1.0)) / 20.0)
}

impl RainBed {
    pub fn new(kind: RainKind, vel: u8, sample_rate: u32, seed: u64) -> RainBed {
        let src = recording(kind);
        let sr = sample_rate as f64;
        let step = src.sr as f64 / sr;
        let level = level_of(vel as f32 / 127.0);
        let mut bed = RainBed {
            key: kind.key(),
            sr,
            step,
            streams: Vec::new(),
            seed: seed ^ 0x2545_f491_4f6c_dd1d,
            level,
            gain: level,
            bright_target: 14_000.0,
            bright: 14_000.0,
            lp: [0.0; 2],
            env: 0.0,
            rise: 1.0 / (1.2 * sample_rate as f32),
            fall: (-4.6 / (3.0 * sample_rate as f32)).exp(),
            releasing: false,
            trim: kind.trim(),
            src,
        };
        // a mono recording reads a different stretch for each side
        let n = if bed.src.channels == 1 { 2 } else { 1 };
        for _ in 0..n {
            let pos = bed.start();
            let until = bed.stretch();
            bed.streams.push(Stream { pos, next: None, fade: 0.0, until });
        }
        bed
    }

    fn rand(&mut self) -> f64 {
        self.seed = self.seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// A random place to start a stretch, leaving room for it and its crossfade.
    fn start(&mut self) -> f64 {
        let need = (STRETCH.1 + XFADE * 2.0) * self.src.sr as f64;
        let room = (self.src.frames() as f64 - need).max(1.0);
        self.rand() * room
    }

    /// Output samples a stretch lasts before the next fades in.
    fn stretch(&mut self) -> f64 {
        (STRETCH.0 + (STRETCH.1 - STRETCH.0) * self.rand()) * self.sr
    }

    pub fn release(&mut self) {
        self.releasing = true;
    }

    pub fn finished(&self) -> bool {
        self.releasing && self.env < 1e-4
    }

    /// Add the next block into `left` and `right`.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32], gain: f32) {
        let fade_step = (1.0 / (XFADE * self.sr)) as f32;
        let frames = self.src.frames();
        let stereo_src = self.src.channels > 1;
        for i in 0..left.len() {
            // move on to a new stretch where one ends
            for s in 0..self.streams.len() {
                self.streams[s].until -= 1.0;
                if self.streams[s].until <= 0.0 && self.streams[s].next.is_none() {
                    let p = self.start();
                    self.streams[s].next = Some(p);
                    self.streams[s].fade = 0.0;
                }
            }
            let mut out = [0.0f32; 2];
            for (s, st) in self.streams.iter_mut().enumerate() {
                let read = |pos: f64, ch: usize| {
                    let f = pos as usize;
                    if f + 1 >= frames {
                        return 0.0;
                    }
                    let t = (pos - f as f64) as f32;
                    let a = self.src.at(f, ch);
                    a + (self.src.at(f + 1, ch) - a) * t
                };
                let (ga, gb) = ((st.fade * FRAC_PI_2).cos(), (st.fade * FRAC_PI_2).sin());
                let chans: &[usize] = if stereo_src { &[0, 1] } else { &[0] };
                for &ch in chans {
                    let mut x = read(st.pos, ch) * ga;
                    if let Some(n) = st.next {
                        x += read(n, ch) * gb;
                    }
                    let side = if stereo_src { ch } else { s };
                    out[side] += x;
                }
                st.pos += self.step;
                if let Some(n) = st.next.as_mut() {
                    *n += self.step;
                    st.fade += fade_step;
                    if st.fade >= 1.0 {
                        st.pos = *n;
                        st.next = None;
                        st.fade = 0.0;
                        st.until = 0.0; // set below
                    }
                }
            }
            for s in 0..self.streams.len() {
                if self.streams[s].until == 0.0 && self.streams[s].next.is_none() {
                    self.streams[s].until = self.stretch();
                }
            }
            if self.releasing {
                self.env *= self.fall;
            } else if self.env < 1.0 {
                self.env = (self.env + self.rise).min(1.0);
            }
            self.gain += (self.level - self.gain) * 0.0002;
            self.bright += (self.bright_target - self.bright) * 0.0005;
            let k = 1.0 - (-TAU * self.bright / self.sr as f32).exp();
            self.lp[0] += (out[0] - self.lp[0]) * k;
            self.lp[1] += (out[1] - self.lp[1]) * k;
            let g = self.env * self.gain * gain * 1.4 * self.trim;
            left[i] += self.lp[0] * g;
            right[i] += self.lp[1] * g;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(v: &[f32]) -> f32 {
        (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt()
    }

    #[test]
    fn every_recording_decodes() {
        for k in RainKind::ALL {
            let r = recording(k);
            assert_eq!(r.sr, 48000, "{k:?}");
            assert!(r.secs() > 25.0, "{k:?}: {} s", r.secs());
            let x: Vec<f32> = (0..r.frames()).step_by(7).map(|f| r.at(f, 0)).collect();
            assert!(rms(&x) > 0.01, "{k:?} is too quiet");
        }
    }

    /// Each rain runs on far past its recording at a steady level, both sides alive
    /// and different (stereo), and dies away when released.
    #[test]
    fn rain_is_endless_steady_and_wide() {
        let sr = 24000;
        for k in RainKind::ALL {
            let mut bed = RainBed::new(k, 110, sr, 3);
            let mut levels = Vec::new();
            for sec in 0..90 {
                let (mut l, mut r) = (vec![0.0; sr as usize], vec![0.0; sr as usize]);
                bed.render(&mut l, &mut r, 1.0);
                assert!(l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 1.0), "{k:?}");
                if sec >= 3 {
                    levels.push(rms(&l) + rms(&r));
                    let diff: Vec<f32> = l.iter().zip(&r).map(|(a, b)| a - b).collect();
                    assert!(rms(&diff) > 0.1 * rms(&l), "{k:?} is mono");
                }
            }
            let (lo, hi) = levels.iter().fold((f32::MAX, 0.0f32), |(a, b), x| (a.min(*x), b.max(*x)));
            assert!(lo > 0.005, "{k:?} goes quiet: {lo}");
            let limit = if k.sparse() { 5.0 } else { 2.5 };
            assert!(hi / lo < limit, "{k:?} is uneven: {lo} {hi}");
            bed.release();
            for _ in 0..8 {
                let (mut l, mut r) = (vec![0.0; sr as usize], vec![0.0; sr as usize]);
                bed.render(&mut l, &mut r, 1.0);
            }
            assert!(bed.finished(), "{k:?}");
        }
    }

    #[test]
    fn keys_round_trip() {
        for k in RainKind::ALL {
            assert_eq!(RainKind::of_key(k.key()), Some(k));
        }
    }
}
