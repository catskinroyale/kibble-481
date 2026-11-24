//! Nebula (D27): a 1970s analogue electronic ambient score, dreamy, spacious and cosmic,
//! at 96 bpm in 4/4, made only of analogue synthesisers (the built-in synth): a modular
//! drone, a sequencer and blips, a string machine, a vintage organ, tape-keyboard flutes
//! and a mellow lead.
//!
//! It opens on air: filtered-noise wind and breath, water-like resonant blips, and a soft
//! drone under a slowly sweeping resonant filter. A rolling sequencer pattern rises out of
//! it, then a soft, round pulsing bass. Over them a simple, naive melody of long notes,
//! written by the seed, repeats every sixteen bars, a little more each time (a passing
//! note, a higher peak, the tape flutes an octave above). String-machine chords swell and
//! fade beneath it, four bars a chord, moving between a few warm minor and major chords.
//! Whooshes, sparkling arpeggios and echoing blips drift across the stereo field. It builds
//! in density (the sequencer from quavers to semiquavers, the organ joining) and in
//! brightness (the filters opening), then dissolves back into the wind and fades. A plate
//! stands in for the spring reverb; a long, wobbling tape echo, tape drift and a little
//! hiss. No drums.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::fx;
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::compose::{Recipe, near};
use crate::project::{Ctrl, CtrlKind, Note, PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;

use crate::palette::Role;

/// The sounds added to the palette for Nebula: the other styles leave them out of their
/// choosing, so every recipe they had still sounds as it did.
pub(crate) const NEBULA_SOUNDS: [&str; 8] = [
    "Solar Wind",
    "Bubble Drops",
    "Cosmic Sequence",
    "Round Pulse Bass",
    "Starlight Lead",
    "Tape Flutes",
    "String Machine",
    "Modular Drone",
];

/// The usual arrangement: the air, the sequencer rising, the melody, the build, and as
/// long again to dissolve, shared in proportion to the length.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [6, 6, 12, 12, 10] } else { [2, 3, 4, 5, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Chord {
    root: i32,
    ivs: &'static [i32],
}

const I: Chord = Chord { root: 0, ivs: &[0, 3, 7] };
const VI: Chord = Chord { root: 8, ivs: &[0, 4, 7, 11] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 3, 7] };
const VII: Chord = Chord { root: 10, ivs: &[0, 4, 7] };

/// A few warm minor and major chords, four bars each: the minor tonic, the flat sixth
/// (with its major seventh), the relative major, the minor fourth, the flat seventh.
const PROGS: [[Chord; 4]; 4] = [[I, VI, III, VII], [I, VI, IV, VII], [I, III, VII, VI], [I, VII, VI, VII]];

/// The natural minor, the melody's scale.
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

impl Chord {
    fn pcs(self) -> Vec<i32> {
        self.ivs.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }
}

/// Every pitch of these pitch classes from `lo` to `hi`.
pub(crate) fn ladder(pcs: &[i32], lo: i32, hi: i32) -> Vec<i32> {
    (lo..=hi).filter(|p| pcs.contains(&p.rem_euclid(12))).collect()
}

/// The pitch of these pitch classes nearest `p` (the lower on a tie).
pub(crate) fn snap(p: i32, pcs: &[i32]) -> i32 {
    ladder(pcs, p - 12, p + 12).into_iter().min_by_key(|&x| ((x - p).abs(), x)).unwrap_or(p)
}

/// One note of the melody: where it starts and how long it lasts, in crotchets from the
/// start of its sixteen-bar phrase, and its pitch above the melody's tonic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MNote {
    at: f64,
    len: f64,
    rel: i32,
}

/// The melody: four segments of four bars, one to a chord, A B A' C: the A's the same
/// rhythm and the same steps from a note of their own chord, the C coming to rest.
struct Theme {
    /// Crotchets in a segment.
    seg: f64,
    notes: Vec<MNote>,
    /// Which segment each note is in.
    segs: Vec<usize>,
}

const LOW: i32 = -7;
const HIGH: i32 = 21;

