//! The Sound tab: what plays the selected track. A synth patch editor, a SoundFont
//! preset browser, or the city kit's key map.

use dd_dsp::patch::{Category, Env, Layer, LfoShape, Patch, factory};
use dd_dsp::sampler::KitKind;
use dd_seq::demo::GM_FONT;
use dd_seq::palette::{Alt, Role};
use dd_seq::project::Source;
use dd_seq::timeline::Msg;
use eframe::egui;

use crate::app::App;
use crate::roll::key_name;
use crate::theme::Tokens;
use crate::widgets::{self, Segment};

/// A short phrase that shows off a sound in its role, in the track's register and the
/// piece's key: (start, length) in seconds, key, velocity.
pub(crate) fn preview(p: &dd_seq::project::Project, track: usize, role: Role) -> dd_seq::engine::Preview {
    use dd_dsp::kit::drum;
    let t = &p.tracks[track];
    let default = match role {
        Role::Bass => 40,
        Role::Drone => 45,
        Role::Lead | Role::Answer => 67,
        Role::Bells => 76,
        Role::Pluck => 69,
        _ => 60,
    };
    let mut keys: Vec<u8> = t.notes.iter().map(|n| n.key).collect();
    keys.sort_unstable();
    let centre = keys.get(keys.len() / 2).copied().unwrap_or(default) as i32;
    // on the piece's tonic, nearest the centre
    let root = p.recipe.map_or(0, |r| r.root as i32);
    let k = (centre - 6..=centre + 5).find(|k| k.rem_euclid(12) == root).unwrap_or(centre).clamp(24, 100) as u8;
    let seq =
        |steps: &[(f64, f64, i32)]| steps.iter().map(|&(at, len, d)| (at + 0.05, len, (k as i32 + d).clamp(0, 127) as u8, 100)).collect();
    match role {
        Role::Brass | Role::Pad | Role::Strings | Role::Keys => seq(&[(0.0, 2.0, 0), (0.0, 2.0, 3), (0.0, 2.0, 7), (0.0, 2.0, 10)]),
        Role::Drone => seq(&[(0.0, 4.0, 0), (0.0, 4.0, 7)]),
        Role::Lead | Role::Answer => seq(&[(0.0, 0.45, 0), (0.5, 0.45, 3), (1.0, 0.45, 7), (1.5, 1.0, 5)]),
        Role::Bass => seq(&[(0.0, 0.4, 0), (0.5, 0.4, 0), (1.0, 0.4, 7), (1.5, 0.6, 5)]),
        Role::Bells | Role::Pluck => seq(&[(0.0, 0.3, 0), (0.25, 0.3, 3), (0.5, 0.3, 7), (0.75, 0.3, 12), (1.0, 0.6, 7)]),
        Role::Drums => [(0.0, drum::KICK), (0.5, drum::GATED_SNARE), (1.0, drum::KICK), (1.5, drum::GATED_SNARE)]
            .iter()
            .flat_map(|&(at, key)| [(at + 0.05, 0.2, key, 110), (at + 0.3, 0.1, drum::HAT, 70)])
            .collect(),
        Role::City => vec![(0.05, 3.0, dd_dsp::rain::keys::CITY, 110)],
    }
}

