//! The panels around the roll: tracks, the inspector's tabs (effects, score, gallery),
//! the transport, the performance strip and About.

use dd_dsp::fx::{EchoTime, Interval, Space, Tuning};
use dd_dsp::kit::fx;
use dd_dsp::rain::RainKind;
use dd_dsp::sampler::KitKind;
use dd_gallery::resolve::{FRAME_RATES, PhotoOrder};
use dd_gallery::slideshow::{Fit, MAX_FADE, MAX_HOLD, MIN_HOLD, timecode};
use dd_seq::city;
use dd_seq::compose::{KEY_NAMES, Mode, Style};
use dd_seq::engine::Cmd;
use dd_seq::palette::{Alt, Role, alternatives, apply, position};
use dd_seq::project::{Block, NEON, Part, Source, Track};
use dd_seq::timeline::Msg;
use eframe::egui::{self, Align2, Color32, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::app::{App, Snap, Tab};
use crate::live::MidiIn;
use crate::theme::Tokens;
use crate::widgets::{self, Icon, Segment, SliderSpec};

fn amount(ui: &mut egui::Ui, id: &str, label: &str, v: &mut f32, default: f32, ends: (&str, &str), help: &str) {
    let spec = SliderSpec { id, label, help, ends: Some(ends), default: default as f64, ..Default::default() };
    let mut x = *v as f64;
    if widgets::slider(ui, &spec, &mut x, true) {
        *v = x as f32;
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    crate::theme::track_colour(c)
}

/// What a track row's dropdown asked for.
enum RowAction {
    Alt(Role, Box<Alt>),
    Rain(Option<RainKind>),
    Layer(u8, bool),
}

/// A small square toggle button with a letter, as on a mixing desk.
pub fn toggle(ui: &mut egui::Ui, label: &str, on: &mut bool, color: Color32, help: &str) -> bool {
    let t = Tokens::get(ui.ctx());
    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 18.0), Sense::click());
    let fill = if *on {
        color
    } else if resp.hovered() {
        t.hover
    } else {
        t.button
    };
    ui.painter().rect_filled(r, 3.0, fill);
    ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, t.button_border), StrokeKind::Inside);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, label, t.semibold(10.5), if *on { t.on_accent } else { t.text_label });
    let clicked = resp.clicked();
    if clicked {
        *on = !*on;
    }
    resp.on_hover_text(help);
    clicked
}

/// Whether the track list offers **+ Add** (a new track of any role, the city or the drums).
/// Off for now (D15): pieces come with their tracks. The menu is kept for when it returns.
const ADD_TRACKS: bool = false;

impl App {
    // ------------------------------------------------------------ tracks

