//! Strata (D30): a deep drone score, vast, slow and heavy, at 50 bpm in 4/4 in the
//! Phrygian mode, with no beat.
//!
//! Out of a cold wind a drone rises from far below (basalt: saws and a sine in the
//! depths) and never stops. Over it a second drone holds the tonic and fifth while its
//! resonant filter climbs and falls through the harmonics, a slowly wandering whistle of
//! overtones. A sub swells in and out under each new chord; a choir of drones holds the
//! chords above (the flat second pressing on the tonic, the flat sixth and seventh);
//! the earth groans and distant metal clangs now and then; a far-off bell tolls. One
//! great slow climb (a long riser and a boom), the overtones at their widest, and
//! everything sinks back into the depths and the wind.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::fx;
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::compose::{Recipe, near};
use crate::nebula::{Writer, envelopes, ramp};
use crate::palette::Role;
use crate::project::{PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;

/// The sounds added to the palette for Strata, Lumen and Mantra: the other styles leave
/// them out of their choosing, so every recipe they had still sounds as it did.
pub(crate) const DRONE_SOUNDS: [&str; 5] = ["Basalt Drone", "Overtone Drone", "Radiant Drone", "Prism Drone", "Buzz Drone"];

/// The usual arrangement: the wind and the depths, the overtones, the choir, the great
/// climb, and sinking back, shared in proportion.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [6, 8, 8, 8, 8] } else { [2, 3, 3, 4, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Chord {
    pub(crate) root: i32,
    pub(crate) ivs: &'static [i32],
}

impl Chord {
    pub(crate) fn pcs(self) -> Vec<i32> {
        self.ivs.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }
}

const I: Chord = Chord { root: 0, ivs: &[0, 3, 7] };
const II: Chord = Chord { root: 1, ivs: &[0, 4, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 3, 7] };
const VI: Chord = Chord { root: 8, ivs: &[0, 4, 7] };
const VII: Chord = Chord { root: 10, ivs: &[0, 3, 7] };

/// Phrygian progressions, four bars a chord, over a drone that never leaves the tonic:
/// the flat second pressing down on it, the flat sixth and the minor seventh.
const PROGS: [[Chord; 4]; 4] = [[I, II, I, VII], [I, VI, II, I], [I, II, VI, VII], [I, IV, II, I]];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Wind,
    Bedrock,
    Overtones,
    Choir,
    Sub,
    Depths,
    Metal,
    Bell,
}

const LAYERS: [Layer; 8] =
    [Layer::Wind, Layer::Bedrock, Layer::Overtones, Layer::Choir, Layer::Sub, Layer::Depths, Layer::Metal, Layer::Bell];

/// How present a layer is `u` of the way through a section of this part: the wind and
/// the depths, the overtones rising out of them, the choir and sub, everything at the
/// climb, and back to the depths and the wind, all fading at the very end.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.75, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (Part::Intro, Wind) => 1.0,
        (Part::Intro, Bedrock) => ramp(u, -0.1, 0.5),
        (Part::Intro, Depths) => 0.6,
        (Part::Intro, _) => 0.0,
        (Part::Neon, Wind) => 1.0 - 0.5 * u,
        (Part::Neon, Overtones) => ramp(u, -0.05, 0.5),
        (Part::Neon, Sub) => ramp(u, 0.4, 0.8),
        (Part::Neon, Choir | Bell) => 0.0,
        (Part::Neon, Metal) => 0.6,
        (Part::Neon, _) => 1.0,
        (Part::Swell, Wind) => 0.4,
        (Part::Swell, Bell) => 0.0,
        (Part::Swell, Choir) => ramp(u, -0.1, 0.5),
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Wind) => 0.3,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Wind) => 0.3 + 0.7 * ramp(u, 0.1, 0.5),
        (Part::Fade, Bedrock | Depths) => 1.0,
        (Part::Fade, Overtones) => 1.0 - ramp(u, 0.3, 0.75),
        (Part::Fade, Choir) => 1.0 - ramp(u, 0.0, 0.4),
        (Part::Fade, Sub) => 1.0 - ramp(u, 0.1, 0.5),
        (Part::Fade, Metal) => 0.5,
        (Part::Fade, Bell) => 1.0 - ramp(u, 0.3, 0.6),
    };
    v * end
}

/// How strongly and brightly everything plays: a slow climb to the fullest part and
/// back down into the dark.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.15,
        Part::Neon => 0.2 + 0.15 * u,
        Part::Swell => 0.35 + 0.25 * u,
        Part::NightShift => 0.6 + 0.4 * (u / 0.6).min(1.0),
        Part::Fade => 0.85 - 0.7 * u,
    }
}