impl Theme {
    fn new(seed: u64, prog: &[Chord; 4], seg: f64) -> Theme {
        let mut rng = Rng::stream(seed, "nebula-theme");
        // the rhythms, in sixteenths of a segment: long notes, a breath of a short one
        const A: [&[u32]; 5] = [&[6, 2, 4, 4], &[4, 4, 8], &[8, 4, 4], &[3, 1, 4, 8], &[6, 2, 8]];
        const B: [&[u32]; 4] = [&[4, 4, 4, 4], &[8, 8], &[2, 2, 4, 8], &[4, 2, 2, 8]];
        const C: [&[u32]; 3] = [&[8, 8], &[16], &[4, 12]];
        let a = *rng.pick(&A);
        let b = *rng.pick(&B);
        let c = *rng.pick(&C);
        let rhythms = [a, b, a, c];
        let scale = ladder(&SCALE, LOW, HIGH);
        let idx = |p: i32| scale.iter().position(|&x| x >= p).unwrap_or(scale.len() - 1);
        let moves = |rng: &mut Rng, n: usize| -> Vec<i32> { (0..n).map(|_| *rng.pick(&[-2, -1, -1, 0, 1, 1, 1, 2])).collect() };
        let a_moves = moves(&mut rng, a.len().saturating_sub(1));
        let b_moves = moves(&mut rng, b.len().saturating_sub(1));
        let c_moves = moves(&mut rng, c.len().saturating_sub(1));
        let mut notes = Vec::new();
        let mut segs = Vec::new();
        let mut last = 7;
        let mut a_start = 7;
        for (si, rh) in rhythms.iter().enumerate() {
            let chord = prog[si];
            let start = match si {
                0 => snap(7, &chord.pcs()),
                1 => snap(last + 3, &chord.pcs()),
                // (the A again from a note of its chord, unless that is a leap from the B)
                2 => Some(snap(a_start, &chord.pcs())).filter(|x| (x - last).abs() <= 7).unwrap_or_else(|| snap(last, &chord.pcs())),
                _ => snap(last - 1, &chord.pcs()),
            };
            if si == 0 {
                a_start = start;
            }
            let mv = match si {
                0 | 2 => &a_moves,
                1 => &b_moves,
                _ => &c_moves,
            };
            let mut i = idx(start);
            let mut at = 0.0;
            for (j, &d) in rh.iter().enumerate() {
                if j > 0 {
                    i = (i as i32 + mv[j - 1]).clamp(0, scale.len() as i32 - 1) as usize;
                }
                let len = d as f64 * seg / 16.0;
                let mut rel = scale[i];
                // long notes, and the last of all, rest on the chord
                if j == 0 || len >= seg / 4.0 || (si == 3 && j + 1 == rh.len()) {
                    rel = snap(rel, &chord.pcs()).clamp(LOW, HIGH);
                    i = idx(rel);
                }
                notes.push(MNote { at: si as f64 * seg + at, len, rel });
                segs.push(si);
                at += len;
                last = rel;
            }
        }
        Theme { seg, notes, segs }
    }

    /// The melody's `rep`th time round: from the second, the longest note of the B and C
    /// steps up on its last beat to lean into what follows; from the third, the B's
    /// highest note reaches a step higher.
    fn render(&self, rep: u32) -> Vec<MNote> {
        let scale = ladder(&SCALE, LOW, HIGH + 2);
        let up = |p: i32| scale.iter().copied().find(|&x| x > p).unwrap_or(p);
        let mut out: Vec<MNote> = self.notes.clone();
        if rep >= 2 {
            let top = (0..out.len()).filter(|&i| self.segs[i] == 1).max_by_key(|&i| (out[i].rel, i));
            if let Some(i) = top {
                out[i].rel = up(out[i].rel);
            }
        }
        if rep >= 1 {
            let mut extra = Vec::new();
            for s in [1, 3] {
                let long = (0..out.len()).filter(|&i| self.segs[i] == s).max_by(|&x, &y| out[x].len.total_cmp(&out[y].len));
                if let Some(i) = long.filter(|&i| out[i].len >= self.seg / 4.0 && (s != 3 || i + 1 < out.len())) {
                    let beat = 1.0;
                    out[i].len -= beat;
                    extra.push(MNote { at: out[i].at + out[i].len, len: beat, rel: up(out[i].rel) });
                }
            }
            out.extend(extra);
            out.sort_by(|a, b| a.at.total_cmp(&b.at));
        }
        out
    }
}

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Wind,
    Bubbles,
    Drone,
    Sequence,
    Bass,
    Strings,
    Lead,
    Flutes,
    Organ,
    Sparkle,
    Whoosh,
    Hiss,
}

const LAYERS: [Layer; 12] = [
    Layer::Wind,
    Layer::Bubbles,
    Layer::Drone,
    Layer::Sequence,
    Layer::Bass,
    Layer::Strings,
    Layer::Lead,
    Layer::Flutes,
    Layer::Organ,
    Layer::Sparkle,
    Layer::Whoosh,
    Layer::Hiss,
];

