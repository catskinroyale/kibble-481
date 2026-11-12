//! A project: tracks of notes and controller moves on a beat grid at one tempo, the
//! score's sections, and the mix (each track's channel strip and the shared effects).

use dd_dsp::fx::Fx;
use dd_dsp::patch::Patch;
use dd_dsp::sampler::KitKind;
use serde::{Deserialize, Serialize};

/// Ticks per quarter note.
pub const PPQ: u32 = 960;
/// Seconds of reverb tail after the last bar.
pub const TAIL_SECS: f64 = 4.0;
pub const SILENT_DB: f32 = -60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub start: u32,
    pub len: u32,
    pub key: u8,
    pub vel: u8,
}

impl Note {
    pub fn end(&self) -> u32 {
        self.start + self.len
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CtrlKind {
    /// A MIDI controller number: 1 modulation, 7 volume, 11 expression, 64 sustain, 74 brightness.
    Cc(u8),
    /// -8192..=8191.
    Bend,
    /// Channel pressure (aftertouch), 0..=127.
    Pressure,
}

impl CtrlKind {
    pub fn label(self) -> String {
        match self {
            CtrlKind::Cc(1) => "Mod wheel".into(),
            CtrlKind::Cc(7) => "Volume".into(),
            CtrlKind::Cc(11) => "Expression".into(),
            CtrlKind::Cc(64) => "Sustain".into(),
            CtrlKind::Cc(74) => "Brightness".into(),
            CtrlKind::Cc(n) => format!("CC {n}"),
            CtrlKind::Bend => "Pitch bend".into(),
            CtrlKind::Pressure => "Touch".into(),
        }
    }

    /// The value when nothing has moved it.
    pub fn rest(self) -> i16 {
        match self {
            CtrlKind::Cc(7) => 100,
            CtrlKind::Cc(11) => 127,
            CtrlKind::Cc(74) => 64,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ctrl {
    pub tick: u32,
    pub kind: CtrlKind,
    pub value: i16,
}

/// What plays a track. (One per track, so the patch isn't boxed.)
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Source {
    Synth(Patch),
    /// A preset of a loaded SoundFont, by the font's file name. Bank 128 is the drum bank.
    SoundFont {
        font: String,
        bank: u16,
        program: u8,
        name: String,
    },
    Kit(KitKind),
}

impl Source {
    pub fn label(&self) -> String {
        match self {
            Source::Synth(p) => p.name.clone(),
            Source::SoundFont { name, .. } => name.clone(),
            Source::Kit(k) => k.label().into(),
        }
    }

    pub fn is_drums(&self) -> bool {
        matches!(self, Source::Kit(KitKind::Drums) | Source::SoundFont { bank: 128, .. })
    }
}

/// A track's mixer channel.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Strip {
    /// Fader in dB; SILENT_DB is off.
    pub gain_db: f32,
    /// -1 = left, 1 = right.
    pub pan: f32,
    /// Stereo width: 0 = mono, 1 = as played, 2 = wider.
    pub width: f32,
    /// Reverb send, 0..1.
    pub reverb: f32,
    /// Echo send, 0..1.
    pub echo: f32,
    /// Play through the string-machine ensemble chorus.
    pub ensemble: bool,
    pub mute: bool,
    pub solo: bool,
}

impl Default for Strip {
    fn default() -> Self {
        Strip { gain_db: 0.0, pan: 0.0, width: 1.0, reverb: 0.35, echo: 0.0, ensemble: false, mute: false, solo: false }
    }
}

impl Strip {
    pub fn gain(&self) -> f32 {
        db_gain(self.gain_db)
    }
}

pub fn db_gain(db: f32) -> f32 {
    if db <= SILENT_DB { 0.0 } else { 10f32.powf(db / 20.0) }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Track {
    pub name: String,
    pub color: [u8; 3],
    pub source: Source,
    pub notes: Vec<Note>,
    pub ctrls: Vec<Ctrl>,
    pub strip: Strip,
    /// The part it plays (brass, pad, bass…), which decides the alternatives offered.
    pub role: Option<crate::palette::Role>,
    /// Its volume set bar by bar, on top of the fader (see `Level`).
    pub levels: Vec<Level>,
}

/// A track's volume from the start of a bar (from 0): 100 is as the fader has it, 0 is
/// silent, 127 about 4 dB louder. Between two bars that have one, the volume ramps from
/// one to the next; before the first it is the first, after the last it stays the last.
/// So a bar that has none, a new one included, follows the bars either side.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Level {
    pub bar: u32,
    pub value: u8,
}

/// The value 100 means "as the fader has it".
pub const LEVEL_UNITY: u8 = 100;

/// The gain of a level value: (value / 100)², like a MIDI volume controller.
pub fn level_gain(value: f32) -> f32 {
    let x = value.max(0.0) / LEVEL_UNITY as f32;
    x * x
}

/// The level value at a position in bars (fractional), from points sorted by bar.
pub fn level_at(levels: &[Level], bar: f64) -> f32 {
    let Some(first) = levels.first() else { return LEVEL_UNITY as f32 };
    if bar <= first.bar as f64 {
        return first.value as f32;
    }
    for w in levels.windows(2) {
        let (a, b) = (w[0], w[1]);
        if bar < b.bar as f64 {
            let t = (bar - a.bar as f64) / (b.bar - a.bar).max(1) as f64;
            return a.value as f32 + (b.value as f32 - a.value as f32) * t as f32;
        }
    }
    levels.last().map_or(LEVEL_UNITY as f32, |l| l.value as f32)
}

/// Set a bar's level, keeping the points sorted and one per bar.
pub fn set_level(levels: &mut Vec<Level>, bar: u32, value: u8) {
    match levels.binary_search_by_key(&bar, |l| l.bar) {
        Ok(i) => levels[i].value = value,
        Err(i) => levels.insert(i, Level { bar, value }),
    }
}

impl Default for Track {
    fn default() -> Self {
        Track {
            name: "Track".into(),
            color: NEON[0],
            source: Source::Synth(Patch::default()),
            notes: Vec::new(),
            ctrls: Vec::new(),
            strip: Strip::default(),
            role: None,
            levels: Vec::new(),
        }
    }
}

/// Track colours: neon signs in the rain.
pub const NEON: [[u8; 3]; 8] = [
    [255, 64, 160],  // magenta
    [0, 220, 255],   // cyan
    [255, 170, 40],  // sodium amber
    [150, 90, 255],  // violet
    [60, 255, 170],  // jade
    [255, 80, 70],   // tail-light red
    [90, 150, 255],  // police blue
    [230, 230, 120], // pale yellow
];

impl Track {
    pub fn new(name: &str, source: Source, index: usize) -> Self {
        Track { name: name.into(), color: NEON[index % NEON.len()], source, ..Track::default() }
    }

    pub fn sort(&mut self) {
        self.notes.sort_by_key(|n| (n.start, n.key));
        self.ctrls.sort_by_key(|c| c.tick);
    }

    pub fn end(&self) -> u32 {
        let n = self.notes.iter().map(|n| n.end()).max().unwrap_or(0);
        let c = self.ctrls.iter().map(|c| c.tick).max().unwrap_or(0);
        n.max(c)
    }
}

/// The kinds of section the composer writes: rain and a pad alone, the brass and bass
/// coming in, everything with the lead on top, and the answer as it all recedes. A
/// piece can use them in any order, as often as it likes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Part {
    Neon,
    Swell,
    NightShift,
    Fade,
}

impl Part {
    pub const ALL: [Part; 4] = [Part::Neon, Part::Swell, Part::NightShift, Part::Fade];

    pub fn label(self) -> &'static str {
        match self {
            Part::Neon => "Neon",
            Part::Swell => "Swell",
            Part::NightShift => "Night shift",
            Part::Fade => "Fade",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Part::Neon => "Rain, the pad and the drone, with bells",
            Part::Swell => "The brass and bass come in, and a heartbeat",
            Part::NightShift => "Everything, with the lead melody, plucks and drums",
            Part::Fade => "The brass and bass drop out; the answer plays as it recedes",
        }
    }

    /// The part a section name stands for (also the names older pieces used).
    pub fn from_name(name: &str) -> Option<Part> {
        let n = name.trim().to_lowercase();
        match n.as_str() {
            "neon" | "rain" | "streets" | "drizzle" => Some(Part::Neon),
            "swell" | "brass" | "pulse" | "overpass" => Some(Part::Swell),
            "night shift" | "lone lead" | "ascent" | "skyline" => Some(Part::NightShift),
            "fade" | "recede" | "afterglow" | "dawn" => Some(Part::Fade),
            _ => None,
        }
    }
}

/// A named section of the score, starting on a bar (from 0).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub bar: u32,
    pub name: String,
    /// What the composer writes in it; none for a plain marker (from a MIDI file).
    #[serde(default)]
    pub part: Option<Part>,
}

impl Section {
    pub fn new(bar: u32, part: Part) -> Section {
        Section { bar, name: part.label().into(), part: Some(part) }
    }

