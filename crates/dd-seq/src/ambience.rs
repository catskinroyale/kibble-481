//! The background ambience (D36): field sounds under the music (rain, birdsong,
//! crickets, a breeze, tape hiss, a record's crackle, a far-off radio, machinery) as
//! beds rather than parts. Each bed is a track of its own marked `ambience`, holding one
//! sound from the City FX kit, so the engine plays and mixes it like any other track,
//! but the app shows the beds apart from the music, each with a switch and a level.
//!
//! The composer still writes its field sounds on its City tracks; `adopt` then splits
//! them into one bed per sound (keeping each track's mix and expression, so the piece
//! sounds just as it did), leaving any score effects (risers, booms, metal, groans) on
//! the track as music. Files saved before beds existed are adopted as they open.

use dd_dsp::kit::fx;
use dd_dsp::rain::RainKind;
use dd_dsp::sampler::KitKind;

use crate::palette::Role;
use crate::project::{NEON, Note, Project, Source, Strip, Track};

/// One kind of bed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bed {
    /// Its key in the City FX kit (for rain, the key of the first kind).
    pub key: u8,
    pub name: &'static str,
    pub help: &'static str,
    /// Its level when added by hand, in dB.
    pub gain_db: f32,
}

const fn bed(key: u8, name: &'static str, help: &'static str, gain_db: f32) -> Bed {
    Bed { key, name, help, gain_db }
}

/// Every bed, in the order the app lists them.
pub const BEDS: [Bed; 14] = [
    bed(fx::RAIN, "Rain", "Rain falling, from one of nine recordings", -6.0),
    bed(fx::THUNDER, "Thunder", "Thunder rolling near the start and before each later section", -6.0),
    bed(fx::HUM, "Machinery hum", "A low hum of machinery", -8.0),
    bed(fx::NEON, "Neon buzz", "A neon sign buzzing, from the second section to the last", -10.0),
    bed(fx::TAPE_HISS, "Tape hiss", "The faint hiss of old tape", -14.0),
    bed(fx::CRACKLE, "Vinyl crackle", "The dust and crackle of a worn record", -10.0),
    bed(fx::RADIO, "Radio static", "A radio far off between stations", -10.0),
    bed(fx::BIRDS, "Birdsong", "Birds singing in the trees", -8.0),
    bed(fx::PLAYGROUND, "Playground", "Children at play across the field", -10.0),
    bed(fx::CRICKETS, "Crickets", "Crickets in the grass", -6.0),
    bed(fx::BREEZE, "Breeze", "A breeze through the trees", -5.0),
    bed(fx::ENGINE, "Engine throb", "A ship's engines throbbing", -8.0),
    bed(fx::PUMP, "Life support", "A life-support pump breathing", -8.0),
    bed(fx::TICKING, "Ticking", "Something ticking in the dark", -10.0),
];

/// The bed a key of the City FX kit belongs to, if it's a bed's (every rain is Rain's).
pub fn bed_of(key: u8) -> Option<&'static Bed> {
    if RainKind::of_key(key).is_some() {
        return Some(&BEDS[0]);
    }
    BEDS.iter().find(|b| b.key == key)
}

/// The bed a track is: its first note's (a bed holds one sound).
pub fn bed_of_track(t: &Track) -> Option<&'static Bed> {
    t.notes.first().and_then(|n| bed_of(n.key))
}

/// The beds' tracks, by index.
pub fn beds(p: &Project) -> Vec<usize> {
    (0..p.tracks.len()).filter(|&i| p.tracks[i].ambience).collect()
}

/// The index of the bed of this kind, if the piece has it.
pub fn find(p: &Project, b: &Bed) -> Option<usize> {
    beds(p).into_iter().find(|&i| bed_of_track(&p.tracks[i]).is_some_and(|x| x.name == b.name))
}

fn is_city(t: &Track) -> bool {
    matches!(t.source, Source::Kit(KitKind::CityFx))
}

