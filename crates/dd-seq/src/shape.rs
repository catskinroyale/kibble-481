//! A track's density and complexity (D31): two dials on each track that change what it
//! plays without touching what is written. Straight up (0.5) plays the notes as written.
//!
//! **Density** turned down leaves out the weakest notes first (a chord counts as one: off
//! the beat before on it, short and soft before long and loud), down to the strongest
//! sixth or so; turned up it fills the gaps after notes (a held note struck again, a hit
//! echoed), on the quavers first and then the semiquavers, each softer.
//!
//! **Complexity** turned down plays fewer notes of each chord (the bass and the top kept
//! longest) and fewer of the different notes in each bar's arpeggio or line (the lowest
//! and the most played kept longest); turned up it adds the chord's top an octave higher,
//! then its bass an octave lower, and doubles some single notes an octave up. On a drum
//! kit it keeps the lowest drums (the kick, then the snare) and adds soft ghost notes.
//!
//! Which notes change is decided by a hash of where they are, not a random stream, so
//! editing one note never changes what happens to another, and the same settings always
//! play the same notes.

use std::borrow::Cow;

use dd_dsp::sampler::KitKind;

use crate::project::{Note, Project, Source, Track};

/// The setting that plays the notes as written.
pub const NEUTRAL: f32 = 0.5;

pub fn is_neutral(v: &f32) -> bool {
    (*v - NEUTRAL).abs() < 1e-4
}

pub fn neutral() -> f32 {
    NEUTRAL
}

/// What a track's dials do to its written notes: which of them play (by index), and the
/// notes added.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shaped {
    pub kept: Vec<bool>,
    pub added: Vec<Note>,
}

/// Notes struck this close together (ticks) are one chord.
const TOGETHER: u32 = 40;

fn hash(a: u32, b: u32, salt: u32) -> f64 {
    let mut x = (a as u64) << 32 | (b as u64) << 8 | salt as u64;
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// How a track is played: pitched, a drum kit, or sound effects (left as they are by
/// complexity: their keys are different sounds, not pitches).
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Pitched,
    Drums,
    Effects,
}

fn kind(source: &Source) -> Kind {
    match source {
        Source::Kit(KitKind::Drums) | Source::SoundFont { bank: 128, .. } => Kind::Drums,
        Source::Kit(_) => Kind::Effects,
        _ => Kind::Pitched,
    }
}

/// How strong a moment in the bar is: the downbeat, a beat, a quaver, a semiquaver, or
/// between.
fn weight(tick: u32, bar: u32, beat: u32) -> f64 {
    let q = (beat / 2).max(1);
    let s = (beat / 4).max(1);
    if tick.is_multiple_of(bar.max(1)) {
        4.0
    } else if tick.is_multiple_of(beat.max(1)) {
        3.0
    } else if tick.is_multiple_of(q) {
        2.0
    } else if tick.is_multiple_of(s) {
        1.0
    } else {
        0.0
    }
}

