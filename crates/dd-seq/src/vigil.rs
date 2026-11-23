//! **Vigil** (D28): Overcity watched over gently. The same theme, chords, form and
//! great swell, written by Overcity's own composer, then given to other instruments:
//! the hollow fifths go to soft sampled French horns instead of the synth brass; the
//! choir or strings that held the chords become a second, slower drone a fifth wide
//! over the first; and the rippling glass is played on a grand piano.
//!
//! Its own random stream (`vigil`) chooses the second drone, so an Overcity recipe in
//! Vigil plays the same notes, only on other instruments.

use crate::compose::{Recipe, fingerprint};
use crate::palette::{Alt, Role, alternatives, apply, same_sound};
use crate::project::{CtrlKind, HIGH_CUT_OFF, Note, Project};
use crate::rng::Rng;

/// Sounds added for Vigil: the other styles leave them out of their choosing.
pub(crate) const VIGIL_SOUNDS: [&str; 1] = ["Grand Piano"];

/// The drones the second may be, how often, and its fader: measured where it plays,
/// an octave and more above the first (the trims are measured lower, and these sounds
/// differ by as much as 10 dB up there).
const SECOND_DRONES: [(&str, f64, f32); 4] =
    [("Glass Drone", 3.0, -5.5), ("Choir Drone", 3.0, -7.0), ("Ozone Drone", 1.5, -3.5), ("Bowed Glass", 1.0, 3.0)];

const ADJECTIVES: [&str; 10] = ["Quiet", "Long", "Patient", "High", "Sleepless", "Still", "Gentle", "Upper", "Lamplit", "Last"];
const NOUNS: [&str; 10] = ["Vigil", "Watch", "Lanterns", "Rooftops", "Spires", "Heights", "Skyline", "Hours", "Windows", "Nocturne"];

fn named(role: Role, label: &str) -> Alt {
    alternatives(role).into_iter().find(|a| a.label == label).expect("a bundled sound")
}

/// Softer: velocities scaled to `k`, a floor kept so nothing vanishes.
fn soften(v: u8, k: f32) -> u8 {
    ((v as f32 * k) as u8).clamp(24, 127)
}

/// Overcity's piece `p` (written with `r` as Overcity) given Vigil's instruments.
pub(crate) fn adapt(p: &mut Project, r: &Recipe) {
    let mut names = Rng::stream(r.seed, "names");
    // (as every style: the first draw once chose the section names)
    let _ = names.next_u64();
    p.title = format!("{} {}", names.pick(&ADJECTIVES), names.pick(&NOUNS));
    let mut rng = Rng::stream(r.seed, "vigil");
    let beat = p.beat_ticks();
    let find = |p: &Project, roles: &[Role]| p.tracks.iter().position(|t| t.role.is_some_and(|x| roles.contains(&x)));

    // the horns: sampled French horns, played softly and kept mellow
    if let Some(i) = find(p, &[Role::Brass]) {
        let t = &mut p.tracks[i];
        apply(t, Role::Brass, &named(Role::Brass, "French Horns"));
        t.name = "Horns".into();
        t.strip.ensemble = false;
        t.strip.echo = 0.0;
        t.strip.reverb = 0.7;
        t.strip.high_cut = 3500.0;
        // (sampled horns under the synth brass's swells are quieter: brought up to sit
        // a little under where the brass did)
        t.strip.gain_db += 5.0;
        for n in t.notes.iter_mut() {
            n.vel = soften(n.vel, 0.85);
        }
        for c in t.ctrls.iter_mut().filter(|c| c.kind == CtrlKind::Cc(11)) {
            c.value = (c.value as f32 * 0.9) as i16;
        }
    }

    // the drones: the first as Overcity holds it, the second following the chords
    let first = find(p, &[Role::Drone]);
    let main_source = first.map(|i| {
        p.tracks[i].name = "Drone 1".into();
        p.tracks[i].source.clone()
    });
    if let Some(i) = find(p, &[Role::Pad, Role::Strings]) {
        let options: Vec<(Alt, f64, f32)> = SECOND_DRONES
            .iter()
            .map(|(label, w, db)| (named(Role::Drone, label), *w, *db))
            .filter(|(a, ..)| main_source.as_ref().is_none_or(|m| !same_sound(m, &a.source)))
            .collect();
        let w: Vec<f64> = options.iter().map(|o| o.1).collect();
        let (alt, _, gain_db) = options[rng.weighted(&w)].clone();
        let t = &mut p.tracks[i];
        t.notes = drone_line(&t.notes, beat, 50);
        t.ctrls.retain(|c| c.kind == CtrlKind::Cc(11));
        t.source = alt.source.clone();
        t.role = Some(Role::Drone);
        t.name = "Drone 2".into();
        // where the choir held the chords: well above the first drone, felt as harmony
        t.strip.gain_db = gain_db;
        (t.strip.reverb, t.strip.echo, t.strip.width, t.strip.pan) = (0.7, 0.0, 1.6, 0.0);
        t.strip.ensemble = false;
        t.strip.high_cut = 7000.0;
    }

    // the piano where the glass rippled
    if let Some(i) = find(p, &[Role::Pluck]) {
        let t = &mut p.tracks[i];
        apply(t, Role::Pluck, &named(Role::Pluck, "Grand Piano"));
        t.name = "Piano".into();
        (t.strip.reverb, t.strip.echo, t.strip.pan) = (0.6, 0.25, 0.25);
        // (a sampled piano struck as softly as the glass was is far quieter)
        t.strip.gain_db += 8.0;
        t.strip.high_cut = HIGH_CUT_OFF;
        for n in t.notes.iter_mut() {
            n.vel = n.vel.saturating_add(10).min(110);
        }
    }
    p.recipe = Some(*r);
    p.composed = fingerprint(p);
}

