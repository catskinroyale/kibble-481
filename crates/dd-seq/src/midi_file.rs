//! Standard MIDI Files: export (type 1, a track per track) and import.
//!
//! Exported files play anywhere: each synth track carries the General MIDI program
//! nearest its patch, SoundFont tracks their bank and program, the drum kit is on the
//! drum channel at the Ambient Electronic SoundFont's Trailer Kit (48) and the city
//! effects at its Trailer FX (119), whose keys this app's kit shares. The whole project
//! is embedded as text too, so a file saved here opens again exactly as it was.
//!
//! Import reads any MIDI file: each track and channel becomes a track, played by the
//! General MIDI SoundFont with the file's programs. Files from Ambient MIDI Generator
//! get their score kit and city effects back.

use std::path::Path;

use dd_dsp::patch::{Category, Patch};
use dd_dsp::sampler::KitKind;
use midly::num::{u4, u7, u15, u24, u28};
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::demo::GM_FONT;
use crate::fonts::FontBank;
use crate::project::{Ctrl, CtrlKind, Note, PPQ, Project, Section, Source, Track};

pub const PROJECT_PREFIX: &str = "kibble-481-project:";
/// The marker in MIDI files saved before the rename; they still open exactly.
pub const OLD_PROJECT_PREFIX: &str = "ddream-project:";
const MIDIGEN_PREFIX: &str = "midigen-recipe:";
pub const KIT_PROGRAM: u8 = dd_dsp::kit::KIT_PROGRAM;
pub const FX_PROGRAM: u8 = dd_dsp::kit::FX_PROGRAM;

/// The General MIDI program nearest a synth patch.
pub fn gm_program(p: &Patch) -> u8 {
    match p.category {
        Category::Brass => 62,                           // Synth Brass 1
        Category::Pad => 88,                             // New Age pad
        Category::Strings => 50,                         // Synth Strings 1
        Category::Lead if p.layers[0].noise > 0.0 => 66, // Tenor Sax, for the smoky leads
        Category::Lead => 81,                            // Saw lead
        Category::Keys => 4,                             // Electric Piano 1
        Category::Bells => 14,                           // Tubular Bells
        Category::Pluck => 107,                          // Koto
        Category::Bass => 38,                            // Synth Bass 1
        Category::Atmosphere => 95,                      // Sweep pad
        Category::Drone => 92,                           // Bowed glass
        Category::Init => 81,
    }
}

