//! Filmstrip (D29): nostalgic downtempo electronica, warm and hazy, like an old
//! educational film for children heard through a worn video tape: bittersweet, innocent,
//! a little eerie, at 90 bpm in 4/4, in the Dorian mode (a minor key with a bright,
//! major sixth).
//!
//! It opens on the room: birdsong, a playground across the field, a record's crackle and
//! tape hiss, and a music box playing the tune alone over tape-keyboard strings. A lazy,
//! swung broken beat drops in, soft and dusty, with a round bass following the kick and
//! a soft electric piano comping a beat behind. A detuned analogue lead, its pitch
//! wobbling and drifting, takes the tune; glockenspiel and music-box accents chime
//! around it, and children's voices sing a playground chant, filtered and warped in
//! pitch. Then the beat leaves, the music box has the tune again, the playground comes
//! back, and the tape winds down. The whole mix plays through worn tape: wow and
//! flutter, saturation and a dulled top.

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

/// The sounds added to the palette for Filmstrip: the other styles leave them out of
/// their choosing, so every recipe they had still sounds as it did.
pub(crate) const FILMSTRIP_SOUNDS: [&str; 3] = ["Wobble Lead", "Tape Strings", "Voice Oohs"];

/// The usual arrangement: the room and the music box, the beat dropping in, the tune
/// on the lead, everything with the children's chant, and the music box alone again.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens: [u32; 5] = if bars >= 24 { [8, 8, 16, 16, 8] } else { [2, 4, 4, 4, 2] };
    let lens = crate::project::share_bars(&lens, bars.max(2));
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// A chord: its root in semitones above the tonic, and its intervals (sevenths and
/// ninths: the electric piano's colours).
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

/// The Dorian chords: the minor tonic with its ninth, the major fourth (the mode's
/// bright sixth in it), the relative major, the flat seventh, the minor fifth and the
/// half-diminished sixth.
const I: Chord = Chord { root: 0, ivs: &[0, 3, 7, 10, 14] };
const IV: Chord = Chord { root: 5, ivs: &[0, 4, 7, 10, 14] };
const III: Chord = Chord { root: 3, ivs: &[0, 4, 7, 11] };
const VII: Chord = Chord { root: 10, ivs: &[0, 4, 7, 11] };
const V: Chord = Chord { root: 7, ivs: &[0, 3, 7, 10] };
const II: Chord = Chord { root: 2, ivs: &[0, 3, 7, 10] };
const VI: Chord = Chord { root: 9, ivs: &[0, 3, 6, 10] };

/// Two bars a chord, eight bars round: the tonic and the bright fourth rocking, or
/// wandering off and back, once to a half-diminished chord for the eeriness.
const PROGS: [[Chord; 4]; 5] = [[I, IV, I, IV], [I, IV, VII, III], [III, II, I, IV], [I, V, VII, IV], [I, VII, IV, VI]];

/// The Dorian scale, the tune's.
const SCALE: [i32; 7] = [0, 2, 3, 5, 7, 9, 10];

/// Where the kick falls, in semiquavers of the bar.
const KICKS: [&[u32]; 5] = [&[0, 7, 10], &[0, 10], &[0, 3, 10], &[0, 6, 10, 11], &[0, 8, 10]];

/// The layers.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Layer {
    Field,
    Vinyl,
    Drums,
    Bass,
    Keys,
    Mellotron,
    Lead,
    MusicBox,
    Glock,
    Voices,
}

const LAYERS: [Layer; 10] = [
    Layer::Field,
    Layer::Vinyl,
    Layer::Drums,
    Layer::Bass,
    Layer::Keys,
    Layer::Mellotron,
    Layer::Lead,
    Layer::MusicBox,
    Layer::Glock,
    Layer::Voices,
];

