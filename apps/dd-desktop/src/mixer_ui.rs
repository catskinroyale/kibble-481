//! The mixer: a channel strip per track and the master. The fader law, meters and
//! knobs are Ambient MIDI Generator's (`mixer_ui.rs`, MIT).

use dd_seq::project::{HIGH_CUT_OFF, LOW_CUT_OFF, SILENT_DB, level_at, level_gain};
use eframe::egui::{self, Align2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::panels::toggle;
use crate::theme::Tokens;
use crate::widgets::{self, DialLayout, DialSpec};

const FADER_H: f32 = 110.0;
const TOP_DB: f32 = 6.0;
pub(crate) const STRIP_W: f32 = 92.0;
/// The space between a strip's groups: name, pan and width, fader, dials.
const GAP: f32 = 4.0;
/// Fader (18) + meter (5) + buttons (20) with 4-point gaps, centred in the strip.
const FADER_INDENT: f32 = (STRIP_W - (18.0 + 5.0 + 20.0 + 2.0 * 4.0)) / 2.0;

impl App {
    pub(crate) fn mixer(&mut self, ui: &mut Ui) {
        let t = Tokens::get(ui.ctx());
        let n = self.doc.project.tracks.len();
        let peaks: Vec<f32> = (0..n).map(|i| self.meter(i)).collect();
        let lit: Vec<bool> = (0..n).map(|i| self.doc.project.audible(i)).collect();
        // where the playhead is, in bars, for each track's volume bar by bar
        let at_bar = self.heard().max(0.0) / self.doc.project.bar_secs();
        let master = (self.meter(n), self.meter(n + 1));
        egui::ScrollArea::horizontal().show(ui, |ui| {
            ui.horizontal_top(|ui| {
                let sel = self.sel;
                let mut clicked = None;
                for (i, tr) in self.doc.project.tracks.iter_mut().enumerate() {
                    let frame = egui::Frame::new()
                        .fill(if i == sel { t.hover } else { t.inset })
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(5, 4));
                    // the strip's own click sits under its controls, so M, S and the faders get theirs
                    let r = ui.scope_builder(egui::UiBuilder::new().id_salt(("strip", i)).sense(Sense::click()), |ui| {
                        frame.show(ui, |ui| {
                            ui.set_width(STRIP_W);
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                let c = crate::theme::track_colour(tr.color);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 3.0;
                                    track_lamp(ui, lit[i], c);
                                    ui.add(egui::Label::new(egui::RichText::new(&tr.name).font(t.semibold(11.5)).color(c)).truncate());
                                });
                                ui.add_space(GAP);
                                // the fader plus the track's bar volume where the music is
                                let auto = (!tr.levels.is_empty()).then(|| {
                                    let g = level_gain(level_at(&tr.levels, at_bar));
                                    if g <= 1e-6 { SILENT_DB } else { tr.strip.gain_db + 20.0 * g.log10() }
                                });
                                let s = &mut tr.strip;
                                knob(
                                    ui,
                                    "Pan",
                                    &mut s.pan,
                                    -1.0..=1.0,
                                    0.0,
                                    |v| {
                                        if v.abs() < 0.01 {
                                            "C".into()
                                        } else if v < 0.0 {
                                            format!("L{:.0}", -v * 100.0)
                                        } else {
                                            format!("R{:.0}", v * 100.0)
                                        }
                                    },
                                    "Left or right",
                                    &t,
                                );
                                knob(ui, "Width", &mut s.width, 0.0..=2.0, 1.0, |v| format!("{:.0}%", v * 100.0), "Mono to extra wide", &t);
                                ui.add_space(GAP);
                                ui.horizontal(|ui| {
                                    // the fader, meter and buttons centred in the strip
                                    ui.add_space(FADER_INDENT);
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    fader(ui, &mut s.gain_db, auto, &t);
                                    meter(ui, &[peaks[i]], true, &t);
                                    ui.vertical(|ui| {
                                        toggle(ui, "M", &mut s.mute, t.mute, "Mute");
                                        toggle(ui, "S", &mut s.solo, t.solo, "Solo");
                                        toggle(ui, "E", &mut s.ensemble, t.accent, "Through the ensemble chorus (amount under Effects)");
                                    });
                                });
                                match auto {
                                    // the volume bar by bar is moving it: show where it is now
                                    Some(a) => db_label(ui, format!("▸{}", db_text(a)), t.accent, &t).on_hover_text(format!(
                                        "Playing at {} now: the fader's {} and the track's volume bar by bar (the Vol lane)",
                                        db_text(a),
                                        db_text(s.gain_db)
                                    )),
                                    None => db_label(ui, db_text(s.gain_db), t.text_dim, &t),
                                };
                                ui.add_space(GAP);
                                rule(ui, &t);
                                ui.add_space(GAP);
                                sends_and_cuts(ui, s);
                            });
                        })
                    });
                    if r.response.clicked() {
                        clicked = Some(i);
                    }
                }
                if let Some(i) = clicked {
                    self.sel = i;
                }
                egui::Frame::new().fill(t.inset).corner_radius(4.0).inner_margin(egui::Margin::symmetric(6, 4)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        // a fixed width, or past the scroll area's edge its labels wrap a
                        // letter to a line and make the mixer tall
                        ui.set_width(52.0);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            track_lamp(ui, true, t.accent);
                            ui.label(egui::RichText::new("Master").font(t.semibold(11.5)));
                        });
                        // level with the tracks' faders: under their pan and width
                        ui.add_space(GAP + 2.0 * (18.0 + 2.0) + GAP);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            fader(ui, &mut self.doc.project.master_db, None, &t);
                            meter(ui, &[master.0, master.1], true, &t);
                        });
                        db_label(ui, db_text(self.doc.project.master_db), t.text_dim, &t);
                    });
                });
            });
        });
    }
}