/// Who made a bundled SoundFont, and its licence, in a line.
/// The patch display: a little green-screen readout of the patch, in VT323.
fn crt_display(ui: &mut egui::Ui, patch: &Patch, t: &Tokens) {
    widgets::inset(ui, |ui| {
        let (r, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 58.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(r, 3.0, t.canvas);
        p.rect_stroke(r, 3.0, egui::Stroke::new(1.0, t.accent.gamma_multiply(0.35)), egui::StrokeKind::Inside);
        // faint scan lines
        let mut y = r.top() + 2.0;
        while y < r.bottom() {
            p.hline(r.x_range(), y, egui::Stroke::new(1.0, t.accent.gamma_multiply(0.04)));
            y += 3.0;
        }
        let number = factory().iter().position(|f| f.name == patch.name).map_or("--".to_string(), |i| format!("{:02}", i + 1));
        let top = format!("PATCH {number} / {}", patch.name.to_uppercase());
        let layers = patch.layers.iter().filter(|l| l.on).count();
        let mut bottom = format!("{} / {} LAYER{}", patch.category.label().to_uppercase(), layers, if layers == 1 { "" } else { "S" });
        if patch.mono {
            bottom += " / MONO";
        }
        if patch.glide > 0.0 {
            bottom += &format!(" / GLIDE {:.2}", patch.glide);
        }
        let glow = t.accent;
        p.text(egui::pos2(r.left() + 10.0, r.top() + 18.0), egui::Align2::LEFT_CENTER, top, t.crt(22.0), glow);
        p.text(egui::pos2(r.left() + 10.0, r.top() + 41.0), egui::Align2::LEFT_CENTER, bottom, t.crt(17.0), glow.gamma_multiply(0.7));
    });
    ui.add_space(6.0);
}

pub fn credit(font: &str) -> &'static str {
    if font.starts_with("GeneralUser") {
        "GeneralUser GS by S. Christian Collins: free for any music, private or commercial (licence in soundfonts/)"
    } else if font.starts_with("FreePats") {
        "FreePats (freepats.zenvoid.org), from Versilian VCSL samples or free synths: CC0, public domain"
    } else if font.starts_with("Ambient-Electronic") {
        "Synthesised by Ambient MIDI Generator, no recorded samples: MIT"
    } else {
        "Loaded by you: check its licence before you publish music made with it"
    }
}

use widgets::Knob;

/// An envelope's four stages as dials.
fn env_knobs<'a>(e: &'a mut Env, d: Env, what: &str) -> Vec<Knob<'a>> {
    let Env { attack, decay, sustain, release } = e;
    vec![
        Knob::new("Attack", attack, (0.001, 12.0), d.attack, 3, " s", &format!("How long the {what} takes to rise")).log(),
        Knob::new("Decay", decay, (0.001, 12.0), d.decay, 3, " s", &format!("How long the {what} takes to fall to the sustain level"))
            .log(),
        Knob::amount("Sustain", sustain, d.sustain, ("none", "full"), &format!("Where the {what} rests while the key is held")),
        Knob::new(
            "Release",
            release,
            (0.001, 12.0),
            d.release,
            3,
            " s",
            &format!("How long the {what} takes to die away after the key is let go"),
        )
        .log(),
    ]
}

