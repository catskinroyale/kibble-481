//! Orbit (D26): a hypnotic minimalist score in 15/8, felt as groups of seven and eight
//! quavers, in A minor leaning towards its relative major without ever settling there.
//! Everything turns on one short piano figure of two bars, written by the seed, that
//! loops from the first bar to the last on a soft, felted grand.
//!
//! Layers join it one at a time, each looping a pattern of its own: a rounded bass guitar
//! on the bar and its second group; a glockenspiel figure in dotted crotchets, five to the
//! bar against the piano's seven and eight; a warm drawbar organ holding the chords; a
//! clean acoustic guitar's counter-line, its shape kept while its notes follow the
//! harmony. The harmony moves under the loop two bars a chord (the minor tonic, the flat
//! sixth, the relative major, the fourth, and a suspended seventh that never resolves
//! to it). Nothing arrives suddenly: each layer fades in and out on its expression and
//! the touch rises section by section. At the peak tubular bells ring a slow melody over
//! the full ensemble, the loudest moment; then the layers leave in the order they came,
//! back to the piano alone, fading, onto a minor seventh that is also the relative
//! major's sixth. A concert hall and a little tape drift; no drums, no city.

use dd_dsp::fx::Space;

use crate::compose::{Recipe, near};
use crate::palette::{Alt, Role, alternatives};
use crate::project::{Ctrl, CtrlKind, Note, PPQ, Part, Project, Strip, Track};
use crate::rng::Rng;

/// The sounds added to the palette for Orbit: the other styles leave them out of their
/// choosing, so every recipe they had still sounds as it did.
pub(crate) const ORBIT_SOUNDS: [&str; 3] = ["Drawbar Organ", "Finger Bass", "Steel Guitar"];

/// The usual arrangement: the figure alone, two sections of layers arriving, the peak,
/// and as long again for them to leave, shared in proportion to the length.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 20 { [4, 8, 8, 8, 8] } else { [2, 4, 4, 4, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Chord {
    root: i32,
    ivs: &'static [i32],
}

/// The minor tonic, the flat sixth, the relative major, the minor fourth, and the
/// seventh suspended (no leading note to pull it home to the relative major).
const I: Chord = Chord { root: 0, ivs: &[0, 3, 7] };
const VI: Chord = Chord { root: 8, ivs: &[0, 4, 7] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 3, 7] };
const VII: Chord = Chord { root: 10, ivs: &[0, 5, 7] };
/// The last chord: the minor seventh, which is also the relative major's sixth.
const LAST: Chord = Chord { root: 0, ivs: &[0, 3, 7, 10] };

/// Each part's progressions, two bars a chord, looping under the figure. The seventh is
/// never followed by the relative major, so the major is hinted at and never arrived at.
const NEON: [&[Chord]; 3] = [&[I, VI], &[I, IV], &[I, I, VI, IV]];
const SWELL: [&[Chord]; 3] = [&[I, VI, III, VII], &[I, VI, IV, VII], &[I, III, VI, VII]];
const PEAK: [&[Chord]; 3] = [&[VI, III, VII, I], &[I, VI, III, VII], &[IV, VI, III, VII]];
const FADE: [&[Chord]; 2] = [&[I, VI], &[I, IV]];

impl Chord {
    /// Its pitch classes above the tonic.
    fn pcs(self) -> Vec<i32> {
        self.ivs.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }
}

/// The two groups a bar of `n` quavers is felt in: 15 as seven and eight.
fn groups(n: u32) -> Vec<u32> {
    if n < 4 { vec![n] } else { vec![n / 2, n - n / 2] }
}

