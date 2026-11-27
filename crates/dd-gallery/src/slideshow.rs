// Adapted from Ambient MIDI Generator (src/slideshow.rs, MIT): the planning is unchanged;
// `Accents::of` reads a project's tracks and sections instead of a generated song.
//! Fitting a photo slideshow to the music: how many images a track holds, or
//! how long a track a number of images needs. Images change on bar lines
//! (or, when no whole number of bars fits, on half bars or beats), after an
//! intro and before an outro of whole bars. Following the music, each
//! change moves to where the music marks a moment (a new section, a phrase,
//! a chord change, a part coming in or dropping out), so the pictures turn
//! with it rather than on an even count across it.

use dd_seq::project::{Project, TAIL_SECS as REVERB_TAIL};

use serde::{Deserialize, Serialize};

/// The shortest and longest time an image may be on screen, fades included.
/// The project's ticks per quarter note.
const PPQ: u32 = dd_seq::project::PPQ;

pub const MIN_HOLD: f64 = 4.0;
pub const MAX_HOLD: f64 = 16.0;
/// The longest crossfade: half the longest time on screen.
pub const MAX_FADE: f64 = MAX_HOLD / 2.0;

/// The most bars an image may be given.
pub const MAX_IMAGE_BARS: u32 = 64;
/// The tempo range the app offers.
pub const MIN_BPM: f64 = 30.0;
pub const MAX_BPM: f64 = 220.0;

/// The tempo (in the app's beats per minute, of the metre's lower note) at which
/// `bars` bars last `secs` seconds; `None` outside the tempo range.
pub fn tempo_for(meter: (u32, u32), bars: u32, secs: f64) -> Option<f64> {
    let beats = (bars.max(1) * meter.0.max(1)) as f64;
    let bpm = 60.0 * beats / secs.max(1e-6) * 4.0 / meter.1.max(1) as f64;
    // to a tenth, as the tempo is shown
    let bpm = (bpm * 10.0).round() / 10.0;
    (MIN_BPM..=MAX_BPM).contains(&bpm).then_some(bpm)
}

/// The tempo nearest `bpm` at which a score of `score_bars` holds `photos` images at
/// these settings exactly as the calculator counts them (whole tenths of a bpm), if
/// one in the tempo range does. With images in whole bars the tempo changes nothing
/// of the count, so there's none unless it fits already.
pub fn tempo_to_fit(s: &Slideshow, meter: (u32, u32), bpm: f64, score_bars: u32, photos: u32) -> Option<f64> {
    let fits = |b: f64| {
        let grid = Grid::new(b, meter);
        Calc::new(s, grid, score_bars as f64 * grid.bar(), photos).needed_bars == score_bars
    };
    if fits(bpm) {
        return Some(bpm);
    }
    if s.even_bars() {
        return None;
    }
    let tenths = |b: f64| (b * 10.0).round() as i64;
    let (lo, hi, at) = (tenths(MIN_BPM), tenths(MAX_BPM), tenths(bpm));
    (1..=(hi - lo)).flat_map(|d| [at - d, at + d]).filter(|t| (lo..=hi).contains(t)).map(|t| t as f64 / 10.0).find(|b| fits(*b))
}

/// The shortest and longest exact time on screen.
pub const EXACT_MIN: f64 = 1.0;
pub const EXACT_MAX: f64 = 60.0;

/// How one image gives way to the next on the exported timeline and video.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transition {
    /// The next image dissolves in over the last, centred on the change.
    #[default]
    Crossfade,
    /// Each image fades out to black, then the next fades in, centred on the change.
    FadeToBlack,
    /// Straight from one to the next.
    Cut,
    /// Every image fades up from black as it starts (`fade_in`), and cuts away at its end.
    FadeIn,
    /// Every image cuts in, and fades out to black as it ends (`fade_out`).
    FadeOut,
    /// Every image fades up from black (`fade_in`) and out to black (`fade_out`).
    FadeInOut,
}

impl Transition {
    pub const ALL: [Transition; 6] =
        [Transition::Crossfade, Transition::FadeToBlack, Transition::Cut, Transition::FadeIn, Transition::FadeOut, Transition::FadeInOut];

    pub fn label(self) -> &'static str {
        match self {
            Transition::Crossfade => "Crossfade",
            Transition::FadeToBlack => "Fade to black",
            Transition::Cut => "Cut",
            Transition::FadeIn => "Fade in",
            Transition::FadeOut => "Fade out",
            Transition::FadeInOut => "Fade in and out",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Transition::Crossfade => "The next image dissolves in over the last, centred on the change",
            Transition::FadeToBlack => "Each image fades out to black, then the next fades in from black, centred on the change",
            Transition::Cut => "No fade: straight from one image to the next on the change",
            Transition::FadeIn => "Every image fades up from black as it starts, and cuts away at its end",
            Transition::FadeOut => "Every image cuts in, and fades out to black as it ends",
            Transition::FadeInOut => "Every image fades up from black as it starts, and out to black as it ends, each fade its own length",
        }
    }

    /// Whether the one fade (`fade`, centred on the change) counts.
    pub fn fades(self) -> bool {
        matches!(self, Transition::Crossfade | Transition::FadeToBlack)
    }

    /// Whether each image fades up from black as it starts.
    pub fn fades_in(self) -> bool {
        matches!(self, Transition::FadeIn | Transition::FadeInOut)
    }

    /// Whether each image fades out to black as it ends.
    pub fn fades_out(self) -> bool {
        matches!(self, Transition::FadeOut | Transition::FadeInOut)
    }
}

