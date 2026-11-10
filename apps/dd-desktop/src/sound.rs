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
use crate::widgets::{self, Segment, SliderSpec};

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

#[allow(clippy::too_many_arguments)]
fn sl(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    v: &mut f32,
    range: (f32, f32),
    default: f32,
    decimals: usize,
    suffix: &str,
    log: bool,
    help: &str,
) {
    let spec = SliderSpec {
        id,
        label,
        help,
        min: range.0 as f64,
        max: range.1 as f64,
        default: default as f64,
        step: if decimals == 0 { 1.0 } else { 10f64.powi(-(decimals as i32)) },
        decimals,
        suffix,
        log,
        signed: range.0 < 0.0,
        ..Default::default()
    };
    let mut x = *v as f64;
    if widgets::slider(ui, &spec, &mut x, true) {
        *v = x as f32;
    }
}

fn amount(ui: &mut egui::Ui, id: &str, label: &str, v: &mut f32, default: f32, ends: (&str, &str), help: &str) {
    let spec = SliderSpec { id, label, help, ends: Some(ends), default: default as f64, ..Default::default() };
    let mut x = *v as f64;
    if widgets::slider(ui, &spec, &mut x, true) {
        *v = x as f32;
    }
}

fn secs(ui: &mut egui::Ui, id: &str, label: &str, v: &mut f32, default: f32, help: &str) {
    sl(ui, id, label, v, (0.001, 12.0), default, 3, " s", true, help);
}

fn env_ui(ui: &mut egui::Ui, id: &str, e: &mut Env, d: Env, what: &str) {
    secs(ui, &format!("{id}-a"), "Attack", &mut e.attack, d.attack, &format!("How long the {what} takes to rise"));
    secs(ui, &format!("{id}-d"), "Decay", &mut e.decay, d.decay, &format!("How long the {what} takes to fall to the sustain level"));
    amount(
        ui,
        &format!("{id}-s"),
        "Sustain",
        &mut e.sustain,
        d.sustain,
        ("none", "full"),
        &format!("Where the {what} rests while the key is held"),
    );
    secs(
        ui,
        &format!("{id}-r"),
        "Release",
        &mut e.release,
        d.release,
        &format!("How long the {what} takes to die away after the key is let go"),
    );
}