/// The strip's lamp: lit while the track is heard (not muted, nor silenced by
/// another track's solo).
fn track_lamp(ui: &mut Ui, lit: bool, c: egui::Color32) {
    widgets::lamp(ui, lit, c, 12.0).on_hover_text(if lit { "Heard" } else { "Silent: muted, or another track is soloed" });
}

/// The level under a fader, centred across the strip.
fn db_label(ui: &mut Ui, text: String, colour: egui::Color32, t: &Tokens) -> egui::Response {
    let w = ui.available_width();
    ui.allocate_ui_with_layout(vec2(w, 14.0), egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.label(egui::RichText::new(text).font(t.mono(10.5)).color(colour))
    })
    .inner
}

/// A faint line across the strip between its groups.
pub(crate) fn rule(ui: &mut Ui, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0, t.divider));
}

/// Reverb and echo over the low and high cuts, as small dials two by two. Each
/// points straight up at its neutral setting: the reverb at its usual 35% (turn
/// either way), the echo with none (turn right for more), the low cut off (turn
/// right to cut higher) and the high cut off (turn left to cut lower).
fn sends_and_cuts(ui: &mut Ui, s: &mut dd_seq::project::Strip) {
    const REVERB: f32 = 0.35;
    const R: f32 = 10.0;
    let pct = |v: f32| format!("{:.0}%", v * 100.0);
    ui.columns(2, |c| {
        widgets::unit_dial(
            &mut c[0],
            "Reverb",
            "Send to the reverb. Straight up is the usual 35%",
            &mut s.reverb,
            REVERB,
            R,
            DialLayout::Under,
            pct,
        );
        widgets::unit_dial(&mut c[1], "Echo", "Send to the echo. Straight up is none", &mut s.echo, 0.0, R, DialLayout::Under, pct);
    });
    ui.add_space(3.0);
    // the cuts, evenly by octave from 20 Hz to 20 kHz
    ui.columns(2, |c| {
        let spec = DialSpec {
            label: "Lo cut",
            help: "Cut below this, 12 dB an octave: clears rumble and mud, or thins a sound out. Straight up is off",
            neutral: LOW_CUT_OFF,
            turns: 0.0..=1.0,
            radius: R,
            layout: DialLayout::Under,
        };
        widgets::dial(
            &mut c[0],
            &spec,
            &mut s.low_cut,
            hz_pos,
            |x| if x <= 0.0 { LOW_CUT_OFF } else { pos_hz(x) },
            |hz| if hz <= LOW_CUT_OFF { "Off".into() } else { hz_text(hz) },
        );
        let spec = DialSpec {
            label: "Hi cut",
            help: "Cut above this, 12 dB an octave: darkens or distances a sound, or keeps it out of a voice's way. Straight up is off",
            neutral: HIGH_CUT_OFF,
            turns: -1.0..=0.0,
            radius: R,
            layout: DialLayout::Under,
        };
        widgets::dial(
            &mut c[1],
            &spec,
            &mut s.high_cut,
            |hz| hz_pos(hz) - 1.0,
            |x| if x >= 0.0 { HIGH_CUT_OFF } else { pos_hz(x + 1.0) },
            |hz| if hz >= HIGH_CUT_OFF { "Off".into() } else { hz_text(hz) },
        );
    });
}

