//! Homeward: a slow, warm, quietly hopeful score built on one singable eight-bar theme
//! in a minor key (D23). The seed writes the theme: its rhythms, a first phrase that
//! rises and comes to rest on the dominant, and a second that lifts towards the relative
//! major, peaks and falls home to the tonic. Its harmony follows it, a chord a bar, with
//! a raised leading tone at the cadences.
//!
//! The sections tell its story, each statement ending on a cadence: the Intro has it on
//! the piano alone; the Neon passes it to the cellos (or violas) over soft piano chords,
//! a harp, a bowed double bass and a quiet sub; the Swell reharmonises it for a flute,
//! clarinet or horn with a countermelody in the cellos; the Night shift builds a pulse
//! under layered strings and a warm analogue pad to one peak, the theme in full strings
//! in octaves; and the Fade brings it back on the piano as it began, and settles on a
//! held tonic chord. No drums, no city, nothing sudden.
//!
//! Return (D24) is Homeward with its instruments chosen: the theme opens on a soft
//! electric piano, the violas carry it, a clarinet takes the variation with cellos of
//! their own answering, and the solo piano brings it home. Its lift goes to the relative
//! major (the III chord under the theme's peak where the note allows), and its plan
//! leaves the final chord room to fade.

use crate::compose::{Recipe, near};
use crate::project::{Ctrl, CtrlKind, Note, Part, Project, Source};
use crate::rng::Rng;

/// Homeward's tracks.
pub(crate) struct Tracks {
    pub piano: usize,
    pub cellos: usize,
    pub strings: usize,
    pub double_bass: usize,
    pub sub: usize,
    pub pad: usize,
    pub lead: usize,
    pub harp: usize,
    /// Return's: the electric piano that opens it, and cellos apart from the violas that
    /// carry the theme (the countermelody, the pulse and the peak's low octave).
    pub ep: Option<usize>,
    pub counter: Option<usize>,
}

impl Tracks {
    fn is_return(&self) -> bool {
        self.ep.is_some()
    }
}

/// The usual arrangement: the theme's statements in whole eight bars where the piece is
/// long enough (four or two when shorter), the build to the peak the longest.
pub(crate) fn plan(bars: u32) -> Vec<(Part, u32)> {
    let bars = bars.max(2);
    let unit = if bars >= 40 {
        8
    } else if bars >= 20 {
        4
    } else {
        2
    };
    let snap = |f: f64| ((bars as f64 * f / unit as f64).round() as u32 * unit).min(bars);
    let mut bounds = vec![(Part::Intro, 0)];
    for (part, f) in [(Part::Neon, 0.16), (Part::Swell, 0.32), (Part::NightShift, 0.48), (Part::Fade, 0.8)] {
        let b = snap(f);
        if b > bounds.last().map_or(0, |l| l.1) && b < bars {
            bounds.push((part, b));
        }
    }
    bounds.iter().enumerate().map(|(i, (part, b))| (*part, bounds.get(i + 1).map_or(bars, |n| n.1) - b)).collect()
}

/// Return's arrangement: Intro, Neon and Swell a statement each, the Night shift twice
/// that (the build and the peak), and the Fade half as long again to hold its last chord,
/// shared in proportion to the length (as Homeward's when too short to share).
pub(crate) fn return_plan(bars: u32) -> Vec<(Part, u32)> {
    if bars < 20 {
        return plan(bars);
    }
    let parts = [Part::Intro, Part::Neon, Part::Swell, Part::NightShift, Part::Fade];
    let lens = crate::project::share_bars(&[8, 8, 8, 16, 12], bars);
    parts.into_iter().zip(lens).filter(|(_, n)| *n > 0).collect()
}

/// The mix: intimate and spacious, a natural hall, no chorus, the piano soft as felt.
pub(crate) fn mix(p: &mut Project, t: &Tracks) {
    let viola = matches!(&p.tracks[t.cellos].source, Source::SoundFont { name, .. } if name == "Viola");
    for (ti, name) in [
        (t.piano, "Piano"),
        (t.cellos, if viola { "Violas" } else { "Cellos" }),
        (t.strings, "Strings"),
        (t.double_bass, "Double Bass"),
        (t.sub, "Sub"),
        (t.pad, "Pad"),
        (t.lead, if t.is_return() { "Clarinet" } else { "Lead" }),
        (t.harp, "Harp"),
    ] {
        p.tracks[ti].name = name.into();
    }
    if let (Some(ep), Some(c)) = (t.ep, t.counter) {
        p.tracks[ep].name = "Electric Piano".into();
        p.tracks[c].name = "Cellos".into();
    }
    let set = |p: &mut Project, ti: usize, rev: f32, echo: f32, pan: f32, width: f32| {
        let st = &mut p.tracks[ti].strip;
        (st.reverb, st.echo, st.pan, st.width, st.ensemble) = (rev, echo, pan, width, false);
    };
    set(p, t.piano, 0.45, 0.0, -0.15, 1.2);
    set(p, t.cellos, 0.5, 0.0, -0.3, 1.2);
    set(p, t.strings, 0.6, 0.0, 0.1, 1.6);
    set(p, t.double_bass, 0.35, 0.0, -0.1, 1.0);
    set(p, t.sub, 0.05, 0.0, 0.0, 1.0);
    set(p, t.pad, 0.6, 0.0, 0.0, 1.5);
    set(p, t.lead, 0.5, 0.12, 0.2, 1.0);
    set(p, t.harp, 0.55, 0.1, 0.35, 1.3);
    if let (Some(ep), Some(c)) = (t.ep, t.counter) {
        set(p, ep, 0.45, 0.08, -0.1, 1.3);
        set(p, c, 0.5, 0.0, 0.25, 1.2);
        // the violas a little left, the cellos right, as they sit
        p.tracks[t.cellos].strip.pan = -0.3;
        p.tracks[ep].strip.high_cut = 6000.0;
        p.tracks[c].strip.high_cut = 6000.0;
    }
    // felt: the piano's brightness rounded off
    p.tracks[t.piano].strip.high_cut = 5000.0;
    p.tracks[t.piano].strip.gain_db += 2.0;
    p.tracks[t.pad].strip.high_cut = 7000.0;
    p.tracks[t.pad].strip.gain_db -= 3.0;
    // a rounded low end: the sub quiet, the bass without its scrape
    p.tracks[t.sub].strip.gain_db -= 3.0;
    p.tracks[t.sub].strip.high_cut = 400.0;
    p.tracks[t.double_bass].strip.high_cut = 3000.0;
    p.tracks[t.lead].strip.gain_db -= 1.0;
    p.tracks[t.harp].strip.gain_db += 1.0;
    p.fx.ensemble = 0.0;
    p.fx.shimmer = 0.0;
    p.fx.size = 0.62;
    p.fx.tone = 0.5;
    p.fx.predelay = 0.2;
    p.fx.width = 0.9;
    p.fx.duck = 0.0;
    p.fx.echo.feedback = 0.3;
    p.fx.echo.wow = 0.0;
    p.master_db = 4.0;
}