/// Kept so older documents open; the plan now always follows the score as it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    /// Count the images the current track holds.
    ImagesToTrack,
    /// Size the track to hold a given number of images.
    TrackToImages,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slideshow {
    pub fit: Fit,
    /// Images to fit the track to (TrackToImages).
    pub images: u32,
    /// Seconds on screen per image, fades included: exactly, if `exact`; otherwise
    /// the nearest whole bars (MIN_HOLD..=MAX_HOLD).
    pub hold: f32,
    /// Every image exactly `hold` seconds, on an even count, whether or not that
    /// falls on a bar line.
    pub exact: bool,
    /// Every image exactly `image_bars` bars, whatever the tempo (which then sets how
    /// long that is on screen; `hold` is the time aimed for). Before `exact`.
    pub by_bars: bool,
    /// Bars per image when `by_bars`: usually even, as chords and phrases come in twos.
    pub image_bars: u32,
    /// Every image a whole phrase of the score: its chord length times one, two, four
    /// or eight (or a half or quarter of a long chord), the intro whole chords too, so
    /// every change lands on a chord change or halfway through one. Before `by_bars`.
    /// New galleries start with it; older documents, without the field, keep their own.
    #[serde(default)]
    pub by_chords: bool,
    /// How each image gives way to the next.
    pub transition: Transition,
    /// Crossfade between images, in seconds.
    pub fade: f32,
    /// Fading each image up from black (FadeIn, FadeInOut), in seconds.
    pub fade_in: f32,
    /// Fading each image out to black (FadeOut, FadeInOut), in seconds.
    pub fade_out: f32,
    /// Fading out only: the first image's fade up from black, in seconds (0: none).
    pub first_fade_in: f32,
    /// The last image's fade out drawn out to the end of the soundtrack, its reverb
    /// tail included, so the picture reaches black as the sound dies away.
    pub stretch_final_fade: bool,
    /// Bars of music before the first image.
    pub intro_bars: u32,
    /// Bars of music after the last image; at least one, the final ring-out.
    pub outro_bars: u32,
    /// Mark the image changes on the timeline.
    pub show_on_timeline: bool,
    /// Change images where the music marks a moment, letting each image's
    /// time vary around `hold`, rather than on an even count.
    pub follow_music: bool,
    /// The folder of photos to export a video editor's timeline with.
    pub folder: Option<std::path::PathBuf>,
    pub order: crate::resolve::PhotoOrder,
    /// The photos' file names in the order set by hand (`PhotoOrder::Custom`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arranged: Vec<String>,
    /// Frames a second of the exported timeline and markers.
    pub fps: u32,
}

impl Default for Slideshow {
    fn default() -> Self {
        Slideshow {
            fit: Fit::ImagesToTrack,
            images: 30,
            hold: 12.0,
            exact: false,
            by_bars: false,
            image_bars: 4,
            by_chords: true,
            transition: Transition::Crossfade,
            fade: 6.0,
            fade_in: 1.0,
            fade_out: 1.0,
            first_fade_in: 0.0,
            stretch_final_fade: false,
            intro_bars: 2,
            outro_bars: 2,
            show_on_timeline: true,
            follow_music: true,
            folder: None,
            order: crate::resolve::PhotoOrder::Name,
            arranged: Vec::new(),
            fps: 25,
        }
    }
}

impl Slideshow {
    /// The photos to show, in order: the folder's, by name, date or as set by hand (none
    /// without a folder).
    pub fn photos(&self) -> Result<Vec<std::path::PathBuf>, String> {
        let Some(folder) = &self.folder else { return Ok(Vec::new()) };
        let found = crate::resolve::photos(folder, self.order)?;
        Ok(if self.order == crate::resolve::PhotoOrder::Custom { crate::resolve::arrange(found, &self.arranged) } else { found })
    }

    /// Whether the images change only on bar lines counted from the start (not exact
    /// seconds or the music's accents).
    pub fn even_bars(&self) -> bool {
        self.chords() || self.by_bars
    }

    /// Whether the images keep to the chords: `by_chords`, unless exact seconds or bars
    /// per image were asked for.
    pub fn chords(&self) -> bool {
        self.by_chords && !self.exact && !self.by_bars
    }

    /// Bars of intro: with the chords, rounded up to whole chords (or whole images, when
    /// an image is shorter than a chord) so the first image starts on a chord change.
    pub fn intro_on(&self, grid: &Grid) -> u32 {
        if !self.chords() {
            return self.intro_bars;
        }
        let step = phrase_bars(self, grid).min(grid.chord.max(1));
        self.intro_bars.div_ceil(step) * step
    }

    /// The fade centred on each change that editors overlap the clips for (a crossfade
    /// or a dip to black), in seconds; none for the other transitions.
    pub fn dissolve(&self) -> f64 {
        if self.transition.fades() { self.fade as f64 } else { 0.0 }
    }

    /// The fade up from black at each image's start and out to black at its end, in
    /// seconds (0 where the transition has none).
    pub fn edge_fades(&self) -> (f64, f64) {
        let t = self.transition;
        (if t.fades_in() { self.fade_in.max(0.0) as f64 } else { 0.0 }, if t.fades_out() { self.fade_out.max(0.0) as f64 } else { 0.0 })
    }
}

