//! Chamber (D25): a chamber-orchestra score with a dark, sci-fi noir twist. A small
//! orchestra (a solo violin, violas, cellos, a double bass, pizzicato, a clarinet or cor
//! anglais, a bassoon, a horn, a harp, a celesta and a piano) plays over a cold machine
//! underneath: a glass drone, and a sub that beats like a heart and, at the climax, like
//! an engine.
//!
//! Its harmony is noir and slightly uncanny: the minor tonic moving by chromatic mediants
//! (a minor chord a major third down, a major one a major third up), the Neapolitan, the
//! flat sixth, always turning on a dominant seventh, two bars a chord; the ending a minor
//! chord with a major seventh and a ninth, left hanging. The seed writes one eight-bar
//! theme from a motif: stated, sequenced on the next chord, raised to a climax, and
//! falling to the leading tone.
//!
//! The sections tell it: the Intro is the void, the drone, the heartbeat and the motif on
//! the celesta, a high violin harmonic; the Neon gives the theme to the clarinet over a
//! ticking pizzicato ostinato, held low strings and a harp; the Swell to the solo violin,
//! the violas against it, a bassoon walking beneath, horn swells and the celesta echoing;
//! the Night shift builds on tremolo strings, the engine pulse and low piano clusters to
//! the theme in three octaves of strings; and the Fade passes it from the clarinet to the
//! cellos before everything settles on the last chord. No drums, no city.

use dd_dsp::fx::Space;

use crate::compose::{Mode, Recipe, near};
use crate::palette::{Alt, Role, alternatives};
use crate::project::{Ctrl, CtrlKind, Note, Part, Project, Strip, Track};
use crate::rng::Rng;

/// The sounds added to the palette for Chamber: the other styles leave them out of their
/// choosing, so every recipe they had still sounds as it did.
pub(crate) const CHAMBER_SOUNDS: [&str; 1] = ["Violin"];

/// The usual arrangement: a short void, a statement each for the clarinet and the violin,
/// a build and its climax twice that, and a fade long enough to hold the last chord,
/// shared in proportion to the length.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 20 { [4, 8, 8, 16, 10] } else { [2, 4, 4, 6, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Chord {
    root: i32,
    ivs: &'static [i32],
}

const MIN: &[i32] = &[0, 3, 7];
const MAJ: &[i32] = &[0, 4, 7];
const MAJ7: &[i32] = &[0, 4, 7, 11];
const MINMAJ7: &[i32] = &[0, 3, 7, 11];
const DOM7: &[i32] = &[0, 4, 7, 10];

const fn ch(root: i32, ivs: &'static [i32]) -> Chord {
    Chord { root, ivs }
}

/// Eight bars, two a chord, always turning on the dominant seventh: the flat sixth and
/// the fourth; the Neapolitan; a minor chord a major third down (the uncanny one); the
/// tonic with its major seventh; a major chord a major third up; a falling line.
const PROGS: [[Chord; 4]; 6] = [
    [ch(0, MIN), ch(8, MAJ), ch(5, MIN), ch(7, DOM7)],
    [ch(0, MIN), ch(8, MAJ7), ch(1, MAJ), ch(7, DOM7)],
    [ch(0, MIN), ch(8, MIN), ch(0, MIN), ch(7, DOM7)],
    [ch(0, MINMAJ7), ch(5, MIN), ch(8, MAJ), ch(7, DOM7)],
    [ch(0, MIN), ch(4, MAJ), ch(5, MIN), ch(7, DOM7)],
    [ch(0, MIN), ch(10, MAJ), ch(8, MAJ), ch(7, DOM7)],
];

/// The last chord: the minor tonic with its major seventh and ninth.
const FINAL: Chord = ch(0, &[0, 3, 7, 11, 14]);

impl Chord {
    /// Its pitch classes above the tonic.
    fn pcs(self) -> Vec<i32> {
        self.ivs.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }

    /// The scale a melody uses over it: the mode's, each note a semitone from one of
    /// the chord's giving way to it (never the tonic or fifth, if another will do).
    fn scale(self, steps: [u8; 7]) -> Vec<i32> {
        let tones = self.pcs();
        let mut s: Vec<i32> = steps.iter().map(|&x| x as i32).collect();
        for &t in &tones {
            if s.contains(&t) {
                continue;
            }
            let near = |x: i32| (x - t).rem_euclid(12) == 1 || (t - x).rem_euclid(12) == 1;
            match s.iter().copied().filter(|&x| near(x) && !tones.contains(&x)).min_by_key(|&x| (x == 0 || x == 7, x)) {
                Some(x) => {
                    let i = s.iter().position(|&y| y == x).unwrap_or(0);
                    s[i] = t;
                }
                None => s.push(t),
            }
        }
        s.sort_unstable();
        s.dedup();
        s
    }
}

/// Every pitch (above the tonic, in semitones) of these pitch classes over a wide range.
fn ladder(pcs: &[i32]) -> Vec<i32> {
    let mut l: Vec<i32> = (-3..=3).flat_map(|o| pcs.iter().map(move |pc| pc + 12 * o)).collect();
    l.sort_unstable();
    l.dedup();
    l
}

/// The pitch of these pitch classes nearest `p` (the lower on a tie).
fn snap(p: i32, pcs: &[i32]) -> i32 {
    ladder(pcs).into_iter().min_by_key(|&x| ((x - p).abs(), x)).unwrap_or(p)
}

/// `n` steps along the scale from `p`.
fn walk(p: i32, n: i32, scale: &[i32]) -> i32 {
    let l = ladder(scale);
    let at = snap(p, scale);
    let i = l.iter().position(|&x| x == at).unwrap_or(0) as i32;
    l[(i + n).clamp(0, l.len() as i32 - 1) as usize]
}