fn meta(delta: u32, m: MetaMessage<'_>) -> TrackEvent<'_> {
    TrackEvent { delta: u28::new(delta), kind: TrackEventKind::Meta(m) }
}

/// MIDI channels for the tracks: the drums on 10, the others in turn round the rest.
fn channels(p: &Project) -> Vec<u8> {
    let mut next = 0u8;
    p.tracks
        .iter()
        .map(|t| {
            if t.source.is_drums() {
                9
            } else {
                let c = next;
                next = (next + 1) % 15;
                if c >= 9 { c + 1 } else { c }
            }
        })
        .collect()
}

pub fn to_bytes(p: &Project) -> std::io::Result<Vec<u8>> {
    let end = p.end_tick();
    let embedded = format!("{PROJECT_PREFIX}{}", serde_json::to_string(p).unwrap_or_default());
    let names: Vec<String> = p.tracks.iter().map(|t| format!("{} ({})", t.name, t.source.label())).collect();
    let section_names: Vec<(u32, String)> = p.sections.iter().map(|s| (s.bar * p.bar_ticks(), s.name.clone())).collect();
    let us_per_quarter = (60_000_000.0 / p.bpm).round() as u32;
    let denom_pow = (p.meter.1.max(1) as f32).log2() as u8;

    let mut tracks: Vec<Vec<TrackEvent>> = Vec::new();
    let mut t0 = vec![
        meta(0, MetaMessage::TrackName(p.title.as_bytes())),
        meta(0, MetaMessage::Text(embedded.as_bytes())),
        meta(0, MetaMessage::Tempo(u24::new(us_per_quarter))),
        meta(0, MetaMessage::TimeSignature(p.meter.0 as u8, denom_pow, 24, 8)),
    ];
    let mut last = 0;
    let mut secs = section_names.iter().collect::<Vec<_>>();
    secs.sort_by_key(|s| s.0);
    for (t, name) in secs {
        t0.push(meta(t - last, MetaMessage::Marker(name.as_bytes())));
        last = *t;
    }
    t0.push(meta(end.saturating_sub(last), MetaMessage::EndOfTrack));
    tracks.push(t0);

    for ((t, ch), name) in p.tracks.iter().zip(channels(p)).zip(&names) {
        let ch4 = u4::new(ch);
        let midi =
            |delta: u32, message: MidiMessage| TrackEvent { delta: u28::new(delta), kind: TrackEventKind::Midi { channel: ch4, message } };
        let cc = |c: u8, v: u8| MidiMessage::Controller { controller: u7::new(c), value: u7::new(v.min(127)) };
        let mut track = vec![meta(0, MetaMessage::TrackName(name.as_bytes()))];
        let (bank, program) = match &t.source {
            Source::Synth(patch) => (0, gm_program(patch)),
            Source::SoundFont { bank, program, .. } => (if *bank == 128 { 0 } else { *bank }, *program),
            Source::Kit(KitKind::Drums) => (0, KIT_PROGRAM),
            Source::Kit(KitKind::CityFx) => (0, FX_PROGRAM),
        };
        track.push(midi(0, cc(0, bank.min(127) as u8)));
        track.push(midi(0, MidiMessage::ProgramChange { program: u7::new(program) }));
        let s = &t.strip;
        let vol = (s.gain().sqrt() * 100.0).round().clamp(0.0, 127.0) as u8;
        track.push(midi(0, cc(7, vol)));
        track.push(midi(0, cc(10, (64.0 + s.pan * 63.5).round().clamp(0.0, 127.0) as u8)));
        track.push(midi(0, cc(91, (s.reverb * 127.0).round() as u8)));
        track.push(midi(0, cc(93, if s.ensemble { 64 } else { 0 })));

        // (tick, rank, message): offs, then controllers, then ons at the same tick
        let mut abs: Vec<(u32, u8, MidiMessage)> = Vec::new();
        for n in &t.notes {
            abs.push((n.start, 2, MidiMessage::NoteOn { key: u7::new(n.key.min(127)), vel: u7::new(n.vel.clamp(1, 127)) }));
            abs.push((n.end(), 0, MidiMessage::NoteOff { key: u7::new(n.key.min(127)), vel: u7::new(0) }));
        }
        for c in &t.ctrls {
            let m = match c.kind {
                CtrlKind::Cc(n) => cc(n, c.value.clamp(0, 127) as u8),
                CtrlKind::Bend => MidiMessage::PitchBend { bend: midly::PitchBend::from_int(c.value.clamp(-8192, 8191)) },
                CtrlKind::Pressure => MidiMessage::ChannelAftertouch { vel: u7::new(c.value.clamp(0, 127) as u8) },
            };
            abs.push((c.tick, 1, m));
        }
        abs.sort_by_key(|(t, r, _)| (*t, *r));
        let mut last = 0;
        for (tick, _, m) in abs {
            track.push(midi(tick - last, m));
            last = tick;
        }
        track.push(meta(end.saturating_sub(last), MetaMessage::EndOfTrack));
        tracks.push(track);
    }

    let mut smf = Smf::new(Header::new(Format::Parallel, Timing::Metrical(u15::new(PPQ as u16))));
    smf.tracks = tracks;
    let mut out = Vec::new();
    smf.write_std(&mut out)?;
    Ok(out)
}

pub fn save(p: &Project, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, to_bytes(p)?)
}

pub fn load(path: &Path, fonts: &FontBank) -> Result<Project, String> {
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut p = from_bytes(&data, fonts)?;
    if p.title == "Untitled" {
        p.title = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    }
    Ok(p)
}