/// A note of the theme: when in its bar and for how long (in beats), and its scale
/// degree (0 the tonic, 7 the octave above).
#[derive(Clone, Copy, Debug)]
struct TNote {
    at: f64,
    len: f64,
    deg: i32,
}

/// The theme: eight bars of notes, and the chord under each (its root as a degree),
/// plainly and reharmonised.
struct Theme {
    bars: Vec<Vec<TNote>>,
    base: [i32; 8],
    alt: [i32; 8],
}

#[derive(Clone, Copy, PartialEq)]
enum Want {
    Any,
    EndLong,
    StartLong,
}

/// A bar's rhythm in beats, adding up to `n`.
fn rhythm(rng: &mut Rng, n: u32, want: Want) -> Vec<f64> {
    let table: &[&[f64]] = match (n, want) {
        (1, _) => &[&[1.0]],
        (2, Want::Any) => &[&[1.0, 1.0], &[1.5, 0.5], &[2.0]],
        (2, _) => &[&[2.0]],
        (3, Want::Any) => &[&[2.0, 1.0], &[1.0, 1.0, 1.0], &[1.5, 0.5, 1.0], &[3.0]],
        (3, Want::EndLong) => &[&[1.0, 2.0], &[3.0]],
        (3, Want::StartLong) => &[&[2.0, 1.0], &[3.0]],
        (4, Want::Any) => {
            &[&[2.0, 1.0, 1.0], &[1.0, 1.0, 2.0], &[3.0, 1.0], &[2.0, 2.0], &[1.5, 0.5, 2.0], &[1.0, 1.0, 1.0, 1.0], &[2.0, 1.5, 0.5]]
        }
        (4, Want::EndLong) => &[&[1.0, 1.0, 2.0], &[2.0, 2.0], &[1.5, 0.5, 2.0], &[1.0, 3.0]],
        (4, Want::StartLong) => &[&[2.0, 1.0, 1.0], &[3.0, 1.0], &[2.0, 2.0]],
        (6, Want::Any) => &[&[3.0, 3.0], &[2.0, 1.0, 3.0], &[3.0, 2.0, 1.0], &[1.0, 2.0, 3.0]],
        (6, Want::EndLong) => &[&[3.0, 3.0], &[2.0, 1.0, 3.0], &[1.0, 2.0, 3.0]],
        (6, Want::StartLong) => &[&[3.0, 3.0], &[3.0, 2.0, 1.0]],
        _ => {
            // longer bars in groups of four, three and two
            let mut groups = Vec::new();
            let mut rem = n;
            while rem > 4 {
                let k = if rem == 5 || rem == 7 { 3 } else { 4 };
                groups.push(k);
                rem -= k;
            }
            groups.push(rem);
            let last = groups.len() - 1;
            let mut out = Vec::new();
            for (i, g) in groups.into_iter().enumerate() {
                let w = match want {
                    Want::EndLong if i == last => Want::EndLong,
                    Want::StartLong if i == 0 => Want::StartLong,
                    _ => Want::Any,
                };
                out.extend(rhythm(rng, g, w));
            }
            return out;
        }
    };
    let weights: Vec<f64> = (0..table.len()).map(|i| if i == 0 { 3.0 } else { 2.0 }).collect();
    table[rng.weighted(&weights)].to_vec()
}

/// Whether the triad on `root` holds degree `deg`.
fn holds(root: i32, deg: i32) -> bool {
    matches!((deg - root).rem_euclid(7), 0 | 2 | 4)
}

/// The chord for a note, from the bar's preferences, else any that holds it.
fn harmonise(deg: i32, prefs: &[i32], not: Option<i32>) -> Option<i32> {
    prefs.iter().chain(&[0, 5, 3, 4, 2, 6, 1]).copied().find(|&c| holds(c, deg) && Some(c) != not)
}

impl Theme {
    fn new(seed: u64, bpb: u32, relative_lift: bool) -> Theme {
        let mut rng = Rng::stream(seed, "homeward-theme");
        // where each bar starts, and where the first phrase comes to rest
        let s1 = [0, 2, 4][rng.weighted(&[3.0, 2.0, 1.0])];
        let a1 = (s1 + rng.range(1, 2)).min(5);
        let a2 = [4, 5][rng.weighted(&[2.0, 1.0])];
        let a3 = [3, 5, 2][rng.weighted(&[2.0, 1.5, 1.0])];
        let half = [4, 1][rng.weighted(&[3.0, 1.0])];
        // the lift: a leap up towards the relative major, the peak, and the way home
        // (from a close on the second, the octave would be a sixth away: the sixth instead)
        let a4 = if half == 1 { [7, 5][rng.weighted(&[0.0, 1.0])] } else { [7, 5][rng.weighted(&[2.0, 1.0])] };
        let a5 = (a4 + rng.range(1, 2)).min(9);
        // down from the peak to a note of the dominant: its fifth, raised seventh or ninth
        let a6 = [4, 6, 8].into_iter().min_by_key(|d| (d - (a5 - 2)).abs()).unwrap_or(4);
        // home: the tonic below, or the octave when the way down starts high (the leading
        // tone rising to it)
        let home = if a6 >= 6 { 7 } else { 0 };
        let anchors = [s1, a1, a2, a3, a4, a5, a6, home];
        let mut rhythms: Vec<Vec<f64>> = Vec::new();
        for b in 0..8 {
            let r = match b {
                // the second phrase echoes the first's rhythm, so it is remembered
                4 => rhythms[0].clone(),
                5 => rhythms[1].clone(),
                3 => rhythm(&mut rng, bpb, Want::EndLong),
                6 => rhythm(&mut rng, bpb, Want::StartLong),
                7 => vec![bpb as f64],
                _ => rhythm(&mut rng, bpb, Want::Any),
            };
            rhythms.push(r);
        }
        let mut bars = Vec::new();
        for (b, r) in rhythms.iter().enumerate() {
            let from = anchors[b];
            let to = if b == 3 { half } else { anchors.get(b + 1).copied().unwrap_or(0) };
            let k = r.len();
            let mut at = 0.0;
            let mut notes: Vec<TNote> = Vec::new();
            for (j, &len) in r.iter().enumerate() {
                let deg = if b == 3 && j == k - 1 {
                    half
                } else if j == 0 {
                    from
                } else {
                    let x = j as f64 / if b == 3 { (k - 1).max(1) as f64 } else { k as f64 };
                    let d = (from as f64 + (to - from) as f64 * x).round() as i32;
                    // now and then a neighbour, never three of a note running
                    let d = if rng.chance(0.25) { d + if rng.chance(0.5) { 1 } else { -1 } } else { d };
                    let prev = notes.last().map_or(d - 1, |n| n.deg);
                    let prev2 = notes.iter().rev().nth(1).map_or(d - 1, |n| n.deg);
                    if d == prev && d == prev2 { d + 1 } else { d }
                };
                notes.push(TNote { at, len, deg: deg.clamp(-2, 10) });
                at += len;
            }
            bars.push(notes);
        }
        // a chord a bar: on the bar's first note (the first phrase's close on its last)
        // (Return's lift reaches the relative major itself: III under the peak where it can)
        let peak: &[i32] = if relative_lift { &[2, 5, 3, 0] } else { &[5, 3, 0] };
        let prefs: [&[i32]; 8] = [&[0], &[5, 3, 2], &[3, 5, 6], &[4], &[5, 2], peak, &[4], &[0]];
        let mut base = [0; 8];
        let mut alt = [0; 8];
        for b in 0..8 {
            let structural = if b == 3 { bars[b].last().map_or(0, |n| n.deg) } else { bars[b][0].deg };
            base[b] = harmonise(structural, prefs[b], None).unwrap_or(0);
            // reharmonised: another chord that holds the same note, the cadences kept
            alt[b] = if matches!(b, 3 | 6 | 7) {
                base[b]
            } else {
                harmonise(structural, &[5, 3, 1, 2, 6, 0, 4], Some(base[b])).unwrap_or(base[b])
            };
        }
        Theme { bars, base, alt }
    }
}