/// A note of the theme: when in its bar and for how long, in beats, and its pitch in
/// semitones above the tonic.
#[derive(Clone, Copy, Debug)]
struct TNote {
    at: f64,
    len: f64,
    rel: i32,
}

/// A bar's rhythm in beats adding up to `n`: busy (the motif) or long (its answer).
fn rhythm(rng: &mut Rng, n: u32, busy: bool) -> Vec<f64> {
    let table: &[&[f64]] = match (n, busy) {
        (1, _) => &[&[1.0]],
        (2, true) => &[&[1.0, 0.5, 0.5], &[0.5, 0.5, 1.0], &[1.5, 0.5]],
        (2, false) => &[&[2.0]],
        (3, true) => &[&[1.5, 0.5, 1.0], &[1.0, 0.5, 0.5, 1.0], &[0.5, 0.5, 2.0]],
        (3, false) => &[&[3.0], &[2.0, 1.0]],
        (4, true) => &[&[1.5, 0.5, 1.0, 1.0], &[1.0, 0.5, 0.5, 2.0], &[0.5, 0.5, 1.0, 2.0], &[2.0, 1.0, 1.0], &[1.0, 1.0, 0.5, 0.5, 1.0]],
        (4, false) => &[&[3.0, 1.0], &[4.0], &[2.0, 2.0], &[1.0, 3.0]],
        (6, true) => &[&[1.5, 0.5, 1.0, 3.0], &[1.0, 0.5, 0.5, 1.0, 2.0, 1.0], &[2.0, 1.0, 2.0, 1.0]],
        (6, false) => &[&[6.0], &[3.0, 3.0], &[4.0, 2.0]],
        _ => {
            // longer bars in groups of four, three and two, the first carrying the motif
            let mut out = Vec::new();
            let mut rem = n;
            let mut first = true;
            while rem > 0 {
                let k = if rem <= 4 {
                    rem
                } else if rem == 5 || rem == 7 {
                    3
                } else {
                    4
                };
                out.extend(rhythm(rng, k, busy && first));
                first = false;
                rem -= k;
            }
            return out;
        }
    };
    let w: Vec<f64> = (0..table.len()).map(|i| if i == 0 { 3.0 } else { 2.0 }).collect();
    table[rng.weighted(&w)].to_vec()
}

/// The theme: eight bars of notes over a progression, two bars a chord.
struct Theme {
    bars: Vec<Vec<TNote>>,
    prog: [Chord; 4],
}

impl Theme {
    fn new(seed: u64, bpb: u32, mode: Mode) -> Theme {
        let mut rng = Rng::stream(seed, "chamber-theme");
        let steps = mode.steps();
        let prog = if mode == Mode::Phrygian && rng.chance(0.6) { PROGS[1] } else { PROGS[rng.range(0, 5) as usize] };
        let motif = rhythm(&mut rng, bpb, true);
        let rhythms = [
            motif.clone(),
            rhythm(&mut rng, bpb, false),
            // the sequence keeps the motif's rhythm
            motif.clone(),
            rhythm(&mut rng, bpb, false),
            if rng.chance(0.5) { motif.clone() } else { rhythm(&mut rng, bpb, true) },
            rhythm(&mut rng, bpb, false),
            rhythm(&mut rng, bpb, true),
            vec![bpb as f64],
        ];
        // the motif's shape in scale steps: a leap, then stepping back against it
        let leap: i32 = *rng.pick(&[-2, 2, 3, -3, 4, -4]);
        let mut contour = vec![leap];
        while contour.len() + 1 < motif.len() {
            let back = -leap.signum();
            contour.push(if rng.chance(0.75) { back } else { -back });
        }
        let invert = rng.chance(0.35);
        let start = *rng.pick(&[7, 3, 12]);
        let tones = |b: usize| prog[b / 2].pcs();
        let scale = |b: usize| prog[b / 2].scale(steps);
        let state = |rel: i32| {
            let mut x = rel;
            while x > 14 {
                x -= 12;
            }
            while x < -2 {
                x += 12;
            }
            x
        };
        let mut pitches: Vec<Vec<i32>> = Vec::new();
        // a motif bar from `from` (moved by octaves to within a sixth of `prev`), along
        // the contour (upside down, when `inv`)
        let motif_bar = |from: i32, b: usize, inv: bool, prev: Option<i32>| -> Vec<i32> {
            let mut p = snap(state(from), &tones(b));
            if let Some(q) = prev {
                while p - q > 9 {
                    p -= 12;
                }
                while q - p > 9 {
                    p += 12;
                }
            }
            let mut out = vec![p];
            for &c in contour.iter() {
                p = walk(p, if inv { -c } else { c }, &scale(b));
                out.push(p);
            }
            out
        };
        // an answer: a chord tone near `target`, then stepping down
        let answer = |target: i32, b: usize, n: usize| -> Vec<i32> {
            let mut p = snap(target, &tones(b));
            let mut out = vec![p];
            while out.len() < n {
                p = walk(p, -1, &scale(b));
                out.push(p);
            }
            out
        };
        let m0 = motif_bar(start, 0, false, None);
        let last = |v: &Vec<i32>| *v.last().unwrap_or(&0);
        pitches.push(m0.clone());
        pitches.push(answer(last(&m0) - leap.signum() * 2, 1, rhythms[1].len()));
        let m2 = motif_bar(m0[0] + *rng.pick(&[2, 3, 5]), 2, false, Some(last(&pitches[1])));
        pitches.push(m2.clone());
        pitches.push(answer(last(&m2) - leap.signum() * 2, 3, rhythms[3].len()));
        // higher, the climax
        let m4 = {
            // a leap up from where the answer left off
            let mut from = last(&pitches[3]) + *rng.pick(&[5, 7, 9]);
            while from > 16 {
                from -= 12;
            }
            let mut p = snap(from, &tones(4));
            let mut out = vec![p];
            for (j, &c) in contour.iter().enumerate() {
                p = walk(p, if invert { -c } else { c }, &scale(4));
                out.push(p);
                if out.len() >= rhythms[4].len() || j + 2 > rhythms[4].len() {
                    break;
                }
            }
            while out.len() < rhythms[4].len() {
                p = walk(p, 1, &scale(4));
                out.push(p);
            }
            out.truncate(rhythms[4].len());
            out
        };
        let top = m4.iter().copied().max().unwrap_or(12);
        pitches.push(m4);
        pitches.push(answer(top + 1, 5, rhythms[5].len()));
        // falling on the dominant, by step, to one of its notes
        let mut p = last(&pitches[5]);
        let mut fall = Vec::new();
        for j in 0..rhythms[6].len() {
            p = walk(p, -1, &scale(6));
            if j + 1 == rhythms[6].len() {
                p = snap(p, &tones(6));
            }
            fall.push(p);
        }
        pitches.push(fall);
        // the leading tone (or the dominant's fifth) held, to turn home
        let lead = [11, -1, 14, 2].into_iter().min_by_key(|x| ((x - (p - 1)).abs(), *x)).unwrap_or(-1);
        pitches.push(vec![lead]);
        // no bar leaps more than a sixth from the last: an octave nearer if it would
        for i in 1..pitches.len() {
            let q = last(&pitches[i - 1]);
            let first = pitches[i][0];
            let shift = if first - q > 9 {
                -12
            } else if q - first > 9 {
                12
            } else {
                0
            };
            for x in pitches[i].iter_mut() {
                *x += shift;
            }
        }
        let bars = rhythms
            .iter()
            .zip(&pitches)
            .map(|(r, ps)| {
                let mut at = 0.0;
                r.iter()
                    .zip(ps.iter().chain(std::iter::repeat(ps.last().unwrap_or(&0))))
                    .map(|(&len, &rel)| {
                        // (kept in range by octaves, so every note keeps its name)
                        let mut rel = rel;
                        while rel > 21 {
                            rel -= 12;
                        }
                        while rel < -7 {
                            rel += 12;
                        }
                        let n = TNote { at, len, rel };
                        at += len;
                        n
                    })
                    .collect()
            })
            .collect();
        Theme { bars, prog }
    }

