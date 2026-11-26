//! Tarn (D35): warm pastoral ambient, nostalgic and reflective, golden hour over a quiet
//! lake and forest, beatless, at 54 bpm in 4/4 in A Dorian (a minor key with a warm,
//! major sixth).
//!
//! Crickets and a breeze, the dust of an old record and the hiss of tape, and a deep
//! analogue drone on the root, its pitch drifting slightly, from first to last. Breathy
//! tape-choir pads and a warm filtered saw pad swell in slowly, both wavering like
//! stretched tape; a felt piano plays a simple motif of a few notes with long pauses
//! between, varying it a little each time; far off, deep in the reverb, a nylon guitar
//! plucks a slow broken chord now and then. Wide, spacious reverb; tape over the whole
//! mix. No drums, no percussion, no voices.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::fx;
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::canopy::{Step, phrases, realise};
use crate::compose::{Recipe, near};
use crate::demo::GM_FONT;
use crate::nebula::{Writer, envelopes, ramp};
use crate::palette::Role;
use crate::project::{PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;
use crate::strata::Chord;

/// The usual arrangement: the shore at dusk, the pads and the piano's motif, the guitar
/// far off, the warmest light, and the crickets alone again.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 12, 8, 8] } else { [2, 3, 4, 4, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// The Dorian chords: the minor tonic with its ninth, the major fourth (the mode's warm
/// sixth is its third) with its ninth, the relative major with its major seventh, the
/// flat seventh, and the minor fifth.
const I: Chord = Chord { root: 0, ivs: &[0, 3, 7, 14] };
const IV: Chord = Chord { root: 5, ivs: &[0, 4, 7, 14] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7, 11] };
const VII: Chord = Chord { root: 10, ivs: &[0, 4, 7] };
const V: Chord = Chord { root: 7, ivs: &[0, 3, 7, 10] };

/// Four bars a chord, sixteen round: the tonic and the warm fourth, wandering and home.
const PROGS: [[Chord; 4]; 5] = [[I, IV, III, VII], [I, VII, IV, I], [I, III, VII, IV], [I, IV, I, V], [I, V, IV, IV]];

/// The Dorian mode.
pub(crate) const DORIAN: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Crickets,
    Breeze,
    Dust,
    Drone,
    Choir,
    Amber,
    Piano,
    Guitar,
}

const LAYERS: [Layer; 8] =
    [Layer::Crickets, Layer::Breeze, Layer::Dust, Layer::Drone, Layer::Choir, Layer::Amber, Layer::Piano, Layer::Guitar];

/// How present a layer is `u` of the way through a section of this part.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.8, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (_, Dust) => 1.0,
        (Part::Intro, Crickets | Breeze) => 1.0,
        (Part::Intro, Drone) => ramp(u, -0.1, 0.45),
        (Part::Intro, Choir) => 0.6 * ramp(u, 0.5, 1.0),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Crickets | Breeze) => 1.0 - 0.3 * u,
        (Part::Neon, Choir) => 0.6 + 0.4 * ramp(u, 0.0, 0.4),
        (Part::Neon, Piano) => ramp(u, -0.05, 0.3),
        (Part::Neon, Amber | Guitar) => 0.0,
        (Part::Neon, _) => 1.0,
        (Part::Swell, Crickets | Breeze) => 0.7,
        (Part::Swell, Amber) => ramp(u, -0.1, 0.45),
        (Part::Swell, Guitar) => ramp(u, 0.2, 0.6),
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Crickets | Breeze) => 0.6,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Crickets | Breeze) => 0.6 + 0.4 * ramp(u, 0.2, 0.6),
        (Part::Fade, Drone) => 1.0,
        (Part::Fade, Choir) => 1.0 - 0.6 * ramp(u, 0.3, 0.8),
        (Part::Fade, Amber) => 1.0 - ramp(u, 0.0, 0.45),
        (Part::Fade, Piano) => 1.0 - ramp(u, 0.35, 0.65),
        (Part::Fade, Guitar) => 1.0 - ramp(u, 0.0, 0.35),
    };
    v * end
}

/// How warmly everything plays: rising gently to the golden light, falling away.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2,
        Part::Neon => 0.3 + 0.15 * u,
        Part::Swell => 0.45 + 0.2 * u,
        Part::NightShift => 0.65 + 0.25 * (u / 0.6).min(1.0),
        Part::Fade => 0.75 - 0.55 * u,
    }
}

