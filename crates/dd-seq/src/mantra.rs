//! Mantra (D30): a hypnotic, ritual drone score at 72 bpm in 4/4, in the harmonic minor.
//!
//! A buzzing plucked drone turns in an endless four-note cycle (the fifth below, the
//! tonic twice, the tonic low) over a deep hum that never stops. A frame drum and two
//! hand drums take up a looping pattern, a shaker joins as it grows; a voice-like lead
//! sings slow phrases that slide from note to note and come to rest on the tonic or the
//! fifth; a choir of voices holds the chords (the flat sixth, the major fifth with its
//! raised leading note) and a bell marks each new cycle at the height. Then the drums
//! fall away, the voice falls silent, and the cycle turns on alone into the hum.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::drum;
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::compose::{Recipe, near};
use crate::nebula::{Writer, envelopes, ladder, ramp};
use crate::palette::Role;
use crate::project::{PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;
use crate::strata::Chord;

/// The usual arrangement: the cycle and the hum, the drums taking it up, the voice,
/// the fullest it gets, and the cycle alone again, shared in proportion.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 12, 12, 8] } else { [2, 3, 4, 5, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

const I: Chord = Chord { root: 0, ivs: &[0, 3, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 3, 7] };
const V: Chord = Chord { root: 7, ivs: &[0, 4, 7] };
const VI: Chord = Chord { root: 8, ivs: &[0, 4, 7] };

/// The choir's chords, four bars each, always back to the tonic the cycle never leaves.
const PROGS: [[Chord; 4]; 4] = [[I, VI, I, V], [I, IV, VI, V], [I, VI, IV, I], [I, V, VI, V]];

/// The harmonic minor, the voice's scale.
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 8, 11];

/// The drums' loops, in semiquavers of the bar: the frame drum, the low hand drum, and
/// the high one's strokes (played as it grows).
struct Loop {
    frame: &'static [u32],
    low: &'static [u32],
    high: &'static [u32],
}

const LOOPS: [Loop; 4] = [
    Loop { frame: &[0, 6, 10], low: &[3, 11], high: &[2, 4, 7, 12, 14, 15] },
    Loop { frame: &[0, 7, 12], low: &[4, 10], high: &[2, 3, 6, 9, 14, 15] },
    Loop { frame: &[0, 3, 8, 11], low: &[6, 14], high: &[2, 5, 7, 10, 13, 15] },
    Loop { frame: &[0, 10], low: &[3, 6, 12], high: &[1, 4, 8, 9, 13, 14] },
];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Cycle,
    Hum,
    Drums,
    Sub,
    Voice,
    Choir,
    Bell,
}

const LAYERS: [Layer; 7] = [Layer::Cycle, Layer::Hum, Layer::Drums, Layer::Sub, Layer::Voice, Layer::Choir, Layer::Bell];

/// How present a layer is `u` of the way through a section of this part.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.78, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (_, Cycle) => 1.0,
        (Part::Intro, Hum) => ramp(u, -0.1, 0.5),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Hum) => 1.0,
        (Part::Neon, Drums) => ramp(u, 0.2, 0.5),
        (Part::Neon, Sub) => ramp(u, 0.5, 0.8),
        (Part::Neon, Voice) => ramp(u, 0.85, 1.0),
        (Part::Neon, _) => 0.0,
        (Part::Swell, Choir) => ramp(u, 0.2, 0.6),
        (Part::Swell, Bell) => 0.0,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Hum) => 1.0,
        (Part::Fade, Drums) => 1.0 - ramp(u, 0.0, 0.35),
        (Part::Fade, Sub) => 1.0 - ramp(u, 0.0, 0.35),
        (Part::Fade, Voice) => 1.0 - ramp(u, 0.15, 0.4),
        (Part::Fade, Choir) => 1.0 - ramp(u, 0.0, 0.5),
        (Part::Fade, Bell) => 1.0 - ramp(u, 0.2, 0.5),
    };
    v * end
}