/// Where a frequency sits on a cut knob: 0 at 20 Hz, 1 at 20 kHz, evenly by octave.
fn hz_pos(hz: f32) -> f32 {
    ((hz.max(LOW_CUT_OFF) / LOW_CUT_OFF).log10() / 3.0).clamp(0.0, 1.0)
}

fn pos_hz(x: f32) -> f32 {
    // to two significant figures, so the values read cleanly
    let hz = LOW_CUT_OFF * 1000f32.powf(x.clamp(0.0, 1.0));
    let mag = 10f32.powf(hz.log10().floor() - 1.0);
    (hz / mag).round() * mag
}

fn hz_text(hz: f32) -> String {
    if hz >= 1000.0 { format!("{:.1} kHz", hz / 1000.0) } else { format!("{hz:.0} Hz") }
}

fn db_text(db: f32) -> String {
    if db <= SILENT_DB { "−∞ dB".into() } else { format!("{db:+.1} dB") }
}

/// A one-line labelled slider: a thin track filled from its resting value,
/// with the value shown while hovered. Double-click resets it.
#[allow(clippy::too_many_arguments)]
fn knob(
    ui: &mut Ui,
    label: &str,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    default: f32,
    value: impl Fn(f32) -> String,
    help: &str,
    t: &Tokens,
) {
    let (lo, hi) = (*range.start(), *range.end());
    let w = ui.available_width();
    let (row, _) = ui.allocate_exact_size(vec2(w, 18.0), Sense::hover());
    let label_w = 44.0;
    let track = Rect::from_min_max(pos2(row.left() + label_w, row.center().y - 7.0), pos2(row.right() - 4.0, row.center().y + 7.0));
    let resp = ui.interact(
        track.expand2(vec2(4.0, 2.0)),
        ui.make_persistent_id(("knob", label, row.min.x as i32, row.min.y as i32)),
        Sense::click_and_drag(),
    );
    if (resp.dragged() || resp.clicked())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let x = ((p.x - track.left()) / track.width()).clamp(0.0, 1.0);
        *v = lo + x * (hi - lo);
    }
    if resp.double_clicked() {
        *v = default;
    }
    let painter = ui.painter();
    let active = resp.hovered() || resp.dragged();
    let text = if active { value(*v) } else { label.to_string() };
    painter.text(pos2(row.left(), row.center().y), Align2::LEFT_CENTER, text, t.font(10.5), if active { t.text } else { t.text_label });
    let x_of = |x: f32| track.left() + track.width() * ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    let y = track.center().y;
    painter.hline(track.x_range(), y, Stroke::new(2.0, t.track));
    let (a, b) = (x_of(default), x_of(*v));
    painter.hline(a.min(b)..=a.max(b), y, Stroke::new(2.0, t.accent));
    painter.vline(a, (y - 3.0)..=(y + 3.0), Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)));
    let col = if active { t.thumb_hover } else { t.thumb };
    painter.circle_filled(pos2(b, y), 4.0, col);
    resp.on_hover_text(format!("{label}: {} · {help}", value(*v)));
}

