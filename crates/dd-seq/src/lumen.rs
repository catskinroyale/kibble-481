//! Lumen (D30): a luminous drone score, slow and radiant, light through high cloud, at
//! 60 bpm in 4/4 in the Dorian mode.
//!
//! A warm drone of sines, gently beating, holds the tonic and fifth from first to last;
//! high above it a prism of bell-like tones rings the chord's upper notes, shimmering
//! into the reverb. Open chords without a third (and the mode's bright major fourth)
//! change every four bars; a soft plucked pulse of quavers begins to turn under them,
//! strings and a deep sub warm the middle, chimes fall now and then like light, and a
//! melody of a few long notes rises at the radiant peak before everything thins back to
//! the drone and the air.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::patch::by_name;

use crate::compose::{Recipe, near};
use crate::nebula::{Writer, envelopes, ladder, ramp, snap};
use crate::palette::Role;
use crate::project::{PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;
use crate::strata::Chord;

/// The usual arrangement: the drone and air, the pulse beginning, the strings and
/// melody, the radiant peak, and thinning back to the drone, shared in proportion.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 12, 12, 8] } else { [2, 3, 4, 5, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

const I: Chord = Chord { root: 0, ivs: &[0, 2, 7] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 4, 7] };
const VII: Chord = Chord { root: 10, ivs: &[0, 4, 7] };

/// Open, bright Dorian progressions, four bars a chord: the tonic as a suspended ninth
/// (no third), the major fourth, the flat third and seventh.
const PROGS: [[Chord; 4]; 5] = [[I, IV, I, VII], [I, III, IV, I], [I, VII, IV, IV], [I, IV, III, VII], [I, VII, III, IV]];

/// The Dorian mode, the melody's scale; and the chimes' (its pentatonic).
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];
const PENTA: [i32; 5] = [0, 2, 3, 7, 9];

/// The pulse: four notes a turn, as degrees of the chord (0 root, 1 the colour, 2 the
/// fifth, 3 the octave).
const PULSES: [[usize; 4]; 4] = [[0, 2, 3, 2], [0, 1, 2, 3], [2, 0, 3, 1], [0, 3, 1, 2]];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Air,
    Radiant,
    Prism,
    Strings,
    Pulse,
    Chimes,
    Bass,
    Lead,
}

const LAYERS: [Layer; 8] =
    [Layer::Air, Layer::Radiant, Layer::Prism, Layer::Strings, Layer::Pulse, Layer::Chimes, Layer::Bass, Layer::Lead];

/// How present a layer is `u` of the way through a section of this part.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.75, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (Part::Intro, Air) => 1.0,
        (Part::Intro, Radiant) => ramp(u, -0.1, 0.45),
        (Part::Intro, Chimes) => 0.5 * ramp(u, 0.4, 0.8),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Air) => 1.0 - 0.4 * u,
        (Part::Neon, Prism) => ramp(u, -0.05, 0.4),
        (Part::Neon, Pulse) => ramp(u, 0.3, 0.7),
        (Part::Neon, Chimes) => 0.6,
        (Part::Neon, Strings | Bass) => 0.0,
        (Part::Neon, Lead) => ramp(u, 0.85, 1.0),
        (Part::Neon, _) => 1.0,
        (Part::Swell, Air) => 0.5,
        (Part::Swell, Strings) => ramp(u, -0.1, 0.4),
        (Part::Swell, Bass) => ramp(u, 0.1, 0.5),
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Air) => 0.4,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Air) => 0.4 + 0.6 * ramp(u, 0.2, 0.6),
        (Part::Fade, Radiant) => 1.0,
        (Part::Fade, Prism) => 1.0 - 0.5 * ramp(u, 0.3, 0.7),
        (Part::Fade, Strings) => 1.0 - ramp(u, 0.1, 0.6),
        (Part::Fade, Pulse) => 1.0 - ramp(u, 0.0, 0.4),
        (Part::Fade, Chimes) => 0.7,
        (Part::Fade, Bass) => 1.0 - ramp(u, 0.0, 0.35),
        (Part::Fade, Lead) => 1.0 - ramp(u, 0.1, 0.35),
    };
    v * end
}