/// Split the field sounds out of the piece's City FX tracks into beds, one a sound,
/// keeping each track's mix, expression and bar levels, so the piece sounds as it did.
/// What's left on a track (score effects) stays there, as music; a track left empty
/// goes. A track that holds one bed already is just marked. Beds go after the music.
pub fn adopt(p: &mut Project) {
    let mut beds_out: Vec<Track> = Vec::new();
    let mut keep: Vec<Track> = Vec::new();
    for t in std::mem::take(&mut p.tracks) {
        if t.ambience || !is_city(&t) {
            if t.ambience {
                beds_out.push(t);
            } else {
                keep.push(t);
            }
            continue;
        }
        // the bed each note belongs to, in order of first appearance
        let mut groups: Vec<(&'static Bed, Vec<Note>)> = Vec::new();
        let mut rest: Vec<Note> = Vec::new();
        for n in &t.notes {
            match bed_of(n.key) {
                Some(b) => match groups.iter_mut().find(|g| g.0.name == b.name) {
                    Some(g) => g.1.push(*n),
                    None => groups.push((b, vec![*n])),
                },
                None => rest.push(*n),
            }
        }
        if groups.is_empty() {
            keep.push(t);
            continue;
        }
        if rest.is_empty() && groups.len() == 1 {
            let name = if t.name == "City" { groups[0].0.name.to_string() } else { t.name.clone() };
            beds_out.push(Track { name, ambience: true, ..t });
            continue;
        }
        for (b, notes) in groups {
            beds_out.push(Track {
                name: b.name.to_string(),
                notes,
                ambience: true,
                route: None,
                density: crate::shape::NEUTRAL,
                complexity: crate::shape::NEUTRAL,
                added: false,
                ..t.clone()
            });
        }
        if !rest.is_empty() {
            let name = if t.name == "City" { "Effects".to_string() } else { t.name.clone() };
            keep.push(Track { name, notes: rest, ..t });
        }
    }
    keep.extend(beds_out);
    p.tracks = keep;
}

/// Adopt a piece opened from a file: as `adopt`, and a composed piece that hadn't been
/// edited still counts as unedited (its fingerprint taken again).
pub fn adopt_opened(p: &mut Project) {
    if p.tracks.iter().all(|t| t.ambience || !is_city(t) || t.notes.iter().all(|n| bed_of(n.key).is_none())) {
        return;
    }
    let untouched = p.untouched();
    adopt(p);
    if untouched {
        p.composed = crate::compose::fingerprint(p);
    }
}

/// Add a bed of this kind (rain of `rain`, for Rain), held from the start to the end
/// (thunder before the sections, the neon from the second), at its usual level. Returns
/// its index; if the piece has one already, that one's.
pub fn add(p: &mut Project, b: &Bed, rain: Option<RainKind>) -> usize {
    if let Some(i) = find(p, b) {
        return i;
    }
    let end = p.end_tick();
    let i = p.tracks.len();
    let strip = Strip { reverb: 0.2, width: 1.6, gain_db: b.gain_db, ..Strip::default() };
    let mut t = Track {
        strip,
        role: Some(Role::City),
        ambience: true,
        color: NEON[i % NEON.len()],
        ..Track::new(b.name, Source::Kit(KitKind::CityFx), i)
    };
    let key = if b.name == "Rain" { rain.unwrap_or(RainKind::ALL[0]).key() } else { b.key };
    t.notes.push(Note { start: 0, len: end.max(1), key, vel: 100 });
    p.tracks.push(t);
    if matches!(b.key, fx::THUNDER | fx::NEON) {
        // (timed by the sections, as the City track's options always were)
        crate::city::set_layer(p, i, b.key, true);
    }
    i
}

/// All the City FX kit's notes in a piece, beds and effects together, as one track (for
/// tests written when they were one track).
#[cfg(test)]
pub(crate) fn merged(p: &Project) -> Track {
    let mut t = Track::new("City", Source::Kit(KitKind::CityFx), 0);
    for x in p.tracks.iter().filter(|t| is_city(t)) {
        t.notes.extend(x.notes.iter().copied());
    }
    t.sort();
    t
}

/// Take a bed away.
pub fn remove(p: &mut Project, i: usize) {
    if p.tracks.get(i).is_some_and(|t| t.ambience) {
        p.tracks.remove(i);
    }
}

/// A freshly composed piece in the same style keeps the beds of the one it replaces:
/// the same set (a bed taken away stays away, one added stays), each at the level and
/// switch it had. Its notes are the new piece's where the style writes that bed.
pub fn keep(new: &mut Project, old: &Project) {
    let old_beds: Vec<&Track> = old.tracks.iter().filter(|t| t.ambience).collect();
    // the new piece's beds the old one didn't have go
    let gone: Vec<usize> = beds(new)
        .into_iter()
        .filter(|&i| {
            let b = bed_of_track(&new.tracks[i]);
            !old_beds.iter().any(|o| bed_of_track(o).map(|x| x.name) == b.map(|x| x.name))
        })
        .collect();
    for i in gone.into_iter().rev() {
        new.tracks.remove(i);
    }
    for o in old_beds {
        let Some(b) = bed_of_track(o) else { continue };
        let rain = o.notes.first().and_then(|n| RainKind::of_key(n.key));
        let i = add(new, b, rain);
        let t = &mut new.tracks[i];
        t.strip = o.strip;
        t.color = o.color;
        if let Some(k) = rain {
            for n in t.notes.iter_mut().filter(|n| RainKind::of_key(n.key).is_some()) {
                n.key = k.key();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compose::{Recipe, Style, compose, fingerprint};

    /// Every style's field sounds are beds of one sound each; the score effects stay as
    /// music; nothing is lost.
    #[test]
    fn composed_field_sounds_are_beds() {
        for style in Style::ALL {
            let p = compose(&Recipe { style, ..Recipe::random(3) }, style.tempo(), style.meter().unwrap_or((4, 4)), 32);
            for t in &p.tracks {
                if t.ambience {
                    let b = bed_of_track(t).expect("a bed");
                    assert!(t.notes.iter().all(|n| bed_of(n.key).map(|x| x.name) == Some(b.name)), "{style:?} {}", t.name);
                } else if is_city(t) {
                    assert!(t.notes.iter().all(|n| bed_of(n.key).is_none()), "{style:?} {}", t.name);
                }
            }
            let names: Vec<&str> = p.tracks.iter().filter(|t| t.ambience).filter_map(bed_of_track).map(|b| b.name).collect();
            let mut unique = names.clone();
            unique.dedup();
            assert_eq!(names.len(), unique.len(), "{style:?}: {names:?}");
        }
    }

    /// An old City track (rain, thunder, hum and a riser together) opens as beds and an
    /// effects track; the piece still counts as unedited.
    #[test]
    fn old_files_are_adopted() {
        let mut p = compose(&Recipe { style: Style::Nocturne, ..Recipe::random(4) }, 66.0, (4, 4), 24);
        // put it back as it was saved before beds
        let beds: Vec<Track> = p.tracks.iter().filter(|t| t.ambience).cloned().collect();
        p.tracks.retain(|t| !t.ambience);
        let mut city = Track::new("City", Source::Kit(KitKind::CityFx), 0);
        for b in &beds {
            city.notes.extend(b.notes.iter().copied());
        }
        city.notes.push(Note { start: 960, len: 960, key: fx::RISER, vel: 90 });
        p.tracks.push(city);
        let notes_before: usize = p.tracks.iter().map(|t| t.notes.len()).sum();
        p.composed = fingerprint(&p);
        adopt_opened(&mut p);
        assert!(p.untouched());
        assert_eq!(p.tracks.iter().map(|t| t.notes.len()).sum::<usize>(), notes_before);
        let fx_track = p.tracks.iter().find(|t| t.notes.iter().any(|n| n.key == fx::RISER && n.start == 960)).expect("the riser stays");
        assert!(!fx_track.ambience && fx_track.notes.iter().all(|n| bed_of(n.key).is_none()));
        assert!(p.tracks.iter().filter(|t| t.ambience).count() >= 2);
    }

    /// Beds added, kept through a new piece in the same style, and taken away.
    #[test]
    fn beds_come_and_go() {
        let mut p = compose(&Recipe { style: Style::Canopy, ..Recipe::random(2) }, 60.0, (4, 4), 24);
        let crickets = &BEDS.iter().find(|b| b.name == "Crickets").copied().unwrap();
        let i = add(&mut p, crickets, None);
        assert!(p.tracks[i].ambience && p.tracks[i].notes[0].end() == p.end_tick());
        assert_eq!(add(&mut p, crickets, None), i);
        p.tracks[i].strip.gain_db = -20.0;
        let birds = find(&p, BEDS.iter().find(|b| b.name == "Birdsong").unwrap()).unwrap();
        remove(&mut p, birds);
        let mut fresh = compose(&Recipe { style: Style::Canopy, ..Recipe::random(9) }, 60.0, (4, 4), 24);
        keep(&mut fresh, &p);
        assert!(find(&fresh, BEDS.iter().find(|b| b.name == "Birdsong").unwrap()).is_none());
        let c = find(&fresh, crickets).unwrap();
        assert_eq!(fresh.tracks[c].strip.gain_db, -20.0);
        // a bed isn't an edit
        assert!(fresh.untouched());
    }
}
