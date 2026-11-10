//! The composer: a seeded neon-noir cue written to fit the tempo, metre and length it
//! is given. A seed picks the progression, voicings, patterns, melody, instruments and
//! the hall; the same recipe at the same tempo and length always writes the same
//! notes. It answers the tempo musically: chords last about six seconds whatever the
//! tempo, so slow pieces change chord every bar and fast ones every few, and the bells
//! fall in eighths when slow and in beats when fast. Its sections (rain, brass, a lone
//! lead, recede) are Neon, Swell, Night shift and Fade; by default they scale to the
//! length, and a piece can also be written to any arrangement of them (`compose_plan`).
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
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe { seed: 1, root: 2, mode: Mode::Aeolian, density: 0.5 }
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

fn chords(r: &Recipe, p: &Project, chord_ticks: u32, end: u32) -> Vec<Chord> {
    let mut rng = Rng::stream(r.seed, "harmony");
    let steps = r.mode.steps();
    let prog = if r.mode == Mode::Phrygian && rng.chance(0.6) { *rng.pick(&PHRYGIAN) } else { *rng.pick(&PROGRESSIONS) };
    let pc = |deg: usize| (r.root + steps[deg % 7]) % 12;
    let mut out: Vec<Chord> = Vec::new();
    let mut t = 0;
    let mut i = 0;
    let _ = p;
    while t < end {
        let last = t + chord_ticks >= end;
        let deg = if last { 0 } else { prog[i % 4] };
        let (a, b, c) = (pc(deg), pc(deg + 2), pc(deg + 4));
        // colour: add9, sus2 or sus4 (a cycle's last chord most often hangs on a sus4)
        let pcs = match (last, i % 4 == 3, rng.weighted(&[3.0, 1.5, 1.0, 1.0])) {
            (true, _, _) => vec![a, pc(deg + 1), c, b],
            (_, true, _) if rng.chance(0.6) => vec![a, pc(deg + 3), c],
            (_, _, 0) => vec![a, b, c, pc(deg + 1)],
            (_, _, 1) => vec![a, pc(deg + 1), c],
            (_, _, 2) => vec![a, pc(deg + 3), c],
            _ => vec![a, b, c, pc(deg + 6)],
        };
        let prev = out.last().map(|c| c.voicing.clone());
        let voicing = voice(&pcs, prev.as_deref(), 60);
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

/// About six seconds a chord, in whole bars.
fn chord_bars(bar_secs: f64, bars: u32) -> u32 {
    ((6.5 / bar_secs).round() as u32).clamp(1, 4).min(bars.max(1))
}

/// The usual arrangement for a length: Neon, Swell, Night shift and Fade starting at
/// a sixth, a half and five sixths, on chord changes (short pieces drop the later ones).
pub fn default_plan(bpm: f64, meter: (u32, u32), bars: u32) -> Vec<(Part, u32)> {
    let p = Project { bpm, meter, length_bars: bars.max(2), ..Project::default() };
    let bars = p.length_bars;
    let cb = chord_bars(p.bar_secs(), bars);
    let snap = |frac: f64| ((bars as f64 * frac / cb as f64).round() as u32 * cb).min(bars);
    let mut bounds = vec![0, snap(1.0 / 6.0), snap(0.5), snap(5.0 / 6.0)];
    bounds.dedup();
    bounds.retain(|b| *b < bars);
    bounds.iter().enumerate().map(|(i, b)| (Part::ALL[i.min(3)], bounds.get(i + 1).unwrap_or(&bars) - b)).collect()
}

/// Write a piece for this recipe at the project's tempo, metre and length, in the
/// usual arrangement. The tracks, sounds, mix, sections and effects are all new.
pub fn compose(r: &Recipe, bpm: f64, meter: (u32, u32), bars: u32) -> Project {
    compose_plan(r, bpm, meter, &default_plan(bpm, meter, bars))
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
    let chord_bars = chord_bars(bar_secs, bars);
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
    let adjectives = ["Neon", "Midnight", "Sodium", "Wet", "Electric", "Static", "Chrome", "Blue", "Distant", "Last"];
    let nouns = ["Rain", "Overpass", "Avenue", "Skyline", "Arcade", "Monorail", "Harbour", "Signal", "Rooftops", "Memory"];
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
        let w: Vec<f64> = (0..alts.len()).map(|i| if i == 0 { 3.0 } else { 1.0 }).collect();
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
    fx.space = [Space::Noir, Space::Cathedral, Space::BlackHole][snd.weighted(&[6.0, 2.0, 1.5])];
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
    let t_brass = add(Role::Brass, brass, Strip { reverb: 0.5, ensemble: true, gain_db: -3.0, ..s });
    let t_pad = add(pad_role, pad, Strip { reverb: 0.6, ensemble: true, gain_db: -6.0, ..s });
    let t_lead = add(Role::Lead, lead, Strip { reverb: 0.55, echo: 0.35, pan: 0.1, gain_db: 0.0, ..s });
    let t_bass = add(Role::Bass, bass, Strip { reverb: 0.08, gain_db: 2.0, ..s });
    let t_bells = add(Role::Bells, bells, Strip { reverb: 0.65, echo: 0.4, gain_db: -4.0, pan: -0.2, ..s });
    let t_pluck = add(Role::Pluck, pluck, Strip { reverb: 0.5, echo: 0.3, pan: 0.35, gain_db: 0.0, ..s });
    let t_answer = add(Role::Answer, answer, Strip { reverb: 0.6, echo: 0.3, pan: -0.15, gain_db: -1.0, ..s });
    let t_drone = add(Role::Drone, drone, Strip { reverb: 0.5, gain_db: -7.0, width: 1.4, ..s });
    let city_alt = alternatives(Role::City).remove(0);
    let t_city = add(Role::City, city_alt, Strip { reverb: 0.25, gain_db: -6.0, width: 1.4, ..s });
    let drums_alt = alternatives(Role::Drums).remove(0);
    let t_drums = add(Role::Drums, drums_alt, Strip { reverb: 0.4, echo: 0.15, gain_db: -5.0, ..s });
    p.tracks = tracks;
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
        for &k in &c.voicing {
            push(&mut t[t_pad].notes, c.start, ring, k, 60);
        }
        let part = part_at(c.start);
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
    for c in chords.iter().filter(|c| matches!(part_at(c.start), Part::Swell | Part::NightShift)) {
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
    let bells_from = if secs[0].0 == Part::Neon && secs[0].2 > chord_ticks { chord_ticks } else { 0 };
    let ringing = |tick: u32| match secs.iter().find(|(_, a, z)| tick >= *a && tick < *z) {
        Some((Part::Fade, a, z)) => tick < a + (z - a) / 2,
        _ => true,
    };
    let mut tick = bells_from;
    let mut i = 0usize;
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
    let scale: Vec<u8> = (55..=81u8).filter(|k| steps.contains(&((k + 12 - r.root % 12) % 12))).collect();
    let phrase_bars = if chord_bars >= 2 { chord_bars * 2 } else { 4 }.min(8);
    let cells: [&[f64]; 7] = [&[3.0, 1.0], &[2.0, 2.0], &[4.0], &[1.0, 3.0], &[1.5, 0.5, 2.0], &[2.0, 1.0, 1.0], &[1.0, 1.0, 2.0]];
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
    let mut pos_idx = scale.iter().position(|k| *k >= 67).unwrap_or(scale.len() / 2) as i32;
    let mut phrase_no = 0;
    for (s2, s3) in of(Part::NightShift) {
        let mut phrase_start = s2;
        while phrase_start < s3 {
            let mut t0 = phrase_start as f64;
            // phrases A A' B A: B gets a fresh rhythm
            let rhythm: Vec<f64> = if phrase_no % 4 == 2 { rhythm_a.iter().rev().copied().collect() } else { rhythm_a.clone() };
            for (ni, d) in rhythm.iter().enumerate() {
                let start = at(t0);
                if start >= s3 {
                    break;
                }
                let c = chord_at(&chords, start);
                let strong = (start - c.start).is_multiple_of(bar);
                let mv = if mel.chance(0.25) { mel.range(-4, 4) } else { mel.range(-2, 2) };
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
                if *d >= 3.0 {
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
    for (s2, s3) in of(Part::NightShift) {
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

    // ---------------------------------------------------------------- the drone
    // the tonic and its fifth, struck again at each section so it swells with the form
    let tonic = near(r.root, 45);
    for (part, a, z) in &secs {
        push(&mut t[t_drone].notes, *a, z - a, tonic, 80);
        if *part != Part::Neon {
            push(&mut t[t_drone].notes, *a, z - a, tonic + 7, 64);
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

    // ---------------------------------------------------------------- drums
    let mut dr = Rng::stream(r.seed, "drums");
    if density > 0.2 {
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
        let plan = [(Neon, 4), (Swell, 6), (Neon, 3), (NightShift, 8), (Swell, 5), (Neon, 2), (Fade, 6)];
        for seed in 0..20 {
            let p = compose_plan(&Recipe { seed, ..Recipe::default() }, 70.0, (4, 4), &plan);
            assert_eq!(p.bars(), 34);
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
                        "Answer" => assert_eq!(part, Fade),
                        _ => {}
                    }
                }
            }
            assert!(p.tracks.iter().any(|t| t.name == "Lead"));
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
