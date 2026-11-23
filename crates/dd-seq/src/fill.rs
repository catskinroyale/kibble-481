//! Notes for an added track whose role the piece's style doesn't write, or for a piece
//! that wasn't composed (D28): a part for the role written over the harmony the piece
//! already has. Each bar's chord is read from the notes sounding in it (the low notes
//! counting most), a bar with none keeps the last; a piece with no notes at all rests
//! on its key's tonic chord. The part follows the sections, quieter in an Intro or Fade.

use dd_dsp::kit::drum;
use dd_dsp::rain::RainKind;

use crate::palette::Role;
use crate::project::{Ctrl, CtrlKind, Note, Part, Project, Source};
use crate::rng::Rng;

/// A chord: its root's pitch class and whether its third is minor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Chord {
    pub root: u8,
    pub minor: bool,
}

impl Chord {
    /// Root, third and fifth as pitch classes.
    fn tones(self) -> [u8; 3] {
        [self.root, (self.root + if self.minor { 3 } else { 4 }) % 12, (self.root + 7) % 12]
    }
}

/// The key nearest `centre` with pitch class `pc`.
fn near(pc: u8, centre: i32) -> u8 {
    let base = centre - (centre - pc as i32).rem_euclid(12);
    let k = if centre - base > 6 { base + 12 } else { base };
    k.clamp(0, 127) as u8
}

/// Every key with one of `pcs` from `lo` to `hi`.
fn keys_of(pcs: &[u8], lo: u8, hi: u8) -> Vec<u8> {
    (lo..=hi).filter(|k| pcs.contains(&(k % 12))).collect()
}

/// The chord of each bar, read from what sounds in it.
pub(crate) fn harmony(p: &Project) -> Vec<Chord> {
    let bar = p.bar_ticks().max(1);
    let bars = p.length_bars.max(p.bars()).max(1);
    let tonic = Chord { root: p.recipe.map_or(9, |r| r.root % 12), minor: true };
    let mut weights = vec![[0.0f64; 12]; bars as usize];
    let mut lows = vec![[0.0f64; 12]; bars as usize];
    for t in p.tracks.iter().filter(|t| !t.source.is_drums() && !matches!(t.source, Source::Kit(_))) {
        for n in &t.notes {
            let (a, z) = (n.start / bar, (n.end().saturating_sub(1)) / bar);
            for b in a..=z.min(bars - 1) {
                let lo = n.start.max(b * bar);
                let hi = n.end().min((b + 1) * bar);
                let w = (hi.saturating_sub(lo)) as f64 / bar as f64 * (0.3 + n.vel as f64 / 127.0);
                weights[b as usize][(n.key % 12) as usize] += w;
                if n.key < 55 {
                    lows[b as usize][(n.key % 12) as usize] += w;
                }
            }
        }
    }
    let mut out = Vec::with_capacity(bars as usize);
    let mut last = tonic;
    for (w, low) in weights.iter().zip(&lows) {
        let total: f64 = w.iter().sum();
        if total > 1e-6 {
            let score = |c: Chord| {
                let t = c.tones();
                let fit = w[t[0] as usize] + 0.8 * w[t[1] as usize] + 0.7 * w[t[2] as usize] + 0.6 * low[t[0] as usize];
                // what the chord leaves out counts against it; staying put a little for it
                fit - 0.35 * (total - t.iter().map(|&k| w[k as usize]).sum::<f64>()) + if c == last { 0.05 * total } else { 0.0 }
            };
            last = (0..12u8)
                .flat_map(|root| [Chord { root, minor: true }, Chord { root, minor: false }])
                .max_by(|a, b| score(*a).total_cmp(&score(*b)))
                .unwrap_or(last);
        }
        out.push(last);
    }
    out
}