    fn chord(&self, tb: usize) -> Chord {
        self.prog[(tb % 8) / 2]
    }
}

/// What a bar of a section plays.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Bar {
    /// Bar `tb` of the theme.
    Theme(usize),
    /// The last chord, held.
    Hold,
}

/// The bars of a section of `n` bars: the theme's bars, lined up so the section ends on
/// its turn to the dominant (the opening one, and an Intro, start it instead), and in
/// the last section a bar or two after for the last chord.
fn layout(part: Part, n: u32, opening: bool, last: bool) -> Vec<Bar> {
    let hold = match (last, n) {
        (true, 8..) => 2,
        (true, 4..) => 1,
        _ => 0,
    };
    let m = n - hold;
    let off = if opening || part == Part::Intro { 0 } else { (8 - m % 8) % 8 };
    (0..m).map(|k| Bar::Theme(((k + off) % 8) as usize)).chain((0..hold).map(|_| Bar::Hold)).collect()
}

struct Writer {
    notes: Vec<Vec<Note>>,
    ctrls: Vec<Vec<Ctrl>>,
    end: u32,
    beat: u32,
    touch: Rng,
}

impl Writer {
    fn note(&mut self, track: usize, start: u32, len: u32, key: i32, vel: f64) {
        if start >= self.end || len == 0 {
            return;
        }
        let vel = (vel + self.touch.range(-3, 3) as f64).round().clamp(1.0, 127.0) as u8;
        self.notes[track].push(Note { start, len: len.min(self.end - start), key: key.clamp(12, 120) as u8, vel });
    }

    fn cc(&mut self, track: usize, tick: u32, v: f64) {
        if tick < self.end {
            self.ctrls[track].push(Ctrl { tick, kind: CtrlKind::Cc(11), value: v.round().clamp(0.0, 127.0) as i16 });
        }
    }

    /// The expression gliding from `from` to `to` over `len` ticks, a step every half beat.
    fn glide(&mut self, track: usize, a: u32, len: u32, from: f64, to: f64) {
        let step = (self.beat / 2).max(1);
        let n = (len / step).max(1);
        for i in 0..=n {
            let tick = a + i * step;
            if i == n && tick > a + len {
                break;
            }
            self.cc(track, tick, from + (to - from) * i as f64 / n as f64);
        }
    }
}

/// Chamber's tracks, in the order an orchestra's score lists them.
struct Tracks {
    celesta: usize,
    lead: usize,
    bassoon: usize,
    horn: usize,
    harp: usize,
    piano: usize,
    violin: usize,
    violas: usize,
    cellos: usize,
    double_bass: usize,
    pizzicato: usize,
    tremolo: usize,
    glass: usize,
    pulse: usize,
}

