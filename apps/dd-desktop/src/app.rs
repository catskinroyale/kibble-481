//! The app: its state, keeping the engine in step with the project, the transport and
//! recording, files, and the layout. The panels are in their own modules.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::JoinHandle;

use crate::dialogs::{Kind, Want};
pub(crate) use dd_gallery::doc::EXTENSION;
use dd_gallery::doc::{Document, OLD_EXTENSION};
use dd_seq::compose::{Recipe, compose, keep_levels, keep_sounds};
use dd_seq::engine::{Cmd, MixSettings};
use dd_seq::fonts::{FontBank, make_instrument, soundfont_dir};
use dd_seq::project::{Block, PPQ, Project, Source};
use dd_seq::record::Take;
use dd_seq::timeline::{Msg, Timeline};
use eframe::egui;

use crate::audio::Audio;
use crate::live::{Live, MidiIn, qwerty_key};
use crate::roll::RollState;
use crate::theme::{APP_NAME, Palette, Tokens, WORDMARK};
use crate::widgets::{self, Icon};

const KEY_PALETTE: &str = "palette";
const KEY_RECENT: &str = "recent";
const KEY_SETTINGS: &str = "settings";
const UNDO_DEPTH: usize = 200;

/// The grid notes snap to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Snap {
    Bar,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    EighthTriplet,
    SixteenthTriplet,
    Off,
}

impl Snap {
    pub const ALL: [Snap; 8] =
        [Snap::Bar, Snap::Half, Snap::Quarter, Snap::Eighth, Snap::Sixteenth, Snap::EighthTriplet, Snap::SixteenthTriplet, Snap::Off];

    pub fn label(self) -> &'static str {
        match self {
            Snap::Bar => "Bar",
            Snap::Half => "1/2",
            Snap::Quarter => "1/4",
            Snap::Eighth => "1/8",
            Snap::Sixteenth => "1/16",
            Snap::EighthTriplet => "1/8 T",
            Snap::SixteenthTriplet => "1/16 T",
            Snap::Off => "Off",
        }
    }

    pub fn ticks(self, p: &Project) -> u32 {
        match self {
            Snap::Bar => p.bar_ticks(),
            Snap::Half => PPQ * 2,
            Snap::Quarter => PPQ,
            Snap::Eighth => PPQ / 2,
            Snap::Sixteenth => PPQ / 4,
            Snap::EighthTriplet => PPQ / 3,
            Snap::SixteenthTriplet => PPQ / 6,
            Snap::Off => PPQ / 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Tab {
    Sound,
    Effects,
    Score,
    Gallery,
}

/// Settings remembered between runs.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub snap: Snap,
    pub count_in: bool,
    pub metronome: bool,
    pub quantize_takes: bool,
    pub replace_takes: bool,
    pub qwerty: bool,
    pub show_mixer: bool,
    pub show_keys: bool,
    pub ghosts: bool,
    pub tab: Tab,
    pub midi_port: Option<String>,
    pub volume: f32,
    pub stems_with_gallery: bool,
    /// A new piece keeps the current tracks' sounds and mix.
    pub keep_sounds: bool,
    /// New notes (and a new piece) turn each track's density and complexity somewhere
    /// new (D36).
    pub vary_dials: bool,
    /// Ask GitHub for a newer release when the app opens.
    pub check_updates: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            snap: Snap::Sixteenth,
            count_in: true,
            metronome: false,
            quantize_takes: false,
            replace_takes: false,
            qwerty: true,
            show_mixer: false,
            show_keys: true,
            ghosts: true,
            tab: Tab::Sound,
            midi_port: None,
            volume: 0.9,
            stems_with_gallery: false,
            keep_sounds: false,
            vary_dials: true,
            check_updates: true,
        }
    }
}

/// The on-screen performance controls.
#[derive(Clone, Copy, Debug, Default)]
pub struct Perf {
    /// Ribbon / pitch bend, -1..1 (springs back to 0).
    pub bend: f32,
    pub wheel: f32,
    /// Touch (aftertouch), 0..1 (springs back to 0).
    pub touch: f32,
    pub sustain: bool,
}

struct Job {
    label: String,
    handle: Option<JoinHandle<Result<String, String>>>,
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
}

pub struct App {
    pub(crate) doc: Document,
    pub(crate) path: Option<PathBuf>,
    saved: Document,
    undo: Vec<Project>,
    redo: Vec<Project>,
    committed: Project,
    pub(crate) fonts: FontBank,
    pub(crate) font_errors: Vec<String>,
    pub(crate) audio: Option<Audio>,
    audio_error: Option<String>,
    pub(crate) live: Arc<Live>,
    pub(crate) midi: MidiIn,
    pub(crate) midi_ports: Vec<String>,
    /// What the engine was last given: its instruments and mix, and its timeline.
    synced: Option<Project>,
    synced_music: Option<Project>,
    synced_loop: Option<Option<(f64, f64)>>,
    pub(crate) inst_errors: Vec<Option<String>>,
    pub(crate) sel: usize,
    pub(crate) roll: RollState,
    pub(crate) settings: Settings,
    pub(crate) octave: i32,
    pub(crate) key_vel: u8,
    qwerty_down: Vec<(egui::Key, u8)>,
    pub(crate) perf: Perf,
    /// Where playback started, to go back to on stop.
    play_from: f64,
    rec_from: f64,
    job: Option<Job>,
    /// The file or folder dialog open, if any (see `dialogs`).
    pub(crate) asking: Option<crate::dialogs::Asking>,
    /// The calculator's proposed section lengths, while the user tries them.
    pub(crate) fit: Option<crate::panels::FitPlan>,
    pub(crate) status: String,
    pub(crate) about: crate::about::About,
    /// The pane reordering the gallery's photos, while it's open.
    pub(crate) reorder: Option<crate::reorder::Reorder>,
    update: Option<crate::update::UpdateStatus>,
    update_rx: Option<std::sync::mpsc::Receiver<crate::update::UpdateStatus>>,
    /// The track whose name is being edited (double-click a name).
    pub(crate) renaming: Option<usize>,
    palette: Palette,
    title: String,
    pub(crate) recent: Vec<PathBuf>,
    pub(crate) clipboard: Vec<dd_seq::project::Note>,
    /// The tempo, metre and length last seen, to notice when they change.
    shape: Shape,
    /// A preview to play once the engine has the track's new sound.
    pub(crate) audition: Option<(usize, dd_seq::engine::Preview)>,
    /// What the level meters show: each track's, then the master's left and right. Read
    /// from the engine once a frame and shared by the track list and the mixer, rising at
    /// once and falling smoothly.
    pub(crate) meters: Vec<f32>,
    meters_at: f64,
    snapshot: Option<crate::snapshot::Snapshot>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let storage = cc.storage;
        let storage = if crate::snapshot::Snapshot::from_env().is_some() { None } else { storage };
        let palette = storage
            .and_then(|s| eframe::get_value::<String>(s, KEY_PALETTE))
            .and_then(|k| Palette::from_key(&k))
            .or_else(|| std::env::var("KIBBLE_PALETTE").ok().and_then(|k| Palette::from_key(&k)))
            .unwrap_or(Palette::Amber);
        let settings: Settings = storage.and_then(|s| eframe::get_value(s, KEY_SETTINGS)).unwrap_or_default();
        let recent: Vec<PathBuf> = storage.and_then(|s| eframe::get_value(s, KEY_RECENT)).unwrap_or_default();
        crate::theme::install(&cc.egui_ctx, palette);