/// The beat grid of a piece.
#[derive(Clone, Copy, Debug)]
pub struct Grid {
    /// Seconds per beat (the meter's lower note value).
    pub beat: f64,
    pub beats_per_bar: u32,
    pub bpm: f64,
    pub meter: (u32, u32),
    /// Bars a chord (1 when not known): the score's harmonic rhythm.
    pub chord: u32,
}

impl Grid {
    pub fn new(bpm: f64, meter: (u32, u32)) -> Self {
        Grid { beat: 60.0 / bpm * 4.0 / meter.1 as f64, beats_per_bar: meter.0, bpm, meter, chord: 1 }
    }

    /// A project's grid, its chords' length included.
    pub fn of(p: &Project) -> Self {
        Grid { chord: dd_seq::compose::chord_bars_at(p, p.bpm), ..Grid::new(p.bpm, p.meter) }
    }

    /// The grid at another tempo: a piece written again at a new tempo keeps its chords'
    /// bars (D38), so they're as long in bars as now.
    pub fn at(p: &Project, bpm: f64) -> Self {
        Grid { chord: dd_seq::compose::chord_bars_at(p, p.bpm), ..Grid::new(bpm, p.meter) }
    }

    pub fn bar(&self) -> f64 {
        self.beat * self.beats_per_bar as f64
    }

    /// "bar.beat" (both from 1) of a time on the grid.
    pub fn position(&self, secs: f64) -> String {
        let beats = (secs / self.beat).round() as u32;
        format!("{}.{}", beats / self.beats_per_bar + 1, beats % self.beats_per_bar + 1)
    }
}

/// The image lengths that keep to the chords of `chord` bars: the chord, two, four or
/// eight of them, or a half or quarter of one (whole bars, at most MAX_IMAGE_BARS).
pub fn phrase_choices(chord: u32) -> Vec<u32> {
    let cb = chord.max(1);
    let mut out: Vec<u32> = [4u32, 2].into_iter().filter(|d| cb.is_multiple_of(*d)).map(|d| cb / d).collect();
    out.extend([1u32, 2, 4, 8, 16].into_iter().map(|m| cb * m).filter(|n| *n <= 32.min(MAX_IMAGE_BARS)));
    out.dedup();
    out
}

/// Bars per image with the chords: `image_bars` if it keeps to them, else the choice
/// nearest the time aimed for (the longer phrase on a tie).
pub fn phrase_bars(s: &Slideshow, grid: &Grid) -> u32 {
    let choices = phrase_choices(grid.chord);
    if choices.contains(&s.image_bars) {
        return s.image_bars;
    }
    let aim = s.hold as f64;
    let off = |n: u32| (n as f64 * grid.bar() - aim).abs();
    choices.into_iter().min_by(|a, b| off(*a).total_cmp(&off(*b)).then(b.cmp(a))).unwrap_or(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Bar,
    HalfBar,
    Beat,
    /// Exact seconds, off the grid.
    Second,
}

/// How long each image stays up, in whole units of the grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    pub count: u32,
    pub unit: Unit,
    pub secs: f64,
}

impl Slot {
    /// The time on screen the settings ask for: exactly `hold` seconds, or the
    /// nearest whole bars.
    pub fn of(s: &Slideshow, grid: &Grid) -> Slot {
        if s.chords() {
            let count = phrase_bars(s, grid);
            Slot { count, unit: Unit::Bar, secs: count as f64 * grid.bar() }
        } else if s.by_bars {
            let count = s.image_bars.clamp(1, MAX_IMAGE_BARS);
            Slot { count, unit: Unit::Bar, secs: count as f64 * grid.bar() }
        } else if s.exact {
            let secs = (s.hold as f64).clamp(EXACT_MIN, EXACT_MAX);
            Slot { count: 1, unit: Unit::Second, secs }
        } else {
            Slot::choose(grid, s.hold as f64)
        }
    }

    /// The time on screen nearest `preferred`, between MIN_HOLD and MAX_HOLD,
    /// in whole bars if any number of bars fits: the music moves on bar lines
    /// (a change mid-bar usually lands on nothing), and chords often last two
    /// bars, so a tie goes to an even number of bars. Only when a bar is too
    /// long or too short does it fall back to half bars, then beats.
    pub fn choose(grid: &Grid, preferred: f64) -> Slot {
        let preferred = preferred.clamp(MIN_HOLD, MAX_HOLD);
        let best = |unit: Unit, len: f64| {
            let lo = (MIN_HOLD / len - 1e-9).ceil().max(1.0) as u32;
            let hi = (MAX_HOLD / len + 1e-9).floor() as u32;
            (lo..=hi).map(|count| Slot { count, unit, secs: count as f64 * len }).min_by(|a, b| {
                (a.secs - preferred)
                    .abs()
                    .total_cmp(&(b.secs - preferred).abs())
                    .then((a.count % 2).cmp(&(b.count % 2)))
                    .then(b.count.cmp(&a.count))
            })
        };
        let half = grid.beats_per_bar.is_multiple_of(2).then(|| best(Unit::HalfBar, grid.bar() / 2.0)).flatten();
        best(Unit::Bar, grid.bar())
            .or(half)
            // a beat is at most 4 s even at 30 bpm in x/2, so some number of beats always fits
            .or_else(|| best(Unit::Beat, grid.beat))
            .unwrap_or(Slot { count: 1, unit: Unit::Beat, secs: grid.beat })
    }
}

