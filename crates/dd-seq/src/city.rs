//! The City track's options, as the track list offers them: which rain falls (or none),
//! and whether there is thunder, a machinery hum and a buzzing neon sign. Each edits the
//! track's notes, so it plays, exports and can be edited like any other part.

use dd_dsp::kit::fx;
use dd_dsp::rain::RainKind;

use crate::project::{Note, Project};

/// The rain on a City track: the kind of its longest rain note.
pub fn rain(p: &Project, track: usize) -> Option<RainKind> {
    let t = p.tracks.get(track)?;
    t.notes.iter().filter(|n| RainKind::of_key(n.key).is_some()).max_by_key(|n| n.len).and_then(|n| RainKind::of_key(n.key))
}

/// Make the track's rain this kind, or take it away. Its main rain note changes kind
/// (keeping when and how hard it falls); with none yet, it rains from start to end.
pub fn set_rain(p: &mut Project, track: usize, kind: Option<RainKind>) {
    let end = p.end_tick();
    let Some(t) = p.tracks.get_mut(track) else { return };
    match kind {
        None => t.notes.retain(|n| RainKind::of_key(n.key).is_none()),
        Some(k) => {
            let main =
                t.notes.iter().enumerate().filter(|(_, n)| RainKind::of_key(n.key).is_some()).max_by_key(|(_, n)| n.len).map(|(i, _)| i);
            match main {
                Some(i) => t.notes[i].key = k.key(),
                None => t.notes.push(Note { start: 0, len: end, key: k.key(), vel: 100 }),
            }
            // a second rain of the same kind would double it
            let mut seen = false;
            t.notes.retain(|n| {
                if n.key != k.key() {
                    return true;
                }
                let keep = !seen;
                seen = true;
                keep
            });
        }
    }
    t.sort();
}

pub fn has(p: &Project, track: usize, key: u8) -> bool {
    p.tracks.get(track).is_some_and(|t| t.notes.iter().any(|n| n.key == key))
}

/// Turn thunder, the hum or the neon buzz on or off. Thunder rolls near the start and
/// a beat before each later section; the hum runs throughout; the neon buzzes from the
/// second section to the last.
pub fn set_layer(p: &mut Project, track: usize, key: u8, on: bool) {
    let (bar, beat, end) = (p.bar_ticks(), p.beat_ticks(), p.end_tick());
    let starts: Vec<u32> = p.section_bars().iter().map(|b| b * bar).collect();
    let Some(t) = p.tracks.get_mut(track) else { return };
    t.notes.retain(|n| n.key != key);
    if on {
        let mut add = |start: u32, len: u32, vel: u8| {
            if start < end && len > 0 {
                t.notes.push(Note { start, len: len.min(end - start), key, vel });
            }
        };
        match key {
            fx::THUNDER => {
                add(2 * beat, beat, 90);
                for (i, s) in starts.iter().enumerate().skip(1) {
                    add(s.saturating_sub(beat), beat, if i % 2 == 0 { 110 } else { 96 });
                }
            }
            fx::HUM => add(0, end, 55),
            fx::NEON => {
                let from = starts.get(1).copied().unwrap_or(0);
                let to = starts.last().copied().filter(|t| *t > from).unwrap_or(end);
                add(from, to - from, 50);
            }
            _ => add(0, end, 80),
        }
    }
    t.sort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::rain_on_neon;

    fn city(p: &Project) -> usize {
        p.tracks.iter().position(|t| t.name == "City").unwrap()
    }

    #[test]
    fn rain_changes_kind_and_comes_and_goes() {
        let mut p = rain_on_neon();
        let c = city(&p);
        assert_eq!(rain(&p, c), Some(RainKind::City));
        set_rain(&mut p, c, Some(RainKind::Window));
        assert_eq!(rain(&p, c), Some(RainKind::Window));
        set_rain(&mut p, c, None);
        assert_eq!(rain(&p, c), None);
        set_rain(&mut p, c, Some(RainKind::Light));
        let n = p.tracks[c].notes.iter().find(|n| n.key == RainKind::Light.key()).unwrap();
        assert_eq!((n.start, n.end()), (0, p.end_tick()));
    }

    #[test]
    fn thunder_toggles() {
        let mut p = rain_on_neon();
        let c = city(&p);
        assert!(has(&p, c, fx::THUNDER));
        set_layer(&mut p, c, fx::THUNDER, false);
        assert!(!has(&p, c, fx::THUNDER));
        set_layer(&mut p, c, fx::THUNDER, true);
        let n = p.tracks[c].notes.iter().filter(|n| n.key == fx::THUNDER).count();
        assert_eq!(n, p.sections.len());
    }
}
