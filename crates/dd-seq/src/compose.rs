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
//! flickering arpeggio, brushes, and a lead that leaves room for a voice; and
//! **Overcity**, a vast, rain-soaked city seen from above at night: hollow fifths that
//! swell like horns through fog, a sparse singing theme that leaps and falls, a
//! heartbeat under the sub, rippling glass, and one great swell before it dissolves;
//! and **Derelict**, a drifting ship in deep space: a cold void of drones, tape hiss
//! and failing machinery, out of which a lone horn, an oboe and tremolo strings
//! surface, a brief orchestral eruption, and a collapse back into the drone.
//! All use the same built-in sounds.
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

    pub(crate) fn steps(self) -> [u8; 7] {
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
    /// Vast and slow, cold and faintly sacred: a rain-soaked city seen from above at
    /// night, its horns rising through fog over a felt sub and a slow heartbeat.
    Overcity,
    /// Cold and vast, almost free of time: a derelict ship adrift, its machinery
    /// failing, sparse dissonant orchestral voices surfacing out of the drone, one brief
    /// eruption and a collapse back into the dark.
    Derelict,
    /// Slow, warm and quietly hopeful: one singable eight-bar theme in a minor key,
    /// introduced on the piano, passed to the cellos, reharmonised for a woodwind or horn,
    /// built to one peak in full strings and brought home to a final cadence.
    Homeward,
    /// Homeward with its instruments chosen (D24): the theme on a soft electric piano,
    /// then violas, a clarinet answered by cellos, full strings at the peak and the solo
    /// piano home to a long-held chord, its lift reaching the relative major.
    Return,
    /// A chamber orchestra with a dark sci-fi noir twist (D25): one theme passed from a
    /// clarinet to a solo violin to the strings in octaves, over a glass drone and a sub
    /// that beats like a heart, chromatic noir harmony, a minor-major ninth to end.
    Chamber,
    /// A hypnotic minimal score in 15/8 (D26): one looping two-bar piano figure, layers
    /// arriving one at a time (bass guitar, glockenspiel, organ, acoustic guitar) to a peak
    /// of tubular bells, then leaving one by one back to the piano.
    Orbit,
    /// A 1970s analogue electronic ambient score (D27): wind and water-drop blips, a
    /// rolling sequencer and round bass, a naive melody of long notes over string-machine
    /// chords, sparkles and whooshes across the stereo field, building and dissolving.
    Nebula,
    /// Overcity watched over gently (D28): its theme and form, the horns soft and
    /// sampled, two drones where the choir and strings were, and a grand piano for the
    /// rippling glass.
    Vigil,
}

/// The sounds added to the palette for Homeward: the other styles leave them out of
/// their choosing, so every recipe they had still sounds as it did.
pub(crate) const HOMEWARD_SOUNDS: [&str; 5] = ["Synth Strings (FreePats)", "Viola", "Clarinet", "French Horn", "Concert Harp (FreePats)"];

impl Style {
    pub const ALL: [Style; 13] = [
        Style::Nocturne,
        Style::Pursuit,
        Style::Blackout,
        Style::Momento,
        Style::Noir,
        Style::Overcity,
        Style::Derelict,
        Style::Homeward,
        Style::Return,
        Style::Chamber,
        Style::Orbit,
        Style::Nebula,
        Style::Vigil,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Style::Nocturne => "Nocturne",
            Style::Pursuit => "Pursuit",
            Style::Blackout => "Blackout",
            Style::Momento => "Momento",
            Style::Noir => "Noir",
            Style::Overcity => "Overcity",
            Style::Derelict => "Derelict",
            Style::Homeward => "Homeward",
            Style::Return => "Return",
            Style::Chamber => "Chamber",
            Style::Orbit => "Orbit",
            Style::Nebula => "Nebula",
            Style::Vigil => "Vigil",
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
            Style::Overcity => {
                "Vast and slow, a rain-soaked city seen from above: hollow fifths swelling like horns through fog, a sparse singing theme, a heartbeat sub, one great swell"
            }
            Style::Derelict => {
                "Cold and almost free of time, a ship adrift in deep space: drones, tape hiss and failing machinery, a lone horn, oboe and tremolo strings out of the dark, one brief eruption, no beat"
            }
            Style::Homeward => {
                "Slow, warm and quietly hopeful: one singable theme, on the piano, then the cellos, a woodwind or horn, full strings at a single peak, and home to a final cadence"
            }
            Style::Return => {
                "Reflective and quietly hopeful, at dawn: one singable theme on a soft electric piano, then violas, a clarinet answered by cellos, full strings at a single peak, and the solo piano home to a long-held chord"
            }
            Style::Nebula => {
                "Dreamy and cosmic, 1970s analogue synthesisers only: wind and water-drop blips, a rolling sequencer and round bass, a simple melody of long notes over string-machine chords, sparkles and whooshes, building and dissolving back into the air"
            }
            Style::Orbit => {
                "Hypnotic and quietly uneasy, in 15/8: one looping piano figure, then a bass guitar, glockenspiel, organ and acoustic guitar one at a time, tubular bells at the peak, and back to the piano alone"
            }
            Style::Vigil => {
                "Overcity, gentler: the vast city from above at night, its theme on soft French horns over two slow drones, a grand piano rippling where the glass was, one great swell"
            }
            Style::Chamber => {
                "A chamber orchestra in a dark, sci-fi noir: a ticking pizzicato, the theme on a clarinet, then a solo violin, a horn and celesta, over a glass drone and a sub like a heartbeat; strings in octaves at the climax, a minor-major ninth to end"
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
            Style::Overcity => 60.0,
            Style::Derelict => 48.0,
            Style::Homeward => 69.0,
            Style::Return => 66.0,
            Style::Chamber => 62.0,
            Style::Orbit => 92.0,
            Style::Nebula => 96.0,
            Style::Vigil => 60.0,
        }
    }

    /// The key it is written for, if any, which choosing it (and each new piece in it)
    /// sets: Return's E minor.
    pub fn key(self) -> Option<(u8, Mode)> {
        match self {
            Style::Return => Some((4, Mode::Aeolian)),
            Style::Orbit => Some((9, Mode::Aeolian)),
            _ => None,
        }
    }

    /// The mode it is always written in, if any: its key's, or Nebula's natural minor.
    pub fn mode(self) -> Option<Mode> {
        match self {
            Style::Nebula => Some(Mode::Aeolian),
            _ => self.key().map(|k| k.1),
        }
    }

    /// Set a recipe to the key and mode this style is written in, where it has them.
    pub fn settle(self, r: &mut Recipe) {
        if let Some((k, _)) = self.key() {
            r.root = k;
        }
        if let Some(m) = self.mode() {
            r.mode = m;
        }
    }

    /// The metre it is written in, if any, which choosing it sets: Orbit's 15/8.
    pub fn meter(self) -> Option<(u32, u32)> {
        match self {
            Style::Orbit => Some((15, 8)),
            _ => None,
        }
    }

    /// The metre a piece takes on changing to this style from one in `current`: its
    /// own, or back to 4/4 from another style's own.
    pub fn meter_from(self, current: (u32, u32)) -> (u32, u32) {
        match self.meter() {
            Some(m) => m,
            None if Style::ALL.iter().any(|s| s.meter() == Some(current)) => (4, 4),
            None => current,
        }
    }

