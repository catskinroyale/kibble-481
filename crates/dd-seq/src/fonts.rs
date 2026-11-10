//! SoundFonts: the loaded fonts, their presets, and a track instrument that plays one
//! preset through rustysynth with its own built-in reverb off (the mixer's is used).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dd_dsp::Instrument;
use dd_dsp::patch::Patch;
use dd_dsp::poly::PolySynth;
use dd_dsp::sampler::KitPlayer;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};

use crate::project::Source;

#[derive(Clone, Debug, PartialEq)]
pub struct PresetInfo {
    pub bank: u16,
    pub program: u8,
    pub name: String,
}

#[derive(Clone)]
pub struct LoadedFont {
    /// The file name, which projects refer to it by.
    pub name: String,
    pub path: Option<PathBuf>,
    pub font: Arc<SoundFont>,
    pub presets: Vec<PresetInfo>,
}

impl LoadedFont {
    pub fn load(path: &Path) -> Result<LoadedFont, String> {
        let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let font = SoundFont::new(&mut file).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let mut presets: Vec<PresetInfo> = font
            .get_presets()
            .iter()
            .map(|p| PresetInfo {
                bank: p.get_bank_number() as u16,
                program: p.get_patch_number() as u8,
                name: p.get_name().trim().to_string(),
            })
            .collect();
        presets.sort_by_key(|p| (p.bank, p.program));
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        Ok(LoadedFont { name, path: Some(path.to_path_buf()), font: Arc::new(font), presets })
    }

    /// The preset to show for a bank and program, if the font has it.
    pub fn find(&self, bank: u16, program: u8) -> Option<&PresetInfo> {
        self.presets.iter().find(|p| p.bank == bank && p.program == program)
    }
}

/// Every loaded SoundFont.
#[derive(Clone, Default)]
pub struct FontBank {
    pub fonts: Vec<LoadedFont>,
}

impl FontBank {
    /// Load every .sf2 in a folder, in name order. Returns the errors.
    pub fn load_dir(&mut self, dir: &Path) -> Vec<String> {
        let mut errors = Vec::new();
        let Ok(rd) = std::fs::read_dir(dir) else { return vec![format!("No SoundFont folder at {}", dir.display())] };
        let mut paths: Vec<PathBuf> =
            rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sf2"))).collect();
        paths.sort();
        for p in paths {
            if let Err(e) = self.add(&p) {
                errors.push(e);
            }
        }
        // the optional fonts from tools/fetch_soundfonts.sh
        let extra = dir.join("extra");
        if extra.is_dir() {
            errors.extend(self.load_dir(&extra));
        }
        errors
    }

    pub fn add(&mut self, path: &Path) -> Result<(), String> {
        let f = LoadedFont::load(path)?;
        self.fonts.retain(|x| x.name != f.name);
        self.fonts.push(f);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&LoadedFont> {
        self.fonts.iter().find(|f| f.name == name)
    }
}

/// One SoundFont preset on one channel.
pub struct FontInstrument {
    synth: Synthesizer,
    ch: i32,
    bend: i16,
    tuning: f32,
}

/// GM's default bend range, in semitones.
const BEND_RANGE: f32 = 2.0;

impl FontInstrument {
    pub fn new(font: &Arc<SoundFont>, bank: u16, program: u8, sample_rate: u32) -> Result<Self, String> {
        let mut settings = SynthesizerSettings::new(sample_rate as i32);
        settings.enable_reverb_and_chorus = false;
        let mut synth = Synthesizer::new(font, &settings).map_err(|e| format!("{e:?}"))?;
        synth.set_master_volume(1.6);
        // rustysynth treats channel 10 as the drum channel
        let ch = if bank == 128 { 9 } else { 0 };
        if bank != 128 {
            synth.process_midi_message(ch, 0xB0, 0, bank.min(127) as i32);
        }
        synth.process_midi_message(ch, 0xC0, program as i32, 0);
        Ok(FontInstrument { synth, ch, bend: 0, tuning: 0.0 })
    }

