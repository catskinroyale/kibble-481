//! The engine: every track's instrument, the transport, and the mixer. The same code
//! runs in the audio callback and offline for export, so what you hear is what you
//! export.
//!
//! The mixer follows Ambient MIDI Generator's (`mixer.rs`, MIT): each track plays dry
//! through its channel strip (fader, pan, width, mute and solo), feeds the shared
//! reverb and echo from its sends, and sums on the master bus, the tracks that ask for
//! it through the ensemble chorus.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use dd_dsp::Instrument;
use dd_dsp::effects::{CutFilter, EchoFx, Ensemble, soft_clip};
use dd_dsp::fx::Fx;
use dd_dsp::patch::Patch;
use dd_dsp::reverb::Reverb;

use crate::project::{Project, Strip, db_gain};
use crate::timeline::{Msg, Timeline};

pub const MAX_TRACKS: usize = 64;
/// The longest run between control updates, in samples.
const BLOCK: usize = 64;

fn epoch() -> Instant {
    static E: OnceLock<Instant> = OnceLock::new();
    *E.get_or_init(Instant::now)
}

/// What the audio thread tells everyone else: where it is, and the levels.
pub struct Shared {
    pos_bits: AtomicU64,
    /// When `pos_bits` was written, in nanoseconds since `epoch()`.
    at_nanos: AtomicU64,
    playing: AtomicBool,
    /// Seconds of audio rendered ahead of the speakers.
    latency_bits: AtomicU64,
    peaks: Vec<AtomicU32>,
}

impl Shared {
    pub fn new() -> Arc<Shared> {
        epoch();
        Arc::new(Shared {
            pos_bits: AtomicU64::new(0f64.to_bits()),
            at_nanos: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            latency_bits: AtomicU64::new(0f64.to_bits()),
            peaks: (0..MAX_TRACKS + 2).map(|_| AtomicU32::new(0)).collect(),
        })
    }