/// Tarn's instruments: the shore wide, the dust and hiss dry, the drone deep and
/// centred, the pads wide, the felt piano close and dark, the guitar far off.
fn tracks(p: &mut Project) -> [usize; 8] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    let gm = |program: u8, name: &str| Source::SoundFont { font: GM_FONT.into(), bank: 0, program, name: name.into() };
    let city = || Source::Kit(KitKind::CityFx);
    [
        add("Crickets", Role::City, city(), Strip { reverb: 0.2, width: 1.7, gain_db: -3.0, high_cut: 8000.0, ..s }),
        add("Breeze", Role::City, city(), Strip { reverb: 0.15, width: 1.8, gain_db: -5.0, ..s }),
        add("Vinyl & Tape", Role::City, city(), Strip { reverb: 0.0, gain_db: -7.0, high_cut: 9000.0, ..s }),
        add("Drone", Role::Drone, synth("Loam Drone"), Strip { reverb: 0.35, width: 1.3, gain_db: -5.0, ..s }),
        add("Tape Choir", Role::Pad, synth("Breath Choir"), Strip { reverb: 0.6, width: 1.7, ensemble: true, gain_db: -9.0, ..s }),
        add("Amber Pad", Role::Pad, synth("Amber Pad"), Strip { reverb: 0.55, width: 1.9, gain_db: -13.0, ..s }),
        add(
            "Felt Piano",
            Role::Keys,
            gm(0, "Grand Piano"),
            Strip { reverb: 0.45, echo: 0.12, pan: -0.15, gain_db: 3.0, high_cut: 1900.0, ..s },
        ),
        add(
            "Nylon Guitar",
            Role::Pluck,
            gm(24, "Nylon Guitar"),
            Strip { reverb: 0.85, echo: 0.3, pan: 0.35, gain_db: -3.0, low_cut: 180.0, high_cut: 4500.0, ..s },
        ),
    ]
}