    /// A marker read from a file: a part if its name is one.
    pub fn marker(bar: u32, name: String) -> Section {
        Section { bar, part: Part::from_name(&name), name }
    }
}

/// One entry of an arrangement: a section and how many bars it lasts.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub name: String,
    pub part: Option<Part>,
    pub bars: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub version: u32,
    pub title: String,
    pub bpm: f64,
    /// Beats per bar, and the beat's note value.
    pub meter: (u32, u32),
    /// The shortest the piece is, in bars; it grows to hold every note.
    pub length_bars: u32,
    pub tracks: Vec<Track>,
    pub sections: Vec<Section>,
    pub fx: Fx,
    pub master_db: f32,
    /// Loop playback between these bars (from 0, end exclusive).
    pub loop_bars: Option<(u32, u32)>,
    /// The composer's recipe, when the piece was composed.
    pub recipe: Option<crate::compose::Recipe>,
    /// The fingerprint of the notes as composed: while it still matches, the piece is
    /// written again to fit a new tempo or length.
    pub composed: u64,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            version: 1,
            title: "Untitled".into(),
            bpm: 72.0,
            meter: (4, 4),
            length_bars: 16,
            tracks: Vec::new(),
            sections: Vec::new(),
            fx: noir_fx(),
            master_db: 0.0,
            loop_bars: None,
            recipe: None,
            composed: 0,
        }
    }
}