/// Its orchestra, mixed: seated as on a stage (violin left, low strings right), a
/// natural hall, the celesta and glass trailing echoes, the sub dry and round.
fn orchestra(p: &mut Project, r: &Recipe) -> Tracks {
    let mut sr = Rng::stream(r.seed, "chamber-sounds");
    let named = |role: Role, label: &str| alternatives(role).into_iter().find(|a| a.label == label).expect("a bundled sound");
    let lead = if sr.chance(0.7) { "Clarinet" } else { "Cor Anglais" };
    let harp = if sr.chance(0.6) { "Concert Harp (FreePats)" } else { "Harp" };
    let glass = ["Glass Drone", "Bowed Glass"][sr.weighted(&[2.0, 1.0])];
    let bells = ["Celesta", "Celesta Glow"][sr.weighted(&[3.0, 1.0])];
    p.fx.space = [Space::Hall, Space::Noir][sr.weighted(&[3.0, 2.0])];
    let s = Strip::default();
    let mut add = |name: &str, role: Role, alt: Alt, strip: Strip| {
        let i = p.tracks.len();
        let strip = Strip { gain_db: strip.gain_db + alt.trim_db, ensemble: false, ..strip };
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, alt.source, i) });
        i
    };
    Tracks {
        celesta: add("Celesta", Role::Bells, named(Role::Bells, bells), Strip { reverb: 0.6, echo: 0.4, pan: -0.3, gain_db: 1.0, ..s }),
        lead: add(lead, Role::Lead, named(Role::Lead, lead), Strip { reverb: 0.45, echo: 0.12, pan: -0.1, gain_db: -1.0, ..s }),
        bassoon: add("Bassoon", Role::Lead, named(Role::Lead, "Bassoon"), Strip { reverb: 0.4, pan: 0.15, gain_db: -3.0, ..s }),
        horn: add("Horn", Role::Lead, named(Role::Lead, "French Horn"), Strip { reverb: 0.6, pan: 0.25, gain_db: -4.0, ..s }),
        harp: add(
            "Harp",
            Role::Pluck,
            named(Role::Pluck, harp),
            Strip { reverb: 0.5, echo: 0.1, pan: -0.45, width: 1.3, gain_db: 1.0, ..s },
        ),
        piano: add(
            "Piano",
            Role::Keys,
            named(Role::Keys, "Grand Piano"),
            Strip { reverb: 0.55, pan: 0.05, gain_db: 0.0, high_cut: 4000.0, ..s },
        ),
        violin: add("Violin", Role::Strings, named(Role::Strings, "Violin"), Strip { reverb: 0.45, pan: -0.35, gain_db: -1.0, ..s }),
        violas: add(
            "Violas",
            Role::Strings,
            named(Role::Strings, "Viola"),
            Strip { reverb: 0.45, pan: 0.1, width: 1.2, gain_db: -3.0, ..s },
        ),
        cellos: add(
            "Cellos",
            Role::Strings,
            named(Role::Strings, "Cello"),
            Strip { reverb: 0.45, pan: 0.3, width: 1.2, gain_db: -2.0, ..s },
        ),
        double_bass: add(
            "Double Bass",
            Role::Bass,
            named(Role::Bass, "Contrabass"),
            Strip { reverb: 0.35, pan: 0.4, gain_db: 0.0, high_cut: 3000.0, ..s },
        ),
        pizzicato: add(
            "Pizzicato",
            Role::Pluck,
            named(Role::Pluck, "Pizzicato Strings"),
            Strip { reverb: 0.4, echo: 0.15, pan: -0.15, width: 1.3, gain_db: 1.0, ..s },
        ),
        tremolo: add(
            "Tremolo Strings",
            Role::Strings,
            named(Role::Strings, "Tremolo Strings"),
            Strip { reverb: 0.55, width: 1.6, gain_db: -5.0, ..s },
        ),
        glass: add(
            "Glass",
            Role::Drone,
            named(Role::Drone, glass),
            Strip { reverb: 0.75, echo: 0.3, width: 1.8, gain_db: -11.0, high_cut: 9000.0, ..s },
        ),
        pulse: add("Pulse", Role::Bass, named(Role::Bass, "Pure Sub"), Strip { reverb: 0.05, gain_db: -1.0, high_cut: 300.0, ..s }),
    }
}