/// How present a layer is `u` of the way through a section of this part.
fn presence(layer: Layer, part: Part, u: f64) -> f64 {
    use Layer::*;
    let end = if part == Part::Fade { 1.0 - ramp(u, 0.8, 1.0) } else { 1.0 };
    let v = match (part, layer) {
        (_, Vinyl) => 1.0,
        (Part::Intro, Field) => 1.0,
        (Part::Intro, MusicBox) => 1.0,
        (Part::Intro, Mellotron) => 0.7 * ramp(u, 0.2, 0.8),
        (Part::Intro, _) => 0.0,
        (Part::Neon, Field) => 0.6 - 0.3 * u,
        (Part::Neon, Drums) => ramp(u, -0.1, 0.12),
        (Part::Neon, Bass) => ramp(u, -0.05, 0.2),
        (Part::Neon, Keys) => 1.0,
        (Part::Neon, Mellotron) => 0.8,
        (Part::Neon, MusicBox) => 0.4,
        (Part::Neon, Voices) => 0.4,
        (Part::Neon, _) => 0.0,
        (Part::Swell, Field) => 0.3,
        (Part::Swell, Glock) => 0.6,
        (Part::Swell, MusicBox) => 0.4,
        (Part::Swell, Voices) => 0.6,
        (Part::Swell, _) => 1.0,
        (Part::NightShift, Field) => 0.25,
        (Part::NightShift, _) => 1.0,
        (Part::Fade, Field) => 0.3 + 0.7 * ramp(u, 0.15, 0.5),
        (Part::Fade, Drums) => 1.0 - ramp(u, 0.0, 0.3),
        (Part::Fade, Bass) => 1.0 - ramp(u, 0.05, 0.35),
        (Part::Fade, Keys) => 1.0 - ramp(u, 0.3, 0.7),
        (Part::Fade, Mellotron) => 1.0 - ramp(u, 0.4, 0.85),
        (Part::Fade, Lead) => 1.0 - ramp(u, 0.0, 0.3),
        (Part::Fade, MusicBox) => 1.0,
        (Part::Fade, Glock) => 0.5 * (1.0 - ramp(u, 0.0, 0.4)),
        (Part::Fade, Voices) => 0.5 * (1.0 - ramp(u, 0.2, 0.6)),
    };
    v * end
}

/// How strongly everything plays: gentle throughout, a little more at the fullest.
fn intensity(part: Part, u: f64) -> f64 {
    match part {
        Part::Intro => 0.3,
        Part::Neon => 0.45 + 0.1 * u,
        Part::Swell => 0.55 + 0.15 * u,
        Part::NightShift => 0.7 + 0.2 * (u / 0.6).min(1.0),
        Part::Fade => 0.75 - 0.45 * u,
    }
}