        let mut fonts = FontBank::default();
        let font_errors = match soundfont_dir() {
            Some(d) => fonts.load_dir(&d),
            None => vec!["The bundled SoundFonts weren't found: SoundFont tracks will be silent. Load one under Sound.".into()],
        };
        let (audio, audio_error) = match Audio::start() {
            Ok(a) => (Some(a), None),
            Err(e) => (None, Some(e)),
        };
        let live = Live::new(audio.as_ref().map(|a| a.sender()), audio.as_ref().map(|a| a.shared.clone()), cc.egui_ctx.clone());
        // a new piece every time it opens
        let seed = if std::env::var_os("KIBBLE_SCREENSHOT").is_some() { 7 } else { entropy() };
        let doc = Document::new(compose(&Recipe::random(seed), 66.0, (4, 4), 24));
        let mut app = App {
            saved: doc.clone(),
            committed: doc.project.clone(),
            doc,
            path: None,
            undo: Vec::new(),
            redo: Vec::new(),
            fonts,
            font_errors,
            audio,
            audio_error,
            live,
            midi: MidiIn::new(),
            midi_ports: MidiIn::ports(),
            synced: None,
            synced_music: None,
            synced_loop: None,
            inst_errors: Vec::new(),
            sel: 0,
            roll: RollState::default(),
            settings,
            octave: 4,
            key_vel: 100,
            qwerty_down: Vec::new(),
            perf: Perf::default(),
            play_from: 0.0,
            rec_from: 0.0,
            job: None,
            asking: None,
            fit: None,
            status: String::new(),
            about: Default::default(),
            reorder: None,
            update: None,
            update_rx: None,
            renaming: None,
            palette,
            title: String::new(),
            recent,
            clipboard: Vec::new(),
            shape: Shape::default(),
            audition: None,
            meters: Vec::new(),
            meters_at: 0.0,
            snapshot: crate::snapshot::Snapshot::from_env(),
        };
        if app.settings.check_updates && app.snapshot.is_none() {
            app.start_update_check(&cc.egui_ctx);
        }
        if let Some(port) = app.settings.midi_port.clone()
            && app.midi_ports.contains(&port)
        {
            app.midi.connect(Some(&port), app.live.clone());
        } else if app.settings.midi_port.is_none()
            && let Some(first) = app.midi_ports.first().cloned()
        {
            // a keyboard plugged in plays straight away
            app.midi.connect(Some(&first), app.live.clone());
        }
        app.roll.fit_to(&app.doc.project);
        app.shape = Shape::of(&app.doc.project);
        if app.snapshot.as_ref().is_some_and(|s| s.wants("reorder")) {
            if let Some(dir) = std::env::var_os("KIBBLE_SCREENSHOT_PHOTOS") {
                app.doc.gallery.folder = Some(dir.into());
            }
            app.open_reorder(&cc.egui_ctx);
        }
        if let Some(snap) = &app.snapshot {
            // screenshots start from defaults and never touch the user's settings
            app.settings = Settings::default();
            let pick = |name: &str| app.doc.project.tracks.iter().position(|t| t.name == name);
            if snap.wants("mixer") {
                app.settings.show_mixer = true;
            }
            for (w, tab) in [("effects", Tab::Effects), ("score", Tab::Score), ("gallery", Tab::Gallery)] {
                if snap.wants(w) {
                    app.settings.tab = tab;
                }
            }
            if snap.wants("sax") {
                app.sel = pick("Sax").unwrap_or(0);
            }
            if snap.wants("bells") {
                app.sel = pick("Bells").unwrap_or(0);
            }
            for (w, name) in [("bass", "Bass"), ("city", "City"), ("drone", "Drone")] {
                if snap.wants(w) {
                    app.sel = pick(name).unwrap_or(0);
                }
            }
            if snap.wants("layer2") {
                cc.egui_ctx.data_mut(|d| d.insert_temp(egui::Id::unique("dd-edit-layer"), 1usize));
            }
            app.about.open = snap.wants("about");
            if snap.wants("inout") {
                let g = &mut app.doc.gallery;
                (g.transition, g.outro_bars, g.fade_in, g.fade_out, g.first_fade_in) =
                    (dd_gallery::slideshow::Transition::FadeInOut, 0, 8.0, 8.0, 4.0);
            }
            if let Some(dir) = std::env::var_os("KIBBLE_SCREENSHOT_PHOTOS") {
                app.doc.gallery.folder = Some(dir.into());
            }

            // these change the piece after it opens, as a user would
            if snap.wants("edited") {
                app.doc.project.tracks[0].notes.pop();
                app.shape.untouched = false;
            }
            if snap.wants("added") {
                let mut b = app.doc.project.arrangement();
                b.push(Block::new(dd_seq::project::Part::Neon, 4));
                app.doc.project.set_arrangement(&b);
            }
            if snap.wants("long") {
                app.doc.project.length_bars = 48;
            }
            if snap.wants("fast") {
                app.doc.project.bpm = 124.0;
            }
            for (w, style) in [
                ("pursuit", dd_seq::compose::Style::Pursuit),
                ("blackout", dd_seq::compose::Style::Blackout),
                ("momento", dd_seq::compose::Style::Momento),
                ("noir", dd_seq::compose::Style::Noir),
                ("overcity", dd_seq::compose::Style::Overcity),
                ("derelict", dd_seq::compose::Style::Derelict),
                ("homeward", dd_seq::compose::Style::Homeward),
                ("return", dd_seq::compose::Style::Return),
                ("chamber", dd_seq::compose::Style::Chamber),
                ("orbit", dd_seq::compose::Style::Orbit),
                ("nebula", dd_seq::compose::Style::Nebula),
                ("vigil", dd_seq::compose::Style::Vigil),
                ("cinder", dd_seq::compose::Style::Cinder),
                ("filmstrip", dd_seq::compose::Style::Filmstrip),
                ("strata", dd_seq::compose::Style::Strata),
                ("canopy", dd_seq::compose::Style::Canopy),
                ("tarn", dd_seq::compose::Style::Tarn),
            ] {
                if snap.wants(w)
                    && let Some(r) = &mut app.doc.project.recipe
                {
                    r.style = style;
                    style.settle(r);
                    app.doc.project.bpm = style.tempo();
                    app.doc.project.meter = style.meter_from(app.doc.project.meter);
                }
            }
            if snap.wants("volume") {
                // the volume lane, with the selected track rising over six bars and falling at the end
                app.roll.lane = crate::roll::Lane::Volume;
                let levels = &mut app.doc.project.tracks[app.sel].levels;
                for (bar, value) in [(4, 30), (10, 110), (20, 110), (23, 20)] {
                    dd_seq::project::set_level(levels, bar, value);
                }
            }
        }
        app.send(Cmd::Metronome(app.settings.metronome));
        app.send(Cmd::Volume(app.settings.volume));
        app.status = match (&app.audio_error, app.font_errors.first()) {
            (Some(e), _) => format!("No audio output: {e}"),
            (None, Some(e)) => e.clone(),
            (None, None) => format!(
                "{} SoundFonts loaded. Press Space to play \u{201c}{}\u{201d}; 🎲 for another piece.",
                app.fonts.fonts.len(),
                app.doc.project.title
            ),
        };
        app
    }

    pub(crate) fn project(&self) -> &Project {
        &self.doc.project
    }

    pub(crate) fn send(&self, cmd: Cmd) {
        if let Some(a) = &self.audio {
            a.send(cmd);
        }
    }

    pub(crate) fn position(&self) -> f64 {
        self.audio.as_ref().map_or(0.0, |a| a.shared.position())
    }

    pub(crate) fn heard(&self) -> f64 {
        self.audio.as_ref().map_or(0.0, |a| a.shared.heard_now())
    }

    pub(crate) fn playing(&self) -> bool {
        self.audio.as_ref().is_some_and(|a| a.shared.is_playing())
    }

    pub(crate) fn recording(&self) -> bool {
        self.live.recording()
    }

    // ------------------------------------------------------------ engine sync

    /// Bring the engine up to date with the project: instruments, the mix, the
    /// timeline (not during a drag or a take), and the loop.
    fn sync_engine(&mut self, dragging: bool) {
        if self.audio.is_none() {
            return;
        }
        let sr = self.audio.as_ref().map_or(48000, |a| a.sample_rate);
        let p = self.doc.project.clone();
        let prev = self.synced.take();
        let rebuild_all = prev.as_ref().is_none_or(|q| q.tracks.len() != p.tracks.len());
        if rebuild_all {
            let mut list = Vec::new();
            self.inst_errors.clear();
            for t in &p.tracks {
                let (inst, err) = make_instrument(&t.source, &self.fonts, sr);
                list.push(inst);
                self.inst_errors.push(err);
            }
            self.send(Cmd::Instruments(list));
        } else if let Some(q) = &prev {
            for (i, (a, b)) in p.tracks.iter().zip(&q.tracks).enumerate() {
                if a.source == b.source {
                    continue;
                }
                match (&a.source, &b.source) {
                    (Source::Synth(pa), Source::Synth(_)) => self.send(Cmd::Patch(i, Box::new(pa.clone()))),
                    _ => {
                        let (inst, err) = make_instrument(&a.source, &self.fonts, sr);
                        self.send(Cmd::Instrument(i, inst));
                        self.inst_errors[i] = err;
                    }
                }
            }
        }
        if rebuild_all || prev.as_ref().is_some_and(|q| MixSettings::of(q) != MixSettings::of(&p)) {
            self.send(Cmd::Mix(Box::new(MixSettings::of(&p))));
        }
        let music_changed = self.synced_music.as_ref().is_none_or(|q| !same_music(q, &p)) || rebuild_all;
        if music_changed && !dragging && !self.recording() {
            // at a new tempo the playhead stays on the same beat, not the same second
            if let Some(old) = self.synced_music.as_ref().map(|q| q.bpm).filter(|b| *b != p.bpm && p.bpm > 0.0) {
                let at = self.position() * old / p.bpm;
                self.send(Cmd::Seek(at));
                self.play_from *= old / p.bpm;
            }
            self.send(Cmd::Timeline(Arc::new(Timeline::from_project(&p))));
            self.synced_music = Some(p.clone());
        }
        let lp = p.loop_bars.map(|(a, b)| (p.bar_secs() * a as f64, p.bar_secs() * b as f64));
        if self.synced_loop != Some(lp) {
            self.send(Cmd::Loop(lp));
            self.synced_loop = Some(lp);
        }
        self.synced = Some(p);
        let target = self.sel.min(self.doc.project.tracks.len().saturating_sub(1));
        if self.live.track.load(Ordering::Relaxed) != target {
            self.live.release_all();
            self.live.track.store(target, Ordering::Relaxed);
        }
    }

    /// Force every instrument to be made again (after loading a SoundFont).
    pub(crate) fn rebuild_instruments(&mut self) {
        self.synced = None;
    }

    // ------------------------------------------------------------ transport

    pub(crate) fn toggle_play(&mut self) {
        if self.recording() {
            self.stop_recording();
        } else if self.playing() {
            self.send(Cmd::Pause);
        } else {
            let mut at = self.position().max(0.0);
            if at >= self.project().music_secs() - 0.05 {
                at = 0.0;
            }
            self.play_from = at;
            self.send(Cmd::Play(at));
        }
    }

    /// Stop; a second stop goes back to the start.
    pub(crate) fn stop(&mut self) {
        if self.recording() {
            self.stop_recording();
            return;
        }
        if self.playing() {
            self.send(Cmd::Pause);
            self.send(Cmd::Seek(self.play_from));
        } else {
            self.send(Cmd::Seek(0.0));
            self.play_from = 0.0;
        }
    }

    pub(crate) fn seek(&mut self, secs: f64) {
        self.play_from = secs.max(0.0);
        self.send(Cmd::Seek(secs.max(0.0)));
    }

    /// Move the playhead to the start of the next section, or back to the start of this
    /// one (the one before, if it's just there, or playing within a second of it), and
    /// show it in the roll.
    pub(crate) fn skip_section(&mut self, forward: bool) {
        let p = &self.doc.project;
        let starts: Vec<u32> = p.block_ticks().iter().map(|(a, _)| *a).collect();
        let labels = p.section_labels();
        let now = p.secs_tick(self.position().max(0.0));
        let grace = if self.playing() { p.secs_tick(1.0) } else { p.beat_ticks() / 8 };
        let found =
            if forward { starts.iter().position(|&s| s > now + p.beat_ticks() / 8) } else { starts.iter().rposition(|&s| s + grace < now) };
        let (tick, what) = match found {
            Some(i) => (starts[i], labels.get(i).cloned().unwrap_or_default()),
            None if forward => {
                self.status = "The last section".into();
                return;
            }
            None => (0, "The start".into()),
        };
        let (secs, bar) = (p.tick_secs(tick), tick / p.bar_ticks().max(1) + 1);
        self.seek(secs);
        self.roll.reveal = Some(tick as f64);
        self.status = format!("{what}, bar {bar}");
    }

    pub(crate) fn toggle_record(&mut self) {
        if self.recording() {
            self.stop_recording();
            return;
        }
        if self.doc.project.tracks.is_empty() {
            self.status = "No track to record onto: start a new piece from File".into();
            return;
        }
        let was_playing = self.playing();
        let start = self.position().max(0.0);
        *self.live.take.lock().unwrap_or_else(|e| e.into_inner()) = Some(Take::new(self.sel));
        self.rec_from = start;
        self.send(Cmd::Endless(true));
        if !was_playing {
            let pre = if self.settings.count_in { self.project().bar_secs() } else { 0.0 };
            self.play_from = start;
            self.send(Cmd::Play(start - pre));
        }
        self.status = format!("Recording onto {}", self.doc.project.tracks[self.sel].name);
    }

    pub(crate) fn stop_recording(&mut self) {
        let take = self.live.take.lock().unwrap_or_else(|e| e.into_inner()).take();
        let now = self.heard();
        self.send(Cmd::Pause);
        self.send(Cmd::Endless(false));
        self.send(Cmd::Seek(self.rec_from));
        if let Some(take) = take {
            if take.is_empty() {
                self.status = "Nothing was played".into();
                return;
            }
            let grid = self.settings.snap.ticks(&self.doc.project);
            let q = self.settings.quantize_takes.then_some((grid, 1.0));
            let track = take.track;
            let n = take.commit(&mut self.doc.project, now, q, self.settings.replace_takes);
            self.status = format!("Recorded {n} notes onto {}", self.doc.project.tracks.get(track).map_or("", |t| t.name.as_str()));
        }
    }

    /// The panic button: stop where it is (a take ends), and silence every note and the
    /// reverb's, echo's and tape's tails at once, for a clean start.
    pub(crate) fn panic_stop(&mut self) {
        if self.recording() {
            self.stop_recording();
        } else if self.playing() {
            self.send(Cmd::Pause);
        }
        self.panic();
        self.status = "Panic: everything silenced".into();
    }

    pub(crate) fn panic(&mut self) {
        self.live.release_all();
        self.send(Cmd::Panic);
        self.perf = Perf::default();
    }

    /// A message from an on-screen control.
    pub(crate) fn play_live(&self, msg: Msg) {
        self.live.play(msg);
    }

    // ------------------------------------------------------------ undo

    fn track_undo(&mut self, pointer_down: bool) {
        if pointer_down || self.doc.project == self.committed {
            return;
        }
        let before = std::mem::replace(&mut self.committed, self.doc.project.clone());
        self.undo.push(before);
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub(crate) fn undo(&mut self) {
        if let Some(p) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc.project, p));
            self.committed = self.doc.project.clone();
            self.shape = Shape::of(&self.doc.project);
            self.roll.selected.clear();
            self.sel = self.sel.min(self.doc.project.tracks.len().saturating_sub(1));
        }
    }

    pub(crate) fn redo(&mut self) {
        if let Some(p) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc.project, p));
            self.committed = self.doc.project.clone();
            self.shape = Shape::of(&self.doc.project);
            self.roll.selected.clear();
            self.sel = self.sel.min(self.doc.project.tracks.len().saturating_sub(1));
        }
    }

    // ------------------------------------------------------------ files

    fn set_doc(&mut self, doc: Document, path: Option<PathBuf>) {
        self.panic();
        self.send(Cmd::Pause);
        self.send(Cmd::Seek(0.0));
        self.doc = doc;
        self.saved = self.doc.clone();
        self.committed = self.doc.project.clone();
        self.undo.clear();
        self.redo.clear();
        self.sel = 0;
        self.roll = RollState::default();
        self.roll.fit_to(&self.doc.project);
        self.synced = None;
        self.synced_music = None;
        self.shape = Shape::of(&self.doc.project);
        if let Some(p) = &path {
            self.recent.retain(|r| r != p);
            self.recent.insert(0, p.clone());
            self.recent.truncate(8);
        }
        self.path = path;
    }

    /// Write the piece again for a recipe at the current tempo, metre and length.
    /// `same_style`: the piece was in this style before (its dials and beds of ambience
    /// are kept; another style brings its own).
    pub(crate) fn recompose(&mut self, recipe: Recipe, keep: bool, same_style: bool) {
        self.recompose_to(recipe, keep, same_style, None);
    }

    /// Write the piece again in square phrases (one, two or four bars a chord), its
    /// sections moved to whole chords of the new length, its sounds, dials and beds kept.
    pub(crate) fn square_phrases(&mut self) {
        let p = &self.doc.project;
        let Some(r) = p.recipe else { return };
        let recipe = Recipe { square: true, written_at: 0.0, ..r };
        let probe = Project { recipe: Some(recipe), ..p.clone() };
        let cb = dd_seq::compose::chord_bars_at(&probe, p.bpm);
        let plan = p.plan().map(|plan| dd_seq::compose::snap_plan(&plan, cb));
        self.recompose_to(recipe, true, true, plan);
        self.status = format!("Written again in square phrases: {cb} bar{} a chord", if cb == 1 { "" } else { "s" });
    }

    /// Write the piece again for a recipe, to an arrangement of sections if one is
    /// given, else to the current one (if the composer can write it).
    fn recompose_to(&mut self, recipe: Recipe, keep: bool, same_style: bool, plan: Option<Vec<(dd_seq::project::Part, u32)>>) {
        let old = &self.doc.project;
        let same_sections = plan.is_none();
        let mut p = match plan.or_else(|| old.plan()) {
            Some(plan) => dd_seq::compose::compose_plan(&recipe, old.bpm, old.meter, &plan),
            None => compose(&recipe, old.bpm, old.meter, old.length_bars.max(2)),
        };
        if keep {
            keep_sounds(&mut p, old);
        }
        keep_levels(&mut p, old);
        // the same style keeps its dials and its beds of ambience as they were set
        // (the recipe may already name the new style: the caller knows)
        if same_style {
            dd_seq::shape::keep_dials(&mut p, old);
            dd_seq::ambience::keep(&mut p, old);
        }
        dd_seq::compose::keep_added(&mut p, old, keep);
        // the same sections keep the names they were given
        if same_sections && p.sections.len() == old.sections.len() {
            for (s, o) in p.sections.iter_mut().zip(&old.sections) {
                s.name = o.name.clone();
            }
        }
        let bars = p.bars();
        p.loop_bars = old.loop_bars.filter(|(a, _)| *a < bars).map(|(a, b)| (a, b.min(bars)));
        self.doc.project = p;
        self.sel = self.sel.min(self.doc.project.tracks.len().saturating_sub(1));
        self.roll.selected.clear();
        self.shape = Shape::of(&self.doc.project);
    }

    /// Take the peaks the engine has measured since the last frame (taking them resets
    /// them, so this is the only reader) and let the meters fall at 26 dB a second.
    fn update_meters(&mut self, ctx: &egui::Context) {
        let n = self.doc.project.tracks.len();
        let now = ctx.input(|i| i.time);
        let dt = (now - self.meters_at).clamp(0.0, 0.25) as f32;
        self.meters_at = now;
        let fall = 10f32.powf(-26.0 * dt / 20.0);
        let mut fresh: Vec<f32> = (0..n).map(|i| self.audio.as_ref().map_or(0.0, |a| a.shared.take_peak(i))).collect();
        let (l, r) = self.audio.as_ref().map_or((0.0, 0.0), |a| a.shared.take_master());
        fresh.extend([l, r]);
        self.meters.resize(n + 2, 0.0);
        for (m, f) in self.meters.iter_mut().zip(fresh) {
            *m = f.max(*m * fall);
            if *m < 1e-5 {
                *m = 0.0;
            }
        }
        if self.meters.iter().any(|m| *m > 0.0) {
            ctx.request_repaint();
        }
    }

    /// The selected track is always one of the music's (a bed of ambience isn't played,
    /// recorded or edited in the roll): the nearest one after it, else before.
    fn keep_sel_on_music(&mut self) {
        let tracks = &self.doc.project.tracks;
        if tracks.get(self.sel).is_none_or(|t| !t.ambience) {
            return;
        }
        if let Some(i) = (self.sel..tracks.len()).chain((0..self.sel).rev()).find(|&i| !tracks[i].ambience) {
            self.sel = i;
        }
    }

    /// A track's meter level.
    pub(crate) fn meter(&self, track: usize) -> f32 {
        self.meters.get(track).copied().unwrap_or(0.0)
    }

    /// A new seed, key and mode in the same style, at this tempo and length: with
    /// `new_sections`, a new arrangement of sections too (🎲 New piece); without, the
    /// sections as they are (New notes).
    pub(crate) fn new_piece(&mut self, new_sections: bool) {
        let style = self.doc.project.recipe.map(|r| r.style).unwrap_or_default();
        let mut r = Recipe { style, ..Recipe::random(entropy()) };
        style.settle(&mut r);
        let p = &self.doc.project;
        let plan = new_sections.then(|| dd_seq::compose::random_plan(&r, p.bpm, p.meter, p.length_bars.max(2)));
        self.recompose_to(r, self.settings.keep_sounds, true, plan);
        if self.settings.vary_dials {
            dd_seq::shape::vary(&mut self.doc.project, r.seed);
        }
        self.roll.fit_to(&self.doc.project);
        self.status = format!("\u{201c}{}\u{201d}: {} in {}, seed {}", self.doc.project.title, r.style.label(), r.key_label(), r.seed);
    }

    /// Do what a section's button or menu asked: the piece keeps its other notes, and
    /// isn't written again around the change (it now counts as edited).
    pub(crate) fn section_action(&mut self, act: crate::roll::SectionAct) {
        use crate::roll::SectionAct;
        let i = act.section();
        let what = self.doc.project.section_labels().get(i).cloned().unwrap_or_default();
        let p = &mut self.doc.project;
        self.status = match act {
            SectionAct::Regenerate(i) => {
                let seed = entropy() % 1_000_000;
                if dd_seq::compose::regenerate_section(p, i, seed) {
                    format!("New notes for {what} (seed {seed})")
                } else {
                    "Only a composed piece can have new notes written".into()
                }
            }
            SectionAct::RegenerateTrack(i, k) => {
                let seed = entropy() % 1_000_000;
                let name = p.tracks.get(k).map_or(String::new(), |t| t.name.clone());
                if dd_seq::compose::regenerate_track_section(p, i, k, seed) {
                    self.sel = k;
                    format!("New notes for the {name} in {what} (seed {seed})")
                } else if p.recipe.is_none() || p.plan().is_none() {
                    "Only a composed piece can have new notes written".into()
                } else {
                    format!("The style doesn't write a part for the {name}")
                }
            }
            SectionAct::Copy(i, after) => {
                p.copy_section(i, after);
                format!("Copied {what} {}", if after { "after itself" } else { "in front of itself" })
            }
            SectionAct::Delete(i) => {
                p.delete_section(i);
                format!("Took out {what}")
            }
            SectionAct::Rename(i, name) => {
                let mut blocks = p.arrangement();
                if let Some(b) = blocks.get_mut(i) {
                    b.name = if name.is_empty() { b.part.map_or(String::new(), |p| p.label().into()) } else { name };
                }
                p.set_arrangement(&blocks);
                let now = p.section_labels().get(i).cloned().unwrap_or_default();
                format!("{what} is now {now}")
            }
            SectionAct::SetPart(i, part) => {
                let mut blocks = p.arrangement();
                let origin: Vec<Option<usize>> = (0..blocks.len()).map(Some).collect();
                if let Some(b) = blocks.get_mut(i) {
                    let named = b.own_name().is_some();
                    *b =
                        Block { name: if named { b.name.clone() } else { part.label().into() }, part: Some(part), seed: None, ..b.clone() };
                }
                self.apply_arrangement_with(blocks, origin, &[i]);
                format!("{what} is now a {}", part.label())
            }
        };
        self.roll.selected.clear();
        // so the change isn't taken for a new arrangement to write the piece again to
        self.shape = Shape::of(&self.doc.project);
    }

    /// Lay the sections out anew (`origin[i]`: the section block `i` was, none for a new
    /// one). An untouched composed piece is written again whole to fit (once the mouse
    /// is let go). Otherwise each section's notes move with it, and in a composed piece
    /// every section that is new, another part or another length is written again for
    /// its part and length, the rest kept as they are; in one that wasn't composed, a
    /// longer section plays its notes again to fill it.
    pub(crate) fn apply_arrangement(&mut self, blocks: Vec<Block>, origin: Vec<Option<usize>>) {
        self.apply_arrangement_with(blocks, origin, &[]);
    }

    /// As [`apply_arrangement`](Self::apply_arrangement), writing sections `also` again too.
    fn apply_arrangement_with(&mut self, blocks: Vec<Block>, origin: Vec<Option<usize>>, also: &[usize]) {
        let p = &mut self.doc.project;
        if blocks.is_empty() {
            p.sections.clear();
            return;
        }
        if p.untouched() {
            p.rearrange(&blocks, &origin);
            return;
        }
        let old = p.arrangement();
        let rewrite: Vec<usize> = blocks
            .iter()
            .zip(&origin)
            .enumerate()
            .filter(|(i, (b, o))| also.contains(i) || o.and_then(|j| old.get(j)).is_none_or(|ob| ob.part != b.part || ob.bars != b.bars))
            .map(|(i, _)| i)
            .collect();
        p.rearrange_with_notes(&blocks, &origin);
        if !rewrite.is_empty() {
            dd_seq::compose::rewrite_sections(p, &rewrite);
        }
        self.roll.selected.clear();
        self.shape = Shape::of(&self.doc.project);
    }

    /// Add a track of `role` with the notes the piece's style writes for it (else a
    /// part written over the piece's chords).
    pub(crate) fn add_track(&mut self, role: dd_seq::palette::Role) {
        let style = self.doc.project.recipe.map(|r| r.style);
        let (i, by_style) = dd_seq::compose::add_track(&mut self.doc.project, role, None);
        self.sel = i;
        let name = self.doc.project.tracks[i].name.clone();
        self.status = match (by_style, style) {
            (true, Some(s)) => format!("Added the {name}, as {} writes it", s.label()),
            (false, Some(s)) => {
                format!("Added the {name}: {} doesn't write a {} part, so one is written over the piece's chords", s.label(), role.label())
            }
            (_, None) => format!("Added the {name}, written over the piece's chords"),
        };
        self.shape = Shape::of(&self.doc.project);
    }

    /// When the tempo, metre, length or sections change, a composed piece that hasn't been
    /// edited is written again to fit; an edited one keeps its notes, with those held
    /// to the end held to the new end. A changed recipe (key, mode, seed, density)
    /// always writes the piece again. All of it waits for the mouse to be let go.
    fn follow_shape(&mut self, pointer_down: bool) {
        let now = Shape::of(&self.doc.project);
        if pointer_down || self.recording() || now.same(&self.shape) {
            self.shape.untouched = now.untouched;
            return;
        }
        let before = std::mem::replace(&mut self.shape, now.clone());
        match (before.recipe, now.recipe) {
            (Some(old), Some(r)) if old != r => {
                // a new style chooses its chord length from the tempo again
                let r = if r.style != old.style { Recipe { written_at: 0.0, ..r } } else { r };
                // a new seed brings new sounds, unless they're kept; a new style always
                // brings its own (another style's instruments don't suit it)
                let keep = r.style == old.style && (r.seed == old.seed || self.settings.keep_sounds);
                self.recompose(r, keep, r.style == old.style);
                self.status = format!("\u{201c}{}\u{201d}: {}, seed {}", self.doc.project.title, r.key_label(), r.seed);
            }
            (_, Some(r)) if before.untouched => {
                // at a new tempo the piece keeps its notes in the bar, so every part slows
                // or quickens together (else the parts the composer times in seconds,
                // its chords and bells, would change while the rest kept to the beat)
                let r = if now.bpm != before.bpm && now.meter == before.meter {
                    let was = Project { bpm: before.bpm, ..self.doc.project.clone() };
                    dd_seq::compose::at_any_tempo(&was).unwrap_or(r)
                } else {
                    r
                };
                self.recompose(r, true, true);
                self.status = format!("Written again for {} bars at {} bpm", self.doc.project.length_bars, self.doc.project.bpm);
            }
            _ if now.bars != before.bars => {
                dd_seq::compose::resize(&mut self.doc.project, before.bars, now.bars);
                self.shape = Shape::of(&self.doc.project);
            }
            _ => {}
        }
    }

    pub(crate) fn new_doc(&mut self, demo: bool) {
        let p = if demo { dd_seq::demo::rain_on_neon() } else { dd_seq::demo::blank() };
        self.set_doc(Document::new(p), None);
    }

    pub(crate) fn open(&mut self) {
        let d = rfd::AsyncFileDialog::new().add_filter("Kibble-481 project or MIDI", &[EXTENSION, OLD_EXTENSION, "mid", "midi"]);
        self.ask(Want::Open, Kind::File, d);
    }

    pub(crate) fn open_path(&mut self, path: PathBuf) {
        let is_midi =
            path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("mid") || e.eq_ignore_ascii_case("midi"));
        let result = if is_midi { dd_seq::midi_file::load(&path, &self.fonts).map(Document::new) } else { Document::load(&path) };
        match result {
            Ok(doc) => {
                self.status = format!("Opened {}", path.display());
                self.set_doc(doc, if is_midi { None } else { Some(path) });
            }
            Err(e) => self.status = format!("Couldn't open: {e}"),
        }
    }

    /// Add the tracks of a MIDI file to this project.
    pub(crate) fn merge_midi(&mut self) {
        self.ask(Want::MergeMidi, Kind::File, rfd::AsyncFileDialog::new().add_filter("MIDI", &["mid", "midi"]));
    }

    pub(crate) fn merge_midi_from(&mut self, path: PathBuf) {
        match dd_seq::midi_file::load(&path, &self.fonts) {
            Ok(q) => {
                let n = q.tracks.len();
                let base = self.doc.project.tracks.len();
                for (i, mut t) in q.tracks.into_iter().enumerate() {
                    t.color = dd_seq::project::NEON[(base + i) % dd_seq::project::NEON.len()];
                    self.doc.project.tracks.push(t);
                }
                self.status = format!("Added {n} tracks from {}", path.display());
            }
            Err(e) => self.status = format!("Couldn't read that MIDI file: {e}"),
        }
    }

    pub(crate) fn save(&mut self, as_new: bool) {
        match (&self.path, as_new) {
            (Some(p), false) => self.save_to(p.clone()),
            _ => {
                let name = format!("{}.{EXTENSION}", self.version_name());
                let d = rfd::AsyncFileDialog::new().add_filter("Kibble-481 project", &[EXTENSION]).set_file_name(name);
                self.ask(Want::Save, Kind::Save, d);
            }
        }
    }

    pub(crate) fn save_to(&mut self, path: PathBuf) {
        match self.doc.save(&path) {
            Ok(()) => {
                self.saved = self.doc.clone();
                self.status = format!("Saved {}", path.display());
                self.recent.retain(|r| r != &path);
                self.recent.insert(0, path.clone());
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Couldn't save: {e}"),
        }
    }

    /// The name a save or export suggests: the title, the seed and the time in UTC.
    fn version_name(&self) -> String {
        dd_gallery::export::version_name(&self.doc.project, std::time::SystemTime::now())
    }

    pub(crate) fn dirty(&self) -> bool {
        self.doc != self.saved
    }

    pub(crate) fn save_midi(&mut self) {
        let d = rfd::AsyncFileDialog::new().add_filter("MIDI", &["mid"]).set_file_name(format!("{}.mid", self.version_name()));
        self.ask(Want::SaveMidi, Kind::Save, d);
    }

    pub(crate) fn save_midi_to(&mut self, path: PathBuf) {
        self.status = match dd_seq::midi_file::save(&self.doc.project, &path) {
            Ok(()) => format!("Saved {}", path.display()),
            Err(e) => format!("Couldn't save the MIDI file: {e}"),
        };
    }

    pub(crate) fn export_wav(&mut self) {
        let d = rfd::AsyncFileDialog::new().add_filter("WAV", &["wav"]).set_file_name(format!("{}.wav", self.version_name()));
        self.ask(Want::ExportWav, Kind::Save, d);
    }

    pub(crate) fn export_wav_to(&mut self, path: PathBuf) {
        let (p, fonts) = (self.doc.project.clone(), self.fonts.clone());
        self.start_job("Rendering the mix", move |progress, cancel| {
            dd_seq::render::render_wav(&p, &fonts, None, &path, progress, cancel)?;
            Ok(format!("Exported {}", path.display()))
        });
    }

    pub(crate) fn export_stems(&mut self) {
        self.ask(Want::Stems, Kind::Folder, rfd::AsyncFileDialog::new().set_title("Choose a folder for the stems"));
    }

    pub(crate) fn export_stems_to(&mut self, dir: PathBuf) {
        let (p, fonts, name) = (self.doc.project.clone(), self.fonts.clone(), self.version_name());
        self.start_job("Rendering stems", move |progress, cancel| {
            let mut n = 0;
            for (i, t) in p.tracks.iter().enumerate() {
                if t.notes.is_empty() {
                    continue;
                }
                let path = dir.join(format!("{name} - {}.wav", dd_gallery::export::safe_name(&t.name)));
                dd_seq::render::render_wav(&p, &fonts, Some(i), &path, progress, cancel)?;
                n += 1;
            }
            Ok(format!("Exported {n} stems to {}", dir.display()))
        });
    }

    pub(crate) fn export_resolve(&mut self) {
        self.ask(Want::Resolve, Kind::Folder, rfd::AsyncFileDialog::new().set_title("Choose a folder for the Resolve package"));
    }

    pub(crate) fn export_resolve_to(&mut self, dir: PathBuf) {
        let (doc, fonts, stems, name) = (self.doc.clone(), self.fonts.clone(), self.settings.stems_with_gallery, self.version_name());
        self.start_job("Exporting for Resolve", move |progress, cancel| {
            let out = dd_gallery::export::export_for_resolve(&doc, &fonts, &dir, &name, stems, progress, cancel)?;
            Ok(match out.timeline {
                Some(t) => format!("Exported {} with the soundtrack — in Resolve: File › Import › Timeline", t.display()),
                None => format!("Exported the soundtrack and markers to {} (choose a photo folder for a timeline)", dir.display()),
            })
        });
    }

    /// Export the gallery as a video of `size` (kept as the gallery's choice).
    pub(crate) fn export_video(&mut self, size: dd_gallery::slideshow::VideoSize) {
        self.doc.gallery.video = size;
        if self.doc.gallery.folder.is_none() {
            self.status = "Choose a photo folder on the Gallery tab first: the video shows its photos".into();
            return;
        }
        if dd_gallery::video::ffmpeg().is_none() {
            self.status =
                "Video export needs ffmpeg (macOS: brew install ffmpeg; Windows: ffmpeg.exe beside Kibble-481 or on the PATH)".into();
            return;
        }
        let d = rfd::AsyncFileDialog::new()
            .add_filter("MPEG-4 video", &["mp4"])
            .add_filter("QuickTime movie", &["mov"])
            .set_file_name(format!("{}.mp4", self.version_name()));
        self.ask(Want::Video, Kind::Save, d);
    }

    pub(crate) fn export_video_to(&mut self, path: PathBuf) {
        let path = match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase) {
            Some(e) if e == "mp4" || e == "mov" => path,
            _ => path.with_extension("mp4"),
        };
        let (doc, fonts) = (self.doc.clone(), self.fonts.clone());
        self.start_job("Rendering the video", move |progress, cancel| {
            let made = dd_gallery::video::export_video(&doc, &fonts, &path, progress, cancel)?;
            let unread = match made.unread.len() {
                0 => String::new(),
                n => format!(" ({n} photos couldn't be read and show as black)"),
            };
            Ok(format!(
                "Exported {}: {}×{}, {} fps, {} frames ({}){unread}; with {} and {}",
                path.display(),
                made.dims.0,
                made.dims.1,
                made.fps,
                made.frames,
                dd_gallery::slideshow::timecode(made.secs),
                made.midi.file_name().unwrap_or_default().to_string_lossy(),
                made.cues.file_name().unwrap_or_default().to_string_lossy()
            ))
        });
    }

    /// Open the pane to reorder the gallery's photos by hand.
    pub(crate) fn open_reorder(&mut self, ctx: &egui::Context) {
        let g = &self.doc.gallery;
        let Some(folder) = &g.folder else { return };
        let base = if g.order == dd_gallery::resolve::PhotoOrder::Custom { dd_gallery::resolve::PhotoOrder::Name } else { g.order };
        match (g.photos(), dd_gallery::resolve::photos(folder, base)) {
            (Ok(now), Ok(listed)) if !now.is_empty() => self.reorder = Some(crate::reorder::Reorder::new(ctx, now, listed)),
            (Err(e), _) | (_, Err(e)) => self.status = e,
            _ => self.status = "No photos in the folder".into(),
        }
    }

    fn start_job(&mut self, label: &str, f: impl FnOnce(&AtomicU32, &AtomicBool) -> Result<String, String> + Send + 'static) {
        if self.job.is_some() {
            return;
        }
        let progress = Arc::new(AtomicU32::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (p2, c2) = (progress.clone(), cancel.clone());
        let handle = std::thread::spawn(move || f(&p2, &c2));
        self.job = Some(Job { label: label.into(), handle: Some(handle), progress, cancel });
    }

    fn poll_job(&mut self) {
        let Some(job) = &mut self.job else { return };
        if job.handle.as_ref().is_some_and(|h| h.is_finished()) {
            let r = job.handle.take().map(|h| h.join().unwrap_or(Err("the export stopped unexpectedly".into())));
            self.status = match r {
                Some(Ok(s)) => s,
                Some(Err(e)) => format!("Export failed: {e}"),
                None => String::new(),
            };
            self.job = None;
        }
    }

    // ------------------------------------------------------------ keyboard

    fn keyboard(&mut self, ctx: &egui::Context) {
        let typing = ctx.memory(|m| m.focused().is_some());
        let events = ctx.input(|i| i.events.clone());
        let cmd = egui::Modifiers::COMMAND;
        if ctx.input_mut(|i| i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::S)) {
            self.save(true);
        } else if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::S)) {
            self.save(false);
        }
        if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::O)) {
            self.open();
        }
        if typing {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(cmd | egui::Modifiers::SHIFT, egui::Key::Z)) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_key(cmd, egui::Key::Z)) {
            self.undo();
        }
        for e in events {
            let egui::Event::Key { key, pressed, repeat, modifiers, .. } = e else { continue };
            if repeat || modifiers.command || modifiers.ctrl {
                continue;
            }
            match key {
                egui::Key::Space if pressed => self.toggle_play(),
                egui::Key::R if pressed && modifiers.shift => self.toggle_record(),
                egui::Key::Enter if pressed => self.stop(),
                egui::Key::OpenBracket if pressed => self.skip_section(false),
                egui::Key::CloseBracket if pressed => self.skip_section(true),
                _ => {}
            }
            if !self.settings.qwerty || modifiers.shift {
                continue;
            }
            match key {
                egui::Key::Z if pressed => self.octave = (self.octave - 1).max(0),
                egui::Key::X if pressed => self.octave = (self.octave + 1).min(8),
                egui::Key::C if pressed => self.key_vel = self.key_vel.saturating_sub(16).max(16),
                egui::Key::V if pressed => self.key_vel = (self.key_vel + 16).min(127),
                _ => {}
            }
            if let Some(off) = qwerty_key(key) {
                if pressed {
                    let k = (self.octave * 12 + off).clamp(0, 127) as u8;
                    if !self.qwerty_down.iter().any(|(q, _)| *q == key) {
                        self.qwerty_down.push((key, k));
                        self.play_live(Msg::On(k, self.key_vel));
                    }
                } else if let Some(i) = self.qwerty_down.iter().position(|(q, _)| *q == key) {
                    let (_, k) = self.qwerty_down.remove(i);
                    self.play_live(Msg::Off(k));
                }
            }
        }
    }

    // ------------------------------------------------------------ layout

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(WORDMARK).font(t.label(14.0)).color(t.accent))
                .on_hover_text("Neon-noir synthesiser and sequencer");
            ui.add_space(8.0);
            ui.menu_button("File", |ui| {
                if ui.button("New composed piece  🎲").on_hover_text("New notes and new sections").clicked() {
                    self.new_piece(true);
                    ui.close();
                }
                if ui.button("New notes, same sections").clicked() {
                    self.new_piece(false);
                    ui.close();
                }
                if ui.button("New from Rain on Neon").on_hover_text("The hand-written starter scene").clicked() {
                    self.new_doc(true);
                    ui.close();
                }
                if ui.button("New, empty").clicked() {
                    self.new_doc(false);
                    ui.close();
                }
                ui.separator();
                if ui.button("Open…  ⌘O").on_hover_text("A Kibble-481 project, or any MIDI file").clicked() {
                    ui.close();
                    self.open();
                }
                if !self.recent.is_empty() {
                    ui.menu_button("Open recent", |ui| {
                        for p in self.recent.clone() {
                            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            if ui.button(name).on_hover_text(p.display().to_string()).clicked() {
                                self.open_path(p);
                                ui.close();
                            }
                        }
                    });
                }
                if ui.button("Add tracks from a MIDI file…").on_hover_text("From Ambient MIDI Generator, a DAW, anything").clicked() {
                    ui.close();
                    self.merge_midi();
                }
                ui.separator();
                if ui.button("Save  ⌘S").clicked() {
                    ui.close();
                    self.save(false);
                }
                if ui.button("Save as…  ⇧⌘S").clicked() {
                    ui.close();
                    self.save(true);
                }
            });
            if ui
                .button(egui::RichText::new("🎲 New piece").color(t.accent))
                .on_hover_text(
                    "Compose a new piece: new notes (a new seed, key and mode) and a new arrangement of sections, at this tempo and length. Undo brings the last one back.",
                )
                .clicked()
            {
                self.new_piece(true);
            }
            if ui
                .button("New notes")
                .on_hover_text("New notes (a new seed, key and mode) to the sections as they are. Undo brings the last ones back.")
                .clicked()
            {
                self.new_piece(false);
            }
            ui.menu_button("Export", |ui| {
                if ui.button("MIDI file…").on_hover_text("Standard MIDI, a track per track, for any DAW or notation app").clicked() {
                    ui.close();
                    self.save_midi();
                }
                if ui.button("Soundtrack (WAV)…").on_hover_text("The mix, 24-bit 48 kHz, with its reverb tail").clicked() {
                    ui.close();
                    self.export_wav();
                }
                if ui
                    .button("Stems (WAV per track)…")
                    .on_hover_text("Each track alone with its reverb and echo, for mixing in Fairlight")
                    .clicked()
                {
                    ui.close();
                    self.export_stems();
                }
                ui.separator();
                if ui
                    .button("Gallery for DaVinci Resolve…")
                    .on_hover_text("Soundtrack, FCPXML timeline of your photos, EDL markers and a cue list")
                    .clicked()
                {
                    ui.close();
                    self.export_resolve();
                }
                for size in dd_gallery::slideshow::VideoSize::ALL {
                    let (w, h) = size.dims();
                    if ui
                        .button(format!("Gallery as {} video…", size.label()))
                        .on_hover_text(format!(
                            "Your photos over the soundtrack with the gallery's transition, {w}×{h} H.264 at its frame rate (needs ffmpeg)"
                        ))
                        .clicked()
                    {
                        ui.close();
                        self.export_video(size);
                    }
                }
            });
            ui.add_space(8.0);
            if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("↶")).on_hover_text("Undo (⌘Z)").clicked() {
                self.undo();
            }
            if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("↷")).on_hover_text("Redo (⇧⌘Z)").clicked() {
                self.redo();
            }
            ui.add_space(8.0);
            ui.add(egui::TextEdit::singleline(&mut self.doc.project.title).desired_width(200.0).font(t.semibold(13.0)))
                .on_hover_text("The title: used for exported file names");
            if self.dirty() {
                ui.label(egui::RichText::new("●").color(t.text_dim)).on_hover_text("Unsaved changes");
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if widgets::icon_button(ui, Icon::Info, self.about.open, "About, licences and updates").clicked() {
                    self.about.open = !self.about.open;
                }
                if let Some(crate::update::UpdateStatus::Available { version, url }) = &self.update {
                    let resp = ui
                        .add(egui::Button::new(egui::RichText::new(format!("⬆ {version}")).color(Tokens::get(ui.ctx()).accent)).small())
                        .on_hover_text(format!(
                            "Version {version} is available (you have {}). Click to download.",
                            crate::update::VERSION
                        ));
                    if resp.clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                    }
                }
                let resp = widgets::icon_button(ui, Icon::Palette, false, &format!("Colour palette: {}", self.palette.label()));
                egui::Popup::menu(&resp).show(|ui| {
                    ui.set_min_width(250.0);
                    for p in Palette::ALL {
                        if widgets::palette_row(ui, p, p == self.palette).clicked() {
                            self.palette = p;
                            crate::theme::set_palette(ui.ctx(), p);
                            ui.close();
                        }
                    }
                });
                if let Some(job) = &self.job {
                    if ui.button("Cancel").clicked() {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                    let f = job.progress.load(Ordering::Relaxed) as f32 / 1000.0;
                    ui.add(egui::ProgressBar::new(f).desired_width(160.0).text(job.label.clone()));
                } else if !self.status.is_empty() {
                    ui.add(egui::Label::new(egui::RichText::new(&self.status).color(t.text_dim).font(t.font(11.5))).truncate());
                }
            });
        });
    }
}