/// Write Chamber into `p`: its orchestra, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], density: f64) {
    p.tracks.clear();
    let t = orchestra(p, r);
    p.fx.ensemble = 0.0;
    p.fx.shimmer = 0.0;
    p.fx.size = 0.66;
    p.fx.tone = 0.45;
    p.fx.predelay = 0.18;
    p.fx.width = 0.9;
    p.fx.duck = 0.0;
    p.fx.echo.feedback = 0.45;
    p.fx.echo.wow = 0.12;
    p.master_db = 4.0;

    let (beat, bar) = (p.beat_ticks(), p.bar_ticks());
    let bpb = p.meter.0.max(1);
    let end = secs.last().map_or(bar, |s| s.2);
    let theme = Theme::new(r.seed, bpb, r.mode);
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        beat,
        touch: Rng::stream(r.seed, "chamber-touch"),
    };
    let mut orng = Rng::stream(r.seed, "chamber-ostinato");
    let ostinato: [usize; 4] = *orng.pick(&[[0, 1, 2, 1], [0, 2, 1, 3], [0, 1, 3, 1], [0, 3, 2, 1], [0, 1, 2, 4]]);
    let tonic = |target: i32| near(r.root % 12, target) as i32;
    // a chord's root, and its notes, nearest `target`
    let root_key = |c: Chord, target: i32| near(((r.root as i32 + c.root).rem_euclid(12)) as u8, target) as i32;
    let tones = |c: Chord, centre: i32| -> Vec<i32> {
        let mut v: Vec<i32> = c.pcs().into_iter().map(|pc| near(((r.root as i32 + pc).rem_euclid(12)) as u8, centre) as i32).collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let (violin_t, lead_t, viola_t, cello_t, horn_t, celesta_t) = (tonic(74), tonic(64), tonic(62), tonic(50), tonic(55), tonic(84));
    // the theme's bar `tb` on a track, from `t0`
    let melody = |w: &mut Writer, track: usize, tonic: i32, tb: usize, t0: u32, vel: f64, ring_to: Option<u32>| {
        let notes = &theme.bars[tb];
        for (j, n) in notes.iter().enumerate() {
            let s = t0 + (n.at * beat as f64).round() as u32;
            let mut len = (n.len * beat as f64).round() as u32 + beat / 10;
            if let Some(z) = ring_to.filter(|_| j + 1 == notes.len()) {
                len = z.saturating_sub(s);
            }
            let rise = 6.0 * (n.rel as f64 / 12.0) + if j == 0 { 3.0 } else { 0.0 };
            w.note(track, s, len, tonic + n.rel, vel + rise);
        }
    };

    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let n = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let bars = layout(part, n, si == 0, last);
        let themed = bars.iter().filter(|b| matches!(b, Bar::Theme(_))).count();
        let build = if part == Part::NightShift { if themed >= 12 { themed - 8 } else { themed / 2 } } else { 0 };
        let zz = if last { end } else { z };
        // every track starts the section at its usual expression
        for tr in 0..p.tracks.len() {
            w.cc(tr, a, 100.0);
        }
        // the glass under it all: the tonic and fifth, breathing with the section
        let (gv, g0, g1) = match part {
            Part::Intro => (60.0, 30.0, 85.0),
            Part::Neon => (46.0, 70.0, 75.0),
            Part::Swell => (50.0, 75.0, 85.0),
            Part::NightShift => (70.0, 80.0, 115.0),
            Part::Fade => (48.0, 80.0, 40.0),
        };
        for k in [tonic(50), tonic(50) + 7] {
            w.note(t.glass, a, zz - a, k, gv);
        }
        w.glide(t.glass, a, zz - a, g0, g1);
        let mut counter = viola_t + 3;
        for (k, &what) in bars.iter().enumerate() {
            let t0 = a + k as u32 * bar;
            if t0 >= z {
                break;
            }
            let final_bar = last && k + 1 == bars.len();
            let blen = if final_bar { end - t0 } else { bar.min(z - t0) };
            let x = k as f64 / bars.len().max(1) as f64;
            let tb = match what {
                Bar::Theme(tb) => tb,
                Bar::Hold => {
                    if k > 0 && bars[k - 1] == Bar::Hold {
                        continue;
                    }
                    hold(&mut w, &t, &theme, t0, end, beat, bpb, celesta_t, &tones, &|tg| tonic(tg));
                    continue;
                }
            };
            let c = theme.chord(tb);
            // a chord's first bar here: it lasts its two bars where the next is its second
            let starts = k == 0 || tb % 2 == 0;
            let span = {
                let two = tb % 2 == 0 && bars.get(k + 1) == Some(&Bar::Theme(tb + 1));
                let s = if two { 2 * bar } else { bar };
                let s = s.min(z - t0);
                if last && (k + 1 == bars.len() || (two && k + 2 == bars.len())) { end - t0 } else { s }
            };
            let next = bars.get(k + 1).map(|b| match b {
                Bar::Theme(n) => theme.chord(*n),
                Bar::Hold => FINAL,
            });
            let ring = final_bar.then_some(end);
            let bass = root_key(c, 36);
            let sub = root_key(c, 31).max(24);
            match part {
                // the void: the heartbeat, the motif on the celesta, a far harmonic
                Part::Intro => {
                    heart(&mut w, t.pulse, t0, blen, beat, sub, 58.0);
                    if tb % 2 == 0 {
                        melody(&mut w, t.celesta, celesta_t, tb, t0, 50.0, ring);
                    }
                    if starts {
                        w.note(t.cellos, t0, span, root_key(c, 43), 36.0);
                        w.glide(t.cellos, t0, span, 50.0, 70.0);
                    }
                    if k == 0 {
                        w.note(t.violin, a, z - a, tonic(86) + 7, 30.0);
                        w.glide(t.violin, a, z - a, 20.0, 70.0);
                    }
                }
                // the clarinet's theme over the ticking pizzicato and held low strings
                Part::Neon => {
                    melody(&mut w, t.lead, lead_t, tb, t0, 66.0, ring);
                    heart(&mut w, t.pulse, t0, blen, beat, sub, 54.0);
                    pizz(&mut w, t.pizzicato, t0, blen, beat, &ostinato, root_key(c, 48), c, 44.0, density >= 0.35);
                    if starts {
                        let v = tones(c, 55);
                        w.note(t.cellos, t0, span, root_key(c, 43), 42.0);
                        for &kk in &v {
                            w.note(t.violas, t0, span, kk, 38.0);
                        }
                        w.glide(t.violas, t0, span, 55.0, 62.0);
                        w.glide(t.cellos, t0, span, 60.0, 66.0);
                        w.note(t.double_bass, t0, span, bass, 46.0);
                        if density > 0.4 || tb == 0 {
                            roll(&mut w, t.harp, t0, beat, &tones(c, 55), &tones(c, 67), 44.0, false);
                        }
                    }
                }
                // the violin's lament: the violas against it, the bassoon below, horns
                // swelling, the celesta echoing the motif's tail
                Part::Swell => {
                    melody(&mut w, t.violin, violin_t, tb, t0, 70.0, ring);
                    w.glide(t.violin, t0, blen, 85.0 + 15.0 * x, 90.0 + 15.0 * x);
                    heart(&mut w, t.pulse, t0, blen, beat, sub, 60.0);
                    if density >= 0.3 {
                        pizz(&mut w, t.pizzicato, t0, blen, beat, &ostinato, root_key(c, 48), c, 40.0, density >= 0.5);
                    }
                    // the violas: a chord tone a half bar, moving against the theme
                    let notes = &theme.bars[tb];
                    let up = notes.last().map_or(0, |n| n.rel) >= notes[0].rel;
                    let halves = if bpb.is_multiple_of(2) && bpb >= 4 { 2 } else { 1 };
                    for h in 0..halves {
                        let s = t0 + h * blen / halves;
                        let len = if h + 1 == halves { blen - h * blen / halves } else { blen / halves };
                        let cands: Vec<i32> = tones(c, counter)
                            .into_iter()
                            .chain(tones(c, counter + 6))
                            .filter(|&kk| if up { kk <= counter } else { kk >= counter })
                            .collect();
                        let pick = cands.into_iter().min_by_key(|&kk| ((kk - counter).abs() - 2).abs()).unwrap_or(counter);
                        let pick = pick.clamp(viola_t - 7, viola_t + 9);
                        w.note(t.violas, s, len + beat / 8, pick, 54.0);
                        counter = pick;
                    }
                    if starts {
                        w.note(t.cellos, t0, span, root_key(c, 48), 44.0);
                        w.note(t.cellos, t0, span, root_key(c, 48) + c.ivs[2], 40.0);
                        w.note(t.double_bass, t0, span, bass, 50.0);
                        horn(&mut w, t.horn, t0, span, &tones(c, horn_t), root_key(c, horn_t), 60.0, 40.0, 92.0);
                        if tb % 4 == 0 {
                            strike(&mut w, t.piano, t0, span, root_key(c, 31), 52.0, false);
                        }
                    }
                    bassoon(
                        &mut w,
                        t.bassoon,
                        t0,
                        blen,
                        bpb,
                        beat,
                        root_key(c, 43),
                        c,
                        next.filter(|_| tb % 2 == 1),
                        &|cc| root_key(cc, 43),
                        56.0,
                    );
                    // the celesta echoes the motif's last notes in the answering bars
                    if tb % 2 == 1 && bpb >= 2 {
                        let prev = &theme.bars[tb - 1];
                        for (j, nn) in prev.iter().rev().take(3).rev().enumerate() {
                            w.note(t.celesta, t0 + beat + j as u32 * beat / 2, beat / 2, celesta_t + nn.rel, 44.0);
                        }
                    }
                }
                Part::NightShift if k < build => {
                    // the build: tremolo strings layering, the engine, low clusters, the
                    // horn calling the motif
                    let bx = k as f64 / build.max(1) as f64;
                    let nx = (k + 1) as f64 / build.max(1) as f64;
                    let v = tones(c, 62);
                    let layers = 2 + (bx * 3.0) as usize;
                    let mut v: Vec<i32> = v.iter().rev().take(layers.min(v.len())).copied().collect();
                    if bx > 0.5 {
                        v.push(v.iter().max().copied().unwrap_or(67) + 12);
                    }
                    for &kk in &v {
                        w.note(t.tremolo, t0, blen, kk, 50.0 + 35.0 * bx);
                    }
                    w.glide(t.tremolo, t0, blen, 45.0 + 60.0 * bx, 45.0 + 60.0 * nx);
                    engine(&mut w, t.pulse, t0, blen, beat, sub, 52.0 + 28.0 * bx);
                    for b in 0..bpb {
                        let s = t0 + b * beat;
                        if s >= t0 + blen {
                            break;
                        }
                        let accent = if b == 0 { 6.0 } else { 0.0 };
                        w.note(t.cellos, s, beat * 4 / 5, root_key(c, 48), 50.0 + 30.0 * bx + accent);
                        w.note(t.double_bass, s, beat * 4 / 5, bass, 52.0 + 30.0 * bx + accent);
                    }
                    w.glide(t.cellos, t0, blen, 70.0 + 30.0 * bx, 70.0 + 30.0 * nx);
                    pizz(&mut w, t.pizzicato, t0, blen, beat, &ostinato, root_key(c, 48), c, 48.0 + 20.0 * bx, true);
                    strike(&mut w, t.piano, t0, blen, root_key(c, 31), 46.0 + 34.0 * bx, true);
                    if tb % 2 == 0 {
                        melody(&mut w, t.horn, horn_t, tb, t0, 58.0 + 26.0 * bx, None);
                    }
                }
                // the climax: the theme in three octaves of strings, horns and engine
                Part::NightShift => {
                    melody(&mut w, t.violin, violin_t, tb, t0, 100.0, ring);
                    melody(&mut w, t.violas, viola_t, tb, t0, 92.0, ring);
                    melody(&mut w, t.cellos, cello_t, tb, t0, 92.0, ring);
                    for tr in [t.violin, t.violas, t.cellos] {
                        w.glide(tr, t0, blen, 108.0, 112.0);
                    }
                    engine(&mut w, t.pulse, t0, blen, beat, sub, 82.0);
                    for &kk in &tones(c, 64) {
                        w.note(t.tremolo, t0, blen, kk, 80.0);
                    }
                    w.glide(t.tremolo, t0, blen, 100.0, 104.0);
                    for h in 0..2 {
                        let s = t0 + h * blen / 2;
                        w.note(t.double_bass, s, blen / 2 * 19 / 20, bass, 82.0 - 6.0 * h as f64);
                    }
                    bassoon(
                        &mut w,
                        t.bassoon,
                        t0,
                        blen,
                        bpb,
                        beat,
                        root_key(c, 43),
                        c,
                        next.filter(|_| tb % 2 == 1),
                        &|cc| root_key(cc, 43),
                        78.0,
                    );
                    if starts {
                        horn(&mut w, t.horn, t0, span, &tones(c, horn_t), root_key(c, horn_t), 84.0, 80.0, 115.0);
                        strike(&mut w, t.piano, t0, span, root_key(c, 31), 84.0, false);
                        roll(&mut w, t.harp, t0, beat, &tones(c, 55), &tones(c, 67), 70.0, true);
                    }
                }
                // the theme passes from the clarinet to the cellos, everything thinning
                Part::Fade => {
                    let fade = 1.0 - x;
                    if tb < 4 {
                        melody(&mut w, t.lead, lead_t, tb, t0, 58.0, ring);
                    } else {
                        melody(&mut w, t.cellos, cello_t, tb, t0, 60.0, ring);
                    }
                    if k % 2 == 0 {
                        w.note(t.pulse, t0, beat / 4, sub, 30.0 + 26.0 * fade);
                    }
                    if starts {
                        for &kk in &tones(c, 58) {
                            w.note(t.violas, t0, span, kk, 36.0);
                        }
                        w.glide(t.violas, t0, span, 40.0 + 40.0 * fade, 40.0 + 30.0 * fade);
                        w.note(t.double_bass, t0, span, bass, 42.0);
                        if k == 0 {
                            roll(&mut w, t.harp, t0, beat, &tones(c, 55), &tones(c, 67), 40.0, false);
                        }
                    }
                }
            }
        }
    }
    for (i, tr) in p.tracks.iter_mut().enumerate() {
        tr.notes.append(&mut w.notes[i]);
        tr.ctrls.append(&mut w.ctrls[i]);
        tr.ctrls.retain(|c| c.tick < end);
    }
}

