//! The composer: a seeded neon-noir cue written to fit the tempo, metre and length it
//! is given. A seed picks the progression, voicings, patterns, melody, instruments and
//! the hall; the same recipe at the same tempo and length always writes the same
//! notes. It answers the tempo musically: chords last about six seconds whatever the
//! tempo, so slow pieces change chord every bar and fast ones every few, and the bells
//! fall in eighths when slow and in beats when fast. Its sections (rain, brass, a lone
//! lead, recede) are Neon, Swell, Night shift and Fade; by default they scale to the
//! length, and a piece can also be written to any arrangement of them (`compose_plan`).
//!
//! An Intro can open any piece: the drone, a thin pad, and the theme's first notes heard
//! alone before it begins.
//!
//! There are five styles, each writing the same kinds of section its own way:
//! **Nocturne**, the slow, rain-soaked ballad; **Pursuit**, driven by a sequenced bass
//! and an arpeggio, with stabbing brass and a backbeat; **Blackout**, dark and low, led
//! by drones, sub booms and distant metal; and **Momento**, romantic and swaying, a
//! sax-like theme over an electric piano and a soft swung groove; and **Noir**, the
//! melancholy of walking alone at night, told only in music (no rain or city sounds):
//! minor ninths and major sevenths over a slow walking bass, an electric piano, a
//! flickering arpeggio, brushes, and a lead that leaves room for a voice. All use the
//! same built-in sounds.
//!
//! Each musical layer draws from its own random stream (`rng.rs`, from Ambient MIDI
//! Generator), so changing one setting doesn't reshuffle the rest.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::{drum, fx};
use dd_dsp::rain::RainKind;
use serde::{Deserialize, Serialize};

use crate::palette::{Alt, Role, alternatives, same_sound};
use crate::project::{Ctrl, CtrlKind, Note, Part, Project, Section, Source, Strip, Track, noir_fx};
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Aeolian,
    Dorian,
    Phrygian,
    HarmonicMinor,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Aeolian, Mode::Dorian, Mode::Phrygian, Mode::HarmonicMinor];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Aeolian => "Minor",
            Mode::Dorian => "Dorian",
            Mode::Phrygian => "Phrygian",
            Mode::HarmonicMinor => "Harmonic minor",
        }
    }

    fn steps(self) -> [u8; 7] {
        match self {
            Mode::Aeolian => [0, 2, 3, 5, 7, 8, 10],
            Mode::Dorian => [0, 2, 3, 5, 7, 9, 10],
            Mode::Phrygian => [0, 1, 3, 5, 7, 8, 10],
            Mode::HarmonicMinor => [0, 2, 3, 5, 7, 8, 11],
        }
    }
}

/// How the composer writes: what each section plays, how fast the chords move and
/// which sounds it reaches for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// Slow and rain-soaked: brass swells, bells falling like water, a lone lead.
    #[default]
    Nocturne,
    /// Driven: a sequenced bass and an arpeggio running under stabbing brass and a backbeat.
    Pursuit,
    /// Dark and low: drones, sub booms and distant metal, a horn far off.
    Blackout,
    /// Romantic and swaying: a sliding, leaping theme over an electric piano and a soft
    /// swung groove, the pad blooming as it opens.
    Momento,
    /// Melancholy and spacious, in music alone: a lonely walk at night, with room left
    /// for a voice over it.
    Noir,
}

impl Style {
    pub const ALL: [Style; 5] = [Style::Nocturne, Style::Pursuit, Style::Blackout, Style::Momento, Style::Noir];

    pub fn label(self) -> &'static str {
        match self {
            Style::Nocturne => "Nocturne",
            Style::Pursuit => "Pursuit",
            Style::Blackout => "Blackout",
            Style::Momento => "Momento",
            Style::Noir => "Noir",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Style::Nocturne => "Slow and rain-soaked: brass swells, falling bells and a lone lead",
            Style::Pursuit => "Driven: a sequenced bass and arpeggio under stabbing brass and a backbeat",
            Style::Blackout => "Dark and low: drones, sub booms and distant metal, a horn far off",
            Style::Momento => "Romantic and swaying: a sliding theme over electric piano and a soft swung groove",
            Style::Noir => {
                "Melancholy and spacious, music alone (no rain or city sounds): ninths over a walking bass, electric piano, brushes; sits under a voice"
            }
        }
    }

    /// The tempo it suits, which choosing it sets.
    pub fn tempo(self) -> f64 {
        match self {
            Style::Nocturne => 66.0,
            Style::Pursuit => 104.0,
            Style::Blackout => 56.0,
            Style::Momento => 72.0,
            Style::Noir => 64.0,
        }
    }

    /// About how long a chord lasts, in seconds, and the most bars it may take.
    fn chord_secs(self) -> (f64, u32) {
        match self {
            Style::Nocturne => (6.5, 4),
            Style::Pursuit => (4.5, 4),
            Style::Blackout => (10.0, 6),
            Style::Momento => (5.5, 4),
            Style::Noir => (7.5, 4),
        }
    }
}

pub const KEY_NAMES: [&str; 12] = ["C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B"];

/// Everything that decides a composed piece apart from its tempo, metre and length,
/// which are the project's own.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Recipe {
    pub seed: u64,
    /// The tonic's pitch class, 0 = C.
    pub root: u8,
    pub mode: Mode,
    /// How busy the bells, plucks and drums are: 0 = sparse, 1 = busy.
    pub density: f32,
    pub style: Style,
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe { seed: 1, root: 2, mode: Mode::Aeolian, density: 0.5, style: Style::Nocturne }
    }
}

impl Recipe {
    /// A new seed, key and mode.
    pub fn random(entropy: u64) -> Recipe {
        let mut r = Rng::new(entropy);
        Recipe {
            seed: r.next_u64() % 1_000_000,
            root: r.range(0, 11) as u8,
            mode: *r.pick(&[Mode::Aeolian, Mode::Aeolian, Mode::Dorian, Mode::Phrygian, Mode::HarmonicMinor]),
            density: r.range_f(0.3, 0.75) as f32,
            style: Style::Nocturne,
        }
    }

    pub fn key_label(&self) -> String {
        format!("{} {}", KEY_NAMES[self.root as usize % 12], self.mode.label())
    }
}

/// A stable fingerprint of the notes and controllers, so the app can tell whether a
/// composed piece has been edited by hand (FNV-1a). Sections are left out: like the
/// tempo, they're the piece's shape, and changing them writes it again.
pub fn fingerprint(p: &Project) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        for b in x.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    eat(p.tracks.len() as u64);
    for t in &p.tracks {
        eat(t.notes.len() as u64);
        for n in &t.notes {
            eat(((n.start as u64) << 32) | ((n.len as u64) << 16) | ((n.key as u64) << 8) | n.vel as u64);
        }
        for c in &t.ctrls {
            let k = match c.kind {
                CtrlKind::Cc(n) => n as u64,
                CtrlKind::Bend => 300,
                CtrlKind::Pressure => 301,
            };
            eat(((c.tick as u64) << 32) | (k << 16) | (c.value as u16 as u64));
        }
    }
    h
}

/// A chord: its pitch classes (lowest is the root) and where it sounds.
#[derive(Clone, Debug)]
struct Chord {
    start: u32,
    len: u32,
    root_pc: u8,
    /// Upper voicing, MIDI keys.
    voicing: Vec<u8>,
    /// Chord tones as pitch classes.
    pcs: Vec<u8>,
}

/// Keys of the pitch class nearest `target`.
fn near(pc: u8, target: i32) -> u8 {
    let base = target - target.rem_euclid(12) + pc as i32;
    let cands = [base - 12, base, base + 12];
    cands.into_iter().min_by_key(|k| (k - target).abs()).unwrap_or(base).clamp(0, 127) as u8
}

/// The close-position voicing of `pcs` nearest to `prev` (or centred on `centre`).
fn voice(pcs: &[u8], prev: Option<&[u8]>, centre: i32) -> Vec<u8> {
    let mut best: Option<(i32, Vec<i32>)> = None;
    for inv in 0..pcs.len() {
        for lowest in (centre - 10)..(centre + 2) {
            if lowest.rem_euclid(12) as u8 != pcs[inv] {
                continue;
            }
            let mut v = vec![lowest];
            for k in 1..pcs.len() {
                let pc = pcs[(inv + k) % pcs.len()] as i32;
                let last = *v.last().unwrap_or(&lowest);
                let mut n = last - last.rem_euclid(12) + pc;
                while n <= last {
                    n += 12;
                }
                v.push(n);
            }
            let cost = match prev {
                Some(p) => {
                    let mut a: Vec<i32> = p.iter().map(|x| *x as i32).collect();
                    a.sort_unstable();
                    a.iter().zip(&v).map(|(x, y)| (x - y).abs()).sum::<i32>() + (v[0] - centre).abs() / 3
                }
                None => (v.iter().sum::<i32>() / v.len() as i32 - centre).abs(),
            };
            if best.as_ref().is_none_or(|(c, _)| cost < *c) {
                best = Some((cost, v));
            }
        }
    }
    best.map(|(_, v)| v.into_iter().map(|x| x.clamp(0, 127) as u8).collect()).unwrap_or_default()
}

const PROGRESSIONS: [[usize; 4]; 10] = [
    [0, 5, 3, 4],
    [0, 6, 5, 6],
    [0, 3, 5, 4],
    [0, 5, 2, 6],
    [0, 0, 5, 6],
    [5, 6, 0, 0],
    [0, 4, 5, 3],
    [0, 5, 3, 6],
    [0, 2, 5, 4],
    [0, 6, 3, 5],
];
const PHRYGIAN: [[usize; 4]; 2] = [[0, 1, 0, 6], [0, 1, 5, 0]];
/// Minor progressions that push on: i–VI–VII and its relatives.
const DRIVEN: [[usize; 4]; 6] = [[0, 5, 6, 0], [0, 6, 5, 6], [0, 3, 6, 5], [0, 5, 3, 6], [0, 0, 6, 5], [0, 2, 5, 6]];
/// Progressions that barely move: the tonic, a step away and back, a sixth below.
const DARK: [[usize; 4]; 5] = [[0, 1, 0, 5], [0, 5, 0, 1], [0, 0, 5, 0], [0, 3, 0, 1], [0, 5, 1, 0]];

/// Progressions that open towards the relative major: VI, III and VII around the tonic.
const ROMANTIC: [[usize; 4]; 6] = [[0, 5, 2, 6], [5, 6, 0, 0], [0, 3, 5, 6], [5, 2, 3, 0], [0, 5, 3, 4], [2, 5, 0, 6]];

/// Noir's: falling by fourths and steps, through the major sixth and seventh, with a
/// minor (or, in harmonic minor, major) fifth that never quite resolves.
const NOIR: [[usize; 4]; 6] = [[0, 5, 3, 4], [0, 3, 6, 2], [5, 4, 0, 0], [0, 5, 2, 6], [3, 4, 0, 5], [0, 1, 4, 0]];