/// Where the images fall in a track.
#[derive(Clone, Debug)]
pub struct Plan {
    pub grid: Grid,
    /// Each image's time on screen on an even count; following the music,
    /// the time it varies around.
    pub slot: Slot,
    pub images: u32,
    /// When each image appears.
    pub starts: Vec<f64>,
    /// When the last image gives way to the outro.
    pub end: f64,
    /// Music before the first image.
    pub intro: f64,
    /// Music after the last image, including any time too short for another
    /// image. The reverb tail follows it.
    pub outro: f64,
    /// The music's length, in whole bars.
    pub total: f64,
    /// Silence for the reverb after the music; the audio is `total + tail` long.
    pub tail: f64,
}

impl Plan {
    /// Count the images a piece of `total` seconds holds.
    pub fn for_track(s: &Slideshow, grid: Grid, total: f64) -> Plan {
        let slot = Slot::of(s, &grid);
        let intro = s.intro_on(&grid) as f64 * grid.bar();
        let min_outro = s.outro_bars.max(1) as f64 * grid.bar();
        let room = (total - intro - min_outro).max(0.0);
        let images = (room / slot.secs + 1e-6).floor() as u32;
        let outro = total - intro - images as f64 * slot.secs;
        Plan::even(grid, slot, images, intro, outro, total)
    }

    fn even(grid: Grid, slot: Slot, images: u32, intro: f64, outro: f64, total: f64) -> Plan {
        let starts = (0..images).map(|i| intro + i as f64 * slot.secs).collect();
        let end = intro + images as f64 * slot.secs;
        Plan { grid, slot, images, starts, end, intro, outro, total, tail: REVERB_TAIL }
    }

    /// The track needed for the given number of images; `bars` gives its
    /// length in whole bars, final ring-out bar included.
    pub fn for_images(s: &Slideshow, grid: Grid) -> Plan {
        let slot = Slot::of(s, &grid);
        let images = s.images.max(1);
        let intro = s.intro_on(&grid) as f64 * grid.bar();
        let pictures = images as f64 * slot.secs;
        let picture_bars = (pictures / grid.bar() - 1e-6).ceil();
        // at least the generator's shortest piece: four bars and the ring-out
        let bars = (s.intro_on(&grid) as f64 + picture_bars + s.outro_bars.max(1) as f64).max(5.0);
        let total = bars * grid.bar();
        Plan::even(grid, slot, images, intro, total - intro - pictures, total)
    }

    /// Move the image changes onto the music's accents, on the same grid as
    /// the even count: each image stays between two thirds of `hold` and half
    /// as long again (within MIN_HOLD and MAX_HOLD), kept near it by a cost that grows with the square of how far off it
    /// is, and a change is drawn to a strong moment and away from one where
    /// nothing happens. Counting the images for a track, their number
    /// follows; fitting a track to a number of images, it stays, and the
    /// last ends no later than the even count's last. The best placement
    /// over the whole track is found in one pass (dynamic programming).
    /// Unchanged if the accents are of a different track.
    pub fn follow(self, s: &Slideshow, accents: &Accents) -> Plan {
        if s.exact || s.even_bars() || self.images == 0 || (accents.bpm - self.grid.bpm).abs() > 1e-6 || accents.meter != self.grid.meter {
            return self;
        }
        let unit = self.slot.secs / self.slot.count as f64;
        let beats = (unit / self.grid.beat).round().max(1.0) as usize;
        let at = |secs: f64| (secs / unit).round() as usize;
        let (intro, last) = (at(self.intro), at(self.total - s.outro_bars.max(1) as f64 * self.grid.bar()));
        if last <= intro {
            return self;
        }
        let prefer = (s.hold as f64).clamp(MIN_HOLD, MAX_HOLD);
        // from two thirds to half as long again as asked, so the pace holds
        let lo = (prefer * 2.0 / 3.0).max(MIN_HOLD).min(self.slot.secs);
        let hi = (prefer * 1.5).min(MAX_HOLD).max(self.slot.secs);
        let gaps: Vec<usize> = (1..=last - intro).filter(|g| (lo - 1e-6..=hi + 1e-6).contains(&(*g as f64 * unit))).collect();
        let cost = |g: usize| 6.0 * ((g as f64 * unit - prefer) / prefer).powi(2);
        // only how much a moment stands out from the rest counts, so more
        // changes earn nothing in themselves
        let mark = |p: usize| accents.at(p * beats) - accents.typical;
        // a longer outro than asked, in images' worth
        let spare = |p: usize| 2.0 * (last - p) as f64 * unit / prefer;
        let span = last - intro;
        let fixed = (self.slot.count as usize * self.images as usize).min(span);
        let Some(&shortest) = gaps.first() else {
            return self;
        };
        // best[k][p]: the best score of k images from the intro to a change at p
        let rows = if s.fit == Fit::TrackToImages { self.images as usize } else { span / shortest };
        let mut best = vec![vec![f64::NEG_INFINITY; span + 1]; rows + 1];
        let mut from = vec![vec![0usize; span + 1]; rows + 1];
        best[0][0] = mark(intro);
        for k in 1..=rows {
            for p in 1..=span {
                for &g in gaps.iter().filter(|g| **g <= p) {
                    let v = best[k - 1][p - g] - cost(g);
                    if v > best[k][p] {
                        best[k][p] = v;
                        from[k][p] = p - g;
                    }
                }
                if best[k][p] > f64::NEG_INFINITY {
                    best[k][p] += mark(intro + p);
                }
            }
        }
        let ends = |k: usize| (1..=span).map(move |p| (k, p));
        let candidates: Vec<(usize, usize)> = match s.fit {
            Fit::TrackToImages => ends(rows).filter(|(_, p)| *p <= fixed).collect(),
            Fit::ImagesToTrack => (1..=rows).flat_map(ends).collect(),
        };
        let Some((mut k, mut p)) = candidates
            .into_iter()
            .filter(|(k, p)| best[*k][*p] > f64::NEG_INFINITY)
            .max_by(|a, b| (best[a.0][a.1] - spare(intro + a.1)).total_cmp(&(best[b.0][b.1] - spare(intro + b.1))))
        else {
            return self;
        };
        let end = (intro + p) as f64 * unit;
        let mut starts = Vec::new();
        while k > 0 {
            p = from[k][p];
            starts.push((intro + p) as f64 * unit);
            k -= 1;
        }
        starts.reverse();
        Plan { images: starts.len() as u32, starts, end, outro: self.total - end, ..self }
    }