/// A drone that follows the chords: each chord's lowest note and its fifth (else its
/// next note), moved together into the octave from `low`, and held on through the
/// chords that keep them.
fn drone_line(notes: &[Note], beat: u32, low: u8) -> Vec<Note> {
    let mut starts: Vec<u32> = notes.iter().map(|n| n.start).collect();
    starts.sort_unstable();
    starts.dedup();
    let mut out: Vec<Note> = Vec::new();
    for s in starts {
        let mut chord: Vec<&Note> = notes.iter().filter(|n| n.start == s).collect();
        chord.sort_by_key(|n| n.key);
        let Some(lowest) = chord.first() else { continue };
        let len = chord.iter().map(|n| n.len).max().unwrap_or(beat);
        let root = lowest.key as i32;
        let second = chord.iter().find(|n| n.key as i32 == root + 7).or(chord.get(1)).map(|n| n.key as i32);
        let shift = (low as i32 - root).div_euclid(12) * 12 + if (low as i32 - root).rem_euclid(12) == 0 { 0 } else { 12 };
        for (k, vel) in [(Some(root), 96u8), (second, 82)] {
            let Some(k) = k else { continue };
            let key = (k + shift).clamp(0, 127) as u8;
            match out.iter_mut().rev().find(|n| n.key == key && n.end() + beat >= s && n.start < s) {
                Some(n) => n.len = (s + len).max(n.end()) - n.start,
                None => out.push(Note { start: s, len, key, vel }),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Style, compose};

    fn piece(seed: u64, bars: u32) -> Project {
        compose(&Recipe { style: Style::Vigil, ..Recipe::random(seed) }, Style::Vigil.tempo(), (4, 4), bars)
    }

    /// Overcity's notes on Vigil's instruments: soft horns, two drones, a grand piano,
    /// and no choir, strings or synth brass.
    #[test]
    fn vigil_is_overcity_on_other_instruments() {
        for seed in 0..12 {
            let p = piece(seed, 48);
            let o = compose(&Recipe { style: Style::Overcity, ..Recipe::random(seed) }, 60.0, (4, 4), 48);
            assert_eq!(p.recipe.map(|r| r.style), Some(Style::Vigil));
            assert_eq!(p.sections, o.sections);
            let drones = p.tracks.iter().filter(|t| t.role == Some(Role::Drone)).count();
            assert_eq!(drones, 2, "seed {seed}: {:?}", p.tracks.iter().map(|t| &t.name).collect::<Vec<_>>());
            assert!(!p.tracks.iter().any(|t| matches!(t.role, Some(Role::Pad | Role::Strings))), "seed {seed}");
            let horns = p.tracks.iter().find(|t| t.name == "Horns").expect("horns");
            assert!(same_sound(&horns.source, &named(Role::Brass, "French Horns").source));
            if let Some(piano) = p.tracks.iter().find(|t| t.role == Some(Role::Pluck)) {
                assert!(same_sound(&piano.source, &named(Role::Pluck, "Grand Piano").source));
                assert_eq!(piano.name, "Piano");
            }
            // the theme is Overcity's own
            let lead = |q: &Project| q.tracks.iter().find(|t| t.role == Some(Role::Lead)).map(|t| t.notes.clone());
            assert_eq!(lead(&p), lead(&o), "seed {seed}");
            // the second drone holds, and never jumps more than a fifth from the first's octave
            let d2 = p.tracks.iter().find(|t| t.name == "Drone 2").unwrap();
            assert!(d2.notes.iter().all(|n| (50..=69).contains(&n.key)), "seed {seed}");
            assert!(d2.notes.iter().all(|n| n.len >= p.beat_ticks()), "seed {seed}");
            assert!(p.untouched());
        }
    }

    /// A chord's lowest note and fifth become one long note while the chords keep them.
    #[test]
    fn the_second_drone_holds_through_shared_chords() {
        let n = |start, key| Note { start, len: 900, key, vel: 60 };
        let pad = [n(0, 57), n(0, 64), n(0, 72), n(960, 57), n(960, 64), n(960, 69), n(1920, 53), n(1920, 60)];
        let d = drone_line(&pad, 960, 50);
        assert_eq!(d.iter().map(|x| (x.start, x.key)).collect::<Vec<_>>(), [(0, 57), (0, 64), (1920, 53), (1920, 60)]);
        assert_eq!(d[0].len, 960 + 900);
    }
}