/// What a bar of a section plays.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Bar {
    /// Bar `tb` of the theme.
    Theme(usize),
    /// Building to the peak, `x` of the way, on the theme's chord for bar `tb` (or the
    /// dominant, `true`, just before the peak).
    Build(usize, f64, bool),
    /// Held on the tonic after the theme's last statement.
    Hold,
}

/// The bars of a section of `n` bars: whole statements of the theme from its start, and
/// what is left over its last bars, so the section ends on its cadence (one too short
/// for a statement that opens the piece, or is an Intro, opens the theme instead). The
/// Night shift builds first and peaks on the theme; the Fade holds the tonic after it.
fn layout(part: Part, n: u32, opening: bool) -> Vec<Bar> {
    let statements = |n: u32, from_start: bool| -> Vec<Bar> {
        let mut out = Vec::new();
        for _ in 0..n / 8 {
            out.extend((0..8).map(Bar::Theme));
        }
        let rem = (n % 8) as usize;
        let first = if from_start && n < 8 { 0 } else { 8 - rem };
        out.extend((first..first + rem).map(Bar::Theme));
        out
    };
    match part {
        Part::NightShift => {
            let peak = if n >= 12 { 8 } else { n.div_ceil(2).max(1) };
            let build = n - peak;
            let mut out: Vec<Bar> =
                (0..build).map(|k| Bar::Build(k as usize % 8, k as f64 / build.max(1) as f64, k + 1 == build)).collect();
            out.extend(statements(peak, false));
            out
        }
        Part::Fade if n >= 9 => {
            let mut out: Vec<Bar> = (0..8).map(Bar::Theme).collect();
            out.extend((8..n).map(|_| Bar::Hold));
            out
        }
        _ if part == Part::Intro || opening => statements(n, true),
        _ => statements(n, false),
    }
}

/// How the theme rises and falls over its eight bars, 0 to 1.
const SHAPE: [f64; 8] = [0.0, 0.15, 0.35, 0.25, 0.5, 1.0, 0.6, 0.2];

struct Writer {
    notes: Vec<Vec<Note>>,
    ctrls: Vec<Vec<Ctrl>>,
    end: u32,
    beat: u32,
    touch: Rng,
}

impl Writer {
    fn note(&mut self, track: usize, start: u32, len: u32, key: i32, vel: f64) {
        if start >= self.end || len == 0 {
            return;
        }
        let vel = (vel + self.touch.range(-3, 3) as f64).round().clamp(1.0, 127.0) as u8;
        let key = key.clamp(12, 120) as u8;
        self.notes[track].push(Note { start, len: len.min(self.end - start), key, vel });
    }

    /// The expression gliding from `from` to `to` over `len` ticks, a step every half beat.
    fn glide(&mut self, track: usize, a: u32, len: u32, from: f64, to: f64) {
        self.ramp(track, 11, a, len, from, to, self.beat / 2);
    }

    /// A controller gliding from `from` to `to` over `len` ticks, a step every `step`.
    #[allow(clippy::too_many_arguments)]
    fn ramp(&mut self, track: usize, cc: u8, a: u32, len: u32, from: f64, to: f64, step: u32) {
        let step = step.max(1);
        let n = (len / step).max(1);
        for i in 0..=n {
            let tick = a + i * step;
            if tick >= self.end || (i == n && tick > a + len) {
                break;
            }
            let v = from + (to - from) * i as f64 / n as f64;
            self.ctrls[track].push(Ctrl { tick, kind: CtrlKind::Cc(cc), value: v.round().clamp(0.0, 127.0) as i16 });
        }
    }

    /// One controller value.
    fn cc(&mut self, track: usize, tick: u32, cc: u8, v: f64) {
        if tick < self.end {
            self.ctrls[track].push(Ctrl { tick, kind: CtrlKind::Cc(cc), value: v.round().clamp(0.0, 127.0) as i16 });
        }
    }

    /// Legato pedalling: the pedal lifted as a new chord sounds, down again just after,
    /// so each harmony rings into the next without blurring them.
    fn pedal(&mut self, track: usize, t0: u32) {
        self.cc(track, t0, 64, 0.0);
        self.cc(track, t0 + self.beat / 6, 64, 127.0);
    }
}