/// One channel of one track of the file, while reading it.
struct Part {
    name: String,
    channel: u8,
    bank: u16,
    program: Option<u8>,
    notes: Vec<Note>,
    ctrls: Vec<Ctrl>,
    open: Vec<(u8, u32, u8)>,
}

pub fn from_bytes(data: &[u8], fonts: &FontBank) -> Result<Project, String> {
    let smf = Smf::parse(data).map_err(|e| e.to_string())?;
    // a file this app saved opens exactly as it was
    for ev in smf.tracks.iter().take(1).flatten() {
        if let TrackEventKind::Meta(MetaMessage::Text(t)) = ev.kind
            && let Some(json) =
                std::str::from_utf8(t).ok().and_then(|s| s.strip_prefix(PROJECT_PREFIX).or_else(|| s.strip_prefix(OLD_PROJECT_PREFIX)))
        {
            return Project::from_json(json);
        }
    }
    let ppq = match smf.header.timing {
        Timing::Metrical(t) => t.as_int().max(1) as u64,
        Timing::Timecode(..) => return Err("This MIDI file is timed in SMPTE frames, which isn't supported".into()),
    };
    let scale = |t: u64| (t * PPQ as u64 / ppq) as u32;
    let mut p = Project { tracks: Vec::new(), sections: Vec::new(), length_bars: 1, ..Project::default() };
    let mut tempo_set = false;
    let mut from_midigen = false;
    let mut parts: Vec<Part> = Vec::new();
    for (ti, track) in smf.tracks.iter().enumerate() {
        let mut tick = 0u64;
        let mut name = String::new();
        let first = parts.len();
        for ev in track {
            tick += ev.delta.as_int() as u64;
            let at = scale(tick);
            match ev.kind {
                TrackEventKind::Meta(m) => match m {
                    MetaMessage::Tempo(us) if !tempo_set => {
                        p.bpm = (60_000_000.0 / us.as_int().max(1) as f64 * 100.0).round() / 100.0;
                        tempo_set = true;
                    }
                    MetaMessage::TimeSignature(n, d, ..) => p.meter = (n.max(1) as u32, 1u32 << d.min(5)),
                    MetaMessage::TrackName(b) => name = String::from_utf8_lossy(b).trim().to_string(),
                    MetaMessage::Marker(b) => p.sections.push(Section::marker(at, String::from_utf8_lossy(b).trim().to_string())),
                    MetaMessage::Text(b) if b.starts_with(MIDIGEN_PREFIX.as_bytes()) => from_midigen = true,
                    _ => {}
                },
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    let pi = match parts[first..].iter().position(|x| x.channel == ch) {
                        Some(i) => first + i,
                        None => {
                            parts.push(Part {
                                name: String::new(),
                                channel: ch,
                                bank: 0,
                                program: None,
                                notes: Vec::new(),
                                ctrls: Vec::new(),
                                open: Vec::new(),
                            });
                            parts.len() - 1
                        }
                    };
                    let part = &mut parts[pi];
                    match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => part.open.push((key.as_int(), at, vel.as_int())),
                        MidiMessage::NoteOn { key, .. } | MidiMessage::NoteOff { key, .. } => {
                            if let Some(i) = part.open.iter().position(|(k, ..)| *k == key.as_int()) {
                                let (k, start, vel) = part.open.remove(i);
                                part.notes.push(Note { start, len: at.saturating_sub(start).max(1), key: k, vel });
                            }
                        }
                        MidiMessage::Controller { controller, value } => match controller.as_int() {
                            0 => part.bank = value.as_int() as u16,
                            // mixer settings become the strip, not automation
                            7 | 10 | 91 | 93 | 32 => {}
                            c => part.ctrls.push(Ctrl { tick: at, kind: CtrlKind::Cc(c), value: value.as_int() as i16 }),
                        },
                        MidiMessage::ProgramChange { program } => {
                            if part.program.is_none() {
                                part.program = Some(program.as_int());
                            }
                        }
                        MidiMessage::PitchBend { bend } => part.ctrls.push(Ctrl { tick: at, kind: CtrlKind::Bend, value: bend.as_int() }),
                        MidiMessage::ChannelAftertouch { vel } => {
                            part.ctrls.push(Ctrl { tick: at, kind: CtrlKind::Pressure, value: vel.as_int() as i16 })
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        for part in parts[first..].iter_mut() {
            part.name = if name.is_empty() { format!("Track {}", ti) } else { name.clone() };
            for (k, start, vel) in part.open.drain(..) {
                part.notes.push(Note { start, len: scale(tick).saturating_sub(start).max(1), key: k, vel });
            }
        }
    }
    let bar = p.bar_ticks();
    for s in p.sections.iter_mut() {
        s.bar /= bar;
    }
    let gm = fonts.get(GM_FONT);
    for part in parts.into_iter().filter(|x| !x.notes.is_empty()) {
        let program = part.program.unwrap_or(0);
        let drums = part.channel == 9;
        let source = if from_midigen && drums && program == KIT_PROGRAM {
            Source::Kit(KitKind::Drums)
        } else if from_midigen && (program == FX_PROGRAM || part.name.ends_with("(FX)")) {
            Source::Kit(KitKind::CityFx)
        } else {
            let bank = if drums { 128 } else { part.bank };
            let preset =
                gm.and_then(|f| f.find(bank, program).or_else(|| f.find(if drums { 128 } else { 0 }, if drums { 0 } else { program })));
            let (bank, program, name) = match preset {
                Some(x) => (x.bank, x.program, x.name.clone()),
                None => (bank, program, format!("Program {}", program + 1)),
            };
            Source::SoundFont { font: GM_FONT.into(), bank, program, name }
        };
        let i = p.tracks.len();
        let mut t = Track { notes: part.notes, ctrls: part.ctrls, ..Track::new(&part.name, source, i) };
        t.strip.ensemble = false;
        t.sort();
        p.tracks.push(t);
    }
    p.length_bars = p.bars();
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::rain_on_neon;

    #[test]
    fn project_round_trips_through_midi() {
        let p = rain_on_neon();
        let bytes = to_bytes(&p).unwrap();
        let q = from_bytes(&bytes, &FontBank::default()).unwrap();
        assert_eq!(p, q);
    }

    #[test]
    fn files_saved_before_the_rename_open_exactly() {
        let p = rain_on_neon();
        let bytes = to_bytes(&p).unwrap();
        let mut smf = Smf::parse(&bytes).unwrap();
        let old = format!("{OLD_PROJECT_PREFIX}{}", serde_json::to_string(&p).unwrap());
        for ev in smf.tracks[0].iter_mut() {
            if let TrackEventKind::Meta(MetaMessage::Text(t)) = &mut ev.kind
                && t.starts_with(PROJECT_PREFIX.as_bytes())
            {
                *t = old.as_bytes();
            }
        }
        let mut out = Vec::new();
        smf.write_std(&mut out).unwrap();
        assert!(out.windows(PROJECT_PREFIX.len()).all(|w| w != PROJECT_PREFIX.as_bytes()));
        assert_eq!(from_bytes(&out, &FontBank::default()).unwrap(), p);
    }

    #[test]
    fn foreign_files_import_as_tracks() {
        let p = rain_on_neon();
        let mut bytes = to_bytes(&p).unwrap();
        // remove the embedded project so it reads like any other file
        let at = bytes.windows(PROJECT_PREFIX.len()).position(|w| w == PROJECT_PREFIX.as_bytes()).unwrap();
        bytes[at] = b'X';
        let q = from_bytes(&bytes, &FontBank::default()).unwrap();
        assert_eq!(q.bpm, 66.0);
        assert_eq!(q.tracks.len(), p.tracks.iter().filter(|t| !t.notes.is_empty()).count());
        let notes = |p: &Project| p.tracks.iter().map(|t| t.notes.len()).sum::<usize>();
        assert_eq!(notes(&p), notes(&q));
        assert_eq!(q.sections.len(), 4);
        assert_eq!(q.sections[2].bar, 12);
        assert!(q.tracks.iter().any(|t| t.source.is_drums()));
    }
}