/// What `density` and `complexity` do to `notes` (in any order), in bars of `bar` and
/// beats of `beat` ticks.
pub fn shape(notes: &[Note], source: &Source, density: f32, complexity: f32, bar: u32, beat: u32) -> Shaped {
    let n = notes.len();
    let mut kept = vec![true; n];
    let mut added = Vec::new();
    if n == 0 || (is_neutral(&density) && is_neutral(&complexity)) {
        return Shaped { kept, added };
    }
    let kind = kind(source);
    // the chords: notes struck together, in time order
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (notes[i].start, notes[i].key));
    let mut chords: Vec<Vec<usize>> = Vec::new();
    for &i in &order {
        match chords.last_mut() {
            Some(c) if notes[i].start - notes[c[0]].start <= TOGETHER => c.push(i),
            _ => chords.push(vec![i]),
        }
    }

    // ---------------------------------------------------------------- complexity
    let c = complexity.clamp(0.0, 1.0) as f64;
    if c < NEUTRAL as f64 - 1e-4 && kind != Kind::Effects {
        let share = c / NEUTRAL as f64;
        // fewer notes of each chord: the bass and the top kept longest
        for ch in chords.iter().filter(|ch| ch.len() >= 2) {
            let mut by_pitch = ch.clone();
            by_pitch.sort_by_key(|&i| notes[i].key);
            let mut rank = vec![by_pitch[0], by_pitch[by_pitch.len() - 1]];
            rank.extend(by_pitch[1..by_pitch.len() - 1].iter().copied());
            let keep = ((ch.len() as f64 * share).round() as usize).max(1);
            for &i in &rank[keep..] {
                kept[i] = false;
            }
        }
        // fewer of the different notes struck alone in each bar (an arpeggio, a line, a
        // drum pattern): the lowest and the most played kept longest
        let singles: Vec<usize> = chords.iter().filter(|ch| ch.len() == 1).map(|ch| ch[0]).collect();
        let mut k = 0;
        while k < singles.len() {
            let b = notes[singles[k]].start / bar.max(1);
            let mut z = k;
            while z < singles.len() && notes[singles[z]].start / bar.max(1) == b {
                z += 1;
            }
            let mut keys: Vec<(u8, usize)> = Vec::new();
            for &i in &singles[k..z] {
                match keys.iter_mut().find(|(key, _)| *key == notes[i].key) {
                    Some(e) => e.1 += 1,
                    None => keys.push((notes[i].key, 1)),
                }
            }
            if keys.len() >= 3 {
                keys.sort_by_key(|&(key, count)| (std::cmp::Reverse(count), key));
                let low = keys.iter().map(|k| k.0).min().unwrap_or(0);
                let mut rank = vec![low];
                rank.extend(keys.iter().map(|k| k.0).filter(|&key| key != low));
                let keep = ((keys.len() as f64 * share).round() as usize).max(1);
                for &i in &singles[k..z] {
                    if !rank[..keep].contains(&notes[i].key) {
                        kept[i] = false;
                    }
                }
            }
            k = z;
        }
    } else if c > NEUTRAL as f64 + 1e-4 {
        let more = (c - NEUTRAL as f64) / (1.0 - NEUTRAL as f64);
        for ch in &chords {
            let lo = ch.iter().copied().min_by_key(|&i| notes[i].key).unwrap_or(ch[0]);
            let hi = ch.iter().copied().max_by_key(|&i| notes[i].key).unwrap_or(ch[0]);
            let h = hash(notes[lo].start, notes[lo].key as u32, 1);
            match kind {
                Kind::Pitched if ch.len() >= 2 => {
                    // the top an octave up, then the bass an octave down
                    if h < (2.0 * more).min(1.0) && notes[hi].key <= 115 {
                        added.push(Note { key: notes[hi].key + 12, vel: scale(notes[hi].vel, 0.75), ..notes[hi] });
                    }
                    if h < (2.0 * more - 1.0).max(0.0) && notes[lo].key >= 24 {
                        added.push(Note { key: notes[lo].key - 12, vel: scale(notes[lo].vel, 0.8), ..notes[lo] });
                    }
                }
                Kind::Pitched => {
                    if h < 0.6 * more && notes[lo].key <= 115 {
                        added.push(Note { key: notes[lo].key + 12, vel: scale(notes[lo].vel, 0.6), ..notes[lo] });
                    }
                }
                Kind::Drums => {
                    // a soft ghost of the hit a semiquaver before it
                    let s = (beat / 4).max(1);
                    if h < 0.5 * more && notes[hi].start >= s {
                        let g = notes[hi];
                        added.push(Note { start: g.start - s, len: g.len.min(s), vel: scale(g.vel, 0.35), ..g });
                    }
                }
                Kind::Effects => {}
            }
        }
    }

    // ---------------------------------------------------------------- density
    let d = density.clamp(0.0, 1.0) as f64;
    if d < NEUTRAL as f64 - 1e-4 {
        // the strongest share of the chords stays
        let share = 0.15 + 0.85 * d / NEUTRAL as f64;
        // (of the chords complexity left any of)
        let mut ranked: Vec<(f64, usize)> = chords
            .iter()
            .enumerate()
            .filter(|(_, ch)| ch.iter().any(|&i| kept[i]))
            .map(|(k, ch)| {
                let first = notes[ch[0]];
                let len = ch.iter().map(|&i| notes[i].len).max().unwrap_or(0) as f64 / beat.max(1) as f64;
                let vel = ch.iter().map(|&i| notes[i].vel).max().unwrap_or(0) as f64 / 127.0;
                let score = weight(first.start, bar, beat) + 0.6 * len.min(2.0) + 0.8 * vel + 0.5 * hash(first.start, first.key as u32, 2);
                (score, k)
            })
            .collect();
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        let keep = ((ranked.len() as f64 * share).ceil() as usize).max(1).min(ranked.len());
        for &(_, k) in &ranked[keep..] {
            for &i in &chords[k] {
                kept[i] = false;
            }
        }
    } else if d > NEUTRAL as f64 + 1e-4 && kind != Kind::Effects {
        let more = (d - NEUTRAL as f64) / (1.0 - NEUTRAL as f64);
        let s = (beat / 4).max(1);
        // each chord struck again in the gap before the next, on the quavers first
        for (k, ch) in chords.iter().enumerate() {
            let notes_here: Vec<Note> = ch.iter().filter(|&&i| kept[i]).map(|&i| notes[i]).collect();
            if notes_here.is_empty() {
                continue;
            }
            let start = notes_here[0].start;
            let next = chords.get(k + 1).map_or(notes_here.iter().map(|n| n.end()).max().unwrap_or(start), |c| notes[c[0]].start);
            let mut t = (start / s + 1) * s;
            let mut fade = 1.0;
            while t + s / 2 < next {
                let w = weight(t, bar, beat);
                let p = if w >= 2.0 { (2.0 * more).min(1.0) } else { (2.0 * more - 1.0).max(0.0) };
                if hash(t, notes_here[0].key as u32, 3) < p {
                    fade *= 0.85;
                    for m in &notes_here {
                        let len = if t < m.end() { m.end() - t } else { m.len.min(s) };
                        let len = len.min(next - t).max(1);
                        added.push(Note { start: t, len, vel: scale(m.vel, 0.75 * fade), ..*m });
                    }
                }
                t += s;
            }
        }
    }
    // nothing added where a played note already is
    added.retain(|a| !notes.iter().zip(&kept).any(|(n, &k)| k && n.key == a.key && n.start == a.start));
    added.sort_by_key(|a| (a.start, a.key));
    added.dedup_by_key(|a| (a.start, a.key));
    Shaped { kept, added }
}

