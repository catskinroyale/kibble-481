//! Canopy (D35): pastoral ambient electronica, serene and hopeful, sunrise in a forest,
//! beatless, at 60 bpm in 4/4 in D Lydian (the major scale with its bright raised
//! fourth).
//!
//! It opens on the forest waking: birdsong and a breeze through the leaves, and a soft
//! drone of sine and rounded tones on the root and fifth, breathing slowly in and out,
//! that never stops. Warm string-machine pads swell in over four seconds a chord, through
//! the ensemble; an electric piano plays a few notes now and then with plenty of space
//! around them, a celesta answers higher up, and a glockenspiel rings once in a while,
//! wobbling as if played from old tape. At the fullest a second pad glows above the
//! strings; then everything thins back to the drone and the birds. Light tape over the
//! whole mix, a soft hall. No drums, no percussion, no voices.

use std::collections::BTreeSet;

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::fx;
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::compose::{Recipe, near};
use crate::demo::GM_FONT;
use crate::nebula::{Writer, envelopes, ladder, ramp, snap};
use crate::palette::Role;
use crate::project::{Ctrl, CtrlKind, PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;
use crate::strata::Chord;

/// The sounds added to the palette for Canopy and Tarn: the other styles leave them out
/// of their choosing, so every recipe they had still sounds as it did.
pub(crate) const PASTORAL_SOUNDS: [&str; 12] = [
    "Dawn Strings",
    "Breath Choir",
    "Amber Pad",
    "Meadow Drone",
    "Loam Drone",
    "Warm Analog Pad",
    "Airy Choir",
    "Glass Pad",
    "Orchestra Pad",
    "Synth Strings 3",
    "Chorused EP",
    "Bell Tine EP",
];

/// The app's own synthesised font, which has the warm analogue pad.
pub(crate) const AMBIENT_FONT: &str = "Ambient-Electronic.sf2";

/// The usual arrangement: the forest waking, the strings and piano arriving, the celesta
/// and glockenspiel, the fullest light, and thinning back to the birds.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 12, 12, 8] } else { [2, 3, 4, 5, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// The Lydian chords: the tonic with its ninth or major seventh, the major second (the
/// mode's own colour: its third is the raised fourth), the minor third and sixth, and the
/// major fifth with its ninth.
const I: Chord = Chord { root: 0, ivs: &[0, 4, 7, 14] };
const IM7: Chord = Chord { root: 0, ivs: &[0, 4, 7, 11] };
const II: Chord = Chord { root: 2, ivs: &[0, 4, 7] };
const III: Chord = Chord { root: 4, ivs: &[0, 3, 7, 10] };
const V: Chord = Chord { root: 7, ivs: &[0, 4, 7, 14] };
const VI: Chord = Chord { root: 9, ivs: &[0, 3, 7, 10] };

/// Four bars a chord, sixteen round, the major second in every one.
const PROGS: [[Chord; 4]; 5] = [[I, II, I, VI], [IM7, II, VI, V], [I, II, III, II], [I, VI, II, IM7], [IM7, III, II, I]];

/// The Lydian mode, the phrases' scale.
pub(crate) const LYDIAN: [i32; 7] = [0, 2, 4, 6, 7, 9, 11];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Birds,
    Breeze,
    Drone,
    Strings,
    Glow,
    Piano,
    Celesta,
    Glock,
}

const LAYERS: [Layer; 8] =
    [Layer::Birds, Layer::Breeze, Layer::Drone, Layer::Strings, Layer::Glow, Layer::Piano, Layer::Celesta, Layer::Glock];