fn layer_ui(ui: &mut egui::Ui, l: &mut Layer, n: usize) {
    let d = Layer::default();
    let id = |s: &str| format!("l{n}-{s}");
    widgets::inset(ui, |ui| {
        ui.checkbox(&mut l.on, "Layer plays").on_hover_text("Switch this layer on or off");
    });
    ui.add_space(4.0);
    sl(ui, &id("level"), "Level", &mut l.level, (0.0, 1.0), d.level, 2, "", false, "How loud this layer is against the other");
    widgets::inset(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Octave");
            for o in -2i8..=2 {
                let label = match o {
                    -2 => "32'",
                    -1 => "16'",
                    0 => "8'",
                    1 => "4'",
                    _ => "2'",
                };
                ui.selectable_value(&mut l.octave, o, label).on_hover_text(format!("{o:+} octaves"));
            }
        });
    });
    sl(
        ui,
        &id("detune"),
        "Detune",
        &mut l.detune,
        (-50.0, 50.0),
        0.0,
        1,
        " ct",
        false,
        "Pull this layer off pitch against the other, for a thick, beating sound",
    );
    widgets::subtitle(ui, "Oscillators", "Mix the waves; they run into the filters");
    amount(ui, &id("saw"), "Saw", &mut l.saw, d.saw, ("off", "full"), "Bright and buzzy: brass and strings");
    amount(ui, &id("pulse"), "Pulse", &mut l.pulse, 0.0, ("off", "full"), "Hollow to nasal, by its width");
    sl(ui, &id("width"), "Width", &mut l.width, (0.05, 0.5), 0.5, 2, "", false, "Pulse width: 0.5 is a square, narrower is thinner");
    amount(ui, &id("pwm"), "PWM", &mut l.pwm, 0.0, ("still", "sweeping"), "The LFO sweeps the pulse width, for a chorused shimmer");
    amount(ui, &id("sine"), "Sine", &mut l.sine, 0.0, ("off", "full"), "Pure and round: bells and electric piano with FM");
    sl(
        ui,
        &id("fmr"),
        "FM ratio",
        &mut l.fm_ratio,
        (0.5, 16.0),
        1.0,
        2,
        "×",
        true,
        "The modulator's pitch against the note: whole numbers sound harmonic, others like bells",
    );
    amount(ui, &id("fmd"), "FM depth", &mut l.fm_depth, 0.0, ("pure", "metallic"), "How hard the modulator bends the sine");
    sl(
        ui,
        &id("fmt"),
        "FM decay",
        &mut l.fm_decay,
        (0.0, 8.0),
        0.0,
        2,
        " s",
        false,
        "How fast the FM brightness dies away (0 = held): the strike of a bell or tine",
    );
    amount(ui, &id("noise"), "Noise", &mut l.noise, 0.0, ("off", "full"), "Breath, wind, rain");
    amount(ui, &id("drive"), "Drive", &mut l.drive, 0.0, ("clean", "driven"), "Saturate before the filters");
    widgets::subtitle(ui, "Filters", "A high-pass then a low-pass, 12 dB/octave each");
    sl(ui, &id("hpf"), "High-pass", &mut l.hpf, (20.0, 6000.0), 20.0, 0, " Hz", true, "Thin out the bottom");
    amount(ui, &id("hpr"), "HP reso", &mut l.hpf_res, 0.0, ("flat", "peaky"), "A peak at the high-pass cutoff");
    sl(
        ui,
        &id("lpf"),
        "Low-pass",
        &mut l.lpf,
        (40.0, 18000.0),
        d.lpf,
        0,
        " Hz",
        true,
        "Darken the top: where the filter starts before its envelope opens it",
    );
    amount(ui, &id("lpr"), "LP reso", &mut l.lpf_res, d.lpf_res, ("flat", "singing"), "A peak at the low-pass cutoff");
    sl(
        ui,
        &id("envamt"),
        "Env amount",
        &mut l.env_amount,
        (-4.0, 6.0),
        d.env_amount,
        2,
        " oct",
        false,
        "How far the filter envelope opens (or closes) the low-pass",
    );
    amount(ui, &id("kt"), "Key track", &mut l.key_track, d.key_track, ("none", "full"), "Higher notes open the filter more");
    sl(
        ui,
        &id("vf"),
        "Touch → filter",
        &mut l.vel_to_filter,
        (0.0, 3.0),
        d.vel_to_filter,
        2,
        " oct",
        false,
        "Initial touch: harder notes play brighter",
    );
    widgets::subtitle(ui, "Filter envelope", "Opens the low-pass for each note");
    env_ui(ui, &id("fe"), &mut l.filter_env, d.filter_env, "filter");
    widgets::subtitle(ui, "Amp envelope", "The note's loudness");
    env_ui(ui, &id("ae"), &mut l.amp_env, d.amp_env, "level");
    amount(ui, &id("va"), "Touch → level", &mut l.vel_to_amp, d.vel_to_amp, ("even", "dynamic"), "How much harder notes play louder");
    sl(ui, &id("pan"), "Pan", &mut l.pan, (-1.0, 1.0), 0.0, 2, "", false, "Place the layer left or right");
}

