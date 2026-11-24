//! Cinder (D29): dark ambient electronica after the end of the world: desolate,
//! cinematic, ominous but quiet, at 80 bpm in 4/4, in a natural minor key.
//!
//! Out of tape hiss and a far-off radio between stations a long drone rises, and
//! distant voices drift in and out of it as if through a wall. Wide, detuned analogue
//! pad chords move slowly, four bars a chord. A sparse half-time broken beat enters (a
//! dull kick placed differently every few bars, a dry, muffled snare on the third beat)
//! with a deep sub-bass under it; then hollow, brass-like synth stabs echoing into the
//! dark, and a minimal melody of a few long notes, heard once and then left out for as
//! long again, so there is a great deal of empty space. It builds a little, never loud,
//! and falls back to the drone and the static. The whole mix plays through worn tape:
//! wow and flutter, saturation, a dulled top and hiss.

use dd_dsp::fx::{EchoTime, Space};
use dd_dsp::kit::{drum, fx};
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::compose::{Recipe, near};
use crate::demo::GM_FONT;
use crate::nebula::{Writer, envelopes, ladder, ramp, snap};
use crate::palette::Role;
use crate::project::{Ctrl, CtrlKind, PPQ, Part, Project, Source, Strip, Track};
use crate::rng::Rng;

/// The sounds added to the palette for Cinder: the other styles leave them out of their
/// choosing, so every recipe they had still sounds as it did.
pub(crate) const CINDER_SOUNDS: [&str; 2] = ["Ashen Pad", "Hollow Stab"];

/// The usual arrangement: the static and the drone, the beat entering, the stabs and
/// melody, the fullest it gets, and back to the static, shared in proportion.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 12, 12, 8] } else { [2, 3, 4, 5, 4] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Chord {
    root: i32,
    ivs: &'static [i32],
}

impl Chord {
    fn pcs(self) -> Vec<i32> {
        self.ivs.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }
}

const I: Chord = Chord { root: 0, ivs: &[0, 3, 7] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7] };
const IV: Chord = Chord { root: 5, ivs: &[0, 3, 7] };
const V: Chord = Chord { root: 7, ivs: &[0, 3, 7] };
const VI: Chord = Chord { root: 8, ivs: &[0, 4, 7] };
const VII: Chord = Chord { root: 10, ivs: &[0, 4, 7] };

/// Slow, dark minor progressions, four bars a chord: the flat sixth looming over the
/// tonic, the minor fourth and fifth, never a leading note.
const PROGS: [[Chord; 4]; 5] = [[I, VI, IV, I], [I, VII, VI, V], [I, IV, VI, V], [I, VI, I, VII], [I, III, VI, IV]];

/// The natural minor, the melody's scale.
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Where the kick falls, in semiquavers of the bar: always the downbeat, then broken,
/// each pattern held for four bars.
const KICKS: [&[u32]; 6] = [&[0, 10], &[0, 7, 10], &[0, 3, 11], &[0, 6, 9], &[0, 11, 14], &[0, 7]];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Hiss,
    Radio,
    Drone,
    Pad,
    Sub,
    Drums,
    Stabs,
    Voices,
    Lead,
}

const LAYERS: [Layer; 9] =
    [Layer::Hiss, Layer::Radio, Layer::Drone, Layer::Pad, Layer::Sub, Layer::Drums, Layer::Stabs, Layer::Voices, Layer::Lead];

/// How present a layer is `u` of the way through a section of this part: the static and
/// drone alone at first, voices far off; the pad and then the beat and sub entering;
/// stabs and the melody; and back to the static, everything fading at the very end.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.75, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (_, Hiss) => 1.0,
        (Part::Intro, Radio) => 1.0,
        (Part::Intro, Drone) => ramp(u, -0.1, 0.4),
        (Part::Intro, Voices) => 0.7 * ramp(u, 0.3, 0.8),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Radio) => 0.8 - 0.5 * u,
        (Part::Neon, Drone) => 1.0,
        (Part::Neon, Pad) => ramp(u, -0.05, 0.4),
        (Part::Neon, Sub) => ramp(u, 0.35, 0.6),
        (Part::Neon, Drums) => ramp(u, 0.45, 0.6),
        (Part::Neon, Voices) => 0.5,
        (Part::Neon, _) => 0.0,
        (Part::Swell, Radio) => 0.3,
        (Part::Swell, Stabs) => ramp(u, -0.1, 0.3),
        (Part::Swell, Voices) => 0.5,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Radio) => 0.25,
        (Part::NightShift, Voices) => 0.8,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Radio) => 0.3 + 0.7 * ramp(u, 0.2, 0.6),
        (Part::Fade, Drone) => 1.0,
        (Part::Fade, Pad) => 1.0 - ramp(u, 0.3, 0.8),
        (Part::Fade, Sub) => 1.0 - ramp(u, 0.0, 0.3),
        (Part::Fade, Drums) => 1.0 - ramp(u, 0.0, 0.25),
        (Part::Fade, Stabs) => 1.0 - ramp(u, 0.0, 0.3),
        (Part::Fade, Voices) => 0.7,
        (Part::Fade, Lead) => 1.0 - ramp(u, 0.1, 0.35),
    };
    v * end
}