/// How strongly and brightly everything plays: rising to a radiant peak, falling away.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2,
        Part::Neon => 0.25 + 0.2 * u,
        Part::Swell => 0.45 + 0.2 * u,
        Part::NightShift => 0.65 + 0.35 * (u / 0.7).min(1.0),
        Part::Fade => 0.85 - 0.65 * u,
    }
}

/// Lumen's instruments: the drone and prism wide, the pulse and chimes into the echo,
/// the strings through the ensemble, the sub centred.
fn tracks(p: &mut Project) -> [usize; 8] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    [
        add("Air", Role::Pad, synth("Solar Wind"), Strip { reverb: 0.55, width: 1.9, gain_db: -13.0, ..s }),
        add("Radiant", Role::Drone, synth("Radiant Drone"), Strip { reverb: 0.5, width: 1.5, gain_db: -8.0, ..s }),
        add("Prism", Role::Drone, synth("Prism Drone"), Strip { reverb: 0.7, echo: 0.2, width: 1.9, gain_db: -8.0, ..s }),
        add("Strings", Role::Strings, synth("Silk Strings"), Strip { reverb: 0.6, width: 1.7, ensemble: true, gain_db: -12.0, ..s }),
        add("Pulse", Role::Pluck, synth("Lantern Pluck"), Strip { reverb: 0.45, echo: 0.4, pan: -0.15, width: 1.5, gain_db: -10.0, ..s }),
        add("Chimes", Role::Bells, synth("Glass Chimes"), Strip { reverb: 0.7, echo: 0.55, pan: 0.35, width: 1.8, gain_db: -10.0, ..s }),
        add("Bass", Role::Bass, synth("Pure Sub"), Strip { reverb: 0.0, gain_db: -3.0, high_cut: 400.0, ..s }),
        add("Lead", Role::Lead, synth("Starlight Lead"), Strip { reverb: 0.6, echo: 0.45, pan: 0.1, gain_db: -8.0, ..s }),
    ]
}

/// One note of the melody: its start and length in crotchets from the start of its
/// eight-bar phrase, and its pitch above the melody's tonic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MNote {
    at: f64,
    len: f64,
    rel: i32,
}

const LOW: i32 = -3;
const HIGH: i32 = 16;

/// The melody: a few long notes rising through the first six of its eight bars and
/// settling onto the third chord, then a breath.
fn motif(seed: u64, prog: &[Chord; 4], beats_per_bar: f64) -> Vec<MNote> {
    let mut rng = Rng::stream(seed, "lumen-melody");
    const RHYTHMS: [&[(f64, f64)]; 4] = [
        &[(0.0, 1.5), (1.5, 0.5), (2.0, 1.0), (3.0, 1.0), (4.0, 2.0)],
        &[(0.0, 1.0), (1.0, 1.0), (2.0, 1.5), (3.5, 0.5), (4.0, 2.0)],
        &[(0.5, 1.5), (2.0, 1.0), (3.0, 1.0), (4.0, 2.0)],
        &[(0.0, 2.0), (2.0, 0.5), (2.5, 1.5), (4.0, 2.0)],
    ];
    let rhythm = *rng.pick(&RHYTHMS);
    let scale = ladder(&SCALE, LOW, HIGH);
    let idx = |p: i32| scale.iter().position(|&x| x >= p).unwrap_or(scale.len() - 1);
    let start = snap(*rng.pick(&[7, 2, 0]), &prog[0].pcs()).clamp(LOW, HIGH);
    let mut i = idx(start);
    let mut out = Vec::new();
    for (j, &(at, len)) in rhythm.iter().enumerate() {
        if j > 0 {
            // rising most of the way, a step back before the last
            let step = if j + 1 == rhythm.len() { *rng.pick(&[-1, 1, -2]) } else { *rng.pick(&[1, 1, 2, -1]) };
            i = (i as i32 + step).clamp(0, scale.len() as i32 - 1) as usize;
        }
        let mut rel = scale[i];
        if j + 1 == rhythm.len() {
            rel = snap(rel, &prog[2].pcs()).clamp(LOW, HIGH);
        }
        out.push(MNote { at: at * beats_per_bar, len: len * beats_per_bar, rel });
    }
    out
}