/// One layer as a strip: its lamp switches it on, then its level, pitch, oscillators,
/// filters and envelopes as dials.
fn layer_strip(ui: &mut egui::Ui, t: &Tokens, l: &mut Layer, n: usize) {
    let d = Layer::default();
    let (title, help) = if n == 0 {
        ("Layer I", "The first voice of every note")
    } else {
        ("Layer II", "The second voice, played together with the first")
    };
    let on = l.on;
    let mut plays = l.on;
    crate::panels::rack_strip(ui, t, title, help, Some(&mut plays), |ui| {
        let mut k = vec![
            Knob::new("Level", &mut l.level, (0.0, 1.0), d.level, 2, "", "How loud this layer is against the other").enabled(on),
            Knob::new("Pan", &mut l.pan, (-1.0, 1.0), 0.0, 2, "", "Place the layer left or right").enabled(on),
            Knob::new(
                "Detune",
                &mut l.detune,
                (-50.0, 50.0),
                0.0,
                1,
                " ct",
                "Pull this layer off pitch against the other, for a thick, beating sound",
            )
            .enabled(on),
        ];
        widgets::knob_rows(ui, &mut k);
        widgets::strip_heading(ui, "Octave", on);
        for o in -2i8..=2 {
            let label = match o {
                -2 => "32'",
                -1 => "16'",
                0 => "8'",
                1 => "4'",
                _ => "2'",
            };
            if widgets::lamp_row(ui, label, l.octave == o, on).on_hover_text(format!("{o:+} octaves")).clicked() {
                l.octave = o;
            }
        }
        crate::panels::strip_rule(ui, t);
        widgets::strip_heading(ui, "Oscillators", on);
        let mut k = vec![
            Knob::amount("Saw", &mut l.saw, d.saw, ("off", "full"), "Bright and buzzy: brass and strings"),
            Knob::amount("Pulse", &mut l.pulse, 0.0, ("off", "full"), "Hollow to nasal, by its width"),
            Knob::new("Width", &mut l.width, (0.05, 0.5), 0.5, 2, "", "Pulse width: 0.5 is a square, narrower is thinner"),
            Knob::amount("PWM", &mut l.pwm, 0.0, ("still", "sweeping"), "The LFO sweeps the pulse width, for a chorused shimmer"),
            Knob::amount("Sine", &mut l.sine, 0.0, ("off", "full"), "Pure and round: bells and electric piano with FM"),
            Knob::amount("Noise", &mut l.noise, 0.0, ("off", "full"), "Breath, wind, rain"),
            Knob::new(
                "FM ratio",
                &mut l.fm_ratio,
                (0.5, 16.0),
                1.0,
                2,
                "×",
                "The modulator's pitch against the note: whole numbers sound harmonic, others like bells",
            )
            .log(),
            Knob::amount("FM depth", &mut l.fm_depth, 0.0, ("pure", "metallic"), "How hard the modulator bends the sine"),
            Knob::new(
                "FM decay",
                &mut l.fm_decay,
                (0.0, 8.0),
                0.0,
                2,
                " s",
                "How fast the FM brightness dies away (0 = held): the strike of a bell or tine",
            ),
            Knob::amount("Drive", &mut l.drive, 0.0, ("clean", "driven"), "Saturate before the filters"),
        ];
        k.iter_mut().for_each(|x| x.enabled = on);
        widgets::knob_rows(ui, &mut k);
        crate::panels::strip_rule(ui, t);
        widgets::strip_heading(ui, "Filters", on);
        let mut k = vec![
            Knob::new("High-pass", &mut l.hpf, (20.0, 6000.0), 20.0, 0, " Hz", "Thin out the bottom (12 dB an octave)").log(),
            Knob::amount("HP reso", &mut l.hpf_res, 0.0, ("flat", "peaky"), "A peak at the high-pass cutoff"),
            Knob::new(
                "Low-pass",
                &mut l.lpf,
                (40.0, 18000.0),
                d.lpf,
                0,
                " Hz",
                "Darken the top (12 dB an octave): where the filter starts before its envelope opens it",
            )
            .log(),
            Knob::amount("LP reso", &mut l.lpf_res, d.lpf_res, ("flat", "singing"), "A peak at the low-pass cutoff"),
            Knob::new(
                "Env amt",
                &mut l.env_amount,
                (-4.0, 6.0),
                d.env_amount,
                2,
                " oct",
                "How far the filter envelope opens (or closes) the low-pass",
            ),
            Knob::amount("Key track", &mut l.key_track, d.key_track, ("none", "full"), "Higher notes open the filter more"),
        ];
        k.iter_mut().for_each(|x| x.enabled = on);
        widgets::knob_rows(ui, &mut k);
        crate::panels::strip_rule(ui, t);
        widgets::strip_heading(ui, "Filter envelope", on);
        let mut k = env_knobs(&mut l.filter_env, d.filter_env, "filter");
        k.iter_mut().for_each(|x| x.enabled = on);
        widgets::knob_rows(ui, &mut k);
        crate::panels::strip_rule(ui, t);
        widgets::strip_heading(ui, "Amp envelope", on);
        let mut k = env_knobs(&mut l.amp_env, d.amp_env, "level");
        k.iter_mut().for_each(|x| x.enabled = on);
        widgets::knob_rows(ui, &mut k);
        crate::panels::strip_rule(ui, t);
        widgets::strip_heading(ui, "Velocity", on);
        let mut k = vec![
            Knob::new("Filter", &mut l.vel_to_filter, (0.0, 3.0), d.vel_to_filter, 2, " oct", "Initial touch: harder notes play brighter"),
            Knob::amount("Level", &mut l.vel_to_amp, d.vel_to_amp, ("even", "dynamic"), "How much harder notes play louder"),
        ];
        k.iter_mut().for_each(|x| x.enabled = on);
        widgets::knob_rows(ui, &mut k);
    });
    l.on = plays;
}