    /// The style whose writing it shares: Overcity's, for Vigil; else itself.
    pub fn base(self) -> Style {
        match self {
            Style::Vigil => Style::Overcity,
            s => s,
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
            Style::Overcity | Style::Vigil => (8.0, 4),
            Style::Derelict => (14.0, 6),
            // (Homeward's harmony follows its theme, a chord a bar)
            Style::Homeward | Style::Return => (3.5, 1),
            // (Chamber's moves two bars a chord)
            Style::Chamber => (8.0, 2),
            // (Orbit's moves two bars a chord under its loop)
            Style::Orbit => (10.0, 2),
            // (Nebula's moves four bars a chord)
            Style::Nebula => (10.0, 4),
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
pub(crate) fn near(pc: u8, target: i32) -> u8 {
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

/// Overcity's: hovering on the tonic, drifting for a moment to the relative major (III),
/// the sixth or the seventh, and back into shadow.
const OVERCITY: [[usize; 4]; 6] = [[0, 0, 2, 0], [0, 5, 0, 0], [0, 2, 5, 0], [0, 0, 5, 2], [0, 6, 0, 0], [0, 2, 0, 6]];

/// Derelict's: barely moving, sliding a step up and back or sinking to the sixth, never
/// going anywhere.
const DERELICT: [[usize; 4]; 6] = [[0, 1, 0, 0], [0, 0, 1, 5], [0, 5, 0, 1], [0, 0, 3, 1], [0, 1, 5, 0], [0, 4, 0, 1]];

/// An open, hollow voicing: the root low, its fifth and octave, and the colour (the
/// third, or a suspension) a tenth above, out of the way.
fn open_voicing(root: u8, fifth: u8, colour: u8, centre: i32) -> Vec<u8> {
    let r0 = near(root, centre - 9);
    let f = near(fifth, r0 as i32 + 7);
    vec![r0, f, r0 + 12, near(colour, r0 as i32 + 15)]
}

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
        Style::Overcity | Style::Vigil => *rng.pick(&OVERCITY),
        Style::Derelict => *rng.pick(&DERELICT),
        Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => *rng.pick(&ROMANTIC),
    };
    // (Noir's sits a little low, under a voice)
    let centre = match r.style {
        Style::Blackout => 50,
        Style::Noir | Style::Overcity => 57,
        Style::Derelict => 52,
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
            // hollow: the triad, a sus2 or a sus4 (a moment of hope, unresolved)
            Style::Overcity | Style::Vigil => match (last, rng.weighted(&[4.0, 1.5, 1.0])) {
                (true, _) | (_, 0) => vec![a, c, b],
                (_, 1) => vec![a, c, pc(deg + 1)],
                _ => vec![a, c, pc(deg + 3)],
            },
            // cold: bare triads, clusters with the second rubbing against the third, and
            // a tritone where the fifth should be; the end a cluster left hanging
            Style::Derelict => match (last, rng.weighted(&[3.0, 2.0, 1.5])) {
                (true, _) => vec![a, c, pc(deg + 1)],
                (_, 0) => vec![a, b, c],
                (_, 1) => vec![a, c, pc(deg + 1), b],
                _ => vec![a, b, (a + 6) % 12],
            },
            // (Homeward harmonises its theme itself; these stand in for anything else)
            Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => vec![a, b, c],
        };
        let prev = out.last().map(|c| c.voicing.clone());
        let voicing =
            if r.style == Style::Overcity { open_voicing(pcs[0], pcs[1], pcs[2], centre) } else { voice(&pcs, prev.as_deref(), centre) };
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
    let style = style.base();
    const PARTS: [Part; 4] = [Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let p = Project { bpm, meter, length_bars: bars.max(2), ..Project::default() };
    let bars = p.length_bars;
    let cb = chord_bars(style, p.bar_secs(), bars);
    let snap = |frac: f64| ((bars as f64 * frac / cb as f64).round() as u32 * cb).min(bars);
    if style == Style::Derelict && bars >= cb * 5 {
        // the void, creeping tension, its tightening, a brief eruption, and a long
        // collapse back into the drone
        let intro = snap(0.14).max(cb);
        let mut bounds = vec![(Part::Intro, 0), (Part::Neon, intro)];
        for (part, frac) in [(Part::Swell, 0.38), (Part::NightShift, 0.62), (Part::Fade, 0.69)] {
            let mut b = snap(frac);
            if part == Part::Fade {
                // the eruption gets at least a chord
                b = b.max(bounds.last().map_or(0, |l| l.1) + cb);
            }
            if b > bounds.last().map_or(0, |l| l.1) && b < bars {
                bounds.push((part, b));
            }
        }
        return bounds.iter().enumerate().map(|(i, (part, b))| (*part, bounds.get(i + 1).map_or(bars, |n| n.1) - b)).collect();
    }
    if style == Style::Homeward {
        return crate::homeward::plan(bars);
    }
    if style == Style::Return {
        return crate::homeward::return_plan(bars);
    }
    if style == Style::Chamber {
        return crate::chamber::plan(bars);
    }
    if style == Style::Orbit {
        return crate::orbit::plan(bars);
    }
    if style == Style::Nebula {
        return crate::nebula::plan(bars);
    }
    if style == Style::Overcity && bars >= cb * 5 {
        // a long breath first, the theme, its development, the great swell, and dissolving
        let intro = if bars >= cb * 8 { cb * 2 } else { cb };
        let mut bounds = vec![(Part::Intro, 0), (Part::Neon, intro)];
        for (part, frac) in [(Part::Swell, 0.36), (Part::NightShift, 0.6), (Part::Fade, 0.8)] {
            let b = snap(frac);
            if b > bounds.last().map_or(0, |l| l.1) && b < bars {
                bounds.push((part, b));
            }
        }
        return bounds.iter().enumerate().map(|(i, (part, b))| (*part, bounds.get(i + 1).map_or(bars, |n| n.1) - b)).collect();
    }
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
    // an intro now and then (in Noir, Overcity and Derelict, always), a chord or two taken from the opening
    // section or else the longest (its own stream, so the rest is as it was)
    let mut ir = Rng::stream(r.seed, "intro-form");
    if units >= 4
        && ir.chance(
            if matches!(
                r.style,
                Style::Noir
                    | Style::Overcity
                    | Style::Derelict
                    | Style::Homeward
                    | Style::Return
                    | Style::Chamber
                    | Style::Orbit
                    | Style::Nebula
                    | Style::Vigil
            ) {
                1.0
            } else {
                0.35
            },
        )
    {
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
    // Vigil is Overcity written for other instruments
    if r.style == Style::Vigil {
        let mut p = compose_plan(&Recipe { style: Style::Overcity, ..*r }, bpm, meter, &plan);
        crate::vigil::adapt(&mut p, r);
        return p;
    }
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
        Style::Overcity | Style::Vigil => (
            ["High", "Vast", "Cold", "Drowned", "Sleepless", "Silent", "Upper", "Endless", "Sodium", "Last"],
            ["Spires", "Towers", "Arcology", "Canopy", "Ascent", "Zenith", "Vigil", "Heights", "Firmament", "Tiers"],
        ),
        Style::Derelict => (
            ["Derelict", "Silent", "Cold", "Drifting", "Dead", "Lower", "Outer", "Failing", "Hollow", "Last"],
            ["Hull", "Corridor", "Airlock", "Reactor", "Bulkhead", "Orbit", "Signal", "Cargo Hold", "Void", "Beacon"],
        ),
        Style::Homeward => (
            ["Long", "Quiet", "Last", "Distant", "Patient", "Winter", "Gentle", "Evening", "Northern", "Open"],
            ["Road", "Harbour", "Light", "Fields", "Lantern", "Return", "Shore", "Hearth", "Promise", "Horizon"],
        ),
        Style::Return => (
            ["Dawn", "First", "Early", "Pale", "Clear", "Rising", "Still", "Golden", "Morning", "Slow"],
            ["Return", "Crossing", "Shoreline", "Light", "Homecoming", "Harbour", "Hills", "Window", "Tide", "Path"],
        ),
        Style::Chamber => (
            ["Glass", "Cold", "Silent", "Synthetic", "Last", "Pale", "Hollow", "Velvet", "Iron", "Midnight"],
            ["Quartet", "Requiem", "Chamber", "Elegy", "Signal", "Archive", "Serenade", "Inquest", "Sonata", "Memory"],
        ),
        Style::Nebula => (
            ["Solar", "Drifting", "Distant", "Silver", "Floating", "Blue", "Quiet", "Endless", "Northern", "Crystal"],
            ["Nebula", "Equinox", "Aurora", "Horizon", "Tides", "Lagoon", "Starfield", "Daybreak", "Atmosphere", "Comet"],
        ),
        Style::Orbit => (
            ["Slow", "Turning", "Still", "Distant", "Glass", "Silent", "Endless", "Pale", "Hidden", "Quiet"],
            ["Orbit", "Mechanism", "Pendulum", "Lighthouse", "Spiral", "Observatory", "Clockwork", "Tide", "Carousel", "Satellite"],
        ),
    };
    p.title = format!("{} {}", names_rng.pick(&adjectives), names_rng.pick(&nouns));
    // Chamber writes its own orchestra
    if style == Style::Chamber {
        crate::chamber::write(&mut p, r, &secs, density);
        return finish(p, r);
    }
    // and Orbit its ensemble
    if style == Style::Orbit {
        crate::orbit::write(&mut p, r, &secs, density);
        return finish(p, r);
    }
    // and Nebula its synthesisers
    // (always in the natural minor its chords are written in)
    if style == Style::Nebula {
        let r = &Recipe { mode: Mode::Aeolian, ..*r };
        crate::nebula::write(&mut p, r, &secs, density);
        return finish(p, r);
    }

    // ---------------------------------------------------------------- sounds
    // each part from its role's alternatives: the usual sound most often
    let mut snd = Rng::stream(r.seed, "sounds");
    let pad_role = if snd.chance(0.25) { Role::Strings } else { Role::Pad };
    // (Derelict's strings have a track of their own: its pad is the tape-worn synth)
    let pad_role = if matches!(style, Style::Derelict | Style::Homeward | Style::Return) { Role::Pad } else { pad_role };
    let mut pick = |role: Role, not: Option<&Source>| -> Alt {
        let alts: Vec<Alt> = alternatives(role)
            .into_iter()
            // a stab can't hold a chord
            .filter(|a| a.bundled && a.label != "Dark Stab" && not.is_none_or(|n| !same_sound(n, &a.source)))
            // (sounds added for Derelict, so the other styles choose as they always have)
            .filter(|a| matches!(style, Style::Derelict | Style::Homeward | Style::Return) || !DERELICT_SOUNDS.contains(&a.label))
            .filter(|a| matches!(style, Style::Homeward | Style::Return) || !HOMEWARD_SOUNDS.contains(&a.label))
            .filter(|a| !crate::chamber::CHAMBER_SOUNDS.contains(&a.label))
            .filter(|a| !crate::orbit::ORBIT_SOUNDS.contains(&a.label))
            .filter(|a| !crate::nebula::NEBULA_SOUNDS.contains(&a.label))
            .filter(|a| !crate::vigil::VIGIL_SOUNDS.contains(&a.label))
            .collect();
        let w: Vec<f64> = match style {
            Style::Nocturne => (0..alts.len()).map(|i| if i == 0 { 3.0 } else { 1.0 }).collect(),
            // the sounds the style is written for, most often
            // (Derelict's only: never a koto or shakuhachi on board)
            _ => alts
                .iter()
                .map(|a| match (prefers(style, role, a.label), style) {
                    (true, _) => 3.0,
                    (false, Style::Derelict | Style::Homeward | Style::Return) => 0.0,
                    _ => 0.4,
                })
                .collect(),
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
        Style::Overcity | Style::Vigil => [1.0, 5.0, 3.0],
        Style::Derelict => [0.3, 2.0, 5.0],
        // intimate and natural: the hall, now and then the cathedral
        Style::Homeward => [6.0, 1.0, 0.0],
        // the concert hall, always
        Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => [1.0, 0.0, 0.0],
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
        Style::Overcity | Style::Vigil => (-2.0, -4.0, -2.0, -3.0),
        Style::Derelict => (-3.0, -3.0, 0.0, 0.0),
        Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => (-4.0, -3.0, 0.0, 0.0),
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
    let t_keys = matches!(style, Style::Momento | Style::Noir | Style::Homeward | Style::Return).then(|| {
        // from its own stream, so the other parts' sounds stay as they were
        let mut kr = Rng::stream(r.seed, "keys");
        let alts: Vec<Alt> = alternatives(Role::Keys).into_iter().filter(|a| a.bundled).collect();
        let other = if matches!(style, Style::Homeward | Style::Return) { 0.0 } else { 0.4 };
        let w: Vec<f64> = alts.iter().map(|a| if prefers(style, Role::Keys, a.label) { 3.0 } else { other }).collect();
        let alt = alts[kr.weighted(&w)].clone();
        add(Role::Keys, alt, Strip { reverb: 0.45, echo: 0.2, pan: -0.25, gain_db: 1.0, ..s })
    });
    // Derelict's orchestra: tremolo strings and a bowed double bass (their own stream,
    // so the other parts' sounds stay as they were)
    let orchestra = (style == Style::Derelict).then(|| {
        let mut or = Rng::stream(r.seed, "orchestra");
        let mut take = |role: Role, liked: &[&str]| {
            let alts: Vec<Alt> = alternatives(role).into_iter().filter(|a| a.bundled).collect();
            let w: Vec<f64> = alts.iter().map(|a| if liked.contains(&a.label) { 3.0 } else { 0.0 }).collect();
            alts[or.weighted(&w)].clone()
        };
        let strings = take(Role::Strings, &["Tremolo Strings"]);
        let low = take(Role::Bass, &["Contrabass"]);
        let ts = add(Role::Strings, strings, Strip { reverb: 0.7, width: 1.5, gain_db: -3.0, ..s });
        let tb = add(Role::Bass, low, Strip { reverb: 0.55, width: 1.2, gain_db: -2.0, pan: -0.1, ..s });
        (ts, tb)
    });
    // Homeward's strings: cellos (or violas) for the theme, a string section, and a
    // double bass (their own stream, so the other parts' sounds stay as they were)
    // (Return's are chosen: violas for the theme, and an electric piano and cellos of
    // their own)
    let mut extra = None;
    let strings = matches!(style, Style::Homeward | Style::Return).then(|| {
        if style == Style::Return {
            let mut or = Rng::stream(r.seed, "return-strings");
            let named = |role: Role, label: &str| alternatives(role).into_iter().find(|a| a.label == label).expect("bundled");
            let section = {
                let alts = [named(Role::Strings, "Slow Strings"), named(Role::Strings, "Strings")];
                alts[or.weighted(&[3.0, 2.0])].clone()
            };
            let tc = add(Role::Strings, named(Role::Strings, "Viola"), Strip { reverb: 0.5, pan: -0.3, width: 1.2, gain_db: -1.0, ..s });
            let ts = add(Role::Strings, section, Strip { reverb: 0.6, width: 1.6, gain_db: -3.0, ..s });
            let tb = add(Role::Bass, named(Role::Bass, "Contrabass"), Strip { reverb: 0.35, pan: -0.1, gain_db: -2.0, ..s });
            let ep = add(Role::Keys, named(Role::Keys, "Electric Piano"), Strip { reverb: 0.45, pan: -0.1, gain_db: 1.0, ..s });
            let counter =
                add(Role::Strings, named(Role::Strings, "Cello"), Strip { reverb: 0.5, pan: 0.25, width: 1.2, gain_db: -1.0, ..s });
            extra = Some((ep, counter));
            return (tc, ts, tb);
        }
        let mut or = Rng::stream(r.seed, "homeward-strings");
        let mut take = |role: Role, liked: &[f64], labels: &[&str]| {
            let alts: Vec<Alt> = alternatives(role).into_iter().filter(|a| a.bundled).collect();
            let w: Vec<f64> = alts.iter().map(|a| labels.iter().position(|l| *l == a.label).map_or(0.0, |i| liked[i])).collect();
            alts[or.weighted(&w)].clone()
        };
        let cellos = take(Role::Strings, &[3.0, 1.0], &["Cello", "Viola"]);
        let section = take(Role::Strings, &[3.0, 2.0], &["Slow Strings", "Strings"]);
        let low = take(Role::Bass, &[1.0], &["Contrabass"]);
        let tc = add(Role::Strings, cellos, Strip { reverb: 0.5, pan: -0.25, width: 1.2, gain_db: -1.0, ..s });
        let ts = add(Role::Strings, section, Strip { reverb: 0.6, width: 1.6, gain_db: -3.0, ..s });
        let tb = add(Role::Bass, low, Strip { reverb: 0.35, pan: -0.1, gain_db: -2.0, ..s });
        (tc, ts, tb)
    });
    p.tracks = tracks;
    if let (Some((tc, ts, tb)), Some(tk)) = (strings, t_keys) {
        let tracks = crate::homeward::Tracks {
            piano: tk,
            cellos: tc,
            strings: ts,
            double_bass: tb,
            sub: t_bass,
            pad: t_pad,
            lead: t_lead,
            harp: t_pluck,
            ep: extra.map(|e| e.0),
            counter: extra.map(|e| e.1),
        };
        crate::homeward::mix(&mut p, &tracks);
        crate::homeward::write(&mut p, &tracks, r, &secs, density);
        return finish(p, r);
    }
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
    // the city from above: everything far off in a vast space, the theme and the glass
    // trailing tape echoes, the horns and choir wide
    if style == Style::Overcity {
        for (ti, rev, echo) in [(t_lead, 0.7, 0.45), (t_bells, 0.75, 0.55), (t_pluck, 0.65, 0.5), (t_brass, 0.6, 0.1), (t_pad, 0.75, 0.0)] {
            p.tracks[ti].strip.reverb = rev;
            p.tracks[ti].strip.echo = echo;
        }
        p.tracks[t_brass].strip.width = 1.4;
        p.tracks[t_pad].strip.width = 1.5;
        p.tracks[t_pad].strip.high_cut = 9000.0;
        p.tracks[t_city].strip.gain_db -= 4.0;
        p.fx.size = p.fx.size.max(0.8);
        p.fx.echo.feedback = p.fx.echo.feedback.max(0.55);
        p.fx.echo.wow = p.fx.echo.wow.max(0.3);
        // headroom for the great swell
        p.master_db = -2.0;
    }
    // the ship: everything far off in a cavernous dark, worn like old tape, wide; the
    // horn muffled, the ghost of a trumpet drowned in the reverb
    if let Some((ts, tb)) = orchestra {
        p.tracks[tb].name = "Double Bass".into();
        let set = |p: &mut Project, ti: usize, rev: f32, echo: f32, pan: f32, width: f32| {
            let st = &mut p.tracks[ti].strip;
            (st.reverb, st.echo, st.pan, st.width) = (rev, echo, pan, width);
        };
        set(&mut p, t_drone, 0.6, 0.0, 0.0, 1.7);
        set(&mut p, t_pad, 0.75, 0.15, 0.0, 1.6);
        // one horn or trombone, not a chorus of them: no ensemble and no echo, which
        // made the sampled brass sound doubled
        set(&mut p, t_brass, 0.8, 0.0, 0.3, 1.2);
        p.tracks[t_brass].strip.ensemble = false;
        // no string-machine chorus at all: its wavering copies a few ms behind sound like
        // a second, out-of-tune player (the tape is in the echo's wow instead)
        p.tracks[t_pad].strip.ensemble = false;
        p.fx.ensemble = 0.0;
        set(&mut p, t_lead, 0.8, 0.3, -0.25, 1.0);
        set(&mut p, t_answer, 0.95, 0.55, 0.5, 1.0);
        set(&mut p, t_bells, 0.85, 0.5, -0.35, 1.0);
        set(&mut p, t_pluck, 0.75, 0.3, 0.2, 1.4);
        set(&mut p, t_city, 0.5, 0.1, 0.0, 1.7);
        set(&mut p, ts, 0.75, 0.0, 0.0, 1.6);
        p.tracks[t_pad].strip.high_cut = 6000.0;
        p.tracks[t_brass].strip.high_cut = 4000.0;
        p.tracks[t_answer].strip.high_cut = 4500.0;
        p.tracks[t_answer].strip.low_cut = 250.0;
        p.tracks[t_answer].strip.gain_db -= 3.0;
        p.tracks[t_bass].strip.reverb = 0.12;
        // the void is quiet, not empty: the drone and the ship carry it
        p.tracks[t_drone].strip.gain_db += 5.0;
        p.tracks[t_city].strip.gain_db += 5.0;
        p.tracks[t_pad].strip.gain_db += 2.0;
        // long, but short of piling up: about 30 s in the black hole
        p.fx.size = 0.8;
        p.fx.tone = 0.35;
        p.fx.predelay = 0.35;
        p.fx.width = 1.0;
        p.fx.duck = 0.2;
        // the tape: a slow wander of pitch, the parts a little apart, wow on the echo
        p.fx.detune = 0.2;
        p.fx.drift = 0.35;
        p.fx.echo.feedback = 0.6;
        p.fx.echo.wow = 0.35;
        // headroom for the eruption
        p.master_db = -2.5;
        let ship = Ship {
            brass: t_brass,
            pad: t_pad,
            lead: t_lead,
            answer: t_answer,
            bass: t_bass,
            bells: t_bells,
            pluck: t_pluck,
            drone: t_drone,
            city: t_city,
            strings: ts,
            double_bass: tb,
        };
        let tps = beat as f64 / beat_secs;
        derelict(&mut p.tracks, &ship, r, &chords, &secs, &DerelictTime { beat, bar, end, tps }, density);
        return finish(p, r);
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
    if style == Style::Overcity {
        for (ci, c) in chords.iter().enumerate() {
            let ring = if ci == last_chord { end - c.start } else { c.len.saturating_sub(breath / 2) };
            let part = part_at(c.start);
            let open = [c.voicing[0], c.voicing[1], c.voicing[2]];
            match part {
                // out of near-silence, the horns rise on an open fifth and fall away
                Part::Intro => {
                    for k in open {
                        push(&mut t[t_brass].notes, c.start, ring, k, 84);
                    }
                    swell(&mut t[t_brass].ctrls, c.start, ring, 4, 112, 0.65);
                }
                // the choir holds the hollow chord under the theme; the horns breathe
                // in on every other chord
                Part::Neon => {
                    for &k in &c.voicing {
                        push(&mut t[t_pad].notes, c.start, ring, k, 44);
                    }
                    if ci % 2 == 0 {
                        for k in open {
                            push(&mut t[t_brass].notes, c.start, ring, k, 70);
                        }
                        swell(&mut t[t_brass].ctrls, c.start, ring, 6, 80, 0.55);
                    }
                }
                // a high choir above, and a horn swell on every chord
                Part::Swell => {
                    for &k in &c.voicing {
                        push(&mut t[t_pad].notes, c.start, ring, k, 56);
                    }
                    push(&mut t[t_pad].notes, c.start, ring, c.voicing[3] + 12, 40);
                    for &k in &c.voicing {
                        push(&mut t[t_brass].notes, c.start, ring, k, 80);
                    }
                    swell(&mut t[t_brass].ctrls, c.start, ring, 10, 92, 0.6);
                }
                // the great swell: everything open, the horns an octave wider
                Part::NightShift => {
                    for &k in &c.voicing {
                        push(&mut t[t_pad].notes, c.start, ring, k, 66);
                        push(&mut t[t_pad].notes, c.start, ring, k + 12, 54);
                    }
                    for &k in &c.voicing {
                        push(&mut t[t_brass].notes, c.start, ring, k, 112);
                        push(&mut t[t_brass].notes, c.start, ring, k + 12, 96);
                    }
                    push(&mut t[t_brass].notes, c.start, ring, c.voicing[0] - 12, 100);
                    swell(&mut t[t_brass].ctrls, c.start, ring, 30, 127, 0.7);
                }
                // dissolving: the choir alone, a last soft breath of horn on the first chord
                Part::Fade => {
                    for &k in &c.voicing {
                        push(&mut t[t_pad].notes, c.start, ring, k, 44);
                    }
                    if secs.iter().any(|s| s.0 == Part::Fade && s.1 == c.start) {
                        for k in open {
                            push(&mut t[t_brass].notes, c.start, ring, k, 70);
                        }
                        swell(&mut t[t_brass].ctrls, c.start, ring, 60, 80, 0.15);
                    }
                }
            }
        }
    }
    for (ci, c) in chords.iter().enumerate().filter(|_| style != Style::Overcity) {
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
        Style::Overcity | Style::Vigil => {
            // a slow heartbeat, felt more than heard: the root low, twice a bar, from the
            // development through the great swell and on into the first half of the fade
            for (part, a, z) in &secs {
                let stop = match part {
                    Part::Swell | Part::NightShift => *z,
                    Part::Fade => a + (z - a) / 2 / bar * bar,
                    _ => continue,
                };
                let mut b = *a;
                while b < stop {
                    let k = near(chord_at(&chords, b).root_pc, 33);
                    let lift = if *part == Part::NightShift { 14 } else { 0 };
                    push(&mut t[t_bass].notes, b, beat * 3 / 4, k, 74 + lift);
                    push(&mut t[t_bass].notes, b + beat, beat * 3 / 4, k, 54 + lift);
                    b += bar;
                }
            }
        }
        // (Derelict is written by `derelict`)
        Style::Nocturne | Style::Derelict | Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => {}
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
    if style == Style::Overcity {
        // crystalline pings, few and far between, like light on wet streets; and one
        // last distant bell as it all fades
        for c in &chords {
            let part = part_at(c.start);
            if matches!(part, Part::Neon | Part::Swell | Part::Fade) && bel.chance(0.35 + density * 0.2) {
                let k = c.voicing[bel.range(1, 3) as usize] + if bel.chance(0.5) { 24 } else { 36 };
                let off = beat * bel.range(1, (c.len / beat).max(2) as i32 - 1) as u32;
                push(&mut t[t_bells].notes, c.start + off, beat * 2, k.min(108), 38 + bel.range(0, 18) as u8);
            }
        }
        if end > 3 * bar {
            push(&mut t[t_bells].notes, end - 3 * bar, 2 * bar, near(r.root, 86), 54);
        }
        tick = end;
    } else if style != Style::Nocturne {
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
        // (Overcity writes its own theme; these only keep the stream as it would be)
        Style::Overcity
        | Style::Vigil
        | Style::Derelict
        | Style::Homeward
        | Style::Return
        | Style::Chamber
        | Style::Orbit
        | Style::Nebula => {
            vec![&[4.0], &[6.0, 2.0]]
        }
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
    if style == Style::Overcity {
        let tonic_idx = scale.iter().position(|k| *k % 12 == r.root % 12 && *k >= 60).unwrap_or(scale.len() / 2);
        overcity_theme(&mut t[t_lead], &chords, &secs, &scale, tonic_idx, &mut mel, beat, end);
    }
    for (s2, s3) in of(Part::NightShift).filter(|_| style != Style::Overcity) {
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
    if style == Style::Overcity {
        // slow ripples high up, through the development and the great swell: the chord
        // over two octaves in a figure each seed shapes its own way for each section (up,
        // down, up and down, down and up, leaping, closing in, or scattered), at its own
        // pace, length and place in the bar (its own stream, so the rest is as it was)
        let mut rs = Rng::stream(r.seed, "ripple");
        for (part, a, z) in &secs {
            if !matches!(part, Part::Swell | Part::NightShift) {
                continue;
            }
            let shape = rs.range(0, 6);
            let step = *rs.pick(&[beat / 2, beat / 2, beat * 3 / 4, beat / 3]);
            let fits = (bar / step.max(1)).max(3) as i32;
            let count = rs.range(4, 8).min(fits) as usize;
            let offset = if rs.chance(0.4) && step * count as u32 + beat <= bar { beat } else { 0 };
            let scatter: Vec<usize> = {
                let mut v: Vec<usize> = (0..count).collect();
                for i in (1..v.len()).rev() {
                    let j = rs.range(0, i as i32) as usize;
                    v.swap(i, j);
                }
                v
            };
            let mut b = *a;
            while b < *z {
                let c = chord_at(&chords, b);
                let mut tones: Vec<u8> = c.voicing.iter().map(|k| k + 12).chain(c.voicing.iter().map(|k| k + 24)).collect();
                tones.sort_unstable();
                tones.dedup();
                // the figure's notes, low to high, then in the shape's order
                let span: Vec<u8> = (0..count).map(|j| tones[j * (tones.len() - 1) / (count - 1).max(1)]).collect();
                let order = ripple_order(shape, count, &scatter);
                if pl.chance(0.55 + density * 0.3) {
                    for (j, &o) in order.iter().enumerate() {
                        // brightest at the top of the figure
                        let vel = 34 + (o as u8) * 18 / count as u8 + pl.range(0, 8) as u8;
                        push(&mut t[t_pluck].notes, b + offset + j as u32 * step, beat, span[o].min(103), vel);
                    }
                }
                b += bar;
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
    for (s3, s4) in of(Part::Fade).filter(|_| style != Style::Overcity) {
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
    for (a, z) in of(Part::Intro).filter(|_| style != Style::Overcity) {
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
    // (in Overcity an octave down, almost felt rather than heard)
    let tonic = near(r.root, if style == Style::Overcity { 38 } else { 45 });
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
        Style::Overcity | Style::Vigil => {
            // the city far below: a soft wash of rain, its rumble, distant thunder at the
            // edges of the form, and a deep gong-like boom on the great swell
            let pc = r.root % 12;
            t[t_city].notes.clear();
            let soft: Vec<RainKind> = RainKind::ALL.into_iter().filter(|k| k.sparse()).collect();
            let rain = if soft.is_empty() { RainKind::ALL[0] } else { *deep.pick(&soft) };
            push(&mut t[t_city].notes, 0, end, rain.key(), 62);
            push(&mut t[t_city].notes, 0, end, fx::HUM, 46);
            for (i, (part, a, z)) in secs.iter().enumerate() {
                if matches!(part, Part::Intro | Part::Fade) {
                    push(&mut t[t_city].notes, *a, (z - a).min(bar * 2), fx::GRAINS + pc, 44);
                }
                if i > 0 && *part != secs[i - 1].0 && *a > beat {
                    push(&mut t[t_city].notes, a - beat, beat * 2, fx::THUNDER, if *part == Part::NightShift { 96 } else { 62 });
                }
                if *part == Part::NightShift {
                    push(&mut t[t_city].notes, *a, bar * 2, fx::BOOM + pc, 96);
                    let mid = a + (z - a) / 2 / bar * bar;
                    if mid > *a {
                        push(&mut t[t_city].notes, mid, bar * 2, fx::BOOM + pc, 76);
                    }
                }
            }
        }
        Style::Nocturne
        | Style::Momento
        | Style::Noir
        | Style::Derelict
        | Style::Homeward
        | Style::Return
        | Style::Chamber
        | Style::Orbit
        | Style::Nebula => {}
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
        Style::Overcity | Style::Vigil => {
            // only the largest moments: a slow, soft, deep hit every other bar of the great swell
            for (a, z) in of(Part::NightShift) {
                let mut b = a;
                while b < z {
                    push(&mut t[t_drums].notes, b, beat * 2, drum::DEEP_KICK, if b == a { 100 } else { 70 });
                    b += 2 * bar;
                }
            }
        }
        Style::Nocturne | Style::Derelict | Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula => {}
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

    finish(p, r)
}

/// The last touches to a composed piece: its tracks sorted, the silent ones gone, and
/// its recipe and fingerprint kept.
pub(crate) fn finish(mut p: Project, r: &Recipe) -> Project {
    for tr in p.tracks.iter_mut() {
        tr.sort();
    }
    // the tracks the piece leaves silent go
    p.tracks.retain(|tr| !tr.notes.is_empty());
    p.recipe = Some(*r);
    p.composed = fingerprint(&p);
    p
}

/// A swell on the horns: the expression rising from `lo` to `hi` and falling back, the
/// peak `peak` of the way through, in eight steps each way.
fn swell(ctrls: &mut Vec<Ctrl>, a: u32, len: u32, lo: i16, hi: i16, peak: f64) {
    swell_in(ctrls, a, len, lo, hi, peak, 8);
}

/// A swell in `steps` steps each way (more for a smooth one over a long note).
fn swell_in(ctrls: &mut Vec<Ctrl>, a: u32, len: u32, lo: i16, hi: i16, peak: f64, steps: u32) {
    let top = a + (len as f64 * peak) as u32;
    // (a swell that peaks at its end holds there: no fall squeezed into one instant)
    let falls = top < a + len;
    let fine = steps > 8;
    for q in 0..=steps {
        let x = q as f64 / steps as f64;
        let points = [
            (a + ((top - a) as f64 * x) as u32, lo + ((hi - lo) as f64 * x * x) as i16),
            (top + ((a + len - top) as f64 * x) as u32, hi - ((hi - lo) as f64 * x) as i16),
        ];
        for (j, (tick, value)) in points.into_iter().enumerate() {
            if j == 1 && !falls {
                continue;
            }
            ctrls.push(Ctrl { tick, kind: CtrlKind::Cc(11), value });
            if fine {
                // the filter follows the expression step by step: thrown open at once,
                // it's heard as a stab
                ctrls.push(Ctrl { tick, kind: CtrlKind::Cc(74), value: 30 + value * 45 / 100 });
            }
        }
    }
    if !fine {
        // the filter opens with it, a little behind (three points, as the styles before
        // Derelict were written)
        ctrls.push(Ctrl { tick: a, kind: CtrlKind::Cc(74), value: 40 });
        ctrls.push(Ctrl { tick: top, kind: CtrlKind::Cc(74), value: 64 + (hi - lo) / 4 });
        ctrls.push(Ctrl { tick: a + len, kind: CtrlKind::Cc(74), value: 44 });
    }
}

/// The order a ripple of `count` notes (0 lowest) plays in: 0 up, 1 down, 2 up and back
/// down, 3 down and back up, 4 closing in from the outside, 5 opening out from the
/// middle, else `scatter`.
fn ripple_order(shape: i32, count: usize, scatter: &[usize]) -> Vec<usize> {
    // up on the even notes, down on the odd ones
    let arch: Vec<usize> = (0..count).step_by(2).chain((1..count).step_by(2).rev()).collect();
    let closing: Vec<usize> = (0..count).map(|j| if j % 2 == 0 { j / 2 } else { count - 1 - j / 2 }).collect();
    match shape {
        0 => (0..count).collect(),
        1 => (0..count).rev().collect(),
        2 => arch,
        3 => arch.iter().map(|x| count - 1 - x).collect(),
        4 => closing,
        5 => closing.into_iter().rev().collect(),
        _ => scatter.to_vec(),
    }
}

/// Overcity's theme: slow, aching phrases of long notes, each opening with a rising
/// leap (a fifth or a sixth) and then falling step by step through the scale to a
/// held chord tone, with a long silence after. Each statement starts a little higher
/// and plays a little stronger. Stated in the opening Neon, varied through the
/// development, sung once more at its highest into the great swell.
#[allow(clippy::too_many_arguments)]
fn overcity_theme(
    track: &mut Track,
    chords: &[Chord],
    secs: &[(Part, u32, u32)],
    scale: &[u8],
    tonic_idx: usize,
    rng: &mut Rng,
    beat: u32,
    end: u32,
) {
    let n = scale.len() as i32;
    let at = |b: f64| (b * beat as f64).round() as u32;
    let mut phrase = 0i32;
    for (part, a, z) in secs {
        let (lift_base, once) = match part {
            Part::Neon => (0, false),
            Part::Swell => (1, false),
            Part::NightShift => (4, true),
            _ => continue,
        };
        // a breath in before the first note
        let mut tick = a + at(2.0);
        while tick + at(8.0) < *z {
            let c = chord_at(chords, tick);
            // start on a chord tone near the tonic, a step higher each time round
            let lift = (lift_base + phrase.min(3)).min(5);
            let mut i = (tonic_idx as i32 - 3 + lift).clamp(0, n - 1);
            if let Some(j) = (0..n).filter(|j| c.pcs.contains(&(scale[*j as usize] % 12))).min_by_key(|j| (j - i).abs()) {
                i = j;
            }
            let leap = if rng.chance(0.55) { 4 } else { 5 };
            let falls = rng.range(2, 4);
            let mut notes: Vec<(i32, f64)> = vec![(i, 2.0), ((i + leap).min(n - 1), 4.0)];
            let mut k = (i + leap).min(n - 1);
            for f in 0..falls {
                k = (k - 1).max(0);
                let d = if f == falls - 1 {
                    2.0
                } else if rng.chance(0.5) {
                    1.0
                } else {
                    1.5
                };
                notes.push((k, d));
            }
            // the held end, on a chord tone near where the fall arrives
            let mut t0 = tick + notes.iter().map(|(_, d)| at(*d)).sum::<u32>();
            let land = chord_at(chords, t0.min(end.saturating_sub(1)));
            let close = (0..n).filter(|j| land.pcs.contains(&(scale[*j as usize] % 12))).min_by_key(|j| (j - (k - 1)).abs()).unwrap_or(k);
            notes.push((close, 5.0));
            let vel = (78 + phrase * 6 + if once { 14 } else { 0 }).min(116) as u8;
            t0 = tick;
            for (ni, (j, d)) in notes.iter().enumerate() {
                if t0 >= *z || t0 >= end {
                    break;
                }
                let len = at(*d).min(z - t0);
                let key = scale[*j as usize];
                track.notes.push(Note { start: t0, len, key, vel: vel.saturating_sub(if ni >= 2 { 6 } else { 0 }) });
                if ni == 1 {
                    // slide up into the top of the leap
                    for (dt, v) in [(0, -2400), (beat / 6, -900), (beat / 3, 0)] {
                        track.ctrls.push(Ctrl { tick: t0 + dt.min(len), kind: CtrlKind::Bend, value: v });
                    }
                }
                if *d >= 4.0 {
                    // the long notes bloom: a slow vibrato swell
                    for q in 0..=8 {
                        track.ctrls.push(Ctrl { tick: t0 + len * q / 8, kind: CtrlKind::Cc(1), value: (q * 10) as i16 });
                    }
                    track.ctrls.push(Ctrl { tick: t0 + len, kind: CtrlKind::Cc(1), value: 0 });
                }
                t0 += at(*d);
            }
            phrase += 1;
            if once {
                break;
            }
            // and generous space after
            tick = t0 + at(rng.range(4, 7) as f64);
        }
    }
}

/// The sounds added to the palette for Derelict: the other styles leave them out of
/// their choosing, so every recipe they had still sounds as it did.
const DERELICT_SOUNDS: [&str; 13] = [
    "Tremolo Strings",
    "Cor Anglais",
    "Solo Oboe",
    "Muted Trumpet",
    "Contrabass",
    "Trombone",
    "Soundtrack",
    "Metal Pad",
    "Space Voice",
    "Bassoon",
    "Baritone Sax",
    "Pizzicato Strings",
    "Goblin",
];

/// Derelict's tracks.
struct Ship {
    brass: usize,
    pad: usize,
    lead: usize,
    answer: usize,
    bass: usize,
    bells: usize,
    pluck: usize,
    drone: usize,
    city: usize,
    strings: usize,
    double_bass: usize,
}

/// The beat and bar in ticks, the piece's end, and ticks a second.
struct DerelictTime {
    beat: u32,
    bar: u32,
    end: u32,
    tps: f64,
}

/// Derelict: a ship adrift in deep space, almost free of time. The void (Intro): the
/// drone, tape hiss, faint ticking, a hull groaning and a pipe ringing far off. Creeping
/// tension (Neon): the life support pulsing on the sub in uneven groups, the worn pad
/// breathing in and out with long silences between, a lone horn, a celesta, steam
/// venting, and an oboe or cor anglais surfacing with a fragment that never resolves.
/// Tightening (Swell): tremolo strings entering one by one, each when it will, over a
/// bowed double bass, metal scraping, one sudden swell cut off into silence. A brief
/// eruption (Night shift): a harp glissando up into horns, strings and pad at full
/// cry over timpani and a deep boom. The collapse (Fade): a glissando falling, the
/// pulse failing, the machinery winding down, a ghostly muted trumpet or saxophone
/// drifting past, and the drone alone with a last celesta. Very little melody; most
/// entries fall a little off the beat, so there is no pulse to hear.
fn derelict(t: &mut [Track], k: &Ship, r: &Recipe, chords: &[Chord], secs: &[(Part, u32, u32)], tm: &DerelictTime, density: f64) {
    let DerelictTime { beat, bar, end, tps } = *tm;
    // swells stepped every eighth of a beat or so, so a long one climbs smoothly rather
    // than in audible stairs
    let swell = |ctrls: &mut Vec<Ctrl>, a: u32, len: u32, lo: i16, hi: i16, peak: f64| {
        swell_in(ctrls, a, len, lo, hi, peak, (len / (beat / 8).max(1)).clamp(8, 160));
    };
    let push = |v: &mut Vec<Note>, start: u32, len: u32, key: u8, vel: u8| {
        if start < end && len > 0 {
            v.push(Note { start, len: len.min(end - start), key, vel: vel.clamp(1, 127) });
        }
    };
    let part_at = |tick: u32| secs.iter().find(|(_, a, z)| tick >= *a && tick < *z).map_or(Part::Fade, |s| s.0);
    let first_of = |c: &Chord, part: Part| secs.iter().any(|s| s.0 == part && s.1 == c.start);
    let pc = r.root % 12;
    let steps = r.mode.steps();
    let in_key = |k: &u8| steps.contains(&((k + 12 - pc) % 12));
    let last_chord = chords.len() - 1;
    let ring = |ci: usize, c: &Chord| if ci == last_chord { end - c.start } else { c.len.saturating_sub(beat / 2) };
    // the start of each eruption that follows something else
    let eruptions: Vec<(u32, u32)> = secs
        .iter()
        .enumerate()
        .filter(|(i, s)| s.0 == Part::NightShift && (*i == 0 || secs[i - 1].0 != Part::NightShift))
        .map(|(_, s)| (s.1, s.2))
        .collect();

    // ------------------------------------------------------------ the drone
    let mut rng = Rng::stream(r.seed, "derelict-drone");
    let tonic = near(pc, 38);
    for (part, a, z) in secs {
        push(&mut t[k.drone].notes, *a, z - a, tonic, if *part == Part::Intro { 70 } else { 84 });
        if matches!(part, Part::Swell | Part::NightShift) {
            push(&mut t[k.drone].notes, *a, z - a, tonic + 7, 62);
        }
        if *part == Part::NightShift || (*part == Part::Swell && rng.chance(0.5)) {
            // a minor ninth grinding above
            push(&mut t[k.drone].notes, *a, z - a, tonic + 13, 40);
        }
    }

    // ------------------------------------------------------------ the worn pad
    let mut rng = Rng::stream(r.seed, "derelict-pad");
    // the one sudden swell in each tightening: any chord but the last before what follows
    let sudden: Vec<u32> = secs
        .iter()
        .filter(|s| s.0 == Part::Swell)
        .filter_map(|(_, a, z)| {
            let starts: Vec<u32> = chords.iter().filter(|c| c.start >= *a && c.start + c.len < *z).map(|c| c.start).collect();
            (!starts.is_empty()).then(|| starts[rng.range(0, starts.len() as i32 - 1) as usize])
        })
        .collect();
    for (ci, c) in chords.iter().enumerate() {
        let len = ring(ci, c);
        match part_at(c.start) {
            Part::Intro => {}
            // on some chords, breathing a little louder and back, long silences between
            Part::Neon => {
                if ci % 2 == 1 || rng.chance(0.3) {
                    continue;
                }
                for &key in &c.voicing {
                    push(&mut t[k.pad].notes, c.start, len, key, 56);
                }
                swell(&mut t[k.pad].ctrls, c.start, len, 44, 84, 0.55);
            }
            // every chord, steady with a gentle swell; one in each section sudden, rising
            // hard and cut off into nothing
            Part::Swell => {
                let sudden = sudden.contains(&c.start);
                let len = if sudden { len * 2 / 3 } else { len };
                for &key in &c.voicing {
                    push(&mut t[k.pad].notes, c.start, len, key, 64);
                }
                if sudden {
                    swell(&mut t[k.pad].ctrls, c.start, len, 40, 112, 0.8);
                } else {
                    swell(&mut t[k.pad].ctrls, c.start, len, 56, 96, 0.6);
                }
            }
            Part::NightShift => {
                for &key in &c.voicing {
                    push(&mut t[k.pad].notes, c.start, len, key, 92);
                    push(&mut t[k.pad].notes, c.start, len, key + 12, 70);
                }
                swell(&mut t[k.pad].ctrls, c.start, len, 70, 127, 0.2);
            }
            // the collapse: the top of the chord, dying away, on the first chord only
            Part::Fade => {
                if first_of(c, Part::Fade) {
                    for &key in c.voicing.iter().rev().take(3) {
                        push(&mut t[k.pad].notes, c.start, len, key, 56);
                    }
                    swell(&mut t[k.pad].ctrls, c.start, len, 50, 86, 0.1);
                }
            }
        }
    }

    // ------------------------------------------------------------ the life support
    // a sub pulsing in uneven groups, like failing machinery: twos in the tension,
    // threes as it tightens, fours in the eruption, and in the collapse ever further
    // apart, notes dropping out, until it stops
    let mut rng = Rng::stream(r.seed, "derelict-pulse");
    for (part, a, z) in secs {
        let (n, mut gap, stop) = match part {
            Part::Neon => (2, 2 * bar, *z),
            Part::Swell => (3, bar + beat * 2, *z),
            Part::NightShift => (4, bar, *z),
            Part::Fade => (2, 2 * bar, a + (z - a) * 2 / 3),
            Part::Intro => continue,
        };
        let mut b = a + beat / 2 + rng.range(0, beat as i32) as u32;
        while b < stop {
            let key = near(chord_at(chords, b).root_pc, 33);
            for j in 0..n {
                if *part == Part::Fade && rng.chance(0.3) {
                    continue;
                }
                push(&mut t[k.bass].notes, b + j * beat * 3 / 4, beat / 2, key, 92 - j as u8 * 9);
            }
            if *part == Part::Fade {
                gap += bar;
            }
            b += gap + rng.range(0, beat as i32) as u32;
        }
    }

    // ------------------------------------------------------------ tremolo strings
    // in the tightening each note of a cluster enters when it will, the whole slowly
    // louder; at full cry in the eruption; a last high thread at the collapse
    let mut rng = Rng::stream(r.seed, "derelict-strings");
    for (part, a, z) in secs {
        if *part == Part::Swell {
            swell(&mut t[k.strings].ctrls, *a, z - a, 40, 104, 1.0);
        }
    }
    for (ci, c) in chords.iter().enumerate() {
        let len = ring(ci, c);
        match part_at(c.start) {
            Part::Swell => {
                let top = *c.voicing.last().unwrap_or(&64);
                for &key in c.voicing.iter().chain([top + 1].iter()) {
                    let late = rng.range(0, (beat * 3) as i32) as u32;
                    push(&mut t[k.strings].notes, c.start + late, len.saturating_sub(late), key + 12, 70 + rng.range(0, 14) as u8);
                }
            }
            Part::NightShift => {
                for &key in &c.voicing {
                    push(&mut t[k.strings].notes, c.start, len, key + 12, 112);
                    push(&mut t[k.strings].notes, c.start, len, key + 24, 96);
                }
                // (from where the tightening's crescendo left off)
                swell(&mut t[k.strings].ctrls, c.start, len, 100, 127, 0.15);
            }
            Part::Fade if first_of(c, Part::Fade) => {
                push(&mut t[k.strings].notes, c.start + beat, len, near(pc, 84), 44);
                swell(&mut t[k.strings].ctrls, c.start + beat, len, 20, 60, 0.2);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ bowed double bass
    for (ci, c) in chords.iter().enumerate() {
        let len = ring(ci, c);
        let key = near(c.root_pc, 34);
        match part_at(c.start) {
            Part::Swell => {
                push(&mut t[k.double_bass].notes, c.start + beat / 2, len, key, 76);
                swell(&mut t[k.double_bass].ctrls, c.start, len, 60, 100, 0.6);
            }
            Part::NightShift => {
                push(&mut t[k.double_bass].notes, c.start, len, key, 110);
                push(&mut t[k.double_bass].notes, c.start, len, key + 12, 96);
                swell(&mut t[k.double_bass].ctrls, c.start, len, 70, 127, 0.2);
            }
            Part::Fade if first_of(c, Part::Fade) => {
                push(&mut t[k.double_bass].notes, c.start, len, key, 74);
                swell(&mut t[k.double_bass].ctrls, c.start, len, 50, 92, 0.1);
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ the horns
    // a lone horn in the tension, a call of three notes that turns on a semitone; one
    // swell rising towards the eruption; the eruption itself
    let mut rng = Rng::stream(r.seed, "derelict-horn");
    let horn_scale: Vec<u8> = (48..=72u8).filter(in_key).collect();
    for (part, a, z) in secs {
        match part {
            Part::Neon if z - a >= 2 * bar => {
                let mut b = a + bar + rng.range(0, beat as i32 * 2) as u32;
                let home = horn_scale.iter().position(|x| *x >= near(pc, 57)).unwrap_or(0) as i32;
                let turn: [i32; 3] = *rng.pick(&[[0, 1, 0], [4, 5, 4], [0, -1, -2], [2, 1, 0]]);
                let lens = [3, 5, 7];
                for (j, d) in turn.iter().zip(lens) {
                    let key = horn_scale[(home + j).clamp(0, horn_scale.len() as i32 - 1) as usize];
                    let len = beat * d;
                    if b + len > *z {
                        break;
                    }
                    push(&mut t[k.brass].notes, b, len, key, 58);
                    swell(&mut t[k.brass].ctrls, b, len, 30, 76, 0.4);
                    b += len + beat / 2;
                }
            }
            _ => {}
        }
    }
    for (ci, c) in chords.iter().enumerate() {
        let len = ring(ci, c);
        let part = part_at(c.start);
        let next_erupts = eruptions.iter().any(|(a, _)| *a == c.start + c.len);
        if part == Part::Swell && next_erupts {
            // (let go just before the eruption strikes the same notes again)
            for key in [c.voicing[0], near((c.root_pc + 7) % 12, c.voicing[0] as i32 + 7)] {
                push(&mut t[k.brass].notes, c.start, c.len.saturating_sub(beat / 8), key, 80);
            }
            swell(&mut t[k.brass].ctrls, c.start, c.len, 20, 104, 1.0);
        }
        if part == Part::NightShift {
            for &key in &c.voicing {
                push(&mut t[k.brass].notes, c.start, len, key, 118);
                push(&mut t[k.brass].notes, c.start, len, key.saturating_sub(12), 104);
            }
            // (from where the build into it left off)
            swell(&mut t[k.brass].ctrls, c.start, len, 100, 127, 0.25);
        }
    }

    // ------------------------------------------------------------ the oboe
    // a fragment surfacing out of the dark: a few long notes moving by step and
    // semitone, ending somewhere unresolved (late in the tension, early in the
    // tightening)
    let mut rng = Rng::stream(r.seed, "derelict-oboe");
    // (a bassoon an octave and a half lower than an oboe)
    let low_reed = matches!(&t[k.lead].source, Source::SoundFont { bank: 0, program: 70, .. });
    let reed: Vec<u8> = if low_reed { (46..=67u8).filter(in_key).collect() } else { (58..=79u8).filter(in_key).collect() };
    for (part, a, z) in secs {
        let from = match part {
            Part::Neon => a + (z - a) / 2,
            Part::Swell if rng.chance(0.4 + density * 0.4) => *a + bar,
            _ => continue,
        };
        let mut b = from + rng.range(0, beat as i32 * 2) as u32;
        let mut i = reed.iter().position(|x| *x >= near((pc + 7) % 12, if low_reed { 55 } else { 67 })).unwrap_or(reed.len() / 2) as i32;
        let notes = rng.range(3, 5);
        for n in 0..notes {
            let d = if n == notes - 1 { 6 } else { *rng.pick(&[2u32, 3, 3, 4]) };
            let len = beat * d;
            if b + len > *z {
                break;
            }
            let key = if n == notes - 1 {
                // the last, a second above the tonic or a sixth: never home
                near((pc + steps[if rng.chance(0.5) { 1 } else { 5 }]) % 12, reed[i as usize] as i32)
            } else {
                reed[i.clamp(0, reed.len() as i32 - 1) as usize]
            };
            push(&mut t[k.lead].notes, b, len, key, 66);
            if d >= 3 {
                for q in 0..=8 {
                    t[k.lead].ctrls.push(Ctrl { tick: b + len * q / 8, kind: CtrlKind::Cc(1), value: (q * 7) as i16 });
                }
                t[k.lead].ctrls.push(Ctrl { tick: b + len, kind: CtrlKind::Cc(1), value: 0 });
            }
            i = (i + *rng.pick(&[-1, -1, 1, -2, 2])).clamp(0, reed.len() as i32 - 1);
            b += len + if rng.chance(0.4) { beat } else { 0 };
        }
    }

    // ------------------------------------------------------------ the ghost
    // in the collapse, a muted trumpet (or a saxophone) drifts past far off: a smoky
    // phrase bending through the blue note, ending on the minor third
    let mut rng = Rng::stream(r.seed, "derelict-ghost");
    // (a baritone sax an octave down)
    let home = near(pc, if matches!(&t[k.answer].source, Source::SoundFont { bank: 0, program: 67, .. }) { 50 } else { 62 });
    for (part, a, z) in secs {
        if *part != Part::Fade || z - a < 3 * bar {
            continue;
        }
        let shape: &[(i32, f64)] = rng.pick(&[
            &[(0, 1.5), (3, 1.0), (5, 2.0), (6, 3.0), (5, 1.0), (3, 6.0)][..],
            &[(7, 2.0), (6, 1.0), (5, 3.0), (3, 2.0), (0, 6.0)][..],
            &[(3, 2.0), (5, 1.5), (3, 1.5), (-2, 3.0), (0, 1.0), (3, 6.0)][..],
        ]);
        let mut b = a + bar + rng.range(0, beat as i32 * 2) as u32;
        for (j, (st, d)) in shape.iter().enumerate() {
            let len = (*d * beat as f64) as u32;
            if b + len > *z {
                break;
            }
            push(&mut t[k.answer].notes, b, len, (home as i32 + st) as u8, 64);
            if j == 0 || *d >= 3.0 {
                // a scoop up into it
                for (dt, v) in [(0, -1800), (beat / 4, -600), (beat / 2, 0)] {
                    t[k.answer].ctrls.push(Ctrl { tick: b + dt.min(len), kind: CtrlKind::Bend, value: v });
                }
            }
            b += len;
        }
    }

    // ------------------------------------------------------------ celesta
    let mut rng = Rng::stream(r.seed, "derelict-celesta");
    for c in chords {
        let part = part_at(c.start);
        if matches!(part, Part::Neon | Part::Swell) && rng.chance(0.3 + density * 0.25) {
            let key = (c.voicing[rng.range(0, c.voicing.len() as i32 - 1) as usize] + 24).min(100);
            let b = c.start + beat * rng.range(1, (c.len / beat).max(2) as i32 - 1) as u32 + rng.range(0, beat as i32 / 2) as u32;
            push(&mut t[k.bells].notes, b, beat * 2, key, 42 + rng.range(0, 14) as u8);
            if rng.chance(0.4) {
                // and a tritone answering it
                push(&mut t[k.bells].notes, b + beat + beat / 3, beat * 2, (key + 6).min(106), 34);
            }
        }
    }
    if end > 4 * bar {
        let b = end - 3 * bar;
        push(&mut t[k.bells].notes, b, 2 * bar, near(pc, 84), 46);
        push(&mut t[k.bells].notes, b + beat * 3 / 2, 2 * bar, near((pc + 1) % 12, 85), 30);
    }

    // ------------------------------------------------------------ harp glissandi
    // up into each eruption, down as it collapses, and once slowly in the tightening;
    // each its own gesture, drawn from the seed (its own stream, so the rest is as it was)
    let mut hr = Rng::stream(r.seed, "derelict-harp");
    let scale: Vec<u8> = (36..=96u8).filter(in_key).collect();
    for (a, z) in &eruptions {
        let run = harp_run(&mut hr, &scale, chord_at(chords, a.saturating_sub(1)), true, 1.0, beat);
        let span = run.last().map_or(0, |n| n.0);
        if *a > span {
            for (o, key, vel) in &run {
                push(&mut t[k.pluck].notes, a - span - 1 + o, beat, *key, *vel);
            }
        }
        if *z < end {
            for (o, key, vel) in harp_run(&mut hr, &scale, chord_at(chords, *z), false, 0.85, beat) {
                push(&mut t[k.pluck].notes, z + o, beat, key, vel);
            }
        }
    }
    for (part, a, z) in secs {
        if *part == Part::Swell && z - a >= 3 * bar {
            let b = a + (z - a) / 2 + hr.range(0, beat as i32) as u32;
            let up = hr.chance(0.6);
            for (o, key, vel) in harp_run(&mut hr, &scale, chord_at(chords, b), up, 0.45, beat * 2) {
                push(&mut t[k.pluck].notes, b + o, beat, key, vel);
            }
        }
    }

    // ------------------------------------------------------------ the ship
    let mut rng = Rng::stream(r.seed, "derelict-ship");
    let city = &mut t[k.city].notes;
    let secs_to_ticks = |s: f64| (s * tps).round() as u32;
    // the tape's hiss, faint; the ship's engines throbbing deep below, distant in the
    // void, labouring in the eruption, winding down and stopping as it collapses; the
    // life support breathing until it fails
    push(city, 0, end, fx::TAPE_HISS, 28);
    for (part, a, z) in secs {
        let (vel, to) = match part {
            Part::Intro => (58, *z),
            Part::NightShift => (96, *z),
            Part::Fade => (64, a + (z - a) * 2 / 3),
            _ => (78, *z),
        };
        push(city, *a, to.saturating_sub(*a), fx::ENGINE, vel);
        if matches!(part, Part::Neon | Part::Swell | Part::Fade) {
            let to = if *part == Part::Fade { a + (z - a) / 2 } else { *z };
            push(city, *a, to.saturating_sub(*a), fx::PUMP, if *part == Part::Fade { 44 } else { 54 });
        }
    }
    let first = secs.iter().find(|s| s.0 != Part::Intro).map_or(0, |s| s.1);
    if first < end {
        push(city, first, end - first, fx::ANALOG_DRONE + pc, 56);
    }
    for (i, (part, a, z)) in secs.iter().enumerate() {
        // faint ticking in the void and the tension, and in the collapse until just
        // before the end
        match part {
            Part::Intro | Part::Neon => push(city, *a, z - a, fx::TICKING, if *part == Part::Intro { 50 } else { 38 }),
            Part::Fade => {
                let stop = z.saturating_sub(2 * bar).max(*a);
                push(city, a + bar, stop.saturating_sub(a + bar), fx::TICKING, 46);
            }
            _ => {}
        }
        let mut b = *a;
        while b < *z {
            let off = |rng: &mut Rng| b + rng.range(0, (bar - beat / 2) as i32) as u32;
            let root = chord_at(chords, b).root_pc;
            let (groan, steam, clang, scrape) = match part {
                Part::Intro => (0.25, 0.0, 0.2, 0.0),
                Part::Neon => (0.15, 0.08 + density * 0.08, 0.12, 0.04),
                Part::Swell => (0.2, 0.1 + density * 0.1, 0.06, 0.15 + density * 0.1),
                Part::NightShift => (0.0, 0.0, 0.0, 0.0),
                Part::Fade => (0.18, 0.05, 0.12, 0.05),
            };
            if (b == *a && *part == Part::Intro) || rng.chance(groan) {
                let o = off(&mut rng);
                push(city, o, 2 * bar, fx::HULL + root, 64 + rng.range(0, 28) as u8);
            }
            if rng.chance(steam) {
                let o = off(&mut rng);
                push(city, o, bar, fx::STEAM, 40 + rng.range(0, 24) as u8);
            }
            if rng.chance(clang) {
                let o = off(&mut rng);
                let note = [pc, (pc + 1) % 12, (pc + 6) % 12][rng.range(0, 2) as usize];
                push(city, o, 2 * bar, fx::PIPE + note, 34 + rng.range(0, 26) as u8);
            }
            if rng.chance(scrape) {
                let o = off(&mut rng);
                push(city, o, bar, if rng.chance(0.6) { fx::SCRAPE } else { fx::METAL }, 50 + rng.range(0, 26) as u8);
            }
            b += bar;
        }
        // into each eruption: a reversed swell, a timpani roll, and a deep boom on it
        if *part == Part::NightShift && (i == 0 || secs[i - 1].0 != Part::NightShift) {
            let rev = secs_to_ticks(dd_dsp::kit::REVERSE_LONG_SECS);
            if *a >= rev {
                push(city, a - rev, rev, fx::REVERSE_LONG, 70);
            }
            let roll = (beat / 6).max(1);
            let from = a.saturating_sub(bar);
            let to = (*a + bar).min(*z);
            let mut b = from;
            while b < to {
                let x = if b < *a { (b - from) as f64 / (a - from).max(1) as f64 } else { 1.0 - (b - a) as f64 / (to - a).max(1) as f64 };
                push(city, b, roll, fx::TIMPANI + pc, (24.0 + 64.0 * x) as u8);
                b += roll;
            }
            push(city, *a, 2 * bar, fx::BOOM + pc, 78);
            push(city, *a, 2 * bar, fx::TIMPANI + pc, 92);
        }
        // and the collapse: falling away, a softer boom
        if *part == Part::Fade && i > 0 && secs[i - 1].0 != Part::Fade {
            push(city, *a, 2 * bar, fx::DOWNLIFTER, 72);
            push(city, *a, 2 * bar, fx::BOOM + pc, 56);
        }
    }
    // swells that would run on past the end finish on it
    for tr in t.iter_mut() {
        for c in tr.ctrls.iter_mut() {
            c.tick = c.tick.min(end.saturating_sub(1));
        }
    }
}

/// A harp glissando drawn from `rng`: its notes as (offset from the first, key, velocity),
/// rising or falling. The seed chooses what it runs through (every note of the scale, the
/// scale in thirds, five notes of it, or the chord spread as an arpeggio), where it
/// starts and how far it goes (one and a half to three octaves), its shape (straight,
/// stepping back as it goes, or in two breaths), its pace (`beat` is a beat at the
/// slowest; even, quickening or slowing), whether it doubles an octave down, and how it
/// swells. `strength` scales how loud it gets.
fn harp_run(rng: &mut Rng, scale: &[u8], chord: &Chord, up: bool, strength: f64, beat: u32) -> Vec<(u32, u8, u8)> {
    // the notes it may use
    let pool: Vec<u8> = match rng.weighted(&[3.0, 2.0, 1.5, 2.0]) {
        0 => scale.to_vec(),
        1 => scale.iter().copied().step_by(2).collect(),
        // five of the seven: leave out two degrees
        2 => {
            let (x, y) = (rng.range(0, 6) as u8, rng.range(0, 6) as u8);
            let degree = |k: u8| scale.iter().position(|s| *s % 12 == k % 12).unwrap_or(0) as u8 % 7;
            scale.iter().copied().filter(|k| degree(*k) != x && degree(*k) != y).collect()
        }
        _ => scale.iter().copied().filter(|k| chord.pcs.contains(&(k % 12))).collect(),
    };
    let pool = if pool.len() < 6 { scale.to_vec() } else { pool };
    // where it lies: a low start between about C2 and C4, one and a half to three octaves
    let lo = rng.range(40, 58) as u8;
    let mut hi = lo.saturating_add(rng.range(18, 36) as u8).min(98);
    let within = |hi: u8| pool.iter().copied().filter(|k| *k >= lo && *k <= hi).collect::<Vec<u8>>();
    // at least eight notes, or it's a figure rather than a sweep: a sparse pool reaches further
    while within(hi).len() < 8 && hi < 98 {
        hi += 1;
    }
    let keys = within(hi);
    let keys = if keys.len() < 8 { scale.iter().copied().filter(|k| *k >= 48 && *k <= 84).collect() } else { keys };
    // the shape, as a walk through the notes low to high
    let n = keys.len() as i32;
    let shape = rng.weighted(&[3.0, 2.0, 1.5]);
    let mut path: Vec<i32> = match shape {
        // straight
        0 => (0..n).collect(),
        // three on, one back
        1 => {
            let mut v = Vec::new();
            let mut i = 0;
            while i < n {
                v.push(i);
                if v.len() % 4 == 3 && i > 0 {
                    v.push(i - 1);
                }
                i += 1;
            }
            v
        }
        // in two breaths: a run to the middle, a gap, the rest (below)
        _ => (0..n).collect(),
    };
    let breath = shape == 2 && path.len() > 6;
    if !up {
        path.reverse();
    }
    // the pace: from a tenth of a beat to a quarter (the slowest `beat`), even, quickening
    // or slowing
    let base = beat as f64 / *rng.pick(&[10.0, 8.0, 6.0, 5.0, 4.0]);
    let curve = rng.range(0, 2);
    let double = rng.chance(0.3);
    let (soft, loud) = (rng.range(28, 46) as f64, rng.range(80, 104) as f64 * strength);
    let mut out = Vec::new();
    let mut t = 0.0;
    let last = path.len().max(2) - 1;
    for (j, i) in path.iter().enumerate() {
        let x = j as f64 / last as f64;
        let key = keys[*i as usize];
        // louder towards where it's going: the top going up, the first notes coming down
        let shape = if up { x } else { 1.0 - x };
        let vel = (soft + (loud - soft).max(0.0) * shape).clamp(1.0, 127.0) as u8;
        out.push((t as u32, key, vel));
        if double && key >= 52 {
            out.push((t as u32, key - 12, vel.saturating_sub(14)));
        }
        let pace = match curve {
            0 => 1.0,
            1 => 1.6 - 1.1 * x,
            _ => 0.6 + 1.1 * x,
        };
        t += base * pace;
        if breath && j == path.len() / 2 {
            t += beat as f64 * 0.75;
        }
    }
    out
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
        (Style::Overcity, Role::Brass) => &["Skyline Swell", "Tidal Brass", "Neon Brass", "Velvet Horns"],
        (Style::Overcity, Role::Pad | Role::Strings) => {
            &["Night Choir", "Cathedral Sweep", "Vox Haze", "Synth Choir (FreePats)", "Halo Pad"]
        }
        (Style::Overcity, Role::Lead) => &["Glass Lead", "Ribbon Lead"],
        (Style::Overcity, Role::Answer) => &["Choir", "Glass Lead", "Shakuhachi"],
        (Style::Overcity, Role::Bass) => &["Pure Sub", "Sub Pulse Bass"],
        (Style::Overcity, Role::Bells) => &["Glass Chimes", "Night Bells", "Tubular Bells"],
        (Style::Overcity, Role::Pluck) => &["Rain Harp", "Lantern Pluck"],
        (Style::Overcity, Role::Drone) => &["Abyss Drone", "Choir Drone", "Glass Drone"],
        (Style::Derelict, Role::Brass) => &["French Horns", "Trombone", "Velvet Horns"],
        (Style::Derelict, Role::Pad | Role::Strings) => &["Soundtrack", "Metal Pad", "Space Voice", "Orbital Haze", "Cathedral Sweep"],
        (Style::Derelict, Role::Lead) => &["Cor Anglais", "Solo Oboe", "Bassoon"],
        (Style::Derelict, Role::Answer) => &["Muted Trumpet", "Tenor Sax (FreePats)", "Baritone Sax"],
        (Style::Derelict, Role::Bass) => &["Pure Sub", "Sub Pulse Bass", "Undercity Bass"],
        (Style::Derelict, Role::Bells) => &["Celesta", "Celesta Glow", "Tubular Bells", "Ring Bells"],
        (Style::Derelict, Role::Pluck) => &["Harp", "Pizzicato Strings"],
        (Style::Derelict, Role::Drone) => &["Abyss Drone", "Ozone Drone", "Sodium Drone"],
        (Style::Homeward, Role::Pad) => &["Synth Strings (FreePats)", "Warm Pad"],
        (Style::Homeward, Role::Lead) => &["Flute", "Clarinet", "French Horn"],
        (Style::Homeward, Role::Bass) => &["Pure Sub"],
        (Style::Homeward, Role::Pluck) => &["Concert Harp (FreePats)", "Harp"],
        (Style::Homeward, Role::Keys) => &["Grand Piano", "Electric Piano"],
        (Style::Return, Role::Pad) => &["Synth Strings (FreePats)"],
        (Style::Return, Role::Lead) => &["Clarinet"],
        (Style::Return, Role::Bass) => &["Pure Sub"],
        (Style::Return, Role::Pluck) => &["Concert Harp (FreePats)"],
        (Style::Return, Role::Keys) => &["Grand Piano"],
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

/// Write section `i` again with new notes: the piece's style, key, mode and density, a
/// new seed. The rest of the piece is left as it is. Needs a composed piece whose
/// sections are all parts; returns whether it could.
pub fn regenerate_section(p: &mut Project, i: usize, seed: u64) -> bool {
    let (Some(r), Some(plan)) = (p.recipe, p.plan()) else { return false };
    if i >= plan.len() {
        return false;
    }
    let fresh = compose_plan(&Recipe { seed, ..r }, p.bpm, p.meter, &plan);
    replace_with(p, i, &fresh, seed);
    let mut blocks = p.arrangement();
    blocks[i].seed = (seed != r.seed).then_some(seed);
    p.set_arrangement(&blocks);
    true
}

/// Section `i`'s notes from `fresh` (the same arrangement, written with `seed`): the
/// piece's own tracks from theirs, and each added track from the track of its role.
fn replace_with(p: &mut Project, i: usize, fresh: &Project, seed: u64) {
    let Some(&(a, b)) = p.block_ticks().get(i) else { return };
    let added: Vec<Option<Track>> = p
        .tracks
        .iter()
        .map(|t| if t.added { t.role.and_then(|role| role_track(p, role, added_seed(p, &t.name, role, seed))) } else { None })
        .collect();
    let same_count = p.tracks.len() == fresh.tracks.len();
    for (j, t) in p.tracks.iter_mut().enumerate() {
        let src = if t.added {
            added[j].as_ref()
        } else {
            fresh.tracks.iter().find(|f| f.name == t.name).or_else(|| same_count.then(|| &fresh.tracks[j]))
        };
        t.replace_span(a, b, src);
    }
}

/// Write sections `which` again for their parts and lengths, each with its own seed
/// (else the piece's), leaving the rest as they are. Needs a composed piece whose
/// sections are all parts; returns whether it could.
pub fn rewrite_sections(p: &mut Project, which: &[usize]) -> bool {
    let (Some(r), Some(plan)) = (p.recipe, p.plan()) else { return false };
    let blocks = p.arrangement();
    let mut seeds: Vec<u64> = which.iter().filter_map(|&i| blocks.get(i)).map(|b| b.seed.unwrap_or(r.seed)).collect();
    seeds.sort_unstable();
    seeds.dedup();
    for seed in seeds {
        let fresh = compose_plan(&Recipe { seed, ..r }, p.bpm, p.meter, &plan);
        for &i in which.iter().filter(|&&i| blocks.get(i).is_some_and(|b| b.seed.unwrap_or(r.seed) == seed)) {
            replace_with(p, i, &fresh, seed);
        }
    }
    true
}

/// New notes for one track in section `i` only, with a new seed: the piece's style,
/// key, mode and density. Needs a composed piece whose sections are all parts, and a
/// track the style writes; returns whether it could.
pub fn regenerate_track_section(p: &mut Project, i: usize, track: usize, seed: u64) -> bool {
    let (Some(r), Some(plan)) = (p.recipe, p.plan()) else { return false };
    let (Some(&(a, b)), Some(t)) = (p.block_ticks().get(i), p.tracks.get(track)) else { return false };
    let src = if t.added {
        t.role.and_then(|role| role_track(p, role, seed).or_else(|| Some(filled_track(p, role, seed, track))))
    } else {
        let fresh = compose_plan(&Recipe { seed, ..r }, p.bpm, p.meter, &plan);
        fresh.tracks.iter().find(|f| f.name == t.name).cloned().or_else(|| t.role.and_then(|role| role_track(p, role, seed)))
    };
    let Some(src) = src else { return false };
    p.tracks[track].replace_span(a, b, Some(&src));
    true
}

/// The track the piece's style writes for `role` over the piece's sections: written
/// with `seed`, or (when the style leaves the role out with that seed) the next seed
/// that has it. None if the piece isn't composed or the style never writes the role.
pub fn role_track(p: &Project, role: Role, seed: u64) -> Option<Track> {
    let (r, plan) = (p.recipe?, p.plan()?);
    (0..16u64).find_map(|k| {
        let fresh = compose_plan(&Recipe { seed: seed.wrapping_add(k), ..r }, p.bpm, p.meter, &plan);
        fresh.tracks.into_iter().find(|t| t.role == Some(role))
    })
}

/// The seed an added track named `name` is written with where the piece is written
/// with `seed`: the same, so it plays what the style writes for its role, unless the
/// piece has a track of that role of its own; then one of the track's own, so it plays
/// something else.
fn added_seed(p: &Project, name: &str, role: Role, seed: u64) -> u64 {
    if p.tracks.iter().any(|t| !t.added && t.role == Some(role)) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in name.bytes() {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        seed ^ h
    } else {
        seed
    }
}

/// Add a track of `role` with the notes and sound the piece's style writes for it (a
/// second track of a role it has already is written with a seed of its own, so it
/// plays something else). A role the style doesn't write, or a piece that wasn't
/// composed, gets the role's usual sound and a part written over the piece's chords
/// (`fill.rs`), so a new track always has notes. Returns the new track's index and
/// whether the style wrote it. A piece that was untouched stays so. It is called `name`,
/// else after its role (numbered if the piece has one already).
pub fn add_track(p: &mut Project, role: Role, name: Option<&str>) -> (usize, bool) {
    let untouched = p.untouched();
    let mut name = name.map_or_else(|| role.label().to_string(), str::to_string);
    let mut k = 2;
    while p.tracks.iter().any(|t| t.name == name) {
        name = format!("{} {k}", role.label());
        k += 1;
    }
    let index = p.tracks.len();
    let seed = added_seed(p, &name, role, p.recipe.map_or(0, |r| r.seed));
    let written = role_track(p, role, seed);
    let by_style = written.is_some();
    let mut t = written.unwrap_or_else(|| filled_track(p, role, seed, index));
    let bars = p.length_bars.max(p.bars());
    let end = bars * p.bar_ticks();
    t.notes.retain(|n| n.start < end);
    for x in t.notes.iter_mut() {
        x.len = x.len.min(end - x.start);
    }
    t.ctrls.retain(|c| c.tick < end);
    t.name = name;
    t.color = crate::project::NEON[index % crate::project::NEON.len()];
    t.levels.clear();
    t.added = true;
    p.tracks.push(t);
    if untouched {
        p.composed = fingerprint(p);
    }
    (index, by_style)
}

/// A track of `role` in its usual sound, playing a part written over the piece's
/// harmony as it stands (for a role the style doesn't write, or a piece not composed).
fn filled_track(p: &Project, role: Role, seed: u64, index: usize) -> Track {
    let alt = alternatives(role).into_iter().find(|a| a.bundled).unwrap_or_else(|| alternatives(role).remove(0));
    let mut t = Track::new(role.label(), alt.source.clone(), index);
    crate::palette::apply(&mut t, role, &alt);
    (t.notes, t.ctrls) = crate::fill::write(p, role, seed);
    t
}

/// Give a freshly composed piece the tracks added to the one it replaces, each
/// written again for its role (keeping its name, and its sound, mix and colour if
/// `keep`).
pub fn keep_added(new: &mut Project, old: &Project, keep: bool) {
    for o in old.tracks.iter().filter(|t| t.added) {
        let Some(role) = o.role else { continue };
        let (i, _) = add_track(new, role, Some(&o.name));
        let t = &mut new.tracks[i];
        t.levels = o.levels.clone();
        if keep {
            t.source = o.source.clone();
            t.strip = o.strip;
            t.color = o.color;
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
        for style in
            [Style::Pursuit, Style::Blackout, Style::Momento, Style::Noir, Style::Overcity, Style::Derelict, Style::Homeward, Style::Return]
        {
            for seed in 0..15u64 {
                let r = Recipe { style, ..Recipe::random(seed) };
                for (bpm, meter, bars) in
                    [(56.0, (4, 4), 8), (104.0, (4, 4), 24), (90.0, (3, 4), 40), (128.0, (6, 8), 17), (75.0, (5, 4), 3)]
                {
                    let p = compose(&r, bpm, meter, bars);
                    assert_eq!(p.bars(), bars.max(2), "{style:?} seed {seed} {bpm} {meter:?} {bars}");
                    for t in &p.tracks {
                        assert!(t.notes.iter().all(|n| n.end() <= p.end_tick() && n.len > 0 && n.key < 128), "{}", t.name);
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
    /// The notes starting in [a, b), moved to start from 0.
    fn notes_in(p: &Project, a: u32, b: u32) -> Vec<Vec<Note>> {
        p.tracks
            .iter()
            .map(|t| t.notes.iter().filter(|n| n.start >= a && n.start < b).map(|n| Note { start: n.start - a, ..*n }).collect())
            .collect()
    }

    #[test]
    fn sections_copy_delete_and_write_again() {
        let r = Recipe { style: Style::Noir, ..Recipe::default() };
        let p = compose_plan(&r, 64.0, (4, 4), &[(Part::Neon, 4), (Part::Swell, 4), (Part::Fade, 4)]);
        let bar = p.bar_ticks();
        let len = 4 * bar;
        let (swell, fade) = (notes_in(&p, len, 2 * len), notes_in(&p, 2 * len, 3 * len));

        // copied after: the Swell twice, the Fade moved on
        let mut c = p.clone();
        c.copy_section(1, true);
        let parts: Vec<Option<Part>> = c.arrangement().iter().map(|b| b.part).collect();
        assert_eq!(parts, [Some(Part::Neon), Some(Part::Swell), Some(Part::Swell), Some(Part::Fade)]);
        assert_eq!(c.length_bars, 16);
        // notes held past the section's end are cut at it in the copy
        let clip = |v: Vec<Vec<Note>>| -> Vec<Vec<Note>> {
            v.into_iter().map(|t| t.into_iter().map(|n| Note { len: n.len.min(len - n.start), ..n }).collect()).collect()
        };
        assert_eq!(clip(notes_in(&c, 2 * len, 3 * len)), clip(swell.clone()));
        assert_eq!(notes_in(&c, 3 * len, 4 * len), fade);

        // copied before: the same, the copy first
        let mut c = p.clone();
        c.copy_section(1, false);
        assert_eq!(c.length_bars, 16);
        assert_eq!(clip(notes_in(&c, len, 2 * len)), clip(swell.clone()));
        assert_eq!(notes_in(&c, 3 * len, 4 * len), fade);

        // deleted: the Fade moves back, and nothing sounds past the end
        let mut d = p.clone();
        d.delete_section(1);
        assert_eq!(d.arrangement().len(), 2);
        assert_eq!(d.length_bars, 8);
        assert_eq!(notes_in(&d, len, 2 * len), fade);
        assert!(d.tracks.iter().all(|t| t.notes.iter().all(|n| n.end() <= 8 * bar + len)));
        let mut one = compose_plan(&r, 64.0, (4, 4), &[(Part::Neon, 4)]);
        one.recipe = Some(r);
        one.delete_section(0);
        assert_eq!(one.arrangement().len(), 1, "the last section stays");

        // written again: new notes in the Swell, the rest as they were
        let mut g = p.clone();
        g.recipe = Some(r);
        assert!(regenerate_section(&mut g, 1, r.seed + 1));
        assert_ne!(notes_in(&g, len, 2 * len), swell);
        assert_eq!(notes_in(&g, 2 * len, 3 * len), fade);
        let before: Vec<Vec<u32>> = p.tracks.iter().map(|t| t.notes.iter().filter(|n| n.start < len).map(|n| n.start).collect()).collect();
        let after: Vec<Vec<u32>> = g.tracks.iter().map(|t| t.notes.iter().filter(|n| n.start < len).map(|n| n.start).collect()).collect();
        assert_eq!(before, after);
        assert!(g.tracks.iter().all(|t| t.notes.iter().all(|n| n.start >= len || n.end() <= len || n.start >= 2 * len)));
    }

    #[test]
    fn ripples_play_every_note_once() {
        for count in 1..=8 {
            let scatter: Vec<usize> = (0..count).rev().collect();
            for shape in 0..=6 {
                let mut o = ripple_order(shape, count, &scatter);
                assert_eq!(o.len(), count, "shape {shape}, {count} notes");
                o.sort_unstable();
                assert_eq!(o, (0..count).collect::<Vec<_>>(), "shape {shape}, {count} notes");
            }
        }
        // and different seeds shape them differently: the first figure's notes, from its lowest
        let figures: std::collections::BTreeSet<Vec<i32>> = (1..12u64)
            .map(|seed| {
                let r = Recipe { seed, style: Style::Overcity, ..Recipe::default() };
                let p = compose(&r, 60.0, (4, 4), 52);
                let notes: Vec<i32> = p
                    .tracks
                    .iter()
                    .find(|t| t.role == Some(Role::Pluck))
                    .map_or(Vec::new(), |t| t.notes.iter().take(6).map(|n| n.key as i32).collect());
                let low = notes.iter().copied().min().unwrap_or(0);
                notes.iter().map(|k| k - low).collect()
            })
            .collect();
        assert!(figures.len() >= 5, "{figures:?}");
    }

    /// Overcity: a long breath before the theme, the theme leaping and falling, horn
    /// swells written as expression, drums only in the great swell, the city far below,
    /// and a last bell as it fades.
    #[test]
    fn overcity_breathes_then_sings() {
        let r = Recipe { style: Style::Overcity, ..Recipe::default() };
        let p = compose(&r, 60.0, (4, 4), 52);
        let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
        assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
        let blocks = p.block_ticks();
        let track = |role: Role| p.tracks.iter().find(|t| t.role == Some(role)).unwrap_or_else(|| panic!("no {role:?}"));
        let lead = track(Role::Lead);
        assert!(lead.notes.iter().all(|n| n.start >= blocks[1].0), "the intro is the horns alone");
        // the first phrase: a rising leap of a fifth or more, then falling
        let first: Vec<i32> = lead.notes.iter().take(5).map(|n| n.key as i32).collect();
        assert!(first[1] - first[0] >= 7, "{first:?}");
        assert!(first[2..].windows(2).all(|w| w[1] <= w[0]), "{first:?}");
        assert!(track(Role::Brass).ctrls.iter().any(|c| c.kind == CtrlKind::Cc(11)));
        let (ns_a, ns_z) = blocks[3];
        assert!(track(Role::Drums).notes.iter().all(|n| n.start >= ns_a && n.start < ns_z));
        assert!(!track(Role::City).notes.is_empty());
        let end = p.length_bars * p.bar_ticks();
        assert!(track(Role::Bells).notes.iter().any(|n| n.start >= end - 4 * p.bar_ticks()));
        // no electric piano, no answer: the theme sings alone
        assert!(p.tracks.iter().all(|t| !matches!(t.role, Some(Role::Keys | Role::Answer))));
    }

    /// Derelict: a void first, no beat and no rain, the ship's machinery throughout,
    /// the orchestra surfacing, one brief eruption, a long collapse; the sounds added for
    /// it never chosen by the other styles.
    #[test]
    fn derelict_drifts_erupts_and_collapses() {
        for seed in 0..12 {
            let r = Recipe { seed, style: Style::Derelict, ..Recipe::random(seed) };
            for (bars, meter) in [(40, (4, 4)), (24, (4, 4)), (64, (3, 4))] {
                let p = compose(&r, Style::Derelict.tempo(), meter, bars);
                let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
                assert_eq!(
                    parts,
                    [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)],
                    "seed {seed} {bars}"
                );
                let blocks = p.block_ticks();
                let (ns, fade) = (blocks[3], blocks[4]);
                assert!(ns.1 - ns.0 < fade.1 - fade.0, "a brief eruption, a long collapse");
                let track = |name: &str| p.tracks.iter().find(|t| t.name == name).unwrap_or_else(|| panic!("no {name} in seed {seed}"));
                // no beat, no rain
                assert!(p.tracks.iter().all(|t| t.role != Some(Role::Drums)), "seed {seed}");
                let city = track("City");
                assert!(city.notes.iter().all(|n| RainKind::of_key(n.key).is_none() && n.key != fx::THUNDER));
                assert!(city.notes.iter().any(|n| n.key == fx::TAPE_HISS && n.start == 0 && n.end() == p.end_tick()));
                assert!(city.notes.iter().any(|n| n.key == fx::TICKING));
                // the brass a single instrument: no chorus anywhere, no echo on the brass
                assert!(track("Brass").strip.echo == 0.0 && p.fx.ensemble == 0.0);
                assert!(p.tracks.iter().all(|t| !t.strip.ensemble));
                // expression and brightness move in small steps, never a jump that stabs
                for t in &p.tracks {
                    for kind in [CtrlKind::Cc(11), CtrlKind::Cc(74)] {
                        let cs: Vec<&Ctrl> = t.ctrls.iter().filter(|c| c.kind == kind).collect();
                        for w in cs.windows(2) {
                            let quick = p.tick_secs(w[1].tick - w[0].tick.min(w[1].tick)) < 0.2;
                            assert!(
                                !quick || (w[1].value - w[0].value).abs() <= 12,
                                "seed {seed} {}: {kind:?} {} -> {}",
                                t.name,
                                w[0].value,
                                w[1].value
                            );
                        }
                    }
                }
                // the ship's engines, not the city's mains hum
                assert!(city.notes.iter().any(|n| n.key == fx::ENGINE) && city.notes.iter().all(|n| n.key != fx::HUM));
                // only the sounds written for it: no koto, no shakuhachi
                for t in &p.tracks {
                    let role = t.role.expect("a role");
                    if let Some(a) = alternatives(role).into_iter().find(|a| same_sound(&a.source, &t.source)) {
                        assert!(
                            prefers(Style::Derelict, role, a.label)
                                || role == Role::Strings && a.label == "Tremolo Strings"
                                || a.label == "Contrabass",
                            "seed {seed}: {} on {}",
                            a.label,
                            t.name
                        );
                    }
                }
                assert!(city.notes.iter().any(|n| (fx::HULL..fx::HULL + 12).contains(&n.key)));
                // the void: the drone and the ship alone
                for name in ["Pad", "Brass", "Lead", "Strings", "Double Bass", "Bass"] {
                    assert!(track(name).notes.iter().all(|n| n.start >= blocks[1].0), "{name} in the void, seed {seed}");
                }
                // the orchestra
                assert!(matches!(&track("Strings").source, Source::SoundFont { program: 44, .. }), "tremolo strings");
                assert!(matches!(&track("Double Bass").source, Source::SoundFont { program: 43, .. }), "contrabass");
                // very little melody, and the horns at full cry only in the eruption
                assert!(track("Lead").notes.len() <= 10, "seed {seed}: {}", track("Lead").notes.len());
                assert!(track("Brass").notes.iter().filter(|n| n.vel >= 100).all(|n| n.start >= ns.0 && n.start < ns.1));
                assert!(track("Strings").notes.iter().any(|n| n.start >= ns.0 && n.start < ns.1 && n.vel >= 100));
                // the glissando into the eruption ends on it
                let into: Vec<&Note> = track("Pluck").notes.iter().filter(|n| n.start < ns.0 && n.start + p.bar_ticks() > ns.0).collect();
                assert!(into.len() >= 4 && into.iter().all(|n| n.start < ns.0), "seed {seed} {bars}");
            }
        }
        // each seed sweeps the harp its own way
        let mut runs = std::collections::BTreeSet::new();
        for seed in 0..16 {
            let p = compose(&Recipe { seed, root: 2, mode: Mode::Aeolian, density: 0.5, style: Style::Derelict }, 48.0, (4, 4), 40);
            let pluck = p.tracks.iter().find(|t| t.name == "Pluck").unwrap();
            let first = pluck.notes.first().map_or(0, |n| n.start);
            runs.insert(pluck.notes.iter().map(|n| (n.start - first, n.key)).collect::<Vec<_>>());
        }
        assert!(runs.len() >= 15, "only {} different glissandi in 16 seeds", runs.len());
        // (Homeward and Chamber, written after, share its contrabass and pizzicato on purpose)
        for style in Style::ALL
            .into_iter()
            .filter(|s| !matches!(s, Style::Derelict | Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula))
        {
            for seed in 0..60 {
                let p = compose(&Recipe { seed, style, ..Recipe::random(seed) }, style.tempo(), (4, 4), 32);
                for t in &p.tracks {
                    let role = t.role.expect("a role");
                    let label = alternatives(role).into_iter().find(|a| same_sound(&a.source, &t.source)).map(|a| a.label);
                    assert!(label.is_none_or(|l| !DERELICT_SOUNDS.contains(&l)), "{style:?} {seed}: {label:?}");
                }
            }
        }
    }

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
        let old = Section { bar: 0, name: "Recede".into(), part: None, seed: None };
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

#[cfg(test)]
mod section_tests {
    use super::*;
    use crate::project::{Block, PPQ, block_labels};

    fn edited(style: Style, seed: u64) -> Project {
        let r = Recipe { style, ..Recipe::random(seed) };
        let mut p = compose(&r, 72.0, (4, 4), 32);
        // a note moved by hand: the piece counts as edited from now on
        let t = p.tracks.iter_mut().find(|t| t.role == Some(Role::Bass)).expect("bass");
        t.notes[0].vel = t.notes[0].vel.saturating_sub(1).max(1);
        assert!(!p.untouched());
        p
    }

    /// What a dragged section's end does to an edited composed piece (as the app does it).
    fn drag_end(p: &mut Project, i: usize, bars: u32) {
        let mut blocks = p.arrangement();
        blocks[i].bars = bars;
        let origin: Vec<Option<usize>> = (0..blocks.len()).map(Some).collect();
        p.rearrange_with_notes(&blocks, &origin);
        assert!(rewrite_sections(p, &[i]));
    }

    fn notes_in(t: &Track, a: u32, b: u32) -> usize {
        t.notes.iter().filter(|n| n.start >= a && n.start < b).count()
    }

    /// Made longer, a section of an edited piece is written again to its new length:
    /// no empty bars in it, the sections after it keep their notes (moved with them),
    /// and the rain still falls to the end.
    #[test]
    fn a_longer_section_is_written_again_and_the_rest_move_with_it() {
        for style in [Style::Nocturne, Style::Overcity, Style::Derelict] {
            for seed in 0..4 {
                let mut p = edited(style, seed);
                let before = p.clone();
                let old = p.block_ticks();
                let i = 1;
                let grow = 6;
                let bars = p.arrangement()[i].bars + grow;
                drag_end(&mut p, i, bars);
                let shift = grow * p.bar_ticks();
                let now = p.block_ticks();
                assert_eq!(p.length_bars, before.length_bars + grow, "{style:?} {seed}");
                // every track playing in the section plays in its new bars too
                let (a, b) = now[i];
                for t in &p.tracks {
                    let was = before.tracks.iter().find(|o| o.name == t.name).unwrap();
                    if notes_in(was, old[i].0, old[i].1) > 2 && t.role != Some(Role::City) {
                        let tail = b - shift.min(b - a) / 2;
                        assert!(notes_in(t, a, b) > 0, "{style:?} {seed} {}: empty", t.name);
                        assert!(t.notes.iter().any(|n| n.end() > tail), "{style:?} {seed} {}: silent at the end", t.name);
                    }
                }
                // the later sections are as they were, moved along
                for j in i + 1..old.len() {
                    for (t, o) in p.tracks.iter().zip(&before.tracks) {
                        let moved: Vec<(u32, u8)> = o
                            .notes
                            .iter()
                            .filter(|n| n.start >= old[j].0 && n.start < old[j].1)
                            .map(|n| (n.start + shift, n.key))
                            .collect();
                        let here: Vec<(u32, u8)> =
                            t.notes.iter().filter(|n| n.start >= now[j].0 && n.start < now[j].1).map(|n| (n.start, n.key)).collect();
                        assert_eq!(moved, here, "{style:?} {seed} {} in section {j}", t.name);
                    }
                }
                let rains = |p: &Project| {
                    p.tracks
                        .iter()
                        .filter(|t| t.role == Some(Role::City))
                        .any(|t| t.notes.iter().any(|n| RainKind::of_key(n.key).is_some() && n.start == 0 && n.end() == p.end_tick()))
                };
                assert_eq!(rains(&p), rains(&before), "{style:?} {seed}");
            }
        }
    }

    /// New notes for a section keep the rain and anything else held through it.
    #[test]
    fn new_notes_for_a_section_keep_what_is_held_through_it() {
        let mut p = edited(Style::Nocturne, 3);
        assert!(regenerate_section(&mut p, 2, 777));
        let city = p.tracks.iter().find(|t| t.role == Some(Role::City)).unwrap();
        assert!(city.notes.iter().any(|n| RainKind::of_key(n.key).is_some() && n.start == 0 && n.end() == p.end_tick()));
        assert_eq!(p.arrangement()[2].seed, Some(777));
        // made longer, it is written again with its own seed
        let a = p.block_ticks()[2].0;
        let bars = p.arrangement()[2].bars;
        drag_end(&mut p, 2, bars + 2);
        assert_eq!(p.arrangement()[2].seed, Some(777));
        let mut q = p.clone();
        regenerate_section(&mut q, 2, 777);
        let lead = |p: &Project| p.tracks.iter().find(|t| t.name == "Lead").map(|t| t.notes.iter().filter(|n| n.start >= a).count());
        assert_eq!(lead(&p), lead(&q));
    }

    /// Copying or taking out a section keeps the rain falling to the end.
    #[test]
    fn copies_and_deletions_keep_the_rain() {
        let rains = |p: &Project| {
            p.tracks.iter().any(|t| t.notes.iter().any(|n| RainKind::of_key(n.key).is_some() && n.start == 0 && n.end() == p.end_tick()))
        };
        let mut p = edited(Style::Nocturne, 6);
        assert!(rains(&p));
        p.copy_section(1, true);
        assert!(rains(&p));
        p.delete_section(2);
        assert!(rains(&p));
        p.delete_section(0);
        assert!(p.tracks.iter().any(|t| t.notes.iter().any(|n| RainKind::of_key(n.key).is_some() && n.end() == p.end_tick())));
    }

    /// Moving sections in an edited piece takes their notes with them.
    #[test]
    fn moved_sections_take_their_notes() {
        let mut p = edited(Style::Pursuit, 1);
        let before = p.clone();
        let old = p.block_ticks();
        let mut blocks = p.arrangement();
        let origin = vec![Some(1), Some(0), Some(2), Some(3)][..blocks.len()].to_vec();
        blocks.swap(0, 1);
        p.rearrange_with_notes(&blocks, &origin);
        let now = p.block_ticks();
        for (t, o) in p.tracks.iter().zip(&before.tracks) {
            let shape = |t: &Track, (a, b): (u32, u32)| -> Vec<(u32, u8)> {
                t.notes.iter().filter(|n| n.start >= a && n.start < b).map(|n| (n.start - a, n.key)).collect()
            };
            assert_eq!(shape(t, now[0]), shape(o, old[1]), "{}", t.name);
            assert_eq!(shape(t, now[1]), shape(o, old[0]), "{}", t.name);
        }
    }

    /// A hand-made piece's longer section plays its notes again to fill the new bars.
    #[test]
    fn a_longer_hand_made_section_repeats() {
        let mut p = Project { length_bars: 4, ..Project::default() };
        p.sections = vec![Section::new(0, Part::Neon), Section::new(2, Part::Fade)];
        let bar = p.bar_ticks();
        let mut t = Track::new("Keys", Source::Synth(dd_dsp::patch::by_name("Rain Pad")), 0);
        t.notes = vec![
            Note { start: 0, len: PPQ, key: 60, vel: 90 },
            Note { start: bar, len: PPQ, key: 62, vel: 90 },
            Note { start: 2 * bar, len: PPQ, key: 64, vel: 90 },
        ];
        p.tracks.push(t);
        let mut blocks = p.arrangement();
        blocks[0].bars = 5;
        p.rearrange_with_notes(&blocks, &[Some(0), Some(1)]);
        let starts: Vec<(u32, u8)> = p.tracks[0].notes.iter().map(|n| (n.start / bar, n.key)).collect();
        assert_eq!(starts, vec![(0, 60), (1, 62), (2, 60), (3, 62), (4, 60), (5, 64)]);
        assert_eq!(p.length_bars, 7);
    }

    /// Sections are numbered within their part unless given a name; numbered names read
    /// back as their parts.
    #[test]
    fn sections_are_numbered_within_their_part() {
        let mut blocks = vec![Block::new(Part::Intro, 2), Block::new(Part::Neon, 4), Block::new(Part::Intro, 2), Block::new(Part::Neon, 4)];
        blocks[3].name = "Rooftop".into();
        assert_eq!(block_labels(&blocks), ["Intro 1", "Neon 1", "Intro 2", "Rooftop"]);
        assert_eq!(Part::from_name("Intro 2"), Some(Part::Intro));
        assert_eq!(Part::from_name("Night shift 12"), Some(Part::NightShift));
        blocks[0].name = "Intro 7".into();
        assert_eq!(blocks[0].own_name(), None);
    }

    /// A new track plays what the style writes for its role; a second of a role plays
    /// something of its own; a role the style doesn't write is written over its chords. Adding to an
    /// untouched piece leaves it untouched, and a new piece writes added tracks again.
    #[test]
    fn added_tracks_are_written_by_the_style() {
        let r = Recipe { style: Style::Derelict, ..Recipe::random(5) };
        let mut p = compose(&r, 48.0, (4, 4), 32);
        let pad = p.tracks.iter().find(|t| t.role == Some(Role::Pad)).unwrap().clone();
        let (i, notes) = add_track(&mut p, Role::Pad, None);
        assert!(notes && p.tracks[i].added && p.tracks[i].name == "Pad 2");
        assert_ne!(p.tracks[i].notes, pad.notes);
        assert!(p.untouched());
        // (Derelict writes no drums: a part is written over its chords instead)
        let (j, by_style) = add_track(&mut p, Role::Drums, None);
        assert!(!by_style && !p.tracks[j].notes.is_empty());
        // Keys: Derelict writes none, Nocturne does
        let mut q = compose(&Recipe { style: Style::Nocturne, ..r }, 70.0, (4, 4), 32);
        let had = q.tracks.iter().any(|t| t.role == Some(Role::Bells));
        let (k, notes) = add_track(&mut q, Role::Bells, None);
        assert!(notes, "bells");
        if !had {
            assert_eq!(q.tracks[k].name, "Bells");
        }
        let mut fresh = compose(&Recipe { seed: r.seed + 1, ..r }, 48.0, (4, 4), 32);
        keep_added(&mut fresh, &p, true);
        assert!(fresh.tracks.iter().any(|t| t.name == "Pad 2" && t.added && !t.notes.is_empty()));
        assert!(fresh.untouched());
    }

    /// New notes for one track in one section change only those.
    #[test]
    fn new_notes_for_one_track_in_one_section() {
        let mut p = edited(Style::Overcity, 2);
        let before = p.clone();
        let k = p.tracks.iter().position(|t| t.role == Some(Role::Lead)).unwrap();
        let (a, b) = p.block_ticks()[2];
        assert!(regenerate_track_section(&mut p, 2, k, 4242));
        for (j, (t, o)) in p.tracks.iter().zip(&before.tracks).enumerate() {
            let outside = |t: &Track| t.notes.iter().filter(|n| n.start < a || n.start >= b).map(|n| (n.start, n.key)).collect::<Vec<_>>();
            assert_eq!(outside(t), outside(o), "{}", t.name);
            if j != k {
                assert_eq!(t.notes, o.notes, "{}", t.name);
            }
        }
        assert_ne!(p.tracks[k].notes, before.tracks[k].notes);
    }
}