/// Filmstrip's instruments: the field recording and the record's surface, the beat
/// dusty (its top cut) and dry, the bass round, the electric piano and tape strings
/// warm, the lead and chimes into the echo, the children's voices thinned and warped.
fn tracks(p: &mut Project) -> [usize; 10] {
    let s = Strip::default();
    let mut add = |name: &str, role: Role, source: Source, strip: Strip| {
        let i = p.tracks.len();
        p.tracks.push(Track { strip, role: Some(role), ..Track::new(name, source, i) });
        i
    };
    let synth = |name: &str| Source::Synth(by_name(name));
    let gm = |program: u8, name: &str| Source::SoundFont { font: GM_FONT.into(), bank: 0, program, name: name.into() };
    let room_kit = Source::SoundFont { font: GM_FONT.into(), bank: 128, program: 8, name: "Room".into() };
    [
        add(
            "Field Recording",
            Role::City,
            Source::Kit(KitKind::CityFx),
            Strip { reverb: 0.15, width: 1.6, gain_db: -9.0, high_cut: 7000.0, ..s },
        ),
        add("Vinyl & Tape", Role::City, Source::Kit(KitKind::CityFx), Strip { reverb: 0.0, gain_db: -4.0, high_cut: 10000.0, ..s }),
        add("Drums", Role::Drums, room_kit, Strip { reverb: 0.12, gain_db: 0.0, high_cut: 4500.0, ..s }),
        add("Bass", Role::Bass, gm(33, "Finger Bass"), Strip { reverb: 0.05, gain_db: -3.0, high_cut: 900.0, ..s }),
        add("Electric Piano", Role::Keys, gm(4, "Electric Piano"), Strip { reverb: 0.3, echo: 0.15, pan: -0.2, gain_db: 0.0, ..s }),
        add("Tape Strings", Role::Strings, synth("Tape Strings"), Strip { reverb: 0.5, width: 1.6, gain_db: -6.0, ..s }),
        add("Lead", Role::Lead, synth("Wobble Lead"), Strip { reverb: 0.45, echo: 0.35, pan: 0.1, gain_db: -6.0, ..s }),
        add("Music Box", Role::Bells, synth("Music Box"), Strip { reverb: 0.5, echo: 0.3, pan: 0.3, gain_db: 0.0, ..s }),
        add("Glockenspiel", Role::Bells, gm(9, "Glockenspiel"), Strip { reverb: 0.5, echo: 0.4, pan: -0.35, gain_db: 2.0, ..s }),
        add(
            "Children's Voices",
            Role::Pad,
            gm(53, "Voice Oohs"),
            Strip { reverb: 0.6, echo: 0.45, pan: 0.3, width: 1.2, gain_db: -1.0, low_cut: 600.0, high_cut: 3200.0, ..s },
        ),
    ]
}

/// One note of the tune: where it starts and how long it lasts, in crotchets from the
/// start of its eight bars, and its pitch above the tune's tonic.
#[derive(Clone, Copy, Debug, PartialEq)]
struct MNote {
    at: f64,
    len: f64,
    rel: i32,
}

const LOW: i32 = -3;
const HIGH: i32 = 17;

/// The tune: four two-bar segments, one to a chord, A B A C, simple and singable: the
/// A's the same rhythm and the same steps from a note of their chord, the C coming to
/// rest. In crotchets, for bars of `bpb` crotchets.
fn tune(seed: u64, prog: &[Chord; 4], bpb: f64) -> Vec<MNote> {
    let mut rng = Rng::stream(seed, "filmstrip-tune");
    // the rhythms, in quavers of a two-bar segment (of sixteen)
    const A: [&[u32]; 5] = [&[4, 4, 8], &[2, 2, 4, 8], &[6, 2, 8], &[4, 2, 2, 4, 4], &[3, 1, 4, 8]];
    const B: [&[u32]; 4] = [&[2, 2, 2, 2, 8], &[4, 4, 4, 4], &[2, 2, 4, 2, 2, 4], &[4, 2, 2, 8]];
    const C: [&[u32]; 3] = [&[8, 8], &[4, 4, 8], &[16]];
    let (a, b, c) = (*rng.pick(&A), *rng.pick(&B), *rng.pick(&C));
    let rhythms = [a, b, a, c];
    let scale = ladder(&SCALE, LOW, HIGH);
    let idx = |p: i32| scale.iter().position(|&x| x >= p).unwrap_or(scale.len() - 1);
    let mut moves = |n: usize| -> Vec<i32> { (0..n).map(|_| *rng.pick(&[-2, -1, -1, 1, 1, 1, 2, 0])).collect() };
    let (am, bm, cm) = (moves(a.len()), moves(b.len()), moves(c.len()));
    let seg = 2.0 * bpb;
    let mut out = Vec::new();
    let mut last = 9;
    let mut a_start = 9;
    for (si, rh) in rhythms.iter().enumerate() {
        let chord = prog[si];
        let start = match si {
            // (the Dorian sixth, or the fifth, to begin: bittersweet)
            0 => snap(*rng.pick(&[9, 7, 10]), &chord.pcs()),
            1 => snap(last + 2, &chord.pcs()),
            2 => Some(snap(a_start, &chord.pcs())).filter(|x| (x - last).abs() <= 7).unwrap_or_else(|| snap(last, &chord.pcs())),
            _ => snap(last - 1, &chord.pcs()),
        }
        .clamp(LOW, HIGH);
        if si == 0 {
            a_start = start;
        }
        let mv = match si {
            0 | 2 => &am,
            1 => &bm,
            _ => &cm,
        };
        let mut i = idx(start);
        let mut at = 0.0;
        for (j, &d) in rh.iter().enumerate() {
            if j > 0 {
                i = (i as i32 + mv[j - 1]).clamp(0, scale.len() as i32 - 1) as usize;
            }
            let len = d as f64 * seg / 16.0;
            let mut rel = scale[i];
            if j == 0 || len >= seg / 4.0 || (si == 3 && j + 1 == rh.len()) {
                rel = snap(rel, &chord.pcs()).clamp(LOW, HIGH);
                i = idx(rel);
            }
            out.push(MNote { at: si as f64 * seg + at, len, rel });
            at += len;
            last = rel;
        }
    }
    out
}