/// A group cut into cells of two and three quavers, the order the seed's.
fn cells(g: u32, rng: &mut Rng) -> Vec<u32> {
    match g {
        0 => vec![],
        1..=3 => vec![g],
        4 => vec![2, 2],
        5 => rng.pick(&[vec![2, 3], vec![3, 2]]).clone(),
        6 => rng.pick(&[vec![3, 3], vec![2, 2, 2]]).clone(),
        7 => rng.pick(&[vec![2, 2, 3], vec![2, 3, 2], vec![3, 2, 2]]).clone(),
        8 => rng.pick(&[vec![3, 3, 2], vec![3, 2, 3], vec![2, 3, 3]]).clone(),
        _ => {
            let mut v = Vec::new();
            let mut rem = g;
            while rem > 0 {
                let c = if rem == 4 {
                    2
                } else if rem <= 3 {
                    rem
                } else {
                    3
                };
                v.push(c);
                rem -= c;
            }
            v
        }
    }
}

/// Every pitch of these pitch classes from `lo` to `hi` (semitones above the tonic).
fn ladder(pcs: &[i32], lo: i32, hi: i32) -> Vec<i32> {
    (lo..=hi).filter(|p| pcs.contains(&p.rem_euclid(12))).collect()
}

/// One note of a looping figure: its quaver in the two bars, its pitch (semitones above
/// the figure's low tonic) and its accent (1 starting a group, a half starting a cell).
#[derive(Clone, Copy, Debug, PartialEq)]
struct FNote {
    at: u32,
    rel: i32,
    accent: f64,
}

/// The piece's loops, all written once from the seed.
struct Loops {
    /// The cells of each group of a bar (the same in every bar).
    cells: Vec<Vec<u32>>,
    /// The piano's figure over two bars.
    piano: Vec<FNote>,
    /// The glockenspiel's five-note cell, in dotted crotchets, and the one it rests on.
    glock: Vec<i32>,
    glock_rest: Option<usize>,
    /// The guitar's counter-line: a step along its chord's notes at each cell's second
    /// quaver, over two bars.
    guitar: Vec<(u32, usize)>,
}

impl Loops {
    fn new(seed: u64, n: u32) -> Loops {
        let mut rng = Rng::stream(seed, "orbit-figure");
        let gs = groups(n);
        let cells: Vec<Vec<u32>> = gs.iter().map(|&g| cells(g, &mut rng)).collect();
        // the figure's notes: the tonic triad and one colour, the ninth or the seventh
        let colour = if rng.chance(0.6) { 2 } else { 10 };
        let lad = ladder(&[0, 3, 7, colour], 0, 24);
        const SHAPES2: [[usize; 2]; 3] = [[0, 2], [0, 3], [2, 0]];
        const SHAPES3: [[usize; 3]; 4] = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [0, 2, 4]];
        // each cell's shape and where it starts, the second group a step higher
        let mut plan: Vec<(u32, Vec<usize>, usize)> = Vec::new();
        for (gi, cs) in cells.iter().enumerate() {
            for &c in cs {
                let shape: Vec<usize> = match c {
                    1 => vec![0],
                    2 => rng.pick(&SHAPES2).to_vec(),
                    _ => rng.pick(&SHAPES3).iter().copied().chain(std::iter::repeat(1)).take(c as usize).collect(),
                };
                let anchor = rng.range(0, 2) as usize + gi;
                plan.push((c, shape, anchor));
            }
        }
        // the second bar answers the first: its last group's cells a step up or down
        let last_group = cells.last().map_or(0, |c| c.len());
        let answer: Vec<i32> = (0..last_group).map(|_| if rng.chance(0.5) { 1 } else { -1 }).collect();
        let mut piano = Vec::new();
        for bar in 0..2u32 {
            let mut at = bar * n;
            let mut ci = 0;
            for (gi, cs) in cells.iter().enumerate() {
                for (k, _) in cs.iter().enumerate() {
                    let (c, shape, anchor) = &plan[ci];
                    let mut anchor = *anchor as i32;
                    if bar == 1 && gi + 1 == cells.len() {
                        anchor = (anchor + answer[k]).max(0);
                    }
                    for (j, off) in shape.iter().enumerate().take(*c as usize) {
                        let idx = ((anchor as usize) + off).min(lad.len() - 1);
                        let accent = if k == 0 && j == 0 {
                            1.0
                        } else if j == 0 {
                            0.5
                        } else {
                            0.0
                        };
                        piano.push(FNote { at: at + j as u32, rel: lad[idx], accent });
                    }
                    at += c;
                    ci += 1;
                }
            }
        }
        // the glockenspiel's cell: a slow walk on the same notes, high
        let high = ladder(&[0, 3, 7, colour], 12, 31);
        let mut i = rng.range(1, 3) as usize;
        let mut glock = Vec::new();
        for _ in 0..5 {
            glock.push(high[i.min(high.len() - 1)]);
            let step = *rng.pick(&[-2i32, -1, 1, 1, 2]);
            i = (i as i32 + step).clamp(0, high.len() as i32 - 1) as usize;
        }
        let glock_rest = rng.chance(0.7).then(|| rng.range(1, 4) as usize);
        // the guitar: at the second quaver of each cell, a step along its chord
        let mut guitar = Vec::new();
        let mut idx = rng.range(2, 3);
        for bar in 0..2u32 {
            let mut at = bar * n;
            for cs in &cells {
                for &c in cs {
                    if c >= 2 {
                        guitar.push((at + 1, idx as usize));
                        idx = (idx + *rng.pick(&[-1i32, 1, 1, -1, 2, -2, 0])).clamp(0, 5);
                    }
                    at += c;
                }
            }
        }
        Loops { cells, piano, glock, glock_rest, guitar }
    }

    /// Where each group starts in a bar, in quavers.
    fn group_starts(&self) -> Vec<u32> {
        let mut v = Vec::new();
        let mut at = 0;
        for cs in &self.cells {
            v.push(at);
            at += cs.iter().sum::<u32>();
        }
        v
    }
}