/// Write Homeward's notes into `p`'s tracks.
pub(crate) fn write(p: &mut Project, t: &Tracks, r: &Recipe, secs: &[(Part, u32, u32)], density: f64) {
    let (beat, bar) = (p.beat_ticks(), p.bar_ticks());
    let bpb = p.meter.0.max(1);
    let end = secs.last().map_or(bar, |s| s.2);
    let steps = r.mode.steps();
    let theme = Theme::new(r.seed, bpb, t.is_return());
    let mut w = Writer {
        notes: vec![Vec::new(); p.tracks.len()],
        ctrls: vec![Vec::new(); p.tracks.len()],
        end,
        beat,
        touch: Rng::stream(r.seed, "homeward-touch"),
    };
    let mut harp_rng = Rng::stream(r.seed, "homeward-harp");
    let lead_prog = match &p.tracks[t.lead].source {
        Source::SoundFont { program, .. } => *program,
        _ => 73,
    };
    let viola = matches!(&p.tracks[t.cellos].source, Source::SoundFont { program: 41, .. });
    // the theme's tonic for each voice: the piano's right hand, cellos, the lead, the
    // strings' upper octave at the peak
    let tonic = |target: i32| near(r.root % 12, target) as i32;
    let (piano_t, cello_t, strings_t) = (tonic(67), tonic(if viola { 57 } else { 50 }), tonic(72));
    // what opens the theme, and the low strings under it (the cellos of their own, in
    // Return, or the same section)
    let opener = t.ep.unwrap_or(t.piano);
    let (low, low_t) = match t.counter {
        Some(c) => (c, tonic(50)),
        None => (t.cellos, cello_t),
    };
    let lead_t = tonic(match lead_prog {
        71 => 62,
        60 => 55,
        _ => 74,
    });
    // the leading tone is raised at the cadences where the mode has a flat seventh
    let flat7 = steps[6] == 10;
    let pitch = |tonic: i32, deg: i32, raise: bool| -> i32 {
        tonic + 12 * deg.div_euclid(7) + steps[deg.rem_euclid(7) as usize] as i32 + i32::from(raise && deg.rem_euclid(7) == 6 && flat7)
    };
    // a chord's pitch classes on a root degree (with its seventh), the dominant major
    let chord_degs = |root: i32, seventh: bool| -> Vec<i32> {
        let mut d = vec![root, root + 2, root + 4];
        if seventh {
            d.push(root + 6);
        }
        d
    };
    let chord_key = |root: i32, deg: i32, target: i32| -> i32 {
        let k = pitch(r.root as i32 % 12, deg, root.rem_euclid(7) == 4);
        near(k.rem_euclid(12) as u8, target) as i32
    };
    // a voicing of the chord on `root` around `centre`, low to high
    let voicing = |root: i32, seventh: bool, centre: i32| -> Vec<i32> {
        let mut v: Vec<i32> = chord_degs(root, seventh).into_iter().map(|d| chord_key(root, d, centre)).collect();
        v.sort_unstable();
        v.dedup();
        v
    };

    for (si, &(part, a, z)) in secs.iter().enumerate() {
        let n = (z - a).div_ceil(bar);
        let bars = layout(part, n, si == 0);
        let last_section = si + 1 == secs.len();
        let mut counter_prev = low_t + 7;
        for (k, &what) in bars.iter().enumerate() {
            let t0 = a + k as u32 * bar;
            if t0 >= z {
                break;
            }
            let final_bar = last_section && k + 1 == bars.len();
            let blen = if final_bar { end - t0 } else { bar.min(z - t0) };
            let x = k as f64 / bars.len().max(1) as f64;
            let (tb, build) = match what {
                Bar::Theme(tb) => (tb, None),
                Bar::Build(tb, bx, _) => (tb, Some(bx)),
                Bar::Hold => (7, None),
            };
            // the chord: the theme's, reharmonised in the Swell, the dominant before the peak
            let root = match what {
                Bar::Build(_, _, true) => 4,
                Bar::Hold => 0,
                _ if part == Part::Swell => theme.alt[tb],
                _ => theme.base[tb],
            };
            let seventh = part == Part::Swell && root != 0;
            let raise = root.rem_euclid(7) == 4;
            let shape = SHAPE[tb];
            let bass_key = chord_key(root, root, 36);
            let sub_key = chord_key(root, root, 31).max(24);
            // the theme in this bar, for the voice that has it
            let melody = |w: &mut Writer, track: usize, tonic: i32, vel: f64, legato: bool, ornament: bool| {
                let notes = &theme.bars[tb];
                for (j, n) in notes.iter().enumerate() {
                    let s = t0 + (n.at * beat as f64).round() as u32;
                    let mut len = (n.len * beat as f64).round() as u32;
                    if final_bar && j + 1 == notes.len() {
                        len = end - s;
                    } else if legato {
                        len += beat / 8;
                    } else {
                        len = len * 9 / 10;
                    }
                    let lift = 10.0 * shape + if j == 0 { 4.0 } else { 0.0 };
                    let next = notes.get(j + 1).map(|m| m.deg);
                    // a passing note into a leap, in the woodwind's variation
                    if ornament && n.len >= 2.0 && next.is_some_and(|d| (d - n.deg).abs() >= 2) {
                        let half = len / 2;
                        w.note(track, s, half + beat / 16, pitch(tonic, n.deg, raise), vel + lift);
                        let pass = n.deg + (next.unwrap_or(n.deg) - n.deg).signum();
                        w.note(track, s + half, len - half, pitch(tonic, pass, raise), vel + lift - 6.0);
                        continue;
                    }
                    w.note(track, s, len, pitch(tonic, n.deg, raise), vel + lift);
                }
            };
            // Return's piano and violas, played with grace (D24): the theme's notes shaped by
            // the phrase (louder towards its peak, lighter on short notes, sighing at the
            // cadences), the peak's first note and a cadence's last arriving a touch late,
            // legato; the violas bowed, each long note swelling and its vibrato blooming
            // (`bow` the expression they sit around)
            let sing = |w: &mut Writer, track: usize, tonic: i32, vel: f64, bow: Option<f64>| {
                let notes = &theme.bars[tb];
                let (s0, s1) = (shape, SHAPE[(tb + 1) % 8]);
                let cadence = tb == 3 || tb == 7;
                for (j, n) in notes.iter().enumerate() {
                    let arc = s0 + (s1 - s0) * n.at / bpb as f64;
                    let last_note = j + 1 == notes.len();
                    let lean = if (tb == 5 && j == 0) || (cadence && last_note && j > 0) { beat / 16 } else { 0 };
                    let s = t0 + (n.at * beat as f64).round() as u32 + lean;
                    let dur = ((n.len * beat as f64).round() as u32).saturating_sub(lean);
                    let len = if final_bar && last_note { end - s } else { dur + beat / 6 };
                    let metric = if n.at == 0.0 {
                        4.0
                    } else if bpb.is_multiple_of(2) && n.at == (bpb / 2) as f64 {
                        2.0
                    } else {
                        0.0
                    };
                    let light = if n.len < 1.0 { -5.0 } else { 0.0 };
                    let sigh = if cadence && last_note { -6.0 } else { 0.0 };
                    w.note(track, s, len, pitch(tonic, n.deg, raise), vel + 14.0 * arc + metric + light + sigh);
                    if let Some(base) = bow {
                        let e = base + 18.0 * arc + sigh;
                        if n.len >= 1.5 {
                            // a swell over the note, and a vibrato that blooms as it's held
                            let rise = dur * 3 / 5;
                            w.ramp(track, 11, s, rise, e - 10.0, e + 6.0, beat / 8);
                            w.ramp(track, 11, s + rise, dur - rise, e + 6.0, e - 2.0, beat / 8);
                            w.cc(track, s, 1, 8.0);
                            w.ramp(track, 1, s + dur / 3, dur - dur / 3, 8.0, 52.0, beat / 8);
                        } else {
                            w.ramp(track, 11, s, dur, e - 6.0, e, beat / 8);
                            w.cc(track, s, 1, 16.0);
                        }
                    }
                }
            };
            let graceful = t.is_return();
            let held = |len: u32| if final_bar { len.max(blen) } else { len };
            match part {
                // the theme on the piano alone, a soft chord under each half bar
                Part::Intro if graceful => {
                    sing(&mut w, opener, piano_t, 56.0, None);
                    piano_flow(&mut w, opener, t0, blen, beat, &voicing(root, false, 55), bass_key + 12, 34.0, final_bar);
                    w.pedal(opener, t0);
                    if k + 1 == bars.len() && !final_bar {
                        // let go as the violas begin
                        w.cc(opener, z, 64, 0.0);
                    }
                }
                Part::Intro => {
                    melody(&mut w, opener, piano_t, 58.0, false, false);
                    piano_chords(&mut w, opener, t0, blen, bpb, beat, &voicing(root, false, 55), bass_key + 12, 36.0, final_bar);
                }
                Part::Neon => {
                    // the cellos take the theme, legato, breathing with it
                    if graceful {
                        sing(&mut w, t.cellos, cello_t, 72.0, Some(86.0));
                        piano_flow(&mut w, t.piano, t0, blen, beat, &voicing(root, false, 57), bass_key + 12, 30.0, final_bar);
                        w.pedal(t.piano, t0);
                    } else {
                        melody(&mut w, t.cellos, cello_t, 76.0, true, false);
                        w.glide(t.cellos, t0, blen, 88.0 + 18.0 * shape, 88.0 + 18.0 * SHAPE[(tb + 1) % 8]);
                        piano_chords(&mut w, t.piano, t0, blen, bpb, beat, &voicing(root, false, 57), bass_key + 12, 32.0, final_bar);
                    }
                    for &k in &voicing(root, false, 64) {
                        w.note(t.strings, t0, held(blen), k, 40.0);
                    }
                    w.glide(t.strings, t0, blen, 55.0, 60.0);
                    w.note(t.double_bass, t0, held(blen), bass_key, 50.0);
                    w.note(t.sub, t0, held(blen), sub_key, 46.0);
                    if density > 0.45 || k % 2 == 0 || k + 1 == bars.len() {
                        harp(
                            &mut w,
                            &mut harp_rng,
                            t.harp,
                            t0,
                            blen,
                            beat,
                            bpb,
                            &voicing(root, false, 60),
                            46.0,
                            false,
                            k + 1 == bars.len(),
                        );
                    }
                }
                Part::Swell => {
                    // the theme reharmonised, on a flute, clarinet or horn, varied
                    melody(&mut w, t.lead, lead_t, 70.0, true, true);
                    w.glide(t.lead, t0, blen, 72.0 + 20.0 * shape, 72.0 + 20.0 * SHAPE[(tb + 1) % 8]);
                    // the cellos' countermelody: chord tones moving against the theme
                    let notes = &theme.bars[tb];
                    let up = notes.last().map_or(0, |n| n.deg) >= notes[0].deg;
                    let halves = if bpb.is_multiple_of(2) && bpb >= 4 { 2 } else { 1 };
                    for h in 0..halves {
                        let s = t0 + h * bar / halves;
                        let len = if h + 1 == halves { blen - h * bar / halves } else { bar / halves };
                        let tones = voicing(root, seventh, counter_prev);
                        let cands: Vec<i32> =
                            tones.iter().copied().filter(|&k| if up { k <= counter_prev } else { k >= counter_prev }).collect();
                        let pick = cands.into_iter().min_by_key(|&k| ((k - counter_prev).abs() - 2).abs()).unwrap_or(tones[0]);
                        let pick = pick.clamp(low_t, low_t + 14);
                        w.note(low, s, held(len + beat / 8), pick, 58.0 + 6.0 * shape);
                        counter_prev = pick;
                    }
                    w.glide(low, t0, blen, 62.0, 66.0);
                    for &k in &voicing(root, seventh, 65) {
                        w.note(t.strings, t0, held(blen), k, 44.0);
                    }
                    w.glide(t.strings, t0, blen, 60.0, 64.0);
                    if graceful {
                        piano_flow(&mut w, t.piano, t0, blen, beat, &voicing(root, seventh, 58), bass_key + 12, 30.0, final_bar);
                        w.pedal(t.piano, t0);
                    } else {
                        piano_chords(&mut w, t.piano, t0, blen, bpb, beat, &voicing(root, seventh, 58), bass_key + 12, 32.0, final_bar);
                    }
                    w.note(t.double_bass, t0, held(blen), bass_key, 54.0);
                    w.note(t.sub, t0, held(blen), sub_key, 48.0);
                    harp(
                        &mut w,
                        &mut harp_rng,
                        t.harp,
                        t0,
                        blen,
                        beat,
                        bpb,
                        &voicing(root, seventh, 62),
                        48.0,
                        density > 0.55,
                        k + 1 == bars.len(),
                    );
                }
                Part::NightShift => match build {
                    // the pulse builds under layered strings and the warm pad
                    Some(bx) => {
                        let lift = bx + 1.0 / bars.len().max(1) as f64;
                        let layers = 2 + (bx * 3.0) as usize;
                        let v = voicing(root, false, 62);
                        let mut v: Vec<i32> = v.iter().rev().take(layers.min(v.len())).copied().collect();
                        if bx > 0.5 {
                            v.push(v.iter().max().copied().unwrap_or(67) + 12);
                        }
                        for &k in &v {
                            w.note(t.strings, t0, blen, k, 50.0 + 30.0 * bx);
                        }
                        w.glide(t.strings, t0, blen, 55.0 + 45.0 * bx, 55.0 + 45.0 * lift.min(1.0));
                        for &k in &voicing(root, false, 60) {
                            w.note(t.pad, t0, blen, k, 44.0 + 20.0 * bx);
                        }
                        w.glide(t.pad, t0, blen, 50.0 + 40.0 * bx, 50.0 + 40.0 * lift.min(1.0));
                        if graceful {
                            w.cc(t.piano, t0, 64, 0.0);
                        }
                        // the soft pulse: the root and fifth in the cellos and bass, a beat at a time
                        let fifth = chord_key(root, root + 4, low_t + 7);
                        for b in 0..bpb {
                            let s = t0 + b * beat;
                            if s >= t0 + blen {
                                break;
                            }
                            let accent = if b == 0 { 6.0 } else { 0.0 };
                            w.note(low, s, beat * 9 / 10, chord_key(root, root, low_t), 48.0 + 30.0 * bx + accent);
                            w.note(low, s, beat * 9 / 10, fifth, 42.0 + 30.0 * bx);
                            w.note(t.double_bass, s, beat * 9 / 10, bass_key, 50.0 + 30.0 * bx + accent);
                            w.note(t.piano, s, beat * 8 / 10, voicing(root, false, 60)[0], 30.0 + 26.0 * bx);
                            for &k in voicing(root, false, 60).iter().skip(1) {
                                w.note(t.piano, s, beat * 8 / 10, k, 28.0 + 26.0 * bx);
                            }
                        }
                        w.glide(low, t0, blen, 70.0 + 30.0 * bx, 70.0 + 30.0 * lift.min(1.0));
                        w.note(t.sub, t0, blen, sub_key, 48.0 + 20.0 * bx);
                        harp(&mut w, &mut harp_rng, t.harp, t0, blen, beat, bpb, &voicing(root, false, 62), 46.0 + 24.0 * bx, true, false);
                    }
                    // the peak: the theme in full strings in octaves, the cellos under them
                    None => {
                        melody(&mut w, t.strings, strings_t, 96.0, true, false);
                        melody(&mut w, t.strings, strings_t - 12, 88.0, true, false);
                        w.glide(t.strings, t0, blen, 100.0 + 15.0 * shape, 100.0 + 15.0 * SHAPE[(tb + 1) % 8]);
                        melody(&mut w, low, low_t, 90.0, true, false);
                        w.glide(low, t0, blen, 100.0, 104.0);
                        if t.counter.is_some() {
                            // the violas join the strings' lower octave, bowed with the phrase
                            sing(&mut w, t.cellos, cello_t, 82.0, Some(98.0));
                        }
                        for &k in &voicing(root, false, 58) {
                            w.note(t.pad, t0, held(blen), k, 62.0);
                        }
                        w.glide(t.pad, t0, blen, 90.0, 92.0);
                        if graceful {
                            piano_flow(&mut w, t.piano, t0, blen, beat, &voicing(root, false, 60), bass_key + 12, 52.0, final_bar);
                            w.pedal(t.piano, t0);
                        } else {
                            piano_chords(&mut w, t.piano, t0, blen, bpb, beat, &voicing(root, false, 60), bass_key + 12, 56.0, final_bar);
                        }
                        for h in [0, bar / 2] {
                            if h < blen {
                                w.note(
                                    t.double_bass,
                                    t0 + h,
                                    (bar / 2).min(blen - h) * 19 / 20,
                                    bass_key,
                                    78.0 - if h > 0 { 8.0 } else { 0.0 },
                                );
                            }
                        }
                        w.note(t.sub, t0, held(blen), sub_key, 66.0);
                        harp(&mut w, &mut harp_rng, t.harp, t0, blen, beat, bpb, &voicing(root, false, 64), 66.0, true, false);
                    }
                },
                Part::Fade => {
                    // home: the theme on the piano as it began, the strings settling under it
                    // (the theme's last bar rings on through the bars held after it)
                    if what == Bar::Hold {
                        continue;
                    }
                    let fade = 1.0 - x;
                    // the theme's last bar in the piece's last section: the final chord
                    let last_theme = last_section && tb == 7 && bars[k + 1..].iter().all(|b| *b == Bar::Hold);
                    let chord = voicing(root, false, 57);
                    if graceful {
                        sing(&mut w, t.piano, piano_t, 50.0, None);
                        piano_flow(&mut w, t.piano, t0, blen, beat, &chord, bass_key + 12, 28.0, final_bar || last_theme);
                        w.pedal(t.piano, t0);
                    } else {
                        melody(&mut w, t.piano, piano_t, 52.0, false, false);
                        piano_chords(&mut w, t.piano, t0, blen, bpb, beat, &chord, bass_key + 12, 30.0, final_bar || last_theme);
                    }
                    let ring = if last_theme || final_bar { end.saturating_sub(t0) } else { blen };
                    {
                        let mut top = voicing(root, false, 64);
                        if last_theme {
                            // quietly hopeful: the ninth above the tonic
                            top.push(chord_key(0, 1, 74));
                        }
                        for &kk in &top {
                            w.note(t.strings, t0, ring, kk, 40.0);
                        }
                        w.note(low, t0, ring, chord_key(root, root, low_t), 44.0);
                        w.note(low, t0, ring, chord_key(root, root + 4, low_t + 7), 40.0);
                        w.note(t.double_bass, t0, ring, bass_key, 44.0);
                        if x < 0.5 || last_theme {
                            w.note(t.sub, t0, ring, sub_key, 40.0);
                        }
                        if last_theme {
                            for &kk in &voicing(0, false, 60) {
                                w.note(t.pad, t0, ring, kk, 40.0);
                            }
                            w.glide(t.pad, t0, ring, 70.0, 20.0);
                            // a last rolled chord on the harp
                            for (j, &kk) in voicing(0, false, 60).iter().chain(voicing(0, false, 72).iter()).enumerate() {
                                w.note(t.harp, t0 + j as u32 * beat / 6, beat * bpb.min(4), kk, 42.0);
                            }
                        }
                    }
                    let (from, to) = (30.0 + 40.0 * fade, 30.0 + 40.0 * (fade - 1.0 / bars.len().max(1) as f64).max(0.0));
                    let span = if last_theme { end.saturating_sub(t0) } else { blen };
                    let (from, to) = if last_theme { (from, 18.0) } else { (from, to) };
                    w.glide(t.strings, t0, span, from, to);
                    w.glide(low, t0, span, from, to);
                    if k == 0 {
                        harp(&mut w, &mut harp_rng, t.harp, t0, blen, beat, bpb, &chord, 40.0, false, false);
                    }
                }
            }
        }
    }
    for (i, tr) in p.tracks.iter_mut().enumerate() {
        tr.notes.append(&mut w.notes[i]);
        tr.ctrls.append(&mut w.ctrls[i]);
        tr.ctrls.retain(|c| c.tick < end);
    }
    // the tracks as an orchestra sits: the piano, the strings low to high, the lead,
    // the harp and the pad (the others, silent, go)
    let order: Vec<usize> = match (t.ep, t.counter) {
        (Some(ep), Some(c)) => vec![ep, t.piano, t.cellos, c, t.strings, t.double_bass, t.sub, t.lead, t.harp, t.pad],
        _ => vec![t.piano, t.cellos, t.strings, t.double_bass, t.sub, t.lead, t.harp, t.pad],
    };
    let mut tracks = std::mem::take(&mut p.tracks);
    let mut sorted: Vec<crate::project::Track> = order.iter().map(|&i| std::mem::take(&mut tracks[i])).collect();
    for (i, tr) in sorted.iter_mut().enumerate() {
        tr.color = crate::project::NEON[i % crate::project::NEON.len()];
    }
    p.tracks = sorted;
}