/// 0 to 1 as `u` goes from `a` to `b`, eased.
pub(crate) fn ramp(u: f64, a: f64, b: f64) -> f64 {
    let x = ((u - a) / (b - a)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Every layer's presence and the intensity, a value a quaver (`q` ticks) over the piece,
/// from `presence(layer, part, u)` and `intensity(part, u)` with `u` how far through
/// its section; never moving faster than a full fade in about four seconds (a layer by
/// 0.06 a quaver, the intensity by 0.03), so in any order of sections nothing jumps.
pub(crate) fn envelopes<const N: usize>(
    secs: &[(Part, u32, u32)],
    end: u32,
    q: u32,
    presence: impl Fn(usize, Part, f64) -> f64,
    intensity: impl Fn(Part, f64) -> f64,
) -> (Vec<[f64; N]>, Vec<f64>) {
    let nq = end.div_ceil(q) as usize;
    let mut env = vec![[0.0f64; N]; nq];
    let mut force = vec![0.0f64; nq];
    let mut prev: Option<([f64; N], f64)> = None;
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let zz = if si + 1 == secs.len() { end } else { z };
        let span = (zz - a).max(1) as f64;
        for qi in (a / q) as usize..(zz.div_ceil(q) as usize).min(nq) {
            let u = (qi as u32 * q).saturating_sub(a) as f64 / span;
            let mut e: [f64; N] = std::array::from_fn(|l| presence(l, part, u));
            let mut x = intensity(part, u);
            if let Some((pe, px)) = prev {
                for (v, p) in e.iter_mut().zip(pe) {
                    *v = p + (*v - p).clamp(-0.06, 0.06);
                }
                x = px + (x - px).clamp(-0.03, 0.03);
            }
            (env[qi], force[qi]) = (e, x);
            prev = Some((e, x));
        }
    }
    (env, force)
}

/// How present a layer is `u` of the way through a section of this part: the air first,
/// the sequencer and bass rising out of it, the melody and strings, everything at the
/// build, and the air again as the rest falls away, all fading at the very end.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.72, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (_, Hiss) | (_, Whoosh) => 1.0,
        (Part::Intro, Wind) => 1.0,
        (Part::Intro, Bubbles) => 0.9,
        (Part::Intro, Drone) => ramp(u, -0.1, 0.35),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Wind) => 1.0 - 0.7 * ramp(u, 0.0, 0.8),
        (Part::Neon, Bubbles) => 0.8,
        (Part::Neon, Drone) => 1.0,
        (Part::Neon, Sequence) => ramp(u, -0.05, 0.45),
        (Part::Neon, Bass) => ramp(u, 0.3, 0.75),
        (Part::Neon, Strings) => 0.6 * ramp(u, 0.45, 1.0),
        (Part::Neon, Sparkle) => 0.4,
        // (the melody only plays from the Swell: here it is ready for it)
        (Part::Neon, Lead) => ramp(u, 0.8, 1.0),
        (Part::Neon, _) => 0.0,
        (Part::Swell, Wind) => 0.3 - 0.15 * u,
        (Part::Swell, Bubbles) => 0.55,
        (Part::Swell, Strings) => 0.6 + 0.3 * u,
        (Part::Swell, Organ) => 0.6 * ramp(u, 0.5, 1.0),
        (Part::Swell, Flutes) => 0.0,
        (Part::Swell, Sparkle) => 0.8,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Wind) => 0.15,
        (Part::NightShift, Bubbles) => 0.6,
        (Part::NightShift, Flutes) => ramp(u, 0.05, 0.4),
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Wind) => 0.15 + 0.85 * ramp(u, 0.2, 0.6),
        (Part::Fade, Bubbles) => 0.9,
        (Part::Fade, Drone) => 1.0,
        (Part::Fade, Sequence) => 1.0 - ramp(u, 0.0, 0.45),
        (Part::Fade, Bass) => 1.0 - ramp(u, 0.05, 0.4),
        (Part::Fade, Strings) => 1.0 - ramp(u, 0.2, 0.7),
        (Part::Fade, Lead) => 1.0 - ramp(u, 0.15, 0.4),
        (Part::Fade, Flutes) => 1.0 - ramp(u, 0.0, 0.25),
        (Part::Fade, Organ) => 1.0 - ramp(u, 0.05, 0.35),
        (Part::Fade, Sparkle) => 0.6 * (1.0 - ramp(u, 0.0, 0.4)),
    };
    v * end
}