/// The house sound: the vast, dark Noir hall, a slow tape echo and a touch of ensemble.
pub fn noir_fx() -> Fx {
    use dd_dsp::fx::{Echo, EchoTime, Space};
    Fx {
        space: Space::Noir,
        size: 0.6,
        tone: 0.45,
        motion: 0.6,
        predelay: 0.15,
        low_cut: 0.55,
        width: 0.75,
        ensemble: 0.45,
        // a little tape wow; more pulls the repeats audibly out of tune
        echo: Echo { time: EchoTime::DottedQuarter, feedback: 0.5, tone: 0.3, ping_pong: true, to_reverb: 0.6, wow: 0.08 },
        ..Fx::default()
    }
}

impl Project {
    pub fn beat_ticks(&self) -> u32 {
        PPQ * 4 / self.meter.1.max(1)
    }

    pub fn bar_ticks(&self) -> u32 {
        self.beat_ticks() * self.meter.0.max(1)
    }

    pub fn secs_per_tick(&self) -> f64 {
        60.0 / self.bpm.max(1.0) / PPQ as f64
    }

    pub fn tick_secs(&self, tick: u32) -> f64 {
        tick as f64 * self.secs_per_tick()
    }

    pub fn secs_tick(&self, secs: f64) -> u32 {
        (secs.max(0.0) / self.secs_per_tick()).round() as u32
    }

    pub fn bar_secs(&self) -> f64 {
        self.tick_secs(self.bar_ticks())
    }

    /// The piece's length in whole bars: at least `length_bars`, and enough to hold
    /// every note.
    pub fn bars(&self) -> u32 {
        let end = self.tracks.iter().map(|t| t.end()).max().unwrap_or(0);
        end.div_ceil(self.bar_ticks()).max(self.length_bars).max(1)
    }

    pub fn end_tick(&self) -> u32 {
        self.bars() * self.bar_ticks()
    }

    /// The music's length in seconds (whole bars), without the tail.
    pub fn music_secs(&self) -> f64 {
        self.tick_secs(self.end_tick())
    }

    /// "bar.beat.tick" (bar and beat from 1).
    pub fn position(&self, tick: u32) -> String {
        let bar = tick / self.bar_ticks();
        let beat = tick % self.bar_ticks() / self.beat_ticks();
        let rest = tick % self.beat_ticks();
        format!("{}.{}.{:03}", bar + 1, beat + 1, rest * 1000 / self.beat_ticks())
    }

    pub fn audible(&self, track: usize) -> bool {
        let any_solo = self.tracks.iter().any(|t| t.strip.solo);
        self.tracks.get(track).is_some_and(|t| !t.strip.mute && (!any_solo || t.strip.solo))
    }

    /// The bars where sections start.
    pub fn section_bars(&self) -> Vec<u32> {
        let mut b: Vec<u32> = self.sections.iter().map(|s| s.bar).collect();
        b.sort_unstable();
        b.dedup();
        b
    }

    /// The sections in order with their lengths: each lasts until the next, the last
    /// until the end. A piece whose first section starts later gets an unnamed
    /// lead-in.
    pub fn arrangement(&self) -> Vec<Block> {
        let mut secs = self.sections.clone();
        secs.sort_by_key(|s| s.bar);
        secs.dedup_by_key(|s| s.bar);
        let end = self.length_bars.max(secs.last().map_or(0, |s| s.bar + 1));
        let mut out = Vec::new();
        if let Some(first) = secs.first()
            && first.bar > 0
        {
            out.push(Block { name: String::new(), part: None, bars: first.bar });
        }
        for (i, s) in secs.iter().enumerate() {
            let to = secs.get(i + 1).map_or(end, |n| n.bar);
            out.push(Block { name: s.name.clone(), part: s.part.or_else(|| Part::from_name(&s.name)), bars: to - s.bar });
        }
        out
    }