/// Write Lumen into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a shimmering hall, an octave above, an even echo
    p.fx.space = Space::Shimmer;
    p.fx.size = 0.85;
    p.fx.tone = 0.65;
    p.fx.motion = 0.45;
    p.fx.predelay = 0.06;
    p.fx.low_cut = 0.4;
    p.fx.width = 1.0;
    p.fx.duck = 0.15;
    p.fx.shimmer = 0.35;
    p.fx.freeze = false;
    p.fx.ensemble = 0.25;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.0;
    p.fx.echo.time = EchoTime::DottedQuarter;
    p.fx.echo.feedback = 0.45;
    p.fx.echo.tone = 0.55;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.5;
    p.fx.echo.wow = 0.05;
    p.master_db = 6.5;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let beats_per_bar = bar as f64 / PPQ as f64;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "lumen-harmony");
    let prog = *hr.pick(&PROGS);
    let pulse = *hr.pick(&PULSES);
    let melody = motif(r.seed, &prog, beats_per_bar);
    let mut ar = Rng::stream(r.seed, "lumen-air");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "lumen-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [air, radiant, prism, strings, pulse_t, chimes, bass, lead] = tr;
    let lead_tonic = pitch(0, 64);

    let (env, force) = envelopes::<8>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.6 + 0.4 * inten(t));

    w.note(air, 0, end, pitch(0, 67), 64.0);
    let mut rep = 0u32;
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the radiant drone: the tonic and fifth through the section, overlapping the next
        let dl = (zz - a) + bar;
        w.note(radiant, a, dl, pitch(0, 50), 78.0);
        w.note(radiant, a, dl, pitch(7, 57), 62.0);
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let pcs = chord.pcs();
            let new_chord = k == 0 || chord_at(k) != chord_at(k - 1) || k % 4 == 0;
            let span = (4 - k % 4).min(nb - k) * bar;
            let len = span.min(zz.saturating_sub(t_bar) + bar / 2);
            // the prism: the chord's upper notes high above
            if new_chord && pres(Layer::Prism, t_bar + bar) > 0.03 {
                w.note(prism, t_bar, len, pitch(pcs[1], 69), vel(70.0, t_bar));
                w.note(prism, t_bar, len, pitch(pcs[2], 74), vel(60.0, t_bar));
            }
            // the strings: the chord in the middle, warm
            if new_chord && pres(Layer::Strings, t_bar + bar) > 0.03 {
                for (j, pc) in pcs.iter().enumerate() {
                    w.note(strings, t_bar, len, pitch(*pc, [55, 62, 59][j]), vel([70.0, 64.0, 62.0][j], t_bar));
                }
            }
            // the sub: the chord's root, long
            if new_chord && pres(Layer::Bass, t_bar) > 0.03 {
                w.note(bass, t_bar, len.min(3 * bar), pitch(chord.root, 36), vel(92.0, t_bar));
            }
            // the pulse: a turning figure of quavers, every other one when quiet
            let pp = pres(Layer::Pulse, t_bar);
            if pp > 0.05 {
                let notes = [pitch(pcs[0], 62), pitch(pcs[1], 65), pitch(pcs[2], 67), pitch(pcs[0], 62) + 12];
                let mut t = t_bar;
                let mut i = 0usize;
                while t < t_bar + bar && t < zz {
                    if inten(t) > 0.45 || i.is_multiple_of(2) {
                        let accent = if (t - t_bar).is_multiple_of(beat) { 1.0 } else { 0.82 };
                        w.note(pulse_t, t, q * 3 / 2, notes[pulse[i % 4]], vel(80.0 * accent, t));
                    }
                    t += q;
                    i += 1;
                }
            }
            // chimes falling like light: a short descending run on the pentatonic
            let cp = pres(Layer::Chimes, t_bar);
            if k % 2 == 1 && cp > 0.2 && ar.chance(0.3 + 0.4 * cp) {
                let n = ar.range(3, 5) as usize;
                let top = ladder(&PENTA, 12, 26);
                let mut idx = ar.range((n - 1) as i32, top.len() as i32 - 1) as usize;
                let mut s = t_bar + ar.range(0, 2) as u32 * beat;
                for _ in 0..n {
                    w.note(chimes, s, 2 * beat, pitch(0, 72) - 12 + top[idx], ar.range_f(50.0, 70.0));
                    s += q;
                    idx = idx.saturating_sub(1);
                }
            }
        }
        // the melody: its eight bars from the section's start
        if matches!(part, Part::Swell | Part::NightShift | Part::Fade) {
            let mut ph = a;
            while ph < zz {
                for n in &melody {
                    let s = ph + (n.at * PPQ as f64).round() as u32;
                    if s >= zz || pres(Layer::Lead, s) < 0.3 {
                        continue;
                    }
                    // (the second time round, an octave higher at the peak)
                    let up = if part == Part::NightShift && rep % 2 == 1 { 12 } else { 0 };
                    let len = (n.len * PPQ as f64).round() as u32;
                    w.note(lead, s, len, lead_tonic + n.rel + up, vel(80.0, s));
                }
                rep += 1;
                ph += 8 * bar;
            }
        }
    }
    // expression a quaver at a time, and the filters opening with the light
    let wave = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Air => 127.0 * pres(*layer, t),
                _ => 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t)),
            };
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Radiant => Some(50.0 + 24.0 * inten(t) + 8.0 * wave(t, 12.0, 0.0)),
                Layer::Strings => Some(46.0 + 30.0 * inten(t)),
                Layer::Pulse => Some(44.0 + 30.0 * inten(t) + 6.0 * wave(t, 8.0, 0.25)),
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
    use crate::project::CtrlKind;

    fn piece(seed: u64, bars: u32) -> Project {
        compose(&Recipe { style: Style::Lumen, ..Recipe::random(seed) }, Style::Lumen.tempo(), (4, 4), bars)
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the drone and the air alone at first, the pulse turning, the strings
    /// and melody, the pulse gone by the end and the drone still there; Dorian, shimmering.
    #[test]
    fn lumen_brightens_and_thins() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            assert_eq!(p.recipe.map(|r| r.mode), Some(crate::compose::Mode::Dorian));
            assert!(p.fx.shimmer > 0.2);
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            assert!(starts(&p, "Radiant", b[0]) >= 1, "seed {seed}");
            for name in ["Pulse", "Strings", "Lead", "Bass", "Prism"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Pulse", b[1]) >= 8, "seed {seed}");
            for i in [2, 3] {
                assert!(starts(&p, "Pulse", b[i]) > 40, "seed {seed}");
                assert!(starts(&p, "Lead", b[i]) >= 4, "seed {seed}");
                assert!(starts(&p, "Strings", b[i]) >= 3, "seed {seed}");
            }
            let tail = b[4].0 + (b[4].1 - b[4].0) / 2;
            for name in ["Pulse", "Lead", "Bass"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Radiant", b[4]) >= 1, "seed {seed}");
        }
    }

    /// The melody keeps to the mode and its range.
    #[test]
    fn the_melody_keeps_to_the_mode() {
        for seed in 0..40 {
            for prog in PROGS {
                let m = motif(seed, &prog, 4.0);
                assert!((4..=5).contains(&m.len()));
                assert!(m.iter().all(|n| (LOW..=HIGH).contains(&n.rel) && SCALE.contains(&n.rel.rem_euclid(12))));
                assert!(m.last().map_or(0.0, |n| n.at + n.len) <= 24.0);
            }
        }
    }

    /// In any arrangement no layer jumps.
    #[test]
    fn no_sudden_changes_in_any_order() {
        use crate::compose::{compose_plan, random_plan};
        let mut plans: Vec<Vec<(Part, u32)>> = vec![
            [Part::Intro, Part::NightShift, Part::Neon, Part::Fade].iter().map(|p| (*p, 8)).collect(),
            [Part::Swell, Part::Fade, Part::Intro, Part::NightShift].iter().map(|p| (*p, 6)).collect(),
        ];
        for seed in 0..8 {
            let r = Recipe { style: Style::Lumen, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 60.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Lumen, ..Recipe::random(i as u64) }, 60.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Lumen, ..Recipe::random(7) }, 60.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