/// How present a layer is `u` of the way through a section of this part.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.8, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (Part::Intro, Birds | Breeze) => 1.0,
        (Part::Intro, Drone) => ramp(u, -0.1, 0.45),
        (Part::Intro, Strings) => 0.6 * ramp(u, 0.55, 1.0),
        (Part::Intro, Celesta) => 0.5 * ramp(u, 0.5, 0.9),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Birds | Breeze) => 1.0 - 0.25 * u,
        (Part::Neon, Strings) => 0.6 + 0.4 * ramp(u, 0.0, 0.4),
        (Part::Neon, Piano) => ramp(u, 0.1, 0.45),
        (Part::Neon, Celesta) => 0.5,
        (Part::Neon, Glock) => 0.3,
        (Part::Neon, Glow) => 0.0,
        (Part::Neon, _) => 1.0,
        (Part::Swell, Birds | Breeze) => 0.7,
        (Part::Swell, Glow) => 0.7 * ramp(u, 0.2, 0.7),
        (Part::Swell, Glock) => 0.5,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Birds | Breeze) => 0.6,
        (Part::NightShift, Glock) => 0.7,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Birds | Breeze) => 0.6 + 0.4 * ramp(u, 0.2, 0.6),
        (Part::Fade, Drone) => 1.0,
        (Part::Fade, Strings) => 1.0 - ramp(u, 0.3, 0.8),
        (Part::Fade, Glow) => 1.0 - ramp(u, 0.0, 0.4),
        (Part::Fade, Piano) => 1.0 - ramp(u, 0.2, 0.6),
        (Part::Fade, Celesta) => 0.7 * (1.0 - ramp(u, 0.5, 0.85)),
        (Part::Fade, Glock) => 0.5 * (1.0 - ramp(u, 0.0, 0.5)),
    };
    v * end
}

/// How strongly and brightly everything plays: the light rising, then falling away.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2,
        Part::Neon => 0.3 + 0.2 * u,
        Part::Swell => 0.5 + 0.2 * u,
        Part::NightShift => 0.7 + 0.3 * (u / 0.6).min(1.0),
        Part::Fade => 0.8 - 0.6 * u,
    }
}

/// Canopy's instruments: the birds and the breeze wide, the drone and strings warm and
/// wide (the strings through the ensemble), the glow above them, the electric piano,
/// celesta and glockenspiel into the echo and the hall.
fn tracks(p: &mut Project) -> [usize; 8] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    let font = |file: &str, bank: u16, program: u8, name: &str| Source::SoundFont { font: file.into(), bank, program, name: name.into() };
    let city = || Source::Kit(KitKind::CityFx);
    [
        add("Birdsong", Role::City, city(), Strip { reverb: 0.25, width: 1.6, gain_db: -8.0, high_cut: 9000.0, ..s }),
        add("Breeze", Role::City, city(), Strip { reverb: 0.15, width: 1.8, gain_db: -5.0, ..s }),
        add("Drone", Role::Drone, synth("Meadow Drone"), Strip { reverb: 0.45, width: 1.5, gain_db: -9.0, ..s }),
        add("Strings", Role::Strings, synth("Dawn Strings"), Strip { reverb: 0.55, width: 1.7, ensemble: true, gain_db: -10.0, ..s }),
        add(
            "Glow",
            Role::Pad,
            font(AMBIENT_FONT, 0, 88, "Warm Analog Pad"),
            Strip { reverb: 0.6, width: 1.8, gain_db: -10.0, low_cut: 250.0, ..s },
        ),
        add(
            "Electric Piano",
            Role::Keys,
            font(GM_FONT, 8, 4, "Chorused EP"),
            Strip { reverb: 0.45, echo: 0.25, pan: -0.2, gain_db: 3.0, ..s },
        ),
        add("Celesta", Role::Bells, font(GM_FONT, 0, 8, "Celesta"), Strip { reverb: 0.6, echo: 0.35, pan: 0.3, gain_db: 2.0, ..s }),
        add(
            "Glockenspiel",
            Role::Bells,
            font(GM_FONT, 0, 9, "Glockenspiel"),
            Strip { reverb: 0.6, echo: 0.4, pan: -0.35, gain_db: 1.0, high_cut: 8000.0, ..s },
        ),
    ]
}

/// One note of a phrase: its start and length in crotchets from the phrase's start, and
/// how many steps of the scale it is from the phrase's first note.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Step {
    pub(crate) at: f64,
    pub(crate) len: f64,
    pub(crate) step: i32,
}