/// How strongly and brightly everything plays: rising a little to the fullest part,
/// never loud, and falling away.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.2,
        Part::Neon => 0.3 + 0.15 * u,
        Part::Swell => 0.45 + 0.15 * u,
        Part::NightShift => 0.6 + 0.3 * (u / 0.7).min(1.0),
        Part::Fade => 0.8 - 0.6 * u,
    }
}

/// Cinder's instruments: the hiss and radio, the drone and pad wide, the sub and beat
/// dry and centred (the snare muffled), the stabs and melody into the echo, the voices
/// thinned and far back in the reverb.
fn tracks(p: &mut Project) -> [usize; 9] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    let choir = Source::SoundFont { font: GM_FONT.into(), bank: 0, program: 52, name: "Choir".into() };
    [
        add("Tape Hiss", Role::City, Source::Kit(KitKind::CityFx), Strip { reverb: 0.0, gain_db: -9.0, high_cut: 9000.0, ..s }),
        add(
            "Radio",
            Role::City,
            Source::Kit(KitKind::CityFx),
            Strip { reverb: 0.35, echo: 0.2, pan: -0.35, width: 0.6, gain_db: -5.0, low_cut: 300.0, high_cut: 3500.0, ..s },
        ),
        add("Drone", Role::Drone, synth("Abyss Drone"), Strip { reverb: 0.5, width: 1.4, gain_db: -7.0, ..s }),
        add("Pad", Role::Pad, synth("Ashen Pad"), Strip { reverb: 0.6, echo: 0.1, width: 1.9, gain_db: -12.0, ..s }),
        add("Sub", Role::Bass, synth("Pure Sub"), Strip { reverb: 0.0, gain_db: 2.0, high_cut: 400.0, ..s }),
        add("Drums", Role::Drums, Source::Kit(KitKind::Drums), Strip { reverb: 0.1, gain_db: -7.0, high_cut: 2200.0, ..s }),
        add("Stabs", Role::Brass, synth("Hollow Stab"), Strip { reverb: 0.6, echo: 0.45, pan: 0.15, width: 1.4, gain_db: -5.0, ..s }),
        add(
            "Voices",
            Role::Answer,
            choir,
            Strip { reverb: 0.9, echo: 0.5, pan: -0.3, width: 1.6, gain_db: -3.0, low_cut: 350.0, high_cut: 2400.0, ..s },
        ),
        add("Lead", Role::Lead, synth("Glass Lead"), Strip { reverb: 0.65, echo: 0.5, pan: 0.2, gain_db: -8.0, ..s }),
    ]
}

/// One note of the melody: where it starts and how long it lasts, in crotchets from the
/// start of its eight-bar phrase, and its pitch above the melody's tonic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MNote {
    at: f64,
    len: f64,
    rel: i32,
}

/// The melody's lowest and highest, above its tonic.
const LOW: i32 = -5;
const HIGH: i32 = 15;