fn patch_ui(ui: &mut egui::Ui, p: &mut Patch) {
    let d = Patch::default();
    let layer_id = egui::Id::unique("dd-edit-layer");
    let mut layer: usize = ui.data(|m| m.get_temp(layer_id)).unwrap_or(0);
    widgets::section(ui, "layers", "Layers I and II", true, |ui| {
        let segs = [
            Segment { label: "Layer I", selected: layer == 0, enabled: true, help: "The first voice of every note" },
            Segment { label: "Layer II", selected: layer == 1, enabled: true, help: "The second voice, played together with the first" },
        ];
        if let Some(i) = widgets::segmented(ui, &segs, 2) {
            layer = i;
        }
        ui.data_mut(|m| m.insert_temp(layer_id, layer));
        layer_ui(ui, &mut p.layers[layer], layer);
    });
    widgets::section(ui, "perf", "Glide, sag and ribbon", true, |ui| {
        widgets::inset(ui, |ui| {
            ui.checkbox(&mut p.mono, "Mono, legato")
                .on_hover_text("One note at a time; a note played while one is held glides there without restarting");
        });
        sl(ui, "glide", "Glide", &mut p.glide, (0.0, 2.0), 0.0, 2, " s", false, "Portamento: slide from one note to the next");
        sl(
            ui,
            "sag",
            "Sag",
            &mut p.sag,
            (-200.0, 50.0),
            0.0,
            0,
            " ct",
            false,
            "How far the pitch falls as a released note dies away: the drooping brass of an 80s polysynth",
        );
        sl(ui, "sagt", "Sag time", &mut p.sag_time, (0.05, 5.0), d.sag_time, 2, " s", true, "How long the sag takes");
        sl(
            ui,
            "bend",
            "Ribbon range",
            &mut p.bend_range,
            (0.0, 24.0),
            d.bend_range,
            0,
            " st",
            false,
            "Pitch bend range, for the ribbon and the bend wheel",
        );
        sl(
            ui,
            "drift",
            "Drift",
            &mut p.drift,
            (0.0, 30.0),
            d.drift,
            1,
            " ct",
            false,
            "Each voice wanders off pitch slowly, like old analogue oscillators",
        );
        amount(ui, "spread", "Spread", &mut p.spread, d.spread, ("centre", "wide"), "Spread the voices across the stereo field");
        amount(ui, "ring", "Ring mod", &mut p.ring, 0.0, ("off", "full"), "Multiply layer II by layer I: metallic, clangorous");
        sl(ui, "gain", "Gain", &mut p.gain_db, (-24.0, 12.0), 0.0, 1, " dB", false, "The patch's level");
    });
    widgets::section(ui, "lfo", "Vibrato and LFO", false, |ui| {
        let segs: Vec<Segment> = LfoShape::ALL
            .iter()
            .map(|s| Segment { label: s.label(), selected: p.lfo_shape == *s, enabled: true, help: "The LFO's shape" })
            .collect();
        if let Some(i) = widgets::segmented(ui, &segs, 5) {
            p.lfo_shape = LfoShape::ALL[i];
        }
        sl(ui, "rate", "Rate", &mut p.lfo_rate, (0.02, 20.0), d.lfo_rate, 2, " Hz", true, "How fast the LFO swings");
        sl(
            ui,
            "delay",
            "Delay",
            &mut p.lfo_delay,
            (0.0, 4.0),
            d.lfo_delay,
            2,
            " s",
            false,
            "Vibrato fades in this long after each note starts, as a player would",
        );
        sl(ui, "vib", "Vibrato", &mut p.vibrato, (0.0, 400.0), 0.0, 0, " ct", false, "Pitch wobble always on");
        sl(
            ui,
            "lfof",
            "LFO → filter",
            &mut p.lfo_to_filter,
            (0.0, 3.0),
            0.0,
            2,
            " oct",
            false,
            "Sweep the low-pass: slow for movement, fast for growl",
        );
        amount(ui, "trem", "Tremolo", &mut p.tremolo, 0.0, ("off", "deep"), "The LFO on the level");
        sl(
            ui,
            "wheel",
            "Mod wheel",
            &mut p.wheel_vibrato,
            (0.0, 200.0),
            d.wheel_vibrato,
            0,
            " ct",
            false,
            "Vibrato added at full modulation wheel",
        );
    });
    widgets::section(ui, "touch", "Touch", false, |ui| {
        widgets::note(ui, "Pressing harder on a held key (aftertouch, or the Touch strip below) changes the sound, as on the CS-80.", "");
        sl(ui, "tf", "Touch → filter", &mut p.touch_filter, (0.0, 4.0), d.touch_filter, 2, " oct", false, "Brightens as you press");
        sl(ui, "tv", "Touch → vibrato", &mut p.touch_vibrato, (0.0, 100.0), d.touch_vibrato, 0, " ct", false, "Vibrato as you press");
        sl(ui, "tb", "Touch → bend", &mut p.touch_bend, (-200.0, 200.0), 0.0, 0, " ct", false, "Bends the pitch as you press");
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