/// A semiquaver step's place in the bar, swung: the second of each pair late, as a lazy
/// drummer plays it.
fn swung(step: u32, s16: u32) -> u32 {
    step * s16 + if step % 2 == 1 { s16 * 6 / 25 } else { 0 }
}

/// Write Filmstrip into `p`: its instruments, its mix and its notes.
pub(crate) fn write(p: &mut Project, r: &Recipe, secs: &[(Part, u32, u32)], _density: f64) {
    p.tracks.clear();
    let tr = tracks(p);
    // a small warm plate, a soft echo, and well-worn tape over everything
    p.fx.space = Space::Plate;
    p.fx.size = 0.55;
    p.fx.tone = 0.35;
    p.fx.motion = 0.6;
    p.fx.predelay = 0.05;
    p.fx.low_cut = 0.5;
    p.fx.width = 0.8;
    p.fx.duck = 0.15;
    p.fx.shimmer = 0.0;
    p.fx.freeze = false;
    p.fx.ensemble = 0.0;
    p.fx.drift = 0.0;
    p.fx.detune = 0.0;
    p.fx.tape = 0.7;
    p.fx.echo.time = EchoTime::DottedEighth;
    p.fx.echo.feedback = 0.4;
    p.fx.echo.tone = 0.3;
    p.fx.echo.ping_pong = true;
    p.fx.echo.to_reverb = 0.4;
    p.fx.echo.wow = 0.4;
    p.master_db = 6.5;

    let bar = p.bar_ticks();
    let beat = p.beat_ticks();
    let q = PPQ / 2;
    let s16 = PPQ / 4;
    let steps = (bar / s16).max(1);
    let beats = (bar / beat).max(1);
    let bpb = bar as f64 / PPQ as f64;
    let end = secs.last().map_or(bar, |s| s.2);
    let mut hr = Rng::stream(r.seed, "filmstrip-harmony");
    let prog = *hr.pick(&PROGS);
    let theme = tune(r.seed, &prog, bpb);
    let mut dr = Rng::stream(r.seed, "filmstrip-beat");
    let mut ar = Rng::stream(r.seed, "filmstrip-air");
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        touch: Rng::stream(r.seed, "filmstrip-touch"),
    };
    let root = r.root as i32;
    let pitch = |rel_pc: i32, target: i32| near(((root + rel_pc).rem_euclid(12)) as u8, target) as i32;
    let [field, vinyl, drums, bass, keys, mello, lead, mbox, glock, voices] = tr;
    let lead_tonic = pitch(0, 64);

    let (env, force) = envelopes::<10>(secs, end, q, |l, part, u| presence(LAYERS[l], part, u), intensity);
    let nq = env.len();
    let at = |t: u32| ((t / q) as usize).min(nq.saturating_sub(1));
    let pres = |layer: Layer, t: u32| env[at(t)][LAYERS.iter().position(|l| *l == layer).unwrap_or(0)];
    let inten = |t: u32| force[at(t)];
    let vel = |base: f64, t: u32| base * (0.65 + 0.35 * inten(t));
    // (a little loose: always a few ticks behind, never ahead)
    let mut loose = Rng::stream(r.seed, "filmstrip-loose");
    let mut jit = |t: u32| t + loose.range(0, 14) as u32;

    // the record's surface and the tape's hiss, and the birds, all the way through
    w.note(vinyl, 0, end, fx::CRACKLE as i32, 90.0);
    w.note(vinyl, 0, end, fx::TAPE_HISS as i32, 45.0);
    w.note(field, 0, end, fx::BIRDS as i32, 80.0);
    let mut kicks: &[u32] = KICKS[0];
    let mut rep = 0u32;
    let mut warps: Vec<(u32, u32, bool)> = Vec::new();
    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let nb = (z - a).div_ceil(bar);
        let last = si + 1 == secs.len();
        let zz = if last { end } else { z };
        let chord_at = |k: u32| -> Chord { if last && k + 2 >= nb && nb >= 4 { I } else { prog[(k / 2) as usize % 4] } };
        // the playground across the field, at the start and the end
        if matches!(part, Part::Intro | Part::Fade) {
            w.note(field, a, (zz - a) + bar, fx::PLAYGROUND as i32, 76.0);
        }
        for k in 0..nb {
            let t_bar = a + k * bar;
            if t_bar >= zz {
                break;
            }
            let chord = chord_at(k);
            let next = chord_at(k + 1);
            let pcs = chord.pcs();
            let first_bar = k % 2 == 0;
            // tape strings: the chord held from its first bar (the tape runs out by itself)
            if first_bar && pres(Layer::Mellotron, t_bar + bar) > 0.03 {
                let len = (2 * bar).min(zz.saturating_sub(t_bar) + bar / 2);
                for (i, &pc) in pcs.iter().take(4).enumerate() {
                    w.note(mello, t_bar, len, pitch(pc, 60 + 3 * i as i32), vel(66.0, t_bar));
                }
            }
            // the electric piano: the chord on the beat, rolled a little, and pushed on
            // the and of two; shorter in the second bar
            if pres(Layer::Keys, t_bar) > 0.03 {
                let hits: &[(u32, u32, f64)] = if first_bar { &[(0, 6, 64.0), (6, 3, 52.0)] } else { &[(0, 3, 58.0), (10, 4, 54.0)] };
                for &(st, len, v) in hits.iter().filter(|h| h.0 < steps) {
                    let t = jit(t_bar + swung(st, s16));
                    for (i, &pc) in pcs.iter().skip(1).enumerate() {
                        w.note(keys, t + 12 * i as u32, len * s16, pitch(pc, 58 + 3 * i as i32), vel(v, t_bar));
                    }
                    w.note(keys, t, len * s16, pitch(chord.root, 50), vel(v - 6.0, t_bar));
                }
            }
            // the beat: a lazy kick, the snare laid back on two and four, hats swung
            if k % 4 == 0 {
                kicks = *dr.pick(&KICKS);
            }
            let dp = pres(Layer::Drums, t_bar);
            if dp > 0.04 {
                for &st in kicks.iter().filter(|&&st| st < steps) {
                    w.note(drums, jit(t_bar + swung(st, s16)), s16, 36, if st == 0 { 92.0 } else { 78.0 });
                }
                for b in (1..beats).step_by(2) {
                    w.note(drums, t_bar + b * beat + 18, s16, 38, vel(76.0, t_bar));
                }
                if dr.chance(0.3) && steps > 7 {
                    w.note(drums, t_bar + swung(7, s16), s16, 38, 26.0);
                }
                for st in (0..steps).step_by(2) {
                    let open = st == 14 && k % 4 == 3;
                    let v = if st % 4 == 0 { 46.0 } else { 34.0 };
                    w.note(drums, jit(t_bar + swung(st, s16)), s16, if open { 46 } else { 42 }, vel(v, t_bar));
                    if st + 1 < steps && dr.chance(0.18) {
                        w.note(drums, jit(t_bar + swung(st + 1, s16)), s16, 42, 24.0);
                    }
                }
            }
            // the bass: the chord's root on the kicks, a step towards the next chord at
            // the end of its two bars
            if pres(Layer::Bass, t_bar) > 0.03 {
                let ks: Vec<u32> = kicks.iter().copied().filter(|&st| st < steps).collect();
                for (j, &st) in ks.iter().enumerate() {
                    let till = ks.get(j + 1).copied().unwrap_or(steps);
                    let len = (till - st).max(2) * s16 - s16 / 2;
                    let key = if !first_bar && j + 1 == ks.len() && st >= 8 && next != chord {
                        let to = pitch(next.root, 38);
                        if to > pitch(chord.root, 38) { to - 2 } else { to + 1 }
                    } else {
                        pitch(chord.root, 38)
                    };
                    w.note(bass, jit(t_bar + swung(st, s16)), len, key, if st == 0 { 88.0 } else { 74.0 });
                }
            }
            // the glockenspiel: a chime on a new chord, now and then
            if first_bar && pres(Layer::Glock, t_bar) > 0.1 && ar.chance(0.5) {
                let t = t_bar + *ar.pick(&[0u32, 2, 3]) * beat.min(bar / 4);
                w.note(glock, t, beat, pitch(*ar.pick(&pcs), 84), vel(56.0, t));
                if ar.chance(0.5) {
                    w.note(glock, t + q, beat, pitch(*ar.pick(&pcs), 88), vel(46.0, t));
                }
            }
            // the music box: a little falling run of the chord at the end of each eight
            // bars, where the tune isn't its own
            if k % 8 == 7 && !matches!(part, Part::Intro | Part::Fade) && pres(Layer::MusicBox, t_bar) > 0.1 {
                let tones = ladder(&pcs.iter().map(|x| (x + root).rem_euclid(12)).collect::<Vec<_>>(), 79, 96);
                for (j, &key) in tones.iter().rev().take(4).enumerate() {
                    w.note(mbox, t_bar + bar / 2 + j as u32 * q, beat, key, 64.0 - 4.0 * j as f64);
                }
            }
            // the children: a long "ooh" now and then; at the fullest, their chant (the
            // tonic's seventh, fifth and octave: sol, mi, la, sol, mi), warped
            let vp = pres(Layer::Voices, t_bar);
            if k % 8 == 4 && vp > 0.3 && (part == Part::NightShift || ar.chance(0.5)) {
                let sol = pitch(10, 79);
                let chant = [(0.0, 1.0, sol), (1.0, 1.0, sol - 3), (2.0, 0.5, sol + 2), (2.5, 0.5, sol), (3.0, 1.0, sol - 3)];
                for (b0, len, key) in chant {
                    let s = t_bar + (b0 * PPQ as f64) as u32;
                    w.note(voices, s, (len * PPQ as f64) as u32, key, vel(68.0, s));
                }
                warps.push((t_bar, bar, ar.chance(0.4)));
            } else if k % 4 == 1 && vp > 0.1 && ar.chance(0.3) {
                let s = t_bar + beat;
                w.note(voices, s, 2 * beat, pitch(*ar.pick(&pcs), 74), vel(54.0, s));
                warps.push((s, 2 * beat, false));
            }
        }
        // the tune: on the music box at the start and the end, on the lead between
        // (resting every third time round), the glockenspiel doubling its long notes at
        // the fullest
        let phrase = 8 * bar;
        let mut ph = a;
        while ph < zz {
            let on_box = matches!(part, Part::Intro | Part::Fade);
            let on_lead = matches!(part, Part::Swell | Part::NightShift) && (rep % 3 != 2 || part == Part::NightShift);
            for n in &theme {
                let s = ph + (n.at * PPQ as f64).round() as u32;
                if s >= zz {
                    continue;
                }
                let len = (n.len * PPQ as f64).round() as u32;
                if on_box && pres(Layer::MusicBox, s) > 0.2 {
                    w.note(mbox, s, len.max(beat), lead_tonic + 12 + n.rel, vel(70.0, s));
                }
                if on_lead && pres(Layer::Lead, s) > 0.3 {
                    w.note(lead, s, len + PPQ / 16, lead_tonic + n.rel, vel(80.0, s) + if n.len >= 2.0 { 4.0 } else { 0.0 });
                    if n.len >= 1.0 && pres(Layer::Glock, s) > 0.8 {
                        w.note(glock, s, beat, lead_tonic + 24 + n.rel, vel(48.0, s));
                    }
                }
            }
            if matches!(part, Part::Swell | Part::NightShift) {
                rep += 1;
            }
            ph += phrase;
        }
    }
    // the children's voices warped: their pitch swaying as a stretched tape does, now
    // and then sinking as it slows
    for (s, len, sink) in warps {
        let n = 12;
        for k in 0..=n {
            let x = k as f64 / n as f64;
            let sway = 900.0 * (x * std::f64::consts::TAU * 1.5).sin();
            let sag = if sink { -2600.0 * x } else { 0.0 };
            let t = s + (len as f64 * x) as u32;
            if t < end {
                w.ctrls[voices].push(Ctrl { tick: t, kind: CtrlKind::Bend, value: (sway + sag).round() as i16 });
            }
        }
        if s + len < end {
            w.ctrls[voices].push(Ctrl { tick: s + len, kind: CtrlKind::Bend, value: 0 });
        }
    }
    // at the very end, the tape winds down: the music box and strings sag in pitch
    if secs.last().is_some_and(|l| l.0 == Part::Fade) && end > bar {
        let from = end - bar;
        for track in [mbox, mello, keys] {
            for k in 0..=8u32 {
                let x = k as f64 / 8.0;
                w.ctrls[track].push(Ctrl { tick: from + bar * k / 8, kind: CtrlKind::Bend, value: (-8191.0 * x * x).round() as i16 });
            }
        }
    }
    // expression a quaver at a time; the lead's and strings' filters a little open as it
    // fills
    for (ti, layer) in LAYERS.iter().enumerate() {
        let track = tr[ti];
        let (mut l11, mut l74) = (-1.0, -1.0);
        let mut t = 0;
        while t < end {
            let v = match layer {
                Layer::Vinyl | Layer::Field => 127.0 * pres(*layer, t),
                _ => 127.0 * pres(*layer, t) * (0.8 + 0.2 * inten(t)),
            };
            let v = if l11 < 0.0 { v.round() } else { (l11 + (v - l11).clamp(-8.0, 8.0)).round() };
            if v != l11 {
                w.cc(track, t, 11, v);
                l11 = v;
            }
            let b = match layer {
                Layer::Lead => Some(50.0 + 22.0 * inten(t)),
                Layer::Mellotron => Some(54.0 + 16.0 * inten(t)),
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
        compose(&Recipe { style: Style::Filmstrip, ..Recipe::random(seed) }, Style::Filmstrip.tempo(), (4, 4), bars)
    }

    fn track<'a>(p: &'a Project, name: &str) -> &'a Track {
        p.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name}"))
    }

    fn starts(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The story: the room and the music box alone; the beat, bass and piano from the
    /// Neon; the lead with the tune and the children's chant at the fullest; the beat
    /// gone and the music box alone again at the end, the tape winding down. Dorian, on
    /// worn tape, crackle and birds throughout.
    #[test]
    fn filmstrip_opens_and_closes_on_the_music_box() {
        for seed in 0..12 {
            let p = piece(seed, 56);
            assert_eq!(p.recipe.map(|r| r.mode), Some(crate::compose::Mode::Dorian));
            assert!(p.fx.tape > 0.5);
            let b = p.block_ticks();
            assert_eq!(b.len(), 5);
            assert!(starts(&p, "Music Box", b[0]) >= 4, "seed {seed}");
            for name in ["Drums", "Bass", "Electric Piano", "Lead", "Children's Voices"] {
                assert_eq!(starts(&p, name, b[0]), 0, "seed {seed} {name}");
            }
            for name in ["Drums", "Bass", "Electric Piano"] {
                assert!(starts(&p, name, b[1]) > 8, "seed {seed} {name}");
            }
            assert!(starts(&p, "Lead", b[2]) >= 6, "seed {seed}");
            assert!(starts(&p, "Children's Voices", b[3]) >= 5, "seed {seed}");
            let tail = b[4].0 + (b[4].1 - b[4].0) / 2;
            for name in ["Drums", "Lead"] {
                assert_eq!(starts(&p, name, (tail, p.end_tick())), 0, "seed {seed} {name}");
            }
            assert!(starts(&p, "Music Box", b[4]) >= 3, "seed {seed}");
            let vinyl = &crate::ambience::merged(&p);
            assert!(vinyl.notes.iter().any(|n| n.key == fx::CRACKLE && n.start == 0 && n.end() == p.end_tick()));
            let field = &crate::ambience::merged(&p);
            assert!(field.notes.iter().any(|n| n.key == fx::BIRDS) && field.notes.iter().any(|n| n.key == fx::PLAYGROUND));
            // the snare laid back on two and four; the hats' offbeats swung
            let drums = track(&p, "Drums");
            let bar = p.bar_ticks();
            for n in drums.notes.iter().filter(|n| n.key == 38 && n.vel > 50) {
                let x = n.start % bar;
                assert!(x == PPQ + 18 || x == 3 * PPQ + 18, "seed {seed}: {x}");
            }
            // the tape winds down at the end
            let mb = track(&p, "Music Box");
            assert!(mb.ctrls.iter().any(|c| c.kind == CtrlKind::Bend && c.value < -4000));
        }
    }

    /// The tune keeps to the Dorian scale and its range, two bars a chord, eight bars
    /// long, its A segments alike.
    #[test]
    fn the_tune_is_simple() {
        for seed in 0..40 {
            for prog in PROGS {
                let t = tune(seed, &prog, 4.0);
                assert!(t.iter().all(|n| (LOW..=HIGH).contains(&n.rel) && SCALE.contains(&n.rel.rem_euclid(12))), "seed {seed}");
                assert!((t.iter().map(|n| n.len).sum::<f64>() - 32.0).abs() < 1e-9);
                assert!(t.windows(2).all(|w| (w[1].rel - w[0].rel).abs() <= 9), "seed {seed}");
                let seg = |s: f64| t.iter().filter(|n| n.at >= s && n.at < s + 8.0).map(|n| n.at - s).collect::<Vec<_>>();
                assert_eq!(seg(0.0), seg(16.0));
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
            let r = Recipe { style: Style::Filmstrip, ..Recipe::random(seed) };
            plans.push(random_plan(&r, 90.0, (4, 4), 40));
        }
        for (i, plan) in plans.iter().enumerate() {
            let p = compose_plan(&Recipe { style: Style::Filmstrip, ..Recipe::random(i as u64) }, 90.0, (4, 4), plan);
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
            let p = compose(&Recipe { style: Style::Filmstrip, ..Recipe::random(7) }, 90.0, meter, bars);
            assert_eq!(p.bars(), bars.max(2));
            for (a, z) in p.block_ticks() {
                assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| n.start >= a && n.start < z)), "{meter:?} {bars}");
            }
        }
    }
}