    /// No more than `images` images: the time the rest would have had goes to the outro.
    pub fn limit(mut self, images: u32) -> Plan {
        if self.images > images {
            self.end = self.starts.get(images as usize).copied().unwrap_or(self.end);
            if images == 0 {
                self.end = self.intro;
            }
            self.starts.truncate(images as usize);
            self.images = images;
            self.outro = self.total - self.end;
        }
        self
    }

    /// The length of the audio, reverb tail included.
    pub fn audio(&self) -> f64 {
        self.total + self.tail
    }

    /// The music's length in whole bars.
    pub fn bars(&self) -> u32 {
        (self.total / self.grid.bar()).round() as u32
    }

    /// When each image appears.
    pub fn starts(&self) -> impl Iterator<Item = f64> + '_ {
        self.starts.iter().copied()
    }

    /// Each image's start and end.
    pub fn spans(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.starts.iter().copied().zip(self.starts.iter().skip(1).copied().chain([self.end]))
    }

    /// The shortest and longest time an image is on screen.
    pub fn holds(&self) -> (f64, f64) {
        self.spans().fold((f64::INFINITY, 0.0f64), |(lo, hi), (a, b)| (lo.min(b - a), hi.max(b - a)))
    }

    /// One row per image, for a video editor. `start` is when the image
    /// starts to fade in, `end` when the next one does. Editors that overlap
    /// clips to crossfade them (most consumer editors) shorten the timeline by
    /// the fade at every change, so each image there must be `clip` long:
    /// its time on screen plus the fade.
    pub fn cue_csv(&self, fade: f64) -> String {
        let mut out = String::from("image,start,end,seconds,clip,bar.beat,fade\n");
        for (i, (t, end)) in self.spans().enumerate() {
            out.push_str(&format!(
                "{},{},{},{:.2},{:.2},{},{:.2}\n",
                i + 1,
                timecode(t),
                timecode(end),
                end - t,
                end - t + fade,
                self.grid.position(t),
                fade
            ));
        }
        out
    }
}

/// Sums for planning before changing anything: what a score of `bars` holds at
/// these settings, and how long a score `photos` images need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calc {
    /// Seconds each image is on screen (exact, or the nearest whole bars).
    pub each: f64,
    pub bar: f64,
    pub intro: f64,
    /// The shortest outro asked for.
    pub outro: f64,
    /// Images the score as it stands holds, and the time left over (to the outro).
    pub holds: u32,
    pub left_over: f64,
    /// The music the photos need, the whole bars that holds it, and what's spare.
    pub needed: f64,
    pub needed_bars: u32,
    pub spare: f64,
}

impl Calc {
    pub fn new(s: &Slideshow, grid: Grid, score_secs: f64, photos: u32) -> Calc {
        let slot = Slot::of(s, &grid);
        let bar = grid.bar();
        let intro = s.intro_on(&grid) as f64 * bar;
        let outro = s.outro_bars.max(1) as f64 * bar;
        let room = (score_secs - intro - outro).max(0.0);
        let holds = (room / slot.secs + 1e-6).floor() as u32;
        let left_over = room - holds as f64 * slot.secs;
        let needed = intro + photos as f64 * slot.secs + outro;
        let needed_bars = ((needed / bar) - 1e-6).ceil().max(1.0) as u32;
        let spare = (needed_bars as f64 * bar - needed).max(0.0);
        Calc { each: slot.secs, bar, intro, outro, holds, left_over, needed, needed_bars, spare }
    }
}

/// How strongly the music marks each beat of a track: notes starting on it
/// (the bass and drums most), a chord change, parts coming in or dropping
/// out and the music thickening or thinning across a bar line, the start of
/// a score's section and of the four- and eight-bar phrases that count from
/// it; and, against all of these, a beat where nothing starts at all.
#[derive(Clone, Debug, Default)]
pub struct Accents {
    pub bpm: f64,
    pub meter: (u32, u32),
    pub beats: Vec<f64>,
    /// The median accent on the bar lines.
    pub typical: f64,
}