    pub(crate) fn tracks_panel(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Tracks").font(t.semibold(14.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !ADD_TRACKS {
                    return;
                }
                ui.menu_button("+ Add", |ui| {
                    let n = self.doc.project.tracks.len();
                    for role in Role::ALL.into_iter().filter(|r| !matches!(r, Role::City | Role::Drums)) {
                        let alts = alternatives(role);
                        let list: Vec<&str> = alts.iter().map(|a| a.label).collect();
                        if ui
                            .button(role.label())
                            .on_hover_text(format!("Starts as {}; alternatives: {}", list[0], list[1..].join(", ")))
                            .clicked()
                        {
                            let mut tr = Track::new(role.label(), alts[0].source.clone(), n);
                            apply(&mut tr, role, &alts[0]);
                            self.doc.project.tracks.push(tr);
                            self.sel = n;
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button("City: rain, thunder, hum, neon").clicked() {
                        let mut tr = Track::new("City", Source::Kit(KitKind::CityFx), n);
                        tr.role = Some(Role::City);
                        self.doc.project.tracks.push(tr);
                        self.sel = n;
                        ui.close();
                    }
                    if ui.button("Drums").clicked() {
                        let mut tr = Track::new("Drums", Source::Kit(KitKind::Drums), n);
                        tr.role = Some(Role::Drums);
                        self.doc.project.tracks.push(tr);
                        self.sel = n;
                        ui.close();
                    }
                });
            });
        });
        ui.add_space(4.0);
        let mut action: Option<(usize, &str)> = None;
        let mut row_action: Option<(usize, RowAction)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let n = self.doc.project.tracks.len();
            for i in 0..n {
                let selected = i == self.sel;
                let peak = self.meter(i);
                let err = self.inst_errors.get(i).cloned().flatten();
                // what the row's dropdown offers
                let role = Role::of(&self.doc.project.tracks[i]);
                let alts: Vec<(Alt, bool)> = role.map_or(Vec::new(), |r| {
                    alternatives(r)
                        .into_iter()
                        .map(|a| {
                            let ok = self.alt_available(&a);
                            (a, ok)
                        })
                        .collect()
                });
                let current = role.and_then(|r| position(r, &self.doc.project.tracks[i].source));
                let city_rain = city::rain(&self.doc.project, i);
                let layers = [fx::THUNDER, fx::HUM, fx::NEON].map(|k| city::has(&self.doc.project, i, k));
                let frame = egui::Frame::new()
                    .fill(if selected { t.hover } else { t.inset })
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(6, 4))
                    .stroke(Stroke::new(1.0, if selected { rgb(self.doc.project.tracks[i].color) } else { t.divider }));
                // the row's own click sits under its controls, so M, S and the sound get theirs;
                // the name is plain text until double-clicked, so a click on it selects the row
                let renaming = self.renaming == Some(i);
                let mut name_rect = Rect::NOTHING;
                let mut rename_done = false;
                let mut touched = false;
                let resp = ui
                    .scope_builder(egui::UiBuilder::new().id_salt(("track", i)).sense(Sense::click()), |ui| {
                        frame.show(ui, |ui| {
                            let tr = &mut self.doc.project.tracks[i];
                            ui.horizontal(|ui| {
                                let (r, sw) = ui.allocate_exact_size(vec2(10.0, 30.0), Sense::click());
                                ui.painter().rect_filled(r, 2.0, rgb(tr.color));
                                if sw.on_hover_text("Click to change the colour").clicked() {
                                    let k = NEON.iter().position(|c| *c == tr.color).map_or(0, |k| (k + 1) % NEON.len());
                                    tr.color = NEON[k];
                                }
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    let name_w = ui.available_width() - 56.0;
                                    if renaming {
                                        let r = ui.add(
                                            egui::TextEdit::singleline(&mut tr.name)
                                                .id_salt(("rename", i))
                                                .frame(egui::Frame::NONE)
                                                .font(t.semibold(12.5))
                                                .desired_width(name_w),
                                        );
                                        if !r.has_focus() && !r.gained_focus() {
                                            r.request_focus();
                                        }
                                        rename_done = r.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    } else {
                                        let (r, _) = ui.allocate_exact_size(vec2(name_w, 17.0), Sense::hover());
                                        name_rect = r;
                                        let galley = ui.painter().layout_no_wrap(tr.name.clone(), t.semibold(12.5), t.text);
                                        ui.painter().with_clip_rect(r).galley(
                                            r.left_center() - vec2(0.0, galley.size().y / 2.0),
                                            galley,
                                            t.text,
                                        );
                                    }
                                    let w = (ui.available_width() - 60.0).max(80.0);
                                    let col = if err.is_some() { t.caution } else { t.text_dim };
                                    let shown = |s: &str| egui::RichText::new(s).font(t.font(10.5)).color(col);
                                    let resp = if role == Some(Role::City) {
                                        let label = city_rain.map_or("No rain".to_string(), |k| k.label().to_string());
                                        egui::ComboBox::from_id_salt(("city", i))
                                            .width(w)
                                            .height(420.0)
                                            .selected_text(shown(&label))
                                            .show_ui(ui, |ui| {
                                                ui.label(egui::RichText::new("Rain").color(t.text_dim));
                                                for k in RainKind::ALL {
                                                    if ui.selectable_label(city_rain == Some(k), k.label()).clicked() {
                                                        row_action = Some((i, RowAction::Rain(Some(k))));
                                                    }
                                                }
                                                if ui.selectable_label(city_rain.is_none(), "No rain").clicked() {
                                                    row_action = Some((i, RowAction::Rain(None)));
                                                }
                                                ui.separator();
                                                for (j, (key, name)) in
                                                    [(fx::THUNDER, "Thunder"), (fx::HUM, "Machinery hum"), (fx::NEON, "Neon buzz")]
                                                        .into_iter()
                                                        .enumerate()
                                                {
                                                    let mut on = layers[j];
                                                    if ui.checkbox(&mut on, name).changed() {
                                                        row_action = Some((i, RowAction::Layer(key, on)));
                                                    }
                                                }
                                            })
                                            .response
                                    } else if let Some(r) = role {
                                        let label = current.map_or(tr.source.label(), |c| alts[c].0.label.to_string());
                                        egui::ComboBox::from_id_salt(("alt", i))
                                            .width(w)
                                            .height(420.0)
                                            .selected_text(shown(&label))
                                            .show_ui(ui, |ui| {
                                                ui.label(egui::RichText::new(format!("{} sounds", r.label())).color(t.text_dim));
                                                for (j, (a, ok)) in alts.iter().enumerate() {
                                                    let kind = match &a.source {
                                                        Source::Synth(_) => "the built-in synth".to_string(),
                                                        Source::SoundFont { font, .. } => font.clone(),
                                                        Source::Kit(_) => "the city kit".to_string(),
                                                    };
                                                    let b = ui
                                                        .add_enabled(*ok, egui::Button::selectable(current == Some(j), a.label))
                                                        .on_hover_text(kind.clone())
                                                        .on_disabled_hover_text(format!(
                                                            "{kind} isn't loaded: tools/fetch_soundfonts.sh fetches it"
                                                        ));
                                                    if b.clicked() {
                                                        row_action = Some((i, RowAction::Alt(r, Box::new(a.clone()))));
                                                    }
                                                }
                                            })
                                            .response
                                    } else {
                                        ui.add(egui::Label::new(shown(&tr.source.label())).truncate())
                                    };
                                    touched |= resp.clicked();
                                    let resp = resp.on_hover_text("The sound: pick another for this part");
                                    if let Some(e) = &err {
                                        resp.on_hover_text(e);
                                    }
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    toggle(ui, "S", &mut tr.strip.solo, t.solo, "Solo");
                                    toggle(ui, "M", &mut tr.strip.mute, t.mute, "Mute");
                                    // level
                                    let (r, _) = ui.allocate_exact_size(vec2(4.0, 26.0), Sense::hover());
                                    ui.painter().rect_filled(r, 1.0, t.canvas);
                                    // on the mixer meter's scale and colours
                                    let db = if peak > 1e-6 { 20.0 * peak.log10() } else { -90.0 };
                                    let h = crate::mixer_ui::db_pos(db) * r.height();
                                    if h > 0.5 {
                                        let col = if db > -0.5 {
                                            t.alert
                                        } else if db > -6.0 {
                                            t.caution
                                        } else {
                                            rgb(tr.color)
                                        };
                                        ui.painter().rect_filled(Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.max), 1.0, col);
                                    }
                                });
                            });
                        })
                    })
                    .response;
                if resp.clicked() || touched {
                    self.sel = i;
                }
                if resp.double_clicked() && resp.interact_pointer_pos().is_some_and(|p| name_rect.contains(p)) {
                    self.renaming = Some(i);
                }
                if rename_done {
                    self.renaming = None;
                }
                let resp = if renaming { resp } else { resp.on_hover_text("Click to select; double-click the name to rename") };
                resp.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        action = Some((i, "rename"));
                        ui.close();
                    }
                    if ui.button("Duplicate").clicked() {
                        action = Some((i, "dup"));
                        ui.close();
                    }
                    if ui.button("Move up").clicked() {
                        action = Some((i, "up"));
                        ui.close();
                    }
                    if ui.button("Move down").clicked() {
                        action = Some((i, "down"));
                        ui.close();
                    }
                    if ui.button("Clear notes").clicked() {
                        action = Some((i, "clear"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Delete track").clicked() {
                        action = Some((i, "delete"));
                        ui.close();
                    }
                });
                ui.add_space(3.0);
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("The selected track is the one you play and record. Right-click a track for more.")
                    .font(t.font(10.5))
                    .color(t.text_dim),
            );
        });
        if let Some((i, act)) = row_action {
            let stopped = !self.playing();
            match act {
                RowAction::Alt(role, alt) => {
                    apply(&mut self.doc.project.tracks[i], role, &alt);
                    self.status = format!("{} now plays {}", self.doc.project.tracks[i].name, alt.label);
                    if stopped {
                        self.audition = Some((i, crate::sound::preview(&self.doc.project, i, role)));
                    }
                }
                RowAction::Rain(kind) => {
                    city::set_rain(&mut self.doc.project, i, kind);
                    if let (true, Some(k)) = (stopped, kind) {
                        self.audition = Some((i, vec![(0.05, 4.0, k.key(), 100)]));
                    }
                }
                RowAction::Layer(key, on) => {
                    city::set_layer(&mut self.doc.project, i, key, on);
                    if stopped && on {
                        self.audition = Some((i, vec![(0.05, 3.0, key, 100)]));
                    }
                }
            }
            self.sel = i;
        }
        if let Some((i, what)) = action {
            self.renaming = None;
            let tracks = &mut self.doc.project.tracks;
            match what {
                "dup" => {
                    let mut c = tracks[i].clone();
                    c.name = format!("{} copy", c.name);
                    tracks.insert(i + 1, c);
                    self.sel = i + 1;
                }
                "up" if i > 0 => {
                    tracks.swap(i, i - 1);
                    self.sel = i - 1;
                }
                "down" if i + 1 < tracks.len() => {
                    tracks.swap(i, i + 1);
                    self.sel = i + 1;
                }
                "rename" => {
                    self.sel = i;
                    self.renaming = Some(i);
                }
                "clear" => {
                    tracks[i].notes.clear();
                    tracks[i].ctrls.clear();
                }
                "delete" => {
                    tracks.remove(i);
                    self.sel = self.sel.min(tracks.len().saturating_sub(1));
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------ inspector

    pub(crate) fn inspector(&mut self, ui: &mut egui::Ui) {
        let tab = self.settings.tab;
        let segs = [
            Segment { label: "Sound", selected: tab == Tab::Sound, enabled: true, help: "What plays the selected track" },
            Segment {
                label: "Effects",
                selected: tab == Tab::Effects,
                enabled: true,
                help: "The shared reverb, echo, ensemble and tuning",
            },
            Segment { label: "Score", selected: tab == Tab::Score, enabled: true, help: "Tempo, metre, length and sections" },
            Segment {
                label: "Gallery",
                selected: tab == Tab::Gallery,
                enabled: true,
                help: "Photos timed to the music, for DaVinci Resolve",
            },
        ];
        ui.add_space(4.0);
        if let Some(i) = widgets::segmented(ui, &segs, 4) {
            self.settings.tab = [Tab::Sound, Tab::Effects, Tab::Score, Tab::Gallery][i];
        }
        widgets::divider(ui);
        egui::ScrollArea::vertical().id_salt(("inspector", self.settings.tab as u8)).auto_shrink([false, false]).show(ui, |ui| match self
            .settings
            .tab
        {
            Tab::Sound => self.sound_tab(ui),
            Tab::Effects => self.effects_tab(ui),
            Tab::Score => self.score_tab(ui),
            Tab::Gallery => self.gallery_tab(ui),
        });
    }

    fn effects_tab(&mut self, ui: &mut egui::Ui) {
        let fx = &mut self.doc.project.fx;
        widgets::section(ui, "reverb", "Reverb", true, |ui| {
            let segs: Vec<Segment> = Space::ALL
                .iter()
                .map(|s| Segment { label: s.label(), selected: fx.space == *s, enabled: true, help: s.description() })
                .collect();
            if let Some(i) = widgets::segmented(ui, &segs, 4) {
                fx.space = Space::ALL[i];
            }
            amount(ui, "size", "Size", &mut fx.size, 0.5, ("short", "long"), "How long the tail lasts");
            amount(ui, "tone", "Tone", &mut fx.tone, 0.5, ("dark", "bright"), "The tail's brightness");
            amount(ui, "motion", "Motion", &mut fx.motion, 0.5, ("still", "swirling"), "How much the tail moves");
            amount(
                ui,
                "pre",
                "Pre-delay",
                &mut fx.predelay,
                0.0,
                ("none", "¼ s"),
                "A gap before the reverb, so each note is heard clearly first",
            );
            amount(ui, "lowcut", "Low cut", &mut fx.low_cut, 0.5, ("40 Hz", "640 Hz"), "Keep low notes out of the reverb");
            amount(ui, "width", "Width", &mut fx.width, 0.5, ("mono", "wide"), "The reverb's stereo width");
            amount(
                ui,
                "duck",
                "Ducking",
                &mut fx.duck,
                0.0,
                ("off", "deep"),
                "The reverb dips while the music plays and blooms in the gaps",
            );
            if fx.space == Space::Shimmer {
                amount(ui, "shim", "Feedback", &mut fx.shimmer, 0.5, ("subtle", "intense"), "How much of the pitched tail is fed back");
                amount(
                    ui,
                    "shimd",
                    "Detune",
                    &mut fx.shimmer_detune,
                    0.0,
                    ("pure", "wide"),
                    "Split the shimmer into a sharp and a flat copy",
                );
                let segs: Vec<Segment> = Interval::ALL
                    .iter()
                    .map(|s| Segment { label: s.label(), selected: fx.shimmer_interval == *s, enabled: true, help: "" })
                    .collect();
                if let Some(i) = widgets::segmented(ui, &segs, 2) {
                    fx.shimmer_interval = Interval::ALL[i];
                }
            }
            widgets::inset(ui, |ui| {
                ui.checkbox(&mut fx.freeze, "Freeze the tail")
                    .on_hover_text("Hold the reverb as it is, ringing on, while new notes play dry over it");
            });
        });
        widgets::section(ui, "echo", "Echo", true, |ui| {
            let e = &mut fx.echo;
            let segs: Vec<Segment> = EchoTime::ALL
                .iter()
                .map(|s| Segment { label: s.label(), selected: e.time == *s, enabled: true, help: "Echo time, in step with the tempo" })
                .collect();
            if let Some(i) = widgets::segmented(ui, &segs, 5) {
                e.time = EchoTime::ALL[i];
            }
            amount(ui, "efb", "Repeats", &mut e.feedback, 0.45, ("one", "endless"), "How many times it echoes");
            amount(ui, "etone", "Tone", &mut e.tone, 0.4, ("dark", "bright"), "Each repeat darker, like tape");
            amount(ui, "ewow", "Wow", &mut e.wow, 0.2, ("steady", "warped"), "Tape wobble on the repeats");
            amount(ui, "erev", "Into reverb", &mut e.to_reverb, 0.5, ("dry", "wet"), "How much of the echo goes on into the reverb");
            widgets::inset(ui, |ui| {
                ui.checkbox(&mut e.ping_pong, "Ping-pong").on_hover_text("Repeats bounce left and right");
            });
            widgets::note(ui, "How much of each track goes into the echo and reverb is set on the mixer.", "");
        });
        widgets::section(ui, "ensemble", "Ensemble and tuning", true, |ui| {
            amount(
                ui,
                "ens",
                "Ensemble",
                &mut fx.ensemble,
                0.0,
                ("off", "lush"),
                "A string-machine chorus on the tracks that ask for it (Ens on the mixer)",
            );
            widgets::inset(ui, |ui| {
                egui::ComboBox::from_id_salt("tuning").selected_text(fx.tuning.label()).width(240.0).show_ui(ui, |ui| {
                    for tu in Tuning::ALL {
                        ui.selectable_value(&mut fx.tuning, tu, tu.label());
                    }
                });
            });
            ui.add_space(4.0);
            widgets::inset(ui, |ui| {
                if ui
                    .button("Back to the Noir sound")
                    .on_hover_text("The vast, dark hall, slow tape echo and ensemble the app starts with")
                    .clicked()
                {
                    *fx = dd_seq::project::noir_fx();
                }
            });
        });
    }

    fn score_tab(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        self.composer_ui(ui, &t);
        let p = &mut self.doc.project;
        widgets::section(ui, "time", "Tempo and length", true, |ui| {
            let mut bpm = p.bpm;
            let spec = SliderSpec {
                id: "bpm",
                label: "Tempo",
                help: "Beats per minute",
                min: 30.0,
                max: 200.0,
                default: 72.0,
                step: 0.5,
                decimals: 1,
                suffix: " bpm",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut bpm, true) {
                p.bpm = bpm;
            }
            widgets::inset(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Metre");
                    for m in [(4u32, 4u32), (3, 4), (6, 8), (5, 4), (7, 8)] {
                        ui.selectable_value(&mut p.meter, m, format!("{}/{}", m.0, m.1));
                    }
                });
            });
            let mut bars = p.length_bars as f64;
            let spec = SliderSpec {
                id: "bars",
                label: "Length",
                help: "The shortest the piece is; it grows to hold every note",
                min: 1.0,
                max: 400.0,
                default: 16.0,
                step: 1.0,
                decimals: 0,
                suffix: " bars",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut bars, true) {
                // the sections share the change
                p.stretch_to(bars as u32);
            }
            widgets::note(
                ui,
                format!("{} bars, {} plus a {:.0} s tail", p.bars(), timecode(p.music_secs()), dd_seq::project::TAIL_SECS),
                "",
            );
        });
        widgets::section(ui, "sections", "Sections", true, |ui| {
            widgets::note(
                ui,
                "The order of the score's sections. Add them in any order and as often as you like; set each one's length by dragging its end on the Sections strip above the roll. A composed piece is written again to follow them.",
                "",
            );
            let mut blocks = p.arrangement();
            let before = blocks.clone();
            // where each block came from, so the tracks' bar levels move with their sections
            let mut origin: Vec<Option<usize>> = (0..blocks.len()).map(Some).collect();
            let mut remove = None;
            let mut rows: Vec<Rect> = Vec::new();
            let dragging = egui::DragAndDrop::payload::<SectionDrag>(ui.ctx()).map(|d| *d);
            let pointer = ui.ctx().pointer_interact_pos();
            let list = ui.scope(|ui| {
                widgets::inset(ui, |ui| {
                    for (i, b) in blocks.iter_mut().enumerate() {
                        let row = ui.horizontal(|ui| {
                            let grip = ui
                                .add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("≡ {:>2}", i + 1))
                                            .font(egui::FontId::monospace(11.0))
                                            .color(t.text_dim),
                                    )
                                    .selectable(false)
                                    .sense(Sense::drag()),
                                )
                                .on_hover_cursor(egui::CursorIcon::Grab)
                                .on_hover_text("Drag to move this section");
                            grip.dnd_set_drag_payload(SectionDrag::Move(i));
                            let shown = b.part.map_or_else(
                                || if b.name.is_empty() { "(lead-in)".to_string() } else { b.name.clone() },
                                |p| p.label().to_string(),
                            );
                            egui::ComboBox::from_id_salt(("section-part", i)).width(110.0).selected_text(shown).show_ui(ui, |ui| {
                                for part in Part::ALL {
                                    if ui.selectable_label(b.part == Some(part), part.label()).on_hover_text(part.help()).clicked() {
                                        b.part = Some(part);
                                        b.name = part.label().into();
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new(format!("{} bar{}", b.bars, if b.bars == 1 { "" } else { "s" })).color(t.text_label),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("×").on_hover_text("Remove this section").clicked() {
                                    remove = Some(i);
                                }
                            });
                        });
                        rows.push(row.response.rect);
                    }
                    if let Some(i) = remove {
                        blocks.remove(i);
                        origin.remove(i);
                    }
                    ui.add_space(2.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Add");
                        for part in Part::ALL {
                            let b = ui
                                .add(egui::Button::new(format!("+ {}", part.label())).sense(Sense::click_and_drag()))
                                .on_hover_text(format!("{}. Click to add it at the end, or drag it into place.", part.help()));
                            b.dnd_set_drag_payload(SectionDrag::New(part));
                            if b.clicked() {
                                if blocks.is_empty() {
                                    // the first section spans the piece
                                    blocks.push(Block { name: part.label().into(), part: Some(part), bars: p.length_bars.max(1) });
                                } else {
                                    blocks.push(new_block(&blocks, part));
                                }
                                origin.push(None);
                            }
                        }
                    });
                })
            });
            // a section being dragged: show where it will land, and put it there on release
            if let (Some(what), Some(at)) = (dragging, pointer)
                && remove.is_none()
            {
                let area = list.response.rect.expand(8.0);
                let slot = rows.iter().filter(|r| r.center().y < at.y).count();
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                let label = match what {
                    SectionDrag::Move(i) => blocks.get(i).map_or(String::new(), |b| b.part.map_or(b.name.clone(), |p| p.label().into())),
                    SectionDrag::New(part) => format!("+ {}", part.label()),
                };
                let ghost = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, ui.scope_id().with("section-ghost")));
                ghost.text(at + vec2(14.0, 0.0), Align2::LEFT_CENTER, label, t.semibold(12.0), t.accent);
                if area.contains(at) {
                    let y = match (rows.get(slot), rows.last()) {
                        (Some(r), _) => r.top() - 1.0,
                        (None, Some(r)) => r.bottom() + 1.0,
                        (None, None) => list.response.rect.top() + 6.0,
                    };
                    ui.painter().hline(list.response.rect.x_range(), y, Stroke::new(2.0, t.accent));
                    if ui.input(|i| i.pointer.any_released()) {
                        egui::DragAndDrop::clear_payload(ui.ctx());
                        match what {
                            SectionDrag::Move(i) if i < blocks.len() => {
                                let to = if slot > i { slot - 1 } else { slot };
                                let (b, o) = (blocks.remove(i), origin.remove(i));
                                blocks.insert(to, b);
                                origin.insert(to, o);
                            }
                            SectionDrag::New(part) => {
                                let b = if blocks.is_empty() {
                                    Block { name: part.label().into(), part: Some(part), bars: p.length_bars.max(1) }
                                } else {
                                    new_block(&blocks, part)
                                };
                                blocks.insert(slot.min(blocks.len()), b);
                                origin.insert(slot.min(origin.len()), None);
                            }
                            _ => {}
                        }
                    }
                }
            }
            if blocks != before {
                if blocks.is_empty() {
                    p.sections.clear();
                } else {
                    p.rearrange(&blocks, &origin);
                }
            }
        });
        widgets::section(ui, "help", "Playing and recording", false, |ui| {
            let lines = [
                "Space plays and pauses, Enter stops (twice goes to the start), ⇧R records.",
                "Computer keyboard: A W S E D F T G Y H U J K O L P ; play an octave and a bit; Z X move it, C V set how hard.",
                "A MIDI keyboard plays the selected track; its wheel, pedal, bend and aftertouch are recorded too.",
                "The ribbon under the keys bends like a CS-80's; the Touch strip is aftertouch.",
                "Recording counts in a bar, keeps going past the end, and adds to the track (or replaces, if set).",
                "In the roll: click to add, drag to move (⌥ copies), drag the end to resize, double-click to delete, ⇧-drag to select. Q quantises.",
            ];
            for l in lines {
                widgets::note(ui, egui::RichText::new(l).color(t.text_label).text(), "");
            }
        });
    }

    fn composer_ui(&mut self, ui: &mut egui::Ui, t: &Tokens) {
        let untouched = self.doc.project.untouched();
        let (mut dice, mut again) = (None, false);
        let mut new_style = None;
        widgets::section(ui, "composer", "Composer", true, |ui| {
            let Some(r) = &mut self.doc.project.recipe else {
                widgets::note(ui, "This piece was written by hand or imported. 🎲 composes a new one at this tempo and length.", "");
                widgets::inset(ui, |ui| {
                    if ui.button(egui::RichText::new("🎲 New piece").color(t.accent)).clicked() {
                        dice = Some(true);
                    }
                });
                return;
            };
            let status = if untouched {
                "Composed: written again to fit whenever you change the tempo, metre, length or sections."
            } else {
                "Edited by hand: tempo and length changes keep your notes. Changing the settings below writes the piece again (undo brings yours back)."
            };
            widgets::note(ui, egui::RichText::new(status).color(if untouched { t.text_dim } else { t.caution }).text(), "");
            widgets::inset(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Style");
                    for s in Style::ALL {
                        let help = format!("{}. Choosing it sets {} bpm.", s.help(), s.tempo());
                        if ui.selectable_label(r.style == s, s.label()).on_hover_text(help).clicked() && r.style != s {
                            r.style = s;
                            new_style = Some(s);
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Key");
                    egui::ComboBox::from_id_salt("root").width(56.0).selected_text(KEY_NAMES[r.root as usize % 12]).show_ui(ui, |ui| {
                        for (i, k) in KEY_NAMES.iter().enumerate() {
                            ui.selectable_value(&mut r.root, i as u8, *k);
                        }
                    });
                    egui::ComboBox::from_id_salt("mode").width(130.0).selected_text(r.mode.label()).show_ui(ui, |ui| {
                        for m in Mode::ALL {
                            ui.selectable_value(&mut r.mode, m, m.label());
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label("Seed");
                    if ui.small_button("◀").on_hover_text("The previous seed").clicked() {
                        r.seed = r.seed.saturating_sub(1);
                    }
                    ui.add(egui::DragValue::new(&mut r.seed).range(0..=999_999))
                        .on_hover_text("The same seed, key, tempo and length always write the same piece");
                    if ui.small_button("▶").on_hover_text("The next seed").clicked() {
                        r.seed = (r.seed + 1).min(999_999);
                    }
                });
            });
            amount(ui, "density", "Density", &mut r.density, 0.5, ("sparse", "busy"), "How busy the bells, plucks and drums are");
            widgets::inset(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(egui::RichText::new("🎲 New piece").color(t.accent))
                        .on_hover_text("New notes (a new seed, key and mode) and a new arrangement of sections")
                        .clicked()
                    {
                        dice = Some(true);
                    }
                    if ui.button("New notes").on_hover_text("New notes (a new seed, key and mode) to the sections as they are").clicked() {
                        dice = Some(false);
                    }
                    if ui
                        .add_enabled(!untouched, egui::Button::new("Write again"))
                        .on_hover_text("Replace your edits with the piece as composed, at this tempo and length")
                        .clicked()
                    {
                        again = true;
                    }
                });
                ui.checkbox(&mut self.settings.keep_sounds, "New seeds keep my sounds and mix").on_hover_text(
                    "Off: each seed picks its own instruments and hall. On: tracks keep the sounds and mixer settings you chose",
                );
            });
        });
        if let Some(s) = new_style {
            // the style's own tempo; the piece is written again for both together
            self.doc.project.bpm = s.tempo();
            self.status = format!("{}: {}, at {} bpm", s.label(), s.help(), s.tempo());
        }
        if let Some(sections) = dice {
            self.new_piece(sections);
        } else if again && let Some(r) = self.doc.project.recipe {
            self.recompose(r, true);
        }
    }

    fn gallery_tab(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        let photos =
            self.doc.gallery.folder.as_ref().and_then(|d| dd_gallery::resolve::photos(d, self.doc.gallery.order).ok()).unwrap_or_default();
        let g = &mut self.doc.gallery;
        widgets::section(ui, "photos", "Photos", true, |ui| {
            widgets::inset(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Choose photo folder…").clicked()
                        && let Some(d) = rfd::FileDialog::new().pick_folder()
                    {
                        g.folder = Some(d);
                    }
                    if g.folder.is_some() && ui.small_button("×").on_hover_text("No photos").clicked() {
                        g.folder = None;
                    }
                });
                if let Some(d) = &g.folder {
                    ui.label(
                        egui::RichText::new(format!("{} photos in {}", photos.len(), d.display())).font(t.font(10.5)).color(t.text_dim),
                    );
                }
                ui.horizontal(|ui| {
                    ui.label("Order");
                    ui.selectable_value(&mut g.order, PhotoOrder::Name, "By name");
                    ui.selectable_value(&mut g.order, PhotoOrder::Date, "By date");
                });
                ui.horizontal(|ui| {
                    ui.label("Frame rate");
                    for f in FRAME_RATES {
                        ui.selectable_value(&mut g.fps, f, format!("{f}"));
                    }
                });
            });
        });
        widgets::section(ui, "timing", "Timing", true, |ui| {
            let segs = [
                Segment {
                    label: "Images for the track",
                    selected: g.fit == Fit::ImagesToTrack,
                    enabled: true,
                    help: "Count how many images the music holds",
                },
                Segment {
                    label: "Track for the images",
                    selected: g.fit == Fit::TrackToImages,
                    enabled: true,
                    help: "Make the piece long enough for a number of images",
                },
            ];
            if let Some(i) = widgets::segmented(ui, &segs, 2) {
                g.fit = if i == 0 { Fit::ImagesToTrack } else { Fit::TrackToImages };
                if g.fit == Fit::TrackToImages && !photos.is_empty() {
                    g.images = photos.len() as u32;
                }
            }
            if g.fit == Fit::TrackToImages {
                let mut n = g.images as f64;
                let spec = SliderSpec {
                    id: "images",
                    label: "Images",
                    help: "How many photos",
                    min: 1.0,
                    max: 300.0,
                    default: 30.0,
                    step: 1.0,
                    decimals: 0,
                    ..Default::default()
                };
                if widgets::slider(ui, &spec, &mut n, true) {
                    g.images = n as u32;
                }
            }
            let mut hold = g.hold as f64;
            let spec = SliderSpec {
                id: "hold",
                label: "On screen",
                help: "Each image's time, fade included; changes land on bar lines",
                min: MIN_HOLD,
                max: MAX_HOLD,
                default: 12.0,
                step: 0.5,
                decimals: 1,
                suffix: " s",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut hold, true) {
                g.hold = hold as f32;
            }
            let mut fade = g.fade as f64;
            let spec = SliderSpec {
                id: "fade",
                label: "Crossfade",
                help: "The dissolve between images",
                min: 0.0,
                max: MAX_FADE,
                default: 6.0,
                step: 0.5,
                decimals: 1,
                suffix: " s",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut fade, true) {
                g.fade = fade as f32;
            }
            let (mut intro, mut outro) = (g.intro_bars as f64, g.outro_bars as f64);
            let spec = SliderSpec {
                id: "intro",
                label: "Intro",
                help: "Bars of music before the first image (a title)",
                min: 0.0,
                max: 32.0,
                default: 2.0,
                step: 1.0,
                decimals: 0,
                suffix: " bars",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut intro, true) {
                g.intro_bars = intro as u32;
            }
            let spec = SliderSpec {
                id: "outro",
                label: "Outro",
                help: "Bars of music after the last image (credits)",
                min: 1.0,
                max: 32.0,
                default: 2.0,
                step: 1.0,
                decimals: 0,
                suffix: " bars",
                ..Default::default()
            };
            if widgets::slider(ui, &spec, &mut outro, true) {
                g.outro_bars = outro as u32;
            }
            widgets::inset(ui, |ui| {
                ui.checkbox(&mut g.follow_music, "Change on the music's accents")
                    .on_hover_text("Move each change to a section, a phrase or a part coming in, rather than an even count");
            });
        });
        let plan = self.doc.plan();
        if self.doc.gallery.fit == Fit::TrackToImages
            && plan.bars() != self.doc.project.length_bars
            && (plan.bars() >= self.doc.project.bars() || self.doc.project.untouched())
        {
            self.doc.project.length_bars = plan.bars();
        }
        widgets::section(ui, "plan", "Plan and export", true, |ui| {
            let (lo, hi) = plan.holds();
            let lines = [
                format!("{} images, {:.1}–{:.1} s each", plan.images, if plan.images > 0 { lo } else { 0.0 }, hi),
                format!("Intro {}  ·  images to {}  ·  outro {}", timecode(plan.intro), timecode(plan.end), timecode(plan.outro)),
                format!("Soundtrack {} ({} bars) with its tail", timecode(plan.audio()), plan.bars()),
            ];
            for l in lines {
                widgets::note(ui, l, "");
            }
            if !photos.is_empty() && photos.len() as u32 != plan.images {
                let msg = if (photos.len() as u32) > plan.images {
                    format!(
                        "The music holds {} images but there are {} photos: lengthen the piece, shorten the time on screen, or fit the track to the images.",
                        plan.images,
                        photos.len()
                    )
                } else {
                    format!("There are {} photos for {} images: the last photo stays up longer.", photos.len(), plan.images)
                };
                widgets::note(ui, egui::RichText::new(msg).color(t.caution).text(), "");
            }
            widgets::inset(ui, |ui| {
                ui.checkbox(&mut self.settings.stems_with_gallery, "Stems too")
                    .on_hover_text("A WAV of each track as well, for mixing on Resolve's Fairlight page");
                ui.horizontal(|ui| {
                    if ui
                        .button("Export for DaVinci Resolve…")
                        .on_hover_text("Soundtrack, FCPXML timeline, EDL markers and a cue list in one folder")
                        .clicked()
                    {
                        self.export_resolve();
                    }
                    if ui.button("Copy cue list").clicked() {
                        ui.ctx().copy_text(plan.cue_csv(self.doc.gallery.fade as f64));
                        self.status = "Cue list copied".into();
                    }
                });
            });
            widgets::note(
                ui,
                egui::RichText::new("In Resolve: File › Import › Timeline, choose the .fcpxml and let it import the media. Each photo sits at its change with a cross dissolve, over the soundtrack. The .edl holds the changes as markers (Timelines › Import › Timeline Markers from EDL).").color(t.text_dim).text(),
                "",
            );
        });
        // the image changes on the ruler
        let _ = plan;
    }

    // ------------------------------------------------------------ transport

    pub(crate) fn transport(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        ui.horizontal_centered(|ui| {
            ui.add_space(6.0);
            let playing = self.playing();
            let recording = self.recording();
            if widgets::icon_button(ui, Icon::Stop, false, "Stop (Enter); again goes to the start").clicked() {
                self.stop();
            }
            if widgets::icon_button(ui, if playing { Icon::Pause } else { Icon::Play }, playing, "Play or pause (Space)").clicked() {
                self.toggle_play();
            }
            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
            let red = t.alert;
            if recording || resp.hovered() {
                ui.painter().rect_filled(r, 4.0, if recording { red.gamma_multiply(0.35) } else { t.hover });
            }
            ui.painter().circle_filled(r.center(), 6.0, if recording { red } else { red.gamma_multiply(0.7) });
            if resp.on_hover_text("Record onto the selected track (⇧R)").clicked() {
                self.toggle_record();
            }
            let mut looping = self.doc.project.loop_bars.is_some();
            if widgets::icon_button(ui, Icon::Loop, looping, "Loop: ⇧-drag on the ruler to set the bars").clicked() {
                looping = !looping;
                self.doc.project.loop_bars = if looping {
                    let bar = (self.doc.project.secs_tick(self.position().max(0.0)) / self.doc.project.bar_ticks())
                        .min(self.doc.project.bars().saturating_sub(1));
                    Some((bar, (bar + 4).min(self.doc.project.bars())))
                } else {
                    None
                };
            }
            ui.separator();
            let p = &self.doc.project;
            let pos = self.position();
            let txt = if pos < 0.0 {
                format!("count-in {:.1}", -pos / p.tick_secs(p.beat_ticks()))
            } else {
                format!("{}   {}", p.position(p.secs_tick(pos)), timecode(pos))
            };
            ui.label(egui::RichText::new(txt).font(t.mono(15.0)).color(if recording { red } else { t.accent }));
            ui.separator();
            ui.add(egui::DragValue::new(&mut self.doc.project.bpm).range(30.0..=220.0).speed(0.2).suffix(" bpm").max_decimals(1))
                .on_hover_text("Tempo: drag or type");
            let mut metro = self.settings.metronome;
            if ui.selectable_label(metro, "Click").on_hover_text("Metronome while playing").clicked() {
                metro = !metro;
            }
            if metro != self.settings.metronome {
                self.settings.metronome = metro;
                self.send(Cmd::Metronome(metro));
            }
            ui.toggle_value(&mut self.settings.count_in, "Count-in").on_hover_text("A bar of clicks before recording starts");
            ui.separator();
            ui.label("Grid");
            egui::ComboBox::from_id_salt("snap").width(64.0).selected_text(self.settings.snap.label()).show_ui(ui, |ui| {
                for s in Snap::ALL {
                    ui.selectable_value(&mut self.settings.snap, s, s.label());
                }
            });
            ui.checkbox(&mut self.settings.quantize_takes, "Quantise takes").on_hover_text("Move recorded notes onto the grid");
            ui.checkbox(&mut self.settings.replace_takes, "Replace")
                .on_hover_text("A take replaces the notes it plays over, rather than adding to them");
            ui.separator();
            let port = self.midi.port.clone();
            egui::ComboBox::from_id_salt("midi-in")
                .width(150.0)
                .truncate()
                .selected_text(port.clone().unwrap_or_else(|| "No MIDI input".into()))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(port.is_none(), "No MIDI input").clicked() {
                        self.midi.connect(None, self.live.clone());
                        self.settings.midi_port = None;
                    }
                    for name in self.midi_ports.clone() {
                        if ui.selectable_label(port.as_deref() == Some(&name), &name).clicked() {
                            self.midi.connect(Some(&name), self.live.clone());
                            self.settings.midi_port = Some(name);
                        }
                    }
                    if ui.button("Look again").clicked() {
                        self.midi_ports = MidiIn::ports();
                    }
                })
                .response
                .on_hover_text(self.midi.error.clone().unwrap_or_else(|| "The MIDI keyboard that plays the selected track".into()));
            ui.toggle_value(&mut self.settings.qwerty, format!("⌨ C{}", self.octave - 1))
                .on_hover_text("Play with the computer keyboard (A W S E D F…); Z X change octave");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                if ui.button("Panic").on_hover_text("Silence every note at once").clicked() {
                    self.panic();
                }
                ui.toggle_value(&mut self.settings.show_mixer, "Mixer");
                ui.toggle_value(&mut self.settings.show_keys, "Keys");
                let mut vol = self.settings.volume;
                ui.spacing_mut().slider_width = 90.0;
                if ui
                    .add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false).trailing_fill(true))
                    .on_hover_text("Listening volume (not exported)")
                    .changed()
                {
                    self.settings.volume = vol;
                    self.send(Cmd::Volume(vol));
                }
                ui.label(egui::RichText::new("Volume").color(t.text_dim));
            });
        });
    }

    // ------------------------------------------------------------ performance

    pub(crate) fn performance(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        let full = ui.available_rect_before_wrap();
        ui.allocate_rect(full, Sense::hover());
        let ctl_w = 300.0;
        let keys = Rect::from_min_max(pos2(full.left() + 8.0, full.top() + 4.0), pos2(full.right() - ctl_w - 12.0, full.bottom() - 4.0));
        let ctl = Rect::from_min_max(pos2(keys.right() + 10.0, full.top() + 4.0), pos2(full.right() - 8.0, full.bottom() - 4.0));
        // the keyboard: three octaves from the computer keyboard's octave
        let held = self.live.held();
        let lo = ((self.octave - 1).max(0) * 12) as u8;
        let whites: Vec<u8> = (lo..lo.saturating_add(36).min(127)).filter(|k| !matches!(k % 12, 1 | 3 | 6 | 8 | 10)).collect();
        let ww = keys.width() / whites.len() as f32;
        let mut rects: Vec<(u8, Rect, bool)> = Vec::new();
        for (i, k) in whites.iter().enumerate() {
            rects.push((*k, Rect::from_min_size(pos2(keys.left() + i as f32 * ww, keys.top()), vec2(ww - 1.0, keys.height())), false));
            if !matches!(k % 12, 4 | 11) && *k < 127 {
                let x = keys.left() + (i as f32 + 1.0) * ww - ww * 0.3;
                rects.push((k + 1, Rect::from_min_size(pos2(x, keys.top()), vec2(ww * 0.6, keys.height() * 0.6)), true));
            }
        }
        let resp = ui.interact(keys, ui.scope_id().with("keys"), Sense::click_and_drag());
        let pointer = if resp.is_pointer_button_down_on() { resp.interact_pointer_pos() } else { None };
        let under = pointer.and_then(|pp| {
            rects
                .iter()
                .filter(|(_, r, _)| r.contains(pp))
                .max_by_key(|(_, _, b)| *b)
                .map(|(k, r, _)| (*k, ((pp.y - r.top()) / r.height()).clamp(0.2, 1.0)))
        });
        let id = ui.scope_id().with("keys-down");
        let was: Option<u8> = ui.data(|d| d.get_temp(id)).flatten();
        let now = under.map(|(k, _)| k);
        if was != now {
            if let Some(k) = was {
                self.play_live(Msg::Off(k));
            }
            if let Some((k, depth)) = under {
                // further down the key plays harder
                self.play_live(Msg::On(k, (40.0 + depth * 87.0) as u8));
            }
            ui.data_mut(|d| d.insert_temp(id, now));
        }
        let painter = ui.painter();
        let (accent, magenta) = (t.accent, t.secondary);
        for black in [false, true] {
            for (k, r, b) in &rects {
                if *b != black {
                    continue;
                }
                let down = held[*k as usize] > 0;
                let fill = match (down, black) {
                    (true, false) => accent,
                    (true, true) => magenta,
                    (false, false) => t.key_white,
                    (false, true) => t.key_black,
                };
                painter.rect_filled(*r, 2.0, fill);
                if k % 12 == 0 {
                    painter.text(
                        pos2(r.center().x, r.bottom() - 8.0),
                        Align2::CENTER_CENTER,
                        crate::roll::key_name(*k),
                        t.mono(9.0),
                        t.key_line,
                    );
                }
            }
        }
        resp.on_hover_text("Click to play; lower on the key plays harder");

        // ribbon, wheel, touch, sustain
        let row_h = (ctl.height() - 6.0) / 3.0;
        let ribbon = Rect::from_min_size(ctl.min, vec2(ctl.width(), row_h));
        let bend_resp = ui.interact(ribbon, ui.scope_id().with("ribbon"), Sense::click_and_drag());
        let bend = match bend_resp.interact_pointer_pos().filter(|_| bend_resp.is_pointer_button_down_on()) {
            Some(pp) => ((pp.x - ribbon.center().x) / (ribbon.width() / 2.0)).clamp(-1.0, 1.0),
            None => 0.0,
        };
        if bend != self.perf.bend {
            self.perf.bend = bend;
            self.play_live(Msg::Bend((bend * 8191.0) as i16));
        }
        let p = ui.painter();
        p.rect_filled(ribbon, 3.0, t.field);
        p.rect_stroke(ribbon, 3.0, Stroke::new(1.0, t.button_border), StrokeKind::Inside);
        p.vline(ribbon.center().x, ribbon.y_range().shrink(3.0), Stroke::new(1.0, t.track));
        let bx = ribbon.center().x + bend * ribbon.width() / 2.0;
        p.rect_filled(Rect::from_center_size(pos2(bx, ribbon.center().y), vec2(4.0, row_h - 6.0)), 1.0, magenta);
        p.text(pos2(ribbon.left() + 6.0, ribbon.center().y), Align2::LEFT_CENTER, "Ribbon", t.font(10.0), t.text_dim);
        bend_resp.on_hover_text(
            "Slide to bend the pitch, like a CS-80 ribbon; springs back when let go. Range under Sound › Glide, sag and ribbon.",
        );

        let row2 = Rect::from_min_size(pos2(ctl.left(), ribbon.bottom() + 3.0), vec2(ctl.width(), row_h));
        let half = (row2.width() - 6.0) / 2.0;
        let wheel_r = Rect::from_min_size(row2.min, vec2(half, row_h));
        let touch_r = Rect::from_min_size(pos2(row2.left() + half + 6.0, row2.top()), vec2(half, row_h));
        let wresp = ui.interact(wheel_r, ui.scope_id().with("wheel"), Sense::click_and_drag());
        if let Some(pp) = wresp.interact_pointer_pos().filter(|_| wresp.is_pointer_button_down_on()) {
            let w = ((pp.x - wheel_r.left()) / wheel_r.width()).clamp(0.0, 1.0);
            if (w - self.perf.wheel).abs() > 0.004 {
                self.perf.wheel = w;
                self.play_live(Msg::Cc(1, (w * 127.0) as u8));
            }
        }
        if wresp.double_clicked() {
            self.perf.wheel = 0.0;
            self.play_live(Msg::Cc(1, 0));
        }
        let tresp = ui.interact(touch_r, ui.scope_id().with("touch"), Sense::click_and_drag());
        let touch = match tresp.interact_pointer_pos().filter(|_| tresp.is_pointer_button_down_on()) {
            Some(pp) => ((pp.x - touch_r.left()) / touch_r.width()).clamp(0.0, 1.0),
            None => 0.0,
        };
        if (touch - self.perf.touch).abs() > 0.004 || (touch == 0.0 && self.perf.touch != 0.0) {
            self.perf.touch = touch;
            self.play_live(Msg::Pressure((touch * 127.0) as u8));
        }
        let p = ui.painter();
        for (r, v, label, col) in [(wheel_r, self.perf.wheel, "Mod wheel", accent), (touch_r, self.perf.touch, "Touch", magenta)] {
            p.rect_filled(r, 3.0, t.field);
            p.rect_filled(Rect::from_min_size(r.min, vec2(r.width() * v, r.height())), 3.0, col.gamma_multiply(0.45));
            p.rect_stroke(r, 3.0, Stroke::new(1.0, t.button_border), StrokeKind::Inside);
            p.text(pos2(r.left() + 6.0, r.center().y), Align2::LEFT_CENTER, label, t.font(10.0), t.text_label);
        }
        wresp.on_hover_text("Modulation wheel: vibrato. Stays where you leave it; double-click for none.");
        tresp.on_hover_text("Touch: aftertouch, as if pressing harder on the held keys. Springs back.");
        let row3 = Rect::from_min_size(pos2(ctl.left(), row2.bottom() + 3.0), vec2(ctl.width(), row_h));
        let sresp = ui.interact(row3, ui.scope_id().with("sustain"), Sense::click());
        if sresp.clicked() {
            self.perf.sustain = !self.perf.sustain;
            self.play_live(Msg::Cc(64, if self.perf.sustain { 127 } else { 0 }));
        }
        let p = ui.painter();
        p.rect_filled(row3, 3.0, if self.perf.sustain { accent.gamma_multiply(0.45) } else { t.field });
        p.rect_stroke(row3, 3.0, Stroke::new(1.0, t.button_border), StrokeKind::Inside);
        p.text(
            row3.center(),
            Align2::CENTER_CENTER,
            if self.perf.sustain { "Sustain pedal: down" } else { "Sustain pedal: up" },
            t.font(10.0),
            t.text_label,
        );
        sresp.on_hover_text("Click to hold or release the sustain pedal");
    }

    // ------------------------------------------------------------ about

    pub(crate) fn about_window(&mut self, ctx: &egui::Context) {
        let mut open = self.about_open;
        egui::Window::new(format!("About {}", crate::theme::APP_NAME)).open(&mut open).resizable(true).default_width(520.0).show(ctx, |ui| {
            let t = Tokens::get(ui.ctx());
            ui.label(egui::RichText::new(crate::theme::WORDMARK).font(t.label(22.0)).color(t.accent));
            ui.label(egui::RichText::new("A neon-noir synthesiser and sequencer for scoring in MIDI.").font(t.book(17.0)).color(t.text));
            ui.label(egui::RichText::new(format!("VERSION {}", env!("CARGO_PKG_VERSION"))).font(t.mono(11.5)).color(t.text_dim));
            ui.add_space(6.0);
            ui.label("The code is under the MIT License. The bundled SoundFonts have their own licences, all of which allow music made with them to be used commercially:");
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for f in &self.fonts.fonts {
                    ui.label(egui::RichText::new(&f.name).font(t.semibold(12.0)));
                    ui.label(egui::RichText::new(crate::sound::credit(&f.name)).color(t.text_dim));
                }
                ui.add_space(6.0);
                for (what, who) in [
                    ("Reverb, echo, ensemble, city kit, slideshow timing, Resolve export", "from Ambient MIDI Generator (catskinroyale) — MIT"),
                    ("Interface theme and widgets", "adapted from BW Darkroom and LightCraft — MIT"),
                    ("Rain recordings", "softwalls, BeeProductive, launemax, richwise, kyles, bastipictures, conleec (Freesound), kvgarlic and Extemporalist (Wikimedia Commons) — CC0"),
                    ("Composer's random streams", "from Ambient MIDI Generator — MIT"),
                    (
                        "Typefaces",
                        "Michroma (Vernon Adams), Share Tech Mono (Carrois Type Design), VT323 (Peter Hull), Cutive Mono (Vernon Adams), Sorts Mill Goudy (Barry Schwartz), Jost (Indestructible Type), Inter (Rasmus Andersson) — SIL Open Font License 1.1",
                    ),
                    ("SoundFont playback", "rustysynth — MIT"),
                    ("MIDI files / MIDI input", "midly — Unlicense; midir — MIT"),
                    ("Audio / WAV / interface", "cpal, hound, egui and eframe — Apache-2.0 or MIT"),
                ] {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(what).font(t.semibold(11.5)));
                        ui.label(egui::RichText::new(who).color(t.text_dim));
                    });
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("{} is an independent project. The factory sounds are original, written in the spirit of early-1980s polysynth film scores; no film, author, composer or instrument maker is affiliated with it.", crate::theme::APP_NAME))
                        .font(t.typewriter(12.0))
                        .color(t.text_dim),
                );
            });
        });
        self.about_open = open;
    }
}

/// A section being dragged in the Score tab: one of the arrangement's, or a new one.
#[derive(Clone, Copy)]
enum SectionDrag {
    Move(usize),
    New(Part),
}

/// A new section of a part: as long as the last of its kind, or 8 bars.
fn new_block(blocks: &[Block], part: Part) -> Block {
    let bars = blocks.iter().rev().find(|b| b.part == Some(part)).map_or(8, |b| b.bars);
    Block { name: part.label().into(), part: Some(part), bars }
}