/// A few short, simple phrases for a seed: two to four notes each, stepping (now and
/// then leaping a third) and resting on a long last note, so a piece's phrases are kin.
pub(crate) fn phrases(seed: u64, name: &str) -> Vec<Vec<Step>> {
    let mut rng = Rng::stream(seed, name);
    const RHYTHMS: [&[(f64, f64)]; 6] = [
        &[(0.0, 1.5), (1.5, 0.5), (2.0, 2.0)],
        &[(0.0, 1.0), (1.0, 1.0), (2.0, 2.0)],
        &[(0.0, 2.0), (2.0, 2.0)],
        &[(0.5, 1.0), (1.5, 1.5), (3.0, 3.0)],
        &[(0.0, 1.0), (1.0, 3.0)],
        &[(0.0, 1.0), (1.0, 0.5), (1.5, 0.5), (2.0, 2.0)],
    ];
    (0..3)
        .map(|_| {
            let rhythm = *rng.pick(&RHYTHMS);
            let mut step = 0;
            rhythm
                .iter()
                .enumerate()
                .map(|(j, &(at, len))| {
                    if j > 0 {
                        step += *rng.pick(&[-2, -1, -1, 1, 1, 2]);
                    }
                    Step { at, len, step }
                })
                .collect()
        })
        .collect()
}

/// Play a phrase from a chord's note nearest `target`, on the scale `scale` (pitch
/// classes above the tonic), the last note settling on the chord: the notes as
/// (start, length, pitch above the tonic).
pub(crate) fn realise(phrase: &[Step], chord: Chord, scale: &[i32], target: i32, (lo, hi): (i32, i32)) -> Vec<(f64, f64, i32)> {
    let ladder = ladder(scale, lo, hi);
    let start = snap(target, &chord.pcs()).clamp(lo, hi);
    let i0 = ladder.iter().position(|&x| x >= start).unwrap_or(0) as i32;
    phrase
        .iter()
        .enumerate()
        .map(|(j, s)| {
            let i = (i0 + s.step).clamp(0, ladder.len() as i32 - 1) as usize;
            let mut rel = ladder[i];
            if j + 1 == phrase.len() {
                rel = snap(rel, &chord.pcs()).clamp(lo, hi);
            }
            (s.at, s.len, rel)
        })
        .collect()
}

/// A tape's wobble on a track, as pitch bends a demisemiquaver apart while its notes ring
/// (from a beat before each to `ring` ticks after): a slow sway and a slower drag, about
/// `cents` at most, for a bend range of two semitones.
pub(crate) fn wobble(ctrls: &mut Vec<Ctrl>, starts: &[u32], ring: u32, end: u32, beat: u32, secs_per_tick: f64, cents: f64) {
    let step = PPQ / 8;
    let mut ticks = BTreeSet::new();
    for &s in starts {
        let a = s.saturating_sub(beat) / step * step;
        let mut t = a;
        while t < (s + ring).min(end) {
            ticks.insert(t);
            t += step;
        }
    }
    let mut last = i16::MIN;
    for t in ticks {
        let secs = t as f64 * secs_per_tick;
        let c = cents * (0.7 * (std::f64::consts::TAU * 0.45 * secs).sin() + 0.3 * (std::f64::consts::TAU * 0.11 * secs + 1.7).sin());
        let v = (c / 200.0 * 8191.0).round() as i16;
        if v != last {
            ctrls.push(Ctrl { tick: t, kind: CtrlKind::Bend, value: v });
            last = v;
        }
    }
}