/// The layers, in the order they arrive.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Piano,
    Bass,
    Glock,
    Organ,
    Guitar,
    Bells,
}

const LAYERS: [Layer; 6] = [Layer::Piano, Layer::Bass, Layer::Glock, Layer::Organ, Layer::Guitar, Layer::Bells];

/// 0 to 1 as `u` goes from `a` to `b`, eased.
fn ramp(u: f64, a: f64, b: f64) -> f64 {
    let x = ((u - a) / (b - a)).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// How present a layer is `u` of the way through a section of this part: they fade in
/// one at a time over the Neon and the Swell, all play the peak, and fade out in the
/// order they came through the Fade, leaving the piano.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    match (part, layer) {
        (_, Piano) => {
            if part == Part::Fade {
                1.0 - 0.75 * ramp(u, 0.8, 1.0)
            } else {
                1.0
            }
        }
        (Part::Intro, _) => 0.0,
        (Part::Neon, Bass) => ramp(u, -0.05, 0.3),
        (Part::Neon, Glock) => ramp(u, 0.45, 0.8),
        (Part::Neon, _) => 0.0,
        (Part::Swell, Organ) => ramp(u, -0.05, 0.3),
        (Part::Swell, Guitar) => ramp(u, 0.45, 0.8),
        (Part::Swell, Bells) => 0.0,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Bells) => 0.0,
        (Part::Fade, Bass) => 1.0 - ramp(u, 0.6, 0.8),
        (Part::Fade, Glock) => 1.0 - ramp(u, 0.4, 0.6),
        (Part::Fade, Organ) => 1.0 - ramp(u, 0.2, 0.4),
        (Part::Fade, Guitar) => 1.0 - ramp(u, 0.0, 0.2),
    }
}

/// How strongly everything plays: rising section by section to the peak, so the build is
/// all accumulation, and falling away through the Fade.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.3 + 0.05 * u,
        Part::Neon => 0.35 + 0.2 * u,
        Part::Swell => 0.55 + 0.2 * u,
        Part::NightShift => 0.75 + 0.25 * (u / 0.7).min(1.0),
        Part::Fade => 0.95 - 0.65 * u,
    }
}

struct Writer {
    notes: Vec<Vec<Note>>,
    ctrls: Vec<Vec<Ctrl>>,
    end: u32,
    touch: Rng,
}

