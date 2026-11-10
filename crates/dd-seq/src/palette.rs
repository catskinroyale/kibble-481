//! Roles and their alternatives: for each kind of part (brass, pad, bass…) the sounds
//! that can play it, synth patches and SoundFont presets alike. The composer chooses
//! from these, and the Sound tab offers them to swap in one click. Each carries a trim,
//! measured so that swapping keeps the part at about the same loudness.

use dd_dsp::patch::{Category, by_name};
use dd_dsp::sampler::KitKind;
use serde::{Deserialize, Serialize};

use crate::demo::{GM_FONT, SAX_FONT};
use crate::project::{Source, Track};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Brass,
    Pad,
    Strings,
    Lead,
    Answer,
    Bass,
    Bells,
    Pluck,
    Keys,
    Drone,
    City,
    Drums,
}

impl Role {
    pub const ALL: [Role; 12] = [
        Role::Brass,
        Role::Pad,
        Role::Strings,
        Role::Lead,
        Role::Answer,
        Role::Bass,
        Role::Bells,
        Role::Pluck,
        Role::Keys,
        Role::Drone,
        Role::City,
        Role::Drums,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Role::Brass => "Brass",
            Role::Pad => "Pad",
            Role::Strings => "Strings",
            Role::Lead => "Lead",
            Role::Answer => "Answer",
            Role::Bass => "Bass",
            Role::Bells => "Bells",
            Role::Pluck => "Pluck",
            Role::Keys => "Keys",
            Role::Drone => "Drone",
            Role::City => "City",
            Role::Drums => "Drums",
        }
    }

    /// The role of a track: its own, or a guess from its name or sound.
    pub fn of(t: &Track) -> Option<Role> {
        if let Some(r) = t.role {
            return Some(r);
        }
        let name = t.name.to_lowercase();
        if let Some(r) = Role::ALL.iter().find(|r| name.contains(&r.label().to_lowercase())) {
            return Some(*r);
        }
        match &t.source {
            Source::Kit(KitKind::CityFx) => Some(Role::City),
            Source::Kit(KitKind::Drums) => Some(Role::Drums),
            Source::Synth(p) => match p.category {
                Category::Brass => Some(Role::Brass),
                Category::Pad | Category::Atmosphere => Some(Role::Pad),
                Category::Strings => Some(Role::Strings),
                Category::Lead => Some(Role::Lead),
                Category::Keys => Some(Role::Keys),
                Category::Bells => Some(Role::Bells),
                Category::Pluck => Some(Role::Pluck),
                Category::Bass => Some(Role::Bass),
                Category::Drone => Some(Role::Drone),
                Category::Init => None,
            },
            Source::SoundFont { .. } => alternatives_all().into_iter().find(|(_, a)| a.source == t.source).map(|(r, _)| r),
        }
    }
}

/// One sound that can play a role.
#[derive(Clone, Debug, PartialEq)]
pub struct Alt {
    pub label: &'static str,
    pub source: Source,
    /// dB to add to the track's fader so it sits like the role's first sound.
    pub trim_db: f32,
    /// In the app's own SoundFonts (not one fetched by tools/fetch_soundfonts.sh).
    pub bundled: bool,
}

fn synth(label: &'static str, trim_db: f32) -> Alt {
    Alt { label, source: Source::Synth(by_name(label)), trim_db, bundled: true }
}

fn gm(label: &'static str, program: u8, trim_db: f32) -> Alt {
    Alt { label, source: Source::SoundFont { font: GM_FONT.into(), bank: 0, program, name: label.into() }, trim_db, bundled: true }
}

fn font(label: &'static str, file: &str, trim_db: f32, bundled: bool) -> Alt {
    Alt { label, source: Source::SoundFont { font: file.into(), bank: 0, program: 0, name: label.into() }, trim_db, bundled }
}