fn patch_ui(ui: &mut egui::Ui, p: &mut Patch) {
    let t = Tokens::get(ui.ctx());
    let d = Patch::default();
    ui.add_space(8.0);
    widgets::inset(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let [l1, l2] = &mut p.layers;
            layer_strip(ui, &t, l1, 0);
            layer_strip(ui, &t, l2, 1);
            // glide, the LFO and touch stacked in the third column, beside the layers
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                crate::panels::fx_strip(ui, &t, "Glide", "Glide, sag and the ribbon: how the patch moves between notes", |ui| {
                    if widgets::lamp_row(ui, "Mono legato", p.mono, true)
                        .on_hover_text("One note at a time; a note played while one is held glides there without restarting")
                        .clicked()
                    {
                        p.mono = !p.mono;
                    }
                    ui.add_space(2.0);
                    let mut k = vec![
                        Knob::new("Glide", &mut p.glide, (0.0, 2.0), 0.0, 2, " s", "Portamento: slide from one note to the next"),
                        Knob::new(
                            "Sag",
                            &mut p.sag,
                            (-200.0, 50.0),
                            0.0,
                            0,
                            " ct",
                            "How far the pitch falls as a released note dies away: the drooping brass of an 80s polysynth",
                        ),
                        Knob::new("Sag time", &mut p.sag_time, (0.05, 5.0), d.sag_time, 2, " s", "How long the sag takes").log(),
                        Knob::new(
                            "Ribbon",
                            &mut p.bend_range,
                            (0.0, 24.0),
                            d.bend_range,
                            0,
                            " st",
                            "Pitch bend range, for the ribbon and the bend wheel",
                        ),
                        Knob::new(
                            "Drift",
                            &mut p.drift,
                            (0.0, 30.0),
                            d.drift,
                            1,
                            " ct",
                            "Each voice wanders off pitch slowly, like old analogue oscillators",
                        ),
                        Knob::amount("Spread", &mut p.spread, d.spread, ("centre", "wide"), "Spread the voices across the stereo field"),
                        Knob::amount("Ring mod", &mut p.ring, 0.0, ("off", "full"), "Multiply layer II by layer I: metallic, clangorous"),
                        Knob::new("Gain", &mut p.gain_db, (-24.0, 12.0), 0.0, 1, " dB", "The patch's level"),
                    ];
                    widgets::knob_rows(ui, &mut k);
                });
                crate::panels::fx_strip(ui, &t, "LFO", "Vibrato and the LFO: its shape, then where it goes", |ui| {
                    for s in LfoShape::ALL {
                        if widgets::lamp_row(ui, s.label(), p.lfo_shape == s, true).on_hover_text("The LFO's shape").clicked() {
                            p.lfo_shape = s;
                        }
                    }
                    crate::panels::strip_rule(ui, &t);
                    let mut k = vec![
                        Knob::new("Rate", &mut p.lfo_rate, (0.02, 20.0), d.lfo_rate, 2, " Hz", "How fast the LFO swings").log(),
                        Knob::new(
                            "Delay",
                            &mut p.lfo_delay,
                            (0.0, 4.0),
                            d.lfo_delay,
                            2,
                            " s",
                            "Vibrato fades in this long after each note starts, as a player would",
                        ),
                        Knob::new("Vibrato", &mut p.vibrato, (0.0, 400.0), 0.0, 0, " ct", "Pitch wobble always on"),
                        Knob::new(
                            "To filter",
                            &mut p.lfo_to_filter,
                            (0.0, 3.0),
                            0.0,
                            2,
                            " oct",
                            "Sweep the low-pass: slow for movement, fast for growl",
                        ),
                        Knob::amount("Tremolo", &mut p.tremolo, 0.0, ("off", "deep"), "The LFO on the level"),
                        Knob::new(
                            "Wheel",
                            &mut p.wheel_vibrato,
                            (0.0, 200.0),
                            d.wheel_vibrato,
                            0,
                            " ct",
                            "Vibrato added at full modulation wheel",
                        ),
                    ];
                    widgets::knob_rows(ui, &mut k);
                });
                crate::panels::fx_strip(
                    ui,
                    &t,
                    "Touch",
                    "Pressing harder on a held key (aftertouch, or the Touch strip below) changes the sound, as on the CS-80",
                    |ui| {
                        let mut k = vec![
                            Knob::new("Filter", &mut p.touch_filter, (0.0, 4.0), d.touch_filter, 2, " oct", "Brightens as you press"),
                            Knob::new("Vibrato", &mut p.touch_vibrato, (0.0, 100.0), d.touch_vibrato, 0, " ct", "Vibrato as you press"),
                            Knob::new("Bend", &mut p.touch_bend, (-200.0, 200.0), 0.0, 0, " ct", "Bends the pitch as you press"),
                        ];
                        widgets::knob_rows(ui, &mut k);
                    },
                );
            });
        });
    });
}