/// How strongly and brightly everything plays: rising section by section to the build,
/// falling away through the Fade.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2 + 0.05 * u,
        Part::Neon => 0.25 + 0.2 * u,
        Part::Swell => 0.45 + 0.2 * u,
        Part::NightShift => 0.65 + 0.35 * (u / 0.75).min(1.0),
        Part::Fade => 0.9 - 0.7 * u,
    }
}

pub(crate) struct Writer {
    pub(crate) notes: Vec<Vec<Note>>,
    pub(crate) ctrls: Vec<Vec<Ctrl>>,
    pub(crate) end: u32,
    pub(crate) touch: Rng,
}

impl Writer {
    pub(crate) fn note(&mut self, track: usize, start: u32, len: u32, key: i32, vel: f64) {
        if start >= self.end || len == 0 {
            return;
        }
        let vel = (vel + self.touch.range(-3, 3) as f64).round().clamp(1.0, 127.0) as u8;
        self.notes[track].push(Note { start, len: len.min(self.end - start), key: key.clamp(0, 127) as u8, vel });
    }

    pub(crate) fn cc(&mut self, track: usize, tick: u32, cc: u8, v: f64) {
        if tick < self.end {
            self.ctrls[track].push(Ctrl { tick, kind: CtrlKind::Cc(cc), value: v.round().clamp(0.0, 127.0) as i16 });
        }
    }
}

/// Nebula's synthesisers, mixed wide: the air and effects spread across the field, the
/// string machine, organ and flutes through the ensemble, the bass and drone centred, the
/// lead, blips and sparkles into the tape echo.
fn tracks(p: &mut Project) -> [usize; 12] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    [
        add("Wind", Role::Pad, synth("Solar Wind"), Strip { reverb: 0.6, echo: 0.15, width: 1.9, gain_db: -6.0, ..s }),
        add("Blips", Role::Pluck, synth("Bubble Drops"), Strip { reverb: 0.45, echo: 0.5, pan: 0.25, width: 1.9, gain_db: -5.0, ..s }),
        add("Drone", Role::Drone, synth("Modular Drone"), Strip { reverb: 0.5, width: 1.4, gain_db: -7.0, ..s }),
        add(
            "Sequence",
            Role::Pluck,
            synth("Cosmic Sequence"),
            Strip { reverb: 0.4, echo: 0.3, pan: -0.15, width: 1.5, gain_db: -2.0, ..s },
        ),
        add("Bass", Role::Bass, synth("Round Pulse Bass"), Strip { reverb: 0.1, gain_db: 0.0, high_cut: 1200.0, ..s }),
        add(
            "String Machine",
            Role::Strings,
            synth("String Machine"),
            Strip { reverb: 0.55, echo: 0.1, width: 1.8, ensemble: true, gain_db: -11.0, ..s },
        ),
        add("Lead", Role::Lead, synth("Starlight Lead"), Strip { reverb: 0.5, echo: 0.4, pan: 0.05, gain_db: -6.0, ..s }),
        add(
            "Tape Flutes",
            Role::Lead,
            synth("Tape Flutes"),
            Strip { reverb: 0.55, echo: 0.3, pan: 0.3, ensemble: true, gain_db: -9.0, ..s },
        ),
        add("Organ", Role::Keys, synth("Velour Organ"), Strip { reverb: 0.5, pan: -0.25, ensemble: true, gain_db: -10.0, ..s }),
        add("Sparkle", Role::Bells, synth("Glass Chimes"), Strip { reverb: 0.55, echo: 0.6, pan: 0.45, width: 1.8, gain_db: -7.0, ..s }),
        add("Whoosh", Role::Pad, synth("Hover Sweep"), Strip { reverb: 0.65, echo: 0.25, width: 2.0, gain_db: -15.0, ..s }),
        add("Tape Hiss", Role::City, Source::Kit(KitKind::CityFx), Strip { reverb: 0.0, gain_db: -6.0, high_cut: 9000.0, ..s }),
    ]
}