/// The sounds that can play a role, the usual choice first.
pub fn alternatives(role: Role) -> Vec<Alt> {
    match role {
        Role::Brass => vec![
            synth("Neon Brass", 0.0),
            synth("Skyline Swell", -1.5),
            synth("Velvet Horns", 3.5),
            synth("Tidal Brass", -1.5),
            synth("Dark Stab", 6.0),
            synth("Chrome Fanfare", -0.1),
            synth("Ember Horns", 1.0),
            font("Synth Brass (FreePats)", "FreePats-SynthBrass1.sf2", -8.0, true),
            font("FM Brass (FreePats)", "FreePats-SynthBrass2.sf2", -11.0, true),
            gm("Brass Section", 61, 1.5),
            gm("French Horns", 60, 1.0),
        ],
        Role::Pad => vec![
            synth("Rain Pad", 0.0),
            synth("Orbital Haze", 5.0),
            synth("Vox Haze", 5.0),
            synth("Neon Mist", -0.5),
            synth("Cathedral Sweep", 3.5),
            synth("Night Choir", 1.0),
            font("Sweep Pad (FreePats)", "FreePats-SweepPad.sf2", -5.5, true),
            font("New Age (FreePats)", "FreePats-NewAge.sf2", -6.0, true),
            gm("Warm Pad", 89, 0.5),
            gm("Halo Pad", 94, 3.0),
            font("Synth Choir (FreePats)", "FreePats-SynthPadChoir.sf2", -4.0, false),
        ],
        Role::Strings => vec![
            synth("Memory Strings", 0.0),
            synth("Low Cloud Strings", 0.5),
            synth("Silk Strings", 2.3),
            gm("Strings", 48, -1.5),
            gm("Slow Strings", 49, -1.5),
            gm("Synth Strings", 50, -2.0),
            gm("Cello", 42, -6.0),
        ],
        Role::Lead => vec![
            synth("Smoky Lead", 0.0),
            font("Tenor Sax (FreePats)", SAX_FONT, -7.0, true),
            synth("Ribbon Lead", 3.5),
            synth("Glass Lead", 0.5),
            synth("Wide Saw Lead", 3.5),
            synth("Lonely Horn", 1.6),
            gm("Alto Sax", 65, 3.0),
            gm("Shakuhachi", 77, 1.0),
            gm("Flute", 73, -2.5),
        ],
        Role::Answer => vec![
            font("Tenor Sax (FreePats)", SAX_FONT, -6.0, true),
            gm("Shakuhachi", 77, 1.5),
            gm("Choir", 52, 3.0),
            gm("Koto", 107, 6.0),
            gm("Pan Flute", 75, -1.5),
            gm("Oboe", 68, -1.0),
            synth("Glass Lead", 1.5),
            synth("Lonely Horn", 2.4),
        ],
        Role::Bass => vec![
            synth("Sub Pulse Bass", 0.0),
            synth("Undercity Bass", 3.0),
            synth("Rubber Bass", 3.0),
            synth("Pure Sub", -0.5),
            synth("Night Drive Bass", 1.5),
            synth("Brass Bass", -2.5),
            gm("Synth Bass", 38, -2.5),
            gm("Fretless", 35, -5.5),
            gm("Upright Bass", 32, -2.0),
        ],
        Role::Bells => vec![
            synth("Night Bells", 0.0),
            synth("Glass Chimes", 4.0),
            synth("Celesta Glow", 2.5),
            synth("Music Box", 3.0),
            synth("Rain Vibes", 2.5),
            synth("Ring Bells", 1.3),
            gm("Celesta", 8, 0.5),
            gm("Glockenspiel", 9, 4.5),
            gm("Vibraphone", 11, 4.0),
            gm("Tubular Bells", 14, 2.0),
            font("Tubular Bells (FreePats)", "FreePats-TubularBells.sf2", -4.0, false),
        ],
        Role::Pluck => vec![
            gm("Koto", 107, 0.0),
            synth("Lantern Pluck", 0.0),
            synth("Rain Harp", 3.0),
            gm("Shamisen", 106, 1.5),
            gm("Harp", 46, -4.0),
            gm("Nylon Guitar", 24, -1.0),
            gm("Kalimba", 108, -5.0),
        ],
        Role::Keys => vec![
            synth("Neon Tine Piano", 0.0),
            synth("Polysynth Keys", 0.5),
            synth("Neon Clav", 2.5),
            synth("Velour Organ", -2.0),
            gm("Grand Piano", 0, -3.5),
            gm("Electric Piano", 4, -1.0),
            gm("FM Piano", 5, -2.0),
            font("FM Piano (FreePats)", "FreePats-FM-Piano1.sf2", -4.0, false),
            font("Upright Piano (FreePats)", "FreePats-UprightPianoKW.sf2", -4.0, false),
        ],
        Role::Drone => vec![
            synth("Abyss Drone", 0.0),
            synth("Ozone Drone", 3.5),
            synth("Choir Drone", 5.0),
            synth("Glass Drone", 2.5),
            synth("Sodium Drone", -3.0),
            gm("Bowed Glass", 92, 3.0),
        ],
        Role::City => vec![Alt { label: "City FX", source: Source::Kit(KitKind::CityFx), trim_db: 0.0, bundled: true }],
        Role::Drums => vec![
            Alt { label: "Noir Drum Kit", source: Source::Kit(KitKind::Drums), trim_db: 0.0, bundled: true },
            Alt {
                label: "GM Standard Kit",
                source: Source::SoundFont { font: GM_FONT.into(), bank: 128, program: 0, name: "Standard".into() },
                trim_db: 8.0,
                bundled: true,
            },
            Alt {
                label: "GM Room Kit",
                source: Source::SoundFont { font: GM_FONT.into(), bank: 128, program: 8, name: "Room".into() },
                trim_db: 10.0,
                bundled: true,
            },
        ],
    }
}