/// How strongly everything plays: the trance deepening to the height, then letting go.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2 + 0.05 * u,
        Part::Neon => 0.3 + 0.15 * u,
        Part::Swell => 0.45 + 0.2 * u,
        Part::NightShift => 0.65 + 0.35 * (u / 0.6).min(1.0),
        Part::Fade => 0.8 - 0.6 * u,
    }
}

/// Mantra's instruments: the cycle a little left and the voice a little right, the hum
/// and drums centred and close, the choir wide and far back, the bell into the echo.
fn tracks(p: &mut Project) -> [usize; 7] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    [
        add("Cycle", Role::Drone, synth("Buzz Drone"), Strip { reverb: 0.4, echo: 0.1, pan: -0.2, width: 1.4, gain_db: -9.0, ..s }),
        add("Hum", Role::Drone, synth("Abyss Drone"), Strip { reverb: 0.35, width: 1.2, gain_db: -7.0, ..s }),
        add("Drums", Role::Drums, Source::Kit(KitKind::Drums), Strip { reverb: 0.3, gain_db: -7.0, ..s }),
        add("Sub", Role::Bass, synth("Pure Sub"), Strip { reverb: 0.0, gain_db: -1.0, high_cut: 350.0, ..s }),
        add("Voice", Role::Lead, synth("Ribbon Lead"), Strip { reverb: 0.55, echo: 0.35, pan: 0.2, gain_db: -9.0, ..s }),
        add("Choir", Role::Pad, synth("Night Choir"), Strip { reverb: 0.75, width: 1.8, gain_db: -12.0, ..s }),
        add("Bell", Role::Bells, synth("Night Bells"), Strip { reverb: 0.75, echo: 0.45, pan: -0.35, gain_db: -11.0, ..s }),
    ]
}

/// One note of the voice: its start and length in crotchets from the start of its
/// four-bar phrase, and its pitch above the voice's tonic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MNote {
    at: f64,
    len: f64,
    rel: i32,
}

const LOW: i32 = -1;
const HIGH: i32 = 15;

/// One phrase of the voice: long notes stepping up from the tonic or fifth, turning
/// around a note or two, and coming to rest on the tonic or the fifth. `lift` raises
/// where it begins (a step up the scale for each time through the height).
fn phrase(rng: &mut Rng, lift: usize, beats_per_bar: f64) -> Vec<MNote> {
    // (at, len) in bars over the first three of four
    const RHYTHMS: [&[(f64, f64)]; 4] = [
        &[(0.0, 0.75), (0.75, 0.25), (1.0, 0.5), (1.5, 0.5), (2.0, 1.0)],
        &[(0.0, 0.5), (0.5, 0.5), (1.0, 1.0), (2.0, 0.25), (2.25, 0.75)],
        &[(0.25, 0.75), (1.0, 0.5), (1.5, 0.25), (1.75, 0.25), (2.0, 1.0)],
        &[(0.0, 1.0), (1.0, 0.5), (1.5, 0.5), (2.0, 1.0)],
    ];
    let rhythm = *rng.pick(&RHYTHMS);
    let scale = ladder(&SCALE, LOW, HIGH);
    let idx = |p: i32| scale.iter().position(|&x| x >= p).unwrap_or(scale.len() - 1);
    let mut i = (idx(*rng.pick(&[0, 7, 0])) + lift).min(scale.len() - 2);
    let mut out = Vec::new();
    for (j, &(at, len)) in rhythm.iter().enumerate() {
        let rel = if j + 1 == rhythm.len() {
            // home: the tonic or fifth nearest
            *[0, 7, 12].iter().min_by_key(|&&h| (h - scale[i]).abs()).unwrap_or(&0)
        } else {
            if j > 0 {
                let step = *rng.pick(&[1, 1, -1, 2, -1]);
                i = (i as i32 + step).clamp(0, scale.len() as i32 - 1) as usize;
            }
            scale[i]
        };
        out.push(MNote { at: at * beats_per_bar, len: len * beats_per_bar, rel });
    }
    out
}