/// The last chord, from `t0` to the end: the minor tonic with its major seventh in the
/// strings, a ninth high on the violin, the glass under it, a harp roll, one low strike
/// on the piano, the clarinet home on the tonic, the motif once more on the celesta, and
/// one last heartbeat.
#[allow(clippy::too_many_arguments)]
fn hold(
    w: &mut Writer,
    t: &Tracks,
    theme: &Theme,
    t0: u32,
    end: u32,
    beat: u32,
    bpb: u32,
    celesta_t: i32,
    tones: &dyn Fn(Chord, i32) -> Vec<i32>,
    tonic: &dyn Fn(i32) -> i32,
) {
    let span = end.saturating_sub(t0);
    let minmaj = Chord { root: 0, ivs: MINMAJ7 };
    for &k in &tones(minmaj, 60) {
        w.note(t.violas, t0, span, k, 40.0);
    }
    w.note(t.cellos, t0, span, tonic(48), 42.0);
    w.note(t.cellos, t0, span, tonic(48) + 7, 38.0);
    w.note(t.double_bass, t0, span, tonic(36), 42.0);
    w.note(t.violin, t0 + beat, span.saturating_sub(beat), tonic(74) + 14, 34.0);
    w.note(t.lead, t0, span, tonic(64), 46.0);
    for tr in [t.violas, t.cellos, t.double_bass, t.violin, t.lead] {
        w.glide(tr, t0, span, 80.0, 16.0);
    }
    w.note(t.pulse, t0, beat / 4, tonic(31).max(24), 40.0);
    strike(w, t.piano, t0, span, tonic(31), 44.0, false);
    roll(w, t.harp, t0, beat, &tones(FINAL, 55), &tones(FINAL, 67), 38.0, false);
    if bpb >= 2 {
        let motif = &theme.bars[0];
        let s = t0 + beat * bpb.min(4) / 2;
        for n in motif {
            w.note(t.celesta, s + (n.at * beat as f64).round() as u32, (n.len * beat as f64).round() as u32, celesta_t + n.rel, 36.0);
        }
    }
}