impl Writer {
    fn note(&mut self, track: usize, start: u32, len: u32, key: i32, vel: f64) {
        if start >= self.end || len == 0 {
            return;
        }
        let vel = (vel + self.touch.range(-2, 2) as f64).round().clamp(1.0, 127.0) as u8;
        self.notes[track].push(Note { start, len: len.min(self.end - start), key: key.clamp(12, 120) as u8, vel });
    }

    fn cc(&mut self, track: usize, tick: u32, cc: u8, v: f64) {
        if tick < self.end {
            self.ctrls[track].push(Ctrl { tick, kind: CtrlKind::Cc(cc), value: v.round().clamp(0.0, 127.0) as i16 });
        }
    }
}

/// Orbit's ensemble, mixed: the felt piano close and a little left, the organ wide and
/// low behind, the glockenspiel and guitar either side, the bells high in the hall.
fn ensemble(p: &mut Project) -> [usize; 6] {
    let named = |role: Role, label: &str| alternatives(role).into_iter().find(|a| a.label == label).expect("a bundled sound");
    let s = Strip::default();
    let mut add = |name: &str, role: Role, alt: Alt, strip: Strip| {
        let i = p.tracks.len();
        let strip = Strip { gain_db: strip.gain_db + alt.trim_db, ensemble: false, ..strip };
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, alt.source, i) });
        i
    };
    [
        // (a grand with its top rolled off and a soft touch is a felt piano)
        add("Piano", Role::Keys, named(Role::Keys, "Grand Piano"), Strip { reverb: 0.5, pan: -0.1, gain_db: 4.0, high_cut: 2600.0, ..s }),
        add("Bass", Role::Bass, named(Role::Bass, "Finger Bass"), Strip { reverb: 0.15, gain_db: -3.0, high_cut: 1800.0, ..s }),
        add(
            "Glockenspiel",
            Role::Bells,
            named(Role::Bells, "Glockenspiel"),
            Strip { reverb: 0.55, echo: 0.2, pan: 0.35, gain_db: -5.0, high_cut: 9000.0, ..s },
        ),
        add(
            "Organ",
            Role::Keys,
            named(Role::Keys, "Drawbar Organ"),
            Strip { reverb: 0.55, width: 1.5, gain_db: -6.0, high_cut: 3200.0, ..s },
        ),
        add(
            "Guitar",
            Role::Pluck,
            named(Role::Pluck, "Steel Guitar"),
            Strip { reverb: 0.45, echo: 0.12, pan: -0.4, gain_db: -9.6, high_cut: 7000.0, ..s },
        ),
        add(
            "Tubular Bells",
            Role::Bells,
            named(Role::Bells, "Tubular Bells"),
            Strip { reverb: 0.65, pan: 0.15, width: 1.3, gain_db: -8.0, ..s },
        ),
    ]
}