/// The time over which a project is written.
#[derive(Clone, Debug, Default, PartialEq)]
struct Shape {
    bpm: f64,
    meter: (u32, u32),
    bars: u32,
    /// The sections' parts and lengths.
    plan: Vec<(Option<dd_seq::project::Part>, u32)>,
    recipe: Option<Recipe>,
    /// Composed and not edited, as of the last frame.
    untouched: bool,
}

impl Shape {
    fn of(p: &Project) -> Shape {
        Shape {
            bpm: p.bpm,
            meter: p.meter,
            bars: p.length_bars,
            plan: p.arrangement().into_iter().map(|b| (b.part, b.bars)).collect(),
            recipe: p.recipe,
            untouched: p.untouched(),
        }
    }

    fn same(&self, o: &Shape) -> bool {
        (self.bpm, self.meter, self.bars, &self.plan, self.recipe) == (o.bpm, o.meter, o.bars, &o.plan, o.recipe)
    }
}

fn entropy() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
}

/// Whether two projects play the same notes at the same times, at the same bar levels.
fn same_music(a: &Project, b: &Project) -> bool {
    a.bpm == b.bpm
        && a.meter == b.meter
        && a.length_bars == b.length_bars
        && a.tracks.len() == b.tracks.len()
        && a.tracks.iter().zip(&b.tracks).all(|(x, y)| {
            x.notes == y.notes
                && x.density == y.density
                && x.complexity == y.complexity
                && x.ctrls == y.ctrls
                && x.levels == y.levels
                && x.source.is_drums() == y.source.is_drums()
        })
}