fn chords(r: &Recipe, p: &Project, chord_ticks: u32, end: u32) -> Vec<Chord> {
    let mut rng = Rng::stream(r.seed, "harmony");
    let steps = r.mode.steps();
    let prog = match r.style {
        Style::Nocturne if r.mode == Mode::Phrygian && rng.chance(0.6) => *rng.pick(&PHRYGIAN),
        Style::Nocturne => *rng.pick(&PROGRESSIONS),
        Style::Pursuit => *rng.pick(&DRIVEN),
        Style::Blackout => *rng.pick(&DARK),
        Style::Momento => *rng.pick(&ROMANTIC),
        Style::Noir => *rng.pick(&NOIR),
    };
    // (Noir's sits a little low, under a voice)
    let centre = match r.style {
        Style::Blackout => 50,
        Style::Noir => 57,
        _ => 60,
    };
    let pc = |deg: usize| (r.root + steps[deg % 7]) % 12;
    let mut out: Vec<Chord> = Vec::new();
    let mut t = 0;
    let mut i = 0;
    let _ = p;
    while t < end {
        let last = t + chord_ticks >= end;
        let deg = if last { 0 } else { prog[i % 4] };
        let (a, b, c) = (pc(deg), pc(deg + 2), pc(deg + 4));
        let pcs = match r.style {
            // colour: add9, sus2 or sus4 (a cycle's last chord most often hangs on a sus4)
            Style::Nocturne => match (last, i % 4 == 3, rng.weighted(&[3.0, 1.5, 1.0, 1.0])) {
                (true, _, _) => vec![a, pc(deg + 1), c, b],
                (_, true, _) if rng.chance(0.6) => vec![a, pc(deg + 3), c],
                (_, _, 0) => vec![a, b, c, pc(deg + 1)],
                (_, _, 1) => vec![a, pc(deg + 1), c],
                (_, _, 2) => vec![a, pc(deg + 3), c],
                _ => vec![a, b, c, pc(deg + 6)],
            },
            // plain triads, hanging on a sus4 before the cycle turns
            Style::Pursuit => match (last, i % 4 == 3 && rng.chance(0.5)) {
                (true, _) => vec![a, pc(deg + 1), c],
                (_, true) => vec![a, pc(deg + 3), c],
                _ => vec![a, b, c],
            },
            // bare: triads, open sus2s, and a last chord that never settles
            Style::Blackout => match (last, rng.chance(0.4)) {
                (true, _) => vec![a, pc(deg + 1), c],
                (_, true) => vec![a, pc(deg + 1), c],
                _ => vec![a, b, c],
            },
            // lush: sevenths and ninths, ending on an add9 left hanging
            Style::Momento => match (last, rng.weighted(&[3.0, 2.0, 1.0])) {
                (true, _) => vec![a, pc(deg + 1), b, c],
                (_, 0) => vec![a, b, c, pc(deg + 6)],
                (_, 1) => vec![a, b, c, pc(deg + 1)],
                _ => vec![a, b, pc(deg + 6), pc(deg + 1)],
            },
            // bittersweet: sevenths, ninths without the fifth, elevenths (on the sixth,
            // a major seventh with a sharp eleventh), and an unresolved minor ninth to end
            Style::Noir => match (last, rng.weighted(&[3.0, 2.5, 1.5])) {
                (true, _) => vec![a, b, pc(deg + 6), pc(deg + 1)],
                (_, 0) => vec![a, b, c, pc(deg + 6)],
                (_, 1) => vec![a, b, pc(deg + 6), pc(deg + 1)],
                _ => vec![a, b, pc(deg + 3), pc(deg + 6)],
            },
        };
        let prev = out.last().map(|c| c.voicing.clone());
        let voicing = voice(&pcs, prev.as_deref(), centre);
        let len = chord_ticks.min(end - t);
        out.push(Chord { start: t, len, root_pc: a, voicing, pcs });
        t += chord_ticks;
        i += 1;
    }
    out
}

fn chord_at(chords: &[Chord], tick: u32) -> &Chord {
    chords.iter().rev().find(|c| c.start <= tick).unwrap_or(&chords[0])
}

/// About six seconds a chord (four when driven, ten when dark), in whole bars.
fn chord_bars(style: Style, bar_secs: f64, bars: u32) -> u32 {
    let (secs, most) = style.chord_secs();
    ((secs / bar_secs).round() as u32).clamp(1, most).min(bars.max(1))
}