fn alternatives_all() -> Vec<(Role, Alt)> {
    Role::ALL.iter().flat_map(|r| alternatives(*r).into_iter().map(move |a| (*r, a))).collect()
}

/// Whether two sources are the same sound (synth patches by name, so a tweaked patch
/// still counts as itself).
pub fn same_sound(a: &Source, b: &Source) -> bool {
    match (a, b) {
        (Source::Synth(x), Source::Synth(y)) => x.name == y.name,
        (Source::SoundFont { font: f1, bank: b1, program: p1, .. }, Source::SoundFont { font: f2, bank: b2, program: p2, .. }) => {
            f1 == f2 && b1 == b2 && p1 == p2
        }
        _ => a == b,
    }
}

/// Where a track's sound sits among its role's alternatives.
pub fn position(role: Role, source: &Source) -> Option<usize> {
    alternatives(role).iter().position(|a| same_sound(&a.source, source))
}

/// Swap a track to another alternative, keeping its loudness by moving the fader by
/// the difference in trims.
pub fn apply(t: &mut Track, role: Role, alt: &Alt) {
    let old_trim = position(role, &t.source).map_or(0.0, |i| alternatives(role)[i].trim_db);
    t.strip.gain_db = (t.strip.gain_db - old_trim + alt.trim_db).clamp(-60.0, 12.0);
    t.source = alt.source.clone();
    t.role = Some(role);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{FontBank, make_instrument};

    fn fonts() -> FontBank {
        let mut f = FontBank::default();
        f.load_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soundfonts"));
        f
    }

    /// Every bundled alternative exists: the synth patch, or the font and preset.
    #[test]
    fn every_bundled_alternative_exists() {
        let f = fonts();
        let names: Vec<String> = dd_dsp::patch::factory().into_iter().map(|p| p.name).collect();
        for (role, a) in alternatives_all() {
            match &a.source {
                Source::Synth(p) => assert!(names.contains(&p.name) && p.name == a.label, "{role:?}: no patch {}", a.label),
                Source::SoundFont { font, bank, program, .. } if a.bundled => {
                    let lf = f.get(font).unwrap_or_else(|| panic!("{font} isn't bundled"));
                    assert!(lf.find(*bank, *program).is_some(), "{role:?}: {font} has no {bank}:{program}");
                }
                _ => {}
            }
        }
    }

    /// The level of a sound playing what its role typically plays, in dB: its loudest
    /// 400 ms.
    fn level(role: Role, a: &Alt, f: &FontBank) -> f32 {
        let sr = 24000;
        let (mut inst, err) = make_instrument(&a.source, f, sr);
        assert!(err.is_none(), "{err:?}");
        let keys: &[u8] = match role {
            Role::Bass => &[40],
            Role::Drone => &[38, 45],
            Role::Lead | Role::Answer => &[67],
            Role::Bells | Role::Pluck => &[76],
            Role::Drums => &[36, 38, 42],
            Role::City => &[66],
            _ => &[60, 63, 67],
        };
        for k in keys {
            inst.note_on(*k, 100);
        }
        let secs = if role == Role::Drone { 8 } else { 3 };
        let (mut l, mut r) = (vec![0.0f32; sr as usize * secs], vec![0.0f32; sr as usize * secs]);
        for (a, b) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            inst.render(a, b);
        }
        // the loudest 400 ms: how loud it strikes the ear, fair to hits and held sounds alike
        let w = sr as usize * 2 / 5;
        let e = (0..l.len() - w)
            .step_by(w / 4)
            .map(|i| (i..i + w).map(|j| (l[j] as f64).powi(2) + (r[j] as f64).powi(2)).sum::<f64>() / (2 * w) as f64)
            .fold(0.0f64, f64::max);
        (10.0 * e.max(1e-12).log10()) as f32
    }

    /// With their trims, a role's alternatives play within a few dB of each other.
    #[test]
    fn alternatives_are_level_matched() {
        let f = fonts();
        let mut report = String::new();
        let mut worst = (0.0f32, String::new());
        for role in Role::ALL {
            let alts: Vec<Alt> = alternatives(role).into_iter().filter(|a| a.bundled).collect();
            let base = level(role, &alts[0], &f) + alts[0].trim_db;
            for a in &alts {
                let d = level(role, a, &f) + a.trim_db - base;
                report += &format!("{:?} {:<26} {:+.1} dB (trim {:+.1})\n", role, a.label, d, a.trim_db);
                if d.abs() > worst.0.abs() {
                    worst = (d, format!("{role:?} {}", a.label));
                }
            }
        }
        println!("{report}");
        assert!(worst.0.abs() <= 3.0, "{} is {:+.1} dB from its role's usual sound\n{report}", worst.1, worst.0);
    }
}