/// The melody: a few long notes over the first five of its eight bars, rising
/// from a note of the chord and sighing back down onto the next; then silence.
fn motif(seed: u64, prog: &[Chord; 4], beats_per_bar: f64) -> Vec<MNote> {
    let mut rng = Rng::stream(seed, "cinder-melody");
    // (at, len) in bars: long notes, and nothing in the last three bars
    const RHYTHMS: [&[(f64, f64)]; 5] = [
        &[(0.0, 1.5), (1.5, 0.5), (2.0, 2.0)],
        &[(0.0, 1.0), (1.0, 1.0), (2.0, 2.5)],
        &[(0.5, 1.0), (1.5, 0.5), (2.0, 0.5), (2.5, 2.0)],
        &[(0.0, 2.0), (2.0, 1.0), (3.0, 1.5)],
        &[(0.0, 2.5), (3.0, 0.5), (3.5, 1.5)],
    ];
    let rhythm = *rng.pick(&RHYTHMS);
    let scale = ladder(&SCALE, LOW, HIGH);
    let idx = |p: i32| scale.iter().position(|&x| x >= p).unwrap_or(scale.len() - 1);
    let start = snap(*rng.pick(&[7, 3, 10]), &prog[0].pcs()).clamp(LOW, HIGH);
    let mut i = idx(start);
    let mut out = Vec::new();
    for (j, &(at, len)) in rhythm.iter().enumerate() {
        if j > 0 {
            let step = if j + 1 == rhythm.len() { *rng.pick(&[-1, -2, -1]) } else { *rng.pick(&[1, 1, 2, -1]) };
            i = (i as i32 + step).clamp(0, scale.len() as i32 - 1) as usize;
        }
        let mut rel = scale[i];
        // the last note settles on the second chord
        if j + 1 == rhythm.len() {
            rel = snap(rel, &prog[1].pcs()).clamp(LOW, HIGH);
        }
        out.push(MNote { at: at * beats_per_bar, len: len * beats_per_bar, rel });
    }
    out
}