/// The usual arrangement for a length: Neon, Swell, Night shift and Fade starting at
/// a sixth, a half and five sixths, on chord changes (short pieces drop the later ones).
/// Noir opens with an Intro of a chord or two first, where there's room.
pub fn default_plan(style: Style, bpm: f64, meter: (u32, u32), bars: u32) -> Vec<(Part, u32)> {
    const PARTS: [Part; 4] = [Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let p = Project { bpm, meter, length_bars: bars.max(2), ..Project::default() };
    let bars = p.length_bars;
    let cb = chord_bars(style, p.bar_secs(), bars);
    let snap = |frac: f64| ((bars as f64 * frac / cb as f64).round() as u32 * cb).min(bars);
    if style == Style::Noir && bars >= cb * 5 {
        let intro = if bars >= cb * 10 { cb * 2 } else { cb };
        let mut bounds = vec![(Part::Intro, 0), (Part::Neon, intro)];
        for (part, frac) in [(Part::Swell, 0.3), (Part::NightShift, 0.5), (Part::Fade, 5.0 / 6.0)] {
            let b = snap(frac);
            // (a part with no room before the next is left out)
            if b > bounds.last().map_or(0, |l| l.1) && b < bars {
                bounds.push((part, b));
            }
        }
        return bounds.iter().enumerate().map(|(i, (part, b))| (*part, bounds.get(i + 1).map_or(bars, |n| n.1) - b)).collect();
    }
    let mut bounds = vec![0, snap(1.0 / 6.0), snap(0.5), snap(5.0 / 6.0)];
    bounds.dedup();
    bounds.retain(|b| *b < bars);
    bounds.iter().enumerate().map(|(i, b)| (PARTS[i.min(3)], bounds.get(i + 1).unwrap_or(&bars) - b)).collect()
}

/// A fresh arrangement for a recipe at this length: three to seven sections in whole
/// chords, opening quietly (a Neon, or now and then a Swell), reaching at least one
/// night shift, never the same kind twice running, and ending on a fade. The same
/// recipe always draws the same one.
pub fn random_plan(r: &Recipe, bpm: f64, meter: (u32, u32), bars: u32) -> Vec<(Part, u32)> {
    let p = Project { bpm, meter, length_bars: bars.max(2), ..Project::default() };
    let bars = p.length_bars;
    let cb = chord_bars(r.style, p.bar_secs(), bars).max(1);
    let units = (bars / cb).max(1);
    let mut rng = Rng::stream(r.seed, "form");
    if units < 3 {
        return default_plan(r.style, bpm, meter, bars);
    }
    let most = units.min(7) as i32;
    let n = rng.range(3.min(most), most.min(3 + units as i32 / 4)).max(3) as usize;
    // the kinds: a quiet start, a fade to end, a night shift somewhere between
    let mut parts = vec![if rng.chance(0.8) { Part::Neon } else { Part::Swell }];
    while parts.len() < n - 1 {
        let last = *parts.last().unwrap_or(&Part::Neon);
        let options: Vec<Part> = [Part::Neon, Part::Swell, Part::NightShift].into_iter().filter(|p| *p != last).collect();
        // building more often than falling back
        let w: Vec<f64> = options.iter().map(|p| if *p == Part::Neon { 1.0 } else { 2.0 }).collect();
        parts.push(options[rng.weighted(&w)]);
    }
    if !parts.contains(&Part::NightShift) {
        let i = parts.len() - 1;
        if parts[i] == Part::NightShift || i == 0 {
            parts.push(Part::NightShift);
        } else {
            parts[i] = Part::NightShift;
        }
    }
    parts.push(Part::Fade);
    // whole chords each, the night shifts longest, any odd bars to the end
    let mut lens: Vec<u32> = vec![1; parts.len()];
    let mut left = units.saturating_sub(parts.len() as u32);
    while left > 0 {
        let w: Vec<f64> = parts.iter().map(|p| if *p == Part::NightShift { 3.0 } else { 1.0 }).collect();
        lens[rng.weighted(&w)] += 1;
        left -= 1;
    }
    let mut plan: Vec<(Part, u32)> = parts.into_iter().zip(lens).map(|(p, l)| (p, l * cb)).collect();
    let total: u32 = plan.iter().map(|x| x.1).sum();
    if let Some(last) = plan.last_mut() {
        last.1 += bars.saturating_sub(total);
    }
    // an intro now and then (in Noir, always), a chord or two taken from the opening
    // section or else the longest (its own stream, so the rest is as it was)
    let mut ir = Rng::stream(r.seed, "intro-form");
    if units >= 4 && ir.chance(if r.style == Style::Noir { 1.0 } else { 0.35 }) {
        let len = if units >= 10 { 2 * cb } else { cb };
        let from = if plan[0].1 > len {
            Some(0)
        } else {
            (0..plan.len()).filter(|i| plan[*i].1 > len).max_by_key(|i| (plan[*i].1, plan[*i].0 == Part::NightShift))
        };
        if let Some(i) = from {
            plan[i].1 -= len;
            plan.insert(0, (Part::Intro, len));
        }
    }
    plan
}

/// Write a piece for this recipe at the project's tempo, metre and length, in the
/// usual arrangement. The tracks, sounds, mix, sections and effects are all new.
pub fn compose(r: &Recipe, bpm: f64, meter: (u32, u32), bars: u32) -> Project {
    compose_plan(r, bpm, meter, &default_plan(r.style, bpm, meter, bars))
}

/// Write a piece for this recipe to an arrangement of sections, each a part and a
/// number of bars; the piece is as long as they are.
pub fn compose_plan(r: &Recipe, bpm: f64, meter: (u32, u32), plan: &[(Part, u32)]) -> Project {
    let plan: Vec<(Part, u32)> = if plan.is_empty() { vec![(Part::Neon, 2)] } else { plan.to_vec() };
    let total: u32 = plan.iter().map(|(_, b)| (*b).max(1)).sum();
    let mut p = Project { bpm, meter, length_bars: total.max(2), ..Project::default() };
    let bars = p.length_bars;
    let (beat, bar) = (p.beat_ticks(), p.bar_ticks());
    let beats_per_bar = meter.0.max(1);
    let end = bars * bar;
    let bar_secs = p.bar_secs();
    let style = r.style;
    let chord_bars = chord_bars(style, bar_secs, bars);
    let chord_ticks = chord_bars * bar;
    let chords = chords(r, &p, chord_ticks, end);
    let density = r.density.clamp(0.0, 1.0) as f64;

    // the sections, in ticks: (part, start, end); a short last one is held to the end
    let mut secs: Vec<(Part, u32, u32)> = Vec::new();
    let mut b0 = 0;
    for (part, n) in &plan {
        let b1 = b0 + (*n).max(1);
        secs.push((*part, b0 * bar, b1 * bar));
        b0 = b1;
    }
    if let Some(last) = secs.last_mut() {
        last.2 = end;
    }
    p.sections = secs.iter().map(|(part, a, _)| Section::new(a / bar, *part)).collect();
    let part_at = |tick: u32| secs.iter().find(|(_, a, z)| tick >= *a && tick < *z).map_or(Part::Fade, |s| s.0);
    let of = |part: Part| secs.iter().filter(move |s| s.0 == part).map(|s| (s.1, s.2));
    let mut names_rng = Rng::stream(r.seed, "names");
    // (a draw that once chose the section names, kept so titles stay the same)
    let _ = names_rng.next_u64();
    let (adjectives, nouns) = match style {
        Style::Nocturne => (
            ["Neon", "Midnight", "Sodium", "Wet", "Electric", "Static", "Chrome", "Blue", "Distant", "Last"],
            ["Rain", "Overpass", "Avenue", "Skyline", "Arcade", "Monorail", "Harbour", "Signal", "Rooftops", "Memory"],
        ),
        Style::Pursuit => (
            ["Red", "Burning", "Chrome", "Express", "Electric", "Fast", "Wet", "Low", "Neon", "Last"],
            ["Interchange", "Freeway", "Tunnel", "Pursuit", "Underpass", "Expressway", "Getaway", "Overpass", "Run", "Lane"],
        ),
        Style::Blackout => (
            ["Black", "Deep", "Drowned", "Hollow", "Silent", "Lower", "Cold", "Buried", "Dead", "Dark"],
            ["Cistern", "Sublevel", "Undertow", "Reservoir", "Foundry", "Shaft", "Vault", "Water", "Tide", "Station"],
        ),
        Style::Momento => (
            ["Slow", "Golden", "Last", "Warm", "Late", "Silver", "Quiet", "Velvet", "Borrowed", "Second"],
            ["Dance", "Embrace", "Window", "Promise", "Rain", "Letter", "Hours", "Light", "Glance", "Waltz"],
        ),
        Style::Noir => (
            ["Wet", "Lonely", "Last", "Empty", "Paper", "Late", "Flickering", "Sodium", "Quiet", "Cold"],
            ["Crossing", "Lanterns", "Umbrella", "Platform", "Alley", "Reflections", "Taxi", "Train", "Window", "Street"],
        ),
    };
    p.title = format!("{} {}", names_rng.pick(&adjectives), names_rng.pick(&nouns));

    // ---------------------------------------------------------------- sounds
    // each part from its role's alternatives: the usual sound most often
    let mut snd = Rng::stream(r.seed, "sounds");
    let pad_role = if snd.chance(0.25) { Role::Strings } else { Role::Pad };
    let mut pick = |role: Role, not: Option<&Source>| -> Alt {
        let alts: Vec<Alt> = alternatives(role)
            .into_iter()
            // a stab can't hold a chord
            .filter(|a| a.bundled && a.label != "Dark Stab" && not.is_none_or(|n| !same_sound(n, &a.source)))
            .collect();
        let w: Vec<f64> = match style {
            Style::Nocturne => (0..alts.len()).map(|i| if i == 0 { 3.0 } else { 1.0 }).collect(),
            // the sounds the style is written for, most often
            _ => alts.iter().map(|a| if prefers(style, role, a.label) { 3.0 } else { 0.4 }).collect(),
        };
        alts[snd.weighted(&w)].clone()
    };
    let brass = pick(Role::Brass, None);
    let pad = pick(pad_role, None);
    let lead = pick(Role::Lead, None);
    let answer = pick(Role::Answer, Some(&lead.source));
    let bass = pick(Role::Bass, None);
    let bells = pick(Role::Bells, None);
    let pluck = pick(Role::Pluck, None);
    let drone = pick(Role::Drone, None);
    let mut fx = noir_fx();
    let halls = match style {
        Style::Nocturne => [6.0, 2.0, 1.5],
        Style::Pursuit => [6.0, 2.0, 0.5],
        Style::Blackout => [2.0, 3.0, 4.0],
        Style::Momento => [5.0, 4.0, 0.5],
        Style::Noir => [5.0, 3.0, 1.0],
    };
    fx.space = [Space::Noir, Space::Cathedral, Space::BlackHole][snd.weighted(&halls)];
    // an echo of about 0.6 s
    let beat_secs = p.tick_secs(beat);
    fx.echo.time = *EchoTime::ALL
        .iter()
        .min_by(|a, b| ((a.beats() * beat_secs) - 0.6).abs().total_cmp(&((b.beats() * beat_secs) - 0.6).abs()))
        .unwrap_or(&EchoTime::DottedQuarter);
    p.fx = fx;

    let mut tracks: Vec<Track> = Vec::new();
    let mut add = |role: Role, alt: Alt, strip: Strip| {
        let i = tracks.len();
        let strip = Strip { gain_db: strip.gain_db + alt.trim_db, ..strip };
        tracks.push(Track { strip, role: Some(role), ..Track::new(role.label(), alt.source, i) });
        i
    };
    let s = Strip::default();
    // the mix leans on what drives each style: the bass and drums, or the drones
    let (lift_bass, lift_pluck, lift_drone, lift_drums) = match style {
        Style::Nocturne => (0.0, 0.0, 0.0, 0.0),
        Style::Pursuit => (-1.0, -3.0, -3.0, -1.5),
        Style::Blackout => (-3.0, 0.0, 4.0, 0.0),
        Style::Momento => (-1.0, -4.0, -5.0, -3.0),
        Style::Noir => (-2.0, -5.0, -4.0, -2.0),
    };
    let t_brass = add(Role::Brass, brass, Strip { reverb: 0.5, ensemble: true, gain_db: -3.0, ..s });
    let t_pad = add(pad_role, pad, Strip { reverb: 0.6, ensemble: true, gain_db: -6.0, ..s });
    let t_lead = add(Role::Lead, lead, Strip { reverb: 0.55, echo: 0.35, pan: 0.1, gain_db: 0.0, ..s });
    let t_bass = add(Role::Bass, bass, Strip { reverb: 0.08, gain_db: 2.0 + lift_bass, ..s });
    let t_bells = add(Role::Bells, bells, Strip { reverb: 0.65, echo: 0.4, gain_db: -4.0, pan: -0.2, ..s });
    let t_pluck = add(Role::Pluck, pluck, Strip { reverb: 0.5, echo: 0.3, pan: 0.35, gain_db: lift_pluck, ..s });
    let t_answer = add(Role::Answer, answer, Strip { reverb: 0.6, echo: 0.3, pan: -0.15, gain_db: -1.0, ..s });
    let t_drone = add(Role::Drone, drone, Strip { reverb: 0.5, gain_db: -7.0 + lift_drone, width: 1.4, ..s });
    let city_alt = alternatives(Role::City).remove(0);
    let t_city = add(Role::City, city_alt, Strip { reverb: 0.25, gain_db: -6.0, width: 1.4, ..s });
    let drums_alt = {
        let mut alts = alternatives(Role::Drums);
        let i = if style == Style::Noir { alts.iter().position(|a| a.label == "GM Brush Kit").unwrap_or(0) } else { 0 };
        alts.remove(i)
    };
    let t_drums = add(
        Role::Drums,
        drums_alt,
        Strip {
            reverb: if style == Style::Noir { 0.3 } else { 0.4 },
            echo: if matches!(style, Style::Pursuit | Style::Noir) { 0.05 } else { 0.15 },
            gain_db: -5.0 + lift_drums,
            ..s
        },
    );
    let t_keys = matches!(style, Style::Momento | Style::Noir).then(|| {
        // from its own stream, so the other parts' sounds stay as they were
        let mut kr = Rng::stream(r.seed, "keys");
        let alts: Vec<Alt> = alternatives(Role::Keys).into_iter().filter(|a| a.bundled).collect();
        let w: Vec<f64> = alts.iter().map(|a| if prefers(style, Role::Keys, a.label) { 3.0 } else { 0.4 }).collect();
        let alt = alts[kr.weighted(&w)].clone();
        add(Role::Keys, alt, Strip { reverb: 0.45, echo: 0.2, pan: -0.25, gain_db: 1.0, ..s })
    });
    p.tracks = tracks;
    // the dark is quieter at heart: bring it up to sit with the others
    if style == Style::Blackout {
        p.master_db = 1.0;
    }
    // the romance is a lush bed under the theme
    if style == Style::Momento {
        p.tracks[t_pad].strip.gain_db += 2.0;
        p.master_db = 0.5;
    }
    // the night walk sits under a voice: quieter overall, the lead back, the arpeggio
    // far off and dark
    if style == Style::Noir {
        p.tracks[t_lead].strip.gain_db -= 2.0;
        p.tracks[t_lead].strip.echo = 0.45;
        p.tracks[t_pluck].strip.echo = 0.5;
        p.tracks[t_pluck].strip.high_cut = 4000.0;
        p.tracks[t_pad].strip.high_cut = 8000.0;
        p.master_db = -1.0;
    }
    let t = &mut p.tracks;
    let at = |tick: f64| tick.round().max(0.0) as u32;
    let push = |v: &mut Vec<Note>, start: u32, len: u32, key: u8, vel: u8| {
        if start < end && len > 0 {
            v.push(Note { start, len: len.min(end - start), key, vel: vel.clamp(1, 127) });
        }
    };

    // ---------------------------------------------------------------- pad and brass
    let breath = beat;
    let last_chord = chords.len() - 1;
    for (ci, c) in chords.iter().enumerate() {
        let ring = if ci == last_chord { end - c.start } else { c.len.saturating_sub(breath / 2) };
        let part = part_at(c.start);
        if part == Part::Intro {
            // thin: the top two notes, softly
            for &k in c.voicing.iter().rev().take(2) {
                push(&mut t[t_pad].notes, c.start, ring, k, 46);
            }
            continue;
        }
        for &k in &c.voicing {
            push(&mut t[t_pad].notes, c.start, ring, k, 60);
        }
        if style == Style::Pursuit && part == Part::NightShift {
            // stabs, pushed: on the one, the and of two, and the four, every bar
            let pushes: Vec<u32> = [0.0, 1.5, 3.0].iter().map(|b| at(b * beat as f64)).filter(|o| *o < bar).collect();
            for b in (c.start..c.start + c.len).step_by(bar as usize) {
                for (j, o) in pushes.iter().enumerate() {
                    for &k in &c.voicing {
                        push(&mut t[t_brass].notes, b + o, beat / 2, k + 12, if j == 0 { 108 } else { 96 });
                    }
                }
            }
            continue;
        }
        if style == Style::Momento {
            // the pad blooms an octave wider in the night shift, the horns a soft bed under it
            if part == Part::NightShift {
                let top = *c.voicing.last().unwrap_or(&72);
                push(&mut t[t_pad].notes, c.start, ring, top + 12, 52);
                for &k in &c.voicing {
                    push(&mut t[t_brass].notes, c.start, c.len.saturating_sub(breath), k, 70);
                }
            }
            continue;
        }
        if style == Style::Noir {
            // a soft, low swell of horns under the night shift only
            if part == Part::NightShift {
                for &k in &c.voicing {
                    push(&mut t[t_brass].notes, c.start, c.len.saturating_sub(breath), k, 72);
                }
            }
            continue;
        }
        if style == Style::Blackout {
            // low swells of the root and fifth, through the swells and night shifts
            if matches!(part, Part::Swell | Part::NightShift) {
                let (root, fifth) = (near(c.root_pc, 48), near((c.root_pc + 7) % 12, 55));
                for k in [root, fifth] {
                    push(&mut t[t_brass].notes, c.start, c.len.saturating_sub(breath), k, if part == Part::NightShift { 96 } else { 80 });
                }
            }
            continue;
        }
        if matches!(part, Part::Swell | Part::NightShift) {
            let vel = if part == Part::NightShift { 104 } else { 88 };
            for &k in &c.voicing {
                push(&mut t[t_brass].notes, c.start, c.len.saturating_sub(breath), k + 12, vel);
            }
            if part == Part::NightShift && c.voicing.len() > 1 {
                let top = *c.voicing.last().unwrap_or(&72);
                push(&mut t[t_brass].notes, c.start, c.len.saturating_sub(breath), top + 24, vel - 20);
            }
        }
    }

    // ---------------------------------------------------------------- bass
    let mut brng = Rng::stream(r.seed, "bass");
    let bass_style = brng.range(0, 2);
    match style {
        Style::Pursuit => driven_bass(&mut t[t_bass].notes, &chords, &secs, &mut brng, beat, bar, beat_secs, end),
        Style::Blackout => {
            // a sub under every chord once the swells begin, held
            let from = secs.iter().find(|s| s.0 != Part::Neon).map_or(end, |s| s.1);
            for c in chords.iter().filter(|c| c.start + c.len > from) {
                let a = c.start.max(from);
                push(&mut t[t_bass].notes, a, c.start + c.len - a, near(c.root_pc, 33), 100);
            }
        }
        Style::Momento => swaying_bass(&mut t[t_bass].notes, &chords, &secs, beat, bar, beats_per_bar, end),
        Style::Noir => walking_bass(&mut t[t_bass].notes, &chords, &secs, &mut brng, beat, bar, beats_per_bar, end),
        Style::Nocturne => {}
    }
    for c in chords.iter().filter(|c| style == Style::Nocturne && matches!(part_at(c.start), Part::Swell | Part::NightShift)) {
        let k = near(c.root_pc, 45);
        match bass_style {
            0 => push(&mut t[t_bass].notes, c.start, c.len.saturating_sub(beat / 2), k, 96),
            1 => {
                for b in (c.start..c.start + c.len).step_by((bar / 2).max(1) as usize) {
                    push(&mut t[t_bass].notes, b, bar / 2 - beat / 4, k, if (b - c.start) % bar == 0 { 96 } else { 78 });
                }
            }
            _ => {
                for b in (c.start..c.start + c.len).step_by(bar as usize) {
                    push(&mut t[t_bass].notes, b, beat / 2, k, 100);
                    push(&mut t[t_bass].notes, b + beat / 2, beat / 2, k, 72);
                }
            }
        }
    }

    // ---------------------------------------------------------------- bells
    let mut bel = Rng::stream(r.seed, "bells");
    // in eighths when the beat is slow, in beats when it's quick
    let step = if beat_secs > 0.62 { beat / 2 } else { beat };
    let steps_per_bar = (bar / step).max(1) as usize;
    let pattern: Vec<Option<usize>> = (0..steps_per_bar.max(4))
        .map(|i| if i > 0 && bel.chance(0.35 - density * 0.2) { None } else { Some(bel.range(0, 4) as usize) })
        .collect();
    // after the first chord, and through the first half of each fade
    let bells_from = match secs.iter().find(|s| s.0 != Part::Intro) {
        Some((Part::Neon, a, z)) if z - a > chord_ticks => a + chord_ticks,
        Some((_, a, _)) => *a,
        None => end,
    };
    let ringing = |tick: u32| match secs.iter().find(|(_, a, z)| tick >= *a && tick < *z) {
        Some((Part::Fade, a, z)) => tick < a + (z - a) / 2,
        Some((Part::Intro, _, _)) => false,
        _ => true,
    };
    let mut tick = bells_from;
    let mut i = 0usize;
    if style != Style::Nocturne {
        // chimes rather than rain: a note on a chord's first beat now and then, in the
        // quiet sections (Pursuit), or rarely and high in the dark (Blackout)
        let chance = match style {
            Style::Pursuit => 0.7,
            Style::Momento => 0.55,
            Style::Noir => 0.3,
            _ => 0.35,
        };
        for c in &chords {
            if matches!(part_at(c.start), Part::Neon | Part::Fade) && bel.chance(chance) {
                let k = c.voicing.last().copied().unwrap_or(72) + if style == Style::Pursuit { 12 } else { 24 };
                // (off the beat the answer starts on, in the dark and the romance)
                let (off, vel) = if style == Style::Pursuit { (beat, 60) } else { (beat * 2 + beat / 2, 48) };
                push(&mut t[t_bells].notes, c.start + off, beat * 2, k.min(108), vel + bel.range(0, 20) as u8);
            }
        }
        tick = end;
    }
    while tick < end {
        let c = chord_at(&chords, tick);
        if let Some(sidx) = pattern[i % pattern.len()]
            && ringing(tick)
            && bel.chance(0.55 + density * 0.4)
        {
            let k = c.voicing.get(sidx).copied().unwrap_or_else(|| c.voicing.last().copied().unwrap_or(60) + 12) + 24;
            push(&mut t[t_bells].notes, tick, step, k.min(108), 64 + bel.range(0, 34) as u8);
        }
        tick += step;
        i += 1;
    }

    // ---------------------------------------------------------------- lead melody
    let mut mel = Rng::stream(r.seed, "melody");
    let steps = r.mode.steps();
    let (lo, hi, from) = if style == Style::Blackout { (45u8, 70u8, 55u8) } else { (55, 81, 67) };
    let scale: Vec<u8> = (lo..=hi).filter(|k| steps.contains(&((k + 12 - r.root % 12) % 12))).collect();
    let phrase_bars = if chord_bars >= 2 { chord_bars * 2 } else { 4 }.min(8);
    let cells: Vec<&[f64]> = match style {
        Style::Nocturne => vec![&[3.0, 1.0], &[2.0, 2.0], &[4.0], &[1.0, 3.0], &[1.5, 0.5, 2.0], &[2.0, 1.0, 1.0], &[1.0, 1.0, 2.0]],
        // short and pushed, with a held note now and then
        Style::Pursuit => vec![&[1.0, 1.0], &[0.5, 0.5, 1.0], &[1.5, 0.5], &[2.0], &[1.0, 0.5, 0.5], &[0.5, 1.0, 0.5], &[3.0, 1.0]],
        // long, low and few
        Style::Blackout => vec![&[4.0], &[6.0, 2.0], &[3.0, 1.0], &[8.0]],
        // a long note after a leap, then a fall in shorter ones
        Style::Momento => vec![&[3.0, 1.0], &[2.0, 1.0, 1.0], &[4.0], &[1.5, 0.5, 2.0], &[2.0, 2.0], &[1.0, 1.0, 2.0]],
        // few notes, held, a sigh now and then
        Style::Noir => vec![&[3.0, 1.0], &[4.0], &[2.0, 2.0], &[1.5, 0.5, 2.0], &[6.0, 2.0]],
    };
    let phrase_beats = (phrase_bars * beats_per_bar) as f64;
    let rhythm_a: Vec<f64> = {
        let mut v = Vec::new();
        let mut total = 0.0;
        while total < phrase_beats - 2.0 {
            for d in *mel.pick(&cells) {
                if total + d <= phrase_beats - 1.0 {
                    v.push(*d);
                    total += d;
                }
            }
            if v.is_empty() {
                break;
            }
        }
        v
    };
    let mut pos_idx = scale.iter().position(|k| *k >= from).unwrap_or(scale.len() / 2) as i32;
    let mut phrase_no = 0;
    for (s2, s3) in of(Part::NightShift) {
        let mut phrase_start = s2;
        while phrase_start < s3 {
            let mut t0 = phrase_start as f64;
            // phrases A A' B A: B gets a fresh rhythm (in the dark, every other phrase is silence)
            let rhythm: Vec<f64> = if matches!(style, Style::Blackout | Style::Noir) && phrase_no % 2 == 1 {
                Vec::new()
            } else if phrase_no % 4 == 2 {
                rhythm_a.iter().rev().copied().collect()
            } else {
                rhythm_a.clone()
            };
            for (ni, d) in rhythm.iter().enumerate() {
                let start = at(t0);
                if start >= s3 {
                    break;
                }
                let c = chord_at(&chords, start);
                let strong = (start - c.start).is_multiple_of(bar);
                let mv = if style == Style::Momento {
                    // leap up a fifth or more, then fall by step
                    if mel.chance(0.3) { mel.range(4, 6) } else { mel.range(-2, 1) }
                } else if mel.chance(0.25) {
                    mel.range(-4, 4)
                } else {
                    mel.range(-2, 2)
                };
                pos_idx = (pos_idx + mv).clamp(2, scale.len() as i32 - 3);
                let mut k = scale[pos_idx as usize];
                if strong || *d >= 3.0 || ni + 1 == rhythm.len() {
                    // long and strong notes land on the chord
                    let tones: Vec<u8> = scale.iter().copied().filter(|x| c.pcs.contains(&(x % 12))).collect();
                    if let Some(best) = tones.iter().min_by_key(|x| (**x as i32 - k as i32).abs()) {
                        k = *best;
                        pos_idx = scale.iter().position(|x| *x == k).unwrap_or(pos_idx as usize) as i32;
                    }
                }
                let len = at(d * beat as f64).min(s3 - start);
                push(&mut t[t_lead].notes, start, len, k, 84 + mel.range(0, 16) as u8);
                if style == Style::Momento && mv >= 4 {
                    // slide up into the note at the top of a leap
                    for (dt, v) in [(0, -3000), (beat / 8, -1200), (beat / 4, 0)] {
                        t[t_lead].ctrls.push(Ctrl { tick: start + dt.min(len), kind: CtrlKind::Bend, value: v });
                    }
                }
                if *d >= if matches!(style, Style::Pursuit | Style::Momento) { 2.0 } else { 3.0 } {
                    // a slow vibrato swell on the long notes
                    for q in 0..=8 {
                        t[t_lead].ctrls.push(Ctrl { tick: start + len * q / 8, kind: CtrlKind::Cc(1), value: (q * 9) as i16 });
                    }
                    t[t_lead].ctrls.push(Ctrl { tick: start + len, kind: CtrlKind::Cc(1), value: 0 });
                }
                t0 += d * beat as f64;
            }
            phrase_start += phrase_bars * bar;
            phrase_no += 1;
        }
    }

    // ---------------------------------------------------------------- pluck ostinato
    let mut pl = Rng::stream(r.seed, "pluck");
    let penta: Vec<u8> = [0u8, 3, 5, 7, 10].iter().map(|x| (r.root + x) % 12).collect();
    let penta_keys: Vec<u8> = (60..=79u8).filter(|k| penta.contains(&(k % 12))).collect();
    let figure: Vec<usize> = (0..8).map(|_| pl.range(0, penta_keys.len() as i32 - 1) as usize).collect();
    if style == Style::Pursuit {
        arpeggio(&mut t[t_pluck].notes, &chords, &secs, &mut pl, beat, beat_secs, density, end);
    }
    if style == Style::Noir {
        // a slow arpeggio of the chord an octave up, in quavers, far off, that flickers:
        // a note missing here and there, from halfway through a swell and all through a
        // night shift
        for (part, a, z) in &secs {
            let from = match part {
                Part::Swell => a + (z - a) / 2 / bar * bar,
                Part::NightShift => *a,
                _ => continue,
            };
            let mut tick = from;
            let mut i = 0usize;
            while tick < *z {
                let c = chord_at(&chords, tick);
                let mut tones: Vec<u8> = c.voicing.iter().map(|k| k + 12).collect();
                tones.sort_unstable();
                let n = tones.len().max(1);
                let cyc = (2 * n - 2).max(1);
                let j = i % cyc;
                let idx = if j < n { j } else { cyc - j };
                if pl.chance(0.7 + density * 0.2) {
                    push(&mut t[t_pluck].notes, tick, beat / 2, tones[idx].min(100), 52 + pl.range(0, 12) as u8);
                }
                tick += beat / 2;
                i += 1;
            }
        }
    }
    if style == Style::Momento {
        // a kalimba in dotted quavers, three against the four, through each night shift
        for (s2, s3) in of(Part::NightShift) {
            let mut tick = s2;
            let mut i = 0;
            while tick < s3 {
                if pl.chance(0.6 + density * 0.35) {
                    push(&mut t[t_pluck].notes, tick, beat / 2, penta_keys[figure[i % 8]], 62 + pl.range(0, 14) as u8);
                }
                tick += beat * 3 / 4;
                i += 1;
            }
        }
    }
    for (s2, s3) in of(Part::NightShift).filter(|_| style == Style::Nocturne) {
        // from halfway, on a beat
        let from = s2 + ((s3 - s2) / 2) / beat * beat;
        let mut tick = from;
        let mut i = 0;
        while tick < s3 {
            if pl.chance(0.45 + density * 0.45) {
                push(&mut t[t_pluck].notes, tick, beat / 2, penta_keys[figure[i % 8]], 76 + pl.range(0, 20) as u8);
            }
            tick += beat / 2;
            i += 1;
        }
    }

    // ---------------------------------------------------------------- the answer
    let mut an = Rng::stream(r.seed, "answer");
    let tonic_hi = scale.iter().copied().filter(|k| k % 12 == r.root % 12).find(|k| *k >= 60).unwrap_or(62);
    for (s3, s4) in of(Part::Fade) {
        let mut tick = s3 + beat;
        let tonic = tonic_hi;
        // the last fade leaves its final bar to ring
        let stop = if s4 == end { s4.saturating_sub(bar) } else { s4 };
        for d in [2.0, 1.0, 3.0, 1.5, 4.5] {
            if tick >= stop {
                break;
            }
            let idx = scale.iter().position(|k| *k == tonic).unwrap_or(5) as i32 + an.range(-3, 3);
            let k = scale[idx.clamp(0, scale.len() as i32 - 1) as usize];
            let len = at(d * beat as f64);
            push(&mut t[t_answer].notes, tick, len.min(s4 - tick), k, 86);
            tick += len + if an.chance(0.3) { beat } else { 0 };
        }
    }

    // ---------------------------------------------------------------- the intro
    // the theme's opening heard alone, softly, from the second bar of each intro (its own
    // stream, so a piece without one is as it was)
    let mut ir = Rng::stream(r.seed, "intro");
    for (a, z) in of(Part::Intro) {
        if z - a < 2 * bar {
            continue;
        }
        let mut tick = a + bar;
        let stop = z.saturating_sub(beat);
        let mut idx = scale.iter().position(|k| *k >= from).unwrap_or(scale.len() / 2) as i32;
        for (ni, d) in rhythm_a.iter().take(4).enumerate() {
            if tick >= stop {
                break;
            }
            let c = chord_at(&chords, tick);
            idx = (idx + if ni == 0 { 0 } else { ir.range(-2, 2) }).clamp(0, scale.len() as i32 - 1);
            let mut k = scale[idx as usize];
            if ni == 0 || *d >= 2.0 {
                // the long ones land on the chord
                if let Some(best) = scale.iter().copied().filter(|x| c.pcs.contains(&(x % 12))).min_by_key(|x| (*x as i32 - k as i32).abs())
                {
                    k = best;
                    idx = scale.iter().position(|x| *x == k).unwrap_or(idx as usize) as i32;
                }
            }
            let len = at(d.max(1.0) * beat as f64).min(stop - tick);
            push(&mut t[t_answer].notes, tick, len, k, 70);
            tick += len;
        }
    }

    // ---------------------------------------------------------------- electric piano
    if let Some(tk) = t_keys.filter(|_| style == Style::Noir) {
        // the chord laid down softly as it changes and held, from the first section after
        // the intro; in swells and night shifts a quiet upper voice answers on the and of two
        let mut kr = Rng::stream(r.seed, "keys-part");
        for (ci, c) in chords.iter().enumerate() {
            let part = part_at(c.start);
            if part == Part::Intro {
                continue;
            }
            let len = if ci == last_chord { end - c.start } else { c.len.saturating_sub(beat / 2) };
            for (j, &k) in c.voicing.iter().enumerate() {
                push(&mut t[tk].notes, c.start + j as u32 * (beat / 16), len, k, 58 + kr.range(0, 8) as u8);
            }
            if matches!(part, Part::Swell | Part::NightShift) {
                let mut b = c.start;
                while b < c.start + c.len {
                    if kr.chance(0.5 + density * 0.3) && beat * 3 / 2 < bar {
                        let top = *c.voicing.last().unwrap_or(&64);
                        let k = if kr.chance(0.5) { top } else { c.voicing[c.voicing.len().saturating_sub(2)] };
                        push(&mut t[tk].notes, b + beat * 3 / 2, beat, k + 12, 46 + kr.range(0, 10) as u8);
                    }
                    b += bar;
                }
            }
        }
    }
    if let Some(tk) = t_keys.filter(|_| style == Style::Momento) {
        // a rolled chord on each bar, and with some seeds the next chord anticipated half
        // a beat early; through to the end, the last chord held
        let mut kr = Rng::stream(r.seed, "keys-part");
        let anticipate = kr.chance(0.5);
        for (ci, c) in chords.iter().enumerate() {
            let mut b = c.start;
            while b < c.start + c.len {
                let first = b == c.start;
                let at_ = if anticipate && first && ci > 0 { b - beat / 2 } else { b };
                let len = if ci == last_chord && b + bar >= c.start + c.len { end - b } else { bar - beat / 4 };
                for (j, &k) in c.voicing.iter().enumerate() {
                    let vel = if first { 70 } else { 56 } + kr.range(0, 8) as u8;
                    push(&mut t[tk].notes, at_ + j as u32 * (beat / 24), len, k, vel);
                }
                b += bar;
            }
        }
    }

    // ---------------------------------------------------------------- the drone
    // the tonic and its fifth, struck again at each section so it swells with the form
    let tonic = near(r.root, 45);
    for (part, a, z) in &secs {
        push(&mut t[t_drone].notes, *a, z - a, tonic, 80);
        if !matches!(part, Part::Neon | Part::Intro) || style == Style::Blackout {
            push(&mut t[t_drone].notes, *a, z - a, tonic + 7, 64);
        }
        if style == Style::Noir && *part == Part::NightShift {
            // unease: a faint tritone above, held under it all
            push(&mut t[t_drone].notes, *a, z - a, tonic + 18, 34);
        }
    }

    // ---------------------------------------------------------------- the city
    let mut city = Rng::stream(r.seed, "city");
    // the rain, from one of the recordings, and sometimes a second as the piece recedes
    // pitter-patter you can count drops in, rarely a heavier wash
    let main = RainKind::ALL[city.weighted(&[3.0, 2.0, 2.0, 2.0, 1.5, 1.0, 0.3, 0.3, 0.3])];
    push(&mut t[t_city].notes, 0, end, main.key(), 80 + city.range(0, 16) as u8);
    if city.chance(0.35) {
        let others: Vec<RainKind> = RainKind::ALL.into_iter().filter(|k| k.sparse() && *k != main).collect();
        let second = *city.pick(&others);
        for (a, z) in of(Part::Fade) {
            push(&mut t[t_city].notes, a, z - a, second.key(), 76);
        }
    }
    push(&mut t[t_city].notes, 0, end, fx::HUM, 55);
    if city.chance(0.75) {
        // the neon buzzes through each run of swells and night shifts
        let mut run: Option<(u32, u32)> = None;
        for (part, a, z) in secs.iter().copied().chain([(Part::Fade, end, end)]) {
            if matches!(part, Part::Swell | Part::NightShift) {
                run = Some((run.map_or(a, |r| r.0), z));
            } else if let Some((ra, rz)) = run.take() {
                push(&mut t[t_city].notes, ra, rz - ra, fx::NEON, 50);
            }
        }
    }
    push(&mut t[t_city].notes, 2 * beat, beat, fx::THUNDER, 90);
    let (thunder, braam) = (city.chance(0.7), city.chance(0.6));
    let braam_key = near(r.root, fx::BRAAM as i32 + 6).clamp(fx::BRAAM, fx::BRAAM + 11);
    for (i, (part, a, _)) in secs.iter().enumerate() {
        // into each night shift (from something else): thunder just before, a braam on it
        if *part != Part::NightShift || (i > 0 && secs[i - 1].0 == Part::NightShift) {
            continue;
        }
        if thunder && *a > 2 * beat {
            push(&mut t[t_city].notes, a - beat, beat, fx::THUNDER, 110);
        }
        if braam {
            push(&mut t[t_city].notes, *a, beat, braam_key, 90);
        }
    }

    // the depths: drones from the kit, booms, metal and grains (Blackout); a riser into
    // each night shift (Pursuit)
    let mut deep = Rng::stream(r.seed, "depths");
    match style {
        Style::Blackout => {
            let pc = r.root % 12;
            push(&mut t[t_city].notes, 0, end, fx::ANALOG_DRONE + pc, 70);
            for (i, (part, a, z)) in secs.iter().enumerate() {
                if !matches!(part, Part::Neon | Part::Intro) {
                    push(&mut t[t_city].notes, *a, z - a, fx::DRONE + pc, 62);
                }
                if i > 0 && *part != secs[i - 1].0 {
                    push(&mut t[t_city].notes, *a, beat * 2, fx::BOOM + pc, 78);
                }
                if *part == Part::NightShift && *a >= bar {
                    push(&mut t[t_city].notes, a - bar, bar, fx::REVERSE_LONG, 70);
                }
                let mut b = *a;
                while b < *z {
                    match part {
                        Part::NightShift | Part::Swell if deep.chance(0.2 + density * 0.2) => push(
                            &mut t[t_city].notes,
                            b + beat * deep.range(0, beats_per_bar as i32 - 1) as u32,
                            bar,
                            fx::METAL,
                            60 + deep.range(0, 25) as u8,
                        ),
                        Part::Fade | Part::Neon if deep.chance(0.15) => push(&mut t[t_city].notes, b, bar * 2, fx::GRAINS + pc, 55),
                        _ => {}
                    }
                    b += bar;
                }
            }
        }
        Style::Pursuit => {
            for (i, (part, a, _)) in secs.iter().enumerate() {
                if *part == Part::NightShift && (i == 0 || secs[i - 1].0 != Part::NightShift) && *a >= bar {
                    push(&mut t[t_city].notes, a - bar, bar, fx::RISER, 80);
                }
            }
        }
        Style::Nocturne | Style::Momento | Style::Noir => {}
    }
    if style == Style::Noir {
        // told in music alone: no rain, hum, neon or thunder
        t[t_city].notes.clear();
    }

    // ---------------------------------------------------------------- drums
    let mut dr = Rng::stream(r.seed, "drums");
    match style {
        Style::Pursuit => driven_drums(&mut t[t_drums].notes, &secs, &mut dr, beat, bar, beats_per_bar, density, end),
        Style::Blackout => {
            // a slow, deep pulse from the swells on, taiko at the night shift's phrases
            for (part, a, z) in &secs {
                let mut b = *a;
                while b < *z {
                    match part {
                        Part::Swell | Part::Fade if (b - a) / bar % 2 == 0 => push(&mut t[t_drums].notes, b, beat, drum::DEEP_KICK, 90),
                        Part::NightShift => {
                            push(&mut t[t_drums].notes, b, beat, drum::DEEP_KICK, 90);
                            if (b - a) / bar % 2 == 0 {
                                push(&mut t[t_drums].notes, b, beat, drum::TOM_LOW, 80);
                            } else if dr.chance(0.5 + density * 0.4) && beats_per_bar >= 3 {
                                push(&mut t[t_drums].notes, b + beat * (beats_per_bar / 2), beat / 2, drum::GATED_SNARE, 52);
                            }
                        }
                        _ => {}
                    }
                    b += bar;
                }
            }
        }
        Style::Momento => soft_groove(&mut t[t_drums].notes, &secs, &mut dr, beat, bar, beats_per_bar, density, end),
        Style::Noir => brushes(&mut t[t_drums].notes, &secs, &mut dr, beat, bar, beats_per_bar, density, end),
        Style::Nocturne => {}
    }
    if style == Style::Nocturne && density > 0.2 {
        // a heartbeat from halfway through a swell, and all through a night shift, where
        // the snare and ticks join it
        let mut b = 0;
        while b < end {
            let beating = secs.iter().any(|(part, a, z)| match part {
                Part::Swell => b >= a + ((z - a) / 2) / bar * bar && b < *z,
                Part::NightShift => b >= *a && b < *z,
                _ => false,
            });
            if !beating {
                b += bar;
                continue;
            }
            push(&mut t[t_drums].notes, b, beat / 2, drum::HEARTBEAT, 100);
            push(&mut t[t_drums].notes, b + beat / 2, beat / 2, drum::HEARTBEAT, 70);
            if part_at(b) == Part::NightShift {
                if beats_per_bar >= 3 && dr.chance(0.85) {
                    push(&mut t[t_drums].notes, b + beat * (beats_per_bar / 2), beat / 2, drum::GATED_SNARE, 80);
                }
                for k in 0..beats_per_bar {
                    if dr.chance(density) {
                        push(&mut t[t_drums].notes, b + k * beat + beat / 2, beat / 4, drum::TICK, 40);
                    }
                }
            }
            b += bar;
        }
    }

    for tr in p.tracks.iter_mut() {
        tr.sort();
    }
    // the tracks the piece leaves silent go
    p.tracks.retain(|tr| !tr.notes.is_empty());
    p.recipe = Some(*r);
    p.composed = fingerprint(&p);
    p
}

/// Noir's bass, an upright walking slowly: the root on the one, a chord tone on the
/// three, and on a chord's last beat a chromatic step into the next; from the swell to
/// halfway through each fade, the swell in half notes alone.
#[allow(clippy::too_many_arguments)]
fn walking_bass(
    out: &mut Vec<Note>,
    chords: &[Chord],
    secs: &[(Part, u32, u32)],
    rng: &mut Rng,
    beat: u32,
    bar: u32,
    beats: u32,
    end: u32,
) {
    let mut put = |start: u32, len: u32, key: u8, vel: u8| {
        if start < end && len > 0 {
            out.push(Note { start, len: len.min(end - start), key, vel });
        }
    };
    let half = (beats / 2).max(1) * beat;
    for (part, a, z) in secs {
        let to = match part {
            Part::Swell | Part::NightShift => *z,
            Part::Fade => a + (z - a) / 2 / bar * bar,
            Part::Neon | Part::Intro => continue,
        };
        let mut b = *a;
        while b < to {
            let c = chord_at(chords, b);
            let root = near(c.root_pc, 40);
            put(b, half - beat / 8, root, 88);
            if half < bar {
                let tone = c.pcs.get(if rng.chance(0.5) { 2 } else { 1 }).copied().unwrap_or(c.root_pc);
                put(b + half, half - beat / 8, near(tone, 43), 70);
            }
            // walking into the next chord in a night shift: the last beat a semitone off
            // its root, from above or below
            let next = chords.iter().find(|n| n.start == c.start + c.len);
            if let Some(n) = next
                && *part == Part::NightShift
                && b + bar == n.start
                && beats >= 3
            {
                let target = near(n.root_pc, 40) as i32;
                let k = if rng.chance(0.5) { target - 1 } else { target + 1 };
                put(b + (beats - 1) * beat, beat - beat / 8, k.clamp(24, 60) as u8, 66);
            }
            b += bar;
        }
    }
}

/// Noir's brushes (on the GM brush kit): a slow swirl through each bar, taps on the
/// backbeat and a soft ride, all through a night shift; the swirl alone from halfway
/// through a swell and into a fade.
#[allow(clippy::too_many_arguments)]
fn brushes(out: &mut Vec<Note>, secs: &[(Part, u32, u32)], rng: &mut Rng, beat: u32, bar: u32, beats: u32, density: f64, end: u32) {
    // the GM brush kit's keys
    const KICK: u8 = 36;
    const TAP: u8 = 38;
    const SWIRL: u8 = 40;
    const RIDE: u8 = 51;
    let swing = beat / 6;
    let mut put = |start: u32, len: u32, key: u8, vel: u8| {
        if start < end {
            out.push(Note { start, len: len.min(end - start).max(1), key, vel });
        }
    };
    for (part, a, z) in secs {
        let mut b = *a;
        while b < *z {
            let (swirl, kit) = match part {
                Part::Swell => (b >= a + (z - a) / 2 / bar * bar, false),
                Part::NightShift => (true, true),
                Part::Fade => (b < a + (z - a) / 2, false),
                Part::Neon | Part::Intro => (false, false),
            };
            if swirl {
                put(b, bar, SWIRL, 44);
            }
            if kit {
                put(b, beat / 2, KICK, 46);
                for k in 0..beats {
                    if k % 2 == 1 {
                        put(b + k * beat, beat / 2, TAP, 54 + rng.range(0, 8) as u8);
                    }
                    put(b + k * beat, beat / 2, RIDE, 38 + rng.range(0, 8) as u8);
                    if rng.chance(0.25 + density * 0.35) {
                        put(b + k * beat + beat / 2 + swing, beat / 4, RIDE, 30);
                    }
                }
            }
            b += bar;
        }
    }
}

/// Whether a sound is one a style is written for.
fn prefers(style: Style, role: Role, label: &str) -> bool {
    let liked: &[&str] = match (style, role) {
        (Style::Pursuit, Role::Brass) => &["Chrome Fanfare", "Neon Brass", "Tidal Brass"],
        (Style::Pursuit, Role::Pad | Role::Strings) => &["Neon Mist", "Rain Pad", "Orbital Haze", "Silk Strings"],
        (Style::Pursuit, Role::Lead) => &["Wide Saw Lead", "Glass Lead", "Smoky Lead"],
        (Style::Pursuit, Role::Answer) => &["Glass Lead", "Lonely Horn"],
        (Style::Pursuit, Role::Bass) => &["Night Drive Bass", "Rubber Bass", "Sub Pulse Bass"],
        (Style::Pursuit, Role::Bells) => &["Ring Bells", "Glass Chimes"],
        (Style::Pursuit, Role::Pluck) => &["Lantern Pluck", "Rain Harp"],
        (Style::Blackout, Role::Brass) => &["Velvet Horns", "Ember Horns", "Tidal Brass"],
        (Style::Blackout, Role::Pad | Role::Strings) => &["Cathedral Sweep", "Night Choir", "Neon Mist", "Low Cloud Strings"],
        (Style::Blackout, Role::Lead) => &["Lonely Horn", "Smoky Lead"],
        (Style::Blackout, Role::Answer) => &["Lonely Horn", "Shakuhachi"],
        (Style::Blackout, Role::Bass) => &["Pure Sub", "Undercity Bass"],
        (Style::Blackout, Role::Bells) => &["Ring Bells", "Night Bells"],
        (Style::Blackout, Role::Drone) => &["Abyss Drone", "Sodium Drone", "Choir Drone"],
        (Style::Momento, Role::Brass) => &["Velvet Horns", "Ember Horns", "French Horns"],
        (Style::Momento, Role::Pad | Role::Strings) => &["Neon Mist", "Rain Pad", "Warm Pad", "Halo Pad", "Silk Strings", "Memory Strings"],
        (Style::Momento, Role::Lead) => &["Tenor Sax (FreePats)", "Lonely Horn"],
        (Style::Momento, Role::Answer) => &["Lonely Horn", "Pan Flute", "Oboe", "Tenor Sax (FreePats)"],
        (Style::Momento, Role::Bass) => &["Fretless", "Sub Pulse Bass", "Pure Sub"],
        (Style::Momento, Role::Bells) => &["Celesta Glow", "Glass Chimes", "Music Box"],
        (Style::Momento, Role::Pluck) => &["Kalimba", "Koto", "Lantern Pluck"],
        (Style::Momento, Role::Drone) => &["Choir Drone", "Glass Drone"],
        (Style::Momento, Role::Keys) => &["Neon Tine Piano", "Electric Piano", "FM Piano"],
        (Style::Noir, Role::Brass) => &["Velvet Horns", "Skyline Swell", "French Horns"],
        (Style::Noir, Role::Pad | Role::Strings) => &["Sweep Pad (FreePats)", "Neon Mist", "Orbital Haze", "Warm Pad", "Memory Strings"],
        (Style::Noir, Role::Lead) => &["Tenor Sax (FreePats)", "Alto Sax", "Lonely Horn", "Smoky Lead"],
        (Style::Noir, Role::Answer) => &["Lonely Horn", "Tenor Sax (FreePats)", "Shakuhachi"],
        (Style::Noir, Role::Bass) => &["Upright Bass", "Fretless"],
        (Style::Noir, Role::Bells) => &["Rain Vibes", "Vibraphone", "Celesta Glow"],
        (Style::Noir, Role::Pluck) => &["Lantern Pluck", "Rain Harp"],
        (Style::Noir, Role::Drone) => &["Sodium Drone", "Glass Drone", "Abyss Drone"],
        (Style::Noir, Role::Keys) => &["Neon Tine Piano", "Electric Piano", "Polysynth Keys"],
        _ => &[],
    };
    liked.is_empty() || liked.contains(&label)
}

/// The pulse that drives a Pursuit: the chord's root in quavers (semiquavers when the
/// beat is slow), with octave and fifth jumps from a pattern the seed picks, from the
/// first swell to halfway through each fade, and under the last stretch of a Neon.
#[allow(clippy::too_many_arguments)]
fn driven_bass(
    out: &mut Vec<Note>,
    chords: &[Chord],
    secs: &[(Part, u32, u32)],
    rng: &mut Rng,
    beat: u32,
    bar: u32,
    beat_secs: f64,
    end: u32,
) {
    // 0 root, 1 octave up, 2 fifth
    const PATTERNS: [[u8; 8]; 5] =
        [[0, 0, 0, 0, 0, 0, 0, 0], [0, 0, 1, 0, 0, 1, 0, 1], [0, 0, 0, 2, 0, 0, 0, 2], [0, 1, 0, 1, 0, 1, 0, 1], [0, 0, 2, 0, 1, 0, 2, 0]];
    let pattern = *rng.pick(&PATTERNS);
    let step = if beat_secs > 0.7 { beat / 4 } else { beat / 2 }.max(1);
    for (part, a, z) in secs {
        let (from, to) = match part {
            Part::Swell | Part::NightShift => (*a, *z),
            Part::Fade => (*a, a + (z - a) / 2 / bar * bar),
            Part::Neon => (a + (z - a) * 3 / 4 / bar * bar, *z),
            Part::Intro => continue,
        };
        let mut tick = from;
        let mut i = 0usize;
        while tick < to.min(end) {
            let c = chord_at(chords, tick);
            let root = near(c.root_pc, 40);
            let k = match pattern[i % 8] {
                1 => root + 12,
                2 => near((c.root_pc + 7) % 12, 47),
                _ => root,
            };
            let accent = (tick - from).is_multiple_of(beat);
            let vel = if *part == Part::Neon {
                70
            } else if accent {
                104
            } else {
                84
            };
            out.push(Note { start: tick, len: (step * 3 / 4).max(1).min(end - tick), key: k, vel });
            tick += step;
            i += 1;
        }
    }
}

/// The arpeggio over a Pursuit: the chord's tones an octave up, in semiquavers (quavers
/// when the beat is quick), rising, rising and falling, or broken, quietly from three
/// quarters of the way through a Neon, all through the swells and night shifts, and
/// into the first quarter of a fade.
#[allow(clippy::too_many_arguments)]
fn arpeggio(
    out: &mut Vec<Note>,
    chords: &[Chord],
    secs: &[(Part, u32, u32)],
    rng: &mut Rng,
    beat: u32,
    beat_secs: f64,
    density: f64,
    end: u32,
) {
    let shape = rng.range(0, 2);
    let step = if beat_secs > 0.5 { beat / 4 } else { beat / 2 }.max(1);
    for (part, a, z) in secs {
        let (from, to, vel) = match part {
            Part::Neon => (a + (z - a) * 3 / 4 / beat * beat, *z, 54),
            Part::Swell => (*a, *z, 74),
            Part::NightShift => (*a, *z, 84),
            Part::Fade => (*a, a + (z - a) / 4 / beat * beat, 60),
            // the sequence alone, quietly, before it all starts
            Part::Intro => (*a, *z, 46),
        };
        let mut tick = from;
        let mut i = 0usize;
        while tick < to.min(end) {
            let c = chord_at(chords, tick);
            let mut tones: Vec<u8> = c.voicing.iter().map(|k| k + 12).collect();
            tones.sort_unstable();
            let n = tones.len().max(1);
            let idx = match shape {
                0 => i % n,
                1 => {
                    let cyc = 2 * n - 2;
                    let j = i % cyc.max(1);
                    if j < n { j } else { cyc - j }
                }
                _ => [0, 2, 1, 3, 2, 0, 3, 1][i % 8] % n,
            };
            // the density thins the quiet ones off the beat
            if (tick - from).is_multiple_of(beat) || rng.chance(0.55 + density * 0.45) {
                let accent = if (tick - from).is_multiple_of(beat) { 10 } else { 0 };
                out.push(Note { start: tick, len: step.min(end - tick), key: tones[idx].min(108), vel: vel + accent });
            }
            tick += step;
            i += 1;
        }
    }
}

/// A Pursuit's drums: a kick on every beat as the swell builds; in a night shift the
/// kick, a gated backbeat, quaver hats and a taiko fill to end every four bars, with a
/// crash on its first beat; a heartbeat into the fade.
#[allow(clippy::too_many_arguments)]
fn driven_drums(out: &mut Vec<Note>, secs: &[(Part, u32, u32)], rng: &mut Rng, beat: u32, bar: u32, beats: u32, density: f64, end: u32) {
    let mut put = |start: u32, len: u32, key: u8, vel: u8| {
        if start < end {
            out.push(Note { start, len: len.min(end - start).max(1), key, vel });
        }
    };
    for (i, (part, a, z)) in secs.iter().enumerate() {
        let mut b = *a;
        while b < *z {
            let nth = (b - a) / bar;
            match part {
                Part::Swell => {
                    for k in 0..beats {
                        put(b + k * beat, beat / 2, drum::DEEP_KICK, if k == 0 { 100 } else { 84 });
                    }
                    if b >= a + (z - a) / 2 {
                        for k in 0..beats {
                            if rng.chance(0.4 + density * 0.5) {
                                put(b + k * beat + beat / 2, beat / 4, drum::HAT, 50);
                            }
                        }
                    }
                }
                Part::NightShift => {
                    if nth == 0 && (i == 0 || secs[i - 1].0 != Part::NightShift) {
                        put(b, beat * 2, drum::CRASH, 76);
                    }
                    let fill = nth % 4 == 3 && b + bar <= *z;
                    for k in 0..beats {
                        put(b + k * beat, beat / 2, drum::KICK, if k == 0 { 108 } else { 92 });
                        if beats >= 3 && (k == beats / 2 || (beats == 4 && k == 1) || k == beats - 1) && k % 2 == 1 {
                            put(b + k * beat, beat / 2, drum::GATED_SNARE, 96);
                        }
                        if !(fill && k == beats - 1) {
                            put(b + k * beat + beat / 2, beat / 4, drum::HAT, 56 + rng.range(0, 14) as u8);
                            if density > 0.5 && rng.chance(density - 0.4) {
                                put(b + k * beat + beat / 4, beat / 4, drum::HAT, 36);
                            }
                        }
                    }
                    if fill {
                        // a taiko run down across the last beat
                        let last = b + (beats - 1) * beat;
                        for (j, key) in [drum::TOM_HIGH, drum::TOM_MID, drum::TOM_MID, drum::TOM_LOW].into_iter().enumerate() {
                            put(last + j as u32 * beat / 4, beat / 4, key, 90 + j as u8 * 4);
                        }
                    }
                }
                Part::Fade if b < a + (z - a) / 2 => {
                    put(b, beat / 2, drum::HEARTBEAT, 96);
                    put(b + beat / 2, beat / 2, drum::HEARTBEAT, 68);
                }
                _ => {}
            }
            b += bar;
        }
    }
}

/// Momento's bass: the root held into the and of two, the fifth or octave answering it
/// swung, and a step up into the next chord at the end of its last bar; from the swell
/// to halfway through each fade.
fn swaying_bass(out: &mut Vec<Note>, chords: &[Chord], secs: &[(Part, u32, u32)], beat: u32, bar: u32, beats: u32, end: u32) {
    let swing = beat / 6;
    let mut put = |start: u32, len: u32, key: u8, vel: u8| {
        if start < end && len > 0 {
            out.push(Note { start, len: len.min(end - start), key, vel });
        }
    };
    for (part, a, z) in secs {
        let to = match part {
            Part::Swell | Part::NightShift => *z,
            Part::Fade => a + (z - a) / 2 / bar * bar,
            Part::Neon | Part::Intro => continue,
        };
        let mut b = *a;
        while b < to {
            let c = chord_at(chords, b);
            let root = near(c.root_pc, 43);
            put(b, beat * 3 / 2, root, 92);
            let answer = if ((b - c.start) / bar).is_multiple_of(2) { near((c.root_pc + 7) % 12, 50) } else { root + 12 };
            if beat * 3 / 2 + swing < bar {
                put(b + beat * 3 / 2 + swing, beat / 2, answer, 70);
            }
            // into the next chord, a semitone below its root
            let next = chords.iter().find(|n| n.start == c.start + c.len);
            if let Some(n) = next
                && b + bar == n.start
                && beats >= 3
            {
                put(b + (beats - 1) * beat + beat / 2 + swing, beat / 3, near(n.root_pc, 43) - 1, 64);
            }
            b += bar;
        }
    }
}

/// Momento's groove, soft and swung: a muted kick on the one and the and of two, a
/// half-time snare, a shaker in swung quavers and tabla now and then, from halfway
/// through a swell and all through a night shift; the shaker alone into a fade.
#[allow(clippy::too_many_arguments)]
fn soft_groove(out: &mut Vec<Note>, secs: &[(Part, u32, u32)], rng: &mut Rng, beat: u32, bar: u32, beats: u32, density: f64, end: u32) {
    let swing = beat / 6;
    let mut put = |start: u32, len: u32, key: u8, vel: u8| {
        if start < end {
            out.push(Note { start, len: len.min(end - start).max(1), key, vel });
        }
    };
    for (part, a, z) in secs {
        let mut b = *a;
        while b < *z {
            let (groove, shaker) = match part {
                Part::Swell => (b >= a + (z - a) / 2 / bar * bar, b >= a + (z - a) / 2 / bar * bar),
                Part::NightShift => (true, true),
                Part::Fade => (false, b < a + (z - a) / 2),
                Part::Neon | Part::Intro => (false, false),
            };
            if groove {
                put(b, beat / 2, drum::DEEP_KICK, 88);
                if beat * 3 / 2 < bar {
                    put(b + beat * 3 / 2 + swing, beat / 2, drum::DEEP_KICK, 66);
                }
                if beats >= 3 {
                    put(b + beat * (beats / 2), beat / 2, drum::SNARE, 58);
                }
                if *part == Part::NightShift && rng.chance(0.3 + density * 0.3) {
                    // tabla in dotted quavers across the bar's second half
                    let mut t = b + bar / 2;
                    while t < b + bar {
                        put(t, beat / 4, if (t - b).is_multiple_of(beat) { drum::TABLA_LOW } else { drum::TABLA_HIGH }, 50);
                        t += beat * 3 / 4;
                    }
                }
            }
            if shaker {
                for k in 0..beats {
                    put(b + k * beat, beat / 4, drum::SHAKER, 40);
                    if rng.chance(0.5 + density * 0.5) {
                        put(b + k * beat + beat / 2 + swing, beat / 4, drum::SHAKER, 54);
                    }
                }
            }
            b += bar;
        }
    }
}

/// Give a freshly composed piece the bar levels of the matching tracks (by name) of the
/// one it replaces: they're the mix's shape, set by hand, and outlast a new seed.
pub fn keep_levels(new: &mut Project, old: &Project) {
    for t in new.tracks.iter_mut() {
        if let Some(o) = old.tracks.iter().find(|o| o.name == t.name) {
            t.levels = o.levels.clone();
        }
    }
}

/// Give a freshly composed piece the sounds, mix and colours of the matching tracks
/// (by name) of the one it replaces.
pub fn keep_sounds(new: &mut Project, old: &Project) {
    for t in new.tracks.iter_mut() {
        if let Some(o) = old.tracks.iter().find(|o| o.name == t.name) {
            t.source = o.source.clone();
            t.strip = o.strip;
            t.color = o.color;
        }
    }
    new.fx = old.fx;
    new.master_db = old.master_db;
    new.title = old.title.clone();
}

/// Lengthen or shorten a project that isn't composed: notes held to the old end are
/// held to the new one, notes past the new end go, and those across it are cut.
pub fn resize(p: &mut Project, old_bars: u32, new_bars: u32) {
    let bar = p.bar_ticks();
    let (old_end, new_end) = (old_bars * bar, new_bars * bar);
    for t in p.tracks.iter_mut() {
        t.notes.retain(|n| n.start < new_end);
        for n in t.notes.iter_mut() {
            if n.end() >= old_end.saturating_sub(bar / 4) || n.end() > new_end {
                n.len = new_end - n.start;
            }
        }
        t.ctrls.retain(|c| c.tick < new_end);
        t.levels.retain(|l| l.bar < new_bars);
    }
    p.sections.retain(|s| s.bar < new_bars);
    if let Some((a, b)) = p.loop_bars {
        p.loop_bars = (a < new_bars).then_some((a, b.min(new_bars)));
    }
    p.length_bars = new_bars;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::FontBank;
    use crate::timeline::Timeline;

    fn fonts() -> FontBank {
        let mut f = FontBank::default();
        f.load_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soundfonts"));
        f
    }

    #[test]
    fn same_recipe_same_piece() {
        let r = Recipe { seed: 42, ..Recipe::default() };
        assert_eq!(compose(&r, 70.0, (4, 4), 32), compose(&r, 70.0, (4, 4), 32));
        assert_ne!(compose(&r, 70.0, (4, 4), 32), compose(&Recipe { seed: 43, ..r }, 70.0, (4, 4), 32));
    }

    /// Many seeds, tempos, metres and lengths: the piece fills its length exactly, its
    /// notes stay in range, the rain falls throughout and every sound is bundled.
    #[test]
    fn fits_any_tempo_metre_and_length() {
        let f = fonts();
        for seed in 0..40u64 {
            let r = Recipe::random(seed);
            for (bpm, meter, bars) in [(52.0, (4, 4), 8), (66.0, (4, 4), 24), (90.0, (3, 4), 40), (128.0, (6, 8), 17), (75.0, (5, 4), 3)] {
                let p = compose(&r, bpm, meter, bars);
                assert_eq!(p.bars(), bars.max(2), "seed {seed} {bpm} {meter:?} {bars}");
                let city = p.tracks.iter().find(|t| t.name == "City").expect("city");
                assert!(city.notes.iter().any(|n| RainKind::of_key(n.key).is_some() && n.start == 0 && n.end() == p.end_tick()));
                for t in &p.tracks {
                    assert!(t.notes.iter().all(|n| n.end() <= p.end_tick() && n.len > 0 && n.key <= 120), "{}", t.name);
                    if let Source::SoundFont { font, bank, program, name } = &t.source {
                        assert!(f.get(font).and_then(|lf| lf.find(*bank, *program)).is_some(), "{name} not in {font}");
                    }
                }
                assert!(!Timeline::from_project(&p).events.is_empty());
                assert_eq!(fingerprint(&p), p.composed);
            }
        }
    }

    /// Every style fills any length at any tempo and metre with bundled sounds.
    #[test]
    fn every_style_fits_any_tempo_metre_and_length() {
        let f = fonts();
        for style in [Style::Pursuit, Style::Blackout, Style::Momento, Style::Noir] {
            for seed in 0..15u64 {
                let r = Recipe { style, ..Recipe::random(seed) };
                for (bpm, meter, bars) in
                    [(56.0, (4, 4), 8), (104.0, (4, 4), 24), (90.0, (3, 4), 40), (128.0, (6, 8), 17), (75.0, (5, 4), 3)]
                {
                    let p = compose(&r, bpm, meter, bars);
                    assert_eq!(p.bars(), bars.max(2), "{style:?} seed {seed} {bpm} {meter:?} {bars}");
                    for t in &p.tracks {
                        assert!(t.notes.iter().all(|n| n.end() <= p.end_tick() && n.len > 0 && n.key <= 120), "{}", t.name);
                        if let Source::SoundFont { font, bank, program, name } = &t.source {
                            assert!(f.get(font).and_then(|lf| lf.find(*bank, *program)).is_some(), "{name} not in {font}");
                        }
                    }
                    assert_eq!(fingerprint(&p), p.composed);
                    assert_eq!(compose(&r, bpm, meter, bars), p);
                }
            }
        }
    }

    /// Each style writes its own way: Pursuit drives (a running bass, an arpeggio, a
    /// backbeat), Blackout broods (kit drones throughout, no plucks, slow chords).
    #[test]
    fn styles_sound_like_themselves() {
        use Part::*;
        let plan = [(Neon, 4), (Swell, 4), (NightShift, 8), (Fade, 4)];
        for seed in 0..10 {
            let r = Recipe { seed, style: Style::Pursuit, ..Recipe::default() };
            let p = compose_plan(&r, 104.0, (4, 4), &plan);
            let bar = p.bar_ticks();
            let (ns, ne) = (8 * bar, 16 * bar);
            let in_shift = |name: &str, key: Option<u8>| {
                p.tracks
                    .iter()
                    .find(|t| t.name == name)
                    .map_or(0, |t| t.notes.iter().filter(|n| n.start >= ns && n.start < ne && key.is_none_or(|k| n.key == k)).count())
            };
            assert!(in_shift("Bass", None) >= 8 * 8, "a running bass: {}", in_shift("Bass", None));
            assert!(in_shift("Pluck", None) >= 8 * 8, "an arpeggio");
            assert!(in_shift("Drums", Some(drum::GATED_SNARE)) >= 8 * 2, "a backbeat");
            assert!(in_shift("Drums", Some(drum::KICK)) >= 8 * 4, "four to the bar");

            let r = Recipe { seed, style: Style::Blackout, ..Recipe::default() };
            let p = compose_plan(&r, 56.0, (4, 4), &plan);
            assert!(p.tracks.iter().all(|t| t.name != "Pluck"));
            let city = p.tracks.iter().find(|t| t.name == "City").unwrap();
            assert!(
                city.notes
                    .iter()
                    .any(|n| (fx::ANALOG_DRONE..fx::ANALOG_DRONE + 12).contains(&n.key) && n.start == 0 && n.end() == p.end_tick())
            );
            let pad = p.tracks.iter().find(|t| t.name == "Pad").unwrap();
            assert!(pad.notes.iter().all(|n| n.key < 64), "low voicings");

            // Momento: an electric piano throughout, a theme that slides into its leaps,
            // and a kalimba three against four
            let r = Recipe { seed, style: Style::Momento, ..Recipe::default() };
            let m = compose_plan(&r, 72.0, (4, 4), &plan);
            let keys = m.tracks.iter().find(|t| t.name == "Keys").expect("an electric piano");
            assert!(keys.notes.iter().any(|n| n.start < 4 * bar) && keys.notes.iter().any(|n| n.end() == m.end_tick()));
            let lead = m.tracks.iter().find(|t| t.name == "Lead").unwrap();
            assert!(lead.notes.iter().all(|n| n.start >= ns && n.start < ne));
            let pluck = m.tracks.iter().find(|t| t.name == "Pluck").unwrap();
            assert!(pluck.notes.iter().all(|n| (n.start - ns) % (m.beat_ticks() * 3 / 4) == 0));
            let drums = m.tracks.iter().find(|t| t.name == "Drums").unwrap();
            assert!(drums.notes.iter().any(|n| n.key == drum::SHAKER) && drums.notes.iter().all(|n| n.start >= 6 * bar));
            let mut starts: Vec<u32> = pad.notes.iter().map(|n| n.start).collect();
            starts.dedup();
            assert!(p.tick_secs(starts[1] - starts[0]) >= 8.0, "slow chords");
        }
    }

    /// Noir: music alone (no rain or city sounds at all), opening on an intro, with the
    /// electric piano, a walking bass, brushes, and a lead that rests every other phrase;
    /// any style plays an intro.
    #[test]
    fn noir_is_music_alone() {
        for seed in 0..12 {
            let r = Recipe { seed, style: Style::Noir, ..Recipe::random(seed) };
            for (bpm, meter, bars) in [(64.0, (4, 4), 32), (58.0, (3, 4), 20), (80.0, (4, 4), 64)] {
                let p = compose(&r, bpm, meter, bars);
                assert!(p.tracks.iter().all(|t| t.role != Some(Role::City) && !matches!(t.source, Source::Kit(_))), "seed {seed}");
                assert_eq!(p.plan().unwrap()[0].0, Part::Intro);
                for role in [Role::Keys, Role::Bass, Role::Drums, Role::Lead] {
                    assert!(p.tracks.iter().any(|t| t.role == Some(role)), "{role:?} in seed {seed}");
                }
                let drums = p.tracks.iter().find(|t| t.name == "Drums").unwrap();
                assert!(matches!(&drums.source, Source::SoundFont { bank: 128, program: 40, .. }), "brushes");
                // the lead leaves whole phrases empty
                let lead = p.tracks.iter().find(|t| t.name == "Lead").unwrap();
                let bar = p.bar_ticks();
                let played: std::collections::BTreeSet<u32> = lead.notes.iter().map(|n| n.start / bar).collect();
                let (a, z) = p
                    .plan()
                    .unwrap()
                    .iter()
                    .fold((0, None), |(at, found), (part, n)| (at + n, found.or((*part == Part::NightShift).then_some((at, at + n)))))
                    .1
                    .unwrap();
                assert!(((a..z).count() as f64) > played.len() as f64 * 1.3, "seed {seed}: {played:?} in {a}..{z}");
            }
        }
        for style in Style::ALL {
            let plan = [(Part::Intro, 4), (Part::Neon, 4), (Part::NightShift, 8), (Part::Fade, 4)];
            let p = compose_plan(&Recipe { seed: 5, style, ..Recipe::default() }, style.tempo(), (4, 4), &plan);
            assert_eq!(p.bars(), 20);
            assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start < 4 * p.bar_ticks())), "{style:?}");
        }
    }

    /// New arrangements: the length exactly, a quiet start, a night shift, a fade to end,
    /// no kind twice running, and different ones for different seeds.
    #[test]
    fn random_arrangements_are_sound() {
        let mut seen = std::collections::BTreeSet::new();
        for style in Style::ALL {
            for seed in 0..60u64 {
                for (bpm, meter, bars) in
                    [(66.0, (4, 4), 24), (104.0, (4, 4), 40), (56.0, (3, 4), 16), (72.0, (6, 8), 9), (90.0, (4, 4), 3)]
                {
                    let r = Recipe { seed, style, ..Recipe::default() };
                    let plan = random_plan(&r, bpm, meter, bars);
                    assert_eq!(plan.iter().map(|x| x.1).sum::<u32>(), bars, "{plan:?}");
                    assert!(plan.iter().all(|x| x.1 >= 1));
                    // (a piece too short for three chords takes the usual arrangement)
                    if plan != default_plan(style, bpm, meter, bars) {
                        // an intro, then a quiet start
                        let open = if plan[0].0 == Part::Intro { 1 } else { 0 };
                        assert!(matches!(plan[open].0, Part::Neon | Part::Swell), "{plan:?}");
                        assert!(plan[open..].iter().all(|x| x.0 != Part::Intro), "{plan:?}");
                        // (Noir always opens on one, given four chords or more)
                        if style == Style::Noir && bars >= 16 {
                            assert_eq!(plan[0].0, Part::Intro, "{plan:?}");
                        }
                        assert_eq!(plan.last().unwrap().0, Part::Fade);
                        assert!(plan.iter().any(|x| x.0 == Part::NightShift), "{plan:?}");
                        assert!(plan.windows(2).all(|w| w[0].0 != w[1].0), "{plan:?}");
                    }
                    assert_eq!(random_plan(&r, bpm, meter, bars), plan);
                    let p = compose_plan(&r, bpm, meter, &plan);
                    assert_eq!(p.bars(), bars.max(2));
                    if bars == 24 && style == Style::Nocturne {
                        seen.insert(format!("{plan:?}"));
                    }
                }
            }
        }
        assert!(seen.len() > 20, "only {} arrangements", seen.len());
    }

    #[test]
    fn chords_change_about_every_six_seconds() {
        let r = Recipe::default();
        for bpm in [50.0, 70.0, 100.0, 140.0] {
            let p = compose(&r, bpm, (4, 4), 32);
            let bar = p.bar_secs();
            let pad = p.tracks.iter().find(|t| t.name == "Pad").unwrap();
            let mut starts: Vec<u32> = pad.notes.iter().map(|n| n.start).collect();
            starts.dedup();
            let secs = p.tick_secs(starts[1] - starts[0]);
            assert!((bar * 0.99..=bar * 4.01).contains(&secs) && (2.0..=10.5).contains(&secs), "{bpm}: {secs}");
        }
    }

    #[test]
    fn edits_change_the_fingerprint() {
        let mut p = compose(&Recipe::default(), 66.0, (4, 4), 16);
        let before = fingerprint(&p);
        p.tracks[0].notes[0].vel -= 1;
        assert_ne!(before, fingerprint(&p));
    }

    /// Any order of sections, repeats included: the piece is as long as they are, its
    /// sections are them, and each part plays what it should.
    #[test]
    fn follows_an_arrangement() {
        use Part::*;
        let plan = [(Intro, 4), (Neon, 4), (Swell, 6), (Neon, 3), (NightShift, 8), (Swell, 5), (Neon, 2), (Fade, 6)];
        for seed in 0..20 {
            let p = compose_plan(&Recipe { seed, ..Recipe::default() }, 70.0, (4, 4), &plan);
            assert_eq!(p.bars(), 38);
            assert_eq!(p.plan().unwrap(), plan.to_vec());
            let bar = p.bar_ticks();
            let part_at = |tick: u32| {
                let b = tick / bar;
                let mut at = 0;
                plan.iter()
                    .find(|(_, n)| {
                        at += n;
                        b < at
                    })
                    .unwrap()
                    .0
            };
            for t in &p.tracks {
                for n in &t.notes {
                    let part = part_at(n.start);
                    match t.name.as_str() {
                        "Brass" | "Bass" => assert!(matches!(part, Swell | NightShift), "{} in {part:?}", t.name),
                        "Lead" | "Pluck" => assert_eq!(part, NightShift, "{}", t.name),
                        "Answer" => assert!(matches!(part, Fade | Intro), "answer in {part:?}"),
                        "Bells" | "Drums" => assert_ne!(part, Intro, "{}", t.name),
                        _ => {}
                    }
                }
            }
            assert!(p.tracks.iter().any(|t| t.name == "Lead"));
            // the intro: the theme's opening alone, over a pad of two notes
            let bar = p.bar_ticks();
            let answer = p.tracks.iter().find(|t| t.name == "Answer").unwrap();
            assert!(answer.notes.iter().any(|n| n.start >= bar && n.start < 4 * bar));
            let pad = p.tracks.iter().find(|t| matches!(t.role, Some(Role::Pad | Role::Strings))).unwrap();
            assert!(pad.notes.iter().filter(|n| n.start == 0).count() <= 2);
        }
    }

    #[test]
    fn arrangements_round_trip_and_stretch() {
        let mut p = compose(&Recipe::default(), 66.0, (4, 4), 24);
        let a = p.arrangement();
        assert_eq!(a.iter().map(|b| b.bars).sum::<u32>(), 24);
        p.set_arrangement(&a);
        assert_eq!(p.arrangement(), a);
        p.stretch_to(48);
        let b = p.arrangement();
        assert_eq!(p.length_bars, 48);
        assert_eq!(b.iter().map(|b| b.bars).sum::<u32>(), 48);
        assert!(b.iter().zip(&a).all(|(x, y)| x.part == y.part && x.bars.abs_diff(y.bars * 2) <= 1));
        // older pieces' names still stand for parts
        let demo = crate::demo::rain_on_neon();
        assert_eq!(demo.plan().unwrap().len(), 4);
        let old = Section { bar: 0, name: "Recede".into(), part: None };
        assert_eq!(Project { sections: vec![old], ..Project::default() }.plan().unwrap()[0].0, Part::Fade);
    }

    #[test]
    fn resize_holds_the_rain_to_the_new_end() {
        let mut p = crate::demo::rain_on_neon();
        resize(&mut p, 24, 40);
        assert_eq!(p.bars(), 40);
        let city = p.tracks.iter().find(|t| t.name == "City").unwrap();
        assert!(city.notes.iter().any(|n| n.key == fx::RAIN && n.end() == p.end_tick()));
        resize(&mut p, 40, 10);
        assert_eq!(p.bars(), 10);
        assert!(p.tracks.iter().all(|t| t.notes.iter().all(|n| n.end() <= p.end_tick())));
    }
}

#[cfg(test)]
mod rain_choice {
    use super::*;

    /// The rain is pitter-patter far more often than a heavy wash.
    #[test]
    fn mostly_sparse_rain() {
        let mut counts = std::collections::BTreeMap::new();
        let n = 2000;
        for seed in 0..n {
            let p = compose(&Recipe { seed, ..Recipe::default() }, 66.0, (4, 4), 8);
            let city = p.tracks.iter().find(|t| t.name == "City").unwrap();
            let main = city.notes.iter().filter(|x| RainKind::of_key(x.key).is_some()).max_by_key(|x| x.len).unwrap();
            *counts.entry(format!("{:?}", RainKind::of_key(main.key).unwrap())).or_insert(0) += 1;
        }
        println!("{counts:?}");
        let dense: i32 = ["Window", "WetStreet", "City"].iter().map(|k| counts.get(*k).copied().unwrap_or(0)).sum();
        assert!((dense as f64) < n as f64 * 0.15, "{counts:?}");
    }
}
