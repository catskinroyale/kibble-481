//! The starter scene the app opens with: "Rain on Neon", a short neon-noir cue in
//! D minor, written here note by note so there's something to hear, take apart and
//! build on straight away.

use dd_dsp::kit::{drum, fx};
use dd_dsp::patch::by_name;
use dd_dsp::sampler::KitKind;

use crate::project::{Ctrl, CtrlKind, Note, PPQ, Part, Project, Section, Source, Strip, Track};

pub const GM_FONT: &str = "GeneralUser-GS.sf2";
pub const SAX_FONT: &str = "FreePats-TenorSaxophone.sf2";

/// Two-bar chords, lowest note first.
const CHORDS: [[u8; 5]; 4] = [
    [38, 50, 57, 64, 65], // Dm(add9): D2 | D3 A3 E4 F4
    [34, 46, 53, 57, 62], // Bbmaj7: Bb1 | Bb2 F3 A3 D4
    [31, 43, 50, 53, 58], // Gm(add9): G1 | G2 D3 F3 Bb3
    [33, 45, 50, 52, 57], // Asus4: A1 | A2 D3 E3 A3
];

fn note(bar: u32, beat: f64, beats: f64, key: u8, vel: u8) -> Note {
    let q = PPQ as f64;
    Note { start: (bar as f64 * 4.0 * q + beat * q).round() as u32, len: (beats * q).round().max(1.0) as u32, key, vel }
}

fn chord_at(bar: u32) -> &'static [u8; 5] {
    &CHORDS[(bar / 2 % 4) as usize]
}