fn scale(vel: u8, x: f64) -> u8 {
    ((vel as f64 * x).round() as u8).max(1)
}

/// The notes a track plays: as written when its dials are straight up.
pub fn played<'a>(p: &Project, t: &'a Track) -> Cow<'a, [Note]> {
    if is_neutral(&t.density) && is_neutral(&t.complexity) {
        return Cow::Borrowed(&t.notes);
    }
    let s = shape(&t.notes, &t.source, t.density, t.complexity, p.bar_ticks(), p.beat_ticks());
    let mut out: Vec<Note> = t.notes.iter().zip(&s.kept).filter(|(_, k)| **k).map(|(n, _)| *n).collect();
    out.extend(s.added);
    out.sort_by_key(|n| (n.start, n.key));
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Recipe, Style, compose};

    fn synth() -> Source {
        Source::Synth(dd_dsp::patch::by_name("Neon Brass"))
    }

    fn n(start: u32, len: u32, key: u8) -> Note {
        Note { start, len, key, vel: 90 }
    }

    /// Straight up, every note as written and nothing added.
    #[test]
    fn neutral_plays_as_written() {
        let notes = vec![n(0, 960, 60), n(0, 960, 64), n(480, 240, 67)];
        let s = shape(&notes, &synth(), NEUTRAL, NEUTRAL, 3840, 960);
        assert!(s.kept.iter().all(|k| *k) && s.added.is_empty());
    }

    /// Less complexity keeps the bass and top of a chord first, then the bass alone.
    #[test]
    fn simpler_chords_keep_their_outer_notes() {
        let notes = vec![n(0, 960, 60), n(0, 960, 64), n(0, 960, 67), n(0, 960, 72)];
        let s = shape(&notes, &synth(), NEUTRAL, 0.25, 3840, 960);
        assert_eq!(s.kept, [true, false, false, true]);
        let s = shape(&notes, &synth(), NEUTRAL, 0.0, 3840, 960);
        assert_eq!(s.kept.iter().filter(|k| **k).count(), 1);
        assert!(s.kept[0]);
    }

    /// Less complexity plays fewer of an arpeggio's notes, the lowest kept.
    #[test]
    fn simpler_arpeggios_use_fewer_notes() {
        let notes: Vec<Note> = (0..16).map(|i| n(i * 240, 240, [48, 55, 60, 64][i as usize % 4])).collect();
        let s = shape(&notes, &synth(), NEUTRAL, 0.25, 3840, 960);
        let keys: std::collections::BTreeSet<u8> = notes.iter().zip(&s.kept).filter(|(_, k)| **k).map(|(n, _)| n.key).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&48));
    }

    /// Less density leaves out the weak notes first, keeping the downbeats; more fills
    /// in between, never on top of a note.
    #[test]
    fn density_thins_from_the_weakest_and_fills_gaps() {
        let notes: Vec<Note> = (0..16).map(|i| n(i * 240, 200, 60)).collect();
        let s = shape(&notes, &synth(), 0.15, NEUTRAL, 3840, 960);
        let left = s.kept.iter().filter(|k| **k).count();
        assert!((3..=8).contains(&left), "{left}");
        assert!(s.kept[0] && s.kept[4]);
        let held = vec![n(0, 3840, 48)];
        let s = shape(&held, &synth(), 1.0, NEUTRAL, 3840, 960);
        assert_eq!(s.added.len(), 15, "{:?}", s.added);
        let s = shape(&held, &synth(), 0.75, NEUTRAL, 3840, 960);
        assert_eq!(s.added.len(), 7);
        assert!(s.added.iter().all(|a| a.start % 480 == 0 && a.end() <= 3840));
    }

    /// More complexity adds octaves to a chord, and ghosts (not other drums) on a kit.
    #[test]
    fn richer_adds_octaves_or_ghosts() {
        let chords: Vec<Note> = (0..32).flat_map(|i| [n(i * 1920, 960, 60), n(i * 1920, 960, 67)]).collect();
        let s = shape(&chords, &synth(), NEUTRAL, 1.0, 3840, 960);
        assert_eq!(s.added.len(), 64);
        assert!(s.added.iter().all(|a| a.key == 79 || a.key == 48));
        let kit: Vec<Note> = (0..32).map(|i| n(i * 960 + 960, 100, 38)).collect();
        let s = shape(&kit, &Source::Kit(KitKind::Drums), NEUTRAL, 1.0, 3840, 960);
        assert!(!s.added.is_empty() && s.added.iter().all(|a| a.key == 38 && a.vel < 40));
    }

    /// Every style's every track, at the dials' extremes: never silenced, and nothing
    /// added out of range, past the end, or on top of another note.
    #[test]
    fn every_track_at_every_extreme() {
        for style in Style::ALL {
            let p = compose(&Recipe { style, ..Recipe::random(5) }, style.tempo(), style.meter().unwrap_or((4, 4)), 16);
            for t in &p.tracks {
                for (d, c) in [(0.0, 0.0), (1.0, 1.0), (0.0, 1.0), (1.0, 0.0), (0.3, 0.7)] {
                    let mut t = t.clone();
                    t.density = d;
                    t.complexity = c;
                    let out = played(&p, &t);
                    if !t.notes.is_empty() {
                        assert!(!out.is_empty(), "{style:?} {} {d} {c}", t.name);
                    }
                    // (what it adds: the written notes are the composer's)
                    let new: Vec<&Note> = out.iter().filter(|n| !t.notes.contains(n)).collect();
                    assert!(new.iter().all(|n| n.key <= 127 && n.len > 0 && n.end() <= p.end_tick()), "{style:?} {}: {new:?}", t.name);
                    for a in &new {
                        assert_eq!(out.iter().filter(|n| (n.start, n.key) == (a.start, a.key)).count(), 1, "{style:?} {}: {a:?}", t.name);
                    }
                }
            }
        }
    }
}