/// Write Cinder into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a vast, dark cathedral, a slow wobbling echo, worn tape over everything
    p.fx.space = Space::Cathedral;
    p.fx.size = 0.8;
    p.fx.tone = 0.3;
    p.fx.motion = 0.55;
    p.fx.predelay = 0.15;
    p.fx.low_cut = 0.55;
    p.fx.width = 1.0;
    p.fx.duck = 0.25;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.0;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.45;
    p.fx.echo.time = EchoTime::DottedQuarter;
    p.fx.echo.feedback = 0.5;
    p.fx.echo.tone = 0.25;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.6;
    p.fx.echo.wow = 0.3;
    p.master_db = 6.0;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let s16 = PPQ / 4;
    let steps = (bar / s16).max(1);
    let beats_per_bar = bar as f64 / PPQ as f64;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "cinder-harmony");
    let prog = *hr.pick(&PROGS);
    let melody = motif(r.seed, &prog, beats_per_bar);
    let mut dr = Rng::stream(r.seed, "cinder-beat");
    let mut ar = Rng::stream(r.seed, "cinder-air");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "cinder-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [hiss, radio, drone, pad, sub, drums, stabs, voices, lead] = tr;
    let lead_tonic = pitch(0, 62);

    let (env, force) = envelopes::<9>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.6 + 0.4 * inten(t));

    // the tape's hiss and the radio under everything
    w.note(hiss, 0, end, fx::TAPE_HISS as i32, 40.0);
    w.note(radio, 0, end, fx::RADIO as i32, 90.0);
    // the snare on the bar's middle beat (the third in 4/4): half time
    let beats = (bar / beat).max(1);
    let snare_step = (beats / 2) * (beat / s16);
    let mut rep = 0u32;
    let mut kicks: &[u32] = KICKS[0];
    let mut voice_bends: Vec<(u32, u32)> = Vec::new();
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at =
            |k: u32| -> Chord { if part == Part::Intro || (last && k + 4 >= nb && nb >= 6) { I } else { prog[(k / 4) as usize % 4] } };
        // the drone: the tonic and fifth held through the section, overlapping the next
        let dl = (zz - a) + bar;
        w.note(drone, a, dl, pitch(0, 38), 72.0);
        w.note(drone, a, dl, pitch(7, 45), 54.0);
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let new_chord = k == 0 || chord_at(k) != chord_at(k - 1) || k % 4 == 0;
            // the pad: the chord held wide and low, its ninth above as it builds
            if new_chord && pres(Layer::Pad, t_bar + bar) > 0.03 {
                let span = (4 - k % 4).min(nb - k) * bar;
                let len = span.min(zz.saturating_sub(t_bar) + bar / 2);
                let pcs = chord.pcs();
                w.note(pad, t_bar, len, pitch(pcs[0], 50), vel(70.0, t_bar));
                w.note(pad, t_bar, len, pitch(pcs[2], 55), vel(62.0, t_bar));
                w.note(pad, t_bar, len, pitch(pcs[1], 63), vel(60.0, t_bar));
                if inten(t_bar) > 0.55 {
                    w.note(pad, t_bar, len, pitch(chord.root + 2, 67), vel(48.0, t_bar));
                }
            }
            // the beat: a dull kick broken differently every four bars, the muffled
            // snare in the middle of the bar, ticks only as it fills
            if k % 4 == 0 {
                kicks = *dr.pick(&KICKS);
            }
            let dp = pres(Layer::Drums, t_bar);
            if dp > 0.04 {
                for &st in kicks.iter().filter(|&&st| st < steps) {
                    let t = t_bar + st * s16;
                    w.note(drums, t, s16 * 2, drum::DEEP_KICK as i32, if st == 0 { 100.0 } else { 82.0 });
                    // the sub under each kick, the downbeat's held longest
                    if pres(Layer::Sub, t) > 0.03 {
                        let len = if st == 0 { 6 * s16 } else { 3 * s16 };
                        w.note(sub, t, len, pitch(chord.root, 33), if st == 0 { 100.0 } else { 84.0 });
                    }
                }
                if snare_step < steps {
                    w.note(drums, t_bar + snare_step * s16, s16 * 2, drum::SNARE as i32, vel(92.0, t_bar));
                }
                if dr.chance(0.25) && steps >= 4 {
                    w.note(drums, t_bar + (steps - 2) * s16, s16, drum::SNARE as i32, 30.0);
                }
                let ticks: &[u32] = if inten(t_bar) > 0.6 {
                    &[2, 6, 10, 14]
                } else if inten(t_bar) > 0.45 {
                    &[6, 14]
                } else {
                    &[]
                };
                for &st in ticks.iter().filter(|&&st| st < steps) {
                    if dr.chance(0.75) {
                        w.note(drums, t_bar + st * s16, s16, drum::TICK as i32, dr.range_f(28.0, 46.0));
                    }
                }
            } else if pres(Layer::Sub, t_bar) > 0.03 {
                // (the sub alone before the beat: a long low root on the downbeat)
                w.note(sub, t_bar, 6 * s16, pitch(chord.root, 33), 90.0);
            }
            // the stabs: hollow root, fifth and octave, once on a new chord and, at the
            // fullest, again two bars on
            let sp = pres(Layer::Stabs, t_bar);
            if sp > 0.1 && (new_chord || (k % 4 == 2 && inten(t_bar) > 0.6)) {
                let st = *ar.pick(&[0u32, 3, 6, 10]);
                let t = t_bar + st.min(steps - 1) * s16;
                let base = pitch(chord.root, 55);
                for (d, v) in [(0, 78.0), (7, 70.0), (12, 62.0)] {
                    w.note(stabs, t, 2 * beat, base + d, vel(v, t));
                }
            }
            // a voice far off now and then, sinking as it fades
            if k % 4 == 0 && pres(Layer::Voices, t_bar + bar) > 0.1 && ar.chance(0.35 + 0.4 * pres(Layer::Voices, t_bar + bar)) {
                let pcs = chord.pcs();
                let s = t_bar + ar.range(0, beats as i32 - 1) as u32 * beat;
                let len = bar * ar.range(2, 3) as u32;
                let key = pitch(*ar.pick(&pcs), 66);
                w.note(voices, s, len, key, ar.range_f(44.0, 58.0));
                if ar.chance(0.4) {
                    let other = pitch(*ar.pick(&pcs), key + 4);
                    if other != key {
                        w.note(voices, s + beat, len - beat, other, ar.range_f(38.0, 50.0));
                    }
                }
                voice_bends.push((s, len));
            }
        }
        // the melody: its eight bars from the section's start, then as long silent
        if matches!(part, Part::Swell | Part::NightShift | Part::Fade) {
            let phrase = 8 * bar;
            let mut ph = a;
            while ph < zz {
                let play = rep.is_multiple_of(2);
                if play {
                    for (j, n) in melody.iter().enumerate() {
                        let s = ph + (n.at * PPQ as f64).round() as u32;
                        if s >= zz || pres(Layer::Lead, s) < 0.3 {
                            continue;
                        }
                        // (every other time round, the last note sinks a step further)
                        let rel = if j + 1 == melody.len() && rep % 4 == 2 {
                            ladder(&SCALE, LOW - 2, n.rel - 1).last().copied().unwrap_or(n.rel)
                        } else {
                            n.rel
                        };
                        let len = (n.len * PPQ as f64).round() as u32;
                        w.note(lead, s, len, lead_tonic + rel, vel(if part == Part::Fade { 66.0 } else { 78.0 }, s));
                    }
                }
                rep += 1;
                ph += phrase;
            }
        }
    }
    // the voices' pitch sinking a little through each, like a tape slowing
    for (s, len) in voice_bends {
        let n = 8;
        for k in 0..=n {
            let t = s + len * k / n;
            if t < end {
                w.ctrls[voices].push(Ctrl { tick: t, kind: CtrlKind::Bend, value: -(1400 * k as i16 / n as i16) });
            }
        }
    }
    // expression a quaver at a time, and the filters: the pad and drone breathing slowly
    // and opening a little as it builds
    let sweep = |t: u32, bars: f64, phase: f64| ((t as f64 / (bars * bar as f64) + phase) * std::f64::consts::TAU).sin();
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Hiss | Layer::Radio => 127.0 * pres(*layer, t),
                _ => 127.0 * pres(*layer, t) * (0.75 + 0.25 * inten(t)),
            };
            // (never faster than a step of eight a quaver)
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Pad => Some(38.0 + 34.0 * inten(t) + 14.0 * sweep(t, 16.0, 0.0)),
                Layer::Drone => Some(40.0 + 20.0 * inten(t) + 20.0 * sweep(t, 24.0, 0.3)),
                Layer::Lead => Some(50.0 + 20.0 * inten(t)),
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
        compose(&Recipe { style: Style::Cinder, ..Recipe::random(seed) }, Style::Cinder.tempo(), (4, 4), bars)
    }

    fn track<'a>(p: &'a Project, name: &str) -> &'a Track {
        p.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name}"))
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: static and the drone alone at first; the beat in by the Swell, its
    /// snare always on the third beat; stabs and the melody; the beat, stabs and melody
    /// gone by the end, the static still there. In natural minor, on worn tape.
    #[test]
    fn cinder_rises_a_little_and_falls_back() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            assert_eq!(p.recipe.map(|r| r.mode), Some(crate::compose::Mode::Aeolian));
            assert!(p.fx.tape > 0.2);
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let b = p.block_ticks();
            assert!(starts(&p, "Drone", b[0]) >= 1, "seed {seed}");
            for name in ["Drums", "Sub", "Stabs", "Lead", "Pad"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            for i in [2, 3] {
                assert!(starts(&p, "Drums", b[i]) > 8, "seed {seed}");
                assert!(starts(&p, "Stabs", b[i]) >= 1, "seed {seed}");
            }
            assert!(starts(&p, "Lead", (b[2].0, b[4].0)) >= 3, "seed {seed}");
            let bar = p.bar_ticks();
            let drums = track(&p, "Drums");
            for n in drums.notes.iter().filter(|n| n.key == drum::SNARE && n.vel > 50) {
                assert_eq!(n.start % bar, 2 * p.beat_ticks(), "seed {seed}");
            }
            assert!(drums.notes.iter().filter(|n| n.key == drum::DEEP_KICK).all(|n| n.start % (PPQ / 4) == 0));
            let tail = b[4].0 + (b[4].1 - b[4].0) / 2;
            for name in ["Drums", "Stabs", "Lead"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            assert!(track(&p, "Radio").notes.iter().any(|n| n.key == fx::RADIO && n.start == 0 && n.end() == p.end_tick()));
            // lots of empty space: the melody plays well under half the time
            let lead: u32 = track(&p, "Lead").notes.iter().map(|n| n.len).sum();
            assert!((lead as f64) < 0.5 * (b[4].1 - b[2].0) as f64, "seed {seed}");
        }
    }

    /// The melody keeps to the scale and its range, and rests before it ends.
    #[test]
    fn the_melody_is_few_long_notes() {
        for seed in 0..40 {
            for prog in PROGS {
                let m = motif(seed, &prog, 4.0);
                assert!((3..=4).contains(&m.len()));
                assert!(m.iter().all(|n| (LOW..=HIGH).contains(&n.rel) && SCALE.contains(&n.rel.rem_euclid(12))));
                assert!(m.iter().map(|n| n.len).sum::<f64>() >= 14.0);
                assert!(m.last().map_or(0.0, |n| n.at + n.len) <= 20.0);
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
            let r = Recipe { style: Style::Cinder, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 80.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Cinder, ..Recipe::random(i as u64) }, 80.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Cinder, ..Recipe::random(7) }, 80.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