    /// The song position at the end of the last block rendered.
    pub fn position(&self) -> f64 {
        f64::from_bits(self.pos_bits.load(Ordering::Relaxed))
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_latency(&self, secs: f64) {
        self.latency_bits.store(secs.to_bits(), Ordering::Relaxed);
    }

    /// The song position that is reaching the speakers now: the last block's, moved
    /// on by the time since, less the audio buffered ahead. Recording stamps notes
    /// with this, so they land where they were heard.
    pub fn heard_now(&self) -> f64 {
        let pos = self.position();
        if !self.is_playing() {
            return pos;
        }
        let at = self.at_nanos.load(Ordering::Relaxed);
        let now = epoch().elapsed().as_nanos() as u64;
        let since = (now.saturating_sub(at) as f64 * 1e-9).min(0.25);
        pos + since - f64::from_bits(self.latency_bits.load(Ordering::Relaxed))
    }

    /// The peak level of a track since the last call (and starts the next).
    pub fn take_peak(&self, track: usize) -> f32 {
        self.peaks.get(track).map_or(0.0, |p| f32::from_bits(p.swap(0, Ordering::Relaxed)))
    }

    pub fn take_master(&self) -> (f32, f32) {
        (self.take_peak(MAX_TRACKS), self.take_peak(MAX_TRACKS + 1))
    }

    fn push_peak(&self, i: usize, v: f32) {
        if let Some(p) = self.peaks.get(i) {
            p.fetch_max(v.to_bits(), Ordering::Relaxed);
        }
    }

    fn publish(&self, pos: f64, playing: bool) {
        self.pos_bits.store(pos.to_bits(), Ordering::Relaxed);
        self.at_nanos.store(epoch().elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.playing.store(playing, Ordering::Relaxed);
    }
}

/// The mixer's settings, taken from the project.
#[derive(Clone, Debug, PartialEq)]
pub struct MixSettings {
    pub strips: Vec<Strip>,
    pub audible: Vec<bool>,
    pub master_db: f32,
    pub fx: Fx,
    pub bpm: f64,
    /// Seconds per beat and beats per bar, for the metronome.
    pub beat_secs: f64,
    pub beats_per_bar: u32,
}

impl MixSettings {
    pub fn of(p: &Project) -> MixSettings {
        MixSettings {
            strips: p.tracks.iter().map(|t| t.strip).collect(),
            audible: (0..p.tracks.len()).map(|i| p.audible(i)).collect(),
            master_db: p.master_db,
            fx: p.fx,
            bpm: p.bpm,
            beat_secs: p.tick_secs(p.beat_ticks()),
            beats_per_bar: p.meter.0.max(1),
        }
    }
}

/// A short preview: (start, length) in seconds, key, velocity of each note.
pub type Preview = Vec<(f64, f64, u8, u8)>;

/// Commands to the engine.
pub enum Cmd {
    Timeline(Arc<Timeline>),
    /// Replace every track's instrument (after tracks are added, removed or moved).
    Instruments(Vec<Box<dyn Instrument>>),
    /// Replace one track's instrument (its sound changed).
    Instrument(usize, Box<dyn Instrument>),
    /// Change a synth track's patch while it plays.
    Patch(usize, Box<Patch>),
    Mix(Box<MixSettings>),
    /// Play from a position; before zero, the metronome counts in.
    Play(f64),
    Pause,
    Seek(f64),
    Loop(Option<(f64, f64)>),
    /// A message played live (keyboard, MIDI input) to a track.
    Live(usize, Msg),
    Metronome(bool),
    /// Keep playing past the end (while recording).
    Endless(bool),
    /// A short preview on a track, playing or not: (start, length) in seconds, key, velocity.
    Audition(usize, Preview),
    /// Silence everything at once.
    Panic,
    Volume(f32),
}

/// A strip's settings as gains, gliding to new values over a block so faders don't click.
#[derive(Clone, Copy, Default)]
struct Gains {
    ll: f32,
    lr: f32,
    rl: f32,
    rr: f32,
    reverb: f32,
    echo: f32,
}

impl Gains {
    fn of(s: &Strip, audible: bool) -> Gains {
        let g = if audible { s.gain() } else { 0.0 };
        let w = s.width.clamp(0.0, 2.0);
        let same = 0.5 + 0.5 * w;
        let cross = 0.5 - 0.5 * w;
        let p = s.pan.clamp(-1.0, 1.0);
        let (keep_l, move_l) = if p > 0.0 { (1.0 - p, p * 0.6) } else { (1.0, 0.0) };
        let (keep_r, move_r) = if p < 0.0 { (1.0 + p, -p * 0.6) } else { (1.0, 0.0) };
        Gains {
            ll: g * (keep_l * same + move_r * cross),
            rl: g * (keep_l * cross + move_r * same),
            lr: g * (keep_r * cross + move_l * same),
            rr: g * (keep_r * same + move_l * cross),
            reverb: g * s.reverb.clamp(0.0, 1.0) * 1.2,
            echo: g * s.echo.clamp(0.0, 1.0),
        }
    }

    fn scaled(self, k: f32) -> Gains {
        Gains { ll: self.ll * k, lr: self.lr * k, rl: self.rl * k, rr: self.rr * k, reverb: self.reverb * k, echo: self.echo * k }
    }

    fn lerp(a: Gains, b: Gains, t: f32) -> Gains {
        let m = |x: f32, y: f32| x + (y - x) * t;
        Gains {
            ll: m(a.ll, b.ll),
            lr: m(a.lr, b.lr),
            rl: m(a.rl, b.rl),
            rr: m(a.rr, b.rr),
            reverb: m(a.reverb, b.reverb),
            echo: m(a.echo, b.echo),
        }
    }
}

struct Slot {
    inst: Box<dyn Instrument>,
    gains: Gains,
    /// The strip's low and high cut.
    cut: CutFilter,
    l: Vec<f32>,
    r: Vec<f32>,
}

/// The metronome's click: a short decaying sine.
#[derive(Default)]
struct Click {
    phase: f32,
    inc: f32,
    env: f32,
}

pub struct Engine {
    sr: f32,
    slots: Vec<Slot>,
    timeline: Arc<Timeline>,
    idx: usize,
    pos: f64,
    playing: bool,
    looping: Option<(f64, f64)>,
    metronome: bool,
    endless: bool,
    /// Live messages waiting their turn: samples to wait, track, message.
    pending: Vec<(u64, usize, Msg)>,
    /// Peak level of each track over the last block, for tests and meters.
    last_peaks: Vec<f32>,
    click: Click,
    mix: MixSettings,
    master: f32,
    volume: f32,
    reverb: Reverb,
    echo: EchoFx,
    ensemble: Ensemble,
    duck_env: f32,
    buf: [Vec<f32>; 6],
    shared: Arc<Shared>,
}

impl Engine {
    pub fn new(sample_rate: u32, shared: Arc<Shared>) -> Engine {
        let sr = sample_rate as f32;
        let fx = Fx::default();
        Engine {
            sr,
            slots: Vec::new(),
            timeline: Arc::new(Timeline::default()),
            idx: 0,
            pos: 0.0,
            playing: false,
            looping: None,
            metronome: false,
            endless: false,
            pending: Vec::new(),
            last_peaks: Vec::new(),
            click: Click::default(),
            mix: MixSettings {
                strips: Vec::new(),
                audible: Vec::new(),
                master_db: 0.0,
                fx,
                bpm: 72.0,
                beat_secs: 60.0 / 72.0,
                beats_per_bar: 4,
            },
            master: 1.0,
            volume: 1.0,
            reverb: Reverb::new(sr, &fx),
            echo: EchoFx::new(sr),
            ensemble: Ensemble::new(sr),
            duck_env: 0.0,
            buf: Default::default(),
            shared,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr as u32
    }

    pub fn position(&self) -> f64 {
        self.pos
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Timeline(tl) => {
                self.timeline = tl;
                if self.playing {
                    // pick up the edited music where it is
                    self.release_all();
                    self.idx = self.timeline.index_after(self.pos);
                    self.chase();
                } else {
                    self.idx = self.timeline.index_after(self.pos);
                }
            }
            Cmd::Instruments(list) => {
                self.slots = list
                    .into_iter()
                    .map(|inst| Slot { inst, gains: Gains::default(), cut: CutFilter::new(self.sr), l: Vec::new(), r: Vec::new() })
                    .collect();
                self.apply_tuning();
                self.snap_gains();
            }
            Cmd::Instrument(i, inst) => {
                if let Some(s) = self.slots.get_mut(i) {
                    s.inst = inst;
                }
                self.apply_tuning();
                self.rechase(i);
            }
            Cmd::Patch(i, p) => {
                if let Some(s) = self.slots.get_mut(i) {
                    s.inst.set_patch(&p);
                    // the held notes start again in the new sound, from its attack
                    if self.playing {
                        s.inst.all_notes_off();
                    }
                }
                self.rechase(i);
            }
            Cmd::Audition(track, notes) => {
                let sr = self.sr as f64;
                for (at, len, key, vel) in notes {
                    self.pending.push(((at * sr) as u64, track, Msg::On(key, vel)));
                    self.pending.push((((at + len) * sr) as u64, track, Msg::Off(key)));
                }
            }
            Cmd::Mix(m) => self.set_mix(*m),
            Cmd::Play(from) => {
                self.seek(from);
                self.playing = true;
                if from >= 0.0 {
                    self.chase();
                }
            }
            Cmd::Pause => {
                self.playing = false;
                self.release_all();
            }
            Cmd::Seek(t) => {
                self.seek(t);
                if self.playing {
                    self.chase();
                }
            }
            Cmd::Loop(l) => self.looping = l.filter(|(a, b)| b > a),
            Cmd::Live(track, msg) => self.dispatch(track, msg),
            Cmd::Metronome(on) => self.metronome = on,
            Cmd::Endless(on) => self.endless = on,
            Cmd::Panic => {
                for s in self.slots.iter_mut() {
                    s.inst.reset();
                }
                self.reverb.clear();
                self.echo.clear();
                self.ensemble.clear();
            }
            Cmd::Volume(v) => self.volume = v,
        }
    }

    fn set_mix(&mut self, m: MixSettings) {
        if m.fx != self.mix.fx {
            self.reverb.set(&m.fx);
            self.echo.set(&m.fx.echo, m.bpm);
            self.ensemble.target = m.fx.ensemble.clamp(0.0, 1.0);
        } else if m.bpm != self.mix.bpm {
            self.echo.set_tempo(m.bpm);
        }
        let retune = m.fx.tuning != self.mix.fx.tuning;
        self.mix = m;
        if retune {
            self.apply_tuning();
        }
    }

    /// Jump straight to the mixer's settings (no glide), as after loading.
    fn snap_gains(&mut self) {
        for (i, s) in self.slots.iter_mut().enumerate() {
            s.gains = target_gains(&self.mix, i);
        }
        self.master = db_gain(self.mix.master_db);
    }

    fn apply_tuning(&mut self) {
        let cents = self.mix.fx.tuning.cents();
        for s in self.slots.iter_mut() {
            s.inst.set_tuning(cents);
        }
    }

    fn release_all(&mut self) {
        for s in self.slots.iter_mut() {
            s.inst.all_notes_off();
            s.inst.control(64, 0);
        }
    }

    fn seek(&mut self, t: f64) {
        self.release_all();
        self.pos = t;
        self.idx = self.timeline.index_after(t - 1e-9);
    }

    /// A track's sound changed while playing: send it the notes and controllers that are
    /// sounding, so it takes over at once.
    fn rechase(&mut self, track: usize) {
        if !self.playing || self.pos < 0.0 {
            return;
        }
        for (t, msg) in self.timeline.chase(self.pos) {
            if t as usize == track {
                self.dispatch(track, msg);
            }
        }
    }

    /// The peak level of a track's strip over the last block rendered.
    pub fn shared_peak(&self, track: usize) -> f32 {
        self.last_peaks.get(track).copied().unwrap_or(0.0)
    }

    fn chase(&mut self) {
        for (track, msg) in self.timeline.chase(self.pos) {
            self.dispatch(track as usize, msg);
        }
    }

    fn dispatch(&mut self, track: usize, msg: Msg) {
        let Some(s) = self.slots.get_mut(track) else { return };
        match msg {
            Msg::On(k, v) => s.inst.note_on(k, v),
            Msg::Off(k) => s.inst.note_off(k),
            Msg::Cc(c, v) => s.inst.control(c, v),
            Msg::Bend(b) => s.inst.bend(b),
            Msg::Pressure(p) => s.inst.pressure(p),
        }
    }

    /// Render the next block: events fall on their exact sample.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len().min(right.len());
        let mut i = 0;
        while i < n {
            let mut run = (n - i).min(BLOCK);
            if self.playing {
                if let Some((a, b)) = self.looping
                    && self.pos >= b - 0.5 / self.sr as f64
                {
                    self.seek(a);
                    self.chase();
                }
                let half = 0.5 / self.sr as f64;
                while let Some(e) = self.timeline.events.get(self.idx) {
                    if e.t > self.pos + half {
                        break;
                    }
                    let (track, msg) = (e.track as usize, e.msg);
                    self.idx += 1;
                    self.dispatch(track, msg);
                }
                if self.looping.is_none() && !self.endless && self.pos >= self.timeline.duration {
                    self.playing = false;
                    self.release_all();
                }
            }
            if self.playing {
                let (pos, srf) = (self.pos, self.sr as f64);
                let mut until = |t: f64| {
                    let s = ((t - pos) * srf).ceil();
                    if s >= 1.0 {
                        run = run.min(s as usize);
                    }
                };
                if let Some(e) = self.timeline.events.get(self.idx) {
                    until(e.t);
                }
                if let Some((_, b)) = self.looping {
                    until(b);
                }
                if self.metronome || self.pos < 0.0 {
                    let beat = self.mix.beat_secs.max(0.05);
                    let k = (self.pos / beat).round();
                    if (self.pos - k * beat).abs() * self.sr as f64 <= 0.5 {
                        let down = (k as i64).rem_euclid(self.mix.beats_per_bar as i64) == 0;
                        self.click =
                            Click { phase: 0.0, inc: if down { 1760.0 } else { 1320.0 } / self.sr, env: if down { 0.5 } else { 0.32 } };
                    }
                    until(((self.pos / beat).floor() + 1.0) * beat);
                }
            }
            // previews and other timed live messages, playing or not
            if !self.pending.is_empty() {
                let mut due = Vec::new();
                for p in self.pending.iter_mut() {
                    if p.0 == 0 {
                        due.push((p.1, p.2));
                    } else {
                        run = run.min(p.0 as usize);
                    }
                }
                self.pending.retain(|p| p.0 > 0);
                for (t, m) in due {
                    self.dispatch(t, m);
                }
                for p in self.pending.iter_mut() {
                    p.0 -= run as u64;
                }
            }
            let (l, r) = (&mut left[i..i + run], &mut right[i..i + run]);
            self.mix_run(l, r);
            if self.click.env > 1e-3 {
                let fall = (-1.0 / (0.025 * self.sr)).exp();
                for (a, b) in l.iter_mut().zip(r.iter_mut()) {
                    let x = (self.click.phase * std::f32::consts::TAU).sin() * self.click.env;
                    self.click.phase = (self.click.phase + self.click.inc).fract();
                    self.click.env *= fall;
                    *a += x;
                    *b += x;
                }
            }
            if self.playing {
                self.pos += run as f64 / self.sr as f64;
            }
            i += run;
        }
        self.shared.publish(self.pos, self.playing);
    }

    fn mix_run(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len();
        for b in self.buf.iter_mut() {
            b.clear();
            b.resize(n, 0.0);
        }
        let [rev_l, rev_r, echo_l, echo_r, ens_l, ens_r] = &mut self.buf;
        left.fill(0.0);
        right.fill(0.0);
        let inv = 1.0 / n.max(1) as f32;
        let ens_on = self.ensemble.active();
        // the bar levels where this run ends (the gains glide there across it)
        let at = self.pos + if self.playing { n as f64 / self.sr as f64 } else { 0.0 };
        for (ti, s) in self.slots.iter_mut().enumerate() {
            let to = target_gains(&self.mix, ti).scaled(self.timeline.level(ti, at));
            let from = s.gains;
            s.gains = to;
            s.l.resize(n, 0.0);
            s.r.resize(n, 0.0);
            s.inst.render(&mut s.l, &mut s.r);
            if let Some(st) = self.mix.strips.get(ti) {
                s.cut.set(st.low_cut, st.high_cut);
            }
            if s.cut.active() {
                s.cut.process(&mut s.l, &mut s.r);
            }
            let chorus = ens_on && self.mix.strips.get(ti).is_some_and(|st| st.ensemble);
            let (out_l, out_r) = if chorus { (&mut ens_l[..], &mut ens_r[..]) } else { (&mut *left, &mut *right) };
            let mut peak = 0.0f32;
            for k in 0..n {
                let g = Gains::lerp(from, to, (k + 1) as f32 * inv);
                let (a, b) = (s.l[k], s.r[k]);
                let ol = a * g.ll + b * g.rl;
                let or = a * g.lr + b * g.rr;
                out_l[k] += ol;
                out_r[k] += or;
                rev_l[k] += ol * g.reverb;
                rev_r[k] += or * g.reverb;
                echo_l[k] += ol * g.echo;
                echo_r[k] += or * g.echo;
                peak = peak.max(ol.abs()).max(or.abs());
            }
            if ti < MAX_TRACKS {
                self.shared.push_peak(ti, peak);
            }
            if self.last_peaks.len() <= ti {
                self.last_peaks.resize(ti + 1, 0.0);
            }
            self.last_peaks[ti] = peak;
        }
        if ens_on {
            for k in 0..n {
                let (a, b) = self.ensemble.process(ens_l[k], ens_r[k]);
                left[k] += a;
                right[k] += b;
            }
        }
        let (m0, m1) = (self.master, db_gain(self.mix.master_db) * self.volume);
        self.master = m1;
        let duck = self.mix.fx.duck.clamp(0.0, 1.0);
        let (att, rel) = ((-1.0 / (0.01 * self.sr)).exp(), (-1.0 / (0.4 * self.sr)).exp());
        let (mut pl, mut pr) = (0.0f32, 0.0f32);
        for k in 0..n {
            let (el, er) = self.echo.process(echo_l[k], echo_r[k]);
            let to_rev = self.echo.to_reverb;
            let (wl, wr) = self.reverb.process(rev_l[k] + el * to_rev, rev_r[k] + er * to_rev);
            let level = left[k].abs().max(right[k].abs());
            let c = if level > self.duck_env { att } else { rel };
            self.duck_env = level + (self.duck_env - level) * c;
            let d = 1.0 / (1.0 + duck * 10.0 * self.duck_env);
            let g = m0 + (m1 - m0) * (k + 1) as f32 * inv;
            left[k] = soft_clip((left[k] + wl * d + el) * g);
            right[k] = soft_clip((right[k] + wr * d + er) * g);
            pl = pl.max(left[k].abs());
            pr = pr.max(right[k].abs());
        }
        self.shared.push_peak(MAX_TRACKS, pl);
        self.shared.push_peak(MAX_TRACKS + 1, pr);
    }
}

fn target_gains(mix: &MixSettings, track: usize) -> Gains {
    match mix.strips.get(track) {
        Some(s) => Gains::of(s, mix.audible.get(track).copied().unwrap_or(true)),
        None => Gains::of(&Strip::default(), true),
    }
}
