//! A track's density and complexity (D31): two dials on each track that change what it
//! plays without touching what is written. Straight up (0.5) plays the notes as written.
//!
//! **Density** turned down leaves out the weakest notes first (a chord counts as one: off
//! the beat before on it, short and soft before long and loud), down to the strongest
//! sixth or so, and near the bottom lets what's left ring on into the space (D36); turned
//! up it fills the gaps after notes (a held note struck again, a hit echoed), on the
//! quavers first and then the semiquavers, each softer, more and more of the chords
//! broken into arpeggios rather than struck whole (D36).
//!
//! **Complexity** turned down plays fewer notes of each chord (the bass and the top kept
//! longest) and fewer of the different notes in each bar's arpeggio or line (the lowest
//! and the most played kept longest), and near the bottom bares chords to open fifths
//! (D36); turned up it adds the chord's top an octave higher, then its bass an octave
//! lower, and doubles some single notes an octave up; further up (D36) it colours chords
//! with the next note of the scale above their top (sevenths and ninths), harmonises a
//! melody a third below in the scale, and fills its leaps with passing notes. On a drum
//! kit it keeps the lowest drums (the kick, then the snare) and adds soft ghost notes.
//!
//! The scale is the piece's (`scale_of`): its key and mode if it was composed, else the
//! seven pitch classes it uses most.
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

/// The piece's scale as pitch classes: its recipe's key and mode, else the seven pitch
/// classes its pitched notes use most (all twelve if it has none).
pub fn scale_of(p: &Project) -> [bool; 12] {
    let mut sc = [false; 12];
    if let Some(r) = p.recipe {
        for st in r.mode.steps() {
            sc[((r.root + st) % 12) as usize] = true;
        }
        return sc;
    }
    let mut count = [0usize; 12];
    for t in p.tracks.iter().filter(|t| kind(&t.source) == Kind::Pitched) {
        for n in &t.notes {
            count[(n.key % 12) as usize] += 1;
        }
    }
    if count.iter().all(|c| *c == 0) {
        return [true; 12];
    }
    let mut order: Vec<usize> = (0..12).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(count[i]), i));
    for &i in order.iter().take(7).filter(|&&i| count[i] > 0) {
        sc[i] = true;
    }
    sc
}

/// The pitch `steps` notes of the scale above `key` (below if negative), if in range.
fn along(key: u8, steps: i32, sc: &[bool; 12]) -> Option<u8> {
    let mut k = key as i32;
    let mut left = steps.abs();
    let dir = steps.signum();
    while left > 0 {
        k += dir;
        if !(0..=127).contains(&k) {
            return None;
        }
        if sc[(k % 12) as usize] {
            left -= 1;
        }
    }
    Some(k as u8)
}