/// The heartbeat: two thumps a bar, the second softer.
fn heart(w: &mut Writer, track: usize, t0: u32, blen: u32, beat: u32, key: i32, vel: f64) {
    w.note(track, t0, beat / 4, key, vel);
    if beat / 3 < blen {
        w.note(track, t0 + beat / 3, beat / 4, key, vel - 14.0);
    }
}

/// The engine: quavers, the beat's first accented.
fn engine(w: &mut Writer, track: usize, t0: u32, blen: u32, beat: u32, key: i32, vel: f64) {
    let step = (beat / 2).max(1);
    for i in 0..blen / step {
        w.note(track, t0 + i * step, step * 3 / 5, key, vel - if i % 2 == 1 { 10.0 } else { 0.0 });
    }
}

/// The ticking pizzicato: the root, the fifth, the octave and the tenth in a pattern, in
/// quavers (or crotchets when sparse).
#[allow(clippy::too_many_arguments)]
fn pizz(w: &mut Writer, track: usize, t0: u32, blen: u32, beat: u32, pattern: &[usize; 4], low: i32, c: Chord, vel: f64, busy: bool) {
    let third = c.ivs.get(1).copied().unwrap_or(3);
    let fifth = c.ivs.get(2).copied().unwrap_or(7);
    let rungs = [low, low + fifth, low + 12, low + 12 + third, low + 12 + fifth];
    let step = if busy { beat / 2 } else { beat }.max(1);
    for i in 0..blen / step {
        let accent = if (i * step) % beat == 0 { 6.0 } else { 0.0 };
        w.note(track, t0 + i * step, step * 7 / 10, rungs[pattern[i as usize % 4]], vel + accent);
    }
}

/// A harp rolled up through the chord in quavers (in semiquavers over two octaves, a
/// glissando, when `gliss`).
#[allow(clippy::too_many_arguments)]
fn roll(w: &mut Writer, track: usize, t0: u32, beat: u32, low: &[i32], high: &[i32], vel: f64, gliss: bool) {
    let mut ladder: Vec<i32> = low.iter().chain(high).copied().collect();
    if gliss {
        ladder.extend(high.iter().map(|k| k + 12));
    }
    ladder.sort_unstable();
    ladder.dedup();
    let step = if gliss { beat / 4 } else { beat / 2 }.max(1);
    for (i, &k) in ladder.iter().enumerate() {
        w.note(track, t0 + i as u32 * step, beat * 2, k, vel + 8.0 * i as f64 / ladder.len().max(1) as f64);
    }
}

/// A low piano strike: the root in octaves (with a minor ninth above, a cluster, when
/// `cluster`), left to ring.
fn strike(w: &mut Writer, track: usize, t0: u32, len: u32, root: i32, vel: f64, cluster: bool) {
    w.note(track, t0, len, root, vel);
    w.note(track, t0, len, root + 12, vel - 6.0);
    if cluster {
        w.note(track, t0, len, root + 13, vel - 10.0);
    }
}