/// Write Nebula into `p`: its synthesisers, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a plate for the spring, a long and wobbling tape echo, tape drift, the ensemble
    p.fx.space = Space::Plate;
    p.fx.size = 0.85;
    p.fx.tone = 0.5;
    p.fx.motion = 0.5;
    p.fx.predelay = 0.1;
    p.fx.width = 1.0;
    p.fx.duck = 0.2;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.55;
    p.fx.drift = 0.2;
    p.fx.detune = 0.05;
    p.fx.echo.time = EchoTime::DottedQuarter;
    p.fx.echo.feedback = 0.55;
    p.fx.echo.tone = 0.35;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.5;
    p.fx.echo.wow = 0.25;
    p.master_db = 5.5;

    let bar = p.bar_ticks();
    let q = PPQ / 2;
    let s16 = PPQ / 4;
    let steps = (bar / s16).max(1);
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "nebula-harmony");
    let prog = *hr.pick(&PROGS);
    let seg = 4.0 * bar as f64 / PPQ as f64;
    let theme = Theme::new(r.seed, &prog, seg);
    let mut sr = Rng::stream(r.seed, "nebula-sequence");
    const PATTERNS: [[usize; 8]; 5] =
        [[0, 2, 1, 2, 3, 2, 1, 2], [0, 1, 2, 3, 2, 1, 2, 1], [0, 2, 4, 2, 1, 2, 3, 2], [0, 0, 2, 1, 3, 1, 2, 1], [0, 2, 3, 4, 3, 2, 1, 2]];
    let pattern = *sr.pick(&PATTERNS);
    // the semiquavers between the quavers the sequencer plays as it fills out in the Swell
    let fill: Vec<bool> = (0..steps).map(|_| sr.chance(0.45)).collect();
    let mut ar = Rng::stream(r.seed, "nebula-air");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "nebula-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [wind, blips, drone, seq, bass, strings, lead, flutes, organ, sparkle, whoosh, hiss] = tr;
    let lead_tonic = pitch(0, 60);

    // every layer's presence and the intensity, a value a quaver over the whole piece,
    // never moving faster than a full fade in about four seconds, so in any order of
    // sections nothing jumps
    let nq = end.div_ceil(q) as usize;
    let mut env = vec![[0.0f64; 12]; nq];
    let mut force = vec![0.0f64; nq];
    let mut prev: Option<([f64; 12], f64)> = None;
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let zz = if si + 1 == secs.len() { end } else { z };
        let span = (zz - a).max(1) as f64;
        for qi in (a / q) as usize..(zz.div_ceil(q) as usize).min(nq) {
            let u = (qi as u32 * q).saturating_sub(a) as f64 / span;
            let mut e = LAYERS.map(|l| presence(l, part, u));
            let mut x = intensity(part, u);
            if let Some((pe, px)) = prev {
                for (v, p) in e.iter_mut().zip(pe) {
                    *v = p + (*v - p).clamp(-0.06, 0.06);
                }
                x = px + (x - px).clamp(-0.03, 0.03);
            }
            (env[qi], force[qi]) = (e, x);
            prev = Some((e, x));
        }
    }
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.55 + 0.45 * inten(t));

    // the tape's hiss under everything
    w.note(hiss, 0, end, fx::TAPE_HISS as i32, 34.0);
    let mut whooshes: Vec<(u32, u32)> = Vec::new();
    let mut rep = 0u32;
    let mut strings_spans: Vec<(u32, u32)> = Vec::new();
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the drone: the tonic and fifth held through the section, overlapping the next
        let dl = (zz - a) + bar;
        w.note(drone, a, dl, pitch(0, 45), 70.0);
        w.note(drone, a, dl, pitch(7, 52), 52.0);
        // a whoosh across each new section, and now and then within one
        if si > 0 {
            let s = a.saturating_sub(bar);
            whooshes.push((s, 3 * bar));
            w.note(whoosh, s, 3 * bar, ar.range(62, 82), 80.0);
        }
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let pcs = chord.pcs();
            let new_chord = k == 0 || chord_at(k) != chord_at(k - 1) || k % 4 == 0;
            // wind: a gust every two bars or so, its pitch the filter's
            if k % 2 == 0 && pres(Layer::Wind, t_bar) > 0.05 {
                let len = bar * ar.range(3, 5) as u32;
                w.note(wind, t_bar + ar.range(0, 3) as u32 * PPQ, len, ar.range(58, 88), ar.range_f(55.0, 95.0));
            }
            // blips: a burst of water drops, high, falling or rising
            let chance = 0.25 + 0.6 * pres(Layer::Bubbles, t_bar);
            if pres(Layer::Bubbles, t_bar) > 0.05 && ar.chance(chance) {
                let n = ar.range(2, 6);
                let mut t = t_bar + ar.range(0, steps as i32 - 4) as u32 * s16;
                let gap = if ar.chance(0.5) { s16 } else { s16 / 2 };
                let pent = ladder(&[0, 3, 5, 7, 10].map(|x| (x + root).rem_euclid(12)), 72, 98);
                let mut i = ar.range(0, pent.len() as i32 - 1);
                let dir = if ar.chance(0.5) { 1 } else { -1 };
                for _ in 0..n {
                    w.note(blips, t, gap, pent[i.clamp(0, pent.len() as i32 - 1) as usize], ar.range_f(50.0, 85.0));
                    i += dir * ar.range(1, 2);
                    t += gap;
                }
            }
            // the sequencer: quavers, filling to semiquavers as it builds
            let sp = pres(Layer::Sequence, t_bar);
            if sp > 0.02 {
                let base = pitch(chord.root, 48);
                let tones = ladder(&pcs.iter().map(|x| (x + root).rem_euclid(12)).collect::<Vec<_>>(), base, base + 26);
                for st in 0..steps {
                    let t = t_bar + st * s16;
                    if t >= zz {
                        break;
                    }
                    let dense = inten(t);
                    let play = st % 2 == 0 || dense > 0.78 || (dense > 0.5 && fill[st as usize]);
                    if !play || pres(Layer::Sequence, t) <= 0.02 {
                        continue;
                    }
                    let idx = pattern[(st % 8) as usize].min(tones.len() - 1);
                    let accent = if st % 4 == 0 {
                        10.0
                    } else if st % 2 == 0 {
                        3.0
                    } else {
                        -6.0
                    };
                    w.note(seq, t, s16 * 3 / 4, tones[idx], vel(74.0, t) + accent);
                }
            }
            // the bass: soft round quavers on the chord's root
            if pres(Layer::Bass, t_bar) > 0.02 {
                let key = pitch(chord.root, 38);
                for e in 0..steps / 2 {
                    let t = t_bar + e * q;
                    if t < zz && pres(Layer::Bass, t) > 0.02 {
                        let v = if e == 0 {
                            80.0
                        } else if e % 2 == 0 {
                            64.0
                        } else {
                            54.0
                        };
                        w.note(bass, t, q * 7 / 10, key, vel(v, t));
                    }
                }
            }
            // the string machine and the organ: the chord held, four bars
            if new_chord {
                let span = (4 - k % 4).min(nb - k) * bar;
                let len = span.min(zz.saturating_sub(t_bar) + bar / 2);
                if pres(Layer::Strings, t_bar + span / 2) > 0.03 {
                    for (i, &pc) in pcs.iter().enumerate() {
                        w.note(strings, t_bar, len, pitch(pc, 60 + 2 * i as i32), 70.0);
                    }
                    w.note(strings, t_bar, len, pitch(chord.root, 48), 60.0);
                    strings_spans.push((t_bar, len));
                }
                if pres(Layer::Organ, t_bar + span / 2) > 0.03 {
                    for &pc in &pcs {
                        w.note(organ, t_bar, len.saturating_sub(q / 3), pitch(pc, 58), vel(64.0, t_bar));
                    }
                }
            }
            // sparkles: a chord rising in a quick tinkling run, every few bars
            let every = if inten(t_bar) > 0.7 { 2 } else { 4 };
            if k % every == every - 1 && pres(Layer::Sparkle, t_bar) > 0.1 {
                let tones = ladder(&pcs.iter().map(|x| (x + root).rem_euclid(12)).collect::<Vec<_>>(), 76, 100);
                let t0 = t_bar + bar / 2 + ar.range(0, 3) as u32 * s16;
                let n = ar.range(6, 10).min(tones.len() as i32) as usize;
                for (j, &key) in tones.iter().take(n).enumerate() {
                    w.note(sparkle, t0 + j as u32 * PPQ / 6, PPQ, key, 72.0 - 3.0 * j as f64);
                }
            }
            // a whoosh now and then mid-section
            if k % 8 == 5 && inten(t_bar) > 0.4 && ar.chance(0.6) {
                whooshes.push((t_bar, 2 * bar));
                w.note(whoosh, t_bar, 2 * bar, ar.range(62, 82), 70.0);
            }
        }
        // the melody: its sixteen bars from the section's start, round again and again,
        // the tape flutes doubling it an octave up at the build
        if matches!(part, Part::Swell | Part::NightShift | Part::Fade) {
            let phrase = 16 * bar;
            let mut ph = a;
            while ph < zz {
                for n in theme.render(rep) {
                    let s = ph + (n.at * PPQ as f64).round() as u32;
                    if s >= zz || pres(Layer::Lead, s) < 0.3 {
                        continue;
                    }
                    let len = (n.len * PPQ as f64).round() as u32 + PPQ / 16;
                    // (softer as it leaves, in the Fade)
                    let lv = if part == Part::Fade { 70.0 } else { 84.0 };
                    w.note(lead, s, len, lead_tonic + n.rel, vel(lv, s) + if n.len >= 4.0 { 4.0 } else { 0.0 });
                    if pres(Layer::Flutes, s) > 0.2 {
                        w.note(flutes, s, len, lead_tonic + 12 + n.rel, vel(66.0, s));
                    }
                }
                rep += 1;
                ph += phrase;
            }
        }
    }
    // expression a quaver at a time, and the filters: the sequencer and drone sweeping
    // slowly and opening as it builds, the strings swelling with each chord, the
    // whooshes sweeping up and down
    let sweep = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        if *layer == Layer::Hiss {
            let mut last = -1.0;
            let mut t = 0;
            while t < end {
                let v = (127.0 * pres(Layer::Hiss, t)).round();
                if v != last {
                    w.cc(track, t, 11, v);
                    last = v;
                }
                t += q;
            }
            continue;
        }
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let mut v = 127.0 * pres(*layer, t) * (0.7 + 0.3 * inten(t));
            if *layer == Layer::Strings
                && let Some((s, len)) = strings_spans.iter().rev().find(|(s, len)| t >= *s && t < s + len)
            {
                let x = (t - s) as f64 / *len as f64;
                v *= 0.55 + 0.45 * (std::f64::consts::PI * x).sin();
            }
            // (and never faster than a step of eight a quaver, whatever moves it)
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Sequence => Some(34.0 + 48.0 * inten(t) + 16.0 * sweep(t, 8.0, 0.0)),
                Layer::Drone => Some(44.0 + 30.0 * inten(t) + 22.0 * sweep(t, 16.0, 0.25)),
                Layer::Strings => Some(50.0 + 30.0 * inten(t)),
                Layer::Lead => Some(52.0 + 26.0 * inten(t)),
                Layer::Whoosh => Some(
                    whooshes
                        .iter()
                        .rev()
                        .find(|(s, len)| t >= *s && t < s + len)
                        .map_or(40.0, |(s, len)| 30.0 + 85.0 * (std::f64::consts::PI * (t - s) as f64 / *len as f64).sin()),
                ),
                _ => None,
            };
            if let Some(b) = b {
                let b = b.round().clamp(0.0, 127.0);
                if b != l74 {
                    w.cc(track, t, 74, b);
                    l74 = b;
                }
            }
            t += q;
        }
    }
    for (i, t) in p.tracks.iter_mut().enumerate() {
        t.notes.append(&mut w.notes[i]);
        t.ctrls.append(&mut w.ctrls[i]);
        t.ctrls.retain(|c| c.tick < end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Style, compose};

    fn piece(seed: u64, bars: u32) -> Project {
        compose(&Recipe { style: Style::Nebula, ..Recipe::random(seed) }, Style::Nebula.tempo(), (4, 4), bars)
    }

    fn track<'a>(p: &'a Project, name: &str) -> &'a Track {
        p.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name}"))
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: air, blips and the drone alone at first; the sequencer rising in the
    /// Neon, the melody from the Swell, loudest and brightest at the build; the air again
    /// at the end, with the sequencer, bass and melody gone. Synthesisers only, no drums.
    #[test]
    fn nebula_rises_and_dissolves() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            for name in ["Wind", "Drone"] {
                assert!(starts(&p, name, b[0]) >= 1, "seed {seed} {name}");
            }
            for name in ["Sequence", "Bass", "Lead", "String Machine", "Organ"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Sequence", b[1]) > 8, "seed {seed}");
            assert_eq!(starts(&p, "Lead", b[1]), 0, "seed {seed}");
            assert!(starts(&p, "Lead", b[2]) >= 4, "seed {seed}");
            // the sequencer busier at the build than as it enters
            let per_bar = |i: usize| starts(&p, "Sequence", b[i]) as f64 / ((b[i].1 - b[i].0) / p.bar_ticks()) as f64;
            assert!(per_bar(3) > per_bar(1) * 1.4, "seed {seed}: {} {}", per_bar(1), per_bar(3));
            let lead = track(&p, "Lead");
            let top = |(a, z): (u32, u32)| lead.notes.iter().filter(|n| n.start >= a && n.start < z).map(|n| n.vel).max().unwrap_or(0);
            assert!(top(b[3]) >= top(b[2]).max(top(b[4])), "seed {seed}");
            let bright = track(&p, "Sequence").ctrls.iter().filter(|c| c.kind == CtrlKind::Cc(74)).max_by_key(|c| c.value).unwrap();
            assert!(bright.tick >= b[2].0 && bright.tick < b[4].0, "seed {seed}");
            let tail = b[4].0 + (b[4].1 - b[4].0) / 2;
            for name in ["Sequence", "Bass", "Lead", "Tape Flutes"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Wind", (tail, p.end_tick())) >= 1, "seed {seed}");
            for t in &p.tracks {
                match t.name.as_str() {
                    "Tape Hiss" => assert_eq!(t.source, Source::Kit(KitKind::CityFx)),
                    _ => assert!(matches!(t.source, Source::Synth(_)), "{}", t.name),
                }
                assert!(!matches!(t.role, Some(Role::Drums)));
            }
            assert!(track(&p, "Tape Hiss").notes.iter().all(|n| n.key == fx::TAPE_HISS));
        }
    }

    /// The melody: long, slow notes; its A segments the same rhythm and steps; each time
    /// round it keeps its opening and grows a little.
    #[test]
    fn the_melody_repeats_and_evolves() {
        for seed in 0..40 {
            for prog in PROGS {
                let th = Theme::new(seed, &prog, 16.0);
                let seg = |s: usize| -> Vec<(f64, i32)> {
                    th.notes.iter().zip(&th.segs).filter(|(_, x)| **x == s).map(|(n, _)| (n.at - s as f64 * 16.0, n.rel)).collect()
                };
                let (a, a2) = (seg(0), seg(2));
                assert_eq!(a.iter().map(|x| x.0).collect::<Vec<_>>(), a2.iter().map(|x| x.0).collect::<Vec<_>>());
                let mean = th.notes.iter().map(|n| n.len).sum::<f64>() / th.notes.len() as f64;
                assert!(mean >= 3.0, "seed {seed}: {mean}");
                assert!(th.notes.iter().all(|n| (LOW..=HIGH).contains(&n.rel)));
                assert!(th.notes.windows(2).all(|w| (w[1].rel - w[0].rel).abs() <= 9), "seed {seed}");
                let (r0, r1, r2) = (th.render(0), th.render(1), th.render(2));
                assert_eq!(r0, th.notes);
                assert!(r1.len() > r0.len() && r2.len() >= r1.len());
                assert_eq!(r0[0], r1[0]);
                let total = |v: &[MNote]| v.iter().map(|n| n.len).sum::<f64>();
                assert!((total(&r0) - 64.0).abs() < 1e-9 && (total(&r1) - 64.0).abs() < 1e-9);
            }
        }
    }

    /// In any arrangement no layer jumps.
    #[test]
    fn no_sudden_changes_in_any_order() {
        use crate::compose::{compose_plan, random_plan};
        let orders: [&[Part]; 3] = [
            &[Part::Intro, Part::Neon, Part::NightShift, Part::Swell, Part::NightShift, Part::Fade],
            &[Part::Swell, Part::Fade, Part::Neon, Part::NightShift, Part::Intro],
            &[Part::NightShift, Part::Intro, Part::Fade, Part::Swell],
        ];
        let mut plans: Vec<Vec<(Part, u32)>> = orders.iter().map(|o| o.iter().map(|p| (*p, 8)).collect()).collect();
        for seed in 0..12 {
            let r = Recipe { style: Style::Nebula, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 96.0, (4, 4), 48));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Nebula, ..Recipe::random(i as u64) }, 96.0, (4, 4), plan);
            for t in &p.tracks {
                let cc: Vec<i16> = t.ctrls.iter().filter(|c| c.kind == CtrlKind::Cc(11)).map(|c| c.value).collect();
                assert!(cc.windows(2).all(|w| (w[1] - w[0]).abs() <= 8), "{plan:?} {}: {cc:?}", t.name);
            }
        }
    }

    /// Any arrangement and metre: the length exactly, and sound in every section.
    #[test]
    fn any_arrangement() {
        for (meter, bars) in [((4, 4), 9), ((3, 4), 24), ((6, 8), 17), ((5, 4), 33), ((4, 4), 3), ((15, 8), 20), ((7, 8), 40)] {
            let p = compose(&Recipe { style: Style::Nebula, ..Recipe::random(7) }, 96.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