    /// Lay the sections out from an arrangement; the piece becomes as long as they are.
    pub fn set_arrangement(&mut self, blocks: &[Block]) {
        let mut bar = 0;
        self.sections.clear();
        for b in blocks {
            if !b.name.is_empty() || b.part.is_some() {
                self.sections.push(Section { bar, name: b.name.clone(), part: b.part });
            }
            bar += b.bars.max(1);
        }
        if bar > 0 {
            self.length_bars = bar;
        }
    }

    /// The arrangement the composer can write: every section a part, from bar 1.
    pub fn plan(&self) -> Option<Vec<(Part, u32)>> {
        if self.sections.is_empty() {
            return None;
        }
        self.arrangement().into_iter().map(|b| b.part.map(|p| (p, b.bars))).collect()
    }

    /// Lay out a new arrangement whose blocks came from the old one (`origin[i]` is the
    /// old block new block `i` was, or none for a new one), moving each track's bar
    /// levels with their sections. A block that got shorter keeps its last level at its
    /// new end; a new block has none, so it follows its neighbours.
    pub fn rearrange(&mut self, blocks: &[Block], origin: &[Option<usize>]) {
        let old = self.arrangement();
        if old.is_empty() {
            self.set_arrangement(blocks);
            return;
        }
        let starts = |bs: &[Block]| -> Vec<u32> {
            let mut at = 0;
            bs.iter()
                .map(|b| {
                    let s = at;
                    at += b.bars.max(1);
                    s
                })
                .collect()
        };
        let (old_starts, new_starts) = (starts(&old), starts(blocks));
        let old_end = old_starts.last().zip(old.last()).map_or(0, |(s, b)| s + b.bars.max(1));
        for t in self.tracks.iter_mut() {
            if t.levels.is_empty() {
                continue;
            }
            let mut moved: Vec<Level> = Vec::new();
            for (i, from) in origin.iter().enumerate() {
                let Some(j) = *from else { continue };
                let (Some(&os), Some(ob)) = (old_starts.get(j), old.get(j)) else { continue };
                let new_len = blocks[i].bars.max(1);
                // the points in the old block, the last one standing for any cut off
                let mut inside: Vec<Level> = t.levels.iter().filter(|l| l.bar >= os && l.bar < os + ob.bars.max(1)).copied().collect();
                inside.sort_by_key(|l| l.bar);
                for l in inside {
                    let off = (l.bar - os).min(new_len - 1);
                    set_level(&mut moved, new_starts[i] + off, l.value);
                }
            }
            // points past the old arrangement's end (set beyond the sections) stay put
            for l in t.levels.iter().filter(|l| l.bar >= old_end) {
                let bar = l.bar - old_end + new_starts.last().zip(blocks.last()).map_or(0, |(s, b)| s + b.bars.max(1));
                if moved.binary_search_by_key(&bar, |m| m.bar).is_err() {
                    set_level(&mut moved, bar, l.value);
                }
            }
            t.levels = moved;
        }
        self.set_arrangement(blocks);
    }

    /// Change the length, sharing the change among the sections in proportion to their
    /// lengths (each keeps at least a bar).
    pub fn stretch_to(&mut self, bars: u32) {
        let blocks = self.arrangement();
        if blocks.is_empty() || bars < blocks.len() as u32 {
            self.length_bars = bars;
            return;
        }
        let lens: Vec<u32> = blocks.iter().map(|b| b.bars).collect();
        let total: u32 = lens.iter().sum();
        let mut fresh: Vec<u32> = lens.iter().map(|l| ((*l as u64 * bars as u64 / total.max(1) as u64) as u32).max(1)).collect();
        // hand out what rounding left, to the longest first, or take it back from them
        let mut order: Vec<usize> = (0..fresh.len()).collect();
        order.sort_by_key(|i| std::cmp::Reverse(lens[*i]));
        let mut k = 0;
        while fresh.iter().sum::<u32>() != bars && k < 10_000 {
            let i = order[k % order.len()];
            if fresh.iter().sum::<u32>() < bars {
                fresh[i] += 1;
            } else if fresh[i] > 1 {
                fresh[i] -= 1;
            }
            k += 1;
        }
        let blocks: Vec<Block> = blocks.into_iter().zip(fresh).map(|(b, n)| Block { bars: n, ..b }).collect();
        let origin: Vec<Option<usize>> = (0..blocks.len()).map(Some).collect();
        self.rearrange(&blocks, &origin);
    }

    /// Composed, and not edited by hand since.
    pub fn untouched(&self) -> bool {
        self.recipe.is_some() && crate::compose::fingerprint(self) == self.composed
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Project, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}