/// Write Tarn into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a wide, soft cloud of reverb, a long dark echo, worn tape over everything
    p.fx.space = Space::Cloud;
    p.fx.size = 0.85;
    p.fx.tone = 0.35;
    p.fx.motion = 0.5;
    p.fx.predelay = 0.05;
    p.fx.low_cut = 0.45;
    p.fx.width = 1.0;
    p.fx.duck = 0.1;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.2;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.5;
    p.fx.echo.time = EchoTime::Half;
    p.fx.echo.feedback = 0.35;
    p.fx.echo.tone = 0.3;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.6;
    p.fx.echo.wow = 0.35;
    p.master_db = 8.5;

    let bar = p.bar_ticks();
    let q = PPQ / 2;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "tarn-harmony");
    let prog = *hr.pick(&PROGS);
    // the piano's motif: one of the seed's phrases, played again and again, a little
    // varied; and one other for now and then
    let motifs = phrases(r.seed, "tarn-piano");
    let mut pr = Rng::stream(r.seed, "tarn-piano-play");
    let mut gr = Rng::stream(r.seed, "tarn-guitar");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "tarn-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [crickets, breeze, dust, drone, choir, amber, piano, guitar] = tr;
    let tonic = pitch(0, 62);

    let (env, force) = envelopes::<8>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.65 + 0.35 * inten(t));
    let beats = |b: f64| (b * PPQ as f64).round() as u32;

    // the shore, the record's dust and the tape's hiss, all the way through
    w.note(crickets, 0, end, fx::CRICKETS as i32, 80.0);
    w.note(breeze, 0, end, fx::BREEZE as i32, 76.0);
    w.note(dust, 0, end, fx::CRACKLE as i32, 62.0);
    w.note(dust, 0, end, fx::TAPE_HISS as i32, 40.0);
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the drone: the root, deep, through the section and into the next
        w.note(drone, a, (zz - a) + bar, pitch(0, 45), 84.0);
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
            // the tape choir: the chord in the middle
            if new_chord && pres(Layer::Choir, t_bar + bar) > 0.03 {
                for (j, pc) in pcs.iter().enumerate() {
                    w.note(choir, t_bar, len, pitch(*pc, [57, 61, 64, 67][j]), vel([70.0, 66.0, 64.0, 60.0][j], t_bar));
                }
            }
            // the amber pad: the root and fifth low, the third above, slow to open
            if new_chord && pres(Layer::Amber, t_bar + bar) > 0.03 {
                for (j, pc) in [pcs[0], pcs[2], pcs[1]].iter().enumerate() {
                    w.note(amber, t_bar, len, pitch(*pc, [45, 52, 57][j]), vel(70.0, t_bar));
                }
            }
            let in_chord = k % 4;
            // the felt piano: its motif in a chord's first or second bar, then a long
            // pause; softly, the root low under its first note now and then
            let pp = pres(Layer::Piano, t_bar);
            if in_chord == 0 && pp > 0.15 && pr.chance(0.4 + 0.45 * pp) {
                let motif: &Vec<Step> = if pr.chance(0.75) { &motifs[0] } else { &motifs[1] };
                // (varied a little: now and then the second note a step away)
                let mut motif = motif.clone();
                if motif.len() > 2 && pr.chance(0.3) {
                    motif[1].step += *pr.pick(&[-1, 1]);
                }
                let from = t_bar + beats(*pr.pick(&[0.0, 0.0, 1.0, 4.0]));
                let target = *pr.pick(&[67, 69, 72]);
                for (j, (s, l, rel)) in realise(&motif, chord, &DORIAN, target - tonic, (-5, 15)).into_iter().enumerate() {
                    let t = from + beats(s);
                    if t < zz && pres(Layer::Piano, t) > 0.1 {
                        w.note(piano, t, beats(l + 1.0), tonic + rel, vel(if j == 0 { 52.0 } else { 46.0 }, t));
                    }
                }
                if pr.chance(0.6) {
                    w.note(piano, from, beats(6.0), pitch(chord.root, 45), vel(40.0, from));
                }
            }
            // the guitar far off: a slow broken chord in a chord's third bar, now and then
            let gp = pres(Layer::Guitar, t_bar);
            if in_chord == 2 && gp > 0.15 && gr.chance(0.35 + 0.4 * gp) {
                let from = t_bar + beats(*gr.pick(&[0.0, 1.0]));
                let shape: &[(i32, i32)] = gr.pick(&[
                    &[(0, 52), (7, 59), (12, 64), (4, 68)][..],
                    &[(0, 52), (7, 59), (4, 64)][..],
                    &[(7, 59), (12, 64), (16, 68), (12, 64)][..],
                ]);
                for (j, &(iv, target)) in shape.iter().enumerate() {
                    // (the chord's own third, minor or major)
                    let pc = if iv % 12 == 4 { pcs[1] } else { chord.root + iv };
                    let t = from + j as u32 * beats(0.75);
                    if t < zz && pres(Layer::Guitar, t) > 0.1 {
                        w.note(guitar, t, beats(4.0), pitch(pc, target), gr.range_f(48.0, 64.0));
                    }
                }
            }
        }
    }

    // expression a quaver at a time, and the drone's and pads' filters with the light
    let wave = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Crickets | Layer::Breeze | Layer::Dust => 127.0 * pres(*layer, t),
                Layer::Piano | Layer::Guitar => 127.0 * (0.6 + 0.4 * pres(*layer, t)),
                _ => 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t)),
            };
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Drone => Some(50.0 + 14.0 * inten(t) + 6.0 * wave(t, 6.0, 0.0)),
                Layer::Choir => Some(50.0 + 20.0 * inten(t)),
                Layer::Amber => Some(48.0 + 26.0 * inten(t) + 5.0 * wave(t, 4.0, 0.4)),
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
    use crate::compose::{Mode, Style, compose};
    use crate::project::CtrlKind;

    fn piece(seed: u64, bars: u32) -> Project {
        let mut r = Recipe { style: Style::Tarn, ..Recipe::random(seed) };
        Style::Tarn.settle(&mut r);
        compose(&r, Style::Tarn.tempo(), (4, 4), bars)
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the shore and the drone first, the choir and piano, the amber pad and
    /// guitar later, and the drone and crickets left at the end; A Dorian; no drums.
    #[test]
    fn tarn_glows_and_fades() {
        for seed in 0..12 {
            let p = piece(seed, 44);
            assert_eq!(p.recipe.map(|r| (r.root, r.mode)), Some((9, Mode::Dorian)));
            assert!(p.tracks.iter().all(|t| t.role != Some(Role::Drums)));
            let b = p.block_ticks();
            assert_eq!(b.len(), 5);
            assert!(starts(&p, "Drone", b[0]) >= 1, "seed {seed}");
            for name in ["Felt Piano", "Amber Pad", "Nylon Guitar"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Felt Piano", b[1]) + starts(&p, "Felt Piano", b[2]) >= 3, "seed {seed}");
            assert!(starts(&p, "Amber Pad", b[3]) >= 2, "seed {seed}");
            let tail = b[4].0 + (b[4].1 - b[4].0) * 2 / 3;
            for name in ["Felt Piano", "Amber Pad", "Nylon Guitar"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            let shore = p.tracks.iter().find(|t| t.name == "Crickets").unwrap();
            assert!(shore.notes.iter().any(|n| n.key == fx::CRICKETS && n.start == 0 && n.end() == p.end_tick()));
            let dust = &crate::ambience::merged(&p);
            assert!(dust.notes.iter().any(|n| n.key == fx::CRACKLE));
        }
    }

    /// The piano leaves long pauses and keeps to its motif; every tuned note is Dorian.
    #[test]
    fn sparse_motifs_in_the_mode() {
        for seed in 0..20 {
            let p = piece(seed, 44);
            let bars = p.bars() as usize;
            let piano = p.tracks.iter().find(|t| t.name == "Felt Piano");
            let n = piano.map_or(0, |t| t.notes.len());
            assert!(n <= bars * 3 / 2, "seed {seed}: {n} piano notes in {bars} bars");
            for t in p.tracks.iter().filter(|t| t.role != Some(Role::City)) {
                for n in &t.notes {
                    let pc = (n.key as i32 - 9).rem_euclid(12);
                    assert!(DORIAN.contains(&pc), "seed {seed} {}: {}", t.name, n.key);
                }
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
            let r = Recipe { style: Style::Tarn, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 54.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Tarn, ..Recipe::random(i as u64) }, 54.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Tarn, ..Recipe::random(7) }, 54.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