/// A horn swell on the chord's colour (a note of it other than the root), its expression
/// rising and falling.
#[allow(clippy::too_many_arguments)]
fn horn(w: &mut Writer, track: usize, t0: u32, span: u32, tones: &[i32], root: i32, vel: f64, lo: f64, hi: f64) {
    let k = tones.iter().copied().find(|k| (k - root).rem_euclid(12) != 0).unwrap_or(root);
    w.note(track, t0, span * 19 / 20, k, vel);
    w.glide(track, t0, span / 2, lo, hi);
    w.glide(track, t0 + span / 2, span / 2, hi, lo + 10.0);
}

/// The bassoon's line: the root, then (before a new chord) a semitone into the next
/// root, or else the fifth, a half bar each (the root alone in an odd metre).
#[allow(clippy::too_many_arguments)]
fn bassoon(
    w: &mut Writer,
    track: usize,
    t0: u32,
    blen: u32,
    bpb: u32,
    beat: u32,
    root: i32,
    c: Chord,
    next: Option<Chord>,
    next_root: &dyn Fn(Chord) -> i32,
    vel: f64,
) {
    if bpb % 2 == 1 || bpb < 4 {
        w.note(track, t0, blen * 19 / 20, root, vel);
        return;
    }
    let half = blen / 2;
    w.note(track, t0, half * 19 / 20, root, vel);
    let second = match next {
        Some(n) if n.root != c.root => {
            let target = next_root(n);
            let target = if (target - root).abs() > 6 { target - 12 * (target - root).signum() } else { target };
            target + if target >= root { -1 } else { 1 }
        }
        _ => root + c.ivs.get(2).copied().unwrap_or(7),
    };
    let _ = beat;
    w.note(track, t0 + half, (blen - half) * 19 / 20, second, vel - 4.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Style, compose};

    fn piece(seed: u64, bars: u32) -> Project {
        compose(&Recipe { style: Style::Chamber, ..Recipe::random(seed) }, Style::Chamber.tempo(), (4, 4), bars)
    }

    fn in_span(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the celesta's motif in the void, the clarinet (or cor anglais) in the
    /// Neon, the violin in the Swell, the strings' climax the loudest, and the last chord
    /// (minor, its major seventh and ninth) held to the end. No drums, no city.
    #[test]
    fn the_chamber_tells_its_story() {
        for seed in 0..12 {
            let p = piece(seed, 46);
            let r = p.recipe.unwrap();
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            let lead = if p.tracks.iter().any(|t| t.name == "Clarinet") { "Clarinet" } else { "Cor Anglais" };
            assert!(in_span(&p, "Celesta", b[0]) >= 3, "seed {seed}");
            assert_eq!(in_span(&p, lead, b[0]) + in_span(&p, "Violin", b[1]), 0, "seed {seed}");
            assert!(in_span(&p, lead, b[1]) >= 8, "seed {seed}");
            assert!(in_span(&p, "Violin", b[2]) >= 8, "seed {seed}");
            assert_eq!(in_span(&p, lead, b[2]), 0, "seed {seed}");
            let violin = p.tracks.iter().find(|t| t.name == "Violin").unwrap();
            let loudest = violin.notes.iter().max_by_key(|n| n.vel).unwrap();
            assert!(loudest.start >= b[3].0 && loudest.start < b[3].1, "seed {seed}");
            let end = p.end_tick();
            let held: Vec<u8> =
                p.tracks.iter().flat_map(|t| &t.notes).filter(|n| n.end() == end).map(|n| (n.key + 12 - r.root) % 12).collect();
            for pc in [0, 3, 7, 11, 2] {
                assert!(held.contains(&pc), "seed {seed}: {held:?}");
            }
            assert!(p.tracks.iter().all(|t| !matches!(t.role, Some(Role::Drums | Role::City))));
            assert_eq!(p.fx.ensemble, 0.0);
        }
    }

    /// The theme: eight full bars, the second bar of each chord's pair on its chord, the
    /// last held on the dominant's leading tone or fifth, and its harmony turning on the
    /// dominant seventh; never a leap past an octave.
    #[test]
    fn the_theme_turns_on_the_dominant() {
        for seed in 0..40 {
            for bpb in [3, 4, 5, 6, 7] {
                for mode in Mode::ALL {
                    let th = Theme::new(seed, bpb, mode);
                    assert_eq!(th.bars.len(), 8);
                    for b in &th.bars {
                        assert!((b.iter().map(|n| n.len).sum::<f64>() - bpb as f64).abs() < 1e-9, "seed {seed} {bpb}");
                    }
                    assert_eq!(th.prog[3], ch(7, DOM7));
                    assert_eq!(th.bars[7].len(), 1);
                    assert!(matches!(th.bars[7][0].rel.rem_euclid(12), 11 | 2), "seed {seed}");
                    let rels: Vec<i32> = th.bars.iter().flatten().map(|n| n.rel).collect();
                    assert!(rels.windows(2).all(|w| (w[1] - w[0]).abs() <= 12), "seed {seed}: {rels:?}");
                }
            }
        }
        // the scale over a chord holds all its notes
        for prog in PROGS {
            for c in prog {
                for mode in Mode::ALL {
                    let s = c.scale(mode.steps());
                    assert!(c.pcs().iter().all(|pc| s.contains(pc)), "{c:?} {mode:?} {s:?}");
                }
            }
        }
    }

    /// Any arrangement and metre: the length exactly, and sound in every section.
    #[test]
    fn any_arrangement() {
        for (meter, bars) in [((4, 4), 9), ((3, 4), 24), ((6, 8), 17), ((5, 4), 33), ((4, 4), 3)] {
            let p = compose(&Recipe { style: Style::Chamber, ..Recipe::random(7) }, 62.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