    fn send_bend(&mut self) {
        // tuning rides on the bend, which GM sets to ±2 semitones
        let total = self.bend as f32 / 8192.0 + self.tuning / (BEND_RANGE * 100.0);
        let v = (8192.0 + total * 8192.0).round().clamp(0.0, 16383.0) as i32;
        self.synth.process_midi_message(self.ch, 0xE0, v & 0x7f, v >> 7);
    }
}

impl Instrument for FontInstrument {
    fn note_on(&mut self, key: u8, vel: u8) {
        self.synth.note_on(self.ch, key as i32, vel as i32);
    }
    fn note_off(&mut self, key: u8) {
        self.synth.note_off(self.ch, key as i32);
    }
    fn control(&mut self, cc: u8, value: u8) {
        match cc {
            120 => self.synth.note_off_all(true),
            123 => self.synth.note_off_all(false),
            // brightness has no meaning to most presets; leave it out
            74 => {}
            _ => self.synth.process_midi_message(self.ch, 0xB0, cc as i32, value as i32),
        }
    }
    fn bend(&mut self, value: i16) {
        self.bend = value;
        self.send_bend();
    }
    fn pressure(&mut self, value: u8) {
        // touch as vibrato, the nearest thing most presets have
        self.synth.process_midi_message(self.ch, 0xB0, 1, value as i32);
    }
    fn all_notes_off(&mut self) {
        self.synth.note_off_all(false);
    }
    fn reset(&mut self) {
        self.synth.note_off_all(true);
        self.bend = 0;
        self.send_bend();
    }
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.synth.render(left, right);
    }
    fn set_tuning(&mut self, cents: f32) {
        self.tuning = cents;
        self.send_bend();
    }
}

/// Plays nothing: a track whose SoundFont isn't loaded.
pub struct Silent;

impl Instrument for Silent {
    fn note_on(&mut self, _: u8, _: u8) {}
    fn note_off(&mut self, _: u8) {}
    fn all_notes_off(&mut self) {}
    fn reset(&mut self) {}
    fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
    }
}

/// The instrument for a track's source. A SoundFont that isn't loaded plays silence,
/// and the error says why.
pub fn make_instrument(source: &Source, fonts: &FontBank, sample_rate: u32) -> (Box<dyn Instrument>, Option<String>) {
    match source {
        Source::Synth(p) => (Box::new(PolySynth::new(sample_rate, p.clone())), None),
        Source::Kit(k) => (Box::new(KitPlayer::new(*k, sample_rate)), None),
        Source::SoundFont { font, bank, program, name } => match fonts.get(font) {
            Some(f) => match FontInstrument::new(&f.font, *bank, *program, sample_rate) {
                Ok(i) => (Box::new(i), None),
                Err(e) => (Box::new(Silent), Some(format!("{name}: {e}"))),
            },
            None => (Box::new(Silent), Some(format!("{name} needs the SoundFont {font}, which isn't loaded"))),
        },
    }
}

/// The patch of a synth source, for live edits.
pub fn patch_of(source: &Source) -> Option<&Patch> {
    match source {
        Source::Synth(p) => Some(p),
        _ => None,
    }
}

/// Where to look for the bundled SoundFonts, in order: `$DDREAM_SOUNDFONTS`, the
/// macOS app bundle's Resources, beside the executable, the working directory, and
/// the source tree this was built from.
pub fn soundfont_dir() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::var_os("DDREAM_SOUNDFONTS") {
        candidates.push(d.into());
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("../Resources/soundfonts"));
        candidates.push(dir.join("soundfonts"));
    }
    candidates.push(PathBuf::from("soundfonts"));
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../soundfonts"));
    candidates.into_iter().find(|d| d.join(crate::demo::GM_FONT).is_file())
}
