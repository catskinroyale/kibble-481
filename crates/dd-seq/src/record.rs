//! Recording what's played live onto a track, and quantising notes to the grid.

use crate::project::{Ctrl, CtrlKind, Note, Project};
use crate::timeline::Msg;

/// Notes and controller moves played while recording, stamped in song seconds.
#[derive(Clone, Debug, Default)]
pub struct Take {
    pub track: usize,
    /// (start, end, key, velocity) of finished notes.
    pub notes: Vec<(f64, f64, u8, u8)>,
    /// Notes still held: (key, start, velocity).
    pub open: Vec<(u8, f64, u8)>,
    pub ctrls: Vec<(f64, CtrlKind, i16)>,
}

impl Take {
    pub fn new(track: usize) -> Take {
        Take { track, ..Take::default() }
    }

    pub fn push(&mut self, t: f64, msg: Msg) {
        match msg {
            Msg::On(k, v) if v > 0 => {
                self.close(k, t);
                self.open.push((k, t, v));
            }
            Msg::On(k, _) | Msg::Off(k) => self.close(k, t),
            Msg::Cc(c, v) => self.ctrls.push((t, CtrlKind::Cc(c), v as i16)),
            Msg::Bend(b) => self.ctrls.push((t, CtrlKind::Bend, b)),
            Msg::Pressure(p) => self.ctrls.push((t, CtrlKind::Pressure, p as i16)),
        }
    }

    fn close(&mut self, key: u8, t: f64) {
        if let Some(i) = self.open.iter().position(|(k, ..)| *k == key) {
            let (k, start, vel) = self.open.remove(i);
            self.notes.push((start, t.max(start + 0.005), k, vel));
        }
    }

    /// End every held note at `t`.
    pub fn finish(&mut self, t: f64) {
        for (k, start, vel) in std::mem::take(&mut self.open) {
            self.notes.push((start, t.max(start + 0.005), k, vel));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty() && self.open.is_empty() && self.ctrls.is_empty()
    }

    /// The take as notes on the grid, with the held notes running to `now`. Notes
    /// played during the count-in start on the downbeat.
    pub fn preview(&self, p: &Project, now: f64) -> Vec<Note> {
        self.notes
            .iter()
            .copied()
            .chain(self.open.iter().map(|&(k, s, v)| (s, now.max(s + 0.005), k, v)))
            .filter(|(_, end, ..)| *end > 0.0)
            .map(|(s, e, k, v)| {
                let start = p.secs_tick(s.max(0.0));
                Note { start, len: p.secs_tick(e).saturating_sub(start).max(1), key: k, vel: v }
            })
            .collect()
    }

    /// Add the take to its track, quantising the note starts if asked.
    /// With `replace`, the notes it overlaps in time are removed first.
    pub fn commit(mut self, p: &mut Project, now: f64, quantize: Option<(u32, f32)>, replace: bool) -> usize {
        self.finish(now);
        let mut notes = self.preview(p, now);
        if let Some((grid, strength)) = quantize {
            quantize_notes(&mut notes, grid, strength, false);
        }
        let ctrls: Vec<Ctrl> =
            self.ctrls.iter().filter(|(t, ..)| *t >= 0.0).map(|&(t, kind, value)| Ctrl { tick: p.secs_tick(t), kind, value }).collect();
        let Some(track) = p.tracks.get_mut(self.track) else { return 0 };
        if replace && let (Some(a), Some(b)) = (notes.iter().map(|n| n.start).min(), notes.iter().map(|n| n.end()).max()) {
            track.notes.retain(|n| n.end() <= a || n.start >= b);
        }
        let added = notes.len();
        track.notes.extend(notes);
        track.ctrls.extend(ctrls);
        track.sort();
        added
    }
}

/// Move note starts towards the nearest line of a grid `grid` ticks apart, by
/// `strength` (1 = onto it). With `ends`, note lengths too, to whole grid steps.
pub fn quantize_notes(notes: &mut [Note], grid: u32, strength: f32, ends: bool) {
    let g = grid.max(1) as f64;
    let s = strength.clamp(0.0, 1.0) as f64;
    for n in notes.iter_mut() {
        let target = (n.start as f64 / g).round() * g;
        let start = (n.start as f64 + (target - n.start as f64) * s).round() as u32;
        if ends {
            let end = n.end() as f64;
            let te = ((end / g).round() * g).max(target + g);
            let new_end = (end + (te - end) * s).round() as u32;
            n.len = new_end.saturating_sub(start).max(1);
        }
        n.start = start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::blank;
    use crate::project::PPQ;

    #[test]
    fn take_records_and_quantises() {
        let mut p = blank();
        p.tracks[0].notes.clear();
        let beat = 60.0 / p.bpm;
        let mut take = Take::new(0);
        take.push(-0.05, Msg::On(60, 100)); // a hair early, in the count-in
        take.push(beat * 0.9, Msg::Off(60));
        take.push(beat * 1.04, Msg::On(64, 90));
        take.push(beat * 1.5, Msg::Cc(1, 64));
        let n = take.commit(&mut p, beat * 2.0, Some((PPQ, 1.0)), false);
        assert_eq!(n, 2);
        let notes = &p.tracks[0].notes;
        assert_eq!(notes[0].start, 0);
        assert_eq!(notes[1].start, PPQ);
        assert_eq!(notes[1].key, 64);
        assert_eq!(p.tracks[0].ctrls.len(), 1);
    }

    #[test]
    fn half_strength_moves_half_way() {
        let mut n = [Note { start: 100, len: 50, key: 60, vel: 100 }];
        quantize_notes(&mut n, 240, 0.5, false);
        assert_eq!(n[0].start, 50);
    }
}
