//! A project flattened into time-ordered events for the engine.

use dd_dsp::kit::fx;
use dd_dsp::sampler::KitKind;

use crate::project::{CtrlKind, Project, Source, TAIL_SECS};

/// One message to a track's instrument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Msg {
    On(u8, u8),
    Off(u8),
    Cc(u8, u8),
    Bend(i16),
    Pressure(u8),
}

impl Msg {
    /// Order of messages at the same moment: a note ends before the next starts.
    fn rank(&self) -> u8 {
        match self {
            Msg::Off(_) => 0,
            Msg::Cc(..) | Msg::Bend(_) | Msg::Pressure(_) => 1,
            Msg::On(..) => 2,
        }
    }

    pub fn of_ctrl(kind: CtrlKind, value: i16) -> Msg {
        match kind {
            CtrlKind::Cc(n) => Msg::Cc(n, value.clamp(0, 127) as u8),
            CtrlKind::Bend => Msg::Bend(value.clamp(-8192, 8191)),
            CtrlKind::Pressure => Msg::Pressure(value.clamp(0, 127) as u8),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub t: f64,
    pub track: u16,
    pub msg: Msg,
}

/// A note, for restarting the notes that sound across a seek.
#[derive(Clone, Copy, Debug)]
struct Span {
    start: f64,
    end: f64,
    track: u16,
    key: u8,
    vel: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub events: Vec<Event>,
    spans: Vec<Span>,
    /// The music, in whole bars.
    pub music: f64,
    /// The music and its reverb tail.
    pub duration: f64,
}

/// Whether a note is worth restarting part way through: sustained sounds are; hits,
/// thunder and risers would land at the wrong time, so they aren't.
fn chases(source: &Source, key: u8) -> bool {
    match source {
        Source::Kit(KitKind::Drums) => false,
        Source::Kit(KitKind::CityFx) => {
            matches!(key, fx::HUM | fx::NEON)
                || dd_dsp::rain::RainKind::of_key(key).is_some()
                || (fx::DRONE..fx::DRONE + 12).contains(&key)
                || (fx::ANALOG_DRONE..fx::ANALOG_DRONE + 12).contains(&key)
        }
        Source::SoundFont { bank: 128, .. } => false,
        _ => true,
    }
}

impl Timeline {
    pub fn from_project(p: &Project) -> Timeline {
        let mut events = Vec::new();
        let mut spans = Vec::new();
        for (ti, t) in p.tracks.iter().enumerate() {
            let track = ti as u16;
            let mut notes = t.notes.clone();
            notes.sort_by_key(|n| (n.key, n.start));
            // a key can't sound twice on one track: end each note before the next on its key
            for i in 0..notes.len() {
                if let Some(next) = notes.get(i + 1).copied()
                    && next.key == notes[i].key
                    && notes[i].end() > next.start
                {
                    notes[i].len = next.start.saturating_sub(notes[i].start).max(1);
                }
            }
            for n in &notes {
                let (a, b) = (p.tick_secs(n.start), p.tick_secs(n.end()));
                events.push(Event { t: a, track, msg: Msg::On(n.key, n.vel.max(1)) });
                events.push(Event { t: b, track, msg: Msg::Off(n.key) });
                if chases(&t.source, n.key) {
                    spans.push(Span { start: a, end: b, track, key: n.key, vel: n.vel.max(1) });
                }
            }
            for c in &t.ctrls {
                events.push(Event { t: p.tick_secs(c.tick), track, msg: Msg::of_ctrl(c.kind, c.value) });
            }
        }
        events.sort_by(|a, b| a.t.total_cmp(&b.t).then(a.msg.rank().cmp(&b.msg.rank())).then(a.track.cmp(&b.track)));
        let music = p.music_secs();
        Timeline { events, spans, music, duration: music + TAIL_SECS }
    }

    /// The index of the first event after `t`.
    pub fn index_after(&self, t: f64) -> usize {
        self.events.partition_point(|e| e.t <= t)
    }

    /// What to send to pick up the music at `t`: each track's controllers as they
    /// stand, then the sustained notes sounding across it.
    pub fn chase(&self, t: f64) -> Vec<(u16, Msg)> {
        let mut out: Vec<(u16, Msg)> = Vec::new();
        let mut seen: Vec<(u16, u8)> = Vec::new();
        // the latest of each controller before t, newest first
        for e in self.events[..self.index_after(t)].iter().rev() {
            let id = match e.msg {
                Msg::Cc(n, _) => n,
                Msg::Bend(_) => 200,
                Msg::Pressure(_) => 201,
                _ => continue,
            };
            if !seen.contains(&(e.track, id)) {
                seen.push((e.track, id));
                out.push((e.track, e.msg));
            }
        }
        out.reverse();
        for s in &self.spans {
            if s.start < t && s.end - t > 0.15 {
                out.push((s.track, Msg::On(s.key, s.vel)));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::rain_on_neon;

    #[test]
    fn events_are_ordered_and_balanced() {
        let p = rain_on_neon();
        let tl = Timeline::from_project(&p);
        assert!(tl.events.windows(2).all(|w| w[0].t <= w[1].t));
        let ons = tl.events.iter().filter(|e| matches!(e.msg, Msg::On(..))).count();
        let offs = tl.events.iter().filter(|e| matches!(e.msg, Msg::Off(..))).count();
        assert_eq!(ons, offs);
        assert!((tl.music - 24.0 * 4.0 * 60.0 / 66.0).abs() < 1e-6);
    }

    #[test]
    fn chase_restarts_rain_but_not_thunder() {
        let p = rain_on_neon();
        let tl = Timeline::from_project(&p);
        let city = p.tracks.iter().position(|t| t.name == "City").unwrap() as u16;
        let at = p.bar_secs() * 11.0 + 0.1 + p.bar_secs() * 0.75;
        let c = tl.chase(at);
        assert!(c.contains(&(city, Msg::On(fx::RAIN, 100))));
        assert!(!c.iter().any(|(t, m)| *t == city && matches!(m, Msg::On(fx::THUNDER, _))));
    }
}