/// Strata's instruments: the wind wide, the bedrock and sub centred, the overtones and
/// choir wide in the reverb, the groans, metal and bell far back.
fn tracks(p: &mut Project) -> [usize; 8] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    [
        add("Wind", Role::Pad, synth("Solar Wind"), Strip { reverb: 0.5, width: 1.9, gain_db: -6.0, ..s }),
        add("Bedrock", Role::Drone, synth("Basalt Drone"), Strip { reverb: 0.4, width: 1.3, gain_db: -1.0, ..s }),
        add("Overtones", Role::Drone, synth("Overtone Drone"), Strip { reverb: 0.65, echo: 0.2, width: 1.7, gain_db: -5.0, ..s }),
        add("Choir", Role::Drone, synth("Choir Drone"), Strip { reverb: 0.75, width: 1.8, gain_db: -14.0, ..s }),
        add("Sub", Role::Bass, synth("Pure Sub"), Strip { reverb: 0.0, gain_db: -1.0, high_cut: 300.0, ..s }),
        add("Depths", Role::City, Source::Kit(KitKind::CityFx), Strip { reverb: 0.6, gain_db: -9.0, high_cut: 3000.0, ..s }),
        add(
            "Metal",
            Role::City,
            Source::Kit(KitKind::CityFx),
            Strip { reverb: 0.85, echo: 0.3, pan: 0.35, width: 1.5, gain_db: -7.0, low_cut: 150.0, ..s },
        ),
        add("Bell", Role::Bells, synth("Ring Bells"), Strip { reverb: 0.85, echo: 0.35, pan: -0.3, gain_db: -7.0, ..s }),
    ]
}