/// What `density` and `complexity` do to `notes` (in any order), in bars of `bar` and
/// beats of `beat` ticks, in the scale `sc` (pitch classes).
pub fn shape(notes: &[Note], source: &Source, density: f32, complexity: f32, bar: u32, beat: u32, sc: &[bool; 12]) -> Shaped {
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
            // near the bottom, bared to an open fifth: the bass and the fifth above it,
            // in the top's octave, the rest gone
            let (lo, hi) = (by_pitch[0], by_pitch[by_pitch.len() - 1]);
            let bare = ((0.3 - c) / 0.3).clamp(0.0, 1.0);
            if kind == Kind::Pitched && hi != lo && kept[lo] && hash(notes[lo].start, notes[lo].key as u32, 4) < bare {
                let mut fifth = notes[lo].key as i32 + 7;
                while fifth + 12 <= notes[hi].key as i32 {
                    fifth += 12;
                }
                if fifth <= 127 {
                    for &i in &by_pitch[1..] {
                        kept[i] = false;
                    }
                    match by_pitch[1..].iter().find(|&&i| notes[i].key as i32 == fifth) {
                        Some(&i) => kept[i] = true,
                        None => added.push(Note { key: fifth as u8, ..notes[hi] }),
                    }
                }
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
        // a melody (single notes, mostly above middle C) is harmonised; a bass line isn't
        let mut single_keys: Vec<u8> = chords.iter().filter(|ch| ch.len() == 1).map(|ch| notes[ch[0]].key).collect();
        single_keys.sort_unstable();
        let melodic = single_keys.get(single_keys.len() / 2).is_some_and(|k| *k >= 57);
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
                    // a colour: the scale's next third above the top (a seventh, a ninth)
                    let colour = ((more - 0.25) / 0.75).clamp(0.0, 1.0);
                    if hash(notes[hi].start, notes[hi].key as u32, 5) < colour
                        && let Some(k) = along(notes[hi].key, 2, sc)
                    {
                        added.push(Note { key: k, vel: scale(notes[hi].vel, 0.7), ..notes[hi] });
                    }
                }
                Kind::Pitched => {
                    let harmony = ((more - 0.2) * 1.1).clamp(0.0, 0.85);
                    if h < 0.6 * more && notes[lo].key <= 115 {
                        added.push(Note { key: notes[lo].key + 12, vel: scale(notes[lo].vel, 0.6), ..notes[lo] });
                    } else if melodic
                        && hash(notes[lo].start, notes[lo].key as u32, 6) < harmony
                        && let Some(k) = along(notes[lo].key, -2, sc)
                    {
                        // a third below, in the scale
                        added.push(Note { key: k, vel: scale(notes[lo].vel, 0.7), ..notes[lo] });
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
        // passing notes through a line's leaps of a third or more, where there's time
        let passing = ((more - 0.4) / 0.6).clamp(0.0, 1.0);
        if kind == Kind::Pitched && passing > 0.0 {
            let s16 = (beat / 4).max(1);
            let singles: Vec<Note> = chords.iter().filter(|ch| ch.len() == 1).map(|ch| notes[ch[0]]).collect();
            for w in singles.windows(2) {
                let (a, b) = (w[0], w[1]);
                let gap = b.start.saturating_sub(a.start);
                let leap = b.key as i32 - a.key as i32;
                if gap < beat || leap.abs() < 3 || hash(a.start, a.key as u32, 7) >= passing {
                    continue;
                }
                let mid = (a.start + gap / 2) / s16 * s16;
                if mid <= a.start || mid >= b.start {
                    continue;
                }
                // the note of the scale between them nearest halfway
                let target = (a.key as i32 + b.key as i32) as f64 / 2.0;
                let between = (a.key.min(b.key) + 1..a.key.max(b.key)).filter(|k| sc[(*k % 12) as usize]);
                if let Some(k) = between.min_by(|x, y| (*x as f64 - target).abs().total_cmp(&(*y as f64 - target).abs())) {
                    added.push(Note { start: mid, len: ((b.start - mid) * 9 / 10).max(1), key: k, vel: scale(a.vel, 0.8) });
                }
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
        // near the bottom, what's left rings on into the space (up to two bars)
        let ring = ((0.3 - d) / 0.3).clamp(0.0, 1.0);
        if kind == Kind::Pitched && ring > 0.0 {
            let mut left: Vec<usize> = (0..chords.len()).filter(|&k| chords[k].iter().any(|&i| kept[i])).collect();
            left.sort_by_key(|&k| notes[chords[k][0]].start);
            for (j, &k) in left.iter().enumerate() {
                let start = notes[chords[k][0]].start;
                let until = left.get(j + 1).map_or(start, |&n| notes[chords[n][0]].start);
                if hash(start, notes[chords[k][0]].key as u32, 8) >= ring {
                    continue;
                }
                let here: Vec<usize> = chords[k].iter().copied().filter(|&i| kept[i]).collect();
                for i in here {
                    let len = (until.saturating_sub(notes[i].start)).min(2 * bar);
                    if len > notes[i].len + beat {
                        kept[i] = false;
                        added.push(Note { len, ..notes[i] });
                    }
                }
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
            let mut fade: f64 = 1.0;
            // more and more chords broken into arpeggios: up, down, or up and down
            let mut by_pitch = notes_here.clone();
            by_pitch.sort_by_key(|m| m.key);
            let ah = hash(start, by_pitch[0].key as u32, 9);
            let arp = by_pitch.len() >= 2 && ah < more;
            let shape_of = (ah * 3.0 / more.max(1e-6)) as usize % 3;
            let mut struck = 0usize;
            while t + s / 2 < next {
                let w = weight(t, bar, beat);
                let p = if w >= 2.0 { (2.0 * more).min(1.0) } else { (2.0 * more - 1.0).max(0.0) };
                if hash(t, notes_here[0].key as u32, 3) < p {
                    fade *= 0.85;
                    if arp {
                        let n = by_pitch.len();
                        let i = struck + 1;
                        let idx = match shape_of {
                            0 => i % n,
                            1 => n - 1 - i % n,
                            _ => {
                                let cyc = 2 * n - 2;
                                let x = i % cyc.max(1);
                                if x < n { x } else { cyc - x }
                            }
                        };
                        let m = by_pitch[idx.min(n - 1)];
                        let len = (next - t).min(2 * s).max(1);
                        added.push(Note { start: t, len, vel: scale(m.vel, 0.8 * fade.max(0.6)), ..m });
                    } else {
                        for m in &notes_here {
                            let len = if t < m.end() { m.end() - t } else { m.len.min(s) };
                            let len = len.min(next - t).max(1);
                            added.push(Note { start: t, len, vel: scale(m.vel, 0.75 * fade), ..*m });
                        }
                    }
                    struck += 1;
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
    let s = shape(&t.notes, &t.source, t.density, t.complexity, p.bar_ticks(), p.beat_ticks(), &scale_of(p));
    let mut out: Vec<Note> = t.notes.iter().zip(&s.kept).filter(|(_, k)| **k).map(|(n, _)| *n).collect();
    out.extend(s.added);
    out.sort_by_key(|n| (n.start, n.key));
    Cow::Owned(out)
}

/// Turn each track's dials somewhere new for a new seed (D36), so New notes brings a
/// new texture as well as new notes: each dial stays straight up a third of the time,
/// else turns anywhere from well down to well up. The drones and the beds of ambience
/// are left as they are (they're the style's ground).
pub fn vary(p: &mut Project, seed: u64) {
    let mut rng = crate::rng::Rng::stream(seed, "dials");
    for t in p.tracks.iter_mut() {
        let ground = t.ambience || t.role == Some(crate::palette::Role::Drone) || kind(&t.source) == Kind::Effects;
        let dial = |rng: &mut crate::rng::Rng| if rng.chance(0.34) { NEUTRAL } else { rng.range_f(0.15, 0.88) as f32 };
        let (d, c) = (dial(&mut rng), dial(&mut rng));
        if !ground {
            (t.density, t.complexity) = (d, c);
        } else {
            (t.density, t.complexity) = (NEUTRAL, NEUTRAL);
        }
    }
}

/// A freshly written piece keeps the dials of the tracks (by name) of the one it
/// replaces.
pub fn keep_dials(new: &mut Project, old: &Project) {
    for t in new.tracks.iter_mut() {
        if let Some(o) = old.tracks.iter().find(|o| o.name == t.name) {
            (t.density, t.complexity) = (o.density, o.complexity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Recipe, Style, compose};

    const C_MAJOR: [bool; 12] = [true, false, true, false, true, true, false, true, false, true, false, true];

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
        let s = shape(&notes, &synth(), NEUTRAL, NEUTRAL, 3840, 960, &C_MAJOR);
        assert!(s.kept.iter().all(|k| *k) && s.added.is_empty());
    }

    /// Less complexity keeps the bass and top of a chord first, then at the bottom just
    /// the bass and its fifth: an open fifth.
    #[test]
    fn simpler_chords_keep_their_outer_notes() {
        let notes = vec![n(0, 960, 60), n(0, 960, 64), n(0, 960, 67), n(0, 960, 72)];
        let s = shape(&notes, &synth(), NEUTRAL, 0.31, 3840, 960, &C_MAJOR);
        assert_eq!(s.kept, [true, false, false, true]);
        let s = shape(&notes, &synth(), NEUTRAL, 0.0, 3840, 960, &C_MAJOR);
        assert_eq!(s.kept, [true, false, true, false]);
        assert!(s.added.is_empty());
    }

    /// Less complexity plays fewer of an arpeggio's notes, the lowest kept.
    #[test]
    fn simpler_arpeggios_use_fewer_notes() {
        let notes: Vec<Note> = (0..16).map(|i| n(i * 240, 240, [48, 55, 60, 64][i as usize % 4])).collect();
        let s = shape(&notes, &synth(), NEUTRAL, 0.25, 3840, 960, &C_MAJOR);
        let keys: std::collections::BTreeSet<u8> = notes.iter().zip(&s.kept).filter(|(_, k)| **k).map(|(n, _)| n.key).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&48));
    }

    /// Less density leaves out the weak notes first, keeping the downbeats; more fills
    /// in between, never on top of a note.
    #[test]
    fn density_thins_from_the_weakest_and_fills_gaps() {
        let notes: Vec<Note> = (0..16).map(|i| n(i * 240, 200, 60)).collect();
        let s = shape(&notes, &synth(), 0.15, NEUTRAL, 3840, 960, &C_MAJOR);
        let left = s.kept.iter().filter(|k| **k).count();
        assert!((3..=8).contains(&left), "{left}");
        assert!(s.kept[0] && s.kept[4]);
        let held = vec![n(0, 3840, 48)];
        let s = shape(&held, &synth(), 1.0, NEUTRAL, 3840, 960, &C_MAJOR);
        assert_eq!(s.added.len(), 15, "{:?}", s.added);
        let s = shape(&held, &synth(), 0.75, NEUTRAL, 3840, 960, &C_MAJOR);
        assert_eq!(s.added.len(), 7);
        assert!(s.added.iter().all(|a| a.start % 480 == 0 && a.end() <= 3840));
    }

    /// More complexity adds octaves to a chord, and ghosts (not other drums) on a kit.
    #[test]
    fn richer_adds_octaves_or_ghosts() {
        let chords: Vec<Note> = (0..32).flat_map(|i| [n(i * 1920, 960, 60), n(i * 1920, 960, 67)]).collect();
        let s = shape(&chords, &synth(), NEUTRAL, 1.0, 3840, 960, &C_MAJOR);
        // both octaves on every chord, and the colour (B above the top G) on every one
        assert_eq!(s.added.iter().filter(|a| a.key == 79 || a.key == 48).count(), 64);
        assert_eq!(s.added.iter().filter(|a| a.key == 71).count(), 32);
        assert_eq!(s.added.len(), 96);
        let kit: Vec<Note> = (0..32).map(|i| n(i * 960 + 960, 100, 38)).collect();
        let s = shape(&kit, &Source::Kit(KitKind::Drums), NEUTRAL, 1.0, 3840, 960, &C_MAJOR);
        assert!(!s.added.is_empty() && s.added.iter().all(|a| a.key == 38 && a.vel < 40));
    }

    /// Near the bottom, complexity bares chords to open fifths over their bass.
    #[test]
    fn barest_chords_are_open_fifths() {
        let chords: Vec<Note> = (0..32).flat_map(|i| [n(i * 1920, 960, 48), n(i * 1920, 960, 52), n(i * 1920, 960, 64)]).collect();
        let s = shape(&chords, &synth(), NEUTRAL, 0.12, 3840, 960, &C_MAJOR);
        let fifths = s.added.iter().filter(|a| a.key == 55 || a.key == 67).count();
        assert!(fifths >= 12, "{fifths}");
        // (a bared chord is its bass and the fifth: nothing else of it plays)
        for a in s.added.iter().filter(|a| a.key == 67) {
            for x in chords.iter().enumerate().filter(|(_, x)| x.start == a.start && x.key != 48) {
                assert!(!s.kept[x.0]);
            }
        }
    }

    /// Further up, a melody gets thirds below and passing notes in its leaps, all in the
    /// scale; a bass line gets no harmony.
    #[test]
    fn richest_melodies_are_harmonised_and_filled() {
        let tune: Vec<Note> = (0..24).map(|i| n(i * 960, 900, [72, 67, 76, 72, 79, 74][i as usize % 6])).collect();
        let s = shape(&tune, &synth(), NEUTRAL, 1.0, 3840, 960, &C_MAJOR);
        assert!(s.added.iter().all(|a| C_MAJOR[(a.key % 12) as usize]));
        assert!(s.added.iter().any(|a| a.start % 960 != 0), "passing notes between the beats");
        assert!(s.added.iter().any(|a| tune.iter().any(|t| t.start == a.start && t.key > a.key && t.key - a.key <= 4)), "thirds below");
        let bass: Vec<Note> = (0..24).map(|i| n(i * 960, 900, [36, 43, 40, 45][i as usize % 4])).collect();
        let s = shape(&bass, &synth(), NEUTRAL, 1.0, 3840, 960, &C_MAJOR);
        assert!(!s.added.iter().any(|a| bass.iter().any(|t| t.start == a.start && t.key > a.key)));
    }

    /// Density right up breaks held chords into arpeggios; right down lets the few notes
    /// left ring on.
    #[test]
    fn densest_arpeggiates_and_sparsest_rings() {
        let chords: Vec<Note> = (0..16).flat_map(|i| [n(i * 3840, 3840, 60), n(i * 3840, 3840, 64), n(i * 3840, 3840, 67)]).collect();
        let s = shape(&chords, &synth(), 1.0, NEUTRAL, 3840, 960, &C_MAJOR);
        let mut by_tick = std::collections::BTreeMap::new();
        for a in &s.added {
            *by_tick.entry(a.start).or_insert(0) += 1;
        }
        assert!(by_tick.values().filter(|c| **c == 1).count() > by_tick.len() / 2, "mostly one note at a time");
        let short: Vec<Note> = (0..32).map(|i| n(i * 480, 200, 60 + (i % 3) as u8 * 4)).collect();
        let s = shape(&short, &synth(), 0.0, NEUTRAL, 3840, 960, &C_MAJOR);
        assert!(s.added.iter().any(|a| a.len > 960), "{:?}", s.added);
    }

    /// The dials make real differences: across the corners each setting plays a
    /// different set of pitches or rhythms from the others.
    #[test]
    fn the_dials_vary_a_style_audibly() {
        use crate::compose::{Recipe, Style, compose};
        let p = compose(&Recipe { style: Style::Canopy, ..Recipe::random(1) }, 60.0, (4, 4), 32);
        let mut seen = std::collections::BTreeSet::new();
        for (d, c) in [(0.5, 0.5), (0.1, 0.1), (0.9, 0.9), (0.1, 0.9), (0.9, 0.1), (0.5, 0.95), (0.95, 0.5)] {
            let mut sig = Vec::new();
            for t in p.tracks.iter().filter(|t| !t.ambience) {
                let mut t = t.clone();
                (t.density, t.complexity) = (d, c);
                sig.extend(played(&p, &t).iter().map(|n| (n.start, n.key)));
            }
            seen.insert(sig);
        }
        assert_eq!(seen.len(), 7);
    }

    /// New notes turn the dials: different seeds, different settings; the drones and
    /// beds stay straight up.
    #[test]
    fn new_seeds_turn_the_dials() {
        use crate::compose::{Recipe, Style, compose};
        let p = compose(&Recipe { style: Style::Tarn, ..Recipe::random(1) }, 54.0, (4, 4), 32);
        let mut settings = std::collections::BTreeSet::new();
        for seed in 0..6 {
            let mut q = p.clone();
            vary(&mut q, seed);
            for t in q.tracks.iter().filter(|t| t.ambience || t.role == Some(crate::palette::Role::Drone)) {
                assert!(is_neutral(&t.density) && is_neutral(&t.complexity), "{}", t.name);
            }
            settings.insert(q.tracks.iter().map(|t| ((t.density * 100.0) as i32, (t.complexity * 100.0) as i32)).collect::<Vec<_>>());
        }
        assert_eq!(settings.len(), 6);
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