impl App {
    /// Whether an alternative's sound is to hand.
    pub(crate) fn alt_available(&self, a: &Alt) -> bool {
        match &a.source {
            Source::SoundFont { font, bank, program, .. } => self.fonts.get(font).is_some_and(|f| f.find(*bank, *program).is_some()),
            _ => true,
        }
    }

    pub(crate) fn sound_tab(&mut self, ui: &mut egui::Ui) {
        let t = Tokens::get(ui.ctx());
        let sel = self.sel;
        let Some(track) = self.doc.project.tracks.get(sel) else {
            widgets::note(ui, "No track selected.", "");
            return;
        };
        // The City and drum tracks' notes are key maps (rain on C8 to F#8, a kick on C2…),
        // not melodies, so they keep their kits: played by the synth they'd squeal.
        // Their sounds are chosen in their dropdowns in the track list.
        let kit_part = matches!(track.source, Source::Kit(_)) || matches!(Role::of(track), Some(Role::City | Role::Drums));
        ui.add_space(4.0);
        if kit_part {
            let what = if matches!(track.source, Source::Kit(KitKind::CityFx)) || Role::of(track) == Some(Role::City) {
                "The City track's rain, thunder, machinery hum and neon buzz are chosen in its dropdown in the track list."
            } else {
                "A drum track's notes are a drum map, so it keeps a drum kit: choose the kit in its dropdown in the track list."
            };
            widgets::note(ui, what, "");
            // one switched to the synth or a melodic preset before: put its kit back
            let city = Role::of(track) == Some(Role::City);
            let melodic = match &track.source {
                Source::Kit(_) => false,
                Source::SoundFont { bank, .. } => *bank != 128,
                Source::Synth(_) => true,
            };
            if melodic {
                widgets::inset(ui, |ui| {
                    if ui.button(if city { "Put the city kit back" } else { "Put the drum kit back" }).clicked() {
                        self.doc.project.tracks[sel].source = Source::Kit(if city { KitKind::CityFx } else { KitKind::Drums });
                    }
                });
            }
            ui.add_space(4.0);
        } else {
            let kind = match &track.source {
                Source::Synth(_) => 0,
                _ => 1,
            };
            let segs = [
                Segment { label: "Synth", selected: kind == 0, enabled: true, help: "The built-in two-layer polysynth" },
                Segment { label: "SoundFont", selected: kind == 1, enabled: true, help: "A sampled instrument from a SoundFont" },
            ];
            if let Some(i) = widgets::segmented(ui, &segs, 2)
                && i != kind
            {
                self.doc.project.tracks[sel].source = match i {
                    0 => Source::Synth(dd_dsp::patch::by_name("Neon Brass")),
                    _ => Source::SoundFont { font: GM_FONT.into(), bank: 0, program: 0, name: "Grand Piano".into() },
                };
            }
        }
        if let Some(Some(err)) = self.inst_errors.get(sel) {
            widgets::note(ui, egui::RichText::new(err).color(t.caution).text(), "");
        }
        let hold_id = egui::Id::unique("dd-kit-hold");
        let mut held: Option<(usize, u8)> = ui.data(|d| d.get_temp(hold_id)).flatten();
        if held.is_some_and(|(tr, _)| tr != sel || !matches!(self.doc.project.tracks[sel].source, Source::Kit(_))) {
            // the held sound belonged to another track
            held = None;
            ui.data_mut(|d| d.insert_temp::<Option<(usize, u8)>>(hold_id, None));
        }
        let mut sustained = false;
        let fonts = self.fonts.clone();
        let mut audition: Option<u8> = None;
        let mut load_font = false;
        match &mut self.doc.project.tracks[sel].source {
            Source::Synth(patch) => {
                crt_display(ui, patch, &t);
                widgets::inset(ui, |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("patch").width(190.0).height(520.0).selected_text(patch.name.clone()).show_ui(
                            ui,
                            |ui| {
                                for cat in Category::ALL {
                                    let list: Vec<Patch> = factory().into_iter().filter(|p| p.category == cat).collect();
                                    if list.is_empty() {
                                        continue;
                                    }
                                    ui.label(egui::RichText::new(cat.label()).font(t.semibold(11.0)).color(t.text_dim));
                                    for p in list {
                                        if ui.selectable_label(patch.name == p.name, format!("   {}", p.name)).clicked() {
                                            *patch = p;
                                        }
                                    }
                                }
                            },
                        );
                        if ui.button("Save…").on_hover_text("Save this patch as a file").clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("Kibble-481 patch", &["ddpatch"])
                                .set_file_name(format!("{}.ddpatch", patch.name))
                                .save_file()
                        {
                            let _ = std::fs::write(path, serde_json::to_string_pretty(&*patch).unwrap_or_default());
                        }
                        if ui.button("Load…").on_hover_text("Load a saved patch").clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("Kibble-481 patch", &["ddpatch"]).pick_file()
                            && let Ok(p) = std::fs::read_to_string(path)
                                .map_err(|e| e.to_string())
                                .and_then(|s| serde_json::from_str::<Patch>(&s).map_err(|e| e.to_string()))
                        {
                            *patch = p;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Name");
                        ui.add(egui::TextEdit::singleline(&mut patch.name).desired_width(200.0));
                    });
                });
                ui.add_space(6.0);
                patch_ui(ui, patch);
            }
            Source::SoundFont { font, bank, program, name } => {
                widgets::inset(ui, |ui| {
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("font").width(220.0).selected_text(font.clone()).show_ui(ui, |ui| {
                            for f in &fonts.fonts {
                                if ui.selectable_label(*font == f.name, &f.name).clicked() && *font != f.name {
                                    *font = f.name.clone();
                                    if let Some(p) = f.presets.first() {
                                        (*bank, *program, *name) = (p.bank, p.program, p.name.clone());
                                    }
                                }
                            }
                        });
                        if ui.button("Load .sf2…").on_hover_text("Add a SoundFont from disk").clicked() {
                            load_font = true;
                        }
                    });
                    ui.label(egui::RichText::new(credit(font)).font(t.font(10.5)).color(t.text_dim));
                    ui.add_space(4.0);
                    let filter_id = egui::Id::unique("dd-preset-filter");
                    let mut filter: String = ui.data(|m| m.get_temp(filter_id)).unwrap_or_default();
                    ui.add(
                        egui::TextEdit::singleline(&mut filter).hint_text("Filter presets: koto, choir, sax…").desired_width(f32::INFINITY),
                    );
                    ui.data_mut(|m| m.insert_temp(filter_id, filter.clone()));
                    let f = filter.to_lowercase();
                    if let Some(lf) = fonts.get(font) {
                        egui::ScrollArea::vertical().max_height(ui.available_height() - 20.0).auto_shrink([false, true]).show(ui, |ui| {
                            for pr in lf.presets.iter().filter(|p| f.is_empty() || p.name.to_lowercase().contains(&f)) {
                                let on = *bank == pr.bank && *program == pr.program;
                                let label = if pr.bank == 128 {
                                    format!("Drums  {:>3}  {}", pr.program, pr.name)
                                } else {
                                    format!("{:>3}:{:<3} {}", pr.bank, pr.program, pr.name)
                                };
                                if ui.selectable_label(on, egui::RichText::new(label).font(egui::FontId::monospace(11.5))).clicked() {
                                    (*bank, *program, *name) = (pr.bank, pr.program, pr.name.clone());
                                    audition = Some(if pr.bank == 128 { 38 } else { 60 });
                                }
                            }
                        });
                    } else {
                        ui.colored_label(t.caution, format!("{font} isn't loaded"));
                    }
                });
            }
            Source::Kit(kit) => {
                let k = *kit;
                if k == KitKind::Drums {
                    widgets::note(
                        ui,
                        "Drums on the General MIDI drum keys: dull and heartbeat kicks, a gated snare far off, taiko toms, tabla, shaker and a ticking clock.",
                        "",
                    );
                    widgets::inset(ui, |ui| {
                        egui::ScrollArea::vertical().max_height(ui.available_height() - 20.0).auto_shrink([false, true]).show(ui, |ui| {
                            // the recorded rains first
                            let mut names = k.key_names();
                            names.sort_by_key(|(lo, _, _)| (k.rain_of(*lo).is_none(), *lo));
                            for (lo, hi, nm) in names {
                                let keys = if lo == hi { key_name(lo) } else { format!("{}–{}", key_name(lo), key_name(hi)) };
                                let key = if lo == hi { lo } else { lo + 6 };
                                let holding = held == Some((sel, key));
                                let help = if k.sustains(key) { "Click to hold it, again to stop" } else { "Click to hear it" };
                                if ui
                                    .selectable_label(
                                        holding,
                                        egui::RichText::new(format!("{}{keys:<8} {nm}", if holding { "■ " } else { "" }))
                                            .font(egui::FontId::monospace(11.5)),
                                    )
                                    .on_hover_text(help)
                                    .clicked()
                                {
                                    audition = Some(key);
                                    sustained = k.sustains(key);
                                }
                            }
                        });
                    });
                }
            }
        }
        if let Some(k) = audition {
            if sustained {
                // rain, hum and drones swell in over a second or more: hold until clicked again
                if let Some((_, h)) = held {
                    self.play_live(Msg::Off(h));
                }
                let next = if held == Some((sel, k)) { None } else { Some((sel, k)) };
                if next.is_some() {
                    self.play_live(Msg::On(k, 100));
                }
                ui.data_mut(|d| d.insert_temp(hold_id, next));
            } else {
                self.play_live(Msg::On(k, 100));
                self.play_live(Msg::Off(k));
            }
        }
        if load_font && let Some(path) = rfd::FileDialog::new().add_filter("SoundFont", &["sf2"]).pick_file() {
            match self.fonts.add(&path) {
                Ok(()) => {
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if let Some(f) = self.fonts.get(&name).cloned()
                        && let Source::SoundFont { font, bank, program, name: pn } = &mut self.doc.project.tracks[sel].source
                    {
                        *font = f.name.clone();
                        if let Some(p) = f.presets.first() {
                            (*bank, *program, *pn) = (p.bank, p.program, p.name.clone());
                        }
                    }
                    self.rebuild_instruments();
                    self.status = format!("Loaded {}", path.display());
                }
                Err(e) => self.status = e,
            }
        }
    }
}