/// Write Strata into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // the deepest space there is, a long dark echo, no tape
    p.fx.space = Space::BlackHole;
    p.fx.size = 0.9;
    p.fx.tone = 0.25;
    p.fx.motion = 0.4;
    p.fx.predelay = 0.08;
    p.fx.low_cut = 0.3;
    p.fx.width = 1.0;
    p.fx.duck = 0.0;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.0;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.0;
    p.fx.echo.time = EchoTime::Half;
    p.fx.echo.feedback = 0.45;
    p.fx.echo.tone = 0.2;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.7;
    p.fx.echo.wow = 0.15;
    p.master_db = 7.5;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "strata-harmony");
    let prog = *hr.pick(&PROGS);
    let mut ar = Rng::stream(r.seed, "strata-air");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "strata-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [wind, bedrock, overtones, choir, sub, depths, metal, bell] = tr;

    let (env, force) = envelopes::<8>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.6 + 0.4 * inten(t));

    // the wind under everything
    w.note(wind, 0, end, pitch(0, 62), 70.0);
    let mut climbed = false;
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the bedrock: the tonic in the depths through the section, overlapping the next;
        // its octave too once it builds
        let dl = (zz - a) + bar;
        w.note(bedrock, a, dl, pitch(0, 31), 80.0);
        if matches!(part, Part::Swell | Part::NightShift) {
            w.note(bedrock, a, dl, pitch(0, 43), 60.0);
        }
        // the overtones: the tonic and the fifth, the filter wandering through them
        // (from the Neon: the intro is the depths alone)
        if part != Part::Intro {
            w.note(overtones, a, dl, pitch(0, 43), 76.0);
            w.note(overtones, a, dl, pitch(7, 50), 58.0);
        }
        // the great climb: a long riser into the night shift and a boom as it lands
        if part == Part::NightShift && !climbed {
            climbed = true;
            let rise = p.secs_tick(dd_dsp::kit::RISER_LONG_SECS);
            if a >= rise {
                w.note(depths, a - rise, rise, fx::RISER_LONG as i32, 70.0);
            }
            w.note(depths, a, 4 * bar, fx::BOOM as i32 + pitch(0, 6).rem_euclid(12), 88.0);
        }
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let new_chord = k == 0 || chord_at(k) != chord_at(k - 1) || k % 4 == 0;
            // the choir: the chord held high and wide, four bars at a time
            if new_chord && pres(Layer::Choir, t_bar + bar) > 0.03 {
                let span = (4 - k % 4).min(nb - k) * bar;
                let len = span.min(zz.saturating_sub(t_bar) + bar / 2);
                for (j, pc) in chord.pcs().into_iter().enumerate() {
                    w.note(choir, t_bar, len, pitch(pc, [57, 64, 60][j]), vel([72.0, 64.0, 60.0][j], t_bar));
                }
            }
            // the sub swelling in under each new chord
            if new_chord && pres(Layer::Sub, t_bar) > 0.03 {
                let len = (3 * bar).min(zz.saturating_sub(t_bar) + bar / 2);
                w.note(sub, t_bar, len, pitch(chord.root, 31), vel(96.0, t_bar));
            }
            // the earth groaning, now and then
            if k % 4 == 2 && pres(Layer::Depths, t_bar) > 0.3 && ar.chance(0.5) {
                let s = t_bar + ar.range(0, 3) as u32 * beat;
                w.note(depths, s, 2 * bar, fx::HULL as i32 + pitch(0, 6) % 12, ar.range_f(60.0, 80.0));
            }
            // distant metal
            if pres(Layer::Metal, t_bar) > 0.2 && ar.chance(0.12 + 0.15 * inten(t_bar)) {
                let s = t_bar + ar.range(0, 3) as u32 * beat;
                let key = if ar.chance(0.3) { fx::SCRAPE } else { fx::METAL };
                w.note(metal, s, 2 * bar, key as i32, ar.range_f(50.0, 80.0));
            }
            // a far-off bell tolling the tonic at the start of each four bars
            if k % 4 == 0 && pres(Layer::Bell, t_bar) > 0.2 {
                w.note(bell, t_bar, 2 * bar, pitch(0, 60), vel(70.0, t_bar));
                if inten(t_bar) > 0.6 {
                    w.note(bell, t_bar + 2 * beat, 2 * bar, pitch(7, 55), vel(54.0, t_bar));
                }
            }
        }
    }
    // expression a quaver at a time; the overtone filter wandering through the harmonics,
    // further as it climbs; the bedrock and choir opening a little
    let wave = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        if matches!(layer, Layer::Depths | Layer::Metal) {
            // (one-shots: their level is in their velocity)
            w.cc(track, 0, 11, 127.0);
            continue;
        }
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Wind => 127.0 * pres(*layer, t),
                _ => 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t)),
            };
            // (never faster than a step of eight a quaver)
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                // two slow waves against each other: a wandering, never repeating line
                Layer::Overtones => {
                    let reach = 18.0 + 26.0 * inten(t);
                    Some(60.0 + 8.0 * inten(t) + reach * (0.65 * wave(t, 7.0, 0.0) + 0.35 * wave(t, 3.0, 0.4)))
                }
                Layer::Bedrock => Some(48.0 + 26.0 * inten(t) + 8.0 * wave(t, 11.0, 0.2)),
                Layer::Choir => Some(46.0 + 30.0 * inten(t)),
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
        compose(&Recipe { style: Style::Strata, ..Recipe::random(seed) }, Style::Strata.tempo(), (4, 4), bars)
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the wind and the bedrock alone at first, the overtones and sub, the
    /// choir, the great climb landing on a boom; no beat at all; Phrygian.
    #[test]
    fn strata_climbs_once_and_sinks() {
        for seed in 0..12 {
            let p = piece(seed, 38);
            assert_eq!(p.recipe.map(|r| r.mode), Some(crate::compose::Mode::Phrygian));
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            assert!(starts(&p, "Bedrock", b[0]) >= 1, "seed {seed}");
            for name in ["Overtones", "Choir", "Sub", "Bell"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Choir", b[2]) >= 2, "seed {seed}");
            assert!(starts(&p, "Bell", b[3]) >= 1, "seed {seed}");
            let depths = p.tracks.iter().find(|t| t.name == "Depths").unwrap();
            assert_eq!(depths.notes.iter().filter(|n| n.key / 12 == fx::BOOM / 12 && n.start == b[3].0).count(), 1, "seed {seed}");
            assert!(p.tracks.iter().all(|t| t.role != Some(Role::Drums)));
            // the overtones' filter roams widely
            let ov = p.tracks.iter().find(|t| t.name == "Overtones").unwrap();
            let bright: Vec<i16> = ov.ctrls.iter().filter(|c| c.kind == CtrlKind::Cc(74)).map(|c| c.value).collect();
            let (lo, hi) = (bright.iter().min().copied().unwrap_or(0), bright.iter().max().copied().unwrap_or(0));
            assert!(hi - lo > 50, "seed {seed}: {lo}..{hi}");
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
            let r = Recipe { style: Style::Strata, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 50.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Strata, ..Recipe::random(i as u64) }, 50.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Strata, ..Recipe::random(7) }, 50.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