/// How loud a part plays in each kind of section.
fn lift(part: Option<Part>) -> f64 {
    match part {
        Some(Part::Intro) => 0.7,
        Some(Part::Neon) | None => 0.85,
        Some(Part::Swell) => 1.0,
        Some(Part::NightShift) => 1.08,
        Some(Part::Fade) => 0.72,
    }
}

fn vel(v: f64) -> u8 {
    v.round().clamp(20.0, 120.0) as u8
}

/// The notes (and any swells) a track of `role` plays over the piece as it stands.
pub(crate) fn write(p: &Project, role: Role, seed: u64) -> (Vec<Note>, Vec<Ctrl>) {
    let mut rng = Rng::stream(seed, &format!("fill-{}", role.label()));
    let (bar, beat) = (p.bar_ticks(), p.beat_ticks());
    let beats = p.meter.0.max(1);
    let chords = harmony(p);
    let bars = chords.len() as u32;
    // each bar's section part
    let mut parts = vec![None; bars as usize];
    let mut at = 0;
    for b in p.arrangement() {
        for k in at..(at + b.bars).min(bars) {
            parts[k as usize] = b.part;
        }
        at += b.bars;
    }
    // runs of bars on one chord, held as one
    let mut spans: Vec<(u32, u32, Chord)> = Vec::new();
    for (i, c) in chords.iter().enumerate() {
        let i = i as u32;
        if let Some(s) = spans.last_mut()
            && s.2 == *c
            && s.1 == i
            && i - s.0 < 4
        {
            s.1 = i + 1;
        } else {
            spans.push((i, i + 1, *c));
        }
    }
    let (mut notes, mut ctrls) = (Vec::new(), Vec::new());
    let mut push = |start: u32, len: u32, key: u8, v: f64| notes.push(Note { start, len: len.max(1), key, vel: vel(v) });
    let part_at = |b: u32| parts.get(b as usize).copied().flatten();
    match role {
        Role::Pad | Role::Strings | Role::Brass => {
            for &(a, z, c) in &spans {
                let (start, len) = (a * bar, (z - a) * bar - beat / 4);
                let l = lift(part_at(a));
                let t = c.tones();
                let voicing = if role == Role::Brass {
                    // open: root, fifth, root above
                    let r = near(t[0], 50);
                    vec![r, r + 7, r + 12]
                } else {
                    let r = near(t[0], 55);
                    vec![r, near(t[1], r as i32 + 4), near(t[2], r as i32 + 7), r + 12]
                };
                for k in voicing {
                    push(start, len, k, if role == Role::Brass { 76.0 } else { 62.0 } * l);
                }
                if role == Role::Brass {
                    // a swell on each chord
                    for (f, v) in [(0.0, 40), (0.5, 100), (1.0, 50)] {
                        ctrls.push(Ctrl { tick: start + (len as f64 * f) as u32, kind: CtrlKind::Cc(11), value: v });
                    }
                }
            }
        }
        Role::Bass => {
            for (b, c) in chords.iter().enumerate() {
                let b = b as u32;
                let l = lift(part_at(b));
                let r = near(c.root, 40);
                if beats >= 4 && rng.chance(0.5) {
                    push(b * bar, beat * (beats / 2) - beat / 8, r, 92.0 * l);
                    push(b * bar + beat * (beats / 2), beat * (beats - beats / 2) - beat / 8, near(c.tones()[2], r as i32 + 5), 80.0 * l);
                } else {
                    push(b * bar, bar - beat / 8, r, 92.0 * l);
                }
            }
        }
        Role::Drone => {
            // the commonest root, held through each section, with its fifth past the first
            let mut count = [0u32; 12];
            for c in &chords {
                count[c.root as usize] += 1;
            }
            let tonic = (0..12u8).max_by_key(|k| count[*k as usize]).unwrap_or(9);
            let r = near(tonic, 40);
            let mut at = 0;
            for (i, b) in p.arrangement().iter().enumerate() {
                let (start, len) = (at * bar, (b.bars * bar).min(bars * bar - at * bar));
                at += b.bars;
                if len == 0 {
                    continue;
                }
                push(start, len, r, 80.0);
                if i > 0 || p.sections.is_empty() {
                    push(start, len, r + 7, 64.0);
                }
            }
            if p.sections.is_empty() {
                push(0, bars * bar, r, 80.0);
            }
        }
        Role::Keys => {
            for (b, c) in chords.iter().enumerate() {
                let b = b as u32;
                let l = lift(part_at(b));
                let t = c.tones();
                let r = near(t[0], 60);
                let voicing = [r, near(t[1], r as i32 + 4), near(t[2], r as i32 + 7)];
                for (j, k) in voicing.iter().enumerate() {
                    // a soft roll up the chord
                    push(b * bar + j as u32 * beat / 16, bar / if beats >= 4 { 2 } else { 1 } - beat / 4, *k, 70.0 * l);
                }
                if beats >= 4 {
                    for k in voicing {
                        push(b * bar + bar / 2, bar / 2 - beat / 4, k, 56.0 * l);
                    }
                }
            }
        }
        Role::Bells | Role::Pluck => {
            let (lo, hi) = if role == Role::Bells { (72, 91) } else { (60, 79) };
            let step = if role == Role::Bells { beat } else { beat / 2 };
            let per_bar = bar / step;
            let mut idx = 0usize;
            let mut up = true;
            for (b, c) in chords.iter().enumerate() {
                let b = b as u32;
                let l = lift(part_at(b));
                let ks = keys_of(&c.tones(), lo, hi);
                if ks.is_empty() {
                    continue;
                }
                for s in 0..per_bar {
                    let play = if role == Role::Bells { rng.chance(if s == 0 { 0.7 } else { 0.25 }) } else { rng.chance(0.85) };
                    if !play {
                        continue;
                    }
                    let key = if role == Role::Bells {
                        *rng.pick(&ks)
                    } else {
                        // up and down the chord
                        idx = idx.min(ks.len() - 1);
                        let k = ks[idx];
                        if up && idx + 1 >= ks.len() {
                            up = false;
                        } else if !up && idx == 0 {
                            up = true;
                        }
                        idx = if up { idx + 1 } else { idx.saturating_sub(1) };
                        k
                    };
                    let v = if role == Role::Bells { 70.0 } else { 62.0 } + rng.range(0, 12) as f64;
                    push(b * bar + s * step, step.max(beat / 2), key, v * l);
                }
            }
        }
        Role::Lead | Role::Answer => {
            // a phrase every two bars (the answer in the gaps between): chord tones,
            // moving by the nearest, a long note to end each
            let (lo, hi) = if role == Role::Lead { (64, 81) } else { (57, 74) };
            let mut key = near(chords[0].root, (lo as i32 + hi as i32) / 2);
            let first = if role == Role::Lead { 0 } else { 1 };
            let mut b = first;
            while b < bars {
                let l = lift(part_at(b));
                if part_at(b) == Some(Part::Intro) {
                    b += 2;
                    continue;
                }
                let c = chords[b as usize];
                let ks = keys_of(&c.tones(), lo, hi);
                let mut t = b * bar;
                let end = (b * bar + bar).min(bars * bar);
                while t < end && !ks.is_empty() {
                    let len = *rng.pick(&[beat, beat, beat * 2, beat / 2 + beat]);
                    let last = t + len >= end;
                    // the nearest chord tone a step or two away
                    let dir = if rng.chance(0.5) { 1 } else { -1 };
                    key = *ks
                        .iter()
                        .filter(|k| **k != key || ks.len() == 1)
                        .min_by_key(|k| ((**k as i32 - key as i32) - dir * 2).abs())
                        .unwrap_or(&key);
                    let len = if last { end - t + bar / 2 } else { len.min(end - t) };
                    push(t, len - beat / 8, key, 78.0 * l);
                    t += len;
                }
                b += 2;
            }
        }
        Role::Drums => {
            for (b, _) in chords.iter().enumerate() {
                let b = b as u32;
                let part = part_at(b);
                if matches!(part, Some(Part::Intro)) {
                    continue;
                }
                let l = lift(part);
                let at = b * bar;
                push(at, beat / 2, drum::KICK, 96.0 * l);
                if beats >= 4 {
                    push(at + beat * 2, beat / 2, drum::KICK, 76.0 * l);
                    push(at + beat, beat / 2, drum::SNARE, 70.0 * l);
                    push(at + beat * 3, beat / 2, drum::SNARE, 74.0 * l);
                } else if beats >= 2 {
                    push(at + beat * (beats - 1), beat / 2, drum::SNARE, 70.0 * l);
                }
                if !matches!(part, Some(Part::Fade)) {
                    for k in 0..beats * 2 {
                        if rng.chance(0.8) {
                            push(at + k * beat / 2, beat / 4, drum::HAT, (if k % 2 == 0 { 50.0 } else { 36.0 }) * l);
                        }
                    }
                }
            }
        }
        Role::City => {
            push(0, bars * bar, RainKind::ALL[0].key(), 84.0);
        }
    }
    notes.sort_by_key(|n| (n.start, n.key));
    (notes, ctrls)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Recipe, Style, add_track, compose};
    use crate::project::Track;

    /// The chord of a bar is read from what plays in it.
    #[test]
    fn reads_the_chords() {
        let mut p = Project { length_bars: 3, ..Project::default() };
        let bar = p.bar_ticks();
        let mut t = Track::new("Keys", Source::Synth(dd_dsp::patch::by_name("Rain Pad")), 0);
        for (b, keys) in [(0u32, [45u8, 60, 64]), (1, [41, 57, 60]), (2, [43, 59, 62])] {
            for k in keys {
                t.notes.push(Note { start: b * bar, len: bar, key: k, vel: 90 });
            }
        }
        p.tracks.push(t);
        let h = harmony(&p);
        assert_eq!(h, [Chord { root: 9, minor: true }, Chord { root: 5, minor: false }, Chord { root: 7, minor: false }]);
    }

    /// Every role added to every style gets notes, in the piece's chords; and to a
    /// piece that wasn't composed, or has no notes at all, too.
    #[test]
    fn every_added_track_has_notes() {
        for style in Style::ALL {
            let r = Recipe { style, ..Recipe::random(7) };
            let mut p = compose(&r, style.tempo(), style.meter().unwrap_or((4, 4)), 16);
            for role in Role::ALL {
                let (i, _) = add_track(&mut p, role, None);
                assert!(!p.tracks[i].notes.is_empty(), "{style:?} {role:?}");
                assert!(p.tracks[i].notes.iter().all(|n| n.end() <= p.end_tick()), "{style:?} {role:?}");
            }
        }
        let mut demo = crate::demo::rain_on_neon();
        demo.recipe = None;
        let (i, _) = add_track(&mut demo, Role::Bells, None);
        assert!(!demo.tracks[i].notes.is_empty());
        let mut empty = Project::default();
        for role in Role::ALL {
            let (i, _) = add_track(&mut empty, role, None);
            assert!(!empty.tracks[i].notes.is_empty(), "empty {role:?}");
        }
    }

    /// The written parts keep to the chords read.
    #[test]
    fn filled_parts_keep_to_the_chords() {
        let r = Recipe { style: Style::Derelict, ..Recipe::random(3) };
        let p = compose(&r, 48.0, (4, 4), 24);
        let h = harmony(&p);
        let bar = p.bar_ticks();
        for role in [Role::Keys, Role::Pluck, Role::Bells, Role::Lead, Role::Pad] {
            let (notes, _) = write(&p, role, 11);
            for n in &notes {
                let c = h[(n.start / bar) as usize];
                assert!(c.tones().contains(&(n.key % 12)), "{role:?} {n:?} over {c:?}");
            }
        }
    }
}