/// Write Canopy into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a soft, airy hall, an unhurried echo, a touch of tape over everything
    p.fx.space = Space::Hall;
    p.fx.size = 0.8;
    p.fx.tone = 0.45;
    p.fx.motion = 0.4;
    p.fx.predelay = 0.04;
    p.fx.low_cut = 0.4;
    p.fx.width = 1.0;
    p.fx.duck = 0.1;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.3;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.3;
    p.fx.echo.time = EchoTime::DottedQuarter;
    p.fx.echo.feedback = 0.35;
    p.fx.echo.tone = 0.4;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.5;
    p.fx.echo.wow = 0.2;
    p.master_db = 8.5;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "canopy-harmony");
    let prog = *hr.pick(&PROGS);
    let piano_phrases = phrases(r.seed, "canopy-piano");
    let celesta_phrases = phrases(r.seed, "canopy-celesta");
    let mut pr = Rng::stream(r.seed, "canopy-phrases");
    let mut gr = Rng::stream(r.seed, "canopy-glock");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "canopy-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [birds, breeze, drone, strings, glow, piano, celesta, glock] = tr;
    // the phrases' tonic, near middle C
    let tonic = pitch(0, 62);

    let (env, force) = envelopes::<8>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.65 + 0.35 * inten(t));
    let beats = |b: f64| (b * PPQ as f64).round() as u32;

    // the forest, all the way through
    w.note(birds, 0, end, fx::BIRDS as i32, 84.0);
    w.note(breeze, 0, end, fx::BREEZE as i32, 84.0);
    let mut glock_starts = Vec::new();
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the drone: the root and fifth through the section, overlapping the next
        let dl = (zz - a) + bar;
        w.note(drone, a, dl, pitch(0, 50), 78.0);
        w.note(drone, a, dl, pitch(7, 57), 64.0);
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
            // the strings: the chord, the root low, swelling in slowly
            if new_chord && pres(Layer::Strings, t_bar + bar) > 0.03 {
                for (j, pc) in pcs.iter().enumerate() {
                    w.note(strings, t_bar, len, pitch(*pc, [50, 57, 62, 66][j]), vel([72.0, 64.0, 64.0, 60.0][j], t_bar));
                }
            }
            // the glow: the chord's upper notes above the strings
            if new_chord && pres(Layer::Glow, t_bar + bar) > 0.03 {
                for (j, pc) in pcs.iter().skip(1).enumerate() {
                    w.note(glow, t_bar, len, pitch(*pc, [69, 74, 78][j]), vel(66.0, t_bar));
                }
            }
            let in_chord = k % 4;
            // the electric piano: a phrase now and then in a chord's first two bars
            let pp = pres(Layer::Piano, t_bar);
            if in_chord == 0 && pp > 0.15 && pr.chance(0.35 + 0.45 * pp) {
                let ph = pr.pick(&piano_phrases);
                let from = t_bar + beats(*pr.pick(&[0.0, 1.0, 4.0]));
                let target = *pr.pick(&[66, 69, 71]);
                for (j, (s, l, rel)) in realise(ph, chord, &LYDIAN, target - tonic, (-5, 17)).into_iter().enumerate() {
                    let t = from + beats(s);
                    if t < zz && pres(Layer::Piano, t) > 0.1 {
                        // (left to ring: the piano's own decay ends it)
                        w.note(piano, t, beats(l + 1.5), tonic + rel, vel(if j == 0 { 62.0 } else { 54.0 }, t));
                    }
                }
                // and a soft fifth under the phrase's first note, now and then
                if pr.chance(0.5) {
                    w.note(piano, from, beats(4.0), pitch(chord.root, 50), vel(44.0, from));
                    w.note(piano, from, beats(4.0), pitch(chord.root + 7, 57), vel(40.0, from));
                }
            }
            // the celesta: answering higher up in a chord's third bar
            let cp = pres(Layer::Celesta, t_bar);
            if in_chord == 2 && cp > 0.15 && pr.chance(0.3 + 0.4 * cp) {
                let ph = pr.pick(&celesta_phrases);
                let from = t_bar + beats(*pr.pick(&[0.0, 1.0, 2.0]));
                let target = *pr.pick(&[76, 79, 81]);
                for (s, l, rel) in realise(ph, chord, &LYDIAN, target - tonic, (9, 29)) {
                    let t = from + beats(s);
                    if t < zz && pres(Layer::Celesta, t) > 0.1 {
                        w.note(celesta, t, beats(l + 2.0), tonic + rel, vel(58.0, t));
                    }
                }
            }
            // the glockenspiel: one or two notes once in a while, in a chord's last bar
            let gp = pres(Layer::Glock, t_bar);
            if in_chord == 3 && gp > 0.15 && gr.chance(0.25 + 0.4 * gp) {
                let t = t_bar + beats(*gr.pick(&[1.0, 2.0]));
                if t < zz {
                    let first = pitch(*gr.pick(&pcs), 84);
                    w.note(glock, t, beats(3.0), first, gr.range_f(52.0, 62.0));
                    glock_starts.push(t);
                    if gr.chance(0.5) {
                        let second = snap(first + *gr.pick(&[-3, 4, 5]), &pcs.iter().map(|pc| pc + root).collect::<Vec<_>>());
                        w.note(glock, t + beats(1.5), beats(3.0), second, gr.range_f(46.0, 56.0));
                        glock_starts.push(t + beats(1.5));
                    }
                }
            }
        }
    }
    // the glockenspiel's tape wobble, about fifteen cents
    let secs_per_tick = 60.0 / p.bpm / PPQ as f64;
    wobble(&mut w.ctrls[glock], &glock_starts, 6 * beat, end, beat, secs_per_tick, 15.0);

    // expression a quaver at a time (the drone breathing), and the filters with the light
    let wave = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Birds | Layer::Breeze => 127.0 * pres(*layer, t),
                Layer::Drone => 127.0 * pres(*layer, t) * (0.8 + 0.2 * wave(t, 3.0, 0.0)),
                // (the keys and bells: their phrases come and go with the layer already)
                Layer::Piano | Layer::Celesta | Layer::Glock => 127.0 * (0.6 + 0.4 * pres(*layer, t)),
                _ => 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t)),
            };
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Drone => Some(52.0 + 16.0 * inten(t) + 6.0 * wave(t, 5.0, 0.3)),
                Layer::Strings => Some(46.0 + 30.0 * inten(t)),
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

    fn piece(seed: u64, bars: u32) -> Project {
        let mut r = Recipe { style: Style::Canopy, ..Recipe::random(seed) };
        Style::Canopy.settle(&mut r);
        compose(&r, Style::Canopy.tempo(), (4, 4), bars)
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the birds, the breeze and the drone alone at first, the strings and
    /// piano arriving, the glow at the fullest, the piano gone by the end and the drone
    /// and birds still there; D Lydian; no drums.
    #[test]
    fn canopy_wakes_and_settles() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            assert_eq!(p.recipe.map(|r| (r.root, r.mode)), Some((2, Mode::Lydian)));
            assert!(p.tracks.iter().all(|t| t.role != Some(Role::Drums)));
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            assert!(starts(&p, "Drone", b[0]) >= 2, "seed {seed}");
            for name in ["Electric Piano", "Glow", "Glockenspiel"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Strings", b[1]) >= 3, "seed {seed}");
            let busy = starts(&p, "Electric Piano", b[2]) + starts(&p, "Electric Piano", b[3]);
            assert!(busy >= 3, "seed {seed}: {busy}");
            assert!(starts(&p, "Glow", b[3]) >= 2, "seed {seed}");
            let tail = b[4].0 + (b[4].1 - b[4].0) * 2 / 3;
            for name in ["Electric Piano", "Glow", "Glockenspiel"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Drone", b[4]) >= 1, "seed {seed}");
            let birds = p.tracks.iter().find(|t| t.name == "Birdsong").unwrap();
            assert!(birds.notes.iter().any(|n| n.key == fx::BIRDS && n.start == 0 && n.end() == p.end_tick()));
        }
    }

    /// Sparse: the piano and celesta leave plenty of space, and every tuned note is in the
    /// Lydian mode.
    #[test]
    fn sparse_and_in_the_mode() {
        for seed in 0..20 {
            let p = piece(seed, 48);
            let bars = p.bars() as usize;
            for name in ["Electric Piano", "Celesta", "Glockenspiel"] {
                let t = p.tracks.iter().find(|t| t.name == name);
                let n = t.map_or(0, |t| t.notes.len());
                assert!(n <= bars * 2, "seed {seed} {name}: {n} notes in {bars} bars");
            }
            for t in p.tracks.iter().filter(|t| t.role != Some(Role::City)) {
                for n in &t.notes {
                    let pc = (n.key as i32 - 2).rem_euclid(12);
                    assert!(LYDIAN.contains(&pc), "seed {seed} {}: {}", t.name, n.key);
                }
            }
        }
    }

    /// The glockenspiel wobbles while it rings, never by more than about fifteen cents.
    #[test]
    fn the_glockenspiel_wobbles() {
        let mut any = false;
        for seed in 0..10 {
            let p = piece(seed, 48);
            if let Some(g) = p.tracks.iter().find(|t| t.name == "Glockenspiel") {
                let bends: Vec<i16> = g.ctrls.iter().filter(|c| c.kind == CtrlKind::Bend).map(|c| c.value).collect();
                any |= bends.iter().any(|v| v.abs() > 200);
                assert!(bends.iter().all(|v| v.abs() <= 620), "seed {seed}");
            }
        }
        assert!(any);
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
            let r = Recipe { style: Style::Canopy, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 60.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Canopy, ..Recipe::random(i as u64) }, 60.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Canopy, ..Recipe::random(7) }, 60.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