impl App {
    /// Ask GitHub for the latest release on a thread; the answer is picked up each frame.
    fn start_update_check(&mut self, ctx: &egui::Context) {
        if matches!(self.update, Some(crate::update::UpdateStatus::Checking)) {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(crate::update::check());
            ctx.request_repaint();
        });
        self.update = Some(crate::update::UpdateStatus::Checking);
        self.update_rx = Some(rx);
    }

    fn poll_update_check(&mut self) {
        if let Some(rx) = &self.update_rx
            && let Ok(status) = rx.try_recv()
        {
            self.update = Some(status);
            self.update_rx = None;
        }
    }
}

impl eframe::App for App {
    fn persist_egui_memory(&self) -> bool {
        self.snapshot.is_none()
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(s) = &mut self.snapshot {
            s.inject(raw);
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.snapshot.is_some() {
            return;
        }
        eframe::set_value(storage, KEY_PALETTE, &self.palette.key().to_string());
        self.settings.midi_port = self.midi.port.clone();
        eframe::set_value(storage, KEY_SETTINGS, &self.settings);
        eframe::set_value(storage, KEY_RECENT, &self.recent);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.keyboard(&ctx);
        self.poll_job();
        self.poll_dialog(&ctx);
        self.update_meters(&ctx);
        self.keep_sel_on_music();
        if self.recording() && !self.playing() && self.position() > 0.0 {
            // the take ran into the end
            self.stop_recording();
        }
        let title = match &self.path {
            Some(p) => format!("{} — {APP_NAME}", p.file_stem().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
            None => format!("{} — {APP_NAME}", self.doc.project.title),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        let tokens = Tokens::get(&ctx);
        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("transport").resizable(false).exact_size(44.0).show(ui, |ui| self.transport(ui));
        // the settings run the full height, beside the keys and the mixer as well
        egui::Panel::right("inspector")
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(0, 4)))
            .resizable(true)
            .default_size(380.0)
            .size_range(330.0..=640.0)
            .show(ui, |ui| self.inspector(ui));
        if self.settings.show_keys {
            egui::Panel::bottom("keys").resizable(false).exact_size(96.0).show(ui, |ui| self.performance(ui));
        }
        if self.settings.show_mixer {
            egui::Panel::bottom("mixer").resizable(false).show(ui, |ui| self.mixer(ui));
        }
        egui::Panel::left("tracks")
            .frame(egui::Frame::side_top_panel(ui.style()).inner_margin(egui::Margin::symmetric(6, 6)))
            .resizable(true)
            .default_size(300.0)
            .size_range(260.0..=460.0)
            .show(ui, |ui| self.tracks_panel(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).fill(tokens.canvas).inner_margin(0.0))
            .show(ui, |ui| self.roll_ui(ui));
        self.poll_update_check();
        self.about.show(&ctx, &self.update, &mut self.settings.check_updates, &self.fonts);
        if let Some(r) = &mut self.reorder
            && let Some(done) = r.show(&ctx)
        {
            if let crate::reorder::Outcome::Save(order) = done {
                let g = &mut self.doc.gallery;
                g.arranged = order.iter().filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned())).collect();
                g.order = dd_gallery::resolve::PhotoOrder::Custom;
                self.status = format!("The gallery's {} photos are in your order now", order.len());
            }
            self.reorder = None;
        }
        if std::mem::take(&mut self.about.check_requested) {
            self.start_update_check(&ctx);
        }

        let pointer_down = ctx.input(|i| i.pointer.any_down());
        self.sel = self.sel.min(self.doc.project.tracks.len().saturating_sub(1));
        self.sync_engine(pointer_down);
        // after the sync, so the preview plays on the new sound
        if let Some((track, notes)) = self.audition.take() {
            self.send(Cmd::Audition(track, notes));
        }
        self.follow_shape(pointer_down);
        self.track_undo(pointer_down);
        if let Some(s) = &mut self.snapshot {
            s.tick(&ctx);
        }
        // (the mixer's meters ask for frames themselves while they move: an open mixer
        // over silence lets the app idle)
        if self.playing() || self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}