impl Accents {
    pub fn of(p: &Project) -> Accents {
        let beat = p.beat_ticks();
        let per_bar = p.meter.0.max(1) as usize;
        let bar = p.bar_ticks();
        let total = p.end_tick();
        let count = (total / beat) as usize + 1;
        let bars = count / per_bar + 1;
        let tracks = p.tracks.len().max(1);
        // the bass and drums mark the beat most; long, quiet pads least
        let weight = |t: usize| {
            let tr = &p.tracks[t];
            if tr.source.is_drums() {
                0.7
            } else if tr.name.to_lowercase().contains("bass") {
                1.0
            } else {
                0.5
            }
        };
        // a little either side, for loosely played timing
        let (early, late) = (crate::slideshow::PPQ / 8, crate::slideshow::PPQ / 4);
        let mut onset = vec![0.0f64; count];
        // the tracks starting notes in each bar
        let mut parts = vec![vec![0u32; tracks]; bars];
        for (ti, t) in p.tracks.iter().enumerate() {
            if !p.audible(ti) {
                continue;
            }
            for n in &t.notes {
                let j = ((n.start + early) / beat) as usize;
                if j < count && n.start + early - (j as u32 * beat) <= early + late {
                    onset[j] += n.vel as f64 / 127.0 * weight(ti);
                }
                if let Some(b) = parts.get_mut((n.start / bar) as usize) {
                    b[ti] += 1;
                }
            }
        }
        let sections: Vec<u32> = p.section_bars();
        let sum = |a: usize, z: usize, i: usize| (a..z.min(bars)).map(|b| parts[b][i]).sum::<u32>();
        let beats: Vec<f64> = (0..count)
            .map(|j| {
                let mut a = if onset[j] > 0.0 { 1.0 - (-onset[j] / 1.5).exp() } else { -2.0 };
                if j % per_bar == 0 {
                    let b = j / per_bar;
                    if b > 0 {
                        let (lo, hi) = (b.saturating_sub(2), b + 2);
                        let moved = (0..tracks).filter(|&i| (sum(lo, b, i) == 0) != (sum(b, hi, i) == 0)).count();
                        a += 0.7 * moved.min(3) as f64;
                        let (x, y) = ((0..tracks).map(|i| sum(lo, b, i)).sum::<u32>(), (0..tracks).map(|i| sum(b, hi, i)).sum::<u32>());
                        a += 0.4 * ((y as f64 + 2.0) / (x as f64 + 2.0)).ln().abs();
                    }
                    if sections.contains(&(b as u32)) {
                        a += 2.0;
                    }
                    let from = sections.iter().copied().filter(|s| *s as usize <= b).max().unwrap_or(0) as usize;
                    let into = b - from;
                    a += if into.is_multiple_of(8) {
                        0.5
                    } else if into.is_multiple_of(4) {
                        0.3
                    } else if into.is_multiple_of(2) {
                        0.1
                    } else {
                        0.0
                    };
                }
                a
            })
            .collect();
        let mut downs: Vec<f64> = beats.iter().step_by(per_bar).copied().collect();
        downs.sort_by(|a, b| a.total_cmp(b));
        let typical = downs.get(downs.len() / 2).copied().unwrap_or(0.0);
        Accents { bpm: p.bpm, meter: p.meter, beats, typical }
    }

    /// The accent on a beat, counted from the start; past the end, nothing.
    pub fn at(&self, beat: usize) -> f64 {
        self.beats.get(beat).copied().unwrap_or(-2.0)
    }
}