/// Write Mantra into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a resonant stone hall, a short echo, a little warmth of tape
    p.fx.space = Space::Cathedral;
    p.fx.size = 0.65;
    p.fx.tone = 0.45;
    p.fx.motion = 0.3;
    p.fx.predelay = 0.04;
    p.fx.low_cut = 0.45;
    p.fx.width = 1.0;
    p.fx.duck = 0.2;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.0;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.15;
    p.fx.echo.time = EchoTime::DottedEighth;
    p.fx.echo.feedback = 0.35;
    p.fx.echo.tone = 0.4;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.4;
    p.fx.echo.wow = 0.1;
    p.master_db = 5.5;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let s16 = PPQ / 4;
    let steps = (bar / s16).max(1);
    let beats_per_bar = bar as f64 / PPQ as f64;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "mantra-harmony");
    let prog = *hr.pick(&PROGS);
    let lp = hr.pick(&LOOPS);
    let mut vr = Rng::stream(r.seed, "mantra-voice");
    let mut dr = Rng::stream(r.seed, "mantra-drums");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "mantra-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [cycle, hum, drums, sub, voice, choir, bell] = tr;
    let voice_tonic = pitch(0, 62);

    let (env, force) = envelopes::<7>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.6 + 0.4 * inten(t));

    // the cycle: a pluck a beat, round and round from the first beat to the last
    let sa = pitch(0, 50);
    let turn = [pitch(7, 43), sa, sa, sa - 12];
    let mut t = 0;
    let mut i = 0usize;
    while t < end {
        let accent = if i.is_multiple_of(4) { 1.0 } else { 0.88 };
        w.note(cycle, t, 2 * beat, turn[i % 4], vel(92.0 * accent, t));
        t += beat;
        i += 1;
    }
    let mut height = 0usize;
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the hum: the tonic and fifth, low, through the section and into the next
        let dl = (zz - a) + bar;
        w.note(hum, a, dl, pitch(0, 38), 76.0);
        w.note(hum, a, dl, pitch(7, 45), 56.0);
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let new_chord = k == 0 || chord_at(k) != chord_at(k - 1) || k % 4 == 0;
            // the choir holding the chord
            if new_chord && pres(Layer::Choir, t_bar + bar) > 0.03 {
                let span = (4 - k % 4).min(nb - k) * bar;
                let len = span.min(zz.saturating_sub(t_bar) + bar / 2);
                for (j, pc) in chord.pcs().into_iter().enumerate() {
                    w.note(choir, t_bar, len, pitch(pc, [55, 62, 59][j]), vel([66.0, 60.0, 58.0][j], t_bar));
                }
            }
            // the bell at the start of each four bars at the height
            if k % 4 == 0 && pres(Layer::Bell, t_bar) > 0.2 {
                w.note(bell, t_bar, 2 * bar, pitch(0, 72), vel(70.0, t_bar));
            }
            // the drums' loop, fuller as it grows
            if pres(Layer::Drums, t_bar) > 0.04 {
                let x = inten(t_bar);
                for &st in lp.frame.iter().filter(|&&st| st < steps) {
                    let t = t_bar + st * s16;
                    w.note(drums, t, s16 * 2, drum::TOM_LOW as i32, if st == 0 { 100.0 } else { 84.0 });
                    if pres(Layer::Sub, t) > 0.03 {
                        let len = if st == 0 { 4 * s16 } else { 2 * s16 };
                        w.note(sub, t, len, pitch(chord.root, 33), if st == 0 { 96.0 } else { 80.0 });
                    }
                }
                if x > 0.35 {
                    for &st in lp.low.iter().filter(|&&st| st < steps) {
                        w.note(drums, t_bar + st * s16, s16, drum::TABLA_LOW as i32, vel(84.0, t_bar));
                    }
                }
                for &st in lp.high.iter().filter(|&&st| st < steps) {
                    if dr.chance(0.25 + 0.7 * x) {
                        w.note(drums, t_bar + st * s16, s16, drum::TABLA_HIGH as i32, dr.range_f(48.0, 72.0) * (0.7 + 0.3 * x));
                    }
                }
                if x > 0.6 {
                    for st in 0..steps {
                        let accent = if st % 2 == 0 { 1.0 } else { 0.7 };
                        w.note(drums, t_bar + st * s16, s16, drum::SHAKER as i32, dr.range_f(34.0, 46.0) * accent);
                    }
                }
            }
        }
        // the voice: a phrase every four bars, at the height every time and rising,
        // otherwise every other; the notes held into the next, so it slides
        if matches!(part, Part::Swell | Part::NightShift | Part::Fade) {
            let mut ph = a;
            let mut n_ph = 0u32;
            while ph < zz {
                let sing = part == Part::NightShift || n_ph.is_multiple_of(2);
                let lift = if part == Part::NightShift { height.min(4) } else { 0 };
                let notes = phrase(&mut vr, lift, beats_per_bar);
                if sing {
                    for (j, n) in notes.iter().enumerate() {
                        let s = ph + (n.at * PPQ as f64).round() as u32;
                        if s >= zz || pres(Layer::Voice, s) < 0.3 {
                            continue;
                        }
                        let len = (n.len * PPQ as f64).round() as u32;
                        let len = if j + 1 < notes.len() { len + s16 } else { len };
                        w.note(voice, s, len, voice_tonic + n.rel, vel(84.0, s));
                    }
                    if part == Part::NightShift {
                        height += 1;
                    }
                }
                n_ph += 1;
                ph += 4 * bar;
            }
        }
    }
    // expression a quaver at a time, and the cycle's buzz brightening with the trance
    let wave = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t));
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Cycle => Some(48.0 + 30.0 * inten(t) + 6.0 * wave(t, 8.0, 0.0)),
                Layer::Hum => Some(50.0 + 20.0 * inten(t)),
                Layer::Choir => Some(46.0 + 26.0 * inten(t)),
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
        compose(&Recipe { style: Style::Mantra, ..Recipe::random(seed) }, Style::Mantra.tempo(), (4, 4), bars)
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the cycle alone at first and on every beat to the end; the drums and
    /// voice in the middle, gone by the end; the voice resting on the tonic or fifth.
    #[test]
    fn mantra_turns_and_lets_go() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            assert_eq!(p.recipe.map(|r| r.mode), Some(crate::compose::Mode::HarmonicMinor));
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            let beats = p.end_tick() / p.beat_ticks();
            assert_eq!(starts(&p, "Cycle", (0, p.end_tick())) as u32, beats, "seed {seed}");
            for name in ["Drums", "Voice", "Choir", "Sub", "Bell"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            for i in [2, 3] {
                assert!(starts(&p, "Drums", b[i]) > 30, "seed {seed}");
                assert!(starts(&p, "Voice", b[i]) >= 4, "seed {seed}");
            }
            assert!(starts(&p, "Bell", b[3]) >= 1, "seed {seed}");
            let tail = b[4].0 + (b[4].1 - b[4].0) / 2;
            for name in ["Drums", "Voice", "Sub"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
        }
    }

    /// Each phrase keeps to the scale and comes home to the tonic or the fifth.
    #[test]
    fn phrases_come_home() {
        let mut rng = Rng::new(3);
        for lift in 0..6 {
            for _ in 0..50 {
                let ph = phrase(&mut rng, lift, 4.0);
                assert!(ph.iter().all(|n| (LOW..=HIGH).contains(&n.rel) && SCALE.contains(&n.rel.rem_euclid(12))));
                assert!([0, 7, 12].contains(&ph.last().map_or(-1, |n| n.rel)));
                assert!(ph.last().map_or(0.0, |n| n.at + n.len) <= 12.0);
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
            let r = Recipe { style: Style::Mantra, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 72.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Mantra, ..Recipe::random(i as u64) }, 72.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Mantra, ..Recipe::random(7) }, 72.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