/// Write Orbit into `p`: its ensemble, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = ensemble(p);
    // a concert hall, warm, with a little tape drift
    p.fx.space = Space::Hall;
    p.fx.size = 0.7;
    p.fx.tone = 0.38;
    p.fx.motion = 0.35;
    p.fx.predelay = 0.15;
    p.fx.width = 0.85;
    p.fx.duck = 0.0;
    p.fx.shimmer = 0.0;
    p.fx.ensemble = 0.0;
    p.fx.drift = 0.12;
    p.fx.detune = 0.03;
    p.fx.echo.feedback = 0.35;
    p.fx.echo.tone = 0.3;
    p.fx.echo.wow = 0.15;
    p.master_db = 9.0;

    let bar = p.bar_ticks();
    // the pulse is the quaver, whatever the metre
    let q = PPQ / 2;
    let n = (bar / q).max(1);
    let end = secs.last().map_or(bar, |s| s.2);
    let lp = Loops::new(r.seed, n);
    let gstarts = lp.group_starts();
    let mut pr = Rng::stream(r.seed, "orbit-harmony");
    let neon = *pr.pick(&NEON);
    let swell = *pr.pick(&SWELL);
    let peak = *pr.pick(&PEAK);
    let fade = *pr.pick(&FADE);
    let mut br = Rng::stream(r.seed, "orbit-bells");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "orbit-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let low = pitch(0, 57);
    let [piano, bass, glock, organ, guitar, bells] = tr;
    let mut organ_prev: Vec<i32> = Vec::new();
    let mut bell_prev: Option<(usize, i32)> = None;

    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let span = (zz - a).max(1) as f64;
        let held = last && nb >= 3;
        let played = if held { nb - 1 } else { nb };
        let prog: &[Chord] = match part {
            Part::Intro => &[I],
            Part::Neon => neon,
            Part::Swell => swell,
            Part::NightShift => peak,
            Part::Fade => fade,
        };
        let u_at = |t: u32| (t.saturating_sub(a)) as f64 / span;
        let vel = |base: f64, t: u32| base * (0.55 + 0.45 * intensity(part, u_at(t)));
        // the bells' melody: an arc over eight bars, on the chord's notes, repeated
        let slots_per_bar = gstarts.len() as u32;
        let phrase = (8 * slots_per_bar).max(1);
        let mut bell_open: Option<(u32, i32, f64)> = None;
        for k in 0..played {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let parity = (t_bar / bar) % 2;
            let chord = prog[(k / 2) as usize % prog.len()];
            let pcs = chord.pcs();
            // the piano's figure, pedalled through each group
            for f in lp.piano.iter().filter(|f| f.at / n == parity) {
                let t = t_bar + (f.at % n) * q;
                if t < zz {
                    w.note(piano, t, q * 8 / 5, low + f.rel, vel(50.0, t) + 9.0 * f.accent);
                }
            }
            for &g in &gstarts {
                let t = t_bar + g * q;
                if t < zz {
                    w.cc(piano, t, 64, 0.0);
                    w.cc(piano, t + q / 6, 64, 127.0);
                }
            }
            // the bass: the root on the bar, and the root or its fifth on the second group
            let broot = pitch(chord.root, 43);
            for (gi, &g) in gstarts.iter().enumerate() {
                let t = t_bar + g * q;
                let len = gstarts.get(gi + 1).map_or(n, |x| *x) - g;
                let u = u_at(t);
                if t < zz && presence(Layer::Bass, part, u) > 0.02 {
                    let key = if gi > 0 && parity == 1 { broot + 7 } else { broot };
                    w.note(bass, t, len * q - q / 4, key, vel(78.0, t) + if gi == 0 { 6.0 } else { 0.0 });
                }
            }
            // the glockenspiel: dotted crotchets, its cell looping across the two bars
            for pos in (0..2 * n).step_by(3).filter(|p| p / n == parity) {
                let t = t_bar + (pos % n) * q;
                let i = (pos / 3) as usize % lp.glock.len();
                if t < zz && lp.glock_rest != Some(i) && presence(Layer::Glock, part, u_at(t)) > 0.02 {
                    w.note(glock, t, 3 * q, low + lp.glock[i], vel(62.0, t) + if pos % n == 0 { 6.0 } else { 0.0 });
                }
            }
            // the organ: the chord held two bars, voiced close to the last
            if k % 2 == 0 && presence(Layer::Organ, part, u_at(t_bar + bar)) > 0.02 {
                let mut v: Vec<i32> = pcs.iter().map(|&pc| pitch(pc, 61)).collect();
                if !organ_prev.is_empty() {
                    v = pcs
                        .iter()
                        .map(|&pc| {
                            let c = pitch(pc, 61);
                            [c - 12, c, c + 12]
                                .into_iter()
                                .filter(|x| (52..=72).contains(x))
                                .min_by_key(|x| organ_prev.iter().map(|o| (o - x).abs()).min().unwrap_or(0))
                                .unwrap_or(c)
                        })
                        .collect();
                }
                v.sort_unstable();
                v.dedup();
                let len = (2 * bar).min(zz - t_bar).saturating_sub(q / 3);
                for &key in &v {
                    w.note(organ, t_bar, len, key, vel(66.0, t_bar));
                }
                organ_prev = v;
            }
            // the guitar's counter-line on this chord's notes
            let gl: Vec<i32> = (55..=76).filter(|p| pcs.contains(&(p - root).rem_euclid(12))).collect();
            for &(at, idx) in lp.guitar.iter().filter(|(at, _)| at / n == parity) {
                let t = t_bar + (at % n) * q;
                if t < zz && presence(Layer::Guitar, part, u_at(t)) > 0.02 && !gl.is_empty() {
                    w.note(guitar, t, 2 * q, gl[idx.min(gl.len() - 1)], vel(60.0, t));
                }
            }
            // the bells, at the peak: a slow melody, a note to a group, ringing on
            if part == Part::NightShift {
                for (gi, &g) in gstarts.iter().enumerate() {
                    let t = t_bar + g * q;
                    if t >= zz {
                        break;
                    }
                    let slot = (k * slots_per_bar + gi as u32) % phrase;
                    let x = slot as f64 / (phrase - 1).max(1) as f64;
                    let target = low as f64 + 9.0 + 11.0 * (std::f64::consts::PI * x.powf(0.8)).sin();
                    let cands = ladder(&pcs.iter().map(|&pc| (pc + root).rem_euclid(12)).collect::<Vec<_>>(), 60, 84);
                    let prev = bell_prev.map(|b| b.1);
                    let key = cands
                        .iter()
                        .copied()
                        .min_by_key(|&c| {
                            let leap = prev.map_or(0, |pv| (c - pv).abs());
                            ((c as f64 - target).abs() * 10.0) as i32 + if leap > 7 { 60 } else { 0 }
                        })
                        .unwrap_or(low + 12);
                    // some notes held through the next group, for a slower line
                    let tie = prev == Some(key) || (gi > 0 && br.chance(0.3) && bell_open.is_some());
                    let v = 84.0 + 28.0 * (u_at(t) / 0.7).min(1.0) + if gi == 0 { 4.0 } else { 0.0 };
                    if !tie {
                        if let Some((s, kk, vv)) = bell_open.take() {
                            w.note(bells, s, t - s + bar / 2, kk, vv);
                        }
                        bell_open = Some((t, key, v));
                        bell_prev = Some((slot as usize, key));
                    }
                }
            }
        }
        if let Some((s, kk, vv)) = bell_open.take() {
            // the last bell rings on into what follows
            w.note(bells, s, (zz - s) + bar, kk, vv);
        }
        // the last chord: the figure's notes gathered into a minor seventh, rolled and held
        if held {
            let t = a + played * bar;
            w.cc(piano, t, 64, 0.0);
            w.cc(piano, t + q / 6, 64, 127.0);
            let keys: Vec<i32> = LAST.pcs().iter().zip([45, 60, 52, 67]).map(|(&pc, at)| pitch(pc, at)).chain([pitch(0, 69)]).collect();
            for (i, &key) in keys.iter().enumerate() {
                let s = t + i as u32 * q / 2;
                w.note(piano, s, end.saturating_sub(s), key, 40.0 - 2.0 * i as f64);
            }
        }
        // each layer's expression, a step a quaver: fading in and out, never jumping
        for (ti, layer) in LAYERS.iter().enumerate() {
            let track = tr[ti];
            let mut last_v = -1.0;
            let mut t = a;
            while t < zz {
                let u = u_at(t);
                let v = (127.0 * presence(*layer, part, u) * (0.72 + 0.28 * intensity(part, u))).round();
                if v != last_v {
                    w.cc(track, t, 11, v);
                    last_v = v;
                }
                t += q;
            }
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
        compose(&Recipe { style: Style::Orbit, ..Recipe::random(seed) }, Style::Orbit.tempo(), (15, 8), bars)
    }

    fn first(p: &Project, name: &str) -> Option<u32> {
        p.tracks.iter().find(|t| t.name == name).and_then(|t| t.notes.iter().map(|n| n.start).min())
    }

    fn last_start(p: &Project, name: &str) -> Option<u32> {
        p.tracks.iter().find(|t| t.name == name).and_then(|t| t.notes.iter().map(|n| n.start).max())
    }

    /// The story: the figure alone, the layers arriving one at a time (bass, glockenspiel,
    /// organ, guitar), the bells only at the peak and the loudest notes there, the layers
    /// leaving in the order they came, the piano alone at the end on a minor seventh.
    #[test]
    fn orbit_builds_and_falls_away() {
        for seed in 0..12 {
            let p = piece(seed, 40);
            let r = p.recipe.unwrap();
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            let order = ["Piano", "Bass", "Glockenspiel", "Organ", "Guitar"];
            let starts: Vec<u32> = order.iter().map(|t| first(&p, t).expect(t)).collect();
            assert!(starts.windows(2).all(|w| w[0] < w[1]), "seed {seed}: {starts:?}");
            assert!(starts[1] >= b[1].0 && starts[3] >= b[2].0, "seed {seed}");
            let ends: Vec<u32> = order[1..].iter().rev().map(|t| last_start(&p, t).unwrap()).collect();
            assert!(ends.windows(2).all(|w| w[0] < w[1]), "seed {seed}: {ends:?}");
            let bells = p.tracks.iter().find(|t| t.name == "Tubular Bells").unwrap();
            assert!(bells.notes.iter().all(|n| n.start >= b[3].0 && n.start < b[3].1));
            let loudest = p.tracks.iter().flat_map(|t| &t.notes).map(|n| n.vel).max().unwrap();
            assert_eq!(bells.notes.iter().map(|n| n.vel).max(), Some(loudest), "seed {seed}");
            // the piano alone over the last bars
            let tail = b[4].0 + (b[4].1 - b[4].0) * 4 / 5;
            assert!(p.tracks.iter().filter(|t| t.name != "Piano").all(|t| t.notes.iter().all(|n| n.start < tail)), "seed {seed}");
            let end = p.end_tick();
            let held: Vec<u8> =
                p.tracks.iter().flat_map(|t| &t.notes).filter(|n| n.end() == end).map(|n| (n.key + 12 - r.root) % 12).collect();
            for pc in [0, 3, 7, 10] {
                assert!(held.contains(&pc), "seed {seed}: {held:?}");
            }
            assert!(p.tracks.iter().all(|t| !matches!(t.role, Some(Role::Drums | Role::City))));
        }
    }

    /// The figure loops: every two bars of the intro the same notes at the same places,
    /// a note on every quaver, the bar felt as seven and eight.
    #[test]
    fn the_figure_loops() {
        assert_eq!(groups(15), vec![7, 8]);
        for seed in 0..20 {
            let lp = Loops::new(seed, 15);
            assert_eq!(lp.piano.len(), 30);
            assert_eq!(lp.piano.iter().map(|f| f.at).collect::<Vec<_>>(), (0..30).collect::<Vec<_>>());
            let p = piece(seed, 40);
            let b = p.block_ticks();
            let bar = p.bar_ticks();
            let piano = p.tracks.iter().find(|t| t.name == "Piano").unwrap();
            let cycle = |c: u32| -> Vec<(u32, u8)> {
                piano
                    .notes
                    .iter()
                    .filter(|n| n.start >= c * 2 * bar && n.start < (c + 1) * 2 * bar)
                    .map(|n| (n.start % (2 * bar), n.key))
                    .collect()
            };
            assert!(b[0].1 >= 4 * bar);
            assert_eq!(cycle(0), cycle(1), "seed {seed}");
            assert_eq!(cycle(0), cycle(5), "seed {seed}");
        }
    }

    /// Any arrangement and metre: the length exactly, and sound in every section.
    #[test]
    fn any_arrangement() {
        for (meter, bars) in [((15, 8), 9), ((4, 4), 24), ((3, 4), 17), ((6, 8), 33), ((5, 4), 3), ((7, 8), 20)] {
            let p = compose(&Recipe { style: Style::Orbit, ..Recipe::random(7) }, 92.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