/// m:ss.cc
pub fn timecode(secs: f64) -> String {
    let cs = (secs.max(0.0) * 100.0).round() as u64;
    format!("{}:{:02}.{:02}", cs / 6000, cs / 100 % 60, cs % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_fall_between_the_shortest_and_longest() {
        for bpm in [30.0, 40.0, 48.0, 60.0, 72.0, 84.0, 120.0, 200.0] {
            for meter in [(4, 4), (3, 4), (6, 8), (6, 4)] {
                let grid = Grid::new(bpm, meter);
                for hold in [4.0, 6.0, 8.0, 12.0, 16.0] {
                    let s = Slot::choose(&grid, hold);
                    assert!((MIN_HOLD - 1e-6..=MAX_HOLD + 1e-6).contains(&s.secs), "{bpm} {meter:?} {s:?}");
                }
            }
        }
    }

    #[test]
    fn prefers_whole_bars() {
        // 4/4 at 60: a bar is 4 s
        let grid = Grid::new(60.0, (4, 4));
        assert_eq!(Slot::choose(&grid, 8.0), Slot { count: 2, unit: Unit::Bar, secs: 8.0 });
        assert_eq!(Slot::choose(&grid, 4.5), Slot { count: 1, unit: Unit::Bar, secs: 4.0 });
        // 6 s is as far from one bar as from two: two, an even number
        assert_eq!(Slot::choose(&grid, 6.0), Slot { count: 2, unit: Unit::Bar, secs: 8.0 });
        // 4/4 at 120: a bar is 2 s; 6 s is three bars
        assert_eq!(Slot::choose(&Grid::new(120.0, (4, 4)), 6.0).count, 3);
        // 3/4 at 66 has no half bars: a bar is 2.7 s, two are 5.5 s
        assert_eq!(Slot::choose(&Grid::new(66.0, (3, 4)), 6.0).unit, Unit::Bar);
        // 6/4 at 40: a bar is 9 s, which fits, so whole bars win over 4.5 s half bars
        let s = Slot::choose(&Grid::new(40.0, (6, 4)), 6.0);
        assert_eq!((s.count, s.unit), (1, Unit::Bar));
        // 6/4 at 20 (below the tempo range): a bar is 18 s, too long; a half bar is 9 s
        let s = Slot::choose(&Grid::new(20.0, (6, 4)), 6.0);
        assert_eq!((s.count, s.unit), (1, Unit::HalfBar));
        // 5/4 at 15: a bar is 20 s and has no half bar, so it changes on beats
        // of 4 s; 6 s is as near one as two, so two
        let s = Slot::choose(&Grid::new(15.0, (5, 4)), 6.0);
        assert_eq!((s.count, s.unit), (2, Unit::Beat));
        // the default 12 s at 80 bpm in 4/4 (3 s bars) is four bars
        assert_eq!(Slot::choose(&Grid::new(80.0, (4, 4)), 12.0).count, 4);
    }

    #[test]
    fn images_round_trip_through_a_track() {
        for (bpm, meter) in [(60.0, (4, 4)), (66.0, (3, 4)), (40.0, (6, 4)), (84.0, (4, 4))] {
            let grid = Grid::new(bpm, meter);
            for n in [1, 7, 30, 100] {
                let s = Slideshow { images: n, by_chords: false, ..Slideshow::default() };
                let need = Plan::for_images(&s, grid);
                assert_eq!(need.images, n);
                assert!(need.outro >= grid.bar() * 2.0 - 1e-6);
                // the track it asks for holds at least that many images
                let got = Plan::for_track(&s, grid, need.total);
                assert!(got.images >= n, "{bpm} {meter:?} {n}: {}", got.images);
                assert_eq!(need.bars() as f64 * grid.bar(), need.total);
            }
        }
    }

    /// With the chords, an image is a chord or a power of two of them (or a half or
    /// quarter of one), never across them: no three-bar images over four-bar chords.
    #[test]
    fn images_keep_to_the_chords() {
        assert_eq!(phrase_choices(4), vec![1, 2, 4, 8, 16, 32]);
        assert_eq!(phrase_choices(1), vec![1, 2, 4, 8, 16]);
        assert_eq!(phrase_choices(2), vec![1, 2, 4, 8, 16, 32]);
        assert_eq!(phrase_choices(3), vec![3, 6, 12, 24]);
        assert_eq!(phrase_choices(6), vec![3, 6, 12, 24]);
        for (bpm, chord, image_bars, hold, intro) in
            [(56.0, 2u32, 4u32, 12.9f32, 2u32), (60.0, 4, 3, 14.0, 2), (124.0, 4, 4, 12.0, 3), (72.0, 3, 4, 10.0, 1), (90.0, 1, 5, 9.0, 1)]
        {
            let grid = Grid { chord, ..Grid::new(bpm, (4, 4)) };
            let s = Slideshow { hold, image_bars, intro_bars: intro, ..Slideshow::default() };
            assert!(s.chords());
            let n = phrase_bars(&s, &grid);
            assert!(phrase_choices(chord).contains(&n), "{n} bars over {chord}-bar chords");
            let p = Plan::for_track(&s, grid, 64.0 * grid.bar());
            assert!(p.images > 0);
            let step = n.min(chord);
            for t in p.starts() {
                let bar = (t / grid.bar()).round() as u32;
                assert!((t / grid.bar() - bar as f64).abs() < 1e-9, "on a bar line");
                assert_eq!(bar % step, 0, "{bpm} bpm: an image at bar {bar}, chords every {chord}, images {n}");
            }
            // the calculator counts the same intro
            let c = Calc::new(&s, grid, 64.0 * grid.bar(), p.images);
            assert_eq!(c.intro, p.intro);
            assert_eq!(c.holds, p.images);
        }
        // a whole number of bars asked for that keeps to the chords stays
        let grid = Grid { chord: 4, ..Grid::new(60.0, (4, 4)) };
        assert_eq!(phrase_bars(&Slideshow { image_bars: 8, hold: 4.0, ..Slideshow::default() }, &grid), 8);
        // and one that doesn't goes to the nearest the time aimed for
        assert_eq!(phrase_bars(&Slideshow { image_bars: 3, hold: 12.0, ..Slideshow::default() }, &grid), 4);
        // exact seconds and bars per image, if asked for, come first
        assert!(!Slideshow { exact: true, ..Slideshow::default() }.chords());
        assert!(!Slideshow { by_bars: true, ..Slideshow::default() }.chords());
    }

    #[test]
    fn track_plan_adds_up() {
        let grid = Grid::new(60.0, (4, 4));
        let s = Slideshow { by_chords: false, ..Slideshow::default() };
        let p = Plan::for_track(&s, grid, 180.0);
        assert!(p.images > 0);
        assert!((p.intro + p.images as f64 * p.slot.secs + p.outro - 180.0).abs() < 1e-6);
        assert!(p.outro >= 2.0 * grid.bar() - 1e-6 && p.outro < 2.0 * grid.bar() + p.slot.secs);
        let csv = p.cue_csv(1.0);
        assert_eq!(csv.lines().count(), p.images as usize + 1);
        assert_eq!(csv.lines().nth(1).unwrap(), "1,0:08.00,0:20.00,12.00,13.00,3.1,1.00");
        assert_eq!(p.audio(), 180.0 + REVERB_TAIL);
    }

    #[test]
    fn exact_seconds_and_the_calculator() {
        // 66 bpm in 4/4: bars of 3.636 s; every image exactly 8 s, two bars either end
        let grid = Grid::new(66.0, (4, 4));
        let s = Slideshow { hold: 8.0, by_chords: false, exact: true, intro_bars: 2, outro_bars: 2, ..Slideshow::default() };
        let total = 100.0 * grid.bar();
        let p = Plan::for_track(&s, grid, total);
        assert!(p.spans().all(|(a, b)| ((b - a) - 8.0).abs() < 1e-9), "every image exactly 8 s");
        assert_eq!(p.images, 43);
        // following the music leaves exact timing alone
        let c = Calc::new(&s, grid, total, 25);
        assert_eq!(c.holds, 43);
        assert!((c.left_over - (total - 4.0 * grid.bar() - 43.0 * 8.0)).abs() < 1e-9);
        // 25 photos: 2 bars + 200 s + 2 bars = 214.5 s, which 59 bars hold
        assert!((c.needed - (4.0 * grid.bar() + 200.0)).abs() < 1e-9);
        assert_eq!(c.needed_bars, 59);
        assert!(c.spare >= 0.0 && c.spare < grid.bar());
        // a plan for 25 photos in that 100-bar score stops at the 25th; the rest is outro
        let l = p.clone().limit(25);
        assert_eq!(l.images, 25);
        assert!((l.end - (2.0 * grid.bar() + 200.0)).abs() < 1e-9);
        assert!((l.outro - (total - l.end)).abs() < 1e-9);
    }

    /// Images in whole bars: exactly that many bars each whatever the tempo, the
    /// calculator's bars exact, and the tempo that gives a time on screen.
    #[test]
    fn images_in_whole_bars() {
        for (bpm, meter) in [(60.0, (4, 4)), (66.0, (3, 4)), (92.0, (15, 8)), (120.0, (4, 4))] {
            let grid = Grid::new(bpm, meter);
            let s = Slideshow { by_bars: true, image_bars: 4, intro_bars: 2, outro_bars: 2, ..Slideshow::default() };
            let p = Plan::for_track(&s, grid, 40.0 * grid.bar());
            assert_eq!(p.images, 9, "{bpm} {meter:?}");
            assert!(p.spans().all(|(a, b)| ((b - a) - 4.0 * grid.bar()).abs() < 1e-9));
            let c = Calc::new(&s, grid, 40.0 * grid.bar(), 12);
            assert_eq!(c.needed_bars, 2 + 12 * 4 + 2);
            // no tempo fits 12 images of 4 bars into 40 bars, or 9 into 38; 9 fit 40 as it is
            assert_eq!(tempo_to_fit(&s, meter, bpm, 40, 12), None);
            assert_eq!(tempo_to_fit(&s, meter, bpm, 38, 9), None);
            assert_eq!(tempo_to_fit(&s, meter, bpm, 40, 9), Some(bpm));
        }
        // 4 bars of 4/4 in 8 s: 120 bpm; of 6/8 in 8 s: 135 (in quavers)
        assert_eq!(tempo_for((4, 4), 4, 8.0), Some(120.0));
        assert_eq!(tempo_for((6, 8), 4, 8.0), Some(90.0));
        assert_eq!(tempo_for((4, 4), 2, 1.0), None);
    }

    /// In seconds, a tempo can make the score hold the photos without new sections.
    #[test]
    fn a_tempo_that_fits_the_photos() {
        let s = Slideshow { hold: 8.0, by_chords: false, exact: true, intro_bars: 2, outro_bars: 2, ..Slideshow::default() };
        let bpm = tempo_to_fit(&s, (4, 4), 66.0, 40, 20).expect("a tempo");
        let grid = Grid::new(bpm, (4, 4));
        assert_eq!(Calc::new(&s, grid, 40.0 * grid.bar(), 20).needed_bars, 40);
        // 2 + 2 bars and 160 s in 40 bars: 36 bars of about 4.4 s, near 54 bpm
        assert!((bpm - 54.0).abs() < 1.0, "{bpm}");
    }

    #[test]
    fn timecodes() {
        assert_eq!(timecode(0.0), "0:00.00");
        assert_eq!(timecode(65.5), "1:05.50");
        assert_eq!(Grid::new(60.0, (4, 4)).position(8.0), "3.1");
    }
}

#[cfg(test)]
mod music {
    use super::*;
    use dd_seq::demo::rain_on_neon;

    /// Following the music, the images change on the sections and keep their count.
    #[test]
    fn changes_follow_the_score() {
        let p = rain_on_neon();
        let grid = Grid::new(p.bpm, p.meter);
        let s = Slideshow {
            hold: 8.0,
            by_chords: false,
            intro_bars: 0,
            outro_bars: 2,
            fit: Fit::TrackToImages,
            images: 9,
            ..Slideshow::default()
        };
        let even = Plan::for_images(&s, grid);
        let plan = even.clone().follow(&s, &Accents::of(&p));
        assert_eq!(plan.images, 9);
        let bar = grid.bar();
        // the brass section (bar 4) and the lone lead (bar 12) each get a new image
        for b in [4.0, 12.0] {
            assert!(plan.starts().any(|t| (t - b * bar).abs() < 1e-6), "no change at bar {b}: {:?}", plan.starts);
        }
    }
}