pub fn rain_on_neon() -> Project {
    let mut p = Project { title: "Rain on Neon".into(), bpm: 66.0, meter: (4, 4), length_bars: 24, ..Project::default() };
    p.sections =
        vec![Section::new(0, Part::Neon), Section::new(4, Part::Swell), Section::new(12, Part::NightShift), Section::new(20, Part::Fade)];
    let mut tracks = Vec::new();
    let mut add = |name: &str, source: Source, strip: Strip| {
        let i = tracks.len();
        tracks.push(Track { strip, ..Track::new(name, source, i) });
        i
    };
    let brass =
        add("Brass", Source::Synth(by_name("Neon Brass")), Strip { reverb: 0.5, ensemble: true, gain_db: -3.0, ..Strip::default() });
    let pad = add("Pad", Source::Synth(by_name("Rain Pad")), Strip { reverb: 0.6, ensemble: true, gain_db: -6.0, ..Strip::default() });
    let lead = add("Lead", Source::Synth(by_name("Smoky Lead")), Strip { reverb: 0.55, echo: 0.35, pan: 0.1, ..Strip::default() });
    let bass = add("Bass", Source::Synth(by_name("Sub Pulse Bass")), Strip { reverb: 0.08, gain_db: 2.0, ..Strip::default() });
    let bells = add(
        "Bells",
        Source::Synth(by_name("Night Bells")),
        Strip { reverb: 0.65, echo: 0.45, gain_db: -4.0, pan: -0.2, ..Strip::default() },
    );
    let city = add("City", Source::Kit(KitKind::CityFx), Strip { reverb: 0.3, gain_db: -2.0, width: 1.6, ..Strip::default() });
    let drums = add("Drums", Source::Kit(KitKind::Drums), Strip { reverb: 0.4, echo: 0.15, gain_db: -5.0, ..Strip::default() });
    let koto = add(
        "Koto",
        Source::SoundFont { font: GM_FONT.into(), bank: 0, program: 107, name: "Koto".into() },
        Strip { reverb: 0.5, echo: 0.3, pan: 0.35, gain_db: 2.0, ..Strip::default() },
    );
    let sax = add(
        "Sax",
        Source::SoundFont { font: SAX_FONT.into(), bank: 0, program: 0, name: "Tenor Saxophone".into() },
        Strip { reverb: 0.6, echo: 0.3, pan: -0.1, gain_db: -7.0, ..Strip::default() },
    );
    p.tracks = tracks;
    let t = &mut p.tracks;

    for bar in (0..24).step_by(2) {
        let c = chord_at(bar);
        // the pad holds every chord, a little short so the next one breathes in
        for &k in &c[1..] {
            t[pad].notes.push(note(bar, 0.0, 7.6, k, 62));
        }
        // brass in the middle, swelling and sagging away on the fourth beat of the second bar
        if (4..20).contains(&bar) {
            let vel = if bar >= 12 { 104 } else { 88 };
            for &k in &c[1..] {
                t[brass].notes.push(note(bar, 0.0, 6.5, k + 12, vel));
            }
            t[bass].notes.push(note(bar, 0.0, 3.5, c[0] + 12, 96));
            t[bass].notes.push(note(bar + 1, 0.0, 1.5, c[0] + 12, 84));
            t[bass].notes.push(note(bar + 1, 2.5, 1.0, c[0] + 12, 70));
        }
    }
    // bells, falling like rain: a broken chord high up, with gaps
    let pattern: [Option<usize>; 8] = [Some(4), Some(2), None, Some(3), Some(1), None, Some(4), None];
    for bar in 2..22 {
        let c = chord_at(bar);
        for (i, step) in pattern.iter().enumerate() {
            if let Some(s) = step {
                if (bar + i as u32) % 5 == 4 {
                    continue;
                }
                let vel = 70 + ((bar * 7 + i as u32 * 13) % 30) as u8;
                t[bells].notes.push(note(bar, i as f64 * 0.5, 0.5, c[*s] + 24, vel));
            }
        }
    }
    // the lone lead: long notes with room between phrases
    let melody: [(u32, f64, f64, u8); 12] = [
        (12, 0.0, 3.0, 69),
        (12, 3.0, 1.0, 72),
        (13, 0.0, 4.0, 74),
        (14, 0.0, 2.0, 70),
        (14, 2.0, 2.0, 69),
        (15, 0.0, 4.0, 65),
        (16, 1.0, 3.0, 69),
        (17, 0.0, 2.0, 67),
        (17, 2.0, 2.0, 65),
        (18, 0.0, 3.0, 64),
        (18, 3.0, 1.0, 62),
        (19, 0.0, 4.0, 64),
    ];
    for (bar, beat, len, key) in melody {
        t[lead].notes.push(note(bar, beat, len, key, 96));
    }
    // a slow modulation-wheel swell on the lead's last note
    for i in 0..=16 {
        let tick = note(19, i as f64 * 0.25, 0.0, 0, 0).start;
        t[lead].ctrls.push(Ctrl { tick, kind: CtrlKind::Cc(1), value: (i * 6) as i16 });
    }
    t[lead].ctrls.push(Ctrl { tick: note(20, 0.0, 0.0, 0, 0).start, kind: CtrlKind::Cc(1), value: 0 });
    // the koto: a pentatonic figure, Tokyo at street level
    let penta = [62u8, 65, 67, 69, 72, 74];
    for bar in 16..20 {
        for (i, s) in [0usize, 2, 4, 3, 5, 4, 2, 1].iter().enumerate() {
            if (bar as usize + i) % 3 == 2 {
                continue;
            }
            t[koto].notes.push(note(bar, i as f64 * 0.5, 0.5, penta[*s], 80 + (i as u8 * 5) % 20));
        }
    }
    // the sax answers as the city recedes
    for (bar, beat, len, key) in [(20u32, 1.0, 2.0, 62u8), (20, 3.0, 1.0, 65), (21, 0.0, 3.0, 64), (22, 0.0, 1.5, 62), (22, 1.5, 4.5, 57)] {
        t[sax].notes.push(note(bar, beat, len, key, 88));
    }
    // the city: rain throughout, far-off machinery, thunder, a neon sign buzzing
    t[city].notes.push(note(0, 0.0, 24.0 * 4.0, fx::RAIN, 100));
    t[city].notes.push(note(0, 0.0, 24.0 * 4.0, fx::HUM, 60));
    t[city].notes.push(note(0, 2.0, 1.0, fx::THUNDER, 90));
    t[city].notes.push(note(11, 3.0, 1.0, fx::THUNDER, 110));
    t[city].notes.push(note(4, 0.0, 16.0 * 4.0, fx::NEON, 50));
    t[city].notes.push(note(12, 0.0, 1.0, fx::BRAAM + 2, 90));
    // drums: a slow heartbeat, then a gated snare far off and a ticking clock
    for bar in 8..20 {
        t[drums].notes.push(note(bar, 0.0, 0.5, drum::HEARTBEAT, 100));
        t[drums].notes.push(note(bar, 0.5, 0.5, drum::HEARTBEAT, 70));
        if bar >= 12 {
            t[drums].notes.push(note(bar, 2.0, 0.5, drum::GATED_SNARE, 80));
            for beat in 0..4 {
                t[drums].notes.push(note(bar, beat as f64 + 0.5, 0.25, drum::TICK, 40));
            }
        }
    }
    for tr in p.tracks.iter_mut() {
        tr.sort();
    }
    p
}

/// A blank project with one synth track.
pub fn blank() -> Project {
    let mut p = Project::default();
    p.tracks.push(Track::new("Brass", Source::Synth(by_name("Neon Brass")), 0));
    p.tracks[0].strip.ensemble = true;
    p
}