/// A vertical fader in dB. Double-click resets it to 0 dB. `auto` is the level the
/// track's volume bar by bar puts it at, shown as a marker that moves with the music.
fn fader(ui: &mut Ui, db: &mut f32, auto: Option<f32>, t: &Tokens) {
    let (r, resp) = ui.allocate_exact_size(vec2(18.0, FADER_H), Sense::click_and_drag());
    let y_of = |d: f32| r.bottom() - 6.0 - (r.height() - 12.0) * db_pos(d);
    if (resp.dragged() || resp.clicked())
        && let Some(p) = resp.interact_pointer_pos()
    {
        let x = ((r.bottom() - 6.0 - p.y) / (r.height() - 12.0)).clamp(0.0, 1.0);
        *db = pos_db(x);
    }
    if resp.double_clicked() {
        *db = 0.0;
    }
    let painter = ui.painter();
    let track = Rect::from_center_size(pos2(r.center().x, r.center().y), vec2(3.0, r.height() - 12.0));
    painter.rect_filled(track, 1.5, t.track);
    let zero = y_of(0.0);
    painter.hline(r.x_range(), zero, Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)));
    let y = y_of(*db);
    let cap = Rect::from_center_size(pos2(r.center().x, y), vec2(16.0, 9.0));
    let fill = if resp.hovered() || resp.dragged() { t.thumb_hover } else { t.thumb };
    painter.rect_filled(cap, 2.5, fill);
    painter.hline(cap.x_range().shrink(3.0), y, Stroke::new(1.0, t.inset));
    if let Some(a) = auto {
        // where the volume bar by bar has the track now: a bar in the accent beside the cap
        let ya = y_of(a);
        painter.rect_filled(Rect::from_center_size(pos2(r.center().x, ya), vec2(16.0, 3.0)), 1.0, t.accent);
        painter.vline(r.center().x, y.min(ya)..=y.max(ya), Stroke::new(1.0, t.accent.gamma_multiply(0.6)));
    }
    resp.on_hover_text(format!("{} · drag to set, double-click for 0 dB", db_text(*db)));
}

/// Fader travel: the top quarter covers -6 to +6 dB, the rest down to silence.
pub(crate) fn db_pos(db: f32) -> f32 {
    if db <= SILENT_DB {
        0.0
    } else if db >= -6.0 {
        0.75 + 0.25 * (db + 6.0) / (TOP_DB + 6.0)
    } else {
        0.75 * ((db - SILENT_DB) / (-6.0 - SILENT_DB)).powf(1.6)
    }
}

fn pos_db(x: f32) -> f32 {
    if x <= 0.0 {
        SILENT_DB
    } else if x >= 0.75 {
        -6.0 + (x - 0.75) / 0.25 * (TOP_DB + 6.0)
    } else {
        SILENT_DB + (x / 0.75).powf(1.0 / 1.6) * (-6.0 - SILENT_DB)
    }
}

/// Peak meters, one bar per level, on the fader's scale.
fn meter(ui: &mut Ui, levels: &[f32], active: bool, t: &Tokens) {
    let w = 5.0 * levels.len() as f32 + 2.0 * (levels.len() as f32 - 1.0);
    let (r, _) = ui.allocate_exact_size(vec2(w, FADER_H), Sense::hover());
    let painter = ui.painter();
    let span = r.height() - 12.0;
    for (i, l) in levels.iter().enumerate() {
        let bar = Rect::from_min_size(pos2(r.left() + i as f32 * 7.0, r.top() + 6.0), vec2(5.0, span));
        painter.rect_filled(bar, 1.0, t.canvas);
        let db = if *l > 1e-6 { 20.0 * l.log10() } else { SILENT_DB };
        let h = span * db_pos(db);
        if h > 0.5 {
            let col = if db > -0.5 {
                t.alert
            } else if db > -6.0 {
                t.caution
            } else {
                t.accent
            };
            let col = if active { col } else { col.gamma_multiply(0.35) };
            painter.rect_filled(Rect::from_min_max(pos2(bar.left(), bar.bottom() - h), bar.max), 1.0, col);
        }
        painter.rect_stroke(bar, 1.0, Stroke::new(1.0, t.button_border.gamma_multiply(0.5)), StrokeKind::Outside);
    }
}