/// Soft piano chords: the low root and the chord on the first beat, the chord again at
/// the half bar (or held whole in an odd metre, or to the end on the last).
#[allow(clippy::too_many_arguments)]
fn piano_chords(w: &mut Writer, track: usize, t0: u32, blen: u32, bpb: u32, beat: u32, chord: &[i32], low: i32, vel: f64, ring: bool) {
    let low = if low > 52 { low - 12 } else { low };
    if ring {
        w.note(track, t0, w.end - t0, low, vel - 2.0);
        for &k in chord {
            w.note(track, t0 + beat / 12, w.end - t0, k, vel);
        }
        return;
    }
    let halves = if bpb.is_multiple_of(2) && bpb >= 4 { 2 } else { 1 };
    let step = blen / halves;
    w.note(track, t0, blen * 19 / 20, low, vel - 2.0);
    for h in 0..halves {
        for &k in chord {
            w.note(track, t0 + h * step + beat / 12, step * 9 / 10, k, vel - if h > 0 { 4.0 } else { 0.0 });
        }
    }
}

/// Return's piano under the theme: the bass on the downbeat, then the chord broken
/// gently up and back in quavers, swelling a little through the bar (pedalled); at the
/// end, the bass and the chord rolled slowly upwards and held.
#[allow(clippy::too_many_arguments)]
fn piano_flow(w: &mut Writer, track: usize, t0: u32, blen: u32, beat: u32, chord: &[i32], low: i32, vel: f64, ring: bool) {
    let low = if low > 52 { low - 12 } else { low };
    let mut ladder: Vec<i32> = chord.to_vec();
    ladder.sort_unstable();
    if ladder.is_empty() {
        return;
    }
    if ring {
        w.note(track, t0, w.end - t0, low, vel);
        for (i, &k) in ladder.iter().enumerate() {
            let s = t0 + beat / 8 + i as u32 * beat / 6;
            w.note(track, s, w.end.saturating_sub(s), k, vel - 2.0 + 2.0 * i as f64);
        }
        return;
    }
    if ladder.len() < 4 {
        ladder.push(ladder[0] + 12);
    }
    w.note(track, t0, blen, low, vel + 2.0);
    let step = (beat / 2).max(1);
    let count = (blen / step) as usize;
    let up = ladder.len();
    let period = 2 * (up - 1).max(1);
    for i in 1..count {
        let m = (i - 1) % period;
        let j = if m < up { m } else { period - m };
        let arc = (std::f64::consts::PI * i as f64 / count as f64).sin();
        let on_beat = (i as u32 * step).is_multiple_of(beat);
        w.note(track, t0 + i as u32 * step, step * 2, ladder[j], vel - 7.0 + 6.0 * arc + if on_beat { 2.0 } else { 0.0 });
    }
}

/// A gentle harp arpeggio through the chord, rising (and falling again when `full`):
/// quavers, or crotchets when not `busy`; the last bar of a section runs up two octaves
/// into the next.
#[allow(clippy::too_many_arguments)]
fn harp(
    w: &mut Writer,
    rng: &mut Rng,
    track: usize,
    t0: u32,
    blen: u32,
    beat: u32,
    bpb: u32,
    chord: &[i32],
    vel: f64,
    busy: bool,
    into_next: bool,
) {
    if chord.is_empty() {
        return;
    }
    let step = if busy || into_next { beat / 2 } else { beat };
    let count = (blen / step.max(1)) as usize;
    let span = if into_next { 2 } else { 1 + usize::from(rng.chance(0.5)) };
    let mut ladder: Vec<i32> = Vec::new();
    for o in 0..=span {
        ladder.extend(chord.iter().map(|k| k + 12 * o as i32));
    }
    ladder.sort_unstable();
    ladder.dedup();
    let up = ladder.len();
    let _ = bpb;
    for i in 0..count {
        // up through the ladder and back down (or on up, into the next section)
        let j = if into_next {
            (i * up / count.max(1)).min(up - 1)
        } else {
            let period = 2 * (up - 1).max(1);
            let m = i % period;
            if m < up { m } else { period - m }
        };
        let s = t0 + i as u32 * step;
        let accent = if i == 0 { 6.0 } else { 0.0 };
        let rise = if into_next { 10.0 * i as f64 / count.max(1) as f64 } else { 0.0 };
        w.note(track, s, step * 3, ladder[j], vel + accent + rise);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Style, compose};
    use crate::palette::Role;

    fn piece(seed: u64, bars: u32, meter: (u32, u32)) -> Project {
        compose(&Recipe { style: Style::Homeward, ..Recipe::random(seed) }, 69.0, meter, bars)
    }

    /// Return: the electric piano opens alone, the violas carry the theme, the clarinet
    /// varies it answered by the cellos, the strings peak, the solo piano brings it home
    /// to the tonic chord with its ninth, held; E minor at 66 bpm; and the lift reaches
    /// the relative major wherever the theme's peak note is in it.
    #[test]
    fn return_is_homeward_with_its_own_instruments() {
        assert_eq!(return_plan(52).iter().map(|s| s.1).collect::<Vec<_>>(), [8, 8, 8, 16, 12]);
        for bars in [20, 33, 48, 64, 100] {
            assert_eq!(return_plan(bars).iter().map(|s| s.1).sum::<u32>(), bars);
        }
        let mut lifted = 0;
        for seed in 0..12 {
            let mut r = Recipe { style: Style::Return, ..Recipe::random(seed) };
            (r.root, r.mode) = Style::Return.key().unwrap();
            let p = compose(&r, Style::Return.tempo(), (4, 4), 52);
            let names: Vec<&str> = p.tracks.iter().map(|t| t.name.as_str()).collect();
            assert_eq!(
                names,
                ["Electric Piano", "Piano", "Violas", "Cellos", "Strings", "Double Bass", "Sub", "Clarinet", "Harp", "Pad"],
                "seed {seed}"
            );
            let b = p.block_ticks();
            for t in p.tracks.iter().filter(|t| t.name != "Electric Piano") {
                assert_eq!(in_span(&p, &t.name, b[0]), 0, "seed {seed}: {} in the Intro", t.name);
            }
            assert_eq!(in_span(&p, "Electric Piano", (b[1].0, p.end_tick())), 0, "seed {seed}");
            assert!(in_span(&p, "Violas", b[1]) >= 8, "seed {seed}");
            assert_eq!(in_span(&p, "Violas", b[2]) + in_span(&p, "Violas", b[4]), 0, "seed {seed}");
            assert!(in_span(&p, "Clarinet", b[2]) >= 8 && in_span(&p, "Cellos", b[2]) >= 8, "seed {seed}");
            for i in [0, 1, 3, 4] {
                assert_eq!(in_span(&p, "Clarinet", b[i]), 0, "seed {seed}: the clarinet in section {i}");
            }
            assert!(in_span(&p, "Piano", b[4]) >= 8, "seed {seed}");
            // the last chord: E minor with its ninth (F sharp), held to the end
            let end = p.end_tick();
            let held: Vec<u8> = p.tracks.iter().flat_map(|t| &t.notes).filter(|n| n.end() == end).map(|n| n.key % 12).collect();
            for pc in [4, 7, 11, 6] {
                assert!(held.contains(&pc), "seed {seed}: {held:?}");
            }
            assert!(p.tracks.iter().all(|t| !matches!(t.role, Some(Role::Drums | Role::City))));
            // the lift: III under the peak bar wherever its first note is G, B or D
            let th = Theme::new(r.seed, 4, true);
            if matches!(th.bars[5][0].deg.rem_euclid(7), 2 | 4 | 6) {
                assert_eq!(th.base[5], 2, "seed {seed}");
                lifted += 1;
            }
        }
        assert!(lifted >= 3, "{lifted}");
    }

    fn in_span(p: &Project, name: &str, (a, b): (u32, u32)) -> usize {
        p.tracks.iter().filter(|t| t.name == name).flat_map(|t| &t.notes).filter(|n| n.start >= a && n.start < b).count()
    }

    /// The theme's story: the piano alone in the Intro, the cellos in the Neon, the lead
    /// only in the Swell, the peak in the strings loudest of all, and the piano home in
    /// the Fade, ending on a held tonic chord. No drums, no city, only its own sounds.
    #[test]
    fn the_theme_travels_and_comes_home() {
        for seed in 0..10 {
            let p = piece(seed, 50, (4, 4));
            let r = p.recipe.unwrap();
            let parts: Vec<Option<Part>> = p.arrangement().iter().map(|b| b.part).collect();
            assert_eq!(parts, [Some(Part::Intro), Some(Part::Neon), Some(Part::Swell), Some(Part::NightShift), Some(Part::Fade)]);
            let blocks = p.block_ticks();
            for t in p.tracks.iter().filter(|t| t.name != "Piano") {
                assert_eq!(in_span(&p, &t.name, blocks[0]), 0, "seed {seed}: {} in the Intro", t.name);
            }
            assert!(in_span(&p, "Piano", blocks[0]) >= 8);
            let cellos = if p.tracks.iter().any(|t| t.name == "Violas") { "Violas" } else { "Cellos" };
            assert!(in_span(&p, cellos, blocks[1]) >= 8, "seed {seed}");
            assert!(in_span(&p, "Lead", blocks[2]) >= 8, "seed {seed}");
            for i in [0, 1, 3, 4] {
                assert_eq!(in_span(&p, "Lead", blocks[i]), 0, "seed {seed}: the lead in section {i}");
            }
            assert!(p.tracks.iter().all(|t| !matches!(t.role, Some(Role::Drums | Role::City))));
            // the peak is the loudest the strings play
            let strings = p.tracks.iter().find(|t| t.name == "Strings").unwrap();
            let loudest = strings.notes.iter().max_by_key(|n| n.vel).unwrap();
            assert!(loudest.start >= blocks[3].0 && loudest.start < blocks[3].1, "seed {seed}");
            // the end: a tonic chord held to the last tick
            let end = p.end_tick();
            let held: Vec<u8> = p
                .tracks
                .iter()
                .flat_map(|t| &t.notes)
                .filter(|n| n.end() == end && n.start < end - p.bar_ticks() / 2)
                .map(|n| n.key % 12)
                .collect();
            assert!(held.len() >= 4 && held.contains(&(r.root % 12)), "seed {seed}: {held:?}");
            let low = p.tracks.iter().find(|t| t.name == "Double Bass").unwrap().notes.last().unwrap();
            assert_eq!(low.key % 12, r.root % 12, "seed {seed}: the bass ends on the tonic");
            // its own sounds
            for t in &p.tracks {
                let label = match &t.source {
                    Source::SoundFont { name, .. } => name.clone(),
                    Source::Synth(patch) => patch.name.clone(),
                    Source::Kit(_) => "kit".into(),
                };
                assert!(
                    [
                        "Grand Piano",
                        "Electric Piano",
                        "Cello",
                        "Viola",
                        "Slow Strings",
                        "Strings",
                        "Contrabass",
                        "Pure Sub",
                        "Synth Strings (FreePats)",
                        "Warm Pad",
                        "Flute",
                        "Clarinet",
                        "French Horn",
                        "Concert Harp (FreePats)",
                        "Harp"
                    ]
                    .contains(&label.as_str()),
                    "seed {seed}: {} plays {label}",
                    t.name
                );
            }
            assert_eq!(p.fx.ensemble, 0.0);
        }
    }

    /// The theme: eight bars, every bar filled, the first phrase resting on the dominant
    /// (or the second), the last bar the tonic held; its second phrase echoes the first's
    /// rhythm.
    #[test]
    fn the_theme_is_eight_singable_bars() {
        for seed in 0..40 {
            for bpb in [3, 4, 5, 6] {
                let th = Theme::new(seed, bpb, false);
                assert_eq!(th.bars.len(), 8);
                for b in &th.bars {
                    assert!((b.iter().map(|n| n.len).sum::<f64>() - bpb as f64).abs() < 1e-9);
                }
                assert!(matches!(th.bars[3].last().unwrap().deg.rem_euclid(7), 4 | 1));
                assert_eq!(th.bars[7].len(), 1);
                assert_eq!(th.bars[7][0].deg.rem_euclid(7), 0);
                assert_eq!(th.base[7], 0);
                assert_eq!(th.base[6], 4, "a dominant before home");
                let rh = |b: usize| th.bars[b].iter().map(|n| n.len).collect::<Vec<_>>();
                assert_eq!(rh(0), rh(4));
                // within an octave and a third, mostly by step, never a leap past a fifth
                let degs: Vec<i32> = th.bars.iter().flatten().map(|n| n.deg).collect();
                assert!(degs.iter().max().unwrap() - degs.iter().min().unwrap() <= 10, "seed {seed}: {degs:?}");
                let leaps = degs.windows(2).filter(|w| (w[1] - w[0]).abs() > 2).count();
                assert!(leaps <= 4, "seed {seed}: {degs:?}");
                assert!(degs.windows(2).all(|w| (w[1] - w[0]).abs() <= 4), "seed {seed}: {degs:?}");
            }
        }
    }

    /// Other styles never choose the sounds added for Homeward (and Return, which shares them).
    #[test]
    fn its_sounds_are_its_own() {
        // (Chamber, written after, shares its strings, woodwinds, horn and harp on purpose)
        for style in
            Style::ALL.into_iter().filter(|s| !matches!(s, Style::Homeward | Style::Return | Style::Chamber | Style::Orbit | Style::Nebula))
        {
            for seed in 0..20 {
                let p = compose(&Recipe { style, ..Recipe::random(seed) }, 70.0, (4, 4), 32);
                for t in &p.tracks {
                    if let Source::SoundFont { name, .. } = &t.source {
                        assert!(!crate::compose::HOMEWARD_SOUNDS.contains(&name.as_str()), "{style:?} {seed}: {name}");
                    }
                }
            }
        }
    }
}
